//! `geom_wordcloud` (#313).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::wordcloud::GeomWordcloud;
use ggplot_rs::GGPlot;

fn words() -> Vec<(String, Vec<Value>)> {
    let w = [
        "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta",
    ];
    vec![
        (
            "w".into(),
            w.iter().map(|s| Value::Str(s.to_string())).collect(),
        ),
        (
            "n".into(),
            (0..w.len())
                .map(|i| Value::Float(40.0 - 4.0 * i as f64))
                .collect(),
        ),
    ]
}

/// (x, y, half-width, half-height) of every word, from its font size.
fn boxes(svg: &str) -> Vec<(f64, f64, f64, f64, bool)> {
    svg.match_indices("data-geom=\"wordcloud\"")
        .map(|(i, _)| {
            let start = svg[..i].rfind("<text").unwrap();
            let tag = &svg[start..i];
            let num = |k: &str| {
                let p = tag.find(&format!(" {k}=\"")).unwrap() + k.len() + 3;
                tag[p..p + tag[p..].find('"').unwrap()]
                    .parse::<f64>()
                    .unwrap()
            };
            let rotated = tag.contains("rotate(");
            let text_end = svg[i..].find("</text>").unwrap();
            let word = &svg[i..i + text_end];
            let word = &word[word.rfind('>').unwrap() + 1..];
            let size = num("font-size");
            let w = ggplot_rs::render::text::text_width(word, size) * 1.08 / 2.0;
            let h = size * 1.05 / 2.0;
            let (hw, hh) = if rotated { (h, w) } else { (w, h) };
            (num("x"), num("y"), hw, hh, rotated)
        })
        .collect()
}

#[test]
fn words_are_placed_without_overlap_and_deterministically() {
    let render = || {
        GGPlot::new(words())
            .aes(Aes::new().label("w").weight("n"))
            .geom_wordcloud()
            .render_svg_native_with_size(600, 400)
            .unwrap()
    };
    let svg = render();
    assert_eq!(svg, render());
    let b = boxes(&svg);
    assert_eq!(b.len(), 8);
    for i in 0..b.len() {
        for j in i + 1..b.len() {
            let (p, q) = (b[i], b[j]);
            let sep = (p.0 - q.0).abs() >= p.2 + q.2 - 0.5 || (p.1 - q.1).abs() >= p.3 + q.3 - 0.5;
            assert!(sep, "words {i} and {j} overlap: {p:?} {q:?}");
        }
    }
}

#[test]
fn rotation_share_and_tiny_panel_drop_words() {
    let svg = GGPlot::new(words())
        .aes(Aes::new().label("w").weight("n"))
        .geom_wordcloud_with(GeomWordcloud {
            rotate_share: 1.0,
            ..Default::default()
        })
        .render_svg_native_with_size(600, 400)
        .unwrap();
    assert!(boxes(&svg).iter().all(|b| b.4), "all vertical");
    let small = GGPlot::new(words())
        .aes(Aes::new().label("w").weight("n"))
        .geom_wordcloud()
        .render_svg_native_with_size(140, 90)
        .unwrap();
    assert!(
        boxes(&small).len() < 8,
        "some words dropped when they can't fit"
    );
}
