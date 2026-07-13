//! Device graph types and Graphviz export.
//!
//! The graph is built from collected
//! [`mikrotik_types::device::DeviceSnapshot`] values plus neighbor evidence for
//! devices that were seen but not crawled. It can be serialized as structured
//! data or rendered to DOT for external Graphviz tools.

use std::collections::BTreeMap;

use mikrotik_types::abstractions::LinkKind;
use mikrotik_types::device::DeviceKey;
use mikrotik_types::device::DeviceSnapshot;
use mikrotik_types::topology::TopologyLink;

/// Interface address indexing for link labels.
mod address_index;
/// Graph construction from collected evidence.
mod builder;
/// Invisible Graphviz layout constraints.
mod constraints;
/// Graphviz visual edge rendering.
mod edge;
/// Link endpoint label preparation.
mod endpoint;
/// Graphviz DOT escaping helpers.
mod escape;
/// Graphviz export-level helpers.
mod export;
/// Deterministic graph layout algorithms.
mod layout;
/// Serializable graph container.
pub mod model;
/// Graphviz node rendering.
mod node;
/// Graph rank and radio/backhaul classification helpers.
mod rank;
/// Graphviz presentation styles.
mod style;

use self::address_index::GraphAddressIndex;
pub use self::builder::build_graph;
pub use self::builder::build_graph_with_neighbor_evidence;
use self::builder::target_address_host;
use self::constraints::push_graphviz_ranks;
use self::constraints::push_graphviz_sfdp_radio_chain_constraints;
use self::constraints::push_graphviz_sfdp_rank_anchors;
use self::edge::GraphvizEdge;
use self::edge::collapsed_graphviz_edges;
use self::edge::graphviz_link_kind;
use self::edge::orient_graphviz_edge;
use self::edge::push_graphviz_edge;
use self::export::graphviz_visible_nodes;
use self::export::push_graphviz_graph_attributes;
use self::layout::recursive_radial_positions;
use self::layout::typed_radial_positions;
use self::model::NetworkGraph;
use self::node::push_graphviz_node;
use self::rank::graphviz_node_has_bgp_state;
use self::rank::graphviz_rank;
use crate::constants::GRAPHVIZ_COLLECTED_DEVICE_FILL;
use crate::constants::GRAPHVIZ_COLLECTED_DEVICE_STROKE;
use crate::constants::GRAPHVIZ_DEVICE_FONT;
use crate::constants::GRAPHVIZ_DEVICE_NODE_HEIGHT;
use crate::constants::GRAPHVIZ_DEVICE_NODE_WIDTH;
use crate::constants::GRAPHVIZ_EDGE_COLOR;
use crate::constants::GRAPHVIZ_EDGE_ENDPOINT_MARKER;
use crate::constants::GRAPHVIZ_EDGE_ENDPOINT_MARKER_SIZE;
use crate::constants::GRAPHVIZ_EDGE_PEN_WIDTH;
use crate::constants::GRAPHVIZ_FONT;
use crate::constants::GRAPHVIZ_FONT_SIZE;
use crate::constants::GRAPHVIZ_NODE_MARGIN;
use crate::options::DotExportOptions;
use crate::options::LinkFilter;

/// Upstream or opaque external BGP peer rank.
pub const GRAPHVIZ_RANK_UPSTREAM: u8 = 0;
/// Owned BGP router rank.
pub const GRAPHVIZ_RANK_OWNED_BGP: u8 = 1;
/// Edge or border router rank.
pub const GRAPHVIZ_RANK_EDGE_BORDER: u8 = 2;
/// Core or OSPF router rank.
pub const GRAPHVIZ_RANK_CORE_OSPF: u8 = 3;
/// Downstream customer router rank.
pub const GRAPHVIZ_RANK_CUSTOMER: u8 = 4;
/// Unknown or uncategorized downstream rank.
pub const GRAPHVIZ_RANK_UNKNOWN: u8 = 5;

impl NetworkGraph {
    /// Build a graph from collected device snapshots.
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_snapshots(snapshots: Vec<DeviceSnapshot>) -> Self {
        build_graph(&snapshots, [])
    }

    /// Build a graph from borrowed collected device snapshots.
    #[must_use]
    pub fn from_snapshot_refs(snapshots: &[DeviceSnapshot]) -> Self {
        build_graph(snapshots, [])
    }

    /// Render this graph as Graphviz DOT.
    #[must_use]
    pub fn to_graphviz_dot(&self) -> String {
        self.to_graphviz_dot_with_options(&DotExportOptions::default())
    }

    /// Render this graph as Graphviz DOT with explicit export options.
    #[must_use]
    pub fn to_graphviz_dot_with_options(&self, options: &DotExportOptions) -> String {
        let address_index = GraphAddressIndex::new(self);
        let mut dot = String::from("digraph mikrotik_topology {\n");
        push_graphviz_graph_attributes(&mut dot, options);
        dot.push_str("  node [shape=cylinder, style=filled, fixedsize=shape, fontname=\"");
        dot.push_str(GRAPHVIZ_DEVICE_FONT);
        dot.push_str("\", fontsize=");
        dot.push_str(GRAPHVIZ_FONT_SIZE);
        dot.push_str(", width=");
        dot.push_str(GRAPHVIZ_DEVICE_NODE_WIDTH);
        dot.push_str(", height=");
        dot.push_str(GRAPHVIZ_DEVICE_NODE_HEIGHT);
        dot.push_str(", margin=\"");
        dot.push_str(GRAPHVIZ_NODE_MARGIN);
        dot.push_str("\", fillcolor=\"");
        dot.push_str(GRAPHVIZ_COLLECTED_DEVICE_FILL);
        dot.push_str("\", color=\"");
        dot.push_str(GRAPHVIZ_COLLECTED_DEVICE_STROKE);
        dot.push_str("\"];\n");
        dot.push_str("  edge [fontname=\"");
        dot.push_str(GRAPHVIZ_FONT);
        dot.push_str("\", fontsize=");
        dot.push_str(GRAPHVIZ_FONT_SIZE);
        dot.push_str(", dir=both, arrowhead=");
        dot.push_str(GRAPHVIZ_EDGE_ENDPOINT_MARKER);
        dot.push_str(", arrowtail=");
        dot.push_str(GRAPHVIZ_EDGE_ENDPOINT_MARKER);
        dot.push_str(", arrowsize=");
        dot.push_str(GRAPHVIZ_EDGE_ENDPOINT_MARKER_SIZE);
        dot.push_str(", color=\"");
        dot.push_str(GRAPHVIZ_EDGE_COLOR);
        dot.push_str("\", penwidth=");
        dot.push_str(GRAPHVIZ_EDGE_PEN_WIDTH);
        dot.push_str("];\n");

        let graphviz_edges = collapsed_graphviz_edges(&self.edges, self);
        let graphviz_edges = graphviz_edges.into_iter().map(|edge| {
            if options.is_layered_layout() {
                orient_graphviz_edge(edge, self, options)
            } else {
                edge
            }
        });
        let graphviz_edges = graphviz_edges
            .filter(|edge| options.link_filter.includes(edge.link_kind))
            .collect::<Vec<_>>();
        let visible_nodes = graphviz_visible_nodes(self, &graphviz_edges, options);
        let node_positions = if options.is_recursive_radial_layout() {
            recursive_radial_positions(self, &graphviz_edges, &visible_nodes, options)
        } else if options.is_typed_radial_layout() {
            typed_radial_positions(self, &graphviz_edges, &visible_nodes, options.root_node.as_deref())
        } else {
            BTreeMap::new()
        };
        let node_url_prefix = options.node_url_prefix.as_deref();
        for node in &self.nodes {
            if visible_nodes.contains(&node.key) {
                push_graphviz_node(
                    &mut dot,
                    node,
                    node_positions.get(&node.key).copied(),
                    options,
                    node_url_prefix,
                    "  ",
                );
            }
        }
        if options.is_layered_layout() {
            push_graphviz_ranks(&mut dot, self, &visible_nodes, options);
        }
        if options.is_sfdp_layout() {
            push_graphviz_sfdp_rank_anchors(&mut dot, self, &visible_nodes, options);
            push_graphviz_sfdp_radio_chain_constraints(&mut dot, self, &visible_nodes);
        }
        for (index, edge) in graphviz_edges.iter().enumerate() {
            push_graphviz_edge(&mut dot, index, edge, self, &address_index, options);
        }

        dot.push_str("}\n");
        dot
    }

    /// Return the node key collected from one target address.
    #[must_use]
    pub fn node_key_for_target_address(&self, target_address: &str) -> Option<&DeviceKey> {
        let host = target_address_host(target_address).unwrap_or(target_address);
        self.nodes.iter().find_map(|node| {
            let snapshot = node.snapshot.as_ref()?;
            (snapshot.target_address.to_string() == target_address
                || snapshot
                    .management_addresses
                    .iter()
                    .any(|address| address.to_string() == host))
            .then_some(&node.key)
        })
    }

    /// Classify a graph edge using collected device roles and BGP evidence.
    #[must_use]
    pub fn link_kind(&self, edge: &TopologyLink) -> LinkKind {
        if edge.is_bgp() {
            return LinkKind::Bgp;
        }
        if edge.is_management() {
            return LinkKind::Management;
        }
        if edge.is_wireless() {
            return LinkKind::Wireless;
        }
        if edge.is_fallback() {
            return LinkKind::Fallback;
        }
        if edge.is_route() {
            return LinkKind::Route;
        }

        let visual = GraphvizEdge {
            local_node: edge.local_node.clone(),
            local_interface: edge.local_interface.clone(),
            remote_node: edge.remote_node.clone(),
            remote_interface: edge.remote_interface.clone(),
            link_kind: LinkKind::Unknown,
        };
        graphviz_link_kind(&visual, self)
    }

    /// Return graph edges included by a link filter.
    #[must_use]
    pub fn filtered_edges(&self, filter: LinkFilter) -> Vec<&TopologyLink> {
        self.edges
            .iter()
            .filter(|edge| filter.includes(self.link_kind(edge)))
            .collect()
    }
}

#[cfg(test)]
mod tests;
