use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;

use mikrotik_types::device::DeviceKey;
use mikrotik_types::device::DeviceRole;

use super::GRAPHVIZ_RANK_CORE_OSPF;
use super::GRAPHVIZ_RANK_CUSTOMER;
use super::GRAPHVIZ_RANK_EDGE_BORDER;
use super::GRAPHVIZ_RANK_OWNED_BGP;
use super::GRAPHVIZ_RANK_UNKNOWN;
use super::GRAPHVIZ_RANK_UPSTREAM;
use super::model::NetworkGraph;
use super::node::graphviz_key_is_seed;
use super::node::graphviz_node_label;
use super::node::graphviz_node_role;
use crate::options::DotExportOptions;

/// Return the top-to-bottom visual rank for one node.
pub(super) fn graphviz_rank(node: &DeviceKey, graph: &NetworkGraph, options: &DotExportOptions) -> u8 {
    if node.as_str().starts_with("bgp:") {
        return GRAPHVIZ_RANK_UPSTREAM;
    }
    if options
        .owned_bgp_nodes
        .iter()
        .any(|owned_node| owned_node == node.as_str())
        && graphviz_node_has_bgp_state(node, graph)
    {
        return GRAPHVIZ_RANK_OWNED_BGP;
    }
    if graphviz_key_is_seed(node, options) {
        return GRAPHVIZ_RANK_EDGE_BORDER;
    }
    let exact_downstream_rank = graphviz_downstream_rank_exact(node, graph, options);
    let is_radio_path = graphviz_is_radio_node(node, graph) || graphviz_has_radio_ancestor(node, graph, options);
    graphviz_node_role(node, graph).map_or_else(
        || {
            if is_radio_path {
                exact_downstream_rank.unwrap_or(GRAPHVIZ_RANK_UNKNOWN)
            } else {
                exact_downstream_rank.map_or(GRAPHVIZ_RANK_UNKNOWN, |rank| rank.min(GRAPHVIZ_RANK_CUSTOMER))
            }
        },
        |role| match role {
            DeviceRole::BgpRouter => GRAPHVIZ_RANK_OWNED_BGP,
            DeviceRole::CoreRouter => GRAPHVIZ_RANK_CORE_OSPF,
            DeviceRole::CustomerRouter => {
                if is_radio_path {
                    exact_downstream_rank
                        .unwrap_or(GRAPHVIZ_RANK_CUSTOMER)
                        .max(GRAPHVIZ_RANK_CUSTOMER)
                } else {
                    GRAPHVIZ_RANK_CUSTOMER
                }
            }
            DeviceRole::Switch | DeviceRole::Radio | DeviceRole::Unknown => {
                if is_radio_path {
                    exact_downstream_rank.unwrap_or(GRAPHVIZ_RANK_UNKNOWN)
                } else {
                    exact_downstream_rank.map_or(GRAPHVIZ_RANK_UNKNOWN, |rank| rank.min(GRAPHVIZ_RANK_CUSTOMER))
                }
            }
        },
    )
}

/// Return true when a collected node has BGP control-plane state.
pub(super) fn graphviz_node_has_bgp_state(node: &DeviceKey, graph: &NetworkGraph) -> bool {
    graph
        .nodes
        .iter()
        .find(|candidate| &candidate.key == node)
        .and_then(|candidate| candidate.snapshot.as_ref())
        .is_some_and(|snapshot| {
            !snapshot.bgp_sessions.is_empty() || !snapshot.bgp_connections.is_empty() || !snapshot.bgp_peers.is_empty()
        })
}

/// Return the uncapped downstream depth from the edge router, when the graph has one.
fn graphviz_downstream_rank_exact(node: &DeviceKey, graph: &NetworkGraph, options: &DotExportOptions) -> Option<u8> {
    let edge = options
        .root_node
        .as_deref()
        .map(|node| node.to_owned().into())
        .or_else(|| {
            graph
                .nodes
                .iter()
                .find(|candidate| graphviz_node_role(&candidate.key, graph) == Some(DeviceRole::CoreRouter))
                .map(|candidate| candidate.key.clone())
        })?;
    let mut ranks = BTreeMap::<DeviceKey, u8>::new();
    let mut queue = VecDeque::from([(edge, GRAPHVIZ_RANK_EDGE_BORDER)]);

    while let Some((current, rank)) = queue.pop_front() {
        if ranks.contains_key(&current) {
            continue;
        }
        ranks.insert(current.clone(), rank);

        for neighbor in graphviz_downstream_neighbors(&current, graph) {
            if !ranks.contains_key(&neighbor) {
                queue.push_back((neighbor, rank.saturating_add(1)));
            }
        }
    }

    ranks.get(node).copied()
}

/// Return whether one node sits downstream from a radio-like node.
fn graphviz_has_radio_ancestor(node: &DeviceKey, graph: &NetworkGraph, options: &DotExportOptions) -> bool {
    let Some(root) = options
        .root_node
        .as_deref()
        .map(|node| node.to_owned().into())
        .or_else(|| {
            graph
                .nodes
                .iter()
                .find(|candidate| graphviz_node_role(&candidate.key, graph) == Some(DeviceRole::CoreRouter))
                .map(|candidate| candidate.key.clone())
        })
    else {
        return false;
    };
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([(root, false)]);
    while let Some((current, has_radio_ancestor)) = queue.pop_front() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if &current == node {
            return has_radio_ancestor;
        }
        let next_has_radio_ancestor = has_radio_ancestor || graphviz_is_radio_node(&current, graph);
        for neighbor in graphviz_downstream_neighbors(&current, graph) {
            if !visited.contains(&neighbor) {
                queue.push_back((neighbor, next_has_radio_ancestor));
            }
        }
    }
    false
}

/// Return true for radio/backhaul device names that follow `<src>-<dst>`.
fn graphviz_is_radio_node(node: &DeviceKey, graph: &NetworkGraph) -> bool {
    graphviz_node_label(node, graph).is_some_and(|label| is_radio_name(&label))
}

/// Return true when a name looks like a point-to-point radio label.
pub(super) fn is_radio_name(label: &str) -> bool {
    radio_name_parts(label).is_some()
}

/// Return endpoint names for point-to-point radio labels.
pub(super) fn radio_name_parts(label: &str) -> Option<(&str, &str)> {
    let mut parts = label.split('-');
    let left = parts.next()?.trim();
    let right = parts.next()?.trim();
    if parts.next().is_some()
        || left.is_empty()
        || right.is_empty()
        || left.starts_with("Rt_")
        || left.starts_with("RT_")
        || left.eq_ignore_ascii_case("serial")
    {
        return None;
    }
    Some((left, right))
}

/// Return non-BGP visual neighbors used for downstream rank discovery.
fn graphviz_downstream_neighbors(node: &DeviceKey, graph: &NetworkGraph) -> Vec<DeviceKey> {
    graph
        .edges
        .iter()
        .filter_map(|edge| {
            let neighbor = if edge.local_node == *node {
                &edge.remote_node
            } else if edge.remote_node == *node {
                &edge.local_node
            } else {
                return None;
            };
            if graphviz_node_role(neighbor, graph) == Some(DeviceRole::BgpRouter) {
                None
            } else {
                Some(neighbor.clone())
            }
        })
        .collect()
}
