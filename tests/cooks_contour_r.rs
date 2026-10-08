//! U4 — Cook's-distance contours on a residuals-vs-leverage plot, checked
//! against R (`plot.lm(which = 5)`'s contour formula and `cooks.distance` of
//! `lm(mpg ~ wt + hp, mtcars)`; fixtures from
//! `validation/generate_diagnostics.R`).

use ggplot_rs::aes::{Aes, Aesthetic};
use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::geom::cooks::{cooks_contour_y, cooks_distance, GeomCooksContour};
use ggplot_rs::prelude::*;

const DIR: &str = "validation/fixtures/diag";

fn csv(name: &str) -> DataFrame {
    DataFrame::from_csv(&format!("{DIR}/{name}")).unwrap()
}
fn num(df: &DataFrame, col: &str, i: usize) -> f64 {
    df.column(col).unwrap()[i].as_f64().unwrap()
}

#[test]
fn contour_curve_matches_plot_lm() {
    let want = csv("cooks_contour.csv");
    assert!(want.nrows() > 0);
    for i in 0..want.nrows() {
        let (level, h, y) = (
            num(&want, "level", i),
            num(&want, "h", i),
            num(&want, "y", i),
        );
        let got = cooks_contour_y(level, 3.0, h);
        assert!(
            (got - y).abs() < 1e-12,
            "level {level} h {h}: {got} vs R {y}"
        );
    }
}

#[test]
fn contour_is_the_cooks_distance_level_set_on_a_real_fit() {
    // R: lm(mpg ~ wt + hp, mtcars) has p = 3; D_i = r_i² h_i / (p (1 − h_i)).
    let infl = csv("cooks_mtcars.csv");
    assert_eq!(infl.nrows(), 32);
    for i in 0..infl.nrows() {
        let (h, r, d) = (num(&infl, "h", i), num(&infl, "r", i), num(&infl, "d", i));
        assert!((cooks_distance(r, h, 3.0) - d).abs() < 1e-10 * d.max(1.0));
        // A point is outside the level-D contour iff its Cook's D exceeds D.
        assert_eq!(r.abs() > cooks_contour_y(0.5, 3.0, h), d > 0.5);
    }
}

fn panel(svg: &str) -> (f64, f64, f64, f64) {
    let s = svg.split("data-plot=\"").nth(1).unwrap();
    let v: Vec<f64> = s[..s.find('"').unwrap()]
        .split(' ')
        .map(|t| t.parse().unwrap())
        .collect();
    (v[0], v[1], v[2], v[3])
}

fn contour_lines(svg: &str) -> Vec<Vec<(f64, f64)>> {
    svg.split("<polyline points=\"")
        .skip(1)
        .filter(|p| p.contains("Cook&#39;s distance"))
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

#[test]
fn residuals_vs_leverage_with_contours_on_native_svg() {
    let infl = csv("cooks_mtcars.csv");
    let plot = || {
        GGPlot::new(infl.clone())
            .aes(Aes::new().x("h").y("r"))
            .geom_point()
            .stat_cooks_contour(3, &[0.5, 1.0])
    };
    let svg = plot().render_svg_native_with_size(480, 360).unwrap();
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
    let lines = contour_lines(&svg);
    assert!(lines.len() >= 2, "upper and lower curves: {}", lines.len());
    let (px, py, pw, ph) = panel(&svg);
    for l in &lines {
        for &(x, y) in l {
            assert!(x >= px - 0.01 && x <= px + pw + 0.01, "x {x} outside panel");
            assert!(y >= py - 0.01 && y <= py + ph + 0.01, "y {y} outside panel");
        }
    }
    assert!(svg.contains("stroke-dasharray"));
    assert!(svg.contains(r#"data-value="0.5""#) && svg.contains(r#"data-value="1""#));
    assert!(svg.contains(">0.5</text>"), "level label");
    // The contours never widen the axes.
    let built = plot().try_build().unwrap();
    let (lo, hi) = built.scales.get(&Aesthetic::Y).unwrap().domain().unwrap();
    let r: Vec<f64> = (0..32).map(|i| num(&infl, "r", i)).collect();
    assert_eq!(lo, r.iter().cloned().fold(f64::INFINITY, f64::min));
    assert_eq!(hi, r.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
}

#[test]
fn contours_appear_in_every_facet_panel_and_respect_h_range() {
    let infl = csv("cooks_mtcars.csv");
    let model: Vec<Value> = (0..32)
        .map(|i| Value::Str(if i % 2 == 0 { "m1" } else { "m2" }.into()))
        .collect();
    let mut df = infl.clone();
    df.add_column("model".into(), model);
    let built = GGPlot::new(df.clone())
        .aes(Aes::new().x("h").y("r"))
        .geom_point()
        .stat_cooks_contour(3, &[0.5])
        .facet_wrap("model", None)
        .try_build()
        .unwrap();
    assert_eq!(built.panels.len(), 2);
    assert!(built.panels_data.iter().all(|p| p[1].nrows() == 1));

    let full = GGPlot::new(infl.clone())
        .aes(Aes::new().x("h").y("r"))
        .geom_point()
        .stat_cooks_contour(3, &[0.5])
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let narrow = GGPlot::new(infl)
        .aes(Aes::new().x("h").y("r"))
        .geom_point()
        .geom_cooks_contour_with(GeomCooksContour::new(3, &[0.5]).with_h_range(0.2, 0.3))
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let span = |svg: &str| {
        let xs: Vec<f64> = contour_lines(svg).iter().flatten().map(|p| p.0).collect();
        xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            - xs.iter().cloned().fold(f64::INFINITY, f64::min)
    };
    assert!(span(&narrow) < span(&full));
}

#[test]
fn contours_on_degenerate_axes_draw_nothing_without_panicking() {
    let data: Vec<(String, Vec<Value>)> = vec![
        ("h".into(), vec![Value::Str("a".into())]),
        ("r".into(), vec![Value::Float(1.0)]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("h").y("r"))
        .geom_point()
        .stat_cooks_contour(2, &[0.5, f64::NAN, -1.0])
        .render_svg_native_with_size(200, 200)
        .unwrap();
    assert!(contour_lines(&svg).is_empty());
    // Negative leverages only: nothing in (0, 1].
    let data: Vec<(String, Vec<Value>)> = vec![
        ("h".into(), vec![Value::Float(-3.0), Value::Float(-2.0)]),
        ("r".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("h").y("r"))
        .geom_point()
        .stat_cooks_contour(2, &[0.5])
        .render_svg_native_with_size(200, 200)
        .unwrap();
    assert!(contour_lines(&svg).is_empty());
}
