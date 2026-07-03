//! Regression: QQ theoretical quantiles use R's ppoints `a` (3/8 for n<=10,
//! 1/2 for n>10). Found via validation against ggplot2 4.0.3.
use ggplot_rs::data::DataFrame;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::scale_set::ScaleSet;
use ggplot_rs::stat::Stat;

#[test]
fn qq_first_theoretical_matches_r_for_large_n() {
    let mut df = DataFrame::new();
    df.add_column(
        "y".into(),
        (0..100).map(|i| Value::Float(i as f64)).collect(),
    );
    let d = ggplot_rs::stat::qq::StatQQ.compute_group(&df, &ScaleSet::new());
    let min_theo = d
        .column("x")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .fold(f64::INFINITY, f64::min);
    // R: qnorm(ppoints(100))[1] = qnorm(0.005) = -2.5758 (a = 1/2 for n > 10).
    assert!(
        (min_theo - (-2.5758)).abs() < 2e-3,
        "got {min_theo}, expected ~-2.5758"
    );
}
