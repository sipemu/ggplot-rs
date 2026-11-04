//! #412: `strip.placement = "outside"` — a switched strip sits beyond the
//! axis, not between it and the panel.

use ggplot_rs::prelude::*;

fn svg(outside: bool) -> String {
    let n = 12;
    GGPlot::new(vec![
        (
            "x".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "c".to_string(),
            (0..n)
                .map(|i| Value::Str(["c1", "c2"][i % 2].into()))
                .collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .facet_grid(None, Some("c"))
    .facet_switch(FacetSwitch::X)
    .theme_with(|t| t.strip_placement_outside = outside)
    .render_svg_native_with_size(400, 300)
    .unwrap()
}

fn y_of(s: &str, label: &str) -> f64 {
    let t = s
        .split("<text")
        .find(|t| {
            t.split('>')
                .nth(1)
                .is_some_and(|c| c.starts_with(&format!("{label}<")))
        })
        .unwrap();
    let a = t.find(" y=\"").unwrap() + 4;
    t[a..].split('"').next().unwrap().parse().unwrap()
}

#[test]
fn outside_strip_below_the_axis_labels() {
    let (inside, outside) = (svg(false), svg(true));
    // Inside: strip above the "5" tick label; outside: below it.
    assert!(y_of(&inside, "c1") < y_of(&inside, "5"));
    assert!(y_of(&outside, "c1") > y_of(&outside, "5"));
}
