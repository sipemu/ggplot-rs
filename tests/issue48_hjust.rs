//! Issue #48 — theme text hjust aligns the title (left/center/right).
use ggplot_rs::prelude::*;

fn title_anchor(hjust: f64) -> String {
    let d = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ];
    let mut t = theme_minimal();
    t.title.hjust = hjust;
    let svg = GGPlot::new(d)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme(t)
        .title("HJUSTMARK")
        .render_svg_with_size(400, 300)
        .unwrap();
    // Find the <text ...> element that contains the title and read its anchor.
    let i = svg.find("HJUSTMARK").expect("title present");
    let tstart = svg[..i].rfind("<text").expect("text element");
    let seg = &svg[tstart..i];
    for a in ["start", "middle", "end"] {
        if seg.contains(&format!("text-anchor=\"{a}\"")) {
            return a.to_string();
        }
    }
    "?".into()
}

#[test]
fn title_hjust_controls_alignment() {
    assert_eq!(title_anchor(0.0), "start", "hjust=0 should left-align");
    assert_eq!(title_anchor(0.5), "middle", "hjust=0.5 should center");
    assert_eq!(title_anchor(1.0), "end", "hjust=1 should right-align");
}
