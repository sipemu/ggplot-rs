//! sipemu/ggplot-rs#286: with a Bottom legend and a caption, the caption was
//! drawn at a fixed offset from the canvas bottom — ignoring the
//! `caption_area` the layout reserved — so it overlapped the legend labels
//! and its descenders reached the figure edge.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::theme::LegendPosition;

fn plot() -> GGPlot {
    base().theme_void().legend_position(LegendPosition::Bottom)
}

fn base() -> GGPlot {
    let n = 25;
    let d = vec![
        (
            "x".to_string(),
            (0..n)
                .map(|i| Value::Float((i % 5) as f64))
                .collect::<Vec<_>>(),
        ),
        (
            "y".to_string(),
            (0..n).map(|i| Value::Float((i / 5) as f64)).collect(),
        ),
        (
            "z".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
    ];
    GGPlot::new(d)
        .aes(Aes::new().x("x").y("y").fill("z"))
        .geom_tile()
        .caption("Hexagons 5 km across")
}

/// `(y, font-size)` of every `<text>`.
fn texts(svg: &str) -> Vec<(String, f64, f64)> {
    svg.split("<text")
        .skip(1)
        .map(|t| {
            let attr = |n: &str| -> f64 {
                t.split(&format!(" {n}=\""))
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap()
            };
            let body = t
                .split('>')
                .nth(1)
                .unwrap()
                .split('<')
                .next()
                .unwrap()
                .to_string();
            (body, attr("y"), attr("font-size"))
        })
        .collect()
}

#[test]
fn caption_sits_below_the_legend_and_inside_the_canvas() {
    check(plot(), 163, 306);
}

#[test]
fn other_themes_and_facets() {
    fn bottom(p: GGPlot) -> GGPlot {
        p.legend_position(LegendPosition::Bottom)
    }
    type Case = (&'static str, fn() -> GGPlot);
    let cases: [Case; 7] = [
        ("gray", || bottom(base())),
        ("minimal", || bottom(base().theme_minimal())),
        ("bw", || bottom(base().theme_bw())),
        ("classic", || bottom(base().theme_classic())),
        ("void-wide", || plot()),
        ("facet", || bottom(base().facet_wrap("y", None))),
        ("facet-void", || {
            bottom(base().facet_wrap("y", None).theme_void())
        }),
    ];
    for (name, p) in cases {
        for (w, h) in [(163, 306), (640, 480)] {
            eprintln!("{name} {w}x{h}");
            check(p(), w, h);
        }
    }
}

fn check(p: GGPlot, w: u32, h: u32) {
    let h = h as f64;
    let svg = p.render_svg_native_with_size(w, h as u32).unwrap();
    let t = texts(&svg);
    let (_, cy, cs) = t
        .iter()
        .find(|(b, ..)| b.starts_with("Hexagons"))
        .unwrap()
        .clone();
    // Descenders (~0.25 em) stay on the canvas.
    assert!(cy + 0.25 * cs <= h, "caption baseline {cy} too low: {t:?}");
    // Its cap height (~0.75 em) clears every legend label's descenders.
    for (b, y, s) in t
        .iter()
        .filter(|(b, y, _)| !b.starts_with("Hexagons") && *y > cy - 3.0 * cs)
    {
        assert!(
            y + 0.25 * s <= cy - 0.75 * cs,
            "legend label {b:?} at {y} overlaps the caption at {cy}: {t:?}"
        );
    }
}
