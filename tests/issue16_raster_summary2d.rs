//! Issue #16 — geom_raster and stat_summary_2d render end-to-end.

use ggplot_rs::prelude::*;

fn grid() -> Vec<(String, Vec<Value>)> {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut z = Vec::new();
    for i in 0..8 {
        for j in 0..8 {
            x.push(Value::Float(i as f64));
            y.push(Value::Float(j as f64));
            z.push(Value::Float((i * j) as f64));
        }
    }
    vec![
        ("x".to_string(), x),
        ("y".to_string(), y),
        ("z".to_string(), z),
    ]
}

#[test]
fn geom_raster_renders_grid() {
    let svg = GGPlot::new(grid())
        .aes(Aes::new().x("x").y("y").fill("z"))
        .geom_raster()
        .scale_fill_viridis_c()
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn stat_summary_2d_mean_of_z_renders() {
    // Scatter with a z value; bin into a grid coloured by mean(z).
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut z = Vec::new();
    for i in 0..200 {
        let t = i as f64 * 0.1;
        x.push(Value::Float(t.cos() * 5.0));
        y.push(Value::Float(t.sin() * 5.0));
        z.push(Value::Float(t));
    }
    let data = vec![
        ("x".to_string(), x),
        ("y".to_string(), y),
        ("z".to_string(), z),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_bin2d()
        .stat(StatSummary2d::new(SummaryFun::Mean).with_bins(10, 10))
        .scale_fill_viridis_c()
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
