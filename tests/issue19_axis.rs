//! Issue #19 — per-side expansion + top axis position.
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".into(),
            (0..10).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".into(),
            (0..10).map(|i| Value::Float((i % 4) as f64)).collect(),
        ),
    ]
}

#[test]
fn per_side_expansion_is_asymmetric() {
    use ggplot_rs::scale::continuous::ScaleContinuous;
    use ggplot_rs::scale::Scale;
    let mut s = ScaleContinuous::new().with_expand_sides(0.0, 0.0, 0.0, 5.0);
    s.train(&[Value::Float(0.0), Value::Float(10.0)]);
    // Lower side: no expansion → min maps to 0. Upper: +5 → max maps to 10/15.
    assert!(
        (s.map(&Value::Float(0.0)) - 0.0).abs() < 1e-9,
        "lower not expanded"
    );
    let hi = s.map(&Value::Float(10.0));
    assert!(
        (hi - 10.0 / 15.0).abs() < 1e-6,
        "upper expansion wrong: {hi}"
    );
}

#[test]
fn x_axis_top_renders() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_continuous(ScaleContinuous::new().with_position_opposite())
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
