//! #182 (feature `sf`): `ScaleFillBivariate` + `GeomBivariateLegend` end to
//! end, and `geom_sf`'s (ordinary, per-aesthetic) legend following the
//! theme the same way every other geom's does.

#![cfg(feature = "sf")]

use ggplot_rs::aes::Aesthetic;
use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::geom::bivariate_legend::{GeomBivariateLegend, LegendCorner};
use ggplot_rs::prelude::*;
use ggplot_rs::scale::bivariate::ScaleFillBivariate;
use ggplot_rs::theme::LegendPosition;

fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

fn four_regions_df() -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))",
            "POLYGON ((10 0, 20 0, 20 10, 10 10, 10 0))",
            "POLYGON ((0 10, 10 10, 10 20, 0 20, 0 10))",
            "POLYGON ((10 10, 20 10, 20 20, 10 20, 10 10))",
        ]),
    );
    // low-low, high-x/low-y, low-x/high-y, high-high
    df.add_column("class".into(), strs(&["1-1", "3-1", "1-3", "3-3"]));
    df
}

#[test]
fn bivariate_choropleth_renders_distinct_corner_colors() {
    let s = ScaleFillBivariate::tertile(
        Aesthetic::Fill,
        (217, 95, 2),
        (117, 112, 179),
        "Income",
        "Education",
    );
    let svg = GGPlot::new(four_regions_df())
        .aes(Aes::new().fill("class"))
        .geom_sf()
        .scale_fill(s)
        .legend_position(LegendPosition::None)
        .render_svg_native()
        .expect("bivariate render");
    // Four distinct <path ...fill="#RRGGBB"...> swatches for the four
    // polygons (not counting the legend, suppressed above).
    let fills: std::collections::HashSet<&str> = svg
        .match_indices("fill=\"#")
        .filter_map(|(i, _)| svg[i + 6..].get(0..7))
        .collect();
    assert!(fills.len() >= 4, "expected >= 4 distinct fills: {fills:?}");
}

#[test]
fn bivariate_legend_draws_a_grid_and_two_captions() {
    let s = ScaleFillBivariate::tertile(
        Aesthetic::Fill,
        (217, 95, 2),
        (117, 112, 179),
        "Income",
        "Education",
    );
    let legend = GeomBivariateLegend::from_scale(&s).corner(LegendCorner::BottomRight);
    let svg = GGPlot::new(four_regions_df())
        .aes(Aes::new().fill("class"))
        .geom_sf_with(ggplot_rs::geom::sf::GeomSf::default())
        .scale_fill(s)
        .bivariate_legend(legend)
        .legend_position(LegendPosition::None)
        .render_svg_native()
        .expect("bivariate legend render");
    assert!(svg.contains("Income"), "{svg}");
    assert!(svg.contains("Education"), "{svg}");
    assert!(
        svg.contains("data-geom=\"bivariate_legend\""),
        "legend layer drew marks: {svg}"
    );
    // 3x3 = 9 swatch rects from the legend grid itself.
    let legend_rects = svg.matches("data-geom=\"bivariate_legend\"").count();
    assert!(legend_rects >= 9, "at least 9 legend cells: {legend_rects}");
}

#[test]
fn bivariate_legend_corners_place_differently() {
    let s = ScaleFillBivariate::tertile(Aesthetic::Fill, (200, 0, 0), (0, 0, 200), "x", "y");
    let mut outputs = Vec::new();
    for corner in [
        LegendCorner::BottomLeft,
        LegendCorner::BottomRight,
        LegendCorner::TopLeft,
        LegendCorner::TopRight,
    ] {
        let legend = GeomBivariateLegend::from_scale(&s).corner(corner);
        let svg = GGPlot::new(four_regions_df())
            .aes(Aes::new().fill("class"))
            .geom_sf()
            .scale_fill(ScaleFillBivariate::tertile(
                Aesthetic::Fill,
                (200, 0, 0),
                (0, 0, 200),
                "x",
                "y",
            ))
            .bivariate_legend(legend)
            .legend_position(LegendPosition::None)
            .render_svg_native()
            .expect("render");
        outputs.push(svg);
    }
    for i in 0..outputs.len() {
        for j in (i + 1)..outputs.len() {
            assert_ne!(outputs[i], outputs[j], "corners {i} and {j} identical");
        }
    }
}

/// #182: "geom_sf's legends follow the theme (fonts, ink/paper, dark
/// mode) like the other geoms do." `geom_sf`'s Fill legend rides the
/// exact same generic discrete-legend code every other geom's Fill/Color
/// legend does (no sf-specific legend path exists) — this proves it with
/// an explicit, fully custom theme (not just `theme_dark()`, which only
/// darkens the panel, matching ggplot2) covering legend background,
/// legend text and legend title.
#[test]
fn geom_sf_legend_follows_a_fully_custom_theme() {
    let mut theme = Theme::default();
    theme.legend_background.fill = Some((20, 20, 24));
    theme.legend_text.color = (230, 230, 230);
    theme.legend_title.color = (255, 255, 255);
    theme.legend_text.family = "Georgia".to_string();

    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))",
            "POLYGON ((10 0, 20 0, 20 10, 10 10, 10 0))",
        ]),
    );
    df.add_column("region".into(), strs(&["West", "East"]));
    let svg = GGPlot::new(df)
        .aes(Aes::new().fill("region"))
        .geom_sf()
        .theme(theme)
        .render_svg_native()
        .expect("themed sf legend render");
    assert!(svg.contains("#141418"), "legend background: {svg}");
    assert!(svg.contains("#E6E6E6"), "legend text color: {svg}");
    assert!(svg.contains("#FFFFFF"), "legend title color: {svg}");
    assert!(svg.contains("Georgia"), "legend text font family: {svg}");
}
