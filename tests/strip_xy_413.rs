//! #413: `strip_text_x` / `strip_text_y` / `strip_background_x` / `_y` —
//! separate looks for horizontal and turned facet strips.

use ggplot_rs::prelude::*;

fn plot(t: Theme) -> String {
    let n = 8;
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
            "r".to_string(),
            (0..n)
                .map(|i| Value::Str(["r1", "r2"][i % 2].into()))
                .collect(),
        ),
        (
            "c".to_string(),
            (0..n)
                .map(|i| Value::Str(["c1", "c2"][(i / 2) % 2].into()))
                .collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .facet_grid(Some("r"), Some("c"))
    .theme(t)
    .render_svg_native_with_size(400, 300)
    .unwrap()
}

fn fill_of(svg: &str, label: &str) -> String {
    let t = svg
        .split("<text")
        .find(|t| {
            t.split('>')
                .nth(1)
                .is_some_and(|c| c.starts_with(&format!("{label}<")))
        })
        .unwrap();
    let a = t.find(" fill=\"").unwrap() + 7;
    t[a..].split('"').next().unwrap().to_string()
}

#[test]
fn row_and_column_strips_styled_separately() {
    let mut t = theme_gray();
    t.strip_text_y = Some(ElementText {
        color: (200, 0, 0),
        ..t.strip_text.clone()
    });
    t.strip_background_x = Some(ElementRect {
        fill: Some((0, 0, 200)),
        ..t.strip_background.clone()
    });
    let s = plot(t);
    assert_eq!(fill_of(&s, "r1"), "#C80000");
    assert_ne!(fill_of(&s, "c1"), "#C80000");
    assert!(s.contains("fill=\"#0000C8\""), "column strip background");
    // Defaults unchanged without them.
    let base = plot(theme_gray());
    assert_eq!(fill_of(&base, "r1"), fill_of(&base, "c1"));
}
