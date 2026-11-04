//! #413: `plot.tag.location = "panel"`.

use ggplot_rs::prelude::*;

fn tag_x(panel: bool) -> f64 {
    let svg = GGPlot::new(vec![
        (
            "x".to_string(),
            (0..4).map(|i| Value::Float(i as f64 * 1000.0)).collect(),
        ),
        (
            "y".to_string(),
            (0..4).map(|i| Value::Float(i as f64 * 1000.0)).collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .tag("A")
    .theme_with(|t| t.tag_location_panel = panel)
    .render_svg_native_with_size(400, 300)
    .unwrap();
    let t = svg.split("<text").find(|t| t.contains(">A<")).unwrap();
    let a = t.find(" x=\"").unwrap() + 4;
    t[a..].split('"').next().unwrap().parse().unwrap()
}

#[test]
fn panel_tag_sits_right_of_the_y_axis() {
    assert!(tag_x(true) > tag_x(false) + 10.0);
}
