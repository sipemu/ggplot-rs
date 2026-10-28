//! `geom_jitter` / `position_jitter` / `position_jitterdodge` on a discrete
//! x: the categories can't move, so the jitter used to be dropped and every
//! point sat on the category's centre line.

use ggplot_rs::data::Value;
use ggplot_rs::position::jitterdodge::PositionJitterDodge;
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let n = 40;
    vec![
        (
            "g".to_string(),
            (0..n)
                .map(|i| Value::Str(["a", "b"][i % 2].into()))
                .collect(),
        ),
        (
            "h".to_string(),
            (0..n)
                .map(|i| Value::Str(["u", "v"][(i / 2) % 2].into()))
                .collect(),
        ),
        (
            "y".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
    ]
}

fn distinct_cx(svg: &str) -> usize {
    let mut xs: Vec<String> = svg
        .split("<circle")
        .skip(1)
        .filter_map(|c| {
            c.split(" cx=\"")
                .nth(1)?
                .split('"')
                .next()
                .map(String::from)
        })
        .collect();
    xs.sort();
    xs.dedup();
    xs.len()
}

#[test]
fn geom_jitter_spreads_points_around_each_category() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("g").y("y"))
        .geom_jitter()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(distinct_cx(&svg) > 10, "points stay on two lines");
}

#[test]
fn jitterdodge_spreads_points_on_a_discrete_axis() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("g").y("y").color("h"))
        .geom_point()
        .position(PositionJitterDodge::default())
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(distinct_cx(&svg) > 10, "points stay on dodge lines");
}
