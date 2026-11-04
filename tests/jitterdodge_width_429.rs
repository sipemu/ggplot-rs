//! #429: `position_jitterdodge()` scales its jitter by the dodge group count
//! (ggplot2's `jitter.width / (ndodge + 2)`, `jitter.width = NULL` → 0.4 ×
//! resolution), so each group's points stay over its own dodged box.

use ggplot_rs::data::Value;
use ggplot_rs::position::jitterdodge::PositionJitterDodge;
use ggplot_rs::prelude::*;

const OFF: &str = ".x_dodge_offset";

fn data(reverse: bool) -> Vec<(String, Vec<Value>)> {
    let mut rows = Vec::new();
    for x in ["a", "b"] {
        for g in ["p", "q", "r"] {
            for k in 0..30 {
                rows.push((x, g, k as f64));
            }
        }
    }
    if reverse {
        rows.reverse();
    }
    vec![
        (
            "x".into(),
            rows.iter().map(|r| Value::Str(r.0.into())).collect(),
        ),
        (
            "g".into(),
            rows.iter().map(|r| Value::Str(r.1.into())).collect(),
        ),
        ("y".into(), rows.iter().map(|r| Value::Float(r.2)).collect()),
    ]
}

fn col(d: &ggplot_rs::data::DataFrame, c: &str, i: usize) -> Value {
    d.column(c).unwrap()[i].clone()
}

#[test]
fn points_stay_inside_their_own_dodge_slot() {
    for reverse in [false, true] {
        let built = GGPlot::new(data(reverse))
            .aes(Aes::new().x("x").y("y").fill("g"))
            .geom_boxplot()
            .geom_point()
            .position(PositionJitterDodge::default())
            .build();
        // Box centre per (x, g) from the boxplot layer.
        let boxes = &built.layers[0].data;
        let centre = |x: &str, g: &str| -> f64 {
            (0..boxes.nrows())
                .find(|&i| {
                    col(boxes, "x", i).to_group_key() == x
                        && col(boxes, "fill", i).to_group_key() == g
                })
                .map(|i| col(boxes, OFF, i).as_f64().unwrap())
                .unwrap()
        };
        let pts = &built.layers[1].data;
        let slot = 0.9 / 3.0;
        let max_jitter = 0.4 / (3.0 + 2.0);
        let mut spread: f64 = 0.0;
        for i in 0..pts.nrows() {
            let (x, g) = (
                col(pts, "x", i).to_group_key(),
                col(pts, "g", i).to_group_key(),
            );
            let d = col(pts, OFF, i).as_f64().unwrap() - centre(&x, &g);
            assert!(
                d.abs() <= max_jitter + 1e-9,
                "{x}/{g}: {d} (reverse={reverse})"
            );
            assert!(d.abs() < slot / 2.0);
            spread = spread.max(d.abs());
        }
        assert!(spread > max_jitter * 0.8, "points barely jitter: {spread}");
    }
}

#[test]
fn explicit_width_is_divided_by_ndodge_plus_two() {
    let built = GGPlot::new(data(false))
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .position(PositionJitterDodge::new(1.0, 0.0).with_dodge_width(0.9))
        .build();
    let d = &built.layers[0].data;
    let offs: Vec<f64> = (0..d.nrows())
        .map(|i| col(d, OFF, i).as_f64().unwrap())
        .collect();
    // Dodge centres −0.3 / 0 / 0.3; jitter ≤ 1 / 5 either side.
    let max = offs.iter().cloned().fold(f64::MIN, f64::max);
    assert!(max <= 0.3 + 0.2 + 1e-9 && max > 0.3 + 0.15, "{max}");
}

#[test]
fn continuous_x_jitter_uses_the_resolution() {
    let n = 60;
    let built = GGPlot::new(vec![
        (
            "x".to_string(),
            (0..n).map(|i| Value::Float((i % 3) as f64 * 2.0)).collect(),
        ),
        (
            "g".to_string(),
            (0..n)
                .map(|i| Value::Str(["u", "v"][(i / 3) % 2].into()))
                .collect(),
        ),
        (
            "y".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y").color("g"))
    .geom_point()
    .position(PositionJitterDodge::default())
    .build();
    let d = &built.layers[0].data;
    // resolution 2 → jitter ≤ 0.8 / 4 = 0.2; dodge centres ±0.225.
    for i in 0..d.nrows() {
        let x = col(d, "x", i).as_f64().unwrap();
        let base = (x / 2.0).round() * 2.0;
        assert!((x - base).abs() <= 0.225 + 0.2 + 1e-9, "{x}");
    }
}
