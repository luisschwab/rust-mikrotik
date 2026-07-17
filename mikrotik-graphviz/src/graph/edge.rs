use std::collections::BTreeSet;
use std::collections::HashMap;

use mikrotik_types::abstractions::LinkKind;
use mikrotik_types::device::DeviceRole;
use mikrotik_types::device::TopologyNodeKey;
use mikrotik_types::primitives::interface::InterfaceName;
use mikrotik_types::topology::TopologyLink;

use super::address_index::GraphAddressIndex;
use super::endpoint::EdgeEndpointLabel;
use super::endpoint::edge_endpoint_labels;
use super::escape::push_dot_escaped;
use super::escape::push_html_escaped;
use super::model::NetworkGraph;
use super::node::graphviz_node_role;
use super::rank::graphviz_rank;
use super::style::GraphvizLinkStyle;
use super::style::graphviz_link_style;
use super::style::link_kind_label;
use crate::constants::GRAPHVIZ_EDGE_PEN_WIDTH;
use crate::constants::GRAPHVIZ_FONT;
use crate::constants::GRAPHVIZ_LINK_TABLE_CELL_PADDING;
use crate::constants::GRAPHVIZ_LINK_TABLE_FONT_SIZE;
use crate::constants::GRAPHVIZ_SFDP_EDGE_LENGTH;
use crate::options::DotExportOptions;

/// Collapsed visual edge used only for Graphviz output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GraphvizEdge {
    /// Local visual endpoint.
    pub(super) local_node: TopologyNodeKey,
    /// Local visual interface.
    pub(super) local_interface: Option<InterfaceName>,
    /// Remote visual endpoint.
    pub(super) remote_node: TopologyNodeKey,
    /// Remote visual interface.
    pub(super) remote_interface: Option<InterfaceName>,
    /// Visual link kind.
    pub(super) link_kind: LinkKind,
}

/// Add one Graphviz edge statement.
pub(super) fn push_graphviz_edge(
    dot: &mut String,
    index: usize,
    edge: &GraphvizEdge,
    graph: &NetworkGraph,
    address_index: &GraphAddressIndex,
    options: &DotExportOptions,
) {
    let (local, remote) = edge_endpoint_labels(edge, address_index, graph);

    if options.hide_link_tables || (local.is_none() && remote.is_none()) {
        let tooltip = graphviz_link_tooltip(local.as_ref(), remote.as_ref(), edge.link_kind);
        push_graphviz_direct_edge(dot, edge, tooltip.as_deref(), options);
        return;
    }

    let link_kind = edge.link_kind;
    let link_style = graphviz_link_style(edge);

    let label_node = format!("__link_{index}");
    dot.push_str("  \"");
    push_dot_escaped(dot, &label_node);
    dot.push_str("\" [shape=plain, margin=0, label=<");
    push_graphviz_link_table(dot, local.as_ref(), remote.as_ref(), link_kind, link_style);
    dot.push('>');
    if let Some(tooltip) = graphviz_link_tooltip(local.as_ref(), remote.as_ref(), link_kind) {
        dot.push_str(", tooltip=\"");
        push_dot_escaped(dot, &tooltip);
        dot.push('"');
    }
    dot.push_str("];\n");

    dot.push_str("  \"");
    push_dot_escaped(dot, edge.local_node.as_str());
    if options.is_layered_layout() {
        dot.push_str("\":e -> \"");
    } else {
        dot.push_str("\" -> \"");
    }
    push_dot_escaped(dot, &label_node);
    if options.is_layered_layout() {
        dot.push_str("\":w");
    } else {
        dot.push('"');
    }
    dot.push_str(" [weight=4, color=\"");
    dot.push_str(link_style.stroke);
    dot.push_str("\", penwidth=");
    dot.push_str(GRAPHVIZ_EDGE_PEN_WIDTH);
    push_graphviz_sfdp_edge_length(dot, options);
    dot.push_str(", tooltip=\"\"];\n");

    dot.push_str("  \"");
    push_dot_escaped(dot, &label_node);
    if options.is_layered_layout() {
        dot.push_str("\":e -> \"");
    } else {
        dot.push_str("\" -> \"");
    }
    push_dot_escaped(dot, edge.remote_node.as_str());
    if options.is_layered_layout() {
        dot.push_str("\":w");
    } else {
        dot.push('"');
    }
    dot.push_str(" [weight=4, color=\"");
    dot.push_str(link_style.stroke);
    dot.push_str("\", penwidth=");
    dot.push_str(GRAPHVIZ_EDGE_PEN_WIDTH);
    push_graphviz_sfdp_edge_length(dot, options);
    dot.push_str(", tooltip=\"\"];\n");
}

/// Collapse reciprocal directed graph edges into one visual edge.
pub(super) fn collapsed_graphviz_edges(edges: &[TopologyLink], graph: &NetworkGraph) -> Vec<GraphvizEdge> {
    let mut reciprocal_interfaces = HashMap::new();
    let mut bgp_evidence = BTreeSet::new();
    for edge in edges {
        reciprocal_interfaces.insert(
            (edge.local_node.clone(), edge.remote_node.clone()),
            (edge.local_interface.clone(), edge.remote_interface.clone()),
        );
        if edge.is_bgp() {
            bgp_evidence.insert(ordered_pair(&edge.local_node, &edge.remote_node));
        }
    }

    let mut seen = BTreeSet::new();
    let mut collapsed = Vec::new();
    for edge in edges {
        let pair = ordered_pair(&edge.local_node, &edge.remote_node);
        if !seen.insert(pair) {
            continue;
        }

        let reciprocal = reciprocal_interfaces.get(&(edge.remote_node.clone(), edge.local_node.clone()));
        let (local_interface, remote_interface) = reciprocal.map_or_else(
            || (edge.local_interface.clone(), edge.remote_interface.clone()),
            |(reverse_local, reverse_remote)| {
                (
                    edge.local_interface.clone().or_else(|| reverse_remote.clone()),
                    edge.remote_interface.clone().or_else(|| reverse_local.clone()),
                )
            },
        );

        collapsed.push(GraphvizEdge {
            local_node: edge.local_node.clone(),
            local_interface,
            remote_node: edge.remote_node.clone(),
            remote_interface,
            link_kind: graph_link_kind_from_edge(edge, graph, &bgp_evidence),
        });
    }

    collapsed
}

/// Orient a visual edge so Graphviz can rank upstream -> customers top-to-bottom.
pub(super) fn orient_graphviz_edge(
    edge: GraphvizEdge,
    graph: &NetworkGraph,
    options: &DotExportOptions,
) -> GraphvizEdge {
    let local_rank = graphviz_rank(&edge.local_node, graph, options);
    let remote_rank = graphviz_rank(&edge.remote_node, graph, options);
    if local_rank <= remote_rank {
        edge
    } else {
        GraphvizEdge {
            local_node: edge.remote_node,
            local_interface: edge.remote_interface,
            remote_node: edge.local_node,
            remote_interface: edge.local_interface,
            link_kind: edge.link_kind,
        }
    }
}

/// Classify one visual link from collected device roles.
pub(super) fn graphviz_link_kind(edge: &GraphvizEdge, graph: &NetworkGraph) -> LinkKind {
    if edge.link_kind == LinkKind::Wireless {
        return LinkKind::Wireless;
    }
    let local = graphviz_node_role(&edge.local_node, graph);
    let remote = graphviz_node_role(&edge.remote_node, graph);
    if matches!(local, Some(DeviceRole::BgpRouter)) || matches!(remote, Some(DeviceRole::BgpRouter)) {
        LinkKind::Bgp
    } else if is_internal_node(&edge.local_node, local) || is_internal_node(&edge.remote_node, remote) {
        LinkKind::Internal
    } else if matches!(local, Some(DeviceRole::CustomerRouter)) || matches!(remote, Some(DeviceRole::CustomerRouter)) {
        LinkKind::Customer
    } else {
        LinkKind::Unknown
    }
}

/// Add a direct Graphviz edge.
fn push_graphviz_direct_edge(dot: &mut String, edge: &GraphvizEdge, tooltip: Option<&str>, options: &DotExportOptions) {
    let link_style = graphviz_link_style(edge);
    dot.push_str("  \"");
    push_dot_escaped(dot, edge.local_node.as_str());
    dot.push_str("\" -> \"");
    push_dot_escaped(dot, edge.remote_node.as_str());
    dot.push('"');
    dot.push_str(" [color=\"");
    dot.push_str(link_style.stroke);
    dot.push_str("\", penwidth=");
    dot.push_str(GRAPHVIZ_EDGE_PEN_WIDTH);
    push_graphviz_sfdp_edge_length(dot, options);
    if let Some(tooltip) = tooltip {
        dot.push_str(", tooltip=\"");
        push_dot_escaped(dot, tooltip);
        dot.push('"');
    }
    dot.push(']');
    dot.push_str(";\n");
}

/// Add SFDP edge length tuning to visible edges.
fn push_graphviz_sfdp_edge_length(dot: &mut String, options: &DotExportOptions) {
    if options.is_sfdp_layout() {
        dot.push_str(", len=");
        dot.push_str(GRAPHVIZ_SFDP_EDGE_LENGTH);
    }
}

/// Build the SVG tooltip text for a link.
fn graphviz_link_tooltip(
    local: Option<&EdgeEndpointLabel>,
    remote: Option<&EdgeEndpointLabel>,
    link_kind: LinkKind,
) -> Option<String> {
    if local.is_none() && remote.is_none() {
        return None;
    }

    let mut tooltip = format!("LINK TYPE: {}", link_kind_label(link_kind).to_ascii_uppercase());
    if let Some(local) = local {
        push_graphviz_endpoint_tooltip(&mut tooltip, local);
    }
    if let Some(remote) = remote {
        push_graphviz_endpoint_tooltip(&mut tooltip, remote);
    }
    Some(tooltip)
}

/// Append one endpoint to a link tooltip.
fn push_graphviz_endpoint_tooltip(tooltip: &mut String, endpoint: &EdgeEndpointLabel) {
    tooltip.push_str("\n\nNAME: ");
    tooltip.push_str(&endpoint.device);
    tooltip.push_str("\nINTERFACE: ");
    tooltip.push_str(&endpoint.interface);
    tooltip.push_str("\nADDRESSES: [");
    tooltip.push_str(&endpoint.addresses.join(", "));
    tooltip.push(']');
}

/// Add a Graphviz HTML table for one link label.
fn push_graphviz_link_table(
    dot: &mut String,
    local: Option<&EdgeEndpointLabel>,
    remote: Option<&EdgeEndpointLabel>,
    link_kind: LinkKind,
    link_style: GraphvizLinkStyle,
) {
    dot.push_str("<TABLE BORDER=\"1\" CELLBORDER=\"1\" CELLSPACING=\"0\" CELLPADDING=\"");
    dot.push_str(GRAPHVIZ_LINK_TABLE_CELL_PADDING);
    dot.push_str("\" COLOR=\"");
    dot.push_str(link_style.stroke);
    dot.push_str("\" BGCOLOR=\"");
    dot.push_str(link_style.fill);
    dot.push_str("\">");
    if let Some(local) = local {
        push_graphviz_link_table_row(dot, local, link_kind);
    }
    if let Some(remote) = remote {
        push_graphviz_link_table_row(dot, remote, link_kind);
    }
    dot.push_str("</TABLE>");
}

/// Add one Graphviz HTML table row for a link endpoint.
fn push_graphviz_link_table_row(dot: &mut String, endpoint: &EdgeEndpointLabel, link_kind: LinkKind) {
    let primary_address = endpoint.addresses.first().map_or("", String::as_str);
    push_graphviz_link_table_cells(
        dot,
        &endpoint.device,
        &endpoint.interface,
        primary_address,
        link_kind_label(link_kind),
    );

    for address in endpoint.addresses.iter().skip(1) {
        push_graphviz_link_table_cells(dot, "", "", address, "");
    }
}

/// Add one Graphviz HTML table row.
fn push_graphviz_link_table_cells(dot: &mut String, device: &str, interface: &str, address: &str, link_type: &str) {
    dot.push_str("<TR><TD ALIGN=\"LEFT\"><FONT FACE=\"");
    push_html_escaped(dot, GRAPHVIZ_FONT);
    dot.push_str("\" POINT-SIZE=\"");
    dot.push_str(GRAPHVIZ_LINK_TABLE_FONT_SIZE);
    dot.push_str("\">");
    push_graphviz_cell_value(dot, device);
    dot.push_str("</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"");
    push_html_escaped(dot, GRAPHVIZ_FONT);
    dot.push_str("\" POINT-SIZE=\"");
    dot.push_str(GRAPHVIZ_LINK_TABLE_FONT_SIZE);
    dot.push_str("\">");
    push_graphviz_cell_value(dot, interface);
    dot.push_str("</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"");
    push_html_escaped(dot, GRAPHVIZ_FONT);
    dot.push_str("\" POINT-SIZE=\"");
    dot.push_str(GRAPHVIZ_LINK_TABLE_FONT_SIZE);
    dot.push_str("\">");
    push_graphviz_cell_value(dot, address);
    dot.push_str("</FONT></TD><TD ALIGN=\"LEFT\"><FONT FACE=\"");
    push_html_escaped(dot, GRAPHVIZ_FONT);
    dot.push_str("\" POINT-SIZE=\"");
    dot.push_str(GRAPHVIZ_LINK_TABLE_FONT_SIZE);
    dot.push_str("\">");
    push_graphviz_cell_value(dot, link_type);
    dot.push_str("</FONT></TD></TR>");
}

/// Append a non-empty Graphviz HTML table cell value.
fn push_graphviz_cell_value(dot: &mut String, value: &str) {
    if value.is_empty() {
        dot.push_str("&#160;");
    } else {
        push_html_escaped(dot, value);
    }
}

/// Return true for core/internal routers that are not the edge hub.
fn is_internal_node(node: &TopologyNodeKey, role: Option<DeviceRole>) -> bool {
    role == Some(DeviceRole::CoreRouter) && !node.as_str().contains("EDGE")
}

/// Return a deterministic unordered device pair key.
fn ordered_pair(left: &TopologyNodeKey, right: &TopologyNodeKey) -> (TopologyNodeKey, TopologyNodeKey) {
    if left <= right {
        (left.clone(), right.clone())
    } else {
        (right.clone(), left.clone())
    }
}

/// Classify a collapsed visual edge from underlying edge evidence and device roles.
fn graph_link_kind_from_edge(
    edge: &TopologyLink,
    graph: &NetworkGraph,
    bgp_evidence: &BTreeSet<(TopologyNodeKey, TopologyNodeKey)>,
) -> LinkKind {
    if bgp_evidence.contains(&ordered_pair(&edge.local_node, &edge.remote_node)) {
        LinkKind::Bgp
    } else {
        graph.link_kind(edge)
    }
}
