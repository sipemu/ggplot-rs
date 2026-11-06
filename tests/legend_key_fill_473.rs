//! #473: a layer's legend key takes the layer's constant fill and alpha where
//! the legend doesn't map them (ggplot2's draw_key_polygon reads the layer's
//! params) — a translucent ribbon under a colour-mapped line no longer keys
//! as an opaque grey35 square.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let f = |v: &[f64]| v.iter().map(|&x| Value::Float(x)).collect::<Vec<_>>();
    let s = |v: &[&str]| {
        v.iter()
            .map(|x| Value::Str(x.to_string()))
            .collect::<Vec<_>>()
    };
    vec![
        ("x".into(), f(&[1., 2., 3., 1., 2., 3.])),
        ("y".into(), f(&[5., 6., 5., 3., 4., 3.])),
        ("lo".into(), f(&[4., 5., 4., 2., 3., 2.])),
        ("hi".into(), f(&[6., 7., 6., 4., 5., 4.])),
        ("cat".into(), s(&["a", "a", "a", "b", "b", "b"])),
    ]
}

fn key_rects(svg: &str) -> Vec<&str> {
    // Legend key glyph rects: 12 × 12, not the key background (stroke 0).
    svg.split("<rect ")
        .filter(|r| r.contains("width=\"12.00\" height=\"12.00\""))
        .filter(|r| !r.contains("stroke-width=\"0.00\""))
        .collect()
}

#[test]
fn ribbon_key_uses_the_ribbons_fill_and_alpha() {
    let (svg, _) = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("cat"))
        .geom_ribbon_with(GeomRibbon {
            fill: (50, 100, 200),
            alpha: 0.13,
            ..Default::default()
        })
        .layer_aes(Aes::new().x("x").ymin("lo").ymax("hi").color("cat"))
        .geom_line()
        .render_svg_native_with_warnings(640, 400)
        .unwrap();
    let keys = key_rects(&svg);
    assert_eq!(keys.len(), 2, "one ribbon key per level: {svg}");
    for k in keys {
        assert!(k.contains("fill=\"#3264C8\""), "{k}");
        assert!(k.contains("fill-opacity=\"0.130\""), "{k}");
        assert!(!k.contains("#595959"), "{k}");
    }
}

#[test]
fn a_mapped_fill_still_wins_and_keeps_the_layer_alpha() {
    let (svg, _) = GGPlot::new(data())
        .aes(Aes::new().x("x").ymin("lo").ymax("hi").fill("cat"))
        .geom_ribbon_with(GeomRibbon {
            fill: (50, 100, 200),
            alpha: 0.4,
            ..Default::default()
        })
        .render_svg_native_with_warnings(640, 400)
        .unwrap();
    let keys = key_rects(&svg);
    assert_eq!(keys.len(), 2, "{svg}");
    for k in &keys {
        assert!(!k.contains("fill=\"#3264C8\""), "the scale's colour: {k}");
        assert!(k.contains("fill-opacity=\"0.400\""), "{k}");
    }
}
