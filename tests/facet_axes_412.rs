//! #412: `facet_axes` — axes on every panel (`axes = "all"`), optionally
//! labelled only on the outer panels (`axis.labels = "margins"`); a free
//! scale under `facet_wrap` labels every panel's axis, as in ggplot2.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let n = 24;
    vec![
        ("x".into(), (0..n).map(|i| Value::Float(i as f64)).collect()),
        (
            "y".into(),
            (0..n)
                .map(|i| {
                    Value::Float(if i % 2 == 0 {
                        i as f64
                    } else {
                        i as f64 * 100.0
                    })
                })
                .collect(),
        ),
        (
            "r".into(),
            (0..n)
                .map(|i| Value::Str(["r1", "r2"][i % 2].into()))
                .collect(),
        ),
        (
            "c".into(),
            (0..n)
                .map(|i| Value::Str(["c1", "c2"][(i / 2) % 2].into()))
                .collect(),
        ),
    ]
}

fn svg(p: GGPlot) -> String {
    p.render_svg_native_with_size(500, 400).unwrap()
}

fn grid() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .facet_grid(Some("r"), Some("c"))
}

fn count(s: &str, label: &str) -> usize {
    s.matches(&format!(">{label}<")).count()
}

#[test]
fn axes_all_labels_every_panel() {
    let base = svg(grid());
    let all = svg(grid().facet_axes(FacetAxes::all()));
    // 2 × 2 grid: x labels under 2 panels by default, under all 4 with "all".
    assert_eq!(count(&base, "20"), 2);
    assert_eq!(count(&all, "20"), 4);
    let margins = svg(grid().facet_axes(FacetAxes::all().margin_labels()));
    assert_eq!(count(&margins, "20"), 2);
    // …but inner panels still get their tick marks.
    let lines = |s: &str| s.matches("<line").count() + s.matches("<polyline").count();
    assert!(lines(&margins) > lines(&base));
}

#[test]
fn free_wrap_scales_label_every_panel() {
    let s = svg(GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .facet_wrap_free("r", None, FacetScales::FreeY));
    // r1 spans 0..22, r2 100..2300: both panels' y labels are drawn.
    assert!(count(&s, "2000") >= 1 && count(&s, "20") >= 1, "{s}");
}
