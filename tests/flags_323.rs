//! `geom_flags` (#323).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::flags::{FlagShape, GeomFlags};
use ggplot_rs::GGPlot;

fn f(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&x| Value::Float(x)).collect()
}

#[test]
fn flags_on_a_series_stack_at_equal_x() {
    let xs = f(&[0.0, 1.0, 2.0, 3.0, 4.0]);
    let ys = f(&[10.0, 12.0, 11.0, 15.0, 14.0]);
    let price = vec![("t".to_string(), xs.clone()), ("p".to_string(), ys.clone())];
    let events = vec![
        ("t".to_string(), f(&[1.5, 3.0, 3.0])),
        (
            "what".to_string(),
            ["Earnings", "Split", "Dividend"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
    ];
    let (svg, warnings) = GGPlot::new(price)
        .aes(Aes::new().x("t").y("p"))
        .geom_line()
        .geom_flags_with(GeomFlags::default().on_series(&xs, &ys))
        .layer_data(events.clone())
        .layer_aes(Aes::new().x("t").label("what"))
        .render_svg_native_with_warnings(500, 300)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>1.5: Earnings</title>"));
    for t in [">Earnings<", ">Split<", ">Dividend<"] {
        assert!(svg.contains(t));
    }
    // Two flags at t=3 stack: their boxes have different tops.
    let tops: Vec<f64> = svg
        .match_indices("<rect")
        .map(|(i, _)| &svg[i..i + svg[i..].find('>').unwrap()])
        .filter(|t| t.contains("data-geom=\"flags\""))
        .map(|t| {
            let p = t.find(" y=\"").unwrap() + 4;
            t[p..p + t[p..].find('"').unwrap()].parse().unwrap()
        })
        .collect();
    assert_eq!(tops.len(), 3);
    assert!((tops[1] - tops[2]).abs() > 10.0, "{tops:?}");

    let pins = GGPlot::new(events)
        .aes(Aes::new().x("t").label("what"))
        .geom_flags_with(GeomFlags {
            shape: FlagShape::CirclePin,
            ..Default::default()
        })
        .render_svg_native_with_size(400, 200)
        .unwrap();
    assert!(pins.contains(">E<") && pins.contains(">S<"));
}

#[test]
fn axis_flags_colour_primary_paper_and_empty_labels() {
    use ggplot_rs::theme::Theme;
    let events = vec![
        ("t".to_string(), f(&[1.0, 2.0, 3.0])),
        (
            "what".to_string(),
            vec![
                Value::Str("A".into()),
                Value::Str(String::new()),
                Value::Str("C".into()),
            ],
        ),
        (
            "kind".to_string(),
            vec![
                Value::Str("x".into()),
                Value::Str("y".into()),
                Value::Str("x".into()),
            ],
        ),
    ];
    // Default builder: flags on the axis, brand primary, theme paper fill.
    let svg = GGPlot::new(events.clone())
        .aes(Aes::new().x("t").label("what"))
        .geom_flags()
        .primary_color((200, 0, 100))
        .theme(Theme::default().with_ink_paper((240, 240, 240), (20, 20, 20)))
        .render_svg_native_with_size(400, 200)
        .unwrap();
    assert!(svg.contains("#C80064"), "primary colour");
    assert!(svg.contains("fill=\"#141414\""), "paper fill");
    assert_eq!(svg.matches("data-geom=\"flags\"").count() >= 3, true);
    // Colour mapping.
    let svg = GGPlot::new(events)
        .aes(Aes::new().x("t").label("what").color("kind"))
        .geom_flags()
        .render_svg_native_with_size(400, 200)
        .unwrap();
    assert!(svg.contains(">x<") || svg.contains("#F8766D"));
}
