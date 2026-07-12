//! Graphviz renderer constants.

use core::f64::consts::PI;

/// Graphviz radial layout engine.
pub const GRAPHVIZ_RADIAL_LAYOUT: &str = "twopi";
/// Deterministic radial layout grouped by link type.
pub const GRAPHVIZ_TYPED_RADIAL_LAYOUT: &str = "typed-radial";
/// Deterministic recursive radial tree layout rooted at the primary seed.
pub const GRAPHVIZ_RECURSIVE_RADIAL_LAYOUT: &str = "recursive-radial";
/// Graphviz force-directed multiscale layout engine.
pub const GRAPHVIZ_SFDP_LAYOUT: &str = "sfdp";
/// Graphviz engine used for fixed-position typed radial output.
pub const GRAPHVIZ_FIXED_POSITION_LAYOUT: &str = "neato";
/// Graphviz layered layout engine.
pub const GRAPHVIZ_LAYERED_LAYOUT: &str = "dot";
/// Default Graphviz graph layout.
pub const GRAPHVIZ_GRAPH_LAYOUT: &str = GRAPHVIZ_RADIAL_LAYOUT;
/// Default Graphviz rank direction.
pub const GRAPHVIZ_RANK_DIRECTION: &str = "TB";
/// Graphviz rank separation per radial hop for hub-and-spoke layouts.
pub const GRAPHVIZ_RADIAL_RANK_SEPARATION: &str = "3.2 1.8 1.2 1.0";
/// Graphviz rank separation for layered layouts.
pub const GRAPHVIZ_LAYERED_RANK_SEPARATION: &str = "2.0";
/// Default Graphviz rank separation.
pub const GRAPHVIZ_RANK_SEPARATION: &str = GRAPHVIZ_RADIAL_RANK_SEPARATION;
/// Graphviz node separation for radial hub-and-spoke layouts.
pub const GRAPHVIZ_RADIAL_NODE_SEPARATION: &str = "1.0";
/// Graphviz node separation for layered layouts.
pub const GRAPHVIZ_LAYERED_NODE_SEPARATION: &str = "0.6";
/// Default Graphviz node separation.
pub const GRAPHVIZ_NODE_SEPARATION: &str = GRAPHVIZ_RADIAL_NODE_SEPARATION;
/// Default Graphviz spline style.
pub const GRAPHVIZ_SPLINES: &str = "line";
/// Graphviz draw order: render edges before nodes so links sit behind devices.
pub const GRAPHVIZ_OUTPUT_ORDER: &str = "edgesfirst";
/// Graphviz overlap removal for radial hub-and-spoke layouts.
pub const GRAPHVIZ_RADIAL_OVERLAP: &str = "prism";
/// Graphviz radial overlap scaling.
pub const GRAPHVIZ_RADIAL_OVERLAP_SCALING: &str = "0";
/// Graphviz overlap behavior for fixed-position typed radial layouts.
pub const GRAPHVIZ_FIXED_POSITION_OVERLAP: &str = "false";
/// Graphviz overlap behavior for layered layouts.
pub const GRAPHVIZ_LAYERED_OVERLAP: &str = "false";
/// Default Graphviz overlap behavior.
pub const GRAPHVIZ_OVERLAP: &str = GRAPHVIZ_RADIAL_OVERLAP;
/// Default Graphviz overlap scaling.
pub const GRAPHVIZ_OVERLAP_SCALING: &str = GRAPHVIZ_RADIAL_OVERLAP_SCALING;
/// Graphviz separation margin used by overlap removal.
pub const GRAPHVIZ_SEPARATION: &str = "+12";
/// Graphviz separation margin for SFDP overlap removal.
pub const GRAPHVIZ_SFDP_SEPARATION: &str = "+42";
/// Graphviz SFDP spring constant; higher values spread nodes apart.
pub const GRAPHVIZ_SFDP_SPRING_CONSTANT: &str = "3.6";
/// Graphviz SFDP visible edge length.
pub const GRAPHVIZ_SFDP_EDGE_LENGTH: &str = "3.2";
/// Graphviz font used by the topology renderer.
pub const GRAPHVIZ_FONT: &str = "Berkeley Mono";
/// Graphviz font used for device nodes.
pub const GRAPHVIZ_DEVICE_FONT: &str = "Berkeley Mono Bold";
/// Directory containing the bundled Graphviz fonts.
pub const GRAPHVIZ_FONT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../mikrotik-observer/assets/font");
/// Graphviz DPI used for raster PNG exports.
pub const GRAPHVIZ_PNG_DPI: &str = "300";
/// Graphviz output format used for raster PNG exports.
pub const GRAPHVIZ_PNG_FORMAT: &str = "png:cairo";
/// Graphviz base font size.
pub const GRAPHVIZ_FONT_SIZE: &str = "10";
/// Graphviz link table font size.
pub const GRAPHVIZ_LINK_TABLE_FONT_SIZE: &str = GRAPHVIZ_FONT_SIZE;
/// HTML tooltip font size.
pub const GRAPHVIZ_HTML_TOOLTIP_FONT_SIZE: &str = "25px";
/// Graphviz edge stroke color.
pub const GRAPHVIZ_EDGE_COLOR: &str = "#94a3b8";
/// Graphviz edge stroke width.
pub const GRAPHVIZ_EDGE_PEN_WIDTH: &str = "0.85";
/// Graphviz edge marker at both endpoints.
pub const GRAPHVIZ_EDGE_ENDPOINT_MARKER: &str = "dot";
/// Graphviz edge endpoint marker size.
pub const GRAPHVIZ_EDGE_ENDPOINT_MARKER_SIZE: &str = "0.65";
/// Graphviz node margin.
pub const GRAPHVIZ_NODE_MARGIN: &str = "0.12,0.08";
/// Graphviz device node width in inches.
pub const GRAPHVIZ_DEVICE_NODE_WIDTH: &str = "2.2";
/// Graphviz device node height in inches.
pub const GRAPHVIZ_DEVICE_NODE_HEIGHT: &str = "0.85";
/// Typed radial ring distance in Graphviz points.
pub const GRAPHVIZ_TYPED_RADIAL_RING_DISTANCE: f64 = 380.0;
/// Typed radial minimum angle between nodes on one ring.
pub const GRAPHVIZ_TYPED_RADIAL_MIN_ANGLE: f64 = 0.115;
/// Horizontal spacing between nested typed-radial child nodes in Graphviz points.
pub const GRAPHVIZ_TYPED_RADIAL_CHILD_COLUMN_SPACING: f64 = 250.0;
/// Vertical spacing between nested typed-radial child rows in Graphviz points.
pub const GRAPHVIZ_TYPED_RADIAL_CHILD_ROW_SPACING: f64 = 170.0;
/// Number of extra radial points before the first nested typed-radial child row.
pub const GRAPHVIZ_TYPED_RADIAL_CHILD_ROW_OFFSET: f64 = 320.0;
/// Recursive radial distance between parent and first child arc.
pub const GRAPHVIZ_RECURSIVE_RADIAL_RADIUS: f64 = 720.0;
/// Recursive radial distance between child arc rows.
pub const GRAPHVIZ_RECURSIVE_RADIAL_ROW_GAP: f64 = 440.0;
/// Recursive radial maximum children on one arc row.
pub const GRAPHVIZ_RECURSIVE_RADIAL_MAX_ROW_CHILDREN: usize = 8;
/// Recursive radial root downstream arc start angle.
pub const GRAPHVIZ_RECURSIVE_RADIAL_ROOT_START: f64 = PI * 1.01;
/// Recursive radial root downstream arc end angle.
pub const GRAPHVIZ_RECURSIVE_RADIAL_ROOT_END: f64 = PI * 1.99;
/// Recursive radial nested child arc span.
pub const GRAPHVIZ_RECURSIVE_RADIAL_CHILD_SPAN: f64 = PI * 1.05;
/// Horizontal spacing for disconnected nodes placed at the bottom.
pub const GRAPHVIZ_RECURSIVE_RADIAL_DISCONNECTED_SPACING: f64 = 380.0;
/// Bottom offset for disconnected nodes.
pub const GRAPHVIZ_RECURSIVE_RADIAL_DISCONNECTED_OFFSET: f64 = 3_250.0;
/// Horizontal section spacing between nodes in fixed-position section bands.
pub const GRAPHVIZ_SECTION_NODE_SPACING: f64 = 430.0;
/// Maximum width of one semantic section row before wrapping.
pub const GRAPHVIZ_SECTION_ROW_WIDTH: f64 = 8_000.0;
/// Vertical section spacing between rows inside one section band.
pub const GRAPHVIZ_SECTION_ROW_GAP: f64 = 300.0;
/// Y coordinate for external BGP peer section.
pub const GRAPHVIZ_SECTION_UPSTREAM_Y: f64 = 2_400.0;
/// Y coordinate for owned BGP router section.
pub const GRAPHVIZ_SECTION_OWNED_BGP_Y: f64 = 1_500.0;
/// Y coordinate for border router section.
pub const GRAPHVIZ_SECTION_BORDER_Y: f64 = 600.0;
/// Y coordinate for core router section.
pub const GRAPHVIZ_SECTION_CORE_Y: f64 = -300.0;
/// Y coordinate for customer router section.
pub const GRAPHVIZ_SECTION_CUSTOMER_Y: f64 = -1_250.0;
/// Y coordinate for uncategorized node section.
pub const GRAPHVIZ_SECTION_UNKNOWN_Y: f64 = -2_500.0;
/// Vertical gap for topology depths below the ordinary customer section.
pub const GRAPHVIZ_SECTION_DEEP_DOWNSTREAM_GAP: f64 = 420.0;
/// Invisible SFDP rank-anchor edge weight.
pub const GRAPHVIZ_SFDP_RANK_ANCHOR_WEIGHT: &str = "80";
/// Invisible SFDP rank-anchor edge length.
pub const GRAPHVIZ_SFDP_RANK_ANCHOR_LENGTH: &str = "0.4";
/// Invisible SFDP radio-chain edge weight.
pub const GRAPHVIZ_SFDP_RADIO_CHAIN_WEIGHT: &str = "140";
/// Invisible SFDP radio-chain edge length.
pub const GRAPHVIZ_SFDP_RADIO_CHAIN_LENGTH: &str = "0.65";
/// Graphviz fill color for collected devices.
pub const GRAPHVIZ_COLLECTED_DEVICE_FILL: &str = "#e0f2fe";
/// Graphviz stroke color for collected devices.
pub const GRAPHVIZ_COLLECTED_DEVICE_STROKE: &str = "#0369a1";
/// Graphviz fill color for the seed/root device.
pub const GRAPHVIZ_SEED_DEVICE_FILL: &str = "#ede9fe";
/// Graphviz stroke color for the seed/root device.
pub const GRAPHVIZ_SEED_DEVICE_STROKE: &str = "#7c3aed";
/// Graphviz fill color for inferred devices.
pub const GRAPHVIZ_INFERRED_DEVICE_FILL: &str = "#fef9c3";
/// Graphviz stroke color for inferred devices.
pub const GRAPHVIZ_INFERRED_DEVICE_STROKE: &str = "#ca8a04";
/// Graphviz fill color for unreachable devices.
pub const GRAPHVIZ_UNREACHABLE_DEVICE_FILL: &str = "#f1f5f9";
/// Graphviz fill color for devices with wrong credentials.
pub const GRAPHVIZ_WRONG_CREDENTIALS_FILL: &str = "#fee2e2";
/// Graphviz link table cell padding.
pub const GRAPHVIZ_LINK_TABLE_CELL_PADDING: &str = "4";
/// Graphviz BGP link fill color.
pub const GRAPHVIZ_BGP_FILL: &str = "#dbeafe";
/// Graphviz BGP link stroke color.
pub const GRAPHVIZ_BGP_STROKE: &str = "#2563eb";
/// Graphviz route link fill color.
pub const GRAPHVIZ_ROUTE_FILL: &str = "#ccfbf1";
/// Graphviz route link stroke color.
pub const GRAPHVIZ_ROUTE_STROKE: &str = "#0d9488";
/// Graphviz internal link fill color.
pub const GRAPHVIZ_INTERNAL_FILL: &str = "#ffedd5";
/// Graphviz internal link stroke color.
pub const GRAPHVIZ_INTERNAL_STROKE: &str = "#ea580c";
/// Graphviz customer link fill color.
pub const GRAPHVIZ_CUSTOMER_FILL: &str = "#dcfce7";
/// Graphviz customer link stroke color.
pub const GRAPHVIZ_CUSTOMER_STROKE: &str = "#16a34a";
/// Graphviz management link fill color.
pub const GRAPHVIZ_MANAGEMENT_FILL: &str = "#f5f3ff";
/// Graphviz management link stroke color.
pub const GRAPHVIZ_MANAGEMENT_STROKE: &str = "#7c3aed";
/// Graphviz wireless/backhaul link fill color.
pub const GRAPHVIZ_WIRELESS_FILL: &str = "#ecfeff";
/// Graphviz wireless/backhaul link stroke color.
pub const GRAPHVIZ_WIRELESS_STROKE: &str = "#0891b2";
/// Graphviz unknown link fill color.
pub const GRAPHVIZ_UNKNOWN_FILL: &str = "#f3f4f6";
/// Graphviz unknown link stroke color.
pub const GRAPHVIZ_UNKNOWN_STROKE: &str = "#6b7280";
/// Graphviz node outline color for wrong credentials.
pub const GRAPHVIZ_WRONG_CREDENTIALS_STROKE: &str = "#dc2626";
/// Graphviz label placed above nodes that could not be crawled due to credentials.
pub const GRAPHVIZ_WRONG_CREDENTIALS_LABEL: &str = "ERROR: BAD CREDENTIALS";
/// Graphviz label placed above nodes that refused `RouterOS` API connections.
pub const GRAPHVIZ_API_REFUSED_LABEL: &str = "ERROR: API REFUSED";
