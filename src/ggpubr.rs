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
use crate::plot::GGPlot;

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
