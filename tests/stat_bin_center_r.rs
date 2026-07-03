//! Regression: stat_bin matches ggplot2 4.0.3 — width = range/(bins-1) and bins
//! centered on 0 (boundary = width/2), not started at the data minimum.
use ggplot_rs::prelude::*;

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
