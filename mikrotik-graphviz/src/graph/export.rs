use std::collections::BTreeSet;

use mikrotik_types::device::TopologyNodeKey;

use super::edge::GraphvizEdge;
use super::escape::push_dot_escaped;
use super::model::NetworkGraph;
use crate::constants::GRAPHVIZ_SFDP_SPRING_CONSTANT;
use crate::options::DotExportOptions;

/// Return nodes that should be rendered for one Graphviz export.
pub(super) fn graphviz_visible_nodes(
    graph: &NetworkGraph,
    edges: &[GraphvizEdge],
    options: &DotExportOptions,
) -> BTreeSet<TopologyNodeKey> {
    let mut visible = graph.nodes.iter().map(|node| node.key.clone()).collect::<BTreeSet<_>>();
    for edge in edges {
        visible.insert(edge.local_node.clone());
        visible.insert(edge.remote_node.clone());
    }
    if let Some(root_node) = &options.root_node {
        visible.insert(root_node.clone().into());
    }
    if visible.is_empty() && options.root_node.is_none() {
        visible.extend(graph.nodes.iter().map(|node| node.key.clone()));
    }
    visible
}

/// Add graph-level Graphviz attributes.
pub(super) fn push_graphviz_graph_attributes(dot: &mut String, options: &DotExportOptions) {
    dot.push_str("  graph [layout=");
    push_dot_escaped(dot, options.graphviz_layout_engine());
    if options.is_layered_layout() {
        dot.push_str(", rankdir=");
        push_dot_escaped(dot, &options.rank_direction);
    }
    dot.push_str(", ranksep=\"");
    push_dot_escaped(dot, &options.rank_separation);
    dot.push('"');
    dot.push_str(", nodesep=");
    push_dot_escaped(dot, &options.node_separation);
    dot.push_str(", splines=");
    push_dot_escaped(dot, &options.splines);
    dot.push_str(", outputorder=");
    push_dot_escaped(dot, &options.output_order);
    dot.push_str(", overlap=");
    push_dot_escaped(dot, &options.overlap);
    if let Some(overlap_scaling) = &options.overlap_scaling {
        dot.push_str(", overlap_scaling=");
        push_dot_escaped(dot, overlap_scaling);
    }
    dot.push_str(", sep=\"");
    push_dot_escaped(dot, &options.separation);
    dot.push('"');
    if let Some(root_node) = &options.root_node {
        dot.push_str(", root=\"");
        push_dot_escaped(dot, root_node);
        dot.push('"');
    }
    if options.is_sfdp_layout() {
        dot.push_str(", K=");
        push_dot_escaped(dot, GRAPHVIZ_SFDP_SPRING_CONSTANT);
    }
    dot.push_str("];\n");
}
