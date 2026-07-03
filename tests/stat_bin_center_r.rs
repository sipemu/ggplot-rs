//! Regression: stat_bin / stat_bin2d match ggplot2 4.0.3 — width = range/(bins-1)
//! and bins aligned to 0, not started at the data minimum.
use ggplot_rs::data::DataFrame;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::scale_set::ScaleSet;
use ggplot_rs::stat::Stat;

#[test]
fn bins_centered_like_ggplot2() {
    let x: Vec<Value> = (0..100).map(|i| Value::Float(i as f64 * 0.1)).collect();
    let built = GGPlot::new(vec![("x".to_string(), x)])
        .aes(Aes::new().x("x"))
        .geom_histogram()
        .build();
    let d = &built.layers[0].data;
    assert_eq!(d.nrows(), 30);
    // R: geom_histogram(bins=30) on seq(0,9.9,.1) -> first bin [-0.17069, 0.17069], count 2.
    let xmin0 = d.column("xmin").unwrap()[0].as_f64().unwrap();
    let count0 = d.column("y").unwrap()[0].as_f64().unwrap();
    assert!(
        (xmin0 - (-0.17069)).abs() < 1e-3,
        "first bin xmin {xmin0}, want ~-0.17069"
    );
    assert_eq!(count0, 2.0, "first bin count");
}

#[test]
fn bin2d_edges_aligned_like_ggplot2() {
    // R: geom_bin2d(bins=5) on sin/cos data -> x bin lefts -1.5,-1.0,-0.5,0,0.5
    // (edge on 0, width = range/(bins-1)); 15 non-empty bins, total 100.
    let mut df = DataFrame::new();
    df.add_column(
        "x".into(),
        (0..100)
            .map(|i| Value::Float((i as f64 * 0.1).sin()))
            .collect(),
    );
    df.add_column(
        "y".into(),
        (0..100)
            .map(|i| Value::Float((i as f64 * 0.1).cos()))
            .collect(),
    );
    let d = ggplot_rs::stat::bin2d::StatBin2d {
        bins_x: 5,
        bins_y: 5,
    }
    .compute_group(&df, &ScaleSet::new());

    let total: f64 = d
        .column("fill")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .sum();
    assert_eq!(total as usize, 100, "every point binned");
    assert_eq!(d.nrows(), 15, "non-empty bin count matches R");
    let min_left = d
        .column("xmin")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .fold(f64::INFINITY, f64::min);
    assert!(
        (min_left - (-1.49962)).abs() < 1e-3,
        "first x edge {min_left}, want ~-1.49962"
    );
}
