//! The plotters-free native SVG path (`render_svg_native*`) across geoms, stats,
//! coords, facets and themes. Runs in every feature configuration — including
//! `--no-default-features` (no plotters/image), the setup downstream renderers
//! such as the DuckDB extension use.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

type Cols = Vec<(String, Vec<Value>)>;

fn data() -> Cols {
    let n = 40;
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64 * 0.25)).collect();
    let y: Vec<Value> = (0..n)
        .map(|i| Value::Float((i as f64 * 0.3).sin() * 3.0 + i as f64 * 0.1))
        .collect();
    let g: Vec<Value> = (0..n)
        .map(|i| Value::Str(["a", "b", "c"][i % 3].to_string()))
        .collect();
    let w: Vec<Value> = (0..n).map(|i| Value::Float(1.0 + (i % 5) as f64)).collect();
    vec![
        ("x".into(), x),
        ("y".into(), y),
        ("g".into(), g),
        ("w".into(), w),
    ]
}

fn check(plot: GGPlot, what: &str) -> String {
    let svg = plot
        .render_svg_native_with_size(480, 320)
        .unwrap_or_else(|e| panic!("{what}: {e}"));
    assert!(svg.starts_with("<svg"), "{what}: root element");
    assert!(
        svg.trim_end().ends_with("</svg>"),
        "{what}: closed document"
    );
    assert!(!svg.contains("NaN"), "{what}: NaN leaked into SVG");
    svg
}

fn xy() -> GGPlot {
    GGPlot::new(data()).aes(Aes::new().x("x").y("y"))
}

fn gy() -> GGPlot {
    GGPlot::new(data()).aes(Aes::new().x("g").y("y"))
}

#[test]
fn xy_geoms_render() {
    let cases: Vec<(&str, GGPlot)> = vec![
        ("point", xy().geom_point()),
        ("line", xy().geom_line()),
        ("path", xy().geom_path()),
        ("step", xy().geom_step()),
        ("area", xy().geom_area()),
        ("smooth", xy().geom_smooth()),
        ("jitter", xy().geom_jitter()),
        ("rug", xy().geom_point().geom_rug()),
        ("bin2d", xy().geom_bin2d()),
        ("hex", xy().geom_hex()),
        ("density2d", xy().geom_density2d()),
        ("count", xy().geom_count()),
        ("blank", xy().geom_blank()),
    ];
    for (name, p) in cases {
        check(p, name);
    }
}

#[test]
fn distribution_geoms_render() {
    let x = || GGPlot::new(data()).aes(Aes::new().x("y"));
    check(x().geom_histogram(), "histogram");
    check(x().geom_density(), "density");
    check(x().geom_freqpoly(), "freqpoly");
    check(x().geom_dotplot(), "dotplot");
    check(GGPlot::new(data()).aes(Aes::new().y("y")).geom_qq(), "qq");
    check(gy().geom_boxplot(), "boxplot");
    check(gy().geom_violin(), "violin");
    check(GGPlot::new(data()).aes(Aes::new().x("g")).geom_bar(), "bar");
    check(gy().geom_col(), "col");
}

#[test]
fn grouped_and_coloured_layers_render_legends() {
    let svg = check(
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_point()
            .geom_line()
            .title("Grouped"),
        "grouped",
    );
    assert!(svg.contains("Grouped"));
    for level in ["a", "b", "c"] {
        assert!(svg.contains(&format!(">{level}<")), "legend label {level}");
    }
}

#[test]
fn coords_facets_and_themes_render() {
    check(xy().geom_point().coord_flip(), "coord_flip");
    check(xy().geom_point().coord_polar(), "coord_polar");
    check(xy().geom_point().coord_fixed(1.0), "coord_fixed");
    check(xy().geom_point().facet_wrap("g", Some(2)), "facet_wrap");
    check(xy().geom_point().facet_grid(Some("g"), None), "facet_grid");
    check(xy().geom_point().scale_y_log10(), "log10");
    check(xy().geom_point().theme_minimal(), "theme_minimal");
    check(xy().geom_point().theme_dark(), "theme_dark");
    check(xy().geom_point().theme_void(), "theme_void");
}

#[test]
fn manual_scales_render() {
    let svg = check(
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_point()
            .scale_color_manual(vec![
                ("a", RGBAColor::new(255, 0, 0)),
                ("b", RGBAColor::new(0, 128, 0)),
                ("c", RGBAColor::new(0, 0, 255)),
            ]),
        "manual",
    );
    assert!(svg.contains("255,0,0") || svg.contains("#ff0000") || svg.contains("#FF0000"));
}
