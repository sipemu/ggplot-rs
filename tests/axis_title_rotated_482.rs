//! sipemu/ggplot-rs#482: the x-axis title is drawn below auto-rotated tick
//! labels (it used to sit one text line under the axis, through them).
use ggplot_rs::prelude::*;

fn attr(tag: &str, a: &str) -> Option<f64> {
    let k = tag.find(&format!(" {a}=\""))? + a.len() + 3;
    tag[k..k + tag[k..].find('"')?].parse().ok()
}

#[test]
fn x_title_clears_rotated_labels() {
    let names = [
        "Industrial adhesives",
        "Protective coatings",
        "Specialty lubricants",
        "Construction sealants",
        "Automotive cleaners",
        "Packaging films",
        "Electronic potting",
        "Marine antifouling",
        "Textile finishes",
    ];
    let x: Vec<Value> = names.iter().map(|s| Value::Str(s.to_string())).collect();
    let y: Vec<Value> = (0..names.len())
        .map(|i| Value::Float(100.0 - 9.0 * i as f64))
        .collect();
    let svg = GGPlot::new(vec![("x".into(), x), ("y".into(), y)])
        .aes(Aes::new().x("x").y("y"))
        .geom_col()
        .xlab("Product family")
        .render_svg_native_with_size(700, 350)
        .expect("renders");
    if let Ok(p) = std::env::var("SVG_OUT") {
        std::fs::write(p, &svg).unwrap();
    }
    // the rotated labels: their anchor y and their text length
    let mut lowest: f64 = 0.0;
    for n in names {
        let i = svg.find(&format!(">{n}<")).expect("label drawn");
        let tag = &svg[svg[..i].rfind("<text").unwrap()..i];
        assert!(tag.contains("rotate("), "{n} is rotated: {tag}");
        let ay = attr(tag, "y").unwrap();
        // a −45° label hangs ~ its length · sin 45° below its anchor
        lowest = lowest.max(ay + n.len() as f64 * 4.0 * std::f64::consts::FRAC_1_SQRT_2);
    }
    let i = svg.find(">Product family<").expect("title drawn");
    let tag = &svg[svg[..i].rfind("<text").unwrap()..i];
    let ty = attr(tag, "y").unwrap();
    assert!(
        ty > lowest,
        "title y {ty} below the rotated labels (lowest ≈ {lowest})"
    );
}
