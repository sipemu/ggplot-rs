//! Issue #13 — expanded scale transforms render end-to-end.

use ggplot_rs::prelude::*;

fn data(ys: &[f64]) -> Vec<(String, Vec<Value>)> {
    let xs: Vec<Value> = (0..ys.len())
        .map(|i| Value::Float(i as f64 + 1.0))
        .collect();
    vec![
        ("x".to_string(), xs),
        (
            "y".to_string(),
            ys.iter().map(|v| Value::Float(*v)).collect(),
        ),
    ]
}

fn renders(plot: GGPlot) {
    assert!(plot.render_svg().is_ok());
}

#[test]
fn logit_and_probit_render() {
    let props = [0.05, 0.2, 0.4, 0.6, 0.8, 0.95];
    renders(
        GGPlot::new(data(&props))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_y_logit(),
    );
    renders(
        GGPlot::new(data(&props))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_y_probit(),
    );
}

#[test]
fn pseudo_log_handles_negatives_and_zero() {
    let ys = [-100.0, -1.0, 0.0, 1.0, 100.0];
    renders(
        GGPlot::new(data(&ys))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_y_pseudo_log(),
    );
}

#[test]
fn reciprocal_exp_boxcox_render() {
    let ys = [1.0, 2.0, 3.0, 5.0, 8.0];
    renders(
        GGPlot::new(data(&ys))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_y_reciprocal(),
    );
    renders(
        GGPlot::new(data(&ys))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_x_exp(),
    );
    renders(
        GGPlot::new(data(&ys))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_y_boxcox(0.5),
    );
    renders(
        GGPlot::new(data(&ys))
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_y_boxcox(0.0),
    );
}
