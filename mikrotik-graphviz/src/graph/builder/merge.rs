use core::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use mikrotik_types::device::TopologyNodeKey;
use mikrotik_types::primitives::interface::InterfaceName;
use mikrotik_types::topology::TopologyLink;

/// Merge multiple evidence sources for the same node/interface relationship.
pub(super) fn merge_duplicate_edges(edges: Vec<TopologyLink>) -> Vec<TopologyLink> {
    let mut merged = BTreeMap::<EdgeMergeKey, TopologyLink>::new();
    for edge in edges {
        let (key, edge) = canonical_edge(edge);
        merged
            .entry(key)
            .and_modify(|existing| {
                existing.confidence = existing.confidence.max(edge.confidence);
                for protocol in &edge.discovered_by {
                    if !existing.discovered_by.contains(protocol) {
                        existing.discovered_by.push(protocol.clone());
                    }
                }
            })
            .or_insert(edge);
    }
    merged.into_values().collect()
}

/// Upgrade edges with reciprocal evidence to full confidence.
pub(super) fn mark_reciprocal_edges(edges: &mut [TopologyLink]) {
    let ordinary_pairs = edges
        .iter()
        .filter(|edge| !edge.is_wireless())
        .map(|edge| (edge.local_node.clone(), edge.remote_node.clone()))
        .collect::<BTreeSet<_>>();
    let registration_pairs = edges
        .iter()
        .filter(|edge| edge.is_registration_wireless())
        .map(|edge| (edge.local_node.clone(), edge.remote_node.clone()))
        .collect::<BTreeSet<_>>();

    for edge in edges {
        let reverse = (edge.remote_node.clone(), edge.local_node.clone());
        if (!edge.is_wireless() && ordinary_pairs.contains(&reverse))
            || (edge.is_registration_wireless() && registration_pairs.contains(&reverse))
        {
            edge.confidence = 100;
        }
    }
}

/// Sort edges deterministically.
pub(super) fn edge_order(left: &TopologyLink, right: &TopologyLink) -> Ordering {
    left.local_node
        .cmp(&right.local_node)
        .then_with(|| left.remote_node.cmp(&right.remote_node))
        .then_with(|| left.local_interface.cmp(&right.local_interface))
        .then_with(|| left.remote_interface.cmp(&right.remote_interface))
}

/// Stable key for merging duplicate edge evidence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EdgeMergeKey {
    /// Canonically ordered left node.
    left_node: TopologyNodeKey,
    /// Interface on the canonically ordered left node.
    left_interface: Option<InterfaceName>,
    /// Canonically ordered right node.
    right_node: TopologyNodeKey,
    /// Interface on the canonically ordered right node.
    right_interface: Option<InterfaceName>,
}

/// Return a deterministic orientation for one edge.
fn canonical_edge(edge: TopologyLink) -> (EdgeMergeKey, TopologyLink) {
    if edge.local_node <= edge.remote_node {
        (
            EdgeMergeKey {
                left_node: edge.local_node.clone(),
                left_interface: edge.local_interface.clone(),
                right_node: edge.remote_node.clone(),
                right_interface: edge.remote_interface.clone(),
            },
            edge,
        )
    } else {
        let edge = TopologyLink {
            local_node: edge.remote_node,
            local_interface: edge.remote_interface,
            remote_node: edge.local_node,
            remote_interface: edge.local_interface,
            discovered_by: edge.discovered_by,
            confidence: edge.confidence,
        };
        (
            EdgeMergeKey {
                left_node: edge.local_node.clone(),
                left_interface: edge.local_interface.clone(),
                right_node: edge.remote_node.clone(),
                right_interface: edge.remote_interface.clone(),
            },
            edge,
        )
    }
}
