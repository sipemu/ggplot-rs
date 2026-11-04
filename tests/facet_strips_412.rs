//! #412: `facet_grid` draws its column strips once on top and its row
//! strips once on the right (`switch` moves them below / left), and a
//! `facet_wrap` panel's strip can sit on any side (`strip.position`).

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let n = 24;
    vec![
        ("x".into(), (0..n).map(|i| Value::Float(i as f64)).collect()),
        (
            "y".into(),
            (0..n).map(|i| Value::Float((i * 7 % 13) as f64)).collect(),
        ),
        (
            "r".into(),
            (0..n)
                .map(|i| Value::Str(["r1", "r2"][i % 2].into()))
                .collect(),
        ),
        (
            "c".into(),
            (0..n)
                .map(|i| Value::Str(["c1", "c2", "c3"][(i / 2) % 3].into()))
                .collect(),
        ),
    ]
}

fn plot() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
}

fn svg(p: GGPlot) -> String {
    p.render_svg_native_with_size(500, 350).unwrap()
}

/// `(x, y, transform)` of each `<text>` whose content is `label`.
fn texts(svg: &str, label: &str) -> Vec<(f64, f64, String)> {
    let attr = |t: &str, n: &str| -> Option<String> {
        let a = t.find(&format!(" {n}=\""))? + n.len() + 3;
        Some(t[a..].split('"').next()?.to_string())
    };
    svg.split("<text")
        .skip(1)
        .filter(|t| {
            t.split('>')
                .nth(1)
                .is_some_and(|c| c.starts_with(&format!("{label}<")))
        })
        .map(|t| {
            let tag = &t[..t.find('>').unwrap()];
            (
                attr(tag, "x").unwrap().parse().unwrap(),
                attr(tag, "y").unwrap().parse().unwrap(),
                attr(tag, "transform").unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn grid_strips_once_per_column_and_row() {
    let s = svg(plot().facet_grid(Some("r"), Some("c")));
    // One c1 strip (top row only), one r1 strip (right column only).
    assert_eq!(texts(&s, "c1").len(), 1, "{s}");
    let r1 = texts(&s, "r1");
    assert_eq!(r1.len(), 1, "row labels are drawn");
    assert!(r1[0].2.contains("rotate(90"), "{r1:?}");
    let (c1, c3) = (texts(&s, "c1")[0].clone(), texts(&s, "c3")[0].clone());
    assert!(r1[0].0 > c3.0, "row strip right of the last column");
    assert!(c1.1 < r1[0].1, "column strip above the panels");
}

#[test]
fn switch_moves_strips_below_and_left() {
    let s = svg(plot()
        .facet_grid(Some("r"), Some("c"))
        .facet_switch(FacetSwitch::Both));
    let base = svg(plot().facet_grid(Some("r"), Some("c")));
    let r1 = texts(&s, "r1")[0].clone();
    assert!(r1.2.contains("rotate(270"), "{r1:?}");
    assert!(r1.0 < texts(&base, "r1")[0].0 && r1.0 < 100.0, "{r1:?}");
    assert!(texts(&s, "c1")[0].1 > texts(&base, "c1")[0].1 + 200.0);
    // The x axis labels sit below the switched strips.
    let strip_y = texts(&s, "c1")[0].1;
    assert!(
        texts(&s, "20").iter().all(|t| t.1 > strip_y),
        "{:?}",
        texts(&s, "20")
    );
}

#[test]
fn wrap_strip_positions() {
    let top = svg(plot().facet_wrap("c", Some(3)));
    for (side, check) in [
        (StripPosition::Bottom, "bottom"),
        (StripPosition::Left, "left"),
        (StripPosition::Right, "right"),
    ] {
        let s = svg(plot().facet_wrap("c", Some(3)).facet_strip_position(side));
        let (t, b) = (texts(&top, "c2")[0].clone(), texts(&s, "c2")[0].clone());
        match check {
            "bottom" => assert!(b.1 > t.1 + 150.0 && b.2.is_empty(), "{b:?}"),
            "left" => assert!(b.2.contains("rotate(270") && b.0 < t.0, "{b:?}"),
            _ => assert!(b.2.contains("rotate(90") && b.0 > t.0, "{b:?}"),
        }
        assert_eq!(texts(&s, "c2").len(), 1);
    }
}
