//! ggpubr-style one-call plot constructors.
//!
//! Thin wrappers that build a [`GGPlot`] pre-configured with the right
//! aesthetics, geom, and the publication [`theme_pubr`](crate::theme::presets::theme_pubr),
//! mirroring R's `ggpubr::ggscatter` / `ggboxplot` / … Each returns a `GGPlot`
//! so you can keep chaining (add `stat_cor`, palettes, labels, save, …).
//!
//! These are pure grammar — no statistics — so they're always available (they
//! don't require the `ggpubr` feature, which only gates the stat annotations).

use crate::aes::Aes;
use crate::data::GGData;
use crate::plot::{GGError, GGPlot};

fn base(data: impl GGData, mut aes: Aes, group: Option<&str>, as_fill: bool) -> (GGPlot, Aes) {
    if let Some(g) = group {
        aes = if as_fill { aes.fill(g) } else { aes.color(g) };
    }
    (GGPlot::new(data), aes)
}

/// Publication scatter plot (`ggpubr::ggscatter`). `color` optionally maps a
/// grouping column to point colour.
pub fn ggscatter(data: impl GGData, x: &str, y: &str, color: Option<&str>) -> GGPlot {
    let (plot, aes) = base(data, Aes::new().x(x).y(y), color, false);
    plot.aes(aes).geom_point().theme_pubr()
}

/// Publication line plot (`ggpubr::ggline`). `color` optionally maps a grouping
/// column to line colour.
pub fn ggline(data: impl GGData, x: &str, y: &str, color: Option<&str>) -> GGPlot {
    let (plot, aes) = base(data, Aes::new().x(x).y(y), color, false);
    plot.aes(aes).geom_line().theme_pubr()
}

/// Publication box plot (`ggpubr::ggboxplot`). `fill` optionally maps a grouping
/// column to box fill.
pub fn ggboxplot(data: impl GGData, x: &str, y: &str, fill: Option<&str>) -> GGPlot {
    let (plot, aes) = base(data, Aes::new().x(x).y(y), fill, true);
    plot.aes(aes).geom_boxplot().theme_pubr()
}

/// Publication violin plot (`ggpubr::ggviolin`). `fill` optionally maps a
/// grouping column to violin fill.
pub fn ggviolin(data: impl GGData, x: &str, y: &str, fill: Option<&str>) -> GGPlot {
    let (plot, aes) = base(data, Aes::new().x(x).y(y), fill, true);
    plot.aes(aes).geom_violin().theme_pubr()
}

/// Publication histogram (`ggpubr::gghistogram`) of a single variable `x`.
/// `fill` optionally maps a grouping column to bar fill.
pub fn gghistogram(data: impl GGData, x: &str, fill: Option<&str>) -> GGPlot {
    let (plot, aes) = base(data, Aes::new().x(x), fill, true);
    plot.aes(aes).geom_histogram().theme_pubr()
}

/// Publication density plot (`ggpubr::ggdensity`) of a single variable `x`.
/// `color` optionally maps a grouping column to line colour.
pub fn ggdensity(data: impl GGData, x: &str, color: Option<&str>) -> GGPlot {
    let (plot, aes) = base(data, Aes::new().x(x), color, false);
    plot.aes(aes).geom_density().theme_pubr()
}

/// Arrange several plots in a grid, composed into a single SVG document
/// (`ggpubr::ggarrange`). Plots fill row-major across `ncol` columns; each
/// occupies a `cell_w` × `cell_h` cell and is embedded as a positioned nested
/// `<svg>`. Returns the combined SVG string.
pub fn ggarrange(
    plots: Vec<GGPlot>,
    ncol: usize,
    cell_w: u32,
    cell_h: u32,
) -> Result<String, GGError> {
    let n = plots.len();
    let ncol = ncol.max(1);
    let nrow = n.div_ceil(ncol);
    let total_w = ncol as u32 * cell_w;
    let total_h = nrow.max(1) as u32 * cell_h;

    let mut children = String::new();
    for (i, plot) in plots.into_iter().enumerate() {
        let inner = plot.render_svg_native_with_size(cell_w, cell_h)?;
        let x = (i % ncol) as u32 * cell_w;
        let y = (i / ncol) as u32 * cell_h;
        // Turn each child's root `<svg …>` into a positioned nested `<svg x y …>`;
        // it keeps its own viewBox/width/height so it fills exactly its cell.
        let positioned = inner.replacen("<svg ", &format!("<svg x=\"{x}\" y=\"{y}\" "), 1);
        children.push_str(&positioned);
    }

    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{total_w}\" height=\"{total_h}\" \
         viewBox=\"0 0 {total_w} {total_h}\">{children}</svg>"
    ))
}

/// [`ggarrange`] that writes the combined SVG to `path`.
pub fn ggarrange_save(
    plots: Vec<GGPlot>,
    ncol: usize,
    cell_w: u32,
    cell_h: u32,
    path: &str,
) -> Result<(), GGError> {
    let svg = ggarrange(plots, ncol, cell_w, cell_h)?;
    std::fs::write(path, svg).map_err(GGError::Io)
}

/// [`ggarrange`] rendered as a single PNG. Each plot is rasterised on its own
/// (via the plotters bitmap backend) and composited into an `ncol`-wide grid.
/// Returns the encoded PNG bytes.
#[cfg(not(target_arch = "wasm32"))]
pub fn ggarrange_png(
    plots: Vec<GGPlot>,
    ncol: usize,
    cell_w: u32,
    cell_h: u32,
) -> Result<Vec<u8>, GGError> {
    use crate::render::RenderError;
    let n = plots.len();
    let ncol = ncol.max(1);
    let nrow = n.div_ceil(ncol).max(1);
    let mut canvas = image::RgbaImage::from_pixel(
        ncol as u32 * cell_w,
        nrow as u32 * cell_h,
        image::Rgba([255, 255, 255, 255]),
    );
    for (i, plot) in plots.into_iter().enumerate() {
        let png = plot.render_png_with_size(cell_w, cell_h)?;
        let cell = image::load_from_memory(&png)
            .map_err(|e| GGError::Render(RenderError::BackendError(format!("decode: {e}"))))?
            .to_rgba8();
        let x = ((i % ncol) as u32 * cell_w) as i64;
        let y = ((i / ncol) as u32 * cell_h) as i64;
        image::imageops::overlay(&mut canvas, &cell, x, y);
    }
    let mut out = std::io::Cursor::new(Vec::new());
    canvas
        .write_to(&mut out, image::ImageOutputFormat::Png)
        .map_err(|e| GGError::Render(RenderError::BackendError(format!("encode: {e}"))))?;
    Ok(out.into_inner())
}

/// [`ggarrange_png`] that writes the composited PNG to `path`.
#[cfg(not(target_arch = "wasm32"))]
pub fn ggarrange_save_png(
    plots: Vec<GGPlot>,
    ncol: usize,
    cell_w: u32,
    cell_h: u32,
    path: &str,
) -> Result<(), GGError> {
    let png = ggarrange_png(plots, ncol, cell_w, cell_h)?;
    std::fs::write(path, png).map_err(GGError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Value;

    fn xy() -> Vec<(String, Vec<Value>)> {
        let x: Vec<Value> = (0..12).map(|i| Value::Float(i as f64)).collect();
        let y: Vec<Value> = (0..12).map(|i| Value::Float((i as f64).sin())).collect();
        let g: Vec<Value> = (0..12)
            .map(|i| Value::Str(["a", "b"][i % 2].to_string()))
            .collect();
        vec![
            ("x".to_string(), x),
            ("y".to_string(), y),
            ("g".to_string(), g),
        ]
    }

    #[test]
    fn constructors_build_and_render() {
        assert!(ggscatter(xy(), "x", "y", Some("g")).render_svg().is_ok());
        assert!(ggline(xy(), "x", "y", None).render_svg().is_ok());
        assert!(ggboxplot(xy(), "g", "y", Some("g")).render_svg().is_ok());
        assert!(ggviolin(xy(), "g", "y", Some("g")).render_svg().is_ok());
        assert!(gghistogram(xy(), "y", None).render_svg().is_ok());
        assert!(ggdensity(xy(), "y", Some("g")).render_svg().is_ok());
    }

    #[test]
    fn ggarrange_composes_a_grid() {
        let plots = vec![
            ggscatter(xy(), "x", "y", None),
            ggline(xy(), "x", "y", None),
            ggboxplot(xy(), "g", "y", Some("g")),
            ggdensity(xy(), "y", None),
        ];
        let svg = ggarrange(plots, 2, 300, 220).expect("arrange");
        // One outer document sized 2×2 cells, with four nested <svg> children.
        assert!(svg.contains("width=\"600\" height=\"440\""), "outer size");
        assert_eq!(svg.matches("<svg ").count(), 5, "outer + 4 nested svgs");
        // The children are positioned into the four cells.
        assert!(svg.contains("x=\"0\" y=\"0\""));
        assert!(svg.contains("x=\"300\" y=\"0\""));
        assert!(svg.contains("x=\"0\" y=\"220\""));
        assert!(svg.contains("x=\"300\" y=\"220\""));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn ggarrange_png_composes_a_grid() {
        let plots = vec![
            ggscatter(xy(), "x", "y", None),
            ggline(xy(), "x", "y", None),
            ggboxplot(xy(), "g", "y", Some("g")),
        ];
        let png = ggarrange_png(plots, 2, 200, 160).expect("png");
        // Valid PNG signature and non-trivial size.
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        assert!(png.len() > 1000);
    }

    #[test]
    fn constructors_are_chainable() {
        // The returned GGPlot keeps chaining (here: a title + a manual save-less
        // render), proving these are ordinary builders.
        let svg = ggscatter(xy(), "x", "y", None)
            .title("chained")
            .render_svg()
            .unwrap();
        assert!(svg.contains("chained"));
    }
}
