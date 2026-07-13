use mikrotik_types::abstractions::LinkKind;

use super::edge::GraphvizEdge;
use crate::constants::GRAPHVIZ_BGP_FILL;
use crate::constants::GRAPHVIZ_BGP_STROKE;
use crate::constants::GRAPHVIZ_CUSTOMER_FILL;
use crate::constants::GRAPHVIZ_CUSTOMER_STROKE;
use crate::constants::GRAPHVIZ_INTERNAL_FILL;
use crate::constants::GRAPHVIZ_INTERNAL_STROKE;
use crate::constants::GRAPHVIZ_MANAGEMENT_FILL;
use crate::constants::GRAPHVIZ_MANAGEMENT_STROKE;
use crate::constants::GRAPHVIZ_ROUTE_FILL;
use crate::constants::GRAPHVIZ_ROUTE_STROKE;
use crate::constants::GRAPHVIZ_UNKNOWN_FILL;
use crate::constants::GRAPHVIZ_UNKNOWN_STROKE;
use crate::constants::GRAPHVIZ_WIRELESS_FILL;
use crate::constants::GRAPHVIZ_WIRELESS_STROKE;

/// Visual style for one Graphviz link label node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GraphvizLinkStyle {
    /// Cloud fill color.
    pub(super) fill: &'static str,
    /// Cloud outline color.
    pub(super) stroke: &'static str,
}

/// Return the visual style for one link.
pub(super) fn graphviz_link_style(edge: &GraphvizEdge) -> GraphvizLinkStyle {
    match edge.link_kind {
        LinkKind::Bgp => GraphvizLinkStyle {
            fill: GRAPHVIZ_BGP_FILL,
            stroke: GRAPHVIZ_BGP_STROKE,
        },
        LinkKind::Route => GraphvizLinkStyle {
            fill: GRAPHVIZ_ROUTE_FILL,
            stroke: GRAPHVIZ_ROUTE_STROKE,
        },
        LinkKind::Internal => GraphvizLinkStyle {
            fill: GRAPHVIZ_INTERNAL_FILL,
            stroke: GRAPHVIZ_INTERNAL_STROKE,
        },
        LinkKind::Customer => GraphvizLinkStyle {
            fill: GRAPHVIZ_CUSTOMER_FILL,
            stroke: GRAPHVIZ_CUSTOMER_STROKE,
        },
        LinkKind::Management => GraphvizLinkStyle {
            fill: GRAPHVIZ_MANAGEMENT_FILL,
            stroke: GRAPHVIZ_MANAGEMENT_STROKE,
        },
        LinkKind::Wireless => GraphvizLinkStyle {
            fill: GRAPHVIZ_WIRELESS_FILL,
            stroke: GRAPHVIZ_WIRELESS_STROKE,
        },
        LinkKind::Fallback => GraphvizLinkStyle {
            fill: GRAPHVIZ_MANAGEMENT_FILL,
            stroke: GRAPHVIZ_MANAGEMENT_STROKE,
        },
        LinkKind::Unknown => GraphvizLinkStyle {
            fill: GRAPHVIZ_UNKNOWN_FILL,
            stroke: GRAPHVIZ_UNKNOWN_STROKE,
        },
    }
}

/// Return a short table label for this link kind.
pub(super) const fn link_kind_label(link_kind: LinkKind) -> &'static str {
    match link_kind {
        LinkKind::Bgp => "bgp",
        LinkKind::Route => "route",
        LinkKind::Internal => "internal",
        LinkKind::Customer => "customer",
        LinkKind::Management => "management",
        LinkKind::Wireless => "wireless",
        LinkKind::Fallback => "fallback",
        LinkKind::Unknown => "unknown",
    }
}
