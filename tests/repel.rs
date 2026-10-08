//! geom_text_repel / geom_label_repel on the native SVG path.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

/// Residual-vs-leverage-like cloud with a few labelled influential points.
fn cloud(n: usize) -> Vec<(String, Vec<Value>)> {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut l = Vec::new();
    for i in 0..n {
        let t = i as f64;
        x.push(Value::Float((t * 0.37).sin() * 2.0 + t * 0.01));
        y.push(Value::Float((t * 0.73).cos() * 1.5));
        l.push(if i % 3 == 0 {
            Value::Str(format!("obs {i}"))
        } else {
            Value::Str(String::new())
        });
    }
    vec![
        ("x".to_string(), x),
        ("y".to_string(), y),
        ("label".to_string(), l),
    ]
}

/// Parse `<text x=".." y=".." font-size="..">label</text>` boxes for labels
/// starting with `prefix` (approximate extents as the layout uses them).
fn label_boxes(svg: &str, prefix: &str) -> Vec<(f64, f64, f64, f64)> {
    let mut out = Vec::new();
    for part in svg.split("<text ").skip(1) {
        let attr = |name: &str| -> f64 {
            part.split(&format!("{name}=\""))
                .nth(1)
                .and_then(|r| r.split('"').next())
                .and_then(|v| v.parse().ok())
                .unwrap()
        };
        let text = part.split('>').nth(1).unwrap().split('<').next().unwrap();
        if !text.starts_with(prefix) {
            continue;
        }
        let (x, y, fs) = (attr("x"), attr("y"), attr("font-size"));
        let hw = text.chars().count() as f64 * fs * 0.3;
        out.push((x - hw, y - fs / 2.0, x + hw, y + fs / 2.0));
    }
    out
}

fn overlaps(b: &[(f64, f64, f64, f64)]) -> usize {
    let mut c = 0;
    for i in 0..b.len() {
        for j in i + 1..b.len() {
            if b[i].0 < b[j].2 - 1e-6
                && b[j].0 < b[i].2 - 1e-6
                && b[i].1 < b[j].3 - 1e-6
                && b[j].1 < b[i].3 - 1e-6
            {
                c += 1;
            }
        }
    }
    c
}

fn render(geom: GeomTextRepel, n: usize) -> (String, Vec<String>) {
    GGPlot::new(cloud(n))
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_point()
        .geom_text_repel_with(geom)
        .render_svg_native_with_warnings(640, 480)
        .unwrap()
}

#[test]
fn repelled_labels_do_not_overlap_and_are_deterministic() {
    let (a, wa) = render(GeomTextRepel::default(), 60);
    let (b, _) = render(GeomTextRepel::default(), 60);
    assert_eq!(a, b, "same seed → identical SVG");
    let boxes = label_boxes(&a, "obs ");
    assert_eq!(boxes.len(), 20, "{wa:?}");
    assert_eq!(overlaps(&boxes), 0);
    // Every label stays inside the panel.
    let dp: Vec<f64> = a
        .split("data-plot=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .split(' ')
        .map(|v| v.parse().unwrap())
        .collect();
    for (x0, y0, x1, y1) in &boxes {
        assert!(*x0 >= dp[0] - 0.5 && *x1 <= dp[0] + dp[2] + 0.5);
        assert!(*y0 >= dp[1] - 0.5 && *y1 <= dp[1] + dp[3] + 0.5);
    }
}

#[test]
fn different_seed_still_valid() {
    let (svg, _) = render(GeomTextRepel::default().seed(99), 60);
    assert_eq!(overlaps(&label_boxes(&svg, "obs ")), 0);
}

#[test]
fn displaced_labels_get_segments() {
    // Four labelled points mid-panel (plus unlabelled extremes setting the
    // range); a large upward nudge displaces every label well away.
    let xs = [1.0, 2.0, 3.0, 4.0, 1.0, 4.0];
    let ys = [0.0, 0.0, 0.0, 0.0, -3.0, 3.0];
    let labels = ["a", "b", "c", "d", "", ""];
    let data = vec![
        (
            "x".to_string(),
            xs.iter().map(|v| Value::Float(*v)).collect(),
        ),
        (
            "y".to_string(),
            ys.iter().map(|v| Value::Float(*v)).collect(),
        ),
        (
            "label".to_string(),
            labels.iter().map(|v| Value::Str(v.to_string())).collect(),
        ),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_point()
        .geom_text_repel_with(
            GeomTextRepel::default()
                .nudge(0.0, 1.5)
                .segment_color((200, 0, 0)),
        )
        .render_svg_native_with_size(500, 400)
        .unwrap();
    assert_eq!(
        svg.matches("stroke=\"#C80000\"").count(),
        4,
        "one segment per label"
    );
    // Without displacement no segment is needed.
    let (svg, _) = render(GeomTextRepel::default().segment_color((200, 0, 0)), 3);
    assert_eq!(svg.matches("stroke=\"#C80000\"").count(), 0);
}

#[test]
fn max_overlaps_drops_with_warning() {
    // Huge labels in a small plot cannot all be placed.
    let (svg, warnings) = GGPlot::new(cloud(90))
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_point()
        .geom_text_repel_with(GeomTextRepel {
            size: 22.0,
            ..GeomTextRepel::default().max_overlaps(0).max_iter(200)
        })
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    let w = warnings
        .iter()
        .find(|w| w.contains("unlabeled data point"))
        .expect("drop warning");
    let dropped: usize = w
        .split(": ")
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let drawn = label_boxes(&svg, "obs ").len();
    assert_eq!(drawn + dropped, 30);
    assert_eq!(overlaps(&label_boxes(&svg, "obs ")), 0);
}

#[test]
fn label_cap_warns() {
    let (_, warnings) = render(GeomTextRepel::default().max_labels(5), 60);
    assert!(
        warnings.iter().any(|w| w.contains("repel limit of 5")),
        "{warnings:?}"
    );
}

#[test]
fn label_repel_boxes_carry_host_attributes_and_escape_text() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(1.0), Value::Float(2.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(1.0), Value::Float(f64::NAN)],
        ),
        (
            "label".to_string(),
            vec![
                Value::Str("<script>".into()),
                Value::Str("b\"q".into()),
                Value::Str("gone".into()),
            ],
        ),
    ];
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label_repel()
        .render_svg_native_with_warnings(400, 300)
        .unwrap();
    assert!(!svg.contains("<script>"));
    assert!(svg.contains("&lt;script&gt;"));
    assert!(
        svg.contains("data-x=\"1\""),
        "label boxes are hoverable marks"
    );
    assert!(!svg.contains(">gone<"), "non-finite row dropped");
    assert!(warnings.iter().any(|w| w.contains("non-finite")));
    assert!(!svg.contains("NaN"));
}

#[test]
fn large_n_terminates_quickly() {
    let start = std::time::Instant::now();
    let data = cloud(1500);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_repel()
        .render_svg_native_with_size(800, 600)
        .unwrap();
    assert!(svg.ends_with("</svg>"));
    // 500 labels: bounded by max_time (0.5 s) plus drawing.
    assert!(start.elapsed().as_secs_f64() < 20.0);
}

#[test]
fn faceted_repel_renders() {
    let mut d = cloud(40);
    d.push((
        "f".to_string(),
        (0..40)
            .map(|i| Value::Str(["a", "b"][i % 2].into()))
            .collect(),
    ));
    let svg = GGPlot::new(d)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_point()
        .geom_text_repel()
        .facet_wrap("f", None)
        .render_svg_native()
        .unwrap();
    assert!(svg.contains(">obs 0<"));
}
