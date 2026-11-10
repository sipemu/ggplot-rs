//! #484: a box plot under `coord_flip()` keeps its median and whiskers (each
//! line used to mix one transformed point's x with another's y, so they had
//! zero length there). #485: the interval geoms read a mapped tooltip.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

/// The lengths of every `<line>` / two-point `<polyline>` a boxplot draws.
fn segment_lengths(svg: &str) -> Vec<f64> {
    let mut out = Vec::new();
    for el in svg
        .split('<')
        .filter(|e| e.contains("data-geom=\"boxplot\""))
    {
        let attr = |n: &str| -> Option<f64> {
            let k = el.find(&format!(" {n}=\""))? + n.len() + 3;
            el[k..k + el[k..].find('"')?].parse().ok()
        };
        if el.starts_with("line ") {
            if let (Some(x1), Some(y1), Some(x2), Some(y2)) =
                (attr("x1"), attr("y1"), attr("x2"), attr("y2"))
            {
                out.push(((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt());
            }
        } else if el.starts_with("polyline ") {
            let k = el.find(" points=\"").unwrap() + 9;
            let pts: Vec<(f64, f64)> = el[k..k + el[k..].find('"').unwrap()]
                .split_whitespace()
                .filter_map(|p| {
                    let (a, b) = p.split_once(',')?;
                    Some((a.parse().ok()?, b.parse().ok()?))
                })
                .collect();
            if pts.len() == 2 {
                let (a, b) = (pts[0], pts[1]);
                out.push(((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt());
            }
        }
    }
    out
}

fn boxes(flip: bool) -> String {
    let y: Vec<f64> = (1..=40)
        .map(|i| (i % 10) as f64 + if i > 20 { 5.0 } else { 0.0 })
        .collect();
    let x: Vec<&str> = (1..=40).map(|i| if i > 20 { "B" } else { "A" }).collect();
    let p = GGPlot::new(vec![("x".into(), strs(&x)), ("y".into(), floats(&y))])
        .aes(Aes::new().x("x").y("y"))
        .geom_boxplot();
    let p = if flip { p.coord_flip() } else { p };
    p.render_svg_native_with_size(400, 300).unwrap()
}

#[test]
fn a_flipped_box_keeps_its_median_and_whiskers() {
    let up = segment_lengths(&boxes(false));
    let flipped = segment_lengths(&boxes(true));
    assert!(!up.is_empty(), "no boxplot lines found");
    assert_eq!(up.len(), flipped.len());
    assert!(up.iter().all(|&l| l > 1.0), "upright: {up:?}");
    assert!(flipped.iter().all(|&l| l > 1.0), "flipped: {flipped:?}");
}

#[test]
fn interval_geoms_read_a_mapped_tooltip() {
    let data = || {
        vec![
            ("x".to_string(), strs(&["A", "B"])),
            ("lo".to_string(), floats(&[7.0, 16.0])),
            ("hi".to_string(), floats(&[7.0, 16.0])),
            ("y".to_string(), floats(&[7.0, 16.0])),
            ("tip".to_string(), strs(&["A: Target 7", "B: Target 16"])),
        ]
    };
    let aes = || Aes::new().x("x").ymin("lo").ymax("hi").tooltip("tip");
    let svgs = [
        GGPlot::new(data()).aes(aes()).geom_errorbar(),
        GGPlot::new(data()).aes(aes()).geom_linerange(),
        GGPlot::new(data()).aes(aes().y("y")).geom_pointrange(),
    ]
    .map(|p| p.render_svg_native_with_size(300, 200).unwrap());
    for svg in &svgs {
        assert!(svg.contains("<title>A: Target 7</title>"), "{svg}");
        assert!(svg.contains("<title>B: Target 16</title>"));
    }
    // without one, the default interval text stays
    let plain = GGPlot::new(data())
        .aes(Aes::new().x("x").ymin("lo").ymax("hi"))
        .geom_errorbar()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(plain.contains("<title>A: 7 [7, 7]</title>"), "{plain}");
}
