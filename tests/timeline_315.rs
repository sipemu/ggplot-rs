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
