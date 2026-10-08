//! U7 — Kaplan–Meier / ECDF building blocks on the native SVG path: step
//! ribbons, censor marks, `stat_ecdf` with a DKW band (checked against R),
//! and horizontal error bars.

use ggplot_rs::aes::{Aes, Aesthetic};
use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::prelude::*;
use ggplot_rs::scale::ScaleSet;
use ggplot_rs::stat::Stat;

const DIR: &str = "validation/fixtures/diag";

fn csv(name: &str) -> DataFrame {
    DataFrame::from_csv(&format!("{DIR}/{name}")).unwrap()
}
fn num(df: &DataFrame, col: &str, i: usize) -> f64 {
    df.column(col).unwrap()[i].as_f64().unwrap()
}
fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}
fn polygon_points(svg: &str) -> Vec<Vec<(f64, f64)>> {
    svg.split("<polygon points=\"")
        .skip(1)
        .map(|p| {
            p[..p.find('"').unwrap()]
                .split(' ')
                .map(|xy| {
                    let (a, b) = xy.split_once(',').unwrap();
                    (a.parse().unwrap(), b.parse().unwrap())
                })
                .collect()
        })
        .collect()
}
/// The opening tags (`<tag …>`) of every `tag` element.
fn tags<'a>(svg: &'a str, tag: &str) -> Vec<&'a str> {
    svg.split(&format!("<{tag} "))
        .skip(1)
        .map(|p| &p[..p.find('>').unwrap()])
        .collect()
}
fn panel(svg: &str) -> (f64, f64, f64, f64) {
    let s = svg.split("data-plot=\"").nth(1).unwrap();
    let v: Vec<f64> = s[..s.find('"').unwrap()]
        .split(' ')
        .map(|t| t.parse().unwrap())
        .collect();
    (v[0], v[1], v[2], v[3])
}

#[test]
fn ecdf_dkw_band_matches_r() {
    let input = csv("qq_input.csv");
    let mut df = DataFrame::new();
    df.add_column("x".into(), input.column("resid").unwrap().to_vec());
    let got = StatEcdfBand::new(0.95).compute_group(&df, &ScaleSet::new());
    let want = csv("ecdf_band.csv");
    assert_eq!(got.nrows(), want.nrows());
    for i in 0..want.nrows() {
        for col in ["x", "y", "ymin", "ymax"] {
            let (g, w) = (num(&got, col, i), num(&want, col, i));
            if w.is_infinite() {
                assert_eq!(g, w, "{col}[{i}]");
            } else {
                assert!((g - w).abs() < 1e-12, "{col}[{i}]: {g} vs R {w}");
            }
        }
    }
}

#[test]
fn ecdf_with_band_renders_per_group_and_reaches_the_panel_edges() {
    let input = csv("qq_input.csv");
    let n = input.nrows();
    let mut df = input.clone();
    df.add_column(
        "g".into(),
        (0..n)
            .map(|i| Value::Str(if i < n / 2 { "a" } else { "b" }.into()))
            .collect(),
    );
    let svg = GGPlot::new(df)
        .aes(Aes::new().x("resid").color("g").fill("g"))
        .stat_ecdf_band(0.95)
        .stat_ecdf()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
    let polys = polygon_points(&svg);
    assert_eq!(polys.len(), 2, "one band per group");
    let (px, _, pw, _) = panel(&svg);
    for p in &polys {
        let xs: Vec<f64> = p.iter().map(|q| q.0).collect();
        let lo = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!((lo - px).abs() < 0.01 && (hi - (px + pw)).abs() < 0.01);
    }
    let steps = tags(&svg, "polyline")
        .into_iter()
        .filter(|p| p.contains("data-series=\"a\"") || p.contains("data-series=\"b\""))
        .count();
    assert_eq!(steps, 2, "one ECDF step line per group");
}

/// Kaplan–Meier (survival::survfit on `lung`): step curve, step CI ribbon,
/// censor marks.
fn km_plot() -> GGPlot {
    let km = csv("km_lung.csv");
    GGPlot::new(km)
        .aes(Aes::new().x("time").y("surv"))
        .geom_stepribbon()
        .layer_aes(Aes::new().x("time").ymin("lower").ymax("upper"))
        .geom_step()
        .geom_censor_marks("n_censor")
}

#[test]
fn kaplan_meier_step_ribbon_and_censor_marks() {
    let km = csv("km_lung.csv");
    let n = km.nrows();
    let censored = (0..n).filter(|&i| num(&km, "n_censor", i) > 0.0).count();
    assert!(censored > 0);
    let svg = km_plot().render_svg_native_with_size(500, 350).unwrap();
    assert!(!svg.contains("NaN"));
    // One '+' path per censored time, each a hoverable mark.
    let plus = tags(&svg, "path")
        .into_iter()
        .filter(|p| p.starts_with("d=\"M") && p.contains('H') && p.contains('V'))
        .count();
    assert_eq!(plus, censored);
    // The CI band is a step polygon: upper edge with 2n − 1 vertices (hv).
    let polys = polygon_points(&svg);
    assert_eq!(polys.len(), 1);
    let band = &polys[0];
    assert_eq!(band.len(), 2 * (2 * n - 1));
    // hv: horizontal first — vertex 1 shares y with vertex 0, vertex 2
    // shares x with vertex 1.
    assert!((band[0].1 - band[1].1).abs() < 0.01);
    assert!((band[1].0 - band[2].0).abs() < 0.01);
}

#[test]
fn stratified_km_draws_a_curve_band_and_marks_per_stratum() {
    let data = frame(vec![
        ("t", floats(&[1.0, 3.0, 5.0, 2.0, 4.0, 6.0])),
        ("s", floats(&[0.9, 0.7, 0.5, 0.8, 0.6, 0.3])),
        ("lo", floats(&[0.8, 0.55, 0.3, 0.7, 0.4, 0.1])),
        ("hi", floats(&[1.0, 0.85, 0.7, 0.9, 0.8, 0.5])),
        ("cens", floats(&[0.0, 1.0, 0.0, 2.0, 0.0, 0.0])),
        ("arm", strs(&["A", "A", "A", "B", "B", "B"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("t").y("s").color("arm"))
        .geom_stepribbon_with(GeomStepribbon::new(StepDirection::Hv))
        .layer_aes(Aes::new().x("t").ymin("lo").ymax("hi").fill("arm"))
        .geom_step()
        .geom_censor_marks("cens")
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(polygon_points(&svg).len(), 2);
    for arm in ["A", "B"] {
        let marks = tags(&svg, "path")
            .into_iter()
            .filter(|p| p.contains(&format!("data-series=\"{arm}\"")))
            .count();
        assert_eq!(marks, 1, "one censor mark in arm {arm}");
        let lines = tags(&svg, "polyline")
            .into_iter()
            .filter(|p| p.contains(&format!("data-series=\"{arm}\"")))
            .count();
        assert_eq!(lines, 1, "one step curve for arm {arm}");
    }
}

#[test]
fn step_ribbon_directions() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("lo", floats(&[0.0, 1.0, 2.0])),
        ("hi", floats(&[1.0, 2.0, 3.0])),
    ]);
    for (dir, extra) in [
        (StepDirection::Hv, 2),
        (StepDirection::Vh, 2),
        (StepDirection::Mid, 4),
    ] {
        let svg = GGPlot::new(data.clone())
            .aes(Aes::new().x("x").ymin("lo").ymax("hi"))
            .geom_stepribbon_with(GeomStepribbon::new(dir))
            .render_svg_native_with_size(300, 200)
            .unwrap();
        let p = &polygon_points(&svg)[0];
        // 3 rows per edge plus the inserted corners, twice.
        assert_eq!(p.len(), 2 * (3 + extra));
    }
}

#[test]
fn horizontal_errorbars() {
    let data = frame(vec![
        ("term", strs(&["wt", "hp", "cyl"])),
        ("lo", floats(&[-5.0, -0.05, -2.0])),
        ("hi", floats(&[-2.0, 0.01, 0.5])),
    ]);
    let built = GGPlot::new(data.clone())
        .aes(Aes::new().y("term").xmin("lo").xmax("hi"))
        .geom_errorbarh()
        .try_build()
        .unwrap();
    assert_eq!(
        built.scales.get(&Aesthetic::X).unwrap().domain(),
        Some((-5.0, 0.5))
    );
    let svg = GGPlot::new(data)
        .aes(Aes::new().y("term").xmin("lo").xmax("hi"))
        .geom_errorbarh()
        .geom_vline(0.0)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(
        svg.contains(r#"data-x="hp" data-value="-0.05 0.01""#),
        "{svg}"
    );
    assert_eq!(svg.matches("<title>wt: [-5, -2]</title>").count(), 1);
}

#[test]
fn non_finite_rows_are_dropped_with_warnings() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0, 4.0])),
        ("y", floats(&[1.0, 0.8, f64::NAN, 0.5])),
        ("lo", floats(&[0.9, f64::NAN, 0.5, 0.3])),
        ("hi", floats(&[1.0, 0.9, 0.8, f64::INFINITY])),
        ("c", floats(&[0.0, 1.0, 1.0, 0.0])),
    ]);
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_stepribbon()
        .layer_aes(Aes::new().x("x").ymin("lo").ymax("hi"))
        .geom_step()
        .geom_censor_marks("c")
        .geom_errorbarh()
        .layer_aes(Aes::new().y("y").xmin("lo").xmax("hi"))
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
    assert!(warnings.iter().any(|w| w.starts_with("geom_stepribbon")));
    assert!(warnings.iter().any(|w| w.starts_with("geom_step:")));
}

#[test]
fn native_svg_draws_every_point_shape() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0])),
        ("y", floats(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0])),
        ("s", strs(&["a", "b", "c", "d", "e", "f"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").shape("s"))
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    // circle, triangle, square, diamond, ×, + — each one mark with data-x.
    let marks = svg.matches("data-x=").count();
    assert_eq!(marks, 6);
    let with_x = |tag: &str| {
        tags(&svg, tag)
            .into_iter()
            .filter(|t| t.contains("data-x="))
            .count()
    };
    assert_eq!(with_x("circle"), 1);
    assert_eq!(with_x("polygon"), 3, "triangle, square, diamond");
    assert_eq!(with_x("path"), 2, "cross and plus");
}
