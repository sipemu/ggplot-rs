//! A-grade #58 — coord_polar draws radial sectors (not warped rectangles).
use ggplot_rs::prelude::*;

fn polar_col_svg() -> String {
    let d = vec![
        (
            "g".to_string(),
            ["a", "b", "c", "d"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
        (
            "y".to_string(),
            [2.0, 4.0, 3.0, 5.0]
                .iter()
                .map(|v| Value::Float(*v))
                .collect(),
        ),
    ];
    GGPlot::new(d)
        .aes(Aes::new().x("g").y("y").fill("g"))
        .geom_col()
        .coord_polar()
        .render_svg()
        .unwrap()
}

#[test]
fn polar_bars_are_sector_polygons() {
    let svg = polar_col_svg();
    // Sectors are drawn as filled polygons (>=4 pts), not axis-aligned rects.
    assert!(
        svg.contains("<polygon"),
        "polar bars should render as sector polygons"
    );
}

#[test]
fn cartesian_bars_still_render() {
    let d = vec![
        (
            "g".to_string(),
            ["a", "b", "c"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
        (
            "y".to_string(),
            [2.0, 4.0, 3.0].iter().map(|v| Value::Float(*v)).collect(),
        ),
    ];
    assert!(GGPlot::new(d)
        .aes(Aes::new().x("g").y("y"))
        .geom_col()
        .render_svg()
        .is_ok());
}
