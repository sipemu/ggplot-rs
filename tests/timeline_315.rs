//! `geom_timeline` (#315).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::stat::calendar::days_from_civil;
use ggplot_rs::GGPlot;

fn events() -> Vec<(String, Vec<Value>)> {
    let d = |m: u32, day: u32| Value::DateTime(days_from_civil(2024, m, day) * 86_400);
    vec![
        ("when".into(), vec![d(1, 1), d(1, 3), d(1, 5), d(3, 1)]),
        ("end".into(), vec![Value::Na, Value::Na, d(2, 1), Value::Na]),
        (
            "what".into(),
            ["Start", "Hire", "Build phase", "Ship"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
    ]
}

#[test]
fn labels_dates_spans_and_hidden_y_axis() {
    let (svg, warnings) = GGPlot::new(events())
        .aes(Aes::new().x("when").label("what"))
        .geom_timeline_with(ggplot_rs::geom::timeline::GeomTimeline::default().with_end("end"))
        .render_svg_native_with_warnings(600, 300)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    for t in ["Start", "Hire", "Build phase", "Ship"] {
        assert!(svg.contains(&format!(">{t}<")), "{t}");
    }
    assert!(svg.contains("<title>2024-01-05 – 2024-02-01: Build phase</title>"));
    // Close events stack on different levels: their boxes don't overlap.
    let boxes: Vec<(f64, f64, f64, f64)> = svg
        .match_indices("<rect")
        .filter_map(|(i, _)| {
            let tag = &svg[i..i + svg[i..].find('>').unwrap()];
            if !tag.contains("data-geom=\"timeline\"") || !tag.contains("stroke=\"#1F77B4\"") {
                return None;
            }
            let num = |k: &str| {
                let p = tag.find(&format!(" {k}=\"")).unwrap() + k.len() + 3;
                tag[p..p + tag[p..].find('"').unwrap()]
                    .parse::<f64>()
                    .unwrap()
            };
            Some((num("x"), num("y"), num("width"), num("height")))
        })
        .collect();
    assert_eq!(boxes.len(), 4);
    for i in 0..4 {
        for j in i + 1..4 {
            let (a, b) = (boxes[i], boxes[j]);
            let apart =
                a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1;
            assert!(apart, "{a:?} {b:?}");
        }
    }
    assert!(svg.contains(">2024-02-01<"), "date axis kept");
}

#[test]
fn colours_ink_paper_unlabelled_events_and_crowding() {
    use ggplot_rs::theme::Theme;
    let d = |m: u32, day: u32| Value::DateTime(days_from_civil(2024, m, day) * 86_400);
    let many: Vec<Value> = (1..=20).map(|k| d(1, k)).collect();
    let data = vec![
        ("when".to_string(), many.clone()),
        (
            "what".to_string(),
            (0..20)
                .map(|k| {
                    if k == 3 {
                        Value::Str(String::new())
                    } else {
                        Value::Str(format!("Event number {k}"))
                    }
                })
                .collect(),
        ),
        (
            "kind".to_string(),
            (0..20)
                .map(|k| Value::Str(if k % 2 == 0 { "a" } else { "b" }.into()))
                .collect(),
        ),
    ];
    // Colour mapping + dark theme ink/paper + a cramped panel: labels that
    // find no free level are skipped, the unlabelled event keeps its dot.
    let svg = GGPlot::new(data.clone())
        .aes(Aes::new().x("when").label("what").color("kind"))
        .geom_timeline()
        .theme(Theme::default().with_ink_paper((240, 240, 240), (20, 20, 20)))
        .render_svg_native_with_size(400, 160)
        .unwrap();
    assert_eq!(
        svg.matches("<circle").count(),
        20,
        "every event keeps its dot"
    );
    assert!(
        svg.matches(">Event number").count() < 19,
        "crowded labels are dropped"
    );
    assert!(svg.contains("fill=\"#141414\""), "paper label boxes");
    // Brand primary colours unmapped events.
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("when").label("what"))
        .geom_timeline()
        .primary_color((200, 0, 100))
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert!(svg.contains("#C80064"));
}
