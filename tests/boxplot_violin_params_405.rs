//! #405: ggplot2's geom_boxplot / geom_violin parameters.

use ggplot_rs::prelude::*;
use ggplot_rs::stat::boxplot::StatBoxplot;
use ggplot_rs::stat::ydensity::StatYDensity;
use ggplot_rs::stat::Stat;

fn frame(groups: &[(&str, Vec<f64>)]) -> Vec<(String, Vec<Value>)> {
    let (mut g, mut y) = (Vec::new(), Vec::new());
    for (name, vals) in groups {
        for v in vals {
            g.push(Value::Str(name.to_string()));
            y.push(Value::Float(*v));
        }
    }
    vec![("g".into(), g), ("y".into(), y)]
}

fn df(vals: &[f64]) -> ggplot_rs::data::DataFrame {
    let mut d = ggplot_rs::data::DataFrame::new();
    d.add_column("x".into(), vec![Value::Str("a".into()); vals.len()]);
    d.add_column("y".into(), vals.iter().map(|v| Value::Float(*v)).collect());
    d
}

fn svg(plot: GGPlot) -> String {
    plot.render_svg_native_with_size(500, 400).unwrap()
}

#[test]
fn coef_moves_the_fences_and_n_is_reported() {
    let vals = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 20.0];
    let scales = ggplot_rs::scale::ScaleSet::new();
    let outliers = |s: StatBoxplot| {
        s.compute_group(&df(&vals), &scales)
            .column("outliers")
            .map(|c| c[0].as_str().unwrap_or("").to_string())
    };
    assert_eq!(outliers(StatBoxplot).as_deref(), Some("20"));
    // A long reach keeps 20 inside the whiskers (R: coef = 5 → no outliers).
    assert_eq!(outliers(StatBoxplot::with_coef(5.0)), None);
    let out = StatBoxplot.compute_group(&df(&vals), &scales);
    assert_eq!(out.column("n").unwrap()[0].as_f64(), Some(10.0));
}

#[test]
fn notch_draws_a_polygon_and_hidden_outliers_vanish() {
    let data = frame(&[
        ("a", vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 30.0]),
        ("b", vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]),
    ]);
    let plain = svg(GGPlot::new(data.clone())
        .aes(Aes::new().x("g").y("y"))
        .geom_boxplot());
    assert!(plain.contains("<circle"), "default draws the outlier");
    let notched = svg(GGPlot::new(data.clone())
        .aes(Aes::new().x("g").y("y"))
        .geom_boxplot_with(GeomBoxplot::default().notched().hide_outliers()));
    assert_eq!(
        notched.matches("<polygon").count(),
        2,
        "one notched box each"
    );
    assert!(!notched.contains("<circle"), "outliers hidden");
    // Hidden outliers no longer stretch the y axis to 30.
    assert!(plain.contains(">30<") && !notched.contains(">30<"));
}

#[test]
fn varwidth_narrows_the_smaller_group() {
    let data = frame(&[
        ("a", (0..100).map(f64::from).collect()),
        ("b", vec![1.0, 2.0, 3.0, 4.0]),
    ]);
    let widths = |s: &str| -> Vec<f64> {
        s.split("<rect")
            .skip(1)
            .filter_map(|t| {
                let a = t.find(" width=\"")? + 8;
                t[a..].split('"').next()?.parse().ok()
            })
            .filter(|w: &f64| *w > 5.0 && *w < 300.0)
            .collect()
    };
    let w = widths(&svg(GGPlot::new(data)
        .aes(Aes::new().x("g").y("y"))
        .geom_boxplot_with(GeomBoxplot::default().varwidth())));
    assert_eq!(w.len(), 2, "{w:?}");
    // √4 / √100 = 0.2.
    assert!((w[1] / w[0] - 0.2).abs() < 0.02, "{w:?}");
}

#[test]
fn violin_trim_bw_and_quantiles_in_the_stat() {
    let vals: Vec<f64> = (0..40).map(|i| (i % 10) as f64).collect();
    let scales = ggplot_rs::scale::ScaleSet::new();
    let range = |s: &StatYDensity| {
        let out = s.compute_group(&df(&vals), &scales);
        let y: Vec<f64> = out
            .column("y")
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        (y[0], *y.last().unwrap())
    };
    assert_eq!(range(&StatYDensity::default()), (0.0, 9.0));
    let wide = StatYDensity {
        trim: false,
        bw: Some(1.0),
        ..Default::default()
    };
    let (lo, hi) = range(&wide);
    assert!(
        (lo + 3.0).abs() < 1e-9 && (hi - 12.0).abs() < 1e-9,
        "±3·bw: {lo} {hi}"
    );
    let q = StatYDensity {
        quantiles: vec![0.25, 0.5, 0.75],
        ..Default::default()
    }
    .compute_group(&df(&vals), &scales);
    // R: quantile(rep(0:9, 4), c(.25, .5, .75)) = 2, 4.5, 7.
    assert_eq!(q.column("quantiles").unwrap()[0].as_str(), Some("2,4.5,7"));
}

#[test]
fn violin_scale_modes_and_quantile_lines() {
    let data = frame(&[
        ("a", (0..80).map(|i| (i % 8) as f64).collect()),
        ("b", (0..10).map(|i| (i % 5) as f64 * 2.0).collect()),
    ]);
    let max_width = |scale: ViolinScale| -> Vec<f64> {
        let s = svg(GGPlot::new(data.clone())
            .aes(Aes::new().x("g").y("y"))
            .geom_violin_with(GeomViolin::default().with_scale(scale)));
        s.split("<polygon")
            .skip(1)
            .map(|t| {
                let a = t.find("points=\"").unwrap() + 8;
                let xs: Vec<f64> = t[a..]
                    .split('"')
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .filter_map(|p| p.split(',').next()?.parse().ok())
                    .collect();
                xs.iter().cloned().fold(f64::MIN, f64::max)
                    - xs.iter().cloned().fold(f64::MAX, f64::min)
            })
            .collect()
    };
    let width = max_width(ViolinScale::Width);
    assert!(
        (width[0] - width[1]).abs() < 1.0,
        "same max width: {width:?}"
    );
    let count = max_width(ViolinScale::Count);
    assert!(
        (count[1] / count[0] - 10.0 / 80.0).abs() < 0.02,
        "∝ n: {count:?}"
    );
    let lines = svg(GGPlot::new(data)
        .aes(Aes::new().x("g").y("y"))
        .geom_violin_with(GeomViolin::default().with_quantiles([0.25, 0.5, 0.75])));
    assert!(lines.matches("<line").count() + lines.matches("<polyline").count() >= 6);
}
