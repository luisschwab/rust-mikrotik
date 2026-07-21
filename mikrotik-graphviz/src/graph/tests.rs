use core::net::SocketAddr;
use std::collections::BTreeSet;

use mikrotik_types::abstractions::LinkKind;
use mikrotik_types::api::interface::Interface;
use mikrotik_types::api::interface::WifiRegistration;
use mikrotik_types::api::interface::WirelessRegistration;
use mikrotik_types::api::ip::Neighbor;
use mikrotik_types::api::routing::BgpConnection;
use mikrotik_types::api::system::Identity;
use mikrotik_types::api::system::Resource;
use mikrotik_types::api::system::Routerboard;
use mikrotik_types::device::DeviceRole;
use mikrotik_types::device::RouterOsSnapshot;
use mikrotik_types::device::RoutingSnapshot;
use mikrotik_types::device::SystemSnapshot;
use mikrotik_types::primitives::ip::DiscoveryProtocol;
use mikrotik_types::topology::InferredNeighborEvidence;
use mikrotik_types::topology::NetworkNode;
use mikrotik_types::topology::NetworkNodeStatus;
use mikrotik_types::topology::TopologyLink;

use super::rank::is_radio_name;
use super::*;
use crate::constants::GRAPHVIZ_LAYERED_LAYOUT;
use crate::constants::GRAPHVIZ_RECURSIVE_RADIAL_LAYOUT;
use crate::constants::GRAPHVIZ_SECTION_BORDER_Y;
use crate::constants::GRAPHVIZ_SECTION_CORE_Y;
use crate::constants::GRAPHVIZ_SECTION_CUSTOMER_Y;
use crate::constants::GRAPHVIZ_SECTION_NODE_SPACING;
use crate::constants::GRAPHVIZ_SECTION_OWNED_BGP_Y;
use crate::constants::GRAPHVIZ_SECTION_UPSTREAM_Y;
use crate::constants::GRAPHVIZ_SFDP_LAYOUT;
use crate::snapshot::GraphSnapshot;

#[test]
fn layered_rank_uses_seed_context_and_bgp_state_not_names() {
    let upstream = key("opaque-peer");
    let owned_bgp = key("serial-a");
    let edge_border = key("serial-b");
    let secondary_seed = key("serial-seed-c");
    let core = key("serial-c");
    let customer = key("serial-d");
    let graph = NetworkGraph {
        nodes: vec![
            node(&upstream),
            collected_node(&owned_bgp, true),
            collected_node(&edge_border, true),
            collected_node(&secondary_seed, true),
            node(&core),
            node(&customer),
        ],
        edges: vec![edge(&edge_border, &core), edge(&core, &customer)],
    };
    let options = DotExportOptions {
        root_node: Some(edge_border.to_string()),
        seed_nodes: vec![edge_border.to_string(), secondary_seed.to_string()],
        owned_bgp_nodes: vec![owned_bgp.to_string()],
        ..DotExportOptions::for_layout(GRAPHVIZ_LAYERED_LAYOUT)
    };

    assert_eq!(
        graphviz_rank(&"bgp:198.51.100.1".to_owned().into(), &graph, &options),
        GRAPHVIZ_RANK_UPSTREAM
    );
    assert_eq!(graphviz_rank(&owned_bgp, &graph, &options), GRAPHVIZ_RANK_OWNED_BGP);
    assert_eq!(graphviz_rank(&edge_border, &graph, &options), GRAPHVIZ_RANK_EDGE_BORDER);
    assert_eq!(
        graphviz_rank(&secondary_seed, &graph, &options),
        GRAPHVIZ_RANK_EDGE_BORDER
    );
    assert_eq!(graphviz_rank(&core, &graph, &options), GRAPHVIZ_RANK_CORE_OSPF);
    assert_eq!(graphviz_rank(&customer, &graph, &options), GRAPHVIZ_RANK_CUSTOMER);
}

#[test]
fn sfdp_output_anchors_all_seed_nodes_to_edge_border_rank() {
    let root = key("root-seed");
    let secondary_seed = key("secondary-seed");
    let graph = NetworkGraph {
        nodes: vec![collected_node(&root, false), collected_node(&secondary_seed, false)],
        edges: Vec::new(),
    };
    let options = DotExportOptions {
        root_node: Some(root.to_string()),
        seed_nodes: vec![root.to_string(), secondary_seed.to_string()],
        ..DotExportOptions::for_layout(GRAPHVIZ_SFDP_LAYOUT)
    };

    let dot = graph.to_graphviz_dot_with_options(&options);

    assert!(dot.contains("\"root-seed\" -> \"__sfdp_rank_anchor_2\" [style=invis"));
    assert!(dot.contains("\"secondary-seed\" -> \"__sfdp_rank_anchor_2\" [style=invis"));
}

#[test]
fn recursive_radial_layout_uses_horizontal_kind_sections() {
    let upstream = key("bgp:198.51.100.1");
    let owned_bgp = key("serial-a");
    let border = key("serial-b");
    let core = key("serial-c");
    let customer = key("serial-d");
    let nested_customer = key("serial-e");
    let graph = NetworkGraph {
        nodes: vec![
            node(&upstream),
            collected_node(&owned_bgp, true),
            collected_node(&border, true),
            node(&core),
            node(&customer),
            node(&nested_customer),
        ],
        edges: vec![
            edge(&owned_bgp, &upstream),
            edge(&border, &core),
            edge(&core, &customer),
            edge(&customer, &nested_customer),
        ],
    };
    let graphviz_edges = collapsed_graphviz_edges(&graph.edges, &graph);
    let visible_nodes = graph.nodes.iter().map(|node| node.key.clone()).collect::<BTreeSet<_>>();
    let options = DotExportOptions {
        root_node: Some(border.to_string()),
        owned_bgp_nodes: vec![owned_bgp.to_string()],
        ..DotExportOptions::for_layout(GRAPHVIZ_RECURSIVE_RADIAL_LAYOUT)
    };

    let positions = recursive_radial_positions(&graph, &graphviz_edges, &visible_nodes, &options);

    assert_eq!(positions[&upstream].1, GRAPHVIZ_SECTION_UPSTREAM_Y);
    assert_eq!(positions[&owned_bgp].1, GRAPHVIZ_SECTION_OWNED_BGP_Y);
    assert_eq!(positions[&border].1, GRAPHVIZ_SECTION_BORDER_Y);
    assert_eq!(positions[&core].1, GRAPHVIZ_SECTION_CORE_Y);
    assert_eq!(positions[&customer].1, GRAPHVIZ_SECTION_CUSTOMER_Y);
    assert_eq!(positions[&nested_customer].1, GRAPHVIZ_SECTION_CUSTOMER_Y);
    assert!(
        (positions[&customer].0 - positions[&nested_customer].0).abs() >= GRAPHVIZ_SECTION_NODE_SPACING,
        "customer nodes in the same horizontal section should keep minimum spacing"
    );
}

#[test]
fn radio_chain_uses_uncapped_downstream_depth() {
    let border = key("serial-border");
    let radio_a = key("serial-radio-a");
    let radio_b = key("serial-radio-b");
    let remote_router = key("serial-remote-router");
    let graph = NetworkGraph {
        nodes: vec![
            collected_named_node(&border, "Rt_Border", DeviceRole::CoreRouter),
            collected_named_node(&radio_a, "Orbitel-QI23", DeviceRole::Unknown),
            collected_named_node(&radio_b, "QI23-ESCmusica", DeviceRole::Unknown),
            collected_named_node(&remote_router, "Rt_ESCmusica", DeviceRole::CustomerRouter),
        ],
        edges: vec![
            edge(&border, &radio_a),
            edge(&radio_a, &radio_b),
            edge(&radio_b, &remote_router),
        ],
    };
    let options = DotExportOptions {
        root_node: Some(border.to_string()),
        ..DotExportOptions::for_layout(GRAPHVIZ_SFDP_LAYOUT)
    };

    assert_eq!(graphviz_rank(&radio_a, &graph, &options), GRAPHVIZ_RANK_CORE_OSPF);
    assert_eq!(graphviz_rank(&radio_b, &graph, &options), GRAPHVIZ_RANK_CUSTOMER);
    assert_eq!(
        graphviz_rank(&remote_router, &graph, &options),
        GRAPHVIZ_RANK_CUSTOMER + 1
    );
    assert!(is_radio_name("Orbitel-QI23"));
    assert!(is_radio_name("QI23-ESCmusica"));
    assert!(is_radio_name("radio-sonnata-orbitel"));
    assert!(!is_radio_name("rt-sonnata"));
    assert!(!is_radio_name("RT-ORBITEL-BORDERv2"));
}

#[test]
fn sfdp_output_adds_invisible_radio_chain_constraints() {
    let border = key("border");
    let radio_a = key("serial-radio-a");
    let radio_b = key("serial-radio-b");
    let graph = NetworkGraph {
        nodes: vec![
            collected_named_node(&border, "Rt_Border", DeviceRole::CoreRouter),
            collected_named_node(&radio_a, "Orbitel-QI23", DeviceRole::Unknown),
            collected_named_node(&radio_b, "QI23-ESCmusica", DeviceRole::Unknown),
        ],
        edges: vec![edge(&border, &radio_a), edge(&border, &radio_b)],
    };
    let options = DotExportOptions {
        root_node: Some(border.to_string()),
        ..DotExportOptions::for_layout(GRAPHVIZ_SFDP_LAYOUT)
    };

    let dot = graph.to_graphviz_dot_with_options(&options);

    assert!(dot.contains("\"serial-radio-a\" -> \"serial-radio-b\" [style=invis, weight=140, len=0.65]"));
}

#[test]
fn graph_adds_wireless_edge_from_collected_radio_neighbor_evidence() {
    let router = key("serial-router");
    let radio = key("serial-radio");
    let mut snapshot_router = graph_snapshot(&router, "Rt_Sonata", DeviceRole::CustomerRouter);
    let mut snapshot_radio = graph_snapshot(&radio, "Sonata-Orbitel", DeviceRole::Unknown);
    snapshot_router.target_address = SocketAddr::new("10.100.0.220".parse().unwrap(), 8728);
    snapshot_radio.target_address = SocketAddr::new("10.100.0.230".parse().unwrap(), 8728);

    let snapshots = vec![snapshot_router, snapshot_radio].into_boxed_slice();
    let graph = build_graph_with_neighbor_evidence(
        snapshots.as_ref(),
        [InferredNeighborEvidence {
            neighbor: graph_neighbor("ether1", "bridge", "10.100.0.230", "Sonata-Orbitel"),
            local_node: router,
            local_interface: Some("ether1".parse().unwrap()),
        }],
        [],
    );

    assert_eq!(graph.edges.len(), 1);
    assert!(graph.edges[0].is_wireless());
    assert_eq!(graph.link_kind(&graph.edges[0]), LinkKind::Wireless);
}

#[test]
fn graph_does_not_add_wireless_edge_from_plain_neighbor_evidence() {
    let router_a = key("serial-router-a");
    let router_b = key("serial-router-b");
    let mut snapshot_a = graph_snapshot(&router_a, "Rt_A", DeviceRole::CoreRouter);
    let mut snapshot_b = graph_snapshot(&router_b, "Rt_B", DeviceRole::CustomerRouter);
    snapshot_a.target_address = SocketAddr::new("10.0.0.1".parse().unwrap(), 8728);
    snapshot_b.target_address = SocketAddr::new("10.0.0.2".parse().unwrap(), 8728);

    let snapshots = vec![snapshot_a, snapshot_b].into_boxed_slice();
    let graph = build_graph_with_neighbor_evidence(
        snapshots.as_ref(),
        [InferredNeighborEvidence {
            neighbor: graph_neighbor("ether1", "ether2", "10.0.0.2", "Rt_B"),
            local_node: router_a,
            local_interface: Some("ether1".parse().unwrap()),
        }],
        [],
    );

    assert!(!graph.edges.iter().any(TopologyLink::is_wireless));
}

#[test]
fn graph_does_not_turn_shared_location_tokens_into_wireless_cliques() {
    let radio_a = key("serial-radio-a");
    let radio_b = key("serial-radio-b");
    let mut snapshot_a = graph_snapshot(&radio_a, "Unicred-Patio", DeviceRole::Unknown);
    let mut snapshot_b = graph_snapshot(&radio_b, "CTC_INFRAERO-PATIO", DeviceRole::Unknown);
    snapshot_a.target_address = SocketAddr::new("10.100.0.77".parse().unwrap(), 8728);
    snapshot_b.target_address = SocketAddr::new("10.100.0.149".parse().unwrap(), 8728);

    let snapshots = vec![snapshot_a, snapshot_b].into_boxed_slice();
    let graph = build_graph_with_neighbor_evidence(
        snapshots.as_ref(),
        [InferredNeighborEvidence {
            neighbor: graph_neighbor("bridge", "wlan1", "10.100.0.149", "CTC_INFRAERO-PATIO"),
            local_node: radio_a,
            local_interface: Some("bridge".parse().unwrap()),
        }],
        [],
    );

    assert!(!graph.edges.iter().any(TopologyLink::is_wireless));
}

#[test]
#[allow(clippy::large_stack_arrays)]
fn one_sided_legacy_registration_creates_confidence_95_wireless_edge() {
    let ap = key("serial-ap");
    let station = key("serial-station");
    let mut snapshot_ap = graph_snapshot(&ap, "Orbitel-Sonata", DeviceRole::Radio);
    let mut snapshot_station = graph_snapshot(&station, "Sonata-Orbitel", DeviceRole::Radio);
    add_interface(&mut snapshot_ap, "wlan1", "00:11:22:33:44:01", "wlan");
    add_interface(&mut snapshot_station, "wlan1", "00:11:22:33:44:02", "wlan");
    snapshot_ap
        .snapshot
        .interface
        .wireless_registrations
        .data
        .push(WirelessRegistration {
            interface: Some("wlan1".parse().unwrap()),
            mac_address: Some("00:11:22:33:44:02".parse().unwrap()),
            dot1x_port_enabled: Some(false),
            ..WirelessRegistration::default()
        });

    let graph = build_graph(&[snapshot_ap, snapshot_station], []);

    assert_eq!(graph.edges.len(), 1);
    assert!(graph.edges[0].is_registration_wireless());
    assert!(!graph.edges[0].is_heuristic_wireless());
    assert_eq!(graph.edges[0].confidence, 95);
    assert_eq!(graph.link_kind(&graph.edges[0]), LinkKind::Wireless);
}

#[test]
#[allow(clippy::large_stack_arrays)]
fn reciprocal_mixed_stack_registration_collapses_to_confidence_100() {
    let legacy = key("serial-legacy");
    let wifi = key("serial-wifi");
    let mut snapshot_legacy = graph_snapshot(&legacy, "Orbitel-Sonata", DeviceRole::Radio);
    let mut snapshot_wifi = graph_snapshot(&wifi, "Sonata-Orbitel", DeviceRole::Radio);
    add_interface(&mut snapshot_legacy, "wlan1", "00:11:22:33:44:01", "wlan");
    add_interface(&mut snapshot_wifi, "wifi1", "00:11:22:33:44:02", "wifi");
    snapshot_legacy
        .snapshot
        .interface
        .wireless_registrations
        .data
        .push(WirelessRegistration {
            interface: Some("wlan1".parse().unwrap()),
            mac_address: Some("00:11:22:33:44:02".parse().unwrap()),
            ..WirelessRegistration::default()
        });
    snapshot_wifi
        .snapshot
        .interface
        .wifi_registrations
        .data
        .push(WifiRegistration {
            interface: Some("wifi1".parse().unwrap()),
            mac_address: Some("00:11:22:33:44:01".parse().unwrap()),
            authorized: Some(false),
            ..WifiRegistration::default()
        });

    let graph = build_graph(&[snapshot_legacy, snapshot_wifi], []);

    assert_eq!(graph.edges.len(), 1);
    assert!(graph.edges[0].is_registration_wireless());
    assert_eq!(graph.edges[0].confidence, 100);
}

#[test]
#[allow(clippy::large_stack_arrays)]
fn duplicate_peer_mac_does_not_create_registration_edge_or_anonymous_node() {
    let ap = key("serial-ap");
    let station_a = key("serial-station-a");
    let station_b = key("serial-station-b");
    let mut snapshot_ap = graph_snapshot(&ap, "Orbitel-Sonata", DeviceRole::Radio);
    let mut snapshot_a = graph_snapshot(&station_a, "Sonata-Orbitel", DeviceRole::Radio);
    let mut snapshot_b = graph_snapshot(&station_b, "Backup-Orbitel", DeviceRole::Radio);
    add_interface(&mut snapshot_ap, "wlan1", "00:11:22:33:44:01", "wlan");
    add_interface(&mut snapshot_a, "wlan1", "00:11:22:33:44:02", "wlan");
    add_interface(&mut snapshot_b, "wlan1", "00:11:22:33:44:02", "wlan");
    snapshot_ap
        .snapshot
        .interface
        .wireless_registrations
        .data
        .push(WirelessRegistration {
            interface: Some("wlan1".parse().unwrap()),
            mac_address: Some("00:11:22:33:44:02".parse().unwrap()),
            ..WirelessRegistration::default()
        });

    let graph = build_graph(&[snapshot_ap, snapshot_a, snapshot_b], []);

    assert!(!graph.edges.iter().any(TopologyLink::is_registration_wireless));
    assert_eq!(graph.nodes.len(), 3);
}

#[test]
fn successfully_collected_empty_wifi_table_suppresses_name_fallback() {
    let router = key("serial-router");
    let radio = key("serial-radio");
    let mut snapshot_router = graph_snapshot(&router, "Rt_Sonata", DeviceRole::CustomerRouter);
    let mut snapshot_radio = graph_snapshot(&radio, "Sonata-Orbitel", DeviceRole::Radio);
    snapshot_router.target_address = SocketAddr::new("10.100.0.220".parse().unwrap(), 8728);
    snapshot_radio.target_address = SocketAddr::new("10.100.0.230".parse().unwrap(), 8728);
    add_interface(&mut snapshot_radio, "wifi1", "00:11:22:33:44:02", "wifi");

    let snapshots = vec![snapshot_router, snapshot_radio].into_boxed_slice();
    let graph = build_graph_with_neighbor_evidence(
        snapshots.as_ref(),
        [InferredNeighborEvidence {
            neighbor: graph_neighbor("ether1", "wifi1", "10.100.0.230", "Sonata-Orbitel"),
            local_node: router,
            local_interface: Some("ether1".parse().unwrap()),
        }],
        [],
    );

    assert!(!graph.edges.iter().any(TopologyLink::is_wireless));
}

#[test]
fn unresolved_live_wifi_registration_allows_name_fallback_without_anonymous_node() {
    let router = key("serial-router");
    let radio = key("serial-radio");
    let mut snapshot_router = graph_snapshot(&router, "Rt_Sonata", DeviceRole::CustomerRouter);
    let mut snapshot_radio = graph_snapshot(&radio, "Sonata-Orbitel", DeviceRole::Radio);
    snapshot_router.target_address = SocketAddr::new("10.100.0.220".parse().unwrap(), 8728);
    snapshot_radio.target_address = SocketAddr::new("10.100.0.230".parse().unwrap(), 8728);
    add_interface(&mut snapshot_radio, "wifi1", "00:11:22:33:44:02", "wifi");
    snapshot_radio
        .snapshot
        .interface
        .wifi_registrations
        .data
        .push(WifiRegistration {
            interface: Some("wifi1".parse().unwrap()),
            mac_address: Some("00:11:22:33:44:99".parse().unwrap()),
            ..WifiRegistration::default()
        });

    let snapshots = vec![snapshot_router, snapshot_radio].into_boxed_slice();
    let graph = build_graph_with_neighbor_evidence(
        snapshots.as_ref(),
        [InferredNeighborEvidence {
            neighbor: graph_neighbor("ether1", "wifi1", "10.100.0.230", "Sonata-Orbitel"),
            local_node: router,
            local_interface: Some("ether1".parse().unwrap()),
        }],
        [],
    );

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
    assert!(graph.edges[0].is_heuristic_wireless());
    assert_eq!(graph.edges[0].confidence, 70);
}

#[test]
#[allow(clippy::large_stack_arrays)]
fn reciprocal_mndp_on_radio_ethernet_creates_physical_attachment() {
    let router = key("serial-router");
    let radio = key("serial-radio");
    let mut snapshot_router = graph_snapshot(&router, "rt-sonnata", DeviceRole::CustomerRouter);
    let mut snapshot_radio = graph_snapshot(&radio, "radio-sonnata-orbitel", DeviceRole::Radio);
    add_interface(&mut snapshot_router, "ether1", "6C:3B:6B:32:42:45", "ether");
    add_interface(&mut snapshot_radio, "ether1", "CC:2D:E0:77:0B:86", "ether");
    add_interface(&mut snapshot_radio, "wlan1", "CC:2D:E0:77:0B:87", "wlan");
    add_neighbor(
        &mut snapshot_router,
        "ether1",
        "bridge",
        "10.100.0.230",
        "radio-sonnata-orbitel",
        "CC:2D:E0:77:0B:87",
    );
    add_neighbor(
        &mut snapshot_radio,
        "ether1",
        "ether1",
        "10.100.0.220",
        "rt-sonnata",
        "6C:3B:6B:32:42:45",
    );

    let graph = build_graph(&[snapshot_router, snapshot_radio], []);
    let attachment = graph
        .edges
        .iter()
        .find(|edge| edge.is_mndp_attachment())
        .expect("reciprocal MNDP on the radio Ethernet port should create an attachment");

    assert_eq!(attachment.local_node, radio);
    assert_eq!(
        attachment.local_interface.as_ref().map(ToString::to_string).as_deref(),
        Some("ether1")
    );
    assert_eq!(attachment.remote_node, router);
    assert_eq!(
        attachment.remote_interface.as_ref().map(ToString::to_string).as_deref(),
        Some("ether1")
    );
    assert_eq!(attachment.confidence, 100);
    assert_eq!(graph.link_kind(attachment), LinkKind::Management);
}

#[test]
#[allow(clippy::large_stack_arrays)]
fn mndp_seen_on_radio_wireless_interface_does_not_create_attachment() {
    let router = key("serial-router");
    let radio = key("serial-radio");
    let mut snapshot_router = graph_snapshot(&router, "rt-orbitel", DeviceRole::CustomerRouter);
    let mut snapshot_radio = graph_snapshot(&radio, "radio-sonnata-orbitel", DeviceRole::Radio);
    add_interface(&mut snapshot_router, "ether1", "00:11:22:33:44:01", "ether");
    add_interface(&mut snapshot_radio, "ether1", "00:11:22:33:44:02", "ether");
    add_interface(&mut snapshot_radio, "wlan1", "00:11:22:33:44:03", "wlan");
    add_neighbor(
        &mut snapshot_router,
        "ether1",
        "wlan1",
        "10.100.0.230",
        "radio-sonnata-orbitel",
        "00:11:22:33:44:03",
    );
    add_neighbor(
        &mut snapshot_radio,
        "wlan1",
        "ether1",
        "10.100.0.147",
        "rt-orbitel",
        "00:11:22:33:44:01",
    );

    let graph = build_graph(&[snapshot_router, snapshot_radio], []);

    assert!(!graph.edges.iter().any(TopologyLink::is_mndp_attachment));
}

#[test]
#[allow(clippy::large_stack_arrays)]
fn ambiguous_reciprocal_mndp_radio_ethernet_neighbors_do_not_create_attachments() {
    let router_a = key("serial-router-a");
    let router_b = key("serial-router-b");
    let radio = key("serial-radio");
    let mut snapshot_a = graph_snapshot(&router_a, "rt-sonnata", DeviceRole::CustomerRouter);
    let mut snapshot_b = graph_snapshot(&router_b, "switch-sonnata", DeviceRole::Switch);
    let mut snapshot_radio = graph_snapshot(&radio, "radio-sonnata-orbitel", DeviceRole::Radio);
    add_interface(&mut snapshot_a, "ether1", "00:11:22:33:44:01", "ether");
    add_interface(&mut snapshot_b, "ether1", "00:11:22:33:44:02", "ether");
    add_interface(&mut snapshot_radio, "ether1", "00:11:22:33:44:03", "ether");
    add_neighbor(
        &mut snapshot_a,
        "ether1",
        "ether1",
        "10.100.0.230",
        "radio-sonnata-orbitel",
        "00:11:22:33:44:03",
    );
    add_neighbor(
        &mut snapshot_b,
        "ether1",
        "ether1",
        "10.100.0.230",
        "radio-sonnata-orbitel",
        "00:11:22:33:44:03",
    );
    add_neighbor(
        &mut snapshot_radio,
        "ether1",
        "ether1",
        "10.100.0.220",
        "rt-sonnata",
        "00:11:22:33:44:01",
    );
    add_neighbor(
        &mut snapshot_radio,
        "ether1",
        "ether1",
        "10.100.0.221",
        "switch-sonnata",
        "00:11:22:33:44:02",
    );

    let graph = build_graph(&[snapshot_a, snapshot_b, snapshot_radio], []);

    assert!(!graph.edges.iter().any(TopologyLink::is_mndp_attachment));
}

#[test]
fn graphviz_replaces_direct_l3_shortcut_with_unique_radio_chain_only() {
    let router_a = key("router-a");
    let radio_a = key("radio-a");
    let radio_b = key("radio-b");
    let router_b = key("router-b");
    let graph = NetworkGraph {
        nodes: vec![
            collected_named_node(&router_a, "RtBorder", DeviceRole::CoreRouter),
            collected_named_node(&radio_a, "Orbitel-Sonata", DeviceRole::Radio),
            collected_named_node(&radio_b, "Sonata-Orbitel", DeviceRole::Unknown),
            collected_named_node(&router_b, "RtSonata", DeviceRole::CustomerRouter),
        ],
        edges: vec![
            evidence_edge(&router_a, &router_b, "l3"),
            evidence_edge(&router_a, &radio_a, "l3"),
            evidence_edge(&radio_a, &radio_b, "wireless-registration"),
            evidence_edge(&radio_b, &router_b, "mndp-attachment"),
        ],
    };
    let options = DotExportOptions {
        hide_link_tables: true,
        ..DotExportOptions::default()
    };

    let dot = graph.to_graphviz_dot_with_options(&options);

    assert!(!dot.contains("\"router-a\" -> \"router-b\""));
    assert!(dot.contains("\"router-a\" -> \"radio-a\""));
    assert!(dot.contains("\"radio-a\" -> \"radio-b\""));
    assert!(dot.contains("\"radio-b\" -> \"router-b\""));
    assert!(graph.edges.iter().any(|edge| {
        edge.is_l3()
            && ((edge.local_node == router_a && edge.remote_node == router_b)
                || (edge.local_node == router_b && edge.remote_node == router_a))
    }));
}

#[test]
fn graphviz_ignores_longer_radio_detours_from_the_unique_shortest_chain() {
    let router_a = key("router-a");
    let radio_a = key("radio-a");
    let radio_b = key("radio-b");
    let detour_radio = key("detour-radio");
    let router_b = key("router-b");
    let graph = NetworkGraph {
        nodes: vec![
            collected_named_node(&router_a, "RtBorder", DeviceRole::CoreRouter),
            collected_named_node(&radio_a, "Orbitel-Sonata", DeviceRole::Radio),
            collected_named_node(&radio_b, "Sonata-Orbitel", DeviceRole::Radio),
            collected_named_node(&detour_radio, "Sonata-Patio", DeviceRole::Radio),
            collected_named_node(&router_b, "RtSonata", DeviceRole::CustomerRouter),
        ],
        edges: vec![
            evidence_edge(&router_a, &router_b, "l3"),
            evidence_edge(&router_a, &radio_a, "l3"),
            evidence_edge(&radio_a, &radio_b, "wireless-registration"),
            evidence_edge(&radio_b, &router_b, "mndp-attachment"),
            evidence_edge(&radio_b, &detour_radio, "wireless-registration"),
            evidence_edge(&detour_radio, &router_b, "route"),
        ],
    };
    let options = DotExportOptions {
        hide_link_tables: true,
        ..DotExportOptions::default()
    };

    let dot = graph.to_graphviz_dot_with_options(&options);

    assert!(!dot.contains("\"router-a\" -> \"router-b\""));
    assert!(dot.contains("\"router-a\" -> \"radio-a\""));
    assert!(dot.contains("\"radio-a\" -> \"radio-b\""));
    assert!(dot.contains("\"radio-b\" -> \"router-b\""));
}

#[test]
fn graphviz_keeps_direct_l3_shortcut_when_radio_path_is_ambiguous() {
    let router_a = key("router-a");
    let radio_a = key("radio-a");
    let radio_b = key("radio-b");
    let radio_c = key("radio-c");
    let radio_d = key("radio-d");
    let router_b = key("router-b");
    let graph = NetworkGraph {
        nodes: vec![
            collected_named_node(&router_a, "RtBorder", DeviceRole::CoreRouter),
            collected_named_node(&radio_a, "Orbitel-Sonata", DeviceRole::Radio),
            collected_named_node(&radio_b, "Sonata-Orbitel", DeviceRole::Radio),
            collected_named_node(&radio_c, "Orbitel-Backup", DeviceRole::Radio),
            collected_named_node(&radio_d, "Backup-Orbitel", DeviceRole::Radio),
            collected_named_node(&router_b, "RtSonata", DeviceRole::CustomerRouter),
        ],
        edges: vec![
            evidence_edge(&router_a, &router_b, "l3"),
            evidence_edge(&router_a, &radio_a, "l3"),
            evidence_edge(&radio_a, &radio_b, "wireless-registration"),
            evidence_edge(&radio_b, &router_b, "l3"),
            evidence_edge(&router_a, &radio_c, "route"),
            evidence_edge(&radio_c, &radio_d, "wireless-registration"),
            evidence_edge(&radio_d, &router_b, "l3"),
        ],
    };
    let options = DotExportOptions {
        hide_link_tables: true,
        ..DotExportOptions::default()
    };

    let dot = graph.to_graphviz_dot_with_options(&options);

    assert!(dot.contains("\"router-a\" -> \"router-b\""));
}

#[test]
fn graphviz_keeps_direct_l3_shortcut_when_registration_link_is_missing() {
    let router_a = key("router-a");
    let radio_a = key("radio-a");
    let radio_b = key("radio-b");
    let router_b = key("router-b");
    let graph = NetworkGraph {
        nodes: vec![
            collected_named_node(&router_a, "RtBorder", DeviceRole::CoreRouter),
            collected_named_node(&radio_a, "Orbitel-Sonata", DeviceRole::Radio),
            collected_named_node(&radio_b, "Sonata-Orbitel", DeviceRole::Radio),
            collected_named_node(&router_b, "RtSonata", DeviceRole::CustomerRouter),
        ],
        edges: vec![
            evidence_edge(&router_a, &router_b, "l3"),
            evidence_edge(&router_a, &radio_a, "l3"),
            evidence_edge(&radio_a, &radio_b, "wireless"),
            evidence_edge(&radio_b, &router_b, "l3"),
        ],
    };
    let options = DotExportOptions {
        hide_link_tables: true,
        ..DotExportOptions::default()
    };

    let dot = graph.to_graphviz_dot_with_options(&options);

    assert!(dot.contains("\"router-a\" -> \"router-b\""));
}

#[test]
fn typed_radial_nested_children_stay_behind_parent_at_every_depth() {
    let root = key("root");
    let child = key("child");
    let grandchild_a = key("grandchild-a");
    let grandchild_b = key("grandchild-b");
    let great_grandchild = key("great-grandchild");
    let graph = NetworkGraph {
        nodes: vec![
            node(&root),
            node(&child),
            node(&grandchild_a),
            node(&grandchild_b),
            node(&great_grandchild),
        ],
        edges: vec![
            edge(&root, &child),
            edge(&child, &grandchild_a),
            edge(&child, &grandchild_b),
            edge(&grandchild_a, &great_grandchild),
        ],
    };
    let graphviz_edges = collapsed_graphviz_edges(&graph.edges, &graph);
    let visible_nodes = graph.nodes.iter().map(|node| node.key.clone()).collect::<BTreeSet<_>>();
    let positions = typed_radial_positions(&graph, &graphviz_edges, &visible_nodes, Some(root.as_str()));

    let root_position = positions[&root];
    let child_position = positions[&child];
    let child_vector = vector_between(root_position, child_position);

    for descendant in [&grandchild_a, &grandchild_b, &great_grandchild] {
        let descendant_vector = vector_between(child_position, positions[descendant]);
        assert!(
            dot(child_vector, descendant_vector) > 0.0,
            "{} should be placed outward behind child",
            descendant
        );
    }

    let grandchild_vector = vector_between(child_position, positions[&grandchild_a]);
    let great_grandchild_vector = vector_between(positions[&grandchild_a], positions[&great_grandchild]);
    assert!(
        dot(grandchild_vector, great_grandchild_vector) > 0.0,
        "great-grandchild should be placed outward behind grandchild-a"
    );
}

#[test]
fn graphviz_root_node_uses_seed_colors() {
    let root = key("root");
    let graph = NetworkGraph {
        nodes: vec![collected_node(&root, false)],
        edges: Vec::new(),
    };
    let options = DotExportOptions {
        root_node: Some(root.to_string()),
        ..DotExportOptions::default()
    };

    let dot = graph.to_graphviz_dot_with_options(&options);

    assert!(dot.contains("fillcolor=\"#ede9fe\", color=\"#7c3aed\""));
}

#[test]
fn graphviz_seed_nodes_use_seed_colors() {
    let seed_a = key("seed-a");
    let seed_b = key("seed-b");
    let ordinary = key("ordinary");
    let graph = NetworkGraph {
        nodes: vec![
            collected_node(&seed_a, false),
            collected_node(&seed_b, false),
            collected_node(&ordinary, false),
        ],
        edges: Vec::new(),
    };
    let options = DotExportOptions {
        root_node: Some(seed_a.to_string()),
        seed_nodes: vec![seed_a.to_string(), seed_b.to_string()],
        ..DotExportOptions::default()
    };

    let dot = graph.to_graphviz_dot_with_options(&options);

    assert_eq!(dot.matches("fillcolor=\"#ede9fe\", color=\"#7c3aed\"").count(), 2);
    assert!(
        dot.contains("\"ordinary\" [label=\"ordinary\", style=\"filled\", fillcolor=\"#e0f2fe\", color=\"#0369a1\"")
    );
}

fn key(value: &str) -> TopologyNodeKey {
    TopologyNodeKey::from(value.to_owned())
}

fn node(key: &TopologyNodeKey) -> NetworkNode {
    NetworkNode {
        key: key.clone(),
        status: NetworkNodeStatus::Inferred,
        role: None,
        target_address: None,
        management_addresses: Vec::new(),
        snapshot: None,
        inferred: None,
    }
}

fn collected_node(key: &TopologyNodeKey, has_bgp_state: bool) -> NetworkNode {
    let bgp_connections = if has_bgp_state {
        vec![BgpConnection::default()]
    } else {
        Vec::new()
    };
    NetworkNode {
        key: key.clone(),
        status: NetworkNodeStatus::Collected,
        role: Some(DeviceRole::Unknown),
        target_address: Some(SocketAddr::new("192.0.2.1".parse().unwrap(), 8728)),
        management_addresses: Vec::new(),
        snapshot: Some(RouterOsSnapshot {
            system: SystemSnapshot {
                identity: Identity::default().into(),
                resource: Resource::default().into(),
                routerboard: Routerboard::default().into(),
                ..SystemSnapshot::default()
            },
            routing: RoutingSnapshot {
                bgp_connections: bgp_connections.into(),
                ..RoutingSnapshot::default()
            },
            ..RouterOsSnapshot::default()
        }),
        inferred: None,
    }
}

fn collected_named_node(key: &TopologyNodeKey, name: &str, role: DeviceRole) -> NetworkNode {
    NetworkNode {
        key: key.clone(),
        status: NetworkNodeStatus::Collected,
        role: Some(role),
        target_address: Some(SocketAddr::new("192.0.2.1".parse().unwrap(), 8728)),
        management_addresses: Vec::new(),
        snapshot: Some(RouterOsSnapshot {
            system: SystemSnapshot {
                identity: Identity {
                    name: Some(name.to_owned()),
                }
                .into(),
                resource: Resource::default().into(),
                routerboard: Routerboard {
                    serial_number: Some(key.to_string()),
                    ..Routerboard::default()
                }
                .into(),
                ..SystemSnapshot::default()
            },
            ..RouterOsSnapshot::default()
        }),
        inferred: None,
    }
}

fn graph_snapshot(key: &TopologyNodeKey, name: &str, role: DeviceRole) -> GraphSnapshot {
    GraphSnapshot {
        target_address: SocketAddr::new("192.0.2.1".parse().unwrap(), 8728),
        management_addresses: Vec::new(),
        role,
        snapshot: collected_named_node(key, name, role).snapshot.unwrap(),
    }
}

fn add_interface(snapshot: &mut GraphSnapshot, name: &str, mac_address: &str, interface_type: &str) {
    snapshot.snapshot.interface.interfaces.data.push(Interface {
        name: Some(name.parse().unwrap()),
        mac_address: Some(mac_address.parse().unwrap()),
        interface_type: Some(interface_type.parse().unwrap()),
        ..Interface::default()
    });
}

fn add_neighbor(
    snapshot: &mut GraphSnapshot,
    local_interface: &str,
    remote_interface: &str,
    address: &str,
    identity: &str,
    mac_address: &str,
) {
    snapshot.snapshot.ip.neighbors.data.push(Neighbor {
        interface: Some(local_interface.parse().unwrap()),
        interface_name: Some(remote_interface.parse().unwrap()),
        address: Some(address.parse().unwrap()),
        mac_address: Some(mac_address.parse().unwrap()),
        identity: Some(identity.to_owned()),
        board: Some("RouterBOARD".to_owned()),
        ..Neighbor::default()
    });
}

fn edge(local_node: &TopologyNodeKey, remote_node: &TopologyNodeKey) -> TopologyLink {
    TopologyLink {
        local_node: local_node.clone(),
        local_interface: None,
        remote_node: remote_node.clone(),
        remote_interface: None,
        discovered_by: vec![DiscoveryProtocol::Unknown("management".to_owned())],
        confidence: 100,
    }
}

fn evidence_edge(local_node: &TopologyNodeKey, remote_node: &TopologyNodeKey, evidence: &str) -> TopologyLink {
    TopologyLink {
        local_node: local_node.clone(),
        local_interface: None,
        remote_node: remote_node.clone(),
        remote_interface: None,
        discovered_by: vec![DiscoveryProtocol::Unknown(evidence.to_owned())],
        confidence: 95,
    }
}

fn graph_neighbor(local_interface: &str, remote_interface: &str, address: &str, identity: &str) -> Neighbor {
    Neighbor {
        interface: Some(local_interface.parse().unwrap()),
        interface_name: Some(remote_interface.parse().unwrap()),
        address: Some(address.parse().unwrap()),
        identity: Some(identity.to_owned()),
        board: Some("SXTsq 5 ac".to_owned()),
        ..Neighbor::default()
    }
}

fn vector_between(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    (to.0 - from.0, to.1 - from.1)
}

fn dot(left: (f64, f64), right: (f64, f64)) -> f64 {
    left.0 * right.0 + left.1 * right.1
}
