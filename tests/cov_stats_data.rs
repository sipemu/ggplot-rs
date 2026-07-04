//! Coverage-focused tests for `src/stat/` and `src/data/`.
//!
//! Two goals:
//!   1. Drive every `Stat::compute_group` directly with tiny inputs, and also
//!      render a plot that exercises the stat through the full pipeline.
//!   2. Exercise the `GGData` input adapters (row/column/DataFrame, plus the
//!      feature-gated polars + arrow paths) and the public `DataFrame` methods.
//!
//! Run with:
//!   cargo test --test cov_stats_data
//!   cargo test --no-default-features --features arrow --test cov_stats_data

use std::collections::HashMap;

use ggplot_rs::data::{DataFrame, GGData, Value};
use ggplot_rs::prelude::*;
use ggplot_rs::scale::ScaleSet;
use ggplot_rs::stat::Stat;

// ═══════════════════════════════════════════════════════════════════════════
// Small data builders
// ═══════════════════════════════════════════════════════════════════════════

fn floats(vals: &[f64]) -> Vec<Value> {
    vals.iter().copied().map(Value::Float).collect()
}

fn strs(vals: &[&str]) -> Vec<Value> {
    vals.iter().map(|s| Value::Str((*s).to_string())).collect()
}

fn df_x(vals: &[f64]) -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column("x".to_string(), floats(vals));
    df
}

fn df_xy(xs: &[f64], ys: &[f64]) -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column("x".to_string(), floats(xs));
    df.add_column("y".to_string(), floats(ys));
    df
}

fn df_y(vals: &[f64]) -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column("y".to_string(), floats(vals));
    df
}

/// A tiny gridded (x, y, z) frame for contour-style stats.
fn df_grid() -> DataFrame {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut zs = Vec::new();
    for ix in 0..6 {
        for iy in 0..6 {
            let x = ix as f64;
            let y = iy as f64;
            xs.push(x);
            ys.push(y);
            zs.push((x - 2.5).powi(2) + (y - 2.5).powi(2));
        }
    }
    let mut df = DataFrame::new();
    df.add_column("x".to_string(), floats(&xs));
    df.add_column("y".to_string(), floats(&ys));
    df.add_column("z".to_string(), floats(&zs));
    df
}

// ═══════════════════════════════════════════════════════════════════════════
// Stats: direct compute_group
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stat_identity_compute() {
    let stat = ggplot_rs::stat::identity::StatIdentity;
    let scales = ScaleSet::new();
    let input = df_xy(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]);
    let out = stat.compute_group(&input, &scales);
    assert_eq!(out.nrows(), 3);
    assert!(out.has_column("x") && out.has_column("y"));
    assert_eq!(stat.name(), "identity");
}

#[test]
fn stat_count_compute() {
    let stat = ggplot_rs::stat::count::StatCount;
    let scales = ScaleSet::new();

    // Discrete x.
    let mut d = DataFrame::new();
    d.add_column("x".to_string(), strs(&["a", "b", "a", "a", "b"]));
    let out = stat.compute_group(&d, &scales);
    assert_eq!(out.nrows(), 2);
    let total: f64 = out
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .sum();
    assert_eq!(total, 5.0);

    // Numeric x (preserves float type) + a carried fill column.
    let mut dn = DataFrame::new();
    dn.add_column("x".to_string(), floats(&[1.0, 1.0, 2.0]));
    dn.add_column("fill".to_string(), strs(&["g", "g", "g"]));
    let out2 = stat.compute_group(&dn, &scales);
    assert_eq!(out2.nrows(), 2);
    assert!(out2.has_column("fill"));
}

#[test]
fn stat_bin_compute() {
    let scales = ScaleSet::new();

    // Default bins.
    let stat = ggplot_rs::stat::bin::StatBin::default();
    let out = stat.compute_group(&df_x(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]), &scales);
    assert!(out.has_column("y") && out.has_column("density"));
    let counted: f64 = out
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .sum();
    assert_eq!(counted, 6.0);

    // Explicit binwidth branch.
    let stat_bw = ggplot_rs::stat::bin::StatBin::default().with_binwidth(1.0);
    let out_bw = stat_bw.compute_group(&df_x(&[0.0, 1.0, 2.0, 3.0]), &scales);
    assert!(out_bw.nrows() >= 1);

    // with_bins builder + constant-value edge case.
    let stat_b = ggplot_rs::stat::bin::StatBin::default().with_bins(5);
    let out_const = stat_b.compute_group(&df_x(&[2.0, 2.0, 2.0]), &scales);
    assert!(out_const.nrows() >= 1);

    // Empty input.
    assert_eq!(stat.compute_group(&DataFrame::new(), &scales).nrows(), 0);
}

#[test]
fn stat_boxplot_compute() {
    let stat = ggplot_rs::stat::boxplot::StatBoxplot;
    let scales = ScaleSet::new();
    let mut d = DataFrame::new();
    d.add_column("x".to_string(), strs(&["A", "A", "A", "A", "A"]));
    d.add_column("y".to_string(), floats(&[1.0, 2.0, 3.0, 4.0, 100.0]));
    let out = stat.compute_group(&d, &scales);
    assert_eq!(out.nrows(), 1);
    for c in ["ymin", "lower", "middle", "upper", "ymax"] {
        assert!(out.has_column(c), "missing {c}");
    }
}

#[test]
fn stat_smooth_lm_and_loess_compute() {
    let scales = ScaleSet::new();
    let xs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let ys = [1.1, 1.9, 3.2, 3.9, 5.1, 5.8, 7.2, 7.9];
    let input = df_xy(&xs, &ys);

    // Linear regression path (default).
    let lm = ggplot_rs::stat::smooth::StatSmooth::default();
    let out_lm = lm.compute_group(&input, &scales);
    assert!(out_lm.nrows() > 0);
    assert!(out_lm.has_column("ymin") && out_lm.has_column("ymax"));

    // LOESS path dispatched through StatSmooth.
    let loess = ggplot_rs::stat::smooth::StatSmooth {
        n_points: 20,
        se: true,
        method: ggplot_rs::stat::smooth::SmoothMethod::Loess { span: 0.75 },
    };
    let out_loess = loess.compute_group(&input, &scales);
    assert!(out_loess.nrows() > 0);

    // Too-few-points edge case returns empty.
    let tiny = df_xy(&[1.0], &[1.0]);
    assert_eq!(lm.compute_group(&tiny, &scales).nrows(), 0);
}

#[test]
fn stat_loess_direct_compute() {
    let stat = ggplot_rs::stat::loess::StatLoess {
        span: 0.8,
        n_points: 15,
        se: false,
    };
    let scales = ScaleSet::new();
    let input = df_xy(
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        &[2.0, 3.5, 3.0, 5.0, 6.5, 6.0],
    );
    let out = stat.compute_group(&input, &scales);
    assert!(out.nrows() > 0);
    assert!(out.has_column("x") && out.has_column("y"));
    assert_eq!(ggplot_rs::stat::loess::StatLoess::default().n_points, 80);
}

#[test]
fn stat_density_compute() {
    let stat = ggplot_rs::stat::density::StatDensity::default();
    let scales = ScaleSet::new();
    let out = stat.compute_group(&df_x(&[1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 5.0]), &scales);
    assert_eq!(out.nrows(), 512);
    // < 2 rows returns empty.
    assert_eq!(stat.compute_group(&df_x(&[1.0]), &scales).nrows(), 0);
}

#[test]
fn stat_ecdf_compute() {
    let stat = ggplot_rs::stat::ecdf::StatEcdf;
    let scales = ScaleSet::new();
    let out = stat.compute_group(&df_x(&[3.0, 1.0, 2.0, 4.0]), &scales);
    assert!(out.nrows() > 0);
    assert!(out.has_column("x") && out.has_column("y"));
    assert_eq!(
        ggplot_rs::stat::ecdf::StatEcdf
            .compute_group(&DataFrame::new(), &scales)
            .nrows(),
        0
    );
}

#[test]
fn stat_function_compute() {
    let stat = ggplot_rs::stat::function::StatFunction::new(|x| 2.0 * x + 1.0).with_n_points(11);
    let scales = ScaleSet::new();
    let out = stat.compute_group(&df_x(&[0.0, 10.0]), &scales);
    assert_eq!(out.nrows(), 11);
    assert!(out.has_column("x") && out.has_column("y"));

    // Constant range => empty.
    assert_eq!(stat.compute_group(&df_x(&[5.0, 5.0]), &scales).nrows(), 0);
    // No x column => empty.
    assert_eq!(stat.compute_group(&df_y(&[1.0, 2.0]), &scales).nrows(), 0);
}

#[test]
fn stat_summary_and_summaryfun() {
    use ggplot_rs::stat::summary::{StatSummary, SummaryFun};
    let scales = ScaleSet::new();

    // Every SummaryFun variant through apply().
    let v = [1.0, 2.0, 3.0, 4.0];
    assert_eq!(SummaryFun::Mean.apply(&v), 2.5);
    assert_eq!(SummaryFun::Median.apply(&v), 2.5);
    assert_eq!(SummaryFun::Median.apply(&[1.0, 2.0, 3.0]), 2.0);
    assert_eq!(SummaryFun::Min.apply(&v), 1.0);
    assert_eq!(SummaryFun::Max.apply(&v), 4.0);
    assert_eq!(SummaryFun::Sum.apply(&v), 10.0);
    assert_eq!(SummaryFun::Mean.apply(&[]), 0.0);

    let mut d = DataFrame::new();
    d.add_column("x".to_string(), floats(&[1.0, 1.0, 2.0, 2.0, 3.0]));
    d.add_column("y".to_string(), floats(&[10.0, 20.0, 5.0, 15.0, 8.0]));
    d.add_column("group".to_string(), strs(&["g", "g", "g", "g", "g"]));

    let out = StatSummary::default().compute_group(&d, &scales);
    assert_eq!(out.nrows(), 3);
    for c in ["y", "ymin", "ymax", "group"] {
        assert!(out.has_column(c));
    }
    // mean_se constructor variant.
    let out2 = StatSummary::mean_se().compute_group(&d, &scales);
    assert_eq!(out2.nrows(), 3);
}

#[test]
fn stat_summary_bin_compute() {
    use ggplot_rs::stat::summary::SummaryFun;
    use ggplot_rs::stat::summary_bin::StatSummaryBin;
    let scales = ScaleSet::new();

    let mut d = DataFrame::new();
    d.add_column(
        "x".to_string(),
        floats(&[0.0, 0.5, 1.0, 5.0, 5.5, 9.0, 10.0]),
    );
    d.add_column(
        "y".to_string(),
        floats(&[1.0, 3.0, 2.0, 8.0, 6.0, 10.0, 12.0]),
    );
    d.add_column("color".to_string(), strs(&["c"; 7]));

    let stat = StatSummaryBin::default()
        .with_bins(5)
        .with_fun(SummaryFun::Median)
        .with_fun_range(SummaryFun::Min, SummaryFun::Max);
    let out = stat.compute_group(&d, &scales);
    assert!(out.nrows() >= 1);
    assert!(out.has_column("ymin") && out.has_column("ymax") && out.has_column("color"));

    // Empty input branch.
    assert_eq!(
        StatSummaryBin::default()
            .compute_group(&DataFrame::new(), &scales)
            .nrows(),
        0
    );
}

#[test]
fn stat_sum_compute() {
    let stat = ggplot_rs::stat::sum::StatSum;
    let scales = ScaleSet::new();
    let mut d = DataFrame::new();
    d.add_column("x".to_string(), floats(&[1.0, 1.0, 2.0, 2.0, 2.0]));
    d.add_column("y".to_string(), floats(&[1.0, 1.0, 2.0, 2.0, 3.0]));
    d.add_column("group".to_string(), strs(&["g"; 5]));
    let out = stat.compute_group(&d, &scales);
    // Unique (x,y) pairs: (1,1), (2,2), (2,3) => 3
    assert_eq!(out.nrows(), 3);
    let n_total: f64 = out
        .column("n")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .sum();
    assert_eq!(n_total, 5.0);
    assert!(out.has_column("group"));
}

#[test]
fn stat_ydensity_compute() {
    let stat = ggplot_rs::stat::ydensity::StatYDensity::default();
    let scales = ScaleSet::new();
    let mut d = DataFrame::new();
    d.add_column("x".to_string(), strs(&["A"; 8]));
    d.add_column(
        "y".to_string(),
        floats(&[1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 5.0]),
    );
    let out = stat.compute_group(&d, &scales);
    assert_eq!(out.nrows(), 512); // ggplot2's default density resolution
    assert!(out.has_column("violinwidth"));
    // < 2 rows returns empty.
    assert_eq!(stat.compute_group(&df_y(&[1.0]), &scales).nrows(), 0);
}

#[test]
fn stat_bindot_compute() {
    let stat = ggplot_rs::stat::bindot::StatBindot { bins: 5 };
    let scales = ScaleSet::new();
    let mut d = DataFrame::new();
    d.add_column("x".to_string(), floats(&[1.0, 1.1, 1.2, 2.0, 2.1, 3.0]));
    d.add_column("fill".to_string(), strs(&["f"; 6]));
    let out = stat.compute_group(&d, &scales);
    assert_eq!(out.nrows(), 6);
    assert!(out.has_column("fill"));
    // Constant value edge + default.
    let out_c =
        ggplot_rs::stat::bindot::StatBindot::default().compute_group(&df_x(&[7.0, 7.0]), &scales);
    assert!(out_c.nrows() >= 1);
}

#[test]
fn stat_qq_and_qqline_compute() {
    let scales = ScaleSet::new();
    let sample: Vec<f64> = (0..30).map(|i| i as f64 * 0.5).collect();

    let qq = ggplot_rs::stat::qq::StatQQ;
    let out = qq.compute_group(&df_y(&sample), &scales);
    assert_eq!(out.nrows(), 30);

    let line = ggplot_rs::stat::qq::StatQQLine;
    let out_line = line.compute_group(&df_y(&sample), &scales);
    assert_eq!(out_line.nrows(), 2);

    // n < 4 => QQLine empty.
    assert_eq!(line.compute_group(&df_y(&[1.0, 2.0]), &scales).nrows(), 0);
    // Small-sample ppoints branch (n <= 10) for QQ.
    assert_eq!(
        qq.compute_group(&df_y(&[1.0, 2.0, 3.0]), &scales).nrows(),
        3
    );
}

#[test]
fn stat_bin2d_and_binhex_compute() {
    let scales = ScaleSet::new();
    let grid = df_grid();

    let b2 = ggplot_rs::stat::bin2d::StatBin2d {
        bins_x: 4,
        bins_y: 4,
    };
    let out2 = b2.compute_group(&grid, &scales);
    let total2: f64 = out2
        .column("fill")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .sum();
    assert_eq!(total2 as u64, 36);

    let bh = ggplot_rs::stat::binhex::StatBinHex {
        bins_x: 4,
        bins_y: 4,
    };
    let outh = bh.compute_group(&grid, &scales);
    let totalh: f64 = outh
        .column("fill")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .sum();
    assert_eq!(totalh as u64, 36);
}

#[test]
fn stat_contour_compute() {
    let stat = ggplot_rs::stat::contour::StatContour::default();
    let scales = ScaleSet::new();
    let out = stat.compute_group(&df_grid(), &scales);
    assert!(out.has_column("level") && out.has_column("group"));

    // Missing z column => empty.
    assert_eq!(
        stat.compute_group(&df_xy(&[1.0, 2.0], &[1.0, 2.0]), &scales)
            .nrows(),
        0
    );
    // Custom bins/levels.
    let stat2 = ggplot_rs::stat::contour::StatContour {
        bins: 10,
        n_levels: 4,
    };
    let _ = stat2.compute_group(&df_grid(), &scales);
}

#[test]
fn stat_density2d_compute() {
    let stat = ggplot_rs::stat::density2d::StatDensity2d::new()
        .with_grid(30)
        .with_levels(6);
    let scales = ScaleSet::new();

    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for i in 0..40 {
        let t = i as f64;
        xs.push((t * 0.3).sin() * 2.0 + t * 0.05);
        ys.push((t * 0.2).cos() * 2.0 - t * 0.03);
    }
    let out = stat.compute_group(&df_xy(&xs, &ys), &scales);
    assert!(out.has_column("level"));

    // < 2 points => empty.
    assert_eq!(
        stat.compute_group(&df_xy(&[1.0], &[1.0]), &scales).nrows(),
        0
    );
    // Degenerate (zero variance) => empty.
    assert_eq!(
        StatDensity2d::default()
            .compute_group(&df_xy(&[1.0, 1.0, 1.0], &[2.0, 2.0, 2.0]), &scales)
            .nrows(),
        0
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Stats: render through the full pipeline
// ═══════════════════════════════════════════════════════════════════════════

fn assert_svg(svg: Result<String, GGError>) {
    let s = svg.expect("render failed");
    assert!(s.contains("<svg"), "output is not SVG");
}

#[test]
fn render_stats_x_based() {
    // StatCount (bar), StatBin (histogram), StatDensity, StatEcdf, StatBindot.
    let cat = {
        let mut d = DataFrame::new();
        d.add_column("x".to_string(), strs(&["a", "b", "a", "c", "b", "a"]));
        d
    };
    assert_svg(
        GGPlot::new(cat)
            .aes(Aes::new().x("x"))
            .geom_bar()
            .render_svg(),
    );

    let cont = df_x(&[1.0, 2.0, 2.5, 3.0, 3.5, 4.0, 4.2, 5.0, 6.0, 7.0]);
    assert_svg(
        GGPlot::new(cont.clone())
            .aes(Aes::new().x("x"))
            .geom_histogram()
            .render_svg(),
    );
    assert_svg(
        GGPlot::new(cont.clone())
            .aes(Aes::new().x("x"))
            .geom_density()
            .render_svg(),
    );
    assert_svg(
        GGPlot::new(cont.clone())
            .aes(Aes::new().x("x"))
            .geom_line()
            .stat(ggplot_rs::stat::ecdf::StatEcdf)
            .render_svg(),
    );
    assert_svg(
        GGPlot::new(cont)
            .aes(Aes::new().x("x"))
            .geom_dotplot()
            .render_svg(),
    );
}

#[test]
fn render_stat_function() {
    // Exercises stat/function.rs through the pipeline (0% otherwise).
    let data = df_x(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
    assert_svg(
        GGPlot::new(data)
            .aes(Aes::new().x("x"))
            .geom_line()
            .stat(ggplot_rs::stat::function::StatFunction::new(|x| x * x).with_n_points(50))
            .render_svg(),
    );
}

#[test]
fn render_stats_xy_based() {
    let xs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let ys = [1.1, 2.3, 2.9, 4.2, 4.8, 6.1, 6.9, 8.2];

    // StatSmooth (lm) via geom_smooth.
    assert_svg(
        GGPlot::new(df_xy(&xs, &ys))
            .aes(Aes::new().x("x").y("y"))
            .geom_smooth()
            .render_svg(),
    );
    // StatSmooth (loess) via stat override.
    assert_svg(
        GGPlot::new(df_xy(&xs, &ys))
            .aes(Aes::new().x("x").y("y"))
            .geom_smooth()
            .stat(ggplot_rs::stat::smooth::StatSmooth {
                n_points: 30,
                se: false,
                method: ggplot_rs::stat::smooth::SmoothMethod::Loess { span: 0.75 },
            })
            .render_svg(),
    );
    // StatSummary via stat override on points.
    assert_svg(
        GGPlot::new(df_xy(
            &[1.0, 1.0, 2.0, 2.0, 3.0],
            &[1.0, 3.0, 2.0, 4.0, 5.0],
        ))
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .stat(ggplot_rs::stat::summary::StatSummary::default())
        .render_svg(),
    );
    // StatSummaryBin via stat override.
    assert_svg(
        GGPlot::new(df_xy(&xs, &ys))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .stat(ggplot_rs::stat::summary_bin::StatSummaryBin::default().with_bins(4))
            .render_svg(),
    );
    // StatSum via geom_count.
    assert_svg(
        GGPlot::new(df_xy(&[1.0, 1.0, 2.0, 2.0], &[1.0, 1.0, 2.0, 3.0]))
            .aes(Aes::new().x("x").y("y"))
            .geom_count()
            .render_svg(),
    );
}

#[test]
fn render_stats_distribution() {
    // StatBoxplot + StatYDensity (violin) with discrete x auto-grouping.
    let mut d = DataFrame::new();
    d.add_column(
        "x".to_string(),
        strs(&["A", "A", "A", "A", "B", "B", "B", "B"]),
    );
    d.add_column(
        "y".to_string(),
        floats(&[1.0, 2.0, 3.0, 4.0, 2.0, 3.0, 4.0, 9.0]),
    );
    assert_svg(
        GGPlot::new(d.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_boxplot()
            .render_svg(),
    );
    assert_svg(
        GGPlot::new(d)
            .aes(Aes::new().x("x").y("y"))
            .geom_violin()
            .render_svg(),
    );

    // StatQQ + StatQQLine.
    let sample = df_y(&(0..25).map(|i| i as f64 * 0.3).collect::<Vec<_>>());
    assert_svg(
        GGPlot::new(sample.clone())
            .aes(Aes::new().y("y"))
            .geom_qq()
            .render_svg(),
    );
    assert_svg(
        GGPlot::new(sample)
            .aes(Aes::new().y("y"))
            .geom_qq_line()
            .render_svg(),
    );
}

#[test]
fn render_stats_2d() {
    let grid = df_grid();
    // StatBin2d, StatBinHex, StatContour, StatDensity2d.
    assert_svg(
        GGPlot::new(grid.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_bin2d()
            .render_svg(),
    );
    assert_svg(
        GGPlot::new(grid.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_hex()
            .render_svg(),
    );
    assert_svg(
        GGPlot::new(grid.clone())
            .aes(Aes::new().x("x").y("z"))
            .geom_contour()
            .render_svg(),
    );

    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for i in 0..40 {
        let t = i as f64;
        xs.push((t * 0.3).sin() * 2.0 + t * 0.05);
        ys.push((t * 0.2).cos() * 2.0 - t * 0.03);
    }
    assert_svg(
        GGPlot::new(df_xy(&xs, &ys))
            .aes(Aes::new().x("x").y("y"))
            .geom_density2d()
            .render_svg(),
    );
}

#[test]
fn render_stat_identity() {
    // Default point stat is identity.
    assert_svg(
        GGPlot::new(df_xy(&[1.0, 2.0, 3.0], &[3.0, 1.0, 2.0]))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .render_svg(),
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Data: GGData input adapters
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ggdata_row_oriented_all_value_variants() {
    let rows = vec![
        HashMap::from([
            ("f".to_string(), Value::Float(1.5)),
            ("i".to_string(), Value::Integer(7)),
            ("s".to_string(), Value::Str("a".to_string())),
            ("b".to_string(), Value::Bool(true)),
            ("dt".to_string(), Value::DateTime(1_000_000)),
        ]),
        HashMap::from([
            ("f".to_string(), Value::Na),
            ("i".to_string(), Value::Integer(8)),
            ("s".to_string(), Value::Str("b".to_string())),
            ("b".to_string(), Value::Bool(false)),
            // "dt" intentionally missing -> filled with Na
        ]),
    ];
    let df = rows.into_dataframe();
    assert_eq!(df.nrows(), 2);
    assert_eq!(df.column("f").unwrap()[1], Value::Na);
    assert_eq!(df.column("dt").unwrap()[1], Value::Na);
    assert_eq!(df.column("b").unwrap()[0], Value::Bool(true));

    // Empty row-oriented input.
    let empty: Vec<HashMap<String, Value>> = Vec::new();
    assert_eq!(empty.into_dataframe().nrows(), 0);

    // Feed straight into a plot.
    assert_svg(
        GGPlot::new(vec![
            HashMap::from([
                ("x".to_string(), Value::Float(1.0)),
                ("y".to_string(), Value::Float(2.0)),
            ]),
            HashMap::from([
                ("x".to_string(), Value::Float(2.0)),
                ("y".to_string(), Value::Float(4.0)),
            ]),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg(),
    );
}

#[test]
fn ggdata_column_oriented_and_identity() {
    let cols: Vec<(String, Vec<Value>)> = vec![
        ("x".to_string(), floats(&[1.0, 2.0, 3.0])),
        ("g".to_string(), strs(&["a", "b", "c"])),
        (
            "flag".to_string(),
            vec![Value::Bool(true), Value::Bool(false), Value::Na],
        ),
    ];
    let df = cols.into_dataframe();
    assert_eq!(df.ncols(), 3);
    assert_eq!(df.nrows(), 3);

    // Identity passthrough.
    let same = df.clone().into_dataframe();
    assert_eq!(same.nrows(), df.nrows());
    assert_eq!(same.column_names(), df.column_names());

    assert_svg(
        GGPlot::new(df)
            .aes(Aes::new().x("x").y("x"))
            .geom_point()
            .render_svg(),
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Data: DataFrame public methods
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn dataframe_methods() {
    let mut df = DataFrame::new();
    df.add_column("x".to_string(), floats(&[3.0, 1.0, 2.0]));
    df.add_column("g".to_string(), strs(&["b", "a", "a"]));

    assert_eq!(df.nrows(), 3);
    assert_eq!(df.ncols(), 2);
    assert!(df.has_column("x"));
    assert!(!df.has_column("nope"));
    assert_eq!(df.column_names(), vec!["x", "g"]);
    assert!(df.column("x").is_some());
    assert!(df.column("missing").is_none());

    // sort_by (numeric) + sort_by on a missing column (clone fallback).
    let sorted = df.sort_by("x");
    let xs: Vec<f64> = sorted
        .column("x")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    assert_eq!(xs, vec![1.0, 2.0, 3.0]);
    assert_eq!(df.sort_by("does_not_exist").nrows(), 3);

    // group_by.
    let groups = df.group_by(&["g"]);
    assert_eq!(groups.len(), 2);
    // group_by on empty frame.
    assert!(DataFrame::new().group_by(&["g"]).is_empty());

    // vstack: same-schema, plus onto empty, plus other empty.
    let mut a = DataFrame::new();
    a.add_column("x".to_string(), floats(&[1.0]));
    let mut b = DataFrame::new();
    b.add_column("x".to_string(), floats(&[2.0, 3.0]));
    a.vstack(&b);
    assert_eq!(a.nrows(), 3);

    let mut empty = DataFrame::new();
    empty.vstack(&b);
    assert_eq!(empty.nrows(), 2);
    a.vstack(&DataFrame::new()); // no-op
    assert_eq!(a.nrows(), 3);

    // vstack with mismatched columns -> missing filled with Na.
    let mut c = DataFrame::new();
    c.add_column("x".to_string(), floats(&[10.0]));
    let mut d = DataFrame::new();
    d.add_column("x".to_string(), floats(&[11.0]));
    d.add_column("extra".to_string(), floats(&[99.0]));
    c.vstack(&d);
    assert_eq!(c.nrows(), 2);
    assert!(c.has_column("extra"));
    assert_eq!(c.column("extra").unwrap()[0], Value::Na);
}

#[test]
fn dataframe_from_csv_roundtrip_and_error() {
    // Success path: write a temp CSV and load it.
    let mut path = std::env::temp_dir();
    path.push(format!("cov_stats_data_{}.csv", std::process::id()));
    std::fs::write(&path, "a,b,c\n1,foo,3.5\n2,bar,NA\n4,baz,6\n").unwrap();

    let df = DataFrame::from_csv(path.to_str().unwrap()).unwrap();
    assert_eq!(df.nrows(), 3);
    assert_eq!(df.ncols(), 3);
    assert_eq!(df.column("a").unwrap()[0], Value::Float(1.0));
    assert_eq!(df.column("b").unwrap()[0], Value::Str("foo".to_string()));
    assert_eq!(df.column("c").unwrap()[1], Value::Na);
    std::fs::remove_file(&path).ok();

    // Error path: nonexistent file.
    assert!(DataFrame::from_csv("/definitely/not/here/cov_stats_data_missing.csv").is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// Data: polars adapter (default feature)
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(feature = "polars")]
#[test]
fn ggdata_polars_anyvalue_variants() {
    use polars::prelude::*;

    // Covers Float64, Int64, Int32, Boolean, and String AnyValue arms.
    let pdf = df![
        "f64" => [1.0f64, 2.0, 3.0],
        "i64" => [10i64, 20, 30],
        "i32" => [1i32, 2, 3],
        "b"   => [true, false, true],
        "s"   => ["x", "y", "z"],
    ]
    .unwrap();

    let df = pdf.into_dataframe();
    assert_eq!(df.nrows(), 3);
    assert_eq!(df.ncols(), 5);
    assert_eq!(df.column("f64").unwrap()[0], Value::Float(1.0));
    assert_eq!(df.column("i64").unwrap()[1], Value::Integer(20));
    assert_eq!(df.column("i32").unwrap()[2], Value::Integer(3));
    assert_eq!(df.column("b").unwrap()[0], Value::Bool(true));
    assert_eq!(df.column("s").unwrap()[2], Value::Str("z".to_string()));

    // A null in a numeric column becomes Value::Na.
    let with_null = df![
        "v" => [Some(1.0f64), None, Some(3.0)],
    ]
    .unwrap();
    let dn = with_null.into_dataframe();
    assert_eq!(dn.column("v").unwrap()[1], Value::Na);

    // Render straight from a polars frame.
    assert_svg(
        GGPlot::new(
            df![
                "x" => [1.0f64, 2.0, 3.0],
                "y" => [3.0f64, 1.0, 2.0],
            ]
            .unwrap(),
        )
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg(),
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Data: arrow adapter (behind the `arrow` feature)
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(feature = "arrow")]
#[test]
fn ggdata_arrow_all_array_types() {
    use arrow::array::{
        BooleanArray, Date32Array, Date64Array, Float32Array, Float64Array, Int16Array, Int32Array,
        Int64Array, Int8Array, LargeStringArray, NullArray, StringArray, Time32SecondArray,
        TimestampMicrosecondArray, TimestampMillisecondArray, TimestampNanosecondArray,
        TimestampSecondArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
    };
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    let batch = RecordBatch::try_from_iter(vec![
        (
            "f64",
            Arc::new(Float64Array::from(vec![Some(1.0), None, Some(3.0)])) as _,
        ),
        (
            "f32",
            Arc::new(Float32Array::from(vec![1.5f32, 2.5, 3.5])) as _,
        ),
        ("i64", Arc::new(Int64Array::from(vec![1i64, 2, 3])) as _),
        ("i32", Arc::new(Int32Array::from(vec![4i32, 5, 6])) as _),
        ("i16", Arc::new(Int16Array::from(vec![7i16, 8, 9])) as _),
        ("i8", Arc::new(Int8Array::from(vec![1i8, 2, 3])) as _),
        ("u64", Arc::new(UInt64Array::from(vec![10u64, 20, 30])) as _),
        ("u32", Arc::new(UInt32Array::from(vec![40u32, 50, 60])) as _),
        ("u16", Arc::new(UInt16Array::from(vec![1u16, 2, 3])) as _),
        ("u8", Arc::new(UInt8Array::from(vec![4u8, 5, 6])) as _),
        (
            "bool",
            Arc::new(BooleanArray::from(vec![true, false, true])) as _,
        ),
        (
            "utf8",
            Arc::new(StringArray::from(vec!["a", "b", "c"])) as _,
        ),
        (
            "lutf8",
            Arc::new(LargeStringArray::from(vec!["d", "e", "f"])) as _,
        ),
        ("date32", Arc::new(Date32Array::from(vec![0i32, 1, 2])) as _),
        (
            "date64",
            Arc::new(Date64Array::from(vec![0i64, 86_400_000, 172_800_000])) as _,
        ),
        (
            "ts_s",
            Arc::new(TimestampSecondArray::from(vec![1i64, 2, 3])) as _,
        ),
        (
            "ts_ms",
            Arc::new(TimestampMillisecondArray::from(vec![1000i64, 2000, 3000])) as _,
        ),
        (
            "ts_us",
            Arc::new(TimestampMicrosecondArray::from(vec![
                1_000_000i64,
                2_000_000,
                3_000_000,
            ])) as _,
        ),
        (
            "ts_ns",
            Arc::new(TimestampNanosecondArray::from(vec![
                1_000_000_000i64,
                2_000_000_000,
                3_000_000_000,
            ])) as _,
        ),
        // Unsupported type -> fallback branch (null row -> Na, others -> display string).
        (
            "time",
            Arc::new(Time32SecondArray::from(vec![Some(1i32), None, Some(3)])) as _,
        ),
        // Null array -> `_` fallback with all-null -> Value::Na.
        ("nul", Arc::new(NullArray::new(3)) as _),
    ])
    .unwrap();

    let df = batch.into_dataframe();
    assert_eq!(df.nrows(), 3);
    assert_eq!(df.ncols(), 21);

    // Numeric coercions.
    assert_eq!(df.column("f64").unwrap()[0], Value::Float(1.0));
    assert_eq!(df.column("f64").unwrap()[1], Value::Na);
    assert_eq!(df.column("f32").unwrap()[1], Value::Float(2.5));
    assert_eq!(df.column("i64").unwrap()[0], Value::Integer(1));
    assert_eq!(df.column("i32").unwrap()[0], Value::Integer(4));
    assert_eq!(df.column("i16").unwrap()[0], Value::Integer(7));
    assert_eq!(df.column("i8").unwrap()[0], Value::Integer(1));
    assert_eq!(df.column("u64").unwrap()[0], Value::Integer(10));
    assert_eq!(df.column("u32").unwrap()[0], Value::Integer(40));
    assert_eq!(df.column("u16").unwrap()[0], Value::Integer(1));
    assert_eq!(df.column("u8").unwrap()[0], Value::Integer(4));

    // Bool / strings.
    assert_eq!(df.column("bool").unwrap()[0], Value::Bool(true));
    assert_eq!(df.column("utf8").unwrap()[2], Value::Str("c".to_string()));
    assert_eq!(df.column("lutf8").unwrap()[0], Value::Str("d".to_string()));

    // Dates / timestamps -> DateTime seconds.
    assert_eq!(df.column("date32").unwrap()[1], Value::DateTime(86_400));
    assert_eq!(df.column("date64").unwrap()[1], Value::DateTime(86_400));
    assert_eq!(df.column("ts_s").unwrap()[0], Value::DateTime(1));
    assert_eq!(df.column("ts_ms").unwrap()[0], Value::DateTime(1));
    assert_eq!(df.column("ts_us").unwrap()[0], Value::DateTime(1));
    assert_eq!(df.column("ts_ns").unwrap()[0], Value::DateTime(1));

    // Unsupported type: null row -> Na, others stringified via display fallback.
    assert!(matches!(df.column("time").unwrap()[0], Value::Str(_)));
    assert_eq!(df.column("time").unwrap()[1], Value::Na);
    assert!(matches!(df.column("nul").unwrap()[0], Value::Str(_)));

    // The example in source.rs uses try_from_iter with a null in a float col;
    // confirm a render works off an arrow-derived frame too.
    let batch2 = RecordBatch::try_from_iter(vec![
        ("x", Arc::new(Float64Array::from(vec![1.0, 2.0, 3.0])) as _),
        ("y", Arc::new(Float64Array::from(vec![3.0, 1.0, 2.0])) as _),
    ])
    .unwrap();
    assert_svg(
        GGPlot::new(batch2.into_dataframe())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .render_svg(),
    );
}
