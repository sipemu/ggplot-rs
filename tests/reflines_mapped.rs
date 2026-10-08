//! U1 — data-mapped reference lines (`geom_hline/vline/abline` with
//! `yintercept` / `xintercept` / `slope` / `intercept` aesthetics), on the
//! plotters-free native SVG path.

use ggplot_rs::aes::{Aes, Aesthetic};
use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}
fn scatter() -> Vec<(String, Vec<Value>)> {
    frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0, 4.0])),
        ("y", floats(&[1.0, 2.0, 4.0, 5.0])),
        ("panel", strs(&["a", "a", "b", "b"])),
        ("g", strs(&["u", "v", "u", "v"])),
    ])
}

fn domain(plot: GGPlot, aes: Aesthetic) -> (f64, f64) {
    let built = plot.try_build().unwrap();
    built.scales.get(&aes).unwrap().domain().unwrap()
}

/// Data lines (with a hover title) — grid lines and ticks carry none.
fn polylines(svg: &str) -> usize {
    svg.matches("</polyline>").count()
}

#[test]
fn constant_hline_trains_the_y_scale_like_ggplot2() {
    let (lo, hi) = domain(
        GGPlot::new(scatter())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .geom_hline(10.0),
        Aesthetic::Y,
    );
    assert_eq!((lo, hi), (1.0, 10.0));
    let (lo, _) = domain(
        GGPlot::new(scatter())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .geom_vline(-3.0),
        Aesthetic::X,
    );
    assert_eq!(lo, -3.0);
}

#[test]
fn abline_does_not_train_scales() {
    let (lo, hi) = domain(
        GGPlot::new(scatter())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .geom_abline(1.0, 100.0),
        Aesthetic::Y,
    );
    assert_eq!((lo, hi), (1.0, 5.0));
}

#[test]
fn mapped_hlines_draw_one_line_per_row_with_host_attrs() {
    let bounds = frame(vec![
        ("b", floats(&[-0.5, 0.5, 7.0])),
        ("model", strs(&["m1", "m1", "m2"])),
    ]);
    let svg = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_hline_aes(Aes::new().yintercept("b").color("model"))
        .layer_data(bounds)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(polylines(&svg), 3, "{svg}");
    assert!(
        svg.contains(r#"data-series="m1" data-value="-0.5""#),
        "{svg}"
    );
    assert!(svg.contains(r#"data-series="m2" data-value="7""#));
    assert!(svg.contains("<title>m2: y = 7</title>"));
    // The intercepts trained y (7 > max(y) = 5) …
    let built = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_hline_aes(Aes::new().yintercept("b"))
        .layer_data(frame(vec![("b", floats(&[-0.5, 7.0]))]))
        .try_build()
        .unwrap();
    assert_eq!(
        built.scales.get(&Aesthetic::Y).unwrap().domain(),
        Some((-0.5, 7.0))
    );
    // … and created no scale of their own.
    assert!(built.scales.get(&Aesthetic::Yintercept).is_none());
}

#[test]
fn mapped_lines_are_coloured_by_group() {
    let lines = frame(vec![
        ("b", floats(&[2.0, 3.0])),
        ("model", strs(&["m1", "m2"])),
    ]);
    let svg = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_vline_aes(Aes::new().xintercept("b").color("model").linetype("model"))
        .layer_data(lines)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let strokes: Vec<&str> = svg
        .split("<polyline")
        .skip(1)
        .filter(|p| p.contains("data-series=\"m"))
        .map(|p| {
            let i = p.find("stroke=\"").unwrap() + 8;
            &p[i..i + 7]
        })
        .collect();
    assert_eq!(strokes.len(), 2);
    assert_ne!(strokes[0], strokes[1], "distinct colours per model");
    // Linetype mapped too: one solid, one dashed.
    let dashed = svg
        .split("<polyline")
        .filter(|p| p.contains("data-series=\"m") && p.contains("stroke-dasharray"))
        .count();
    assert_eq!(dashed, 1, "{svg}");
}

#[test]
fn mapped_lines_follow_facet_panels() {
    let lines = frame(vec![
        ("b", floats(&[1.5, 4.5])),
        ("panel", strs(&["a", "b"])),
    ]);
    let built = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_hline_aes(Aes::new().yintercept("b"))
        .layer_data(lines)
        .geom_hline(0.0)
        .facet_wrap("panel", None)
        .try_build()
        .unwrap();
    assert_eq!(built.panels.len(), 2);
    for panel in &built.panels_data {
        // One mapped line per panel; the constant line in every panel.
        assert_eq!(panel[1].nrows(), 1);
        assert_eq!(panel[2].nrows(), 1);
    }
    let a = built.panels_data[0][1].column("yintercept").unwrap()[0].as_f64();
    assert_eq!(a, Some(1.5));
}

#[test]
fn reflines_do_not_inherit_the_plot_mapping() {
    let svg = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .geom_hline(3.0)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let hline = svg
        .split("<polyline")
        .find(|p| p.contains("data-value=\"3\""))
        .expect("hline drawn");
    assert!(!hline.contains("data-series"), "{hline}");
}

#[test]
fn numeric_vline_on_discrete_axis_sits_between_categories() {
    let data = frame(vec![
        ("term", strs(&["a", "b", "c"])),
        ("est", floats(&[1.0, 2.0, 3.0])),
    ]);
    let built = GGPlot::new(data.clone())
        .geom_vline(1.5)
        .aes(Aes::new().x("term").y("est"))
        .geom_point()
        .try_build()
        .unwrap();
    let x = built.scales.get(&Aesthetic::X).unwrap();
    assert!(
        x.is_discrete(),
        "added-first vline must not force continuous x"
    );
    assert_eq!(x.breaks().len(), 3, "1.5 is not a level");
    let svg = GGPlot::new(data)
        .geom_vline(1.5)
        .aes(Aes::new().x("term").y("est"))
        .geom_point()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(svg.contains("data-x=\"1.5\""));
}

#[test]
fn non_finite_intercepts_are_dropped_with_a_warning() {
    let lines = frame(vec![("b", floats(&[f64::NAN, 2.0, f64::INFINITY]))]);
    let (svg, warnings) = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_hline_aes(Aes::new().yintercept("b"))
        .layer_data(lines)
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert!(warnings
        .iter()
        .any(|w| w.contains("geom_hline: removed 2 rows")));
    assert_eq!(polylines(&svg), 1);
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
}

#[test]
fn mapped_ablines_in_data_space_and_clipped() {
    // Identity line on [1,4]×[1,5]: both endpoints inside the panel.
    let lines = frame(vec![
        ("s", floats(&[1.0, 0.0])),
        ("i", floats(&[0.0, 99.0])),
    ]);
    let svg = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_abline_aes(Aes::new().slope("s").intercept("i"))
        .layer_data(lines)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    // y = 99 misses the panel entirely: only the identity line is drawn.
    assert_eq!(polylines(&svg), 1, "{svg}");
    let plot = svg.split("data-plot=\"").nth(1).unwrap();
    let nums: Vec<f64> = plot[..plot.find('"').unwrap()]
        .split(' ')
        .map(|v| v.parse().unwrap())
        .collect();
    let (px, py, pw, ph) = (nums[0], nums[1], nums[2], nums[3]);
    let line = svg
        .split("<polyline points=\"")
        .find(|p| p.contains("y = 0 + 1"))
        .unwrap();
    let pts: Vec<(f64, f64)> = line[..line.find('"').unwrap()]
        .split(' ')
        .map(|p| {
            let (a, b) = p.split_once(',').unwrap();
            (a.parse().unwrap(), b.parse().unwrap())
        })
        .collect();
    for (x, y) in &pts {
        assert!(*x >= px - 0.01 && *x <= px + pw + 0.01);
        assert!(*y >= py - 0.01 && *y <= py + ph + 0.01);
    }
    // Back to data: the drawn line satisfies y = x.
    let xd: Vec<f64> = root_domain(&svg);
    let to_data = |(x, y): (f64, f64)| {
        (
            xd[0] + (x - px) / pw * (xd[1] - xd[0]),
            xd[2] + (py + ph - y) / ph * (xd[3] - xd[2]),
        )
    };
    for p in &pts {
        let (dx, dy) = to_data(*p);
        assert!((dx - dy).abs() < 0.02, "({dx}, {dy}) not on y = x");
    }
}

fn root_domain(svg: &str) -> Vec<f64> {
    let s = svg.split("data-domain=\"").nth(1).unwrap();
    s[..s.find('"').unwrap()]
        .split(' ')
        .map(|v| v.parse().unwrap())
        .collect()
}

#[test]
fn reflines_render_under_coord_flip_and_free_facets() {
    let svg = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_hline(0.0)
        .geom_vline_aes(Aes::new().xintercept("x"))
        .geom_abline(1.0, 0.0)
        .coord_flip()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(!svg.contains("NaN"));
    let svg = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_hline(0.0)
        .facet_wrap_free("panel", None, FacetScales::FreeY)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(!svg.contains("NaN"));
}

#[test]
fn refline_series_are_escaped() {
    let lines = frame(vec![
        ("b", floats(&[2.0])),
        ("m", strs(&["<script>alert(1)</script>"])),
    ]);
    let svg = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_hline_aes(Aes::new().yintercept("b").color("m"))
        .layer_data(lines)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(!svg.contains("<script>"));
}
