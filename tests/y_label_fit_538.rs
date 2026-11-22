//! #538: a flipped bar chart drawn narrow keeps its long category labels on
//! the canvas — a label wider than the room left of the panel is shortened
//! with "…" instead of running past the left edge. A wide plot, where every
//! label fits, draws them whole.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::render::text::text_width;
use ggplot_rs::GGPlot;

fn bars() -> GGPlot {
    let names = [
        "#1 YJ_STU",
        "#10 DenisKokosinskiy_STU",
        "one global model (anofox_ml)",
        "ES_bu",
    ];
    GGPlot::new(vec![
        (
            "who".into(),
            names.iter().map(|s| Value::Str(s.to_string())).collect(),
        ),
        (
            "score".into(),
            [0.52, 0.55, 0.62, 0.67]
                .iter()
                .map(|&f| Value::Float(f))
                .collect(),
        ),
    ])
    .aes(Aes::new().x("who").y("score"))
    .geom_col()
    .coord_flip()
}

/// Each y tick label: its text, its right-anchored x and its font size.
fn y_labels(svg: &str) -> Vec<(String, f64, f64)> {
    svg.split("<text ")
        .skip(1)
        .filter(|t| t.contains("text-anchor=\"end\""))
        .filter_map(|t| {
            let x = t.split("x=\"").nth(1)?.split('"').next()?.parse().ok()?;
            let size = t
                .split("font-size=\"")
                .nth(1)?
                .split('"')
                .next()?
                .parse()
                .ok()?;
            let text = &t[t.find('>')? + 1..t.find("</text>")?];
            Some((text.replace("&#39;", "'"), x, size))
        })
        .collect()
}

#[test]
fn a_narrow_flipped_chart_shortens_labels_to_stay_on_the_canvas() {
    let svg = bars().render_svg_native_with_size(220, 240).unwrap();
    let labels = y_labels(&svg);
    assert_eq!(labels.len(), 4, "{svg}");
    for (text, x, size) in &labels {
        assert!(
            x - text_width(text, *size) >= 0.0,
            "{text:?} at {x} runs off the canvas"
        );
    }
    assert!(labels.iter().any(|(t, ..)| t.ends_with('…')), "{labels:?}");
    assert!(
        labels.iter().any(|(t, ..)| t == "ES_bu"),
        "short labels stay whole"
    );
}

#[test]
fn a_wide_flipped_chart_keeps_every_label_whole() {
    let svg = bars().render_svg_native_with_size(900, 240).unwrap();
    let labels = y_labels(&svg);
    assert!(labels.iter().all(|(t, ..)| !t.ends_with('…')), "{labels:?}");
    assert!(labels
        .iter()
        .any(|(t, ..)| t == "one global model (anofox_ml)"));
}
