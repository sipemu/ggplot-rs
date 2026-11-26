//! #543: `direct_labels()` names each line at its end instead of drawing a
//! colour legend.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut s = Vec::new();
    for (name, base) in [("Chain", 10.0), ("Shortcut", 10.5), ("What arrived", 30.0)] {
        for i in 0..10 {
            x.push(Value::Float(i as f64));
            y.push(Value::Float(base + i as f64));
            s.push(Value::Str(name.into()));
        }
    }
    vec![("x".into(), x), ("y".into(), y), ("series".into(), s)]
}

fn plot() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("series"))
        .geom_line()
}

/// The fill of the `<text>` that reads `name`.
fn fill_of(svg: &str, name: &str) -> Option<String> {
    svg.split("<text").skip(1).find_map(|t| {
        let (head, rest) = t.split_once('>')?;
        rest.starts_with(&format!("{name}<"))
            .then(|| {
                head.split("fill=\"")
                    .nth(1)?
                    .split('"')
                    .next()
                    .map(str::to_string)
            })
            .flatten()
    })
}

fn label_ys(svg: &str) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    for part in svg.split("<text").skip(1) {
        let Some(head) = part.split('>').next() else {
            continue;
        };
        let text = part
            .split('>')
            .nth(1)
            .and_then(|t| t.split('<').next())
            .unwrap_or("");
        if ["Chain", "Shortcut", "What arrived"].contains(&text) {
            let y = head
                .split("y=\"")
                .nth(1)
                .and_then(|v| v.split('"').next())
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(f64::NAN);
            out.push((text.to_string(), y));
        }
    }
    out
}

#[test]
fn each_line_is_named_at_its_end_without_a_legend() {
    let with = plot()
        .direct_labels()
        .render_svg_native_with_size(500, 300)
        .unwrap();
    let labels = label_ys(&with);
    assert_eq!(labels.len(), 3, "one label per line: {labels:?}");
    // a label is drawn in its line's colour (legend text is ink-coloured)
    assert_ne!(
        fill_of(&with, "Chain").as_deref(),
        Some("#000000"),
        "labels take their line's colour"
    );
    // two ends 0.5 apart in y get pulled apart: at least a line of text between them
    let y = |n: &str| labels.iter().find(|(t, _)| t == n).unwrap().1;
    assert!(
        (y("Chain") - y("Shortcut")).abs() >= 10.0,
        "labels never overlap: {labels:?}"
    );
    // the plain plot keeps its legend
    let without = plot().render_svg_native_with_size(500, 300).unwrap();
    assert_eq!(
        fill_of(&without, "Chain").as_deref(),
        Some("#000000"),
        "without direct_labels the legend stays"
    );
}

#[test]
fn a_plot_with_nothing_to_label_keeps_its_legend() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("series"))
        .geom_point()
        .direct_labels()
        .render_svg_native_with_size(500, 300)
        .unwrap();
    assert_eq!(
        fill_of(&svg, "Chain").as_deref(),
        Some("#000000"),
        "points only: the legend is drawn"
    );
}

#[test]
fn a_narrow_render_draws_the_legend_below_instead() {
    // a phone-width chart: the labels' band would squeeze the panel
    let narrow = plot()
        .direct_labels()
        .render_svg_native_with_size(280, 300)
        .unwrap();
    assert_eq!(
        fill_of(&narrow, "Chain").as_deref(),
        Some("#000000"),
        "narrow: the usual legend (ink-coloured text), not end labels"
    );
    // wide enough: still named at the ends
    let wide = plot()
        .direct_labels()
        .render_svg_native_with_size(700, 300)
        .unwrap();
    assert_ne!(fill_of(&wide, "Chain").as_deref(), Some("#000000"));
    // an explicit share wins
    let kept = plot()
        .direct_labels()
        .legend_bottom_when_narrow(0.9)
        .render_svg_native_with_size(280, 300)
        .unwrap();
    assert_ne!(
        fill_of(&kept, "Chain").as_deref(),
        Some("#000000"),
        "legend_bottom_when_narrow(0.9) keeps the end labels"
    );
}
