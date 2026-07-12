//! DOT and Graphviz process options.

use std::path::PathBuf;

use mikrotik_types::abstractions::LinkKind;
use serde::Deserialize;
use serde::Serialize;

use crate::constants::GRAPHVIZ_FIXED_POSITION_LAYOUT;
use crate::constants::GRAPHVIZ_FIXED_POSITION_OVERLAP;
use crate::constants::GRAPHVIZ_FONT_DIR;
use crate::constants::GRAPHVIZ_GRAPH_LAYOUT;
use crate::constants::GRAPHVIZ_LAYERED_LAYOUT;
use crate::constants::GRAPHVIZ_LAYERED_NODE_SEPARATION;
use crate::constants::GRAPHVIZ_LAYERED_OVERLAP;
use crate::constants::GRAPHVIZ_LAYERED_RANK_SEPARATION;
use crate::constants::GRAPHVIZ_NODE_SEPARATION;
use crate::constants::GRAPHVIZ_OUTPUT_ORDER;
use crate::constants::GRAPHVIZ_OVERLAP;
use crate::constants::GRAPHVIZ_OVERLAP_SCALING;
use crate::constants::GRAPHVIZ_PNG_DPI;
use crate::constants::GRAPHVIZ_PNG_FORMAT;
use crate::constants::GRAPHVIZ_RANK_DIRECTION;
use crate::constants::GRAPHVIZ_RANK_SEPARATION;
use crate::constants::GRAPHVIZ_RECURSIVE_RADIAL_LAYOUT;
use crate::constants::GRAPHVIZ_SEPARATION;
use crate::constants::GRAPHVIZ_SFDP_LAYOUT;
use crate::constants::GRAPHVIZ_SFDP_SEPARATION;
use crate::constants::GRAPHVIZ_SPLINES;
use crate::constants::GRAPHVIZ_TYPED_RADIAL_LAYOUT;

/// Options used when invoking Graphviz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphvizRenderOptions {
    /// Directory containing custom fonts.
    pub font_dir: PathBuf,
    /// DPI used for PNG output.
    pub png_dpi: String,
}

impl Default for GraphvizRenderOptions {
    fn default() -> Self {
        Self {
            font_dir: PathBuf::from(GRAPHVIZ_FONT_DIR),
            png_dpi: GRAPHVIZ_PNG_DPI.to_owned(),
        }
    }
}

/// Options that control DOT generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DotExportOptions {
    /// Graphviz layout engine attribute.
    pub layout: String,
    /// Graphviz rank direction.
    pub rank_direction: String,
    /// Distance between visual ranks.
    pub rank_separation: String,
    /// Distance between nodes in the same rank.
    pub node_separation: String,
    /// Graphviz spline routing mode.
    pub splines: String,
    /// Graphviz rendering order.
    pub output_order: String,
    /// Graphviz overlap mode.
    pub overlap: String,
    /// Optional Graphviz overlap scaling for overlap algorithms that support it.
    pub overlap_scaling: Option<String>,
    /// Graphviz overlap-removal separation margin.
    pub separation: String,
    /// Optional node key used as the center/root for radial layouts.
    pub root_node: Option<String>,
    /// Node keys that should render as seed devices.
    pub seed_nodes: Vec<String>,
    /// Node keys that should render as owned BGP routers in hierarchical layouts.
    pub owned_bgp_nodes: Vec<String>,
    /// Optional URL prefix used to link collected graph nodes by serial.
    pub node_url_prefix: Option<String>,
    /// Which links to include in the rendered output.
    pub link_filter: LinkFilter,
    /// Whether link detail tables should be omitted from the rendered output.
    pub hide_link_tables: bool,
}

impl Default for DotExportOptions {
    fn default() -> Self {
        Self {
            layout: GRAPHVIZ_GRAPH_LAYOUT.to_owned(),
            rank_direction: GRAPHVIZ_RANK_DIRECTION.to_owned(),
            rank_separation: GRAPHVIZ_RANK_SEPARATION.to_owned(),
            node_separation: GRAPHVIZ_NODE_SEPARATION.to_owned(),
            splines: GRAPHVIZ_SPLINES.to_owned(),
            output_order: GRAPHVIZ_OUTPUT_ORDER.to_owned(),
            overlap: GRAPHVIZ_OVERLAP.to_owned(),
            overlap_scaling: Some(GRAPHVIZ_OVERLAP_SCALING.to_owned()),
            separation: GRAPHVIZ_SEPARATION.to_owned(),
            root_node: None,
            seed_nodes: Vec::new(),
            owned_bgp_nodes: Vec::new(),
            node_url_prefix: None,
            link_filter: LinkFilter::All,
            hide_link_tables: false,
        }
    }
}

impl DotExportOptions {
    /// Return export options for one supported layout with layout-specific spacing.
    #[must_use]
    pub fn for_layout(layout: impl Into<String>) -> Self {
        let layout = layout.into();
        let mut options = Self {
            layout,
            ..Self::default()
        };
        if options.is_layered_layout() {
            GRAPHVIZ_LAYERED_RANK_SEPARATION.clone_into(&mut options.rank_separation);
            GRAPHVIZ_LAYERED_NODE_SEPARATION.clone_into(&mut options.node_separation);
            GRAPHVIZ_LAYERED_OVERLAP.clone_into(&mut options.overlap);
            options.overlap_scaling = None;
        } else if options.is_sfdp_layout() {
            GRAPHVIZ_SFDP_SEPARATION.clone_into(&mut options.separation);
        } else if options.is_fixed_position_layout() {
            GRAPHVIZ_FIXED_POSITION_OVERLAP.clone_into(&mut options.overlap);
            options.overlap_scaling = None;
        }
        options
    }

    /// Return whether these options request the layered `dot` layout.
    #[must_use]
    pub fn is_layered_layout(&self) -> bool {
        self.layout == GRAPHVIZ_LAYERED_LAYOUT
    }

    /// Return whether these options request the `sfdp` force-directed layout.
    #[must_use]
    pub fn is_sfdp_layout(&self) -> bool {
        self.layout == GRAPHVIZ_SFDP_LAYOUT
    }

    /// Return whether these options request the deterministic typed radial layout.
    #[must_use]
    pub fn is_typed_radial_layout(&self) -> bool {
        self.layout == GRAPHVIZ_TYPED_RADIAL_LAYOUT
    }

    /// Return whether these options request the deterministic recursive radial layout.
    #[must_use]
    pub fn is_recursive_radial_layout(&self) -> bool {
        self.layout == GRAPHVIZ_RECURSIVE_RADIAL_LAYOUT
    }

    /// Return whether these options request a fixed-position layout.
    #[must_use]
    pub fn is_fixed_position_layout(&self) -> bool {
        self.is_typed_radial_layout() || self.is_recursive_radial_layout()
    }

    /// Return the Graphviz layout engine emitted to DOT.
    #[must_use]
    pub fn graphviz_layout_engine(&self) -> &str {
        if self.is_fixed_position_layout() {
            GRAPHVIZ_FIXED_POSITION_LAYOUT
        } else {
            &self.layout
        }
    }
}

/// Link classes that can be included or excluded from rendered topology output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkFilter {
    /// Render every graph link.
    All,
    /// Render routing/topology links plus fallback anchors, excluding ordinary management discovery edges.
    Routing,
    /// Render only non-BGP L3 links.
    PhysicalOnly,
    /// Render only logical BGP links.
    BgpOnly,
}

impl LinkFilter {
    /// Return whether this filter includes a link kind.
    pub(crate) const fn includes(self, kind: LinkKind) -> bool {
        match self {
            Self::All => true,
            Self::Routing => !matches!(kind, LinkKind::Management),
            Self::PhysicalOnly => !matches!(kind, LinkKind::Bgp | LinkKind::Management | LinkKind::Fallback),
            Self::BgpOnly => matches!(kind, LinkKind::Bgp),
        }
    }
}

/// Graphviz output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphvizFormat {
    /// Scalable Vector Graphics.
    Svg,
    /// Raster PNG.
    Png,
}

impl GraphvizFormat {
    /// Return the `dot -T` format string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => GRAPHVIZ_PNG_FORMAT,
        }
    }
}
