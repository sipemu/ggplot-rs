//! #404: grouped slot geoms (boxplot, violin, crossbar, dotplot) dodge within
//! their x slot — boxplot / violin by default, as in ggplot2 — instead of
//! drawing every group's element on top of each other.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let mut class = Vec::new();
    let mut drv = Vec::new();
    let mut hwy = Vec::new();
    for (c, base) in [("compact", 28.0), ("suv", 18.0)] {
        for (d, shift) in [("f", 2.0), ("4", -2.0)] {
            for k in 0..8 {
                class.push(Value::Str(c.into()));
                drv.push(Value::Str(d.into()));
                hwy.push(Value::Float(base + shift + (k as f64 - 3.5)));
            }
        }
    }
    vec![
        ("class".into(), class),
        ("drv".into(), drv),
        ("hwy".into(), hwy),
    ]
}

/// The `x` and `width` of every filled `<rect>` (boxes / crossbars), skipping
/// the panel background and legend keys by their size.
fn rects(svg: &str) -> Vec<(f64, f64)> {
    let attr = |tag: &str, name: &str| -> Option<f64> {
        let a = tag.find(&format!(" {name}=\""))? + name.len() + 3;
        tag[a..].split('"').next()?.parse().ok()
    };
    svg.split("<rect")
        .skip(1)
        .filter_map(|t| {
            let t = &t[..t.find('>').unwrap_or(t.len())];
            Some((attr(t, "x")?, attr(t, "width")?, attr(t, "height")?))
        })
        .filter(|&(_, w, h)| w > 5.0 && w < 200.0 && h > 5.0 && h < 380.0)
        .map(|(x, w, _)| (x, w))
        .collect()
}

fn assert_side_by_side(boxes: &[(f64, f64)], expect: usize) {
    assert!(
        boxes.len() >= expect,
        "expected {expect} boxes, got {boxes:?}"
    );
    let mut b: Vec<_> = boxes.to_vec();
    b.sort_by(|a, c| a.0.total_cmp(&c.0));
    for w in b.windows(2) {
        assert!(
            w[0].0 + w[0].1 <= w[1].0 + 0.5,
            "boxes overlap: {:?} and {:?}",
            w[0],
            w[1]
        );
    }
}

#[test]
fn grouped_boxplots_dodge_by_default() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("class").y("hwy").fill("drv"))
        .geom_boxplot()
        .legend_position(LegendPosition::None)
        .render_svg_native_with_size(600, 400)
        .unwrap();
    assert_side_by_side(&rects(&svg), 4);
}

#[test]
fn explicit_dodge2_and_preserve_single() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("class").y("hwy").color("drv"))
        .geom_boxplot()
        .position(PositionDodge2::new(0.1).with_preserve_single())
        .legend_position(LegendPosition::None)
        .render_svg_native_with_size(600, 400)
        .unwrap();
    assert_side_by_side(&rects(&svg), 4);
}

#[test]
fn single_group_box_keeps_full_width() {
    let grouped = GGPlot::new(data())
        .aes(Aes::new().x("class").y("hwy").fill("drv"))
        .geom_boxplot()
        .legend_position(LegendPosition::None)
        .render_svg_native_with_size(600, 400)
        .unwrap();
    let plain = GGPlot::new(data())
        .aes(Aes::new().x("class").y("hwy"))
        .geom_boxplot()
        .render_svg_native_with_size(600, 400)
        .unwrap();
    let (g, p) = (rects(&grouped), rects(&plain));
    assert_eq!(p.len(), 2, "{p:?}");
    // Two groups per slot → each box half as wide as an ungrouped one.
    assert!((g[0].1 * 2.0 - p[0].1).abs() < 1.0, "{g:?} vs {p:?}");
}

#[test]
fn grouped_violins_are_separate_and_dodged() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("class").y("hwy").fill("drv"))
        .geom_violin()
        .legend_position(LegendPosition::None)
        .render_svg_native_with_size(600, 400)
        .unwrap();
    // One polygon per (class, drv), not one merged polygon per class.
    assert_eq!(svg.matches("<polygon").count(), 4, "{svg}");
}

#[test]
fn crossbars_honour_position_dodge() {
    let mut cols = data();
    let n = cols[0].1.len();
    let num = |v: f64| vec![Value::Float(v); n];
    cols.push(("lo".into(), num(15.0)));
    cols.push(("hi".into(), num(25.0)));
    let svg = GGPlot::new(cols)
        .aes(
            Aes::new()
                .x("class")
                .y("hwy")
                .ymin("lo")
                .ymax("hi")
                .fill("drv"),
        )
        .geom_crossbar()
        .position(PositionDodge::new(0.9))
        .legend_position(LegendPosition::None)
        .render_svg_native_with_size(600, 400)
        .unwrap();
    let mut xs: Vec<i64> = rects(&svg).iter().map(|r| r.0.round() as i64).collect();
    xs.sort_unstable();
    xs.dedup();
    assert!(xs.len() >= 4, "crossbars not dodged: {xs:?}");
}
