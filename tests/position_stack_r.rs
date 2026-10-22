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

/// Review #167 finding 2 on sipemu/ggplot-rs#171: a stacked `geom_line`
/// with one series missing a `y` at one x must not affect the *other*
/// series at that x. Unlike `geom_col`, a line keeps a missing row
/// instead of dropping it (`Geom::keeps_na_as_break`, `build.rs`'s
/// `drop_non_finite`), so it's the one geom where `PositionStack` itself
/// — not an earlier drop — has to get this right. Three series at
/// x = 1..4 (row order: all of `a`, then all of `b`, then all of `c`),
/// `a` missing at x=3; ggplot2 (`layer_data`, `pos_stack`'s `y <-
/// ifelse(is.na(df$y), 0, df$y)`) leaves `b` and `c` stacked at x=3 as
/// if `a`'s row weren't there at all (`b` 1..2, `c` 0..1), and `a`'s own
/// row is left with no finite position (so it draws nothing there,
/// unlike the rest of its line). Before this fix, `a`'s NaN summed into
/// the shared positive-bucket total at x=3, turning `b`'s and `c`'s
/// output there into NaN too.
#[test]
fn stacked_line_with_a_missing_y_leaves_other_series_intact_at_that_x() {
    let mut x = Vec::new();
    let mut grp = Vec::new();
    let mut y = Vec::new();
    for g in ["a", "b", "c"] {
        for (xi, yi) in [(1.0, 1.0), (2.0, 1.0), (3.0, 1.0), (4.0, 1.0)] {
            x.push(Value::Float(xi));
            grp.push(Value::Str(g.to_string()));
            // `a` is missing at x=3; every other (series, x) is 1.0.
            y.push(if g == "a" && xi == 3.0 {
                Value::Float(f64::NAN)
            } else {
                Value::Float(yi)
            });
        }
    }
    let built = GGPlot::new(vec![
        ("x".to_string(), x),
        ("grp".to_string(), grp.clone()),
        ("y".to_string(), y),
    ])
    .aes(Aes::new().x("x").y("y").color("grp").group("grp"))
    .geom_line()
    .position(ggplot_rs::position::stack::PositionStack)
    .build();
    let d = &built.layers[0].data;

    let row = |g: &str, xi: f64| -> usize {
        (0..d.nrows())
            .find(|&i| {
                d.column("grp").unwrap()[i] == Value::Str(g.to_string())
                    && d.column("x").unwrap()[i].as_f64() == Some(xi)
            })
            .unwrap_or_else(|| panic!("no row for {g} at x={xi}"))
    };
    let y_at = |g: &str, xi: f64| d.column("y").unwrap()[row(g, xi)].as_f64().unwrap();
    let ymin_at = |g: &str, xi: f64| d.column("ymin").unwrap()[row(g, xi)].as_f64().unwrap();

    assert!(
        y_at("a", 3.0).is_nan(),
        "a's own missing row must not get a real position"
    );
    for (g, want_ymin, want_y) in [("b", 1.0, 2.0), ("c", 0.0, 1.0)] {
        let (ymin, y) = (ymin_at(g, 3.0), y_at(g, 3.0));
        assert!(
            (ymin - want_ymin).abs() < 1e-9 && (y - want_y).abs() < 1e-9,
            "{g} at x=3 must stack as if a's row weren't there: got [{ymin},{y}], want [{want_ymin},{want_y}]"
        );
    }
    // x=4 (and every other x) is a separate bucket entirely — a's NaN at
    // x=3 must not reach it either.
    for g in ["a", "b", "c"] {
        for xi in [1.0, 2.0, 4.0] {
            assert!(
                y_at(g, xi).is_finite() && ymin_at(g, xi).is_finite(),
                "{g} at x={xi} must stay finite"
            );
        }
    }
}
