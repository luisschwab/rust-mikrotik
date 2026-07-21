use core::net::SocketAddr;

use mikrotik_types::device::DeviceRole;
use mikrotik_types::device::RouterOsSnapshot;
use mikrotik_types::device::TopologyNodeKey;
use mikrotik_types::topology::InferredDevice;
use mikrotik_types::topology::InferredDeviceFailure;
use mikrotik_types::topology::NetworkNode;
use mikrotik_types::topology::NetworkNodeStatus;

use super::escape::push_dot_escaped;
use super::model::NetworkGraph;
use crate::constants::GRAPHVIZ_API_REFUSED_LABEL;
use crate::constants::GRAPHVIZ_COLLECTED_DEVICE_FILL;
use crate::constants::GRAPHVIZ_COLLECTED_DEVICE_STROKE;
use crate::constants::GRAPHVIZ_INFERRED_DEVICE_FILL;
use crate::constants::GRAPHVIZ_INFERRED_DEVICE_STROKE;
use crate::constants::GRAPHVIZ_SEED_DEVICE_FILL;
use crate::constants::GRAPHVIZ_SEED_DEVICE_STROKE;
use crate::constants::GRAPHVIZ_UNKNOWN_STROKE;
use crate::constants::GRAPHVIZ_UNREACHABLE_DEVICE_FILL;
use crate::constants::GRAPHVIZ_WRONG_CREDENTIALS_FILL;
use crate::constants::GRAPHVIZ_WRONG_CREDENTIALS_LABEL;
use crate::constants::GRAPHVIZ_WRONG_CREDENTIALS_STROKE;
use crate::options::DotExportOptions;

/// Add one node statement to Graphviz DOT.
pub(super) fn push_graphviz_node(
    dot: &mut String,
    node: &NetworkNode,
    position: Option<(f64, f64)>,
    options: &DotExportOptions,
    node_url_prefix: Option<&str>,
    indent: &str,
) {
    let failure = graphviz_node_failure(node);
    let mut label = node.graphviz_label();
    if failure == Some(InferredDeviceFailure::WrongCredentials) {
        label.push('\n');
        label.push_str(GRAPHVIZ_WRONG_CREDENTIALS_LABEL);
    } else if failure == Some(InferredDeviceFailure::ApiRefused) {
        label.push('\n');
        label.push_str(GRAPHVIZ_API_REFUSED_LABEL);
    } else if failure == Some(InferredDeviceFailure::Unreachable) {
        label.push('\n');
        label.push_str("ERROR: UNREACHABLE");
    }
    let (fill_color, stroke_color) = graphviz_node_colors(node, options);
    let style = if graphviz_node_is_failure(node) {
        "filled,dashed"
    } else {
        "filled"
    };
    dot.push_str(indent);
    dot.push('"');
    push_dot_escaped(dot, node.key.as_str());
    dot.push_str("\" [label=\"");
    push_dot_escaped(dot, &label);
    dot.push_str("\", style=\"");
    dot.push_str(style);
    dot.push_str("\", fillcolor=\"");
    dot.push_str(fill_color);
    dot.push_str("\", color=\"");
    dot.push_str(stroke_color);
    dot.push('"');
    if let (true, Some(prefix)) = (node.snapshot.is_some(), node_url_prefix) {
        dot.push_str(", URL=\"");
        push_dot_escaped(dot, prefix);
        push_dot_escaped(dot, node.key.as_str());
        dot.push_str("\", target=\"_top\"");
    }
    if let Some(tooltip) = graphviz_node_tooltip(node) {
        if node.snapshot.is_none() || node_url_prefix.is_none() {
            dot.push_str(", URL=\"#\"");
        }
        dot.push_str(", tooltip=\"");
        push_dot_escaped(dot, &tooltip);
        dot.push('"');
    }
    if let Some((x, y)) = position {
        dot.push_str(", pos=\"");
        dot.push_str(&format!("{x:.2},{y:.2}!"));
        dot.push_str("\", pin=true");
    }
    dot.push_str("];\n");
}

/// Return whether one node key is a configured seed.
pub(super) fn graphviz_key_is_seed(node: &TopologyNodeKey, options: &DotExportOptions) -> bool {
    options.root_node.as_deref() == Some(node.as_str()) || options.seed_nodes.iter().any(|seed| seed == node.as_str())
}

/// Return a collected device role for one node.
pub(super) fn graphviz_node_role(node: &TopologyNodeKey, graph: &NetworkGraph) -> Option<DeviceRole> {
    graph
        .nodes
        .iter()
        .find(|candidate| &candidate.key == node)
        .and_then(NetworkNode::role)
}

/// Return the rendered label for one graph node.
pub(super) fn graphviz_node_label(node: &TopologyNodeKey, graph: &NetworkGraph) -> Option<String> {
    graph
        .nodes
        .iter()
        .find(|candidate| &candidate.key == node)
        .map(NetworkNode::label)
}

/// Return whether one node represents a failed crawl.
fn graphviz_node_is_failure(node: &NetworkNode) -> bool {
    graphviz_node_failure(node).is_some()
}

/// Return one inferred node failure, when present.
fn graphviz_node_failure(node: &NetworkNode) -> Option<InferredDeviceFailure> {
    node.inferred.as_ref().and_then(|inferred| inferred.failure)
}

/// Return Graphviz fill/stroke colors for one node.
fn graphviz_node_colors(node: &NetworkNode, options: &DotExportOptions) -> (&'static str, &'static str) {
    if graphviz_node_is_seed(node, options) {
        (GRAPHVIZ_SEED_DEVICE_FILL, GRAPHVIZ_SEED_DEVICE_STROKE)
    } else if graphviz_node_failure(node) == Some(InferredDeviceFailure::WrongCredentials) {
        (GRAPHVIZ_WRONG_CREDENTIALS_FILL, GRAPHVIZ_WRONG_CREDENTIALS_STROKE)
    } else if matches!(
        graphviz_node_failure(node),
        Some(InferredDeviceFailure::ApiRefused | InferredDeviceFailure::Unreachable)
    ) {
        (GRAPHVIZ_UNREACHABLE_DEVICE_FILL, GRAPHVIZ_UNKNOWN_STROKE)
    } else {
        match node.status {
            NetworkNodeStatus::Collected => (GRAPHVIZ_COLLECTED_DEVICE_FILL, GRAPHVIZ_COLLECTED_DEVICE_STROKE),
            NetworkNodeStatus::Inferred => (GRAPHVIZ_INFERRED_DEVICE_FILL, GRAPHVIZ_INFERRED_DEVICE_STROKE),
        }
    }
}

/// Return whether one node should render with seed styling.
fn graphviz_node_is_seed(node: &NetworkNode, options: &DotExportOptions) -> bool {
    graphviz_key_is_seed(&node.key, options)
}

/// Build SVG tooltip text for one graph node.
fn graphviz_node_tooltip(node: &NetworkNode) -> Option<String> {
    if let Some(snapshot) = &node.snapshot {
        return Some(collected_node_tooltip(snapshot, node.target_address));
    }
    node.inferred.as_ref().map(inferred_node_tooltip)
}

/// Build tooltip text for a collected router.
fn collected_node_tooltip(snapshot: &RouterOsSnapshot, target_address: Option<SocketAddr>) -> String {
    let routerboard = &snapshot.system.routerboard;
    let mut tooltip = String::new();
    push_tooltip_line(
        &mut tooltip,
        "NAME",
        snapshot.system.identity.name.as_deref().unwrap_or("UNKNOWN"),
    );
    let target_address = target_address.map_or_else(|| "UNKNOWN".to_owned(), |address| address.to_string());
    push_tooltip_line(&mut tooltip, "MANAGEMENT IP", &target_address);
    push_tooltip_optional(&mut tooltip, "SERIAL", routerboard.serial_number.as_ref());
    tooltip
}

/// Build tooltip text for an inferred router.
fn inferred_node_tooltip(inferred: &InferredDevice) -> String {
    let mut tooltip = String::new();
    push_tooltip_line(&mut tooltip, "STATUS", "INFERRED");
    match inferred.failure {
        Some(InferredDeviceFailure::WrongCredentials) => push_tooltip_line(&mut tooltip, "ERROR", "BAD CREDENTIALS"),
        Some(InferredDeviceFailure::ApiRefused) => push_tooltip_line(&mut tooltip, "ERROR", "API REFUSED"),
        Some(InferredDeviceFailure::Unreachable) => push_tooltip_line(&mut tooltip, "ERROR", "UNREACHABLE"),
        None => {}
    }
    push_tooltip_optional(&mut tooltip, "NAME", inferred.identity.as_ref());
    push_tooltip_optional(
        &mut tooltip,
        "MANAGEMENT ADDRESS",
        inferred.management_address.map(|value| value.to_string()),
    );
    push_tooltip_optional(&mut tooltip, "BOARD", inferred.board.as_ref());
    push_tooltip_optional(&mut tooltip, "PLATFORM", inferred.platform.as_ref());
    push_tooltip_optional(
        &mut tooltip,
        "ROUTEROS VERSION",
        inferred.version.as_ref().map(ToString::to_string),
    );
    push_tooltip_optional(
        &mut tooltip,
        "MAC ADDRESS",
        inferred.mac_address.as_ref().map(ToString::to_string),
    );
    tooltip
}

/// Append a tooltip line when a value exists.
fn push_tooltip_optional(tooltip: &mut String, label: &str, value: Option<impl AsRef<str>>) {
    if let Some(value) = value {
        push_tooltip_line(tooltip, label, value.as_ref());
    }
}

/// Append one tooltip line.
fn push_tooltip_line(tooltip: &mut String, label: &str, value: &str) {
    if !tooltip.is_empty() {
        tooltip.push('\n');
    }
    tooltip.push_str(label);
    tooltip.push_str(": ");
    tooltip.push_str(value);
}
