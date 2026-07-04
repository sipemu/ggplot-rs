//! Validation tests: compare ggplot-rs computed data against R's ggplot_build() output.
//!
//! Each test loads shared input data, builds the same plot specification as the
//! corresponding R code in validation/generate_all.R, and compares the computed
//! DataFrame column-by-column against the expected CSV fixture.
//!
//! Run with: `cargo test --test validation`

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::position::Position;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::ScaleSet;
use ggplot_rs::stat::Stat;

// ═══════════════════════════════════════════════════════════════════════════════
// Helper: approximate DataFrame comparison
// ═══════════════════════════════════════════════════════════════════════════════

/// Compare selected columns of two DataFrames with absolute tolerance.
///
/// For Float columns: passes if |actual - expected| <= tol.
/// For Str columns: passes if strings match exactly.
/// Panics with a descriptive message on mismatch.
fn assert_df_approx_eq(actual: &DataFrame, expected: &DataFrame, columns: &[&str], tol: f64) {
    for &col_name in columns {
        let actual_col = actual
            .column(col_name)
            .unwrap_or_else(|| panic!("actual DataFrame missing column '{col_name}'"));
        let expected_col = expected
            .column(col_name)
            .unwrap_or_else(|| panic!("expected DataFrame missing column '{col_name}'"));

        assert_eq!(
            actual_col.len(),
            expected_col.len(),
            "column '{col_name}': row count mismatch: actual={}, expected={}",
            actual_col.len(),
            expected_col.len()
        );

        for (i, (a, e)) in actual_col.iter().zip(expected_col.iter()).enumerate() {
            match (a, e) {
                (Value::Float(av), Value::Float(ev)) => {
                    let diff = (av - ev).abs();
                    // Use relative tolerance for large values, absolute for small
                    let rel_ok = if ev.abs() > 1.0 {
                        diff / ev.abs() <= tol
                    } else {
                        false
                    };
                    assert!(
                        diff <= tol || rel_ok,
                        "column '{col_name}' row {i}: actual={av}, expected={ev}, diff={diff}, tol={tol}"
                    );
                }
                (Value::Str(av), Value::Str(ev)) => {
                    assert_eq!(
                        av, ev,
                        "column '{col_name}' row {i}: actual='{av}', expected='{ev}'"
                    );
                }
                (Value::Integer(av), Value::Float(ev)) => {
                    let diff = (*av as f64 - ev).abs();
                    assert!(
                        diff <= tol,
                        "column '{col_name}' row {i}: actual={av}, expected={ev}, diff={diff}"
                    );
                }
                (Value::Float(av), Value::Integer(ev)) => {
                    let diff = (av - *ev as f64).abs();
                    assert!(
                        diff <= tol,
                        "column '{col_name}' row {i}: actual={av}, expected={ev}, diff={diff}"
                    );
                }
                (Value::Na, Value::Na) => {}
                _ => panic!(
                    "column '{col_name}' row {i}: type mismatch: actual={a:?}, expected={e:?}"
                ),
            }
        }
    }
}

/// Compare just the row count of two DataFrames.
fn assert_row_count_eq(actual: &DataFrame, expected: &DataFrame, context: &str) {
    assert_eq!(
        actual.nrows(),
        expected.nrows(),
        "{context}: row count mismatch: actual={}, expected={}",
        actual.nrows(),
        expected.nrows()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tier 1: Core stats
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_stat_ecdf_vs_r() {
    // StatEcdf is not a default stat for any geom, so test directly.
    let input = DataFrame::from_csv("validation/fixtures/data/uniform_100.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_ecdf.csv").unwrap();

    let stat = ggplot_rs::stat::ecdf::StatEcdf;
    let scales = ScaleSet::new();
    let result = stat.compute_group(&input, &scales);

    assert_row_count_eq(&result, &expected, "stat_ecdf");
    assert_df_approx_eq(&result, &expected, &["x", "y"], 1e-10);
}

#[test]
fn test_stat_count_vs_r() {
    // Test StatCount directly to avoid pipeline grouping by 'fill' column.
    let all_data = DataFrame::from_csv("validation/fixtures/data/grouped_bars.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_count.csv").unwrap();

    // Extract just the x column (no fill) for ungrouped counting.
    let x_col = all_data.column("x").unwrap();
    let mut input = DataFrame::new();
    input.add_column("x".to_string(), x_col.to_vec());

    let stat = ggplot_rs::stat::count::StatCount;
    let scales = ScaleSet::new();
    let result = stat.compute_group(&input, &scales);

    assert_eq!(result.nrows(), expected.nrows(), "stat_count: row count");

    // Verify counts: extract y values and compare (order-independent)
    let mut actual_y: Vec<f64> = result
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let mut expected_y: Vec<f64> = expected
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();

    actual_y.sort_by(|a, b| a.partial_cmp(b).unwrap());
    expected_y.sort_by(|a, b| a.partial_cmp(b).unwrap());

    assert_eq!(actual_y, expected_y, "stat_count: counts mismatch");
}

#[test]
fn test_stat_bin_vs_r() {
    // geom_histogram() uses StatBin with bins=30 by default.
    let input = DataFrame::from_csv("validation/fixtures/data/uniform_100.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_bin.csv").unwrap();

    let plot = GGPlot::new(input).aes(Aes::new().x("x")).geom_histogram();
    let built = plot.build();

    let actual = &built.layers[0].data;

    // Both should have 30 bins
    assert_row_count_eq(actual, &expected, "stat_bin");

    // Compare bin centers and counts
    assert_df_approx_eq(actual, &expected, &["x", "y", "xmin", "xmax"], 1e-6);
}

#[test]
fn test_stat_boxplot_vs_r() {
    // Test StatBoxplot directly per group (pipeline doesn't auto-group by discrete x).
    let all_data = DataFrame::from_csv("validation/fixtures/data/boxplot_groups.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_boxplot.csv").unwrap();

    let stat = ggplot_rs::stat::boxplot::StatBoxplot;
    let scales = ScaleSet::new();

    let x_col = all_data.column("x").unwrap();
    let y_col = all_data.column("y").unwrap();

    // Process each group separately (A, B, C)
    let mut result_combined = DataFrame::new();
    for group in &["A", "B", "C"] {
        let mut group_x = Vec::new();
        let mut group_y = Vec::new();
        for (x, y) in x_col.iter().zip(y_col.iter()) {
            if let Value::Str(s) = x {
                if s == *group {
                    group_x.push(Value::Str(group.to_string()));
                    group_y.push(y.clone());
                }
            }
        }

        let mut group_data = DataFrame::new();
        group_data.add_column("x".to_string(), group_x);
        group_data.add_column("y".to_string(), group_y);

        let group_result = stat.compute_group(&group_data, &scales);
        result_combined.vstack(&group_result);
    }

    // Should have 3 groups
    assert_row_count_eq(&result_combined, &expected, "stat_boxplot");

    // Compare 5-number summary (sort by middle to align groups)
    let actual_sorted = result_combined.sort_by("middle");
    let expected_sorted = expected.sort_by("middle");

    assert_df_approx_eq(
        &actual_sorted,
        &expected_sorted,
        &["ymin", "lower", "middle", "upper", "ymax"],
        1e-6,
    );
}

#[test]
fn test_stat_density_vs_r() {
    let input = DataFrame::from_csv("validation/fixtures/data/uniform_100.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_density.csv").unwrap();

    let plot = GGPlot::new(input).aes(Aes::new().x("x")).geom_density();
    let built = plot.build();

    let actual = &built.layers[0].data;

    // Both should have 512 evaluation points
    assert_row_count_eq(actual, &expected, "stat_density");

    // Density values should be close (tolerance accounts for bandwidth sensitivity)
    assert_df_approx_eq(actual, &expected, &["x", "y"], 0.01);
}

#[test]
fn test_stat_smooth_lm_vs_r() {
    let input = DataFrame::from_csv("validation/fixtures/data/smooth_input.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_smooth_lm.csv").unwrap();

    let plot = GGPlot::new(input)
        .aes(Aes::new().x("x").y("y"))
        .geom_smooth();
    let built = plot.build();

    let actual = &built.layers[0].data;

    // Both should have 80 fitted points
    assert_row_count_eq(actual, &expected, "stat_smooth_lm");

    // OLS fit should be near-exact; CI uses z≈1.96 (close to t for large n)
    assert_df_approx_eq(actual, &expected, &["x", "y"], 1e-4);

    // CI bounds (looser tolerance due to z vs t difference)
    assert_df_approx_eq(actual, &expected, &["ymin", "ymax"], 0.1);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tier 2: Remaining stats
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_stat_smooth_loess_vs_r() {
    // Test LOESS directly since we need to configure span.
    let input = DataFrame::from_csv("validation/fixtures/data/smooth_input.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_smooth_loess.csv").unwrap();

    let stat = ggplot_rs::stat::loess::StatLoess {
        span: 0.75,
        n_points: 80,
        se: true,
    };
    let scales = ScaleSet::new();
    let result = stat.compute_group(&input, &scales);

    assert_row_count_eq(&result, &expected, "stat_smooth_loess");

    // LOESS implementations can vary; use generous tolerance
    assert_df_approx_eq(&result, &expected, &["x", "y"], 0.05);
}

#[test]
fn test_stat_summary_vs_r() {
    // StatSummary is not a default stat for common geoms; test directly.
    let input = DataFrame::from_csv("validation/fixtures/data/summary_input.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_summary.csv").unwrap();

    let stat = ggplot_rs::stat::summary::StatSummary::default();
    let scales = ScaleSet::new();
    let result = stat.compute_group(&input, &scales);

    assert_row_count_eq(&result, &expected, "stat_summary");

    let result_sorted = result.sort_by("x");
    let expected_sorted = expected.sort_by("x");
    assert_df_approx_eq(
        &result_sorted,
        &expected_sorted,
        &["x", "y", "ymin", "ymax"],
        1e-10,
    );
}

#[test]
fn test_stat_qq_vs_r() {
    let input = DataFrame::from_csv("validation/fixtures/data/qq_input.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_qq.csv").unwrap();

    let plot = GGPlot::new(input).aes(Aes::new().y("y")).geom_qq();
    let built = plot.build();

    let actual = &built.layers[0].data;

    assert_row_count_eq(actual, &expected, "stat_qq");

    // qnorm approximation may differ slightly from R's qnorm
    assert_df_approx_eq(actual, &expected, &["x", "y"], 0.01);
}

#[test]
fn test_stat_qq_line_vs_r() {
    let input = DataFrame::from_csv("validation/fixtures/data/qq_input.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_qq_line.csv").unwrap();

    let plot = GGPlot::new(input).aes(Aes::new().y("y")).geom_qq_line();
    let built = plot.build();

    let actual = &built.layers[0].data;

    assert_row_count_eq(actual, &expected, "stat_qq_line");

    // Compounds qnorm and quantile differences; use moderate tolerance
    assert_df_approx_eq(actual, &expected, &["x", "y"], 0.05);
}

#[test]
fn test_stat_bin2d_vs_r() {
    // Test StatBin2d directly with bins=5 to match fixtures.
    let input = DataFrame::from_csv("validation/fixtures/data/bin2d_input.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_bin2d.csv").unwrap();

    let stat = ggplot_rs::stat::bin2d::StatBin2d {
        bins_x: 5,
        bins_y: 5,
    };
    let scales = ScaleSet::new();
    let result = stat.compute_group(&input, &scales);

    // Row count may differ slightly if empty bins are handled differently
    // Compare total count: sum of fill values should equal input row count
    let actual_total: f64 = result
        .column("fill")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .sum();
    let expected_total: f64 = expected
        .column("fill")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .sum();
    assert_eq!(
        actual_total as u64, expected_total as u64,
        "stat_bin2d: total count mismatch"
    );

    // Verify non-empty bin count
    assert_eq!(
        result.nrows(),
        expected.nrows(),
        "stat_bin2d: number of non-empty bins"
    );
}

#[test]
fn test_stat_binhex_vs_r() {
    // Test StatBinHex directly with bins=5 to match fixtures.
    let input = DataFrame::from_csv("validation/fixtures/data/bin2d_input.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_binhex.csv").unwrap();

    let stat = ggplot_rs::stat::binhex::StatBinHex {
        bins_x: 5,
        bins_y: 5,
    };
    let scales = ScaleSet::new();
    let result = stat.compute_group(&input, &scales);

    // Total count should match
    let actual_total: f64 = result
        .column("fill")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .sum();
    let expected_total: f64 = expected
        .column("fill")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .sum();
    assert_eq!(
        actual_total as u64, expected_total as u64,
        "stat_binhex: total count mismatch"
    );
}

#[test]
fn test_stat_ydensity_vs_r() {
    // Test StatYDensity directly on group A data.
    let all_data = DataFrame::from_csv("validation/fixtures/data/boxplot_groups.csv").unwrap();
    let expected = DataFrame::from_csv("validation/fixtures/stat_ydensity.csv").unwrap();

    // Filter to group A only and set up as x=constant, y=values
    let x_col = all_data.column("x").unwrap();
    let y_col = all_data.column("y").unwrap();

    let mut group_a_x = Vec::new();
    let mut group_a_y = Vec::new();
    for (x, y) in x_col.iter().zip(y_col.iter()) {
        if let Value::Str(s) = x {
            if s == "A" {
                group_a_x.push(Value::Float(0.0)); // x position
                group_a_y.push(y.clone());
            }
        }
    }

    let mut input = DataFrame::new();
    input.add_column("x".to_string(), group_a_x);
    input.add_column("y".to_string(), group_a_y);

    let stat = ggplot_rs::stat::ydensity::StatYDensity::default();
    let scales = ScaleSet::new();
    let result = stat.compute_group(&input, &scales);

    // ggplot2 defaults to trim = TRUE: the violin spans the data range exactly.
    // (R pads its density grid to a slightly different point count, so we compare
    // the normalized profile by interpolation rather than row-by-row.)
    let ry: Vec<f64> = result
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let rw: Vec<f64> = result
        .column("violinwidth")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let rmin = ry.iter().cloned().fold(f64::INFINITY, f64::min);
    let rmax = ry.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        (rmin - 10.0).abs() < 1e-6 && (rmax - 19.5).abs() < 1e-6,
        "violin should be trimmed to the data range [10, 19.5], got [{rmin}, {rmax}]"
    );

    // R's raw density, normalized to [0, 1] like ggplot-rs's violinwidth.
    let ey: Vec<f64> = expected
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let ed_raw: Vec<f64> = expected
        .column("density")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let emax = ed_raw.iter().cloned().fold(0.0, f64::max);
    let ed: Vec<f64> = ed_raw.iter().map(|d| d / emax).collect();

    // Linear interpolation of (xs, ys) at q (xs ascending).
    let interp = |xs: &[f64], ys: &[f64], q: f64| -> f64 {
        match xs.iter().position(|&x| x >= q) {
            Some(0) => ys[0],
            Some(i) => {
                let t = (q - xs[i - 1]) / (xs[i] - xs[i - 1]);
                ys[i - 1] + t * (ys[i] - ys[i - 1])
            }
            None => *ys.last().unwrap(),
        }
    };

    for q in [11.0, 13.0, 15.0, 17.0, 19.0] {
        let a = interp(&ry, &rw, q);
        let b = interp(&ey, &ed, q);
        assert!(
            (a - b).abs() < 0.05,
            "violin normalized density at y={q}: ggplot-rs {a:.4} vs R {b:.4}"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Position adjustments (tested directly)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_position_stack_vs_r() {
    let expected = DataFrame::from_csv("validation/fixtures/position_stack.csv").unwrap();

    // Create the same data that position_stack operates on (post-stat)
    let mut data = DataFrame::new();
    data.add_column(
        "x".to_string(),
        vec![
            Value::Float(1.0),
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(2.0),
            Value::Float(3.0),
            Value::Float(3.0),
        ],
    );
    data.add_column(
        "y".to_string(),
        vec![
            Value::Float(3.0),
            Value::Float(2.0),
            Value::Float(5.0),
            Value::Float(4.0),
            Value::Float(1.0),
            Value::Float(3.0),
        ],
    );

    let position = ggplot_rs::position::stack::PositionStack;
    let params = ggplot_rs::position::PositionParams::default();
    position.compute(&mut data, &params);

    assert_df_approx_eq(&data, &expected, &["x", "y", "ymin"], 1e-10);
}

#[test]
fn test_position_fill_vs_r() {
    let expected = DataFrame::from_csv("validation/fixtures/position_fill.csv").unwrap();

    let mut data = DataFrame::new();
    data.add_column(
        "x".to_string(),
        vec![
            Value::Float(1.0),
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(2.0),
            Value::Float(3.0),
            Value::Float(3.0),
        ],
    );
    data.add_column(
        "y".to_string(),
        vec![
            Value::Float(3.0),
            Value::Float(2.0),
            Value::Float(5.0),
            Value::Float(4.0),
            Value::Float(1.0),
            Value::Float(3.0),
        ],
    );

    let position = ggplot_rs::position::fill::PositionFill;
    let params = ggplot_rs::position::PositionParams::default();
    position.compute(&mut data, &params);

    assert_df_approx_eq(&data, &expected, &["x", "y", "ymin"], 1e-10);
}

#[test]
fn test_position_dodge_vs_r() {
    let expected = DataFrame::from_csv("validation/fixtures/position_dodge.csv").unwrap();

    let mut data = DataFrame::new();
    data.add_column(
        "x".to_string(),
        vec![
            Value::Float(1.0),
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(2.0),
            Value::Float(3.0),
            Value::Float(3.0),
        ],
    );
    data.add_column(
        "y".to_string(),
        vec![
            Value::Float(3.0),
            Value::Float(2.0),
            Value::Float(5.0),
            Value::Float(4.0),
            Value::Float(1.0),
            Value::Float(3.0),
        ],
    );
    data.add_column(
        "fill".to_string(),
        vec![
            Value::Str("g1".to_string()),
            Value::Str("g2".to_string()),
            Value::Str("g1".to_string()),
            Value::Str("g2".to_string()),
            Value::Str("g1".to_string()),
            Value::Str("g2".to_string()),
        ],
    );

    let position = ggplot_rs::position::dodge::PositionDodge;
    let params = ggplot_rs::position::PositionParams::default();
    position.compute(&mut data, &params);

    assert_df_approx_eq(&data, &expected, &["x"], 1e-6);
}
