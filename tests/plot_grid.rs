//! PlotGrid composition on the plotters-free native SVG path.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let n = 30;
    vec![
        (
            "x".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..n)
                .map(|i| Value::Float((i as f64 * 0.4).sin()))
                .collect(),
        ),
        (
            "g".to_string(),
            (0..n)
                .map(|i| Value::Str(["ctrl", "trt"][i % 2].to_string()))
                .collect(),
        ),
    ]
}

fn scatter() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
}

fn line() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_line()
}

fn plain() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
}

/// `(x, y, w, h)` of every nested `<svg>` (skipping the root).
fn cells(svg: &str) -> Vec<(f64, f64, f64, f64)> {
    svg.split("<svg ")
        .skip(2)
        .map(|part| {
            let open = format!(" {}", part.split('>').next().unwrap());
            let a = |n: &str| -> f64 {
                open.split(&format!(" {n}=\""))
                    .nth(1)
                    .and_then(|r| r.split('"').next())
                    .and_then(|v| v.parse().ok())
                    .unwrap()
            };
            (a("x"), a("y"), a("width"), a("height"))
        })
        .collect()
}

fn well_formed(svg: &str) {
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.ends_with("</svg>"));
    assert_eq!(svg.matches("<svg ").count(), svg.matches("</svg>").count());
    assert!(!svg.contains("NaN") && !svg.contains("inf\""));
    // Only the root declares the namespace.
    assert_eq!(svg.matches("xmlns=").count(), 1);
}

#[test]
fn two_by_two_with_tags_title_and_caption() {
    let svg = PlotGrid::new()
        .add(plain())
        .add(plain())
        .add(plain())
        .add(plain())
        .ncol(2)
        .title("Regression diagnostics")
        .subtitle("lm(y ~ x)")
        .caption("source: anofox")
        .tag_levels(TagLevels::Upper)
        .render_svg_native_with_size(800, 600)
        .unwrap();
    well_formed(&svg);
    for t in ["A", "B", "C", "D"] {
        assert!(svg.contains(&format!("data-panel=\"{t}\"")), "panel {t}");
        assert!(svg.contains(&format!(">{t}</text>")), "tag {t} drawn");
    }
    assert!(svg.contains(">Regression diagnostics<"));
    assert!(svg.contains(">source: anofox<"));
    assert!(svg.contains("data-grid=\"2 2\""));
    let c = cells(&svg);
    assert_eq!(c.len(), 4);
    // Equal columns, rows below the header.
    assert_eq!(c[0].2, 400.0);
    assert_eq!(c[1].0, 400.0);
    assert!(c[0].1 > 20.0, "header reserved: {c:?}");
    assert!(c[2].1 + c[2].3 < 600.0, "footer reserved");
    // Each sub-plot keeps its own host attributes.
    assert_eq!(svg.matches("data-plot=\"").count(), 4);
    assert_eq!(svg.matches("data-domain=\"").count(), 4);
    assert!(svg.contains("data-x=\""));
}

#[test]
fn operators_flatten_and_nest() {
    // a | b | c → one row of three.
    let svg = (plain() | plain() | plain())
        .render_svg_native_with_size(900, 300)
        .unwrap();
    let c = cells(&svg);
    assert_eq!(c.len(), 3);
    assert!(c.iter().all(|cell| cell.1 == 0.0 && cell.2 == 300.0));
    assert!(svg.contains("data-grid=\"1 3\""));

    // (a | b) / c → two rows, the top one split in two.
    let svg = ((plain() | plain()) / plain())
        .tag_levels(TagLevels::Lower)
        .render_svg_native_with_size(600, 400)
        .unwrap();
    well_formed(&svg);
    let c = cells(&svg);
    assert_eq!(c.len(), 3);
    assert_eq!((c[0].2, c[0].3), (300.0, 200.0));
    assert_eq!((c[2].0, c[2].1, c[2].2), (0.0, 200.0, 600.0));
    assert!(svg.contains("data-grid=\"2 1\""));
    for t in ["a", "b", "c"] {
        assert!(svg.contains(&format!("data-panel=\"{t}\"")));
    }
}

#[test]
fn relative_widths_and_heights() {
    let svg = PlotGrid::new()
        .add(plain())
        .add(plain())
        .add(plain())
        .add_spacer()
        .ncol(2)
        .widths(&[3.0, 1.0])
        .heights(&[1.0, 3.0])
        .render_svg_native_with_size(800, 800)
        .unwrap();
    let c = cells(&svg);
    assert_eq!(c.len(), 3, "spacer renders nothing");
    assert_eq!((c[0].2, c[0].3), (600.0, 200.0));
    assert_eq!((c[1].0, c[1].2), (600.0, 200.0));
    assert_eq!((c[2].1, c[2].3), (200.0, 600.0));
}

#[test]
fn collected_identical_legends_are_drawn_once() {
    // Different colour groupings → two distinct legends …
    let other = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("x"))
        .geom_point();
    let svg = (scatter() | other)
        .collect_legends(true)
        .render_svg_native_with_size(800, 400)
        .unwrap();
    well_formed(&svg);
    assert_eq!(svg.matches("<g class=\"legend\"").count(), 2);
    // Same grouping drawn by different geoms → the same legend, once.
    let svg = (scatter() | line())
        .collect_legends(true)
        .render_svg_native_with_size(800, 400)
        .unwrap();
    assert_eq!(svg.matches("<g class=\"legend\"").count(), 1);

    let svg = (scatter() | scatter() | scatter())
        .collect_legends(true)
        .render_svg_native_with_size(900, 400)
        .unwrap();
    // … while identical legends collapse to one.
    assert_eq!(svg.matches("<g class=\"legend\"").count(), 1);
    // The sub-plots reserve no legend space: the panel spans most of the cell.
    assert_eq!(svg.matches(">ctrl<").count(), 1);

    let without = (scatter() | scatter())
        .render_svg_native_with_size(800, 400)
        .unwrap();
    assert_eq!(
        without.matches(">ctrl<").count(),
        2,
        "legends kept by default"
    );
}

#[test]
fn legend_strip_at_bottom() {
    let svg = (scatter() / scatter())
        .collect_legends(true)
        .legend_position(GridLegendPosition::Bottom)
        .render_svg_native_with_size(500, 700)
        .unwrap();
    let c = cells(&svg);
    assert!(
        c[1].1 + c[1].3 < 700.0 - 10.0,
        "strip below the grid: {c:?}"
    );
    assert_eq!(svg.matches("<g class=\"legend\"").count(), 1);
}

#[test]
fn nested_grids_and_explicit_tags_round_trip() {
    let inner = PlotGrid::new()
        .add(plain())
        .add(plain())
        .ncol(1)
        .title("inner");
    let svg = PlotGrid::new()
        .add(plain())
        .add(inner)
        .tag_levels(TagLevels::Numeric)
        .tag_affixes("(", ")")
        .render_svg_native_with_size(800, 400)
        .unwrap();
    well_formed(&svg);
    assert!(svg.contains("data-panel=\"(1)\""));
    assert!(svg.contains("data-panel=\"(3)\""));
    assert!(svg.contains(">inner<"));
}

#[test]
fn untagged_panels_are_indexed() {
    let svg = (plain() | plain().tag("Z"))
        .render_svg_native_with_size(400, 200)
        .unwrap();
    assert!(svg.contains("data-panel=\"1\""));
    assert!(svg.contains("data-panel=\"Z\""), "own tag kept");
}

#[test]
fn degenerate_inputs_do_not_panic() {
    // Empty grid, zero size, absurd widths, more cells than fit.
    let svg = PlotGrid::new().render_svg_native_with_size(0, 0).unwrap();
    well_formed(&svg);
    let svg = PlotGrid::new()
        .add(plain())
        .widths(&[f64::NAN, f64::INFINITY])
        .heights(&[-1.0])
        .spacing(f64::NAN)
        .render_svg_native_with_size(10, 10)
        .unwrap();
    well_formed(&svg);
    let (svg, warnings) = PlotGrid::new()
        .add(plain())
        .add(plain())
        .add(plain())
        .ncol(1)
        .nrow(1)
        .render_svg_native_with_warnings(300, 300)
        .unwrap();
    well_formed(&svg);
    // nrow grows to fit: nothing skipped.
    assert!(warnings.iter().all(|w| !w.contains("does not fit")));
    assert_eq!(cells(&svg).len(), 3);
}

#[test]
fn text_is_escaped_and_warnings_are_prefixed() {
    let bad = GGPlot::new(vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(f64::NAN), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point();
    let (svg, warnings) = (bad | plain())
        .title("<script>alert(1)</script>")
        .tag_levels(TagLevels::Custom(vec!["\"q\"".into()]))
        .render_svg_native_with_warnings(600, 300)
        .unwrap();
    assert!(!svg.contains("<script>"));
    assert!(svg.contains("data-panel=\"&quot;q&quot;\""));
    assert!(
        warnings
            .iter()
            .any(|w| w.starts_with("panel \"q\": ") && w.contains("non-finite")),
        "{warnings:?}"
    );
}

#[test]
fn validation_errors_propagate() {
    let ragged = GGPlot::new(vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0)]),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point();
    assert!((ragged | plain()).render_svg_native().is_err());
}

#[test]
fn ggarrange_delegates_to_plot_grid() {
    let svg = ggarrange(vec![plain(), plain(), plain()], 2, 300, 200).unwrap();
    well_formed(&svg);
    assert!(svg.contains("width=\"600\" height=\"400\""));
    assert_eq!(cells(&svg).len(), 3);
    assert!(svg.contains("data-panel=\"3\""));
}

#[test]
fn fragment_for_dashboards() {
    let frag = (plain() | plain())
        .render_svg_native_at(10.0, 20.0, 400, 200)
        .unwrap();
    assert!(frag.starts_with("<svg x=\"10.00\" y=\"20.00\""));
    assert!(!frag.contains("xmlns"));
}
