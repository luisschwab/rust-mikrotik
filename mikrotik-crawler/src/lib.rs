//! Recursive read-only discovery crawler.
//!
//! This module owns the crawler orchestration: connecting to seed targets,
//! collecting [`mikrotik_types::device::DeviceSnapshot`] values, resolving
//! discovered neighbor addresses, and returning a [`CrawlReport`] with the
//! resulting [`mikrotik_graphviz::graph::model::NetworkGraph`].
//!
//! The crawler only calls read-oriented `print` commands, apart from the
//! authentication handshake required by the `RouterOS` API. Recursive discovery
//! is driven by `/ip/neighbor/print`; logical BGP edges are collected from BGP
//! session and connection state after a device is reached. If a BGP peer is
//! intentionally not crawled, the graph builder can still represent it as an
//! opaque inferred peer from the local router's BGP configuration.
//!
//! A [`resolver::TargetResolver`] is used when the discovered address differs from the
//! connectable target. This is common in QEMU runner scenarios, where routers discover
//! each other by in-topology addresses but the host reaches each CHR through a
//! forwarded `host:port`.
//!
//! Target failures are recorded in [`CrawlReport::failed_targets`].
//! Authentication failures for neighbor-discovered targets also annotate the
//! inferred graph node so topology output can show the device in place with a
//! wrong-credentials marker.

pub mod error;
pub mod resolver;

mod config;
mod connector;
mod discovery;
mod oneshot;
mod service;
mod snapshot;
mod state;

pub use config::AddressFamily;
pub use config::CrawlConfig;
pub use config::CrawlerServiceConfig;
pub use config::DEFAULT_COMMAND_TIMEOUT;
pub use config::DEFAULT_CONNECT_RETRIES;
pub use config::DEFAULT_CONNECT_TIMEOUT;
pub use connector::BinaryApiFactory;
pub use connector::BoxFuture;
pub use connector::DiscoveryClient;
pub use connector::DiscoveryClientFactory;
pub use connector::RouterOsApiConnector;
pub use connector::SnapshotClientConnector;
pub use oneshot::CrawlFailure;
pub use oneshot::CrawlReport;
pub use oneshot::Crawler;
pub use service::CrawlerHandle;
pub use service::CrawlerService;
pub use state::CrawlerStateSnapshot;
pub use state::SnapshotEvent;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::Instant;

    use mikrotik_client::builder::Protocol;
    use mikrotik_graphviz::constants::GRAPHVIZ_LAYERED_LAYOUT;
    use mikrotik_graphviz::constants::GRAPHVIZ_TYPED_RADIAL_LAYOUT;
    use mikrotik_graphviz::graph::build_graph;
    use mikrotik_graphviz::graph::build_graph_with_neighbor_evidence;
    use mikrotik_graphviz::graph::model::NetworkGraph;
    use mikrotik_graphviz::options::DotExportOptions;
    use mikrotik_graphviz::options::LinkFilter;
    use mikrotik_types::api::interface::Interface;
    use mikrotik_types::api::ip::Address;
    use mikrotik_types::api::ip::Neighbor;
    use mikrotik_types::api::ip::Route;
    use mikrotik_types::api::routing::BgpConnection;
    use mikrotik_types::api::routing::BgpPeer;
    use mikrotik_types::api::routing::BgpSession;
    use mikrotik_types::api::system::Identity;
    use mikrotik_types::api::system::Resource;
    use mikrotik_types::api::system::Routerboard;
    use mikrotik_types::device::DeviceRole;
    use mikrotik_types::device::DeviceSnapshot;
    use mikrotik_types::device::DeviceStatus;
    use mikrotik_types::primitives::interface::InterfaceType;
    use mikrotik_types::primitives::ip::IpPrefix;
    use mikrotik_types::target::Credentials;
    use mikrotik_types::target::DeviceTarget;
    use mikrotik_types::topology::FailedNeighborCrawl;
    use mikrotik_types::topology::InferredDeviceFailure;
    use mikrotik_types::topology::InferredNeighborEvidence;
    use mikrotik_types::topology::NetworkNodeStatus;

    use super::*;
    use crate::connector::builder_from_target;
    use crate::connector::split_host_port;
    use crate::error::Error;
    use crate::error::Result;
    use crate::resolver::StaticTargetResolver;
    use crate::state::snapshot_targets_by_retry_priority;

    #[test]
    fn snapshot_targets_put_failed_targets_first() {
        let mut state = CrawlerStateSnapshot::default();
        for address in ["10.0.0.2", "10.0.0.1", "10.0.0.3"] {
            state.targets.insert(socket(address), target(address));
        }
        state.failures.insert(socket("10.0.0.2"), "timeout".to_owned());
        state
            .failures
            .insert(socket("10.0.0.3"), "authentication failed".to_owned());

        let targets = snapshot_targets_by_retry_priority(&state, Instant::now())
            .into_iter()
            .map(|target| target.target.address.to_string())
            .collect::<Vec<_>>();

        assert_eq!(targets, ["10.0.0.2:8728", "10.0.0.3:8728", "10.0.0.1:8728"]);
    }

    #[tokio::test]
    async fn crawl_follows_mikrotik_neighbors_and_falls_back_for_unconnected_topology() {
        let factory = Arc::new(FakeFactory::new([
            snapshot(
                "10.0.0.1",
                "r1",
                "s1",
                [neighbor("ether1", "10.0.0.2", "r2"), non_mikrotik_neighbor("10.0.0.99")],
            ),
            snapshot("10.0.0.2", "r2", "s2", [neighbor("ether1", "10.0.0.1", "r1")]),
        ]));
        let seed = target("10.0.0.1");

        let report = Crawler::new(factory).crawl(seed).await.unwrap();

        assert_eq!(report.failed_targets, Vec::new());
        assert_eq!(report.graph.nodes.len(), 2);
        assert_eq!(report.graph.edges.len(), 1);
        assert!(report.graph.edges[0].is_fallback());
        assert!(
            report
                .graph
                .nodes
                .iter()
                .all(|node| node.status == NetworkNodeStatus::Collected)
        );
    }

    #[tokio::test]
    async fn crawl_keeps_neighbor_evidence_for_already_seen_targets() {
        let factory = Arc::new(FakeFactory::new([
            snapshot(
                "10.100.0.155",
                "Rt_Border",
                "border-serial",
                [
                    neighbor_with_remote_interface("sfp4-Orbitel", "10.100.0.220", "Rt_Sonata", "ether1"),
                    neighbor_with_remote_interface("sfp4-Orbitel", "10.100.0.230", "Sonata-Orbitel", "bridge"),
                ],
            ),
            snapshot(
                "10.100.0.220",
                "Rt_Sonata",
                "sonata-serial",
                [neighbor_with_remote_interface(
                    "ether1",
                    "10.100.0.230",
                    "Sonata-Orbitel",
                    "bridge",
                )],
            ),
            snapshot("10.100.0.230", "Sonata-Orbitel", "radio-serial", []),
        ]));

        let report = Crawler::new(factory)
            .with_config(CrawlConfig {
                max_depth: 2,
                max_devices: 1_000,
                max_concurrency: 16,
                address_family: AddressFamily::Ipv4,
                connect_retries: DEFAULT_CONNECT_RETRIES,
            })
            .crawl(target("10.100.0.155"))
            .await
            .unwrap();

        let sonata = report.graph.node_key_for_target_address("10.100.0.220").unwrap();
        let radio = report.graph.node_key_for_target_address("10.100.0.230").unwrap();
        assert!(report.graph.edges.iter().any(|edge| {
            edge.is_wireless()
                && ((&edge.local_node == sonata && &edge.remote_node == radio)
                    || (&edge.local_node == radio && &edge.remote_node == sonata))
        }));
    }

    #[tokio::test]
    async fn crawl_many_collects_multiple_seed_targets() {
        let factory = Arc::new(FakeFactory::new([
            snapshot("10.0.0.1", "r1", "s1", []),
            snapshot("10.0.0.2", "r2", "s2", []),
        ]));

        let report = Crawler::new(factory)
            .crawl_many([target("10.0.0.1"), target("10.0.0.2")])
            .await
            .unwrap();

        assert_eq!(report.failed_targets, Vec::new());
        assert_eq!(collected_count(&report.graph), 2);
    }

    #[tokio::test]
    async fn crawl_honors_max_depth() {
        let factory = Arc::new(FakeFactory::new([
            snapshot("10.0.0.1", "r1", "s1", [neighbor("ether1", "10.0.0.2", "r2")]),
            snapshot("10.0.0.2", "r2", "s2", []),
        ]));
        let seed = target("10.0.0.1");

        let report = Crawler::new(factory)
            .with_config(CrawlConfig {
                max_depth: 0,
                max_devices: 1_000,
                max_concurrency: 16,
                address_family: AddressFamily::Ipv4,
                ..CrawlConfig::default()
            })
            .crawl(seed)
            .await
            .unwrap();

        assert_eq!(report.graph.nodes.len(), 1);
        assert_eq!(collected_count(&report.graph), 1);
        assert_eq!(inferred_count(&report.graph), 0);
    }

    #[tokio::test]
    async fn crawl_honors_max_devices() {
        let factory = Arc::new(FakeFactory::new([
            snapshot("10.0.0.1", "r1", "s1", [neighbor("ether1", "10.0.0.2", "r2")]),
            snapshot("10.0.0.2", "r2", "s2", []),
        ]));
        let seed = target("10.0.0.1");

        let report = Crawler::new(factory)
            .with_config(CrawlConfig {
                max_depth: 10,
                max_devices: 1,
                max_concurrency: 16,
                address_family: AddressFamily::Ipv4,
                ..CrawlConfig::default()
            })
            .crawl(seed)
            .await
            .unwrap();

        assert_eq!(collected_count(&report.graph), 1);
    }

    #[tokio::test]
    async fn crawl_records_failed_targets_and_continues() {
        let factory = Arc::new(FakeFactory::new([snapshot(
            "10.0.0.1",
            "r1",
            "s1",
            [neighbor("ether1", "10.0.0.2", "r2")],
        )]));
        let seed = target("10.0.0.1");

        let report = Crawler::new(factory).crawl(seed).await.unwrap();

        assert_eq!(report.failed_targets.len(), 1);
        assert_eq!(report.failed_targets[0].address, "10.0.0.2:8728");
        assert_eq!(collected_count(&report.graph), 1);
        assert_eq!(inferred_count(&report.graph), 1);
        assert_eq!(report.graph.edges.len(), 1);
        assert!(report.graph.edges[0].is_fallback());
    }

    #[tokio::test]
    async fn crawl_retries_timed_out_targets() {
        let factory = Arc::new(TimeoutOnceFactory::new([
            snapshot("10.0.0.1", "r1", "s1", [neighbor("ether1", "10.0.0.2", "r2")]),
            snapshot("10.0.0.2", "r2", "s2", []),
        ]));
        let seed = target("10.0.0.1");

        let report = Crawler::new(factory.clone())
            .with_config(CrawlConfig {
                max_depth: 1,
                max_devices: 1_000,
                max_concurrency: 1,
                connect_retries: 1,
                address_family: AddressFamily::Ipv4,
            })
            .crawl(seed)
            .await
            .unwrap();

        assert_eq!(report.failed_targets, Vec::new());
        assert_eq!(collected_count(&report.graph), 2);
        assert_eq!(
            factory.connects(),
            vec![
                "10.0.0.1:8728".to_owned(),
                "10.0.0.2:8728".to_owned(),
                "10.0.0.2:8728".to_owned()
            ]
        );
    }

    #[tokio::test]
    async fn crawl_records_failed_seed_targets() {
        let factory = Arc::new(FakeFactory::new([]));
        let report = Crawler::new(factory).crawl(target("10.0.0.1")).await.unwrap();

        assert_eq!(report.failed_targets.len(), 1);
        assert_eq!(report.failed_targets[0].address, "10.0.0.1:8728");
        assert_eq!(collected_count(&report.graph), 0);
    }

    #[tokio::test]
    async fn crawl_avoids_cycles() {
        let factory = Arc::new(FakeFactory::new([
            snapshot("10.0.0.1", "r1", "s1", [neighbor("ether1", "10.0.0.2", "r2")]),
            snapshot("10.0.0.2", "r2", "s2", [neighbor("ether1", "10.0.0.1", "r1")]),
        ]));
        let seed = target("10.0.0.1");

        let report = Crawler::new(factory.clone()).crawl(seed).await.unwrap();

        assert_eq!(report.failed_targets, Vec::new());
        assert_eq!(
            factory.connects(),
            vec!["10.0.0.1:8728".to_owned(), "10.0.0.2:8728".to_owned()]
        );
    }

    #[tokio::test]
    async fn crawl_uses_static_target_resolver_for_discovered_neighbors() {
        let factory = Arc::new(FakeFactory::new([
            snapshot("127.0.0.1:5001", "r1", "s1", [neighbor("ether1", "10.0.0.2", "r2")]),
            snapshot("127.0.0.1:5002", "r2", "s2", []),
        ]));
        let resolver = StaticTargetResolver::new().with_target("10.0.0.2".parse().unwrap(), "127.0.0.1:5002");

        let report = Crawler::new(factory.clone())
            .with_target_resolver(Arc::new(resolver))
            .crawl(target("127.0.0.1:5001"))
            .await
            .unwrap();

        assert_eq!(report.failed_targets, Vec::new());
        assert_eq!(
            factory.connects(),
            vec!["127.0.0.1:5001".to_owned(), "127.0.0.1:5002".to_owned()]
        );
        assert_eq!(collected_count(&report.graph), 2);
    }

    #[test]
    fn graphviz_dot_contains_nodes_and_edges() {
        let mut r1 = snapshot("10.0.0.1", "r1", "s1", [neighbor("ether1", "10.0.0.2", "r2")]);
        r1.addresses = vec![address("10.0.0.1/30", "ether1")];
        let mut r2 = snapshot("10.0.0.2", "r2", "s2", [neighbor("ether2", "10.0.0.1", "r1")]);
        r2.addresses = vec![address("10.0.0.2/30", "ether2")];
        let local_node = r2.stable_key();
        let graph = build_graph_with_neighbor_evidence(
            &[r1, r2],
            [InferredNeighborEvidence {
                neighbor: neighbor("ether1", "10.0.0.1", "r1"),
                local_node,
                local_interface: Some("ether1".parse().unwrap()),
            }],
            [],
        );

        let dot = graph.to_graphviz_dot();

        assert!(dot.contains("digraph mikrotik_topology"));
        assert!(dot.contains("layout=twopi"));
        assert!(dot.contains("outputorder=edgesfirst"));
        assert!(dot.contains("overlap=prism"));
        assert!(dot.contains("overlap_scaling=0"));
        assert!(dot.contains("sep=\"+12\""));
        assert!(dot.contains("color=\"#94a3b8\""));
        assert!(dot.contains("penwidth=0.85"));
        assert!(!dot.contains("rankdir=LR"));
        assert!(dot.contains("\"s1\" [label=\"r1\\ns1\""));
        assert!(dot.contains("\"s2\" [label=\"r2\\ns2\""));
        assert!(dot.contains("\"__link_0\" [shape=plain, margin=0, label=<"));
        assert!(dot.contains(
            "tooltip=\"LINK TYPE: UNKNOWN\\n\\nNAME: r1\\nINTERFACE: ether1\\nADDRESSES: [10.0.0.1/30]\\n\\nNAME: r2\\nINTERFACE: ether2\\nADDRESSES: [10.0.0.2/30]\""
        ));
        assert!(dot.contains(
            "<TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">r1</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">ether1</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">10.0.0.1/30</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">unknown</FONT></TD>"
        ));
        assert!(dot.contains(
            "<TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">r2</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">ether2</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">10.0.0.2/30</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">unknown</FONT></TD>"
        ));
        assert!(dot.contains("\"s1\" -> \"__link_0\" [weight=4, color=\"#6b7280\", penwidth=0.85, tooltip=\"\"]"));
        assert!(dot.contains("\"__link_0\" -> \"s2\" [weight=4, color=\"#6b7280\", penwidth=0.85, tooltip=\"\"]"));
        assert_eq!(dot.matches("\"s1\" -> \"s2\"").count(), 0);
        assert_eq!(dot.matches("\"s2\" -> \"s1\"").count(), 0);

        let layered_dot = graph.to_graphviz_dot_with_options(&DotExportOptions::for_layout(GRAPHVIZ_LAYERED_LAYOUT));
        assert!(layered_dot.contains("rankdir=TB"));
        assert!(layered_dot.contains("\"__link_0\" [shape=plain, margin=0, label=<"));
        assert!(
            layered_dot
                .contains("\"s1\":e -> \"__link_0\":w [weight=4, color=\"#6b7280\", penwidth=0.85, tooltip=\"\"]")
        );
        assert!(
            layered_dot
                .contains("\"__link_0\":e -> \"s2\":w [weight=4, color=\"#6b7280\", penwidth=0.85, tooltip=\"\"]")
        );

        let typed_radial_dot =
            graph.to_graphviz_dot_with_options(&DotExportOptions::for_layout(GRAPHVIZ_TYPED_RADIAL_LAYOUT));
        assert!(typed_radial_dot.contains("layout=neato"));
        assert!(typed_radial_dot.contains("overlap=false"));
        assert!(typed_radial_dot.contains("pos=\"0.00,0.00!\", pin=true"));

        let tableless_dot = graph.to_graphviz_dot_with_options(&DotExportOptions {
            hide_link_tables: true,
            ..DotExportOptions::default()
        });
        assert!(!tableless_dot.contains("__link_0"));
        assert!(
            tableless_dot
                .contains("\"s1\" -> \"s2\" [color=\"#6b7280\", penwidth=0.85, tooltip=\"LINK TYPE: UNKNOWN\\n\\nNAME: r1\\nINTERFACE: ether1\\nADDRESSES: [10.0.0.1/30]\\n\\nNAME: r2\\nINTERFACE: ether2\\nADDRESSES: [10.0.0.2/30]\"];")
        );
    }

    #[test]
    fn graphviz_link_table_uses_extra_address_rows() {
        let mut r1 = snapshot("10.0.0.1", "r1", "s1", [neighbor("ether1", "10.0.0.2", "r2")]);
        r1.addresses = vec![
            address("10.0.0.1/30", "ether1"),
            address("203.0.114.10/32", "ether1"),
            address("203.0.114.11/32", "ether1"),
        ];
        let mut r2 = snapshot("10.0.0.2", "r2", "s2", [neighbor("ether2", "10.0.0.1", "r1")]);
        r2.role = DeviceRole::CustomerRouter;
        r2.addresses = vec![address("10.0.0.2/30", "ether2")];
        let local_node = r2.stable_key();
        let graph = build_graph_with_neighbor_evidence(
            &[r1, r2],
            [InferredNeighborEvidence {
                neighbor: neighbor("ether1", "10.0.0.1", "r1"),
                local_node,
                local_interface: Some("ether1".parse().unwrap()),
            }],
            [],
        );

        let dot = graph.to_graphviz_dot();

        assert!(dot.contains(
            "<TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">r1</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">ether1</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">10.0.0.1/30</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">customer</FONT></TD>"
        ));
        assert!(dot.contains(
            "<TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">&#160;</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">&#160;</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">203.0.114.10/32</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">&#160;</FONT></TD>"
        ));
        assert!(dot.contains(
            "<TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">&#160;</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">&#160;</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">203.0.114.11/32</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"Berkeley Mono\" POINT-SIZE=\"10\">&#160;</FONT></TD>"
        ));
    }

    #[test]
    fn graph_connects_failed_neighbor_when_address_matches_local_l3_prefix() {
        let mut r1 = snapshot("10.0.0.1", "r1", "s1", []);
        r1.addresses = vec![address("10.0.0.1/30", "ether1")];
        let local_node = r1.stable_key();
        let graph = build_graph(
            &[r1],
            [FailedNeighborCrawl {
                neighbor: neighbor("ether1", "10.0.0.2", "r2"),
                local_node,
                local_interface: Some("ether1".parse().unwrap()),
                failure: InferredDeviceFailure::WrongCredentials,
            }],
        );

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        let edge = &graph.edges[0];
        if edge.local_node == "s1".to_owned().into() {
            assert_eq!(edge.local_interface, Some("ether1".parse().unwrap()));
            assert_eq!(edge.remote_interface, None);
        } else {
            assert_eq!(edge.remote_node, "s1".to_owned().into());
            assert_eq!(edge.remote_interface, Some("ether1".parse().unwrap()));
            assert_eq!(edge.local_interface, None);
        }
        let dot = graph.to_graphviz_dot();
        assert!(dot.contains("r2\\nERROR: BAD CREDENTIALS"));
        assert!(dot.contains("ADDRESSES: [10.0.0.2]"));
    }

    #[test]
    fn graph_does_not_infer_l3_topology_from_broad_management_prefixes() {
        let mut customer = snapshot("10.100.0.220", "customer", "customer-serial", []);
        customer.addresses = vec![address("10.100.0.220/24", "ether1")];
        let mut core = snapshot("10.100.0.155", "core", "core-serial", []);
        core.addresses = vec![address("10.100.0.155/24", "sfp1")];

        let graph = build_graph(&[customer, core], []);

        assert_eq!(graph.nodes.len(), 2);
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn graph_uses_neighbor_fallback_for_otherwise_unconnected_collected_nodes() {
        let mut customer = snapshot("10.100.0.220", "customer", "customer-serial", []);
        customer.addresses = vec![address("10.100.0.220/24", "ether1")];
        let mut core = snapshot("10.100.0.155", "core", "core-serial", []);
        core.addresses = vec![address("10.100.0.155/24", "sfp1")];
        let local_node = core.stable_key();

        let graph = build_graph_with_neighbor_evidence(
            &[customer, core],
            [InferredNeighborEvidence {
                neighbor: neighbor("ether1", "10.100.0.220", "customer"),
                local_node,
                local_interface: Some("sfp1".parse().unwrap()),
            }],
            [],
        );

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].local_node, "core-serial".to_owned().into());
        assert_eq!(graph.edges[0].local_interface, Some("sfp1".parse().unwrap()));
        assert_eq!(graph.edges[0].remote_node, "customer-serial".to_owned().into());
        assert_eq!(graph.edges[0].remote_interface, Some("ether2".parse().unwrap()));
        assert_eq!(graph.edges[0].confidence, 25);
        assert!(graph.edges[0].is_fallback());
        assert_eq!(graph.filtered_edges(LinkFilter::Routing).len(), 1);
        assert_eq!(graph.filtered_edges(LinkFilter::PhysicalOnly).len(), 0);
    }

    #[test]
    fn graph_does_not_add_neighbor_fallback_for_already_connected_nodes() {
        let mut customer = snapshot("10.100.0.220", "customer", "customer-serial", []);
        customer.addresses = vec![address("10.100.0.220/30", "ether1")];
        let mut core = snapshot("10.100.0.221", "core", "core-serial", []);
        core.addresses = vec![address("10.100.0.221/30", "sfp1")];
        let local_node = core.stable_key();

        let graph = build_graph_with_neighbor_evidence(
            &[customer, core],
            [InferredNeighborEvidence {
                neighbor: neighbor("ether1", "10.100.0.220", "customer"),
                local_node,
                local_interface: Some("sfp1".parse().unwrap()),
            }],
            [],
        );

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert!(!graph.edges[0].is_management());
    }

    #[test]
    fn graph_adds_fallback_for_failed_neighbor_nodes() {
        let mut r1 = snapshot("10.0.0.1", "r1", "s1", []);
        r1.interfaces = vec![interface("bridge", InterfaceType::Bridge)];
        r1.addresses = vec![address("10.0.0.1/24", "bridge")];

        let local_node = r1.stable_key();
        let graph = build_graph_with_neighbor_evidence(
            &[r1],
            [InferredNeighborEvidence {
                neighbor: neighbor("ether1", "10.0.0.2", "r2"),
                local_node: local_node.clone(),
                local_interface: Some("ether1".parse().unwrap()),
            }],
            [FailedNeighborCrawl {
                neighbor: neighbor("ether1", "10.0.0.2", "r2"),
                local_node,
                local_interface: Some("ether1".parse().unwrap()),
                failure: InferredDeviceFailure::WrongCredentials,
            }],
        );

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert!(graph.edges[0].is_fallback());
        assert_eq!(graph.filtered_edges(LinkFilter::Routing).len(), 1);
        let dot = graph.to_graphviz_dot();
        assert!(dot.contains("LINK TYPE: FALLBACK"));
        assert!(!dot.contains("LINK TYPE: MANAGEMENT"));
    }

    #[test]
    fn graph_keeps_unreachable_neighbor_nodes_connected_with_fallback_edges() {
        let mut r1 = snapshot("10.0.0.1", "r1", "s1", []);
        r1.interfaces = vec![interface("bridge", InterfaceType::Bridge)];
        r1.addresses = vec![address("10.0.0.1/24", "bridge")];

        let local_node = r1.stable_key();
        let graph = build_graph_with_neighbor_evidence(
            &[r1],
            [InferredNeighborEvidence {
                neighbor: neighbor("ether4", "192.168.1.31", "mikrotik-hq"),
                local_node: local_node.clone(),
                local_interface: Some("ether4".parse().unwrap()),
            }],
            [FailedNeighborCrawl {
                neighbor: neighbor("ether4", "192.168.1.31", "mikrotik-hq"),
                local_node,
                local_interface: Some("ether4".parse().unwrap()),
                failure: InferredDeviceFailure::Unreachable,
            }],
        );

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert!(graph.edges[0].is_fallback());
        let dot = graph.to_graphviz_dot();
        assert!(dot.contains("mikrotik-hq\\nERROR: UNREACHABLE"));
        assert!(dot.contains("LINK TYPE: FALLBACK"));
        assert!(!dot.contains("LINK TYPE: MANAGEMENT"));
        assert!(dot.contains("MANAGEMENT ADDRESS: 192.168.1.31"));
    }

    #[test]
    fn graph_labels_refused_api_neighbor_as_api_refused() {
        let mut r1 = snapshot("10.0.0.1", "r1", "s1", []);
        r1.interfaces = vec![interface("bridge", InterfaceType::Bridge)];
        r1.addresses = vec![address("10.0.0.1/24", "bridge")];

        let local_node = r1.stable_key();
        let graph = build_graph_with_neighbor_evidence(
            &[r1],
            [InferredNeighborEvidence {
                neighbor: neighbor("ether1", "10.0.0.2", "r2"),
                local_node: local_node.clone(),
                local_interface: Some("ether1".parse().unwrap()),
            }],
            [FailedNeighborCrawl {
                neighbor: neighbor("ether1", "10.0.0.2", "r2"),
                local_node,
                local_interface: Some("ether1".parse().unwrap()),
                failure: InferredDeviceFailure::ApiRefused,
            }],
        );

        assert_eq!(graph.edges.len(), 1);
        assert!(graph.edges[0].is_fallback());
        let dot = graph.to_graphviz_dot();
        assert!(dot.contains("r2\\nERROR: API REFUSED"));
        assert!(dot.contains("ERROR: API REFUSED"));
        assert!(dot.contains("LINK TYPE: FALLBACK"));
        assert!(dot.contains("MANAGEMENT ADDRESS: 10.0.0.2"));
    }

    #[test]
    fn graph_adds_bgp_edges_from_collected_sessions() {
        let mut r1 = snapshot("127.0.0.1:5001", "r1", "s1", []);
        r1.addresses = vec![address("10.0.0.1/30", "ether2")];
        r1.bgp_sessions = vec![BgpSession {
            remote_address: Some("10.0.0.2".parse().unwrap()),
            established: Some(true),
            ..BgpSession::default()
        }];
        let mut r2 = snapshot("127.0.0.1:5002", "r2", "s2", []);
        r2.addresses = vec![address("10.0.0.2/30", "ether3")];

        let graph = build_graph(&[r1, r2], []);

        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].local_interface, Some("ether2".parse().unwrap()));
        assert_eq!(graph.edges[0].remote_interface, Some("ether3".parse().unwrap()));
        assert_eq!(graph.edges[0].confidence, 95);
    }

    #[test]
    fn graph_adds_bgp_edges_from_configured_connections() {
        let mut r1 = snapshot("127.0.0.1:5001", "r1", "s1", []);
        r1.addresses = vec![address("10.0.0.1/30", "ether2")];
        r1.bgp_connections = vec![BgpConnection {
            remote_address: Some("10.0.0.2/32".parse().unwrap()),
            disabled: Some(false),
            ..BgpConnection::default()
        }];
        let mut r2 = snapshot("127.0.0.1:5002", "r2", "s2", []);
        r2.addresses = vec![address("10.0.0.2/30", "ether3")];

        let graph = build_graph(&[r1, r2], []);

        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].local_interface, Some("ether2".parse().unwrap()));
        assert_eq!(graph.edges[0].remote_interface, Some("ether3".parse().unwrap()));
        assert_eq!(graph.edges[0].confidence, 90);
        assert!(graph.edges[0].is_bgp());
    }

    #[test]
    fn graph_ignores_disabled_bgp_connections() {
        let mut r1 = snapshot("127.0.0.1:5001", "r1", "s1", []);
        r1.addresses = vec![address("10.0.0.1/30", "ether2")];
        r1.bgp_connections = vec![BgpConnection {
            remote_address: Some("198.51.100.1/32".parse().unwrap()),
            disabled: Some(true),
            ..BgpConnection::default()
        }];

        let graph = build_graph(&[r1], []);

        assert!(graph.edges.is_empty());
        assert!(!graph.nodes.iter().any(|node| node.key.as_str() == "bgp:198.51.100.1"));
    }

    #[test]
    fn graph_adds_bgp_edges_from_enabled_v6_peers() {
        let mut r1 = snapshot("127.0.0.1:5001", "r1", "s1", []);
        r1.addresses = vec![address("10.100.0.155/24", "ether2")];
        r1.bgp_peers = vec![BgpPeer {
            name: Some("Rt_BORDERv1".to_owned()),
            remote_address: Some("10.100.0.157".parse().unwrap()),
            remote_as: Some(65001),
            disabled: Some(false),
            established: None,
            ..BgpPeer::default()
        }];

        let graph = build_graph(&[r1], []);

        assert_eq!(graph.edges.len(), 1);
        let edge = &graph.edges[0];
        if edge.local_node.as_str() == "s1" {
            assert_eq!(edge.local_interface, Some("ether2".parse().unwrap()));
            assert_eq!(edge.remote_node, "bgp:10.100.0.157".to_owned().into());
            assert_eq!(edge.remote_interface, None);
        } else {
            assert_eq!(edge.local_node, "bgp:10.100.0.157".to_owned().into());
            assert_eq!(edge.local_interface, None);
            assert_eq!(edge.remote_node, "s1".to_owned().into());
            assert_eq!(edge.remote_interface, Some("ether2".parse().unwrap()));
        }
        assert_eq!(graph.edges[0].confidence, 70);
        assert!(graph.edges[0].is_bgp());
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| { node.key.as_str() == "bgp:10.100.0.157" && node.label() == "Rt_BORDERv1\n10.100.0.157" })
        );
    }

    #[test]
    fn graph_ignores_disabled_v6_bgp_peers() {
        let mut r1 = snapshot("127.0.0.1:5001", "r1", "s1", []);
        r1.addresses = vec![address("10.100.0.155/24", "ether2")];
        r1.bgp_peers = vec![BgpPeer {
            name: Some("Rt_BORDERv1".to_owned()),
            remote_address: Some("10.100.0.157".parse().unwrap()),
            remote_as: Some(65001),
            disabled: Some(true),
            established: None,
            ..BgpPeer::default()
        }];

        let graph = build_graph(&[r1], []);

        assert!(graph.edges.is_empty());
        assert!(!graph.nodes.iter().any(|node| node.key.as_str() == "bgp:10.100.0.157"));
    }

    #[test]
    fn graph_adds_route_next_hop_edges_to_collected_routers() {
        let mut core = snapshot("10.100.0.208", "core", "core-serial", []);
        core.addresses = vec![address("10.100.0.208/24", "mgmt")];
        core.routes = vec![Route {
            dst_address: Some("10.10.10.0/24".parse().unwrap()),
            immediate_gw: Some("10.100.0.147%mgmt".parse().unwrap()),
            active: Some(true),
            connect: Some(false),
            disabled: Some(false),
            ..Route::default()
        }];
        let mut customer = snapshot("10.100.0.147", "customer", "customer-serial", []);
        customer.addresses = vec![
            address("10.100.0.147/24", "ether4_WAN"),
            address("10.10.10.254/24", "lan"),
        ];

        let graph = build_graph(&[core, customer], []);

        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].local_node, "core-serial".to_owned().into());
        assert_eq!(graph.edges[0].local_interface, Some("mgmt".parse().unwrap()));
        assert_eq!(graph.edges[0].remote_node, "customer-serial".to_owned().into());
        assert_eq!(graph.edges[0].remote_interface, Some("ether4_WAN".parse().unwrap()));
        assert_eq!(graph.edges[0].confidence, 80);
    }

    #[test]
    fn graph_ignores_routes_that_are_not_explicitly_active() {
        let mut core = snapshot("10.100.0.208", "core", "core-serial", []);
        core.addresses = vec![address("10.100.0.208/24", "mgmt")];
        core.routes = vec![
            Route {
                dst_address: Some("10.10.10.0/24".parse().unwrap()),
                immediate_gw: Some("10.100.0.147%mgmt".parse().unwrap()),
                active: None,
                connect: Some(false),
                disabled: Some(false),
                ..Route::default()
            },
            Route {
                dst_address: Some("10.10.20.0/24".parse().unwrap()),
                immediate_gw: Some("10.100.0.148%mgmt".parse().unwrap()),
                active: Some(false),
                connect: Some(false),
                disabled: Some(false),
                ..Route::default()
            },
            Route {
                dst_address: Some("10.10.30.0/24".parse().unwrap()),
                immediate_gw: Some("10.100.0.149%mgmt".parse().unwrap()),
                active: Some(true),
                inactive: Some(true),
                connect: Some(false),
                disabled: Some(false),
                ..Route::default()
            },
            Route {
                dst_address: Some("10.10.40.0/24".parse().unwrap()),
                immediate_gw: Some("10.100.0.150%mgmt".parse().unwrap()),
                active: Some(true),
                connect: Some(false),
                disabled: Some(true),
                ..Route::default()
            },
        ];
        let customer1 = snapshot("10.100.0.147", "customer1", "customer-serial-1", []);
        let customer2 = snapshot("10.100.0.148", "customer2", "customer-serial-2", []);
        let customer3 = snapshot("10.100.0.149", "customer3", "customer-serial-3", []);
        let customer4 = snapshot("10.100.0.150", "customer4", "customer-serial-4", []);

        let snapshots = vec![core, customer1, customer2, customer3, customer4];
        let graph = build_graph(&snapshots, []);

        assert!(graph.edges.is_empty());
    }

    #[test]
    fn split_host_port_handles_defaults_ports_and_ipv6() {
        assert_eq!(split_host_port("192.0.2.1").unwrap(), ("192.0.2.1".to_owned(), 8728));
        assert_eq!(
            split_host_port("router.example:18728").unwrap(),
            ("router.example".to_owned(), 18728)
        );
        assert_eq!(
            split_host_port("2001:db8::1").unwrap(),
            ("2001:db8::1".to_owned(), 8728)
        );
        assert_eq!(
            split_host_port("[2001:db8::1]:18728").unwrap(),
            ("2001:db8::1".to_owned(), 18728)
        );
    }

    #[test]
    fn builder_from_target_uses_protocol_ports_for_standard_api_ports() {
        let target = DeviceTarget::new("192.0.2.1:8728", "read-only", Some("secret".to_owned())).unwrap();

        let ssl_builder = builder_from_target(&target, Protocol::ApiSsl, DEFAULT_CONNECT_TIMEOUT);
        assert_eq!(ssl_builder.host, "192.0.2.1");
        assert_eq!(ssl_builder.protocol, Protocol::ApiSsl);
        assert_eq!(ssl_builder.port, Protocol::ApiSsl.default_port());

        let api_target = DeviceTarget::new("192.0.2.1:8729", "read-only", Some("secret".to_owned())).unwrap();
        let api_builder = builder_from_target(&api_target, Protocol::Api, DEFAULT_CONNECT_TIMEOUT);
        assert_eq!(api_builder.host, "192.0.2.1");
        assert_eq!(api_builder.protocol, Protocol::Api);
        assert_eq!(api_builder.port, Protocol::Api.default_port());
    }

    #[test]
    fn builder_from_target_preserves_custom_forwarded_ports() {
        let target = DeviceTarget::new("127.0.0.1:54038", "admin", None).unwrap();

        let builder = builder_from_target(&target, Protocol::ApiSsl, DEFAULT_CONNECT_TIMEOUT);

        assert_eq!(builder.host, "127.0.0.1");
        assert_eq!(builder.protocol, Protocol::ApiSsl);
        assert_eq!(builder.port, 54038);
    }

    #[derive(Debug)]
    struct FakeFactory {
        snapshots: BTreeMap<core::net::SocketAddr, DeviceSnapshot>,
        connects: Mutex<Vec<String>>,
    }

    impl FakeFactory {
        fn new<const N: usize>(snapshots: [DeviceSnapshot; N]) -> Self {
            Self {
                snapshots: snapshots
                    .into_iter()
                    .map(|snapshot| (snapshot.target_address, snapshot))
                    .collect(),
                connects: Mutex::new(Vec::new()),
            }
        }

        fn connects(&self) -> Vec<String> {
            self.connects.lock().unwrap().clone()
        }
    }

    impl DiscoveryClientFactory for FakeFactory {
        fn connect<'a>(&'a self, target: &'a DeviceTarget) -> BoxFuture<'a, Result<Arc<dyn DiscoveryClient>>> {
            Box::pin(async move {
                self.connects.lock().unwrap().push(target.address.to_string());
                let snapshot = self
                    .snapshots
                    .get(&target.address)
                    .cloned()
                    .ok_or_else(|| Error::InvalidTarget {
                        address: target.address.to_string(),
                        message: "missing fake snapshot".to_owned(),
                    })?;

                Ok(Arc::new(FakeClient { snapshot }) as Arc<dyn DiscoveryClient>)
            })
        }
    }

    #[derive(Debug)]
    struct FakeClient {
        snapshot: DeviceSnapshot,
    }

    impl DiscoveryClient for FakeClient {
        fn snapshot<'a>(&'a self, _target_address: &'a str) -> BoxFuture<'a, Result<DeviceSnapshot>> {
            Box::pin(async move { Ok(self.snapshot.clone()) })
        }
    }

    #[derive(Debug)]
    struct TimeoutOnceFactory {
        snapshots: BTreeMap<core::net::SocketAddr, DeviceSnapshot>,
        connects: Mutex<Vec<String>>,
        timed_out: Mutex<BTreeSet<core::net::SocketAddr>>,
    }

    impl TimeoutOnceFactory {
        fn new<const N: usize>(snapshots: [DeviceSnapshot; N]) -> Self {
            Self {
                snapshots: snapshots
                    .into_iter()
                    .map(|snapshot| (snapshot.target_address, snapshot))
                    .collect(),
                connects: Mutex::new(Vec::new()),
                timed_out: Mutex::new(BTreeSet::new()),
            }
        }

        fn connects(&self) -> Vec<String> {
            self.connects.lock().unwrap().clone()
        }
    }

    impl DiscoveryClientFactory for TimeoutOnceFactory {
        fn connect<'a>(&'a self, target: &'a DeviceTarget) -> BoxFuture<'a, Result<Arc<dyn DiscoveryClient>>> {
            Box::pin(async move {
                self.connects.lock().unwrap().push(target.address.to_string());
                if target.address != socket("10.0.0.1") && self.timed_out.lock().unwrap().insert(target.address) {
                    return Err(Error::Client(mikrotik_client::error::Error::Io(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "test timeout",
                    ))));
                }

                let snapshot = self
                    .snapshots
                    .get(&target.address)
                    .cloned()
                    .ok_or_else(|| Error::InvalidTarget {
                        address: target.address.to_string(),
                        message: "missing fake snapshot".to_owned(),
                    })?;

                Ok(Arc::new(FakeClient { snapshot }) as Arc<dyn DiscoveryClient>)
            })
        }
    }

    fn target(address: &str) -> DeviceTarget {
        DeviceTarget {
            address: socket(address),
            credentials: Credentials {
                username: "admin".to_owned(),
                password: Some("password".to_owned()),
            },
        }
    }

    fn snapshot<const N: usize>(
        target_address: &str,
        identity: &str,
        serial: &str,
        neighbors: [Neighbor; N],
    ) -> DeviceSnapshot {
        let (host, _port) = split_host_port(target_address).unwrap();
        DeviceSnapshot {
            target_address: socket(target_address),
            collected_at: time::OffsetDateTime::UNIX_EPOCH,
            status: DeviceStatus::Reachable,
            role: DeviceRole::Unknown,
            fw_update_pending: false,
            management_addresses: vec![host.parse().unwrap()],
            identity: Identity {
                name: Some(identity.to_owned()),
            },
            routerboard: Routerboard {
                serial_number: Some(serial.to_owned()),
                ..Routerboard::default()
            },
            resource: Resource::default(),
            addresses: vec![Address {
                address: Some(format!("{host}/32").parse::<IpPrefix>().unwrap()),
                interface: Some("ether1".parse().unwrap()),
                ..Address::default()
            }],
            neighbors: neighbors.into(),
            ..DeviceSnapshot::default()
        }
    }

    fn socket(address: &str) -> core::net::SocketAddr {
        if let Ok(address) = address.parse() {
            return address;
        }
        format!("{address}:8728").parse().unwrap()
    }

    fn neighbor(interface: &str, address: &str, identity: &str) -> Neighbor {
        neighbor_with_remote_interface(interface, address, identity, "ether2")
    }

    fn neighbor_with_remote_interface(
        interface: &str,
        address: &str,
        identity: &str,
        remote_interface: &str,
    ) -> Neighbor {
        Neighbor {
            interface: Some(interface.parse().unwrap()),
            interface_name: Some(remote_interface.parse().unwrap()),
            address: Some(address.parse().unwrap()),
            identity: Some(identity.to_owned()),
            board: Some("CHR".to_owned()),
            ..Neighbor::default()
        }
    }

    fn address(prefix: &str, interface: &str) -> Address {
        Address {
            address: Some(prefix.parse().unwrap()),
            interface: Some(interface.parse().unwrap()),
            ..Address::default()
        }
    }

    fn interface(name: &str, interface_type: InterfaceType) -> Interface {
        Interface {
            name: Some(name.parse().unwrap()),
            interface_type: Some(interface_type),
            ..Interface::default()
        }
    }

    fn non_mikrotik_neighbor(address: &str) -> Neighbor {
        Neighbor {
            address: Some(address.parse().unwrap()),
            identity: Some("printer".to_owned()),
            ..Neighbor::default()
        }
    }

    fn collected_count(graph: &NetworkGraph) -> usize {
        graph
            .nodes
            .iter()
            .filter(|node| node.status == NetworkNodeStatus::Collected)
            .count()
    }

    fn inferred_count(graph: &NetworkGraph) -> usize {
        graph
            .nodes
            .iter()
            .filter(|node| node.status == NetworkNodeStatus::Inferred)
            .count()
    }
}
