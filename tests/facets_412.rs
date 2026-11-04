//! #412: facet_wrap dir = "v", as.table = FALSE, label_wrap_gen.

use ggplot_rs::facet::FacetLabeller;
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let g = ["alpha beta gamma", "b", "c", "d", "e"];
    vec![
        ("x".into(), (0..5).map(|i| Value::Float(i as f64)).collect()),
        ("y".into(), (0..5).map(|i| Value::Float(i as f64)).collect()),
        (
            "g".into(),
            g.iter().map(|s| Value::Str(s.to_string())).collect(),
        ),
    ]
}

/// (x, y) of each strip label's text, by label.
fn strip_pos(svg: &str, label: &str) -> (f64, f64) {
    let i = svg.find(&format!(">{label}<")).expect(label);
    let tag = &svg[svg[..i].rfind("<text").unwrap()..i];
    let get = |k: &str| -> f64 {
        let a = tag.find(&format!(" {k}=\"")).unwrap() + k.len() + 3;
        tag[a..].split('"').next().unwrap().parse().unwrap()
    };
    (get("x"), get("y"))
}

fn render(p: GGPlot) -> String {
    p.render_svg_native_with_size(600, 400).unwrap()
}

#[test]
fn vertical_dir_fills_columns_first() {
    let base = || {
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .facet_wrap("g", Some(3))
    };
    // dir = "h": b is right of the first panel, same row.
    let h = render(base());
    let (bx, by) = strip_pos(&h, "b");
    let (dx, dy) = strip_pos(&h, "d");
    assert!(dy > by && dx < bx, "h: d starts row 2");
    // dir = "v": b sits under the first panel (column 1, row 2).
    let v = render(base().facet_wrap_dir(true, true));
    let (vbx, vby) = strip_pos(&v, "b");
    let (vcx, vcy) = strip_pos(&v, "c");
    assert!(vby > vcy && vcx > vbx, "v: b under a, c starts column 2");
    // as.table = FALSE: row 1 (b) moves below row 2 (d).
    let t = render(base().facet_wrap_dir(false, false));
    assert!(
        strip_pos(&t, "b").1 > strip_pos(&t, "d").1,
        "first row at the bottom"
    );
}

#[test]
fn label_wrap_splits_long_strip_labels() {
    let svg = render(
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .facet_wrap_labeller("g", Some(3), FacetLabeller::Wrap(6)),
    );
    for line in ["alpha", "beta", "gamma"] {
        assert!(svg.contains(&format!(">{line}<")), "line {line}");
    }
    assert!(!svg.contains(">alpha beta gamma<"));
}
