//! Property-based invariants (A-grade #61). These hold for *any* input, so they
//! catch classes of bugs point tests miss: scale monotonicity, expression
//! totality, bin count-preservation, and render totality.

use ggplot_rs::aes::expr::eval_expression;
use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::scale::continuous::ScaleContinuous;
use ggplot_rs::scale::scale_set::ScaleSet;
use ggplot_rs::scale::Scale;
use ggplot_rs::stat::bin::StatBin;
use ggplot_rs::stat::density::StatDensity;
use ggplot_rs::stat::ecdf::StatEcdf;
use ggplot_rs::stat::Stat;
use proptest::prelude::*;

/// A vector of finite, well-scaled f64s.
fn finite_vec(min: usize, max: usize) -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(-1.0e6f64..1.0e6, min..max)
}

proptest! {
    /// A trained continuous scale maps monotonically: a ≤ b ⇒ map(a) ≤ map(b).
    #[test]
    fn continuous_scale_is_monotone(mut vals in finite_vec(2, 40), a in -1.0e6f64..1.0e6, b in -1.0e6f64..1.0e6) {
        vals.push(a); vals.push(b);
        let mut s = ScaleContinuous::new();
        s.train(&vals.iter().map(|v| Value::Float(*v)).collect::<Vec<_>>());
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        prop_assert!(s.map(&Value::Float(lo)) <= s.map(&Value::Float(hi)) + 1e-9);
    }

    /// eval_expression is total (never panics) and shape-preserving: one value
    /// per row, and `a + b` equals the elementwise sum.
    #[test]
    fn expression_is_total_and_correct(a in finite_vec(1, 30), b in finite_vec(1, 30)) {
        let n = a.len().min(b.len());
        let mut df = DataFrame::new();
        df.add_column("a".into(), a[..n].iter().map(|v| Value::Float(*v)).collect());
        df.add_column("b".into(), b[..n].iter().map(|v| Value::Float(*v)).collect());
        let out = eval_expression("a + b", &df).unwrap();
        prop_assert_eq!(out.len(), n);
        for i in 0..n {
            prop_assert!((out[i].as_f64().unwrap() - (a[i] + b[i])).abs() < 1e-6);
        }
        // An unknown column yields None, not a panic.
        prop_assert!(eval_expression("nope_col + a", &df).is_some()); // references known `a`
        prop_assert!(eval_expression("nope_col", &df).is_none());
    }

    /// stat_bin preserves count: every input point lands in exactly one bin, so
    /// the bin counts sum to the number of finite inputs.
    #[test]
    fn stat_bin_preserves_count(vals in finite_vec(2, 200)) {
        // Need a non-degenerate range for binning.
        prop_assume!(vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                   > vals.iter().cloned().fold(f64::INFINITY, f64::min));
        let mut df = DataFrame::new();
        df.add_column("x".into(), vals.iter().map(|v| Value::Float(*v)).collect());
        let out = StatBin::default().compute_group(&df, &ScaleSet::new());
        let total: f64 = out.column("count").unwrap().iter().filter_map(|v| v.as_f64()).sum();
        prop_assert_eq!(total as usize, vals.len());
    }

    /// Rendering is total: any small finite xy dataset renders without panicking.
    #[test]
    fn render_never_panics(xs in finite_vec(1, 40), ys in finite_vec(1, 40)) {
        let n = xs.len().min(ys.len());
        let data = vec![
            ("x".to_string(), xs[..n].iter().map(|v| Value::Float(*v)).collect::<Vec<_>>()),
            ("y".to_string(), ys[..n].iter().map(|v| Value::Float(*v)).collect::<Vec<_>>()),
        ];
        use ggplot_rs::prelude::*;
        let r = GGPlot::new(data).aes(Aes::new().x("x").y("y")).geom_point().render_svg();
        prop_assert!(r.is_ok());
    }

    /// The ECDF is monotone non-decreasing and bounded in [0, 1].
    #[test]
    fn ecdf_is_monotone_in_unit_range(vals in finite_vec(2, 100)) {
        prop_assume!(vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                   > vals.iter().cloned().fold(f64::INFINITY, f64::min));
        let mut df = DataFrame::new();
        df.add_column("x".into(), vals.iter().map(|v| Value::Float(*v)).collect());
        let out = StatEcdf.compute_group(&df, &ScaleSet::new());
        let ys: Vec<f64> = out.column("y").unwrap().iter().filter_map(|v| v.as_f64()).collect();
        for w in ys.windows(2) {
            prop_assert!(w[1] >= w[0] - 1e-9, "ecdf must be non-decreasing");
        }
        for &y in &ys {
            prop_assert!((-1e-9..=1.0 + 1e-9).contains(&y), "ecdf out of [0,1]: {y}");
        }
    }

    /// A kernel density estimate is never negative.
    #[test]
    fn density_is_non_negative(vals in finite_vec(3, 100)) {
        prop_assume!(vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                   > vals.iter().cloned().fold(f64::INFINITY, f64::min));
        let mut df = DataFrame::new();
        df.add_column("x".into(), vals.iter().map(|v| Value::Float(*v)).collect());
        let out = StatDensity::default().compute_group(&df, &ScaleSet::new());
        for v in out.column("y").unwrap() {
            if let Some(y) = v.as_f64() {
                prop_assert!(y >= -1e-9, "density must be non-negative: {y}");
            }
        }
    }
}
