//! U2 — QQ stats against R: ggplot2 4.0 `stat_qq` / `stat_qq_line` with
//! `distribution`/`dparams`, and qqplotr 0.0.7 `stat_qq_band` (pointwise and
//! KS). Fixtures from `validation/generate_diagnostics.R`.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::prelude::*;
use ggplot_rs::scale::ScaleSet;
use ggplot_rs::stat::qq::{QQBandType, QQDistribution, StatQQBand, StatQQDist, StatQQLineDist};
use ggplot_rs::stat::Stat;

const DIR: &str = "validation/fixtures/diag";

fn csv(name: &str) -> DataFrame {
    DataFrame::from_csv(&format!("{DIR}/{name}")).unwrap()
}

fn rows_where(df: &DataFrame, filters: &[(&str, &str)]) -> Vec<usize> {
    (0..df.nrows())
        .filter(|&i| {
            filters
                .iter()
                .all(|(c, v)| df.column(c).unwrap()[i].as_str() == Some(*v))
        })
        .collect()
}

fn num(df: &DataFrame, col: &str, i: usize) -> f64 {
    df.column(col).unwrap()[i].as_f64().unwrap()
}

fn assert_close(got: f64, want: f64, what: &str) {
    if want.is_infinite() {
        assert_eq!(got, want, "{what}");
        return;
    }
    let tol = 1e-9 * want.abs().max(1.0);
    assert!((got - want).abs() <= tol, "{what}: got {got}, R {want}");
}

/// (fixture key, input column, distribution)
fn cases() -> Vec<(&'static str, &'static str, QQDistribution)> {
    vec![
        ("norm", "resid", QQDistribution::normal()),
        ("t5", "resid", QQDistribution::t(5.0)),
        ("exp", "pos", QQDistribution::Exponential { rate: 0.5 }),
        ("halfnorm", "pos", QQDistribution::half_normal()),
    ]
}

fn sample(col: &str) -> DataFrame {
    let input = csv("qq_input.csv");
    let mut df = DataFrame::new();
    df.add_column("y".into(), input.column(col).unwrap().to_vec());
    df
}

#[test]
fn stat_qq_matches_ggplot2_for_each_distribution() {
    let want = csv("qq_points.csv");
    for (key, col, dist) in cases() {
        let got = StatQQDist::new(dist).compute_group(&sample(col), &ScaleSet::new());
        let rows = rows_where(&want, &[("dist", key)]);
        assert_eq!(got.nrows(), rows.len(), "{key}");
        for (j, &i) in rows.iter().enumerate() {
            assert_close(
                num(&got, "x", j),
                num(&want, "x", i),
                &format!("{key} x[{j}]"),
            );
            assert_close(
                num(&got, "y", j),
                num(&want, "y", i),
                &format!("{key} y[{j}]"),
            );
        }
    }
}

#[test]
fn stat_qq_line_matches_ggplot2_for_each_distribution() {
    let want = csv("qq_line.csv");
    for (key, col, dist) in cases() {
        let got = StatQQLineDist::new(dist).compute_group(&sample(col), &ScaleSet::new());
        let rows = rows_where(&want, &[("dist", key)]);
        assert_eq!(got.nrows(), 2);
        for (j, &i) in rows.iter().enumerate() {
            assert_close(num(&got, "x", j), num(&want, "x", i), &format!("{key} x"));
            assert_close(num(&got, "y", j), num(&want, "y", i), &format!("{key} y"));
        }
    }
}

#[test]
fn stat_qq_band_matches_qqplotr_pointwise_and_ks() {
    let want = csv("qq_band.csv");
    for (key, col, dist) in cases() {
        for (band, label) in [(QQBandType::Pointwise, "pointwise"), (QQBandType::Ks, "ks")] {
            let got = StatQQBand::new(dist)
                .band(band)
                .level(0.95)
                .compute_group(&sample(col), &ScaleSet::new());
            let rows = rows_where(&want, &[("dist", key), ("band", label)]);
            assert_eq!(got.nrows(), rows.len(), "{key} {label}");
            for (j, &i) in rows.iter().enumerate() {
                let what = format!("{key} {label} row {j}");
                assert_close(num(&got, "x", j), num(&want, "x", i), &what);
                assert_close(num(&got, "ymin", j), num(&want, "ymin", i), &what);
                assert_close(num(&got, "ymax", j), num(&want, "ymax", i), &what);
            }
        }
    }
}

#[test]
fn qq_plot_pipeline_uses_the_same_numbers() {
    // Through the full build (scales, groups) the layer data equals the stat.
    let input = csv("qq_input.csv");
    let built = GGPlot::new(input)
        .aes(Aes::new().y("resid"))
        .stat_qq_band(StatQQBand::new(QQDistribution::t(5.0)))
        .stat_qq(QQDistribution::t(5.0))
        .stat_qq_line(QQDistribution::t(5.0))
        .try_build()
        .unwrap();
    let want = csv("qq_points.csv");
    let rows = rows_where(&want, &[("dist", "t5")]);
    let pts = &built.layers[1].data;
    for (j, &i) in rows.iter().enumerate() {
        assert_close(num(pts, "x", j), num(&want, "x", i), "pipeline x");
    }
    // Band + line + points all on the native SVG path, no NaN.
    let input = csv("qq_input.csv");
    let svg = GGPlot::new(input)
        .aes(Aes::new().y("resid"))
        .stat_qq_band(StatQQBand::default().band(QQBandType::Ks))
        .geom_qq()
        .geom_qq_line()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(svg.contains("<polygon"), "band drawn");
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
    assert_eq!(svg.matches("<circle").count(), 30);
    assert!(svg.contains("data-x=\"-2.128"), "points carry data-x");
}

#[test]
fn grouped_qq_draws_a_line_and_band_per_group() {
    let mut y = Vec::new();
    let mut g = Vec::new();
    for k in 0..40 {
        y.push(Value::Float(
            ((k * 7) % 13) as f64 + if k < 20 { 0.0 } else { 5.0 },
        ));
        g.push(Value::Str(if k < 20 { "a" } else { "b" }.into()));
    }
    let data: Vec<(String, Vec<Value>)> = vec![("y".into(), y), ("g".into(), g)];
    let svg = GGPlot::new(data)
        .aes(Aes::new().y("y").color("g").fill("g"))
        .geom_qq_band()
        .geom_qq()
        .geom_qq_line()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(svg.matches("<polygon").count(), 2, "one band per group");
    let qq_lines = svg
        .split("<polyline")
        .filter(|p| p.contains("stroke-dasharray") && p.contains("data-series"))
        .count();
    assert_eq!(qq_lines, 2, "one qq line per group");
}

#[test]
fn degenerate_qq_inputs_do_not_panic() {
    for vals in [
        vec![],
        vec![1.0],
        vec![2.0, 2.0, 2.0],
        vec![f64::NAN, 1.0, 2.0],
    ] {
        let data: Vec<(String, Vec<Value>)> =
            vec![("y".into(), vals.iter().map(|&v| Value::Float(v)).collect())];
        let svg = GGPlot::new(data)
            .aes(Aes::new().y("y"))
            .geom_qq_band()
            .stat_qq(QQDistribution::exponential())
            .stat_qq_line(QQDistribution::t(3.0))
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert!(!svg.contains("NaN"), "{vals:?}");
    }
}
