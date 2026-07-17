use std::collections::BTreeMap;
use std::collections::BTreeSet;

use mikrotik_types::device::TopologyNodeKey;
use mikrotik_types::topology::NetworkNode;

use super::escape::push_dot_escaped;
use super::layout::recursive_section_y;
use super::model::NetworkGraph;
use super::rank::graphviz_rank;
use super::rank::radio_name_parts;
use crate::constants::GRAPHVIZ_SFDP_RADIO_CHAIN_LENGTH;
use crate::constants::GRAPHVIZ_SFDP_RADIO_CHAIN_WEIGHT;
use crate::constants::GRAPHVIZ_SFDP_RANK_ANCHOR_LENGTH;
use crate::constants::GRAPHVIZ_SFDP_RANK_ANCHOR_WEIGHT;
use crate::options::DotExportOptions;

/// Add invisible rank constraints without drawing grouping boxes.
pub(super) fn push_graphviz_ranks(
    dot: &mut String,
    graph: &NetworkGraph,
    visible_nodes: &BTreeSet<TopologyNodeKey>,
    options: &DotExportOptions,
) {
    let max_rank = graph
        .nodes
        .iter()
        .filter(|node| visible_nodes.contains(&node.key))
        .map(|node| graphviz_rank(&node.key, graph, options))
        .max()
        .unwrap_or(0);
    for rank in 0..=max_rank {
        let nodes = graph
            .nodes
            .iter()
            .filter(|node| visible_nodes.contains(&node.key))
            .filter(|node| graphviz_rank(&node.key, graph, options) == rank)
            .collect::<Vec<_>>();
        if nodes.len() < 2 {
            continue;
        }

        dot.push_str("  { rank=same;");
        for node in nodes {
            dot.push_str(" \"");
            push_dot_escaped(dot, node.key.as_str());
            dot.push('"');
        }
        dot.push_str(" }\n");
    }
}

/// Add invisible semantic-rank anchors to bias SFDP into top-to-bottom rows.
pub(super) fn push_graphviz_sfdp_rank_anchors(
    dot: &mut String,
    graph: &NetworkGraph,
    visible_nodes: &BTreeSet<TopologyNodeKey>,
    options: &DotExportOptions,
) {
    let mut ranks = BTreeMap::<u8, Vec<&NetworkNode>>::new();
    for node in &graph.nodes {
        if visible_nodes.contains(&node.key) {
            ranks
                .entry(graphviz_rank(&node.key, graph, options))
                .or_default()
                .push(node);
        }
    }

    for (rank, nodes) in ranks {
        let anchor = format!("__sfdp_rank_anchor_{rank}");
        dot.push_str("  \"");
        push_dot_escaped(dot, &anchor);
        dot.push_str("\" [shape=point, label=\"\", width=0.01, height=0.01, style=invis, pos=\"0,");
        dot.push_str(&format!("{:.2}", recursive_section_y(rank)));
        dot.push_str("!\", pin=true];\n");
        for node in nodes {
            dot.push_str("  \"");
            push_dot_escaped(dot, node.key.as_str());
            dot.push_str("\" -> \"");
            push_dot_escaped(dot, &anchor);
            dot.push_str("\" [style=invis, weight=");
            dot.push_str(GRAPHVIZ_SFDP_RANK_ANCHOR_WEIGHT);
            dot.push_str(", len=");
            dot.push_str(GRAPHVIZ_SFDP_RANK_ANCHOR_LENGTH);
            dot.push_str("];\n");
        }
    }
}

/// Add invisible edges between radios that share location tokens.
pub(super) fn push_graphviz_sfdp_radio_chain_constraints(
    dot: &mut String,
    graph: &NetworkGraph,
    visible_nodes: &BTreeSet<TopologyNodeKey>,
) {
    let radios = graph
        .nodes
        .iter()
        .filter(|node| visible_nodes.contains(&node.key))
        .filter_map(|node| {
            let label = node.label();
            let (left, right) = radio_name_parts(&label)?;
            Some((node.key.clone(), left.to_ascii_lowercase(), right.to_ascii_lowercase()))
        })
        .collect::<Vec<_>>();

    let mut emitted = BTreeSet::new();
    for (left_key, left_a, left_b) in &radios {
        for (right_key, right_a, right_b) in &radios {
            if left_key == right_key {
                continue;
            }
            let shares_endpoint = left_a == right_a || left_a == right_b || left_b == right_a || left_b == right_b;
            if !shares_endpoint {
                continue;
            }
            if !emitted.insert(ordered_pair(left_key, right_key)) {
                continue;
            }
            dot.push_str("  \"");
            push_dot_escaped(dot, left_key.as_str());
            dot.push_str("\" -> \"");
            push_dot_escaped(dot, right_key.as_str());
            dot.push_str("\" [style=invis, weight=");
            dot.push_str(GRAPHVIZ_SFDP_RADIO_CHAIN_WEIGHT);
            dot.push_str(", len=");
            dot.push_str(GRAPHVIZ_SFDP_RADIO_CHAIN_LENGTH);
            dot.push_str("];\n");
        }
    }
}

/// Return a deterministic unordered device pair key.
fn ordered_pair(left: &TopologyNodeKey, right: &TopologyNodeKey) -> (TopologyNodeKey, TopologyNodeKey) {
    if left <= right {
        (left.clone(), right.clone())
    } else {
        (right.clone(), left.clone())
    }
}
