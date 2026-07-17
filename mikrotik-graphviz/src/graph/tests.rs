#![allow(
    clippy::large_stack_arrays,
    reason = "snapshot fixtures intentionally exercise complete typed endpoint payloads"
)]

use core::net::SocketAddr;
use std::collections::BTreeSet;

use mikrotik_types::abstractions::LinkKind;
use mikrotik_types::api::ip::Neighbor;
use mikrotik_types::api::routing::BgpConnection;
use mikrotik_types::device::DeviceRole;
use mikrotik_types::device::RouterOsSnapshot;
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
            system: mikrotik_types::device::SystemSnapshot {
                identity: mikrotik_types::api::system::Identity::default().into(),
                resource: mikrotik_types::api::system::Resource::default().into(),
                routerboard: mikrotik_types::api::system::Routerboard::default().into(),
                ..mikrotik_types::device::SystemSnapshot::default()
            },
            routing: mikrotik_types::device::RoutingSnapshot {
                bgp_connections: bgp_connections.into(),
                ..mikrotik_types::device::RoutingSnapshot::default()
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
            system: mikrotik_types::device::SystemSnapshot {
                identity: mikrotik_types::api::system::Identity {
                    name: Some(name.to_owned()),
                }
                .into(),
                resource: mikrotik_types::api::system::Resource::default().into(),
                routerboard: mikrotik_types::api::system::Routerboard {
                    serial_number: Some(key.to_string()),
                    ..mikrotik_types::api::system::Routerboard::default()
                }
                .into(),
                ..mikrotik_types::device::SystemSnapshot::default()
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
