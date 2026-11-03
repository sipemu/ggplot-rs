//! Per-row `hjust` / `vjust` aesthetics on `geom_text` (R's `aes(hjust =,
//! vjust =)`), and `hjust` outside [0, 1] moving the text by a share of its
//! width (anofox-visualization#959: bar value labels outside a bar's end).
use ggplot_rs::prelude::*;

fn texts(svg: &str) -> Vec<(String, f64, f64, String)> {
    svg.split("<text")
        .skip(1)
        .filter_map(|t| {
            let a = |k: &str| {
                t.split(&format!(" {k}=\""))
                    .nth(1)?
                    .split('"')
                    .next()
                    .map(str::to_string)
            };
            let label = t.split('>').nth(1)?.split('<').next()?.to_string();
            Some((
                label,
                a("x")?.parse().ok()?,
                a("y")?.parse().ok()?,
                a("text-anchor").unwrap_or_default(),
            ))
        })
        .filter(|(l, ..)| l.starts_with('L'))
        .collect()
}

fn draw(hj: [f64; 2], vj: [f64; 2]) -> Vec<(String, f64, f64, String)> {
    let data = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(1.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(1.0)]),
        (
            "l".to_string(),
            vec![Value::Str("La".into()), Value::Str("Lb".into())],
        ),
        (
            "hj".to_string(),
            hj.iter().map(|v| Value::Float(*v)).collect(),
        ),
        (
            "vj".to_string(),
            vj.iter().map(|v| Value::Float(*v)).collect(),
        ),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("l").hjust("hj").vjust("vj"))
        .geom_text()
        .render_svg_native()
        .unwrap();
    texts(&svg)
}

#[test]
fn rows_take_their_own_justification() {
    let t = draw([0.5, 0.5], [-0.5, 1.5]);
    assert_eq!(t.len(), 2, "{t:?}");
    assert!(t[0].2 < t[1].2, "vjust -0.5 sits above vjust 1.5: {t:?}");
    let t = draw([0.0, 1.0], [0.5, 0.5]);
    assert_eq!(
        (t[0].3.as_str(), t[1].3.as_str()),
        ("start", "end"),
        "{t:?}"
    );
    // beyond [0, 1]: moved by a share of the text's width
    let (a, b) = (draw([0.0, 0.0], [0.5, 0.5]), draw([-0.5, 1.5], [0.5, 0.5]));
    assert!(
        b[0].1 > a[0].1 + 1.0,
        "hjust -0.5 starts right of the point: {a:?} {b:?}"
    );
    assert!(b[1].1 < a[0].1 - 1.0, "hjust 1.5 ends left of it: {b:?}");
}
