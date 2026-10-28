//! sipemu/ggplot-rs#348: the y title sat a fixed distance from the panel,
//! touching short tick labels; and a faceted grid drew the x title under
//! every bottom panel (and the y title beside every row).

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

/// `(body, x, y, font-size, anchor, rotated)` of every `<text>`.
fn texts(svg: &str) -> Vec<(String, f64, f64, f64, String, bool)> {
    svg.split("<text")
        .skip(1)
        .map(|t| {
            let open = t.split('>').next().unwrap();
            let attr = |n: &str| -> String {
                open.split(&format!(" {n}=\""))
                    .nth(1)
                    .map(|r| r.split('"').next().unwrap().to_string())
                    .unwrap_or_default()
            };
            let body = t
                .split('>')
                .nth(1)
                .unwrap()
                .split('<')
                .next()
                .unwrap()
                .to_string();
            (
                body,
                attr("x").parse().unwrap(),
                attr("y").parse().unwrap(),
                attr("font-size").parse().unwrap(),
                attr("text-anchor"),
                open.contains("rotate("),
            )
        })
        .collect()
}

#[test]
fn y_title_keeps_a_gap_to_short_tick_labels() {
    let d = vec![
        (
            "region".to_string(),
            ["North", "South", "East"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
        (
            "v".to_string(),
            vec![Value::Float(3.0), Value::Float(5.0), Value::Float(2.0)],
        ),
    ];
    let svg = GGPlot::new(d)
        .aes(Aes::new().x("region").y("v"))
        .geom_col()
        .coord_flip()
        .xlab("Region")
        .render_svg_native_with_size(600, 300)
        .unwrap();
    let t = texts(&svg);
    let title = t.iter().find(|x| x.0 == "Region").expect("y title");
    assert!(title.5, "rotated");
    // Rotated 270°: glyphs extend ~0.25 em right of x.
    let title_right = title.1 + 0.25 * title.3;
    for label in ["North", "South", "East"] {
        let l = t.iter().find(|x| x.0 == label).unwrap();
        assert_eq!(l.4, "end");
        // A generous (wide-font) estimate of the label's left edge.
        let left = l.1 - label.chars().count() as f64 * 0.65 * l.3;
        assert!(
            left - title_right >= 2.0,
            "{label} at {left} vs title {title_right}"
        );
    }
}

#[test]
fn facet_wrap_draws_one_x_and_one_y_title() {
    let n = 40;
    let d = vec![
        (
            "x".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect::<Vec<_>>(),
        ),
        (
            "y".to_string(),
            (0..n).map(|i| Value::Float((i * 7 % 11) as f64)).collect(),
        ),
        (
            "g".to_string(),
            (0..n)
                .map(|i| Value::Str(["a", "b", "c", "d"][i % 4].into()))
                .collect(),
        ),
    ];
    let svg = GGPlot::new(d)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .facet_wrap("g", Some(2))
        .xlab("Month")
        .ylab("Sales")
        .render_svg_native_with_size(600, 500)
        .unwrap();
    let t = texts(&svg);
    let xs: Vec<_> = t.iter().filter(|x| x.0 == "Month").collect();
    let ys: Vec<_> = t.iter().filter(|x| x.0 == "Sales").collect();
    assert_eq!(xs.len(), 1, "{xs:?}");
    assert_eq!(ys.len(), 1, "{ys:?}");
    // Centred under the grid / beside it, not under one panel.
    let strips: Vec<_> = t
        .iter()
        .filter(|x| ["a", "b"].contains(&x.0.as_str()))
        .collect();
    let mid = (strips[0].1 + strips[1].1) / 2.0;
    assert!(
        (xs[0].1 - mid).abs() < 2.0,
        "x title at {} vs grid centre {mid}",
        xs[0].1
    );
    // Vertically centred on the grid: from the first strip down to the
    // x tick labels under the bottom row.
    let top = strips[0].2;
    let bottom = t
        .iter()
        .filter(|x| !x.5 && x.0 != "Month")
        .map(|x| x.2)
        .fold(f64::MIN, f64::max);
    let mid = (top + bottom) / 2.0;
    assert!(
        (ys[0].2 - mid).abs() < 20.0,
        "y title at {} vs grid middle {mid}",
        ys[0].2
    );
}
