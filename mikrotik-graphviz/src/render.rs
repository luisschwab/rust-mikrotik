//! Graphviz process and interactive HTML rendering.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::constants::GRAPHVIZ_HTML_TOOLTIP_FONT_SIZE;
use crate::error::Error;
use crate::error::Result;
use crate::options::GraphvizFormat;
use crate::options::GraphvizRenderOptions;

/// Return whether the `dot` binary is available on `PATH`.
#[must_use]
pub fn has_graphviz_dot() -> bool {
    Command::new("dot").arg("-V").status().is_ok()
}

/// Render one Graphviz artifact from a DOT file.
///
/// # Errors
///
/// Returns an error when the `dot` process cannot be started or exits with a
/// non-success status.
pub fn render_graphviz_artifact(
    format: GraphvizFormat,
    dot_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    options: &GraphvizRenderOptions,
) -> Result<()> {
    let mut command = Command::new("dot");
    command
        .env("GDFONTPATH", &options.font_dir)
        .env("DOTFONTPATH", &options.font_dir)
        .arg(format!("-T{}", format.as_str()));
    if format == GraphvizFormat::Png {
        command.arg(format!("-Gdpi={}", options.png_dpi));
    }

    let status = command
        .arg(dot_path.as_ref())
        .arg("-o")
        .arg(output_path.as_ref())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Graphviz {
            format: format.as_str().to_owned(),
            status,
        })
    }
}

/// Write an interactive HTML wrapper for a Graphviz SVG with pan, zoom, and tooltips.
///
/// # Errors
///
/// Returns an error when the SVG cannot be read or the HTML file cannot be
/// written.
pub fn write_graphviz_interactive_html(svg_path: impl AsRef<Path>, html_path: impl AsRef<Path>) -> Result<()> {
    let svg = fs::read_to_string(svg_path.as_ref())?;
    let html = render_graphviz_html(include_str!("templates/graphviz.html"), &svg);
    fs::write(html_path.as_ref(), html)?;
    Ok(())
}

/// Render one Graphviz HTML template with the SVG and current tooltip font size.
fn render_graphviz_html(template: &str, svg: &str) -> String {
    template
        .replace("{GRAPHVIZ_HTML_TOOLTIP_FONT_SIZE}", GRAPHVIZ_HTML_TOOLTIP_FONT_SIZE)
        .replace("{svg}", svg)
}
