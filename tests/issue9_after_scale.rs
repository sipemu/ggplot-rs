//! Issue #9 — after_scale() color derivation + stage().
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        ("x".into(), (0..6).map(|i| Value::Float(i as f64)).collect()),
        (
            "y".into(),
            (0..6).map(|i| Value::Float((i % 3) as f64)).collect(),
        ),
        (
            "g".into(),
            (0..6)
                .map(|i| Value::Str(["a", "b", "c"][i % 3].into()))
                .collect(),
        ),
    ]
}

#[test]
fn after_scale_fill_is_darker_than_color() {
    let built = GGPlot::new(data())
        .aes(
            Aes::new()
                .x("x")
                .y("y")
                .color("g")
                .after_scale_fill_from_color(-0.5),
        )
        .geom_point()
        .build();
    let v = Value::Str("a".into());
    let c = built.scales.map_color(&Aesthetic::Color, &v).unwrap();
    let fill = built.scales.map_color(&Aesthetic::Fill, &v).unwrap();
    assert_ne!(c, fill, "fill should differ from color");
    assert!(
        fill.0 <= c.0 && fill.1 <= c.1 && fill.2 <= c.2,
        "fill {fill:?} should be darker than color {c:?}"
    );
}

#[test]
fn after_scale_renders() {
    let svg = GGPlot::new(data())
        .aes(
            Aes::new()
                .x("g")
                .y("y")
                .fill("g")
                .after_scale_color_from_fill(-0.4),
        )
        .geom_col()
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn stage_maps_before_and_after_stat() {
    // start x = raw x, after_stat y = count / sum(count)
    let x: Vec<Value> = (0..20).map(|i| Value::Float((i % 4) as f64)).collect();
    let svg = GGPlot::new(vec![("x".to_string(), x)])
        .aes(
            Aes::new()
                .x("x")
                .stage(Aesthetic::Y, "y", "count / sum(count)"),
        )
        .geom_histogram()
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
