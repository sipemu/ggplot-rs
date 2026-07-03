//! Regression: position_stack/fill put the first group on TOP (ggplot2 order),
//! so the stack top-to-bottom matches the legend. Verified vs ggplot2 4.0.3.
use ggplot_rs::prelude::*;

fn first_group_segment(pos: &str) -> (f64, f64) {
    let x: Vec<Value> = [1.0, 1.0].iter().map(|v| Value::Float(*v)).collect();
    let y: Vec<Value> = [3.0, 2.0].iter().map(|v| Value::Float(*v)).collect();
    let f: Vec<Value> = ["g1", "g2"]
        .iter()
        .map(|s| Value::Str(s.to_string()))
        .collect();
    let p = GGPlot::new(vec![
        ("x".to_string(), x),
        ("y".to_string(), y),
        ("fill".to_string(), f),
    ])
    .aes(Aes::new().x("x").y("y").fill("fill"))
    .geom_col();
    let p = if pos == "stack" {
        p.position(ggplot_rs::position::stack::PositionStack)
    } else {
        p.position(ggplot_rs::position::fill::PositionFill)
    };
    let b = p.build();
    let d = &b.layers[0].data;
    // g1 is the first row.
    let i = (0..d.nrows())
        .find(|&i| d.column("fill").unwrap()[i] == Value::Str("g1".into()))
        .unwrap();
    (
        d.column("ymin").unwrap()[i].as_f64().unwrap(),
        d.column("y").unwrap()[i].as_f64().unwrap(),
    )
}

#[test]
fn stack_puts_first_group_on_top() {
    // R: g1 (y=3) -> [2, 5] on top of g2 (y=2) -> [0, 2].
    let (ymin, ymax) = first_group_segment("stack");
    assert!(
        (ymin - 2.0).abs() < 1e-9 && (ymax - 5.0).abs() < 1e-9,
        "g1 stack [{ymin},{ymax}], want [2,5]"
    );
}

#[test]
fn fill_puts_first_group_on_top() {
    // R: g1 -> [0.4, 1].
    let (ymin, ymax) = first_group_segment("fill");
    assert!(
        (ymin - 0.4).abs() < 1e-9 && (ymax - 1.0).abs() < 1e-9,
        "g1 fill [{ymin},{ymax}], want [0.4,1]"
    );
}
