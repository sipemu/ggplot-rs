//! sipemu/ggplot-rs#142: `element_geom` (ink/paper) coverage for every geom
//! that wasn't already covered in 0.21.0 (point, line, path, text, label,
//! boxplot, smooth — see `geom_ink_paper.rs`).
//!
//! Each geom gets one test: under `Theme::default().with_ink_paper(white,
//! black)`, (1) the geom's own hard-coded default colour (black /
//! `(50, 50, 50)` / white fill) is replaced, and (2) a `geom_*_with(...)`
//! layer whose colour is set *explicitly* to that same literal value is left
//! untouched — the `explicit_style` gate in `build.rs` skips
//! `apply_element_geom` for the whole layer, not a per-field value check
//! (PR#141 finding 4).
//!
//! `GeomSf` (feature `sf`) is covered separately in
//! `geom_ink_paper_sf.rs`, since it needs the `sf` feature.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

const WHITE: (u8, u8, u8) = (255, 255, 255);
const BLACK: (u8, u8, u8) = (0, 0, 0);
const GREY: (u8, u8, u8) = (50, 50, 50);

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn ink_theme() -> Theme {
    Theme::default().with_ink_paper(WHITE, BLACK)
}

/// Plain `x`/`y` pair data, enough to satisfy most geoms' required aes.
fn xy() -> Vec<(String, Vec<Value>)> {
    frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0, 4.0])),
        ("y", floats(&[1.0, 2.0, 1.5, 3.0])),
    ])
}

/// A small point cloud, for the 2-D-density/binning stats (hex, bin2d,
/// density2d) that need enough rows to produce a non-empty grid.
fn cloud() -> Vec<(String, Vec<Value>)> {
    let mut x = Vec::new();
    let mut y = Vec::new();
    for i in 0..24 {
        x.push(Value::Float((i as f64 * 0.37).sin() * 3.0 + (i % 5) as f64));
        y.push(Value::Float((i as f64 * 0.53).cos() * 3.0 + (i % 4) as f64));
    }
    frame(vec![("x", x), ("y", y)])
}

/// A small `(x, y, z)` grid for `stat_contour`, which reads `z` by column
/// name directly (not through the `Aes` mapping).
fn surface() -> Vec<(String, Vec<Value>)> {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut z = Vec::new();
    for i in 0..8 {
        for j in 0..8 {
            let xv = i as f64 - 4.0;
            let yv = j as f64 - 4.0;
            x.push(Value::Float(xv));
            y.push(Value::Float(yv));
            z.push(Value::Float((-(xv * xv + yv * yv) / 10.0).exp()));
        }
    }
    frame(vec![("x", x), ("y", y), ("z", z)])
}

/// One category's worth of `y` values, for violin/density/area/crossbar.
fn cat_y() -> Vec<(String, Vec<Value>)> {
    frame(vec![
        ("x", strs(&["a", "a", "a", "a"])),
        ("y", floats(&[1.0, 2.0, 3.0, 4.0])),
    ])
}

#[test]
fn ink_recolours_geom_segment_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("xend", floats(&[2.0])),
        ("yend", floats(&[2.0])),
    ]);
    let aes = || Aes::new().x("x").y("y").xend("xend").yend("yend");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_segment()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");
    assert!(!svg.contains("#000000"), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_segment_with(GeomSegment {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_curve_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("xend", floats(&[2.0])),
        ("yend", floats(&[2.0])),
    ]);
    let aes = || Aes::new().x("x").y("y").xend("xend").yend("yend");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_curve()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_curve_with(GeomCurve {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_hline_and_an_explicit_colour_is_unaffected() {
    let svg = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_hline(2.0)
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_hline_with(GeomHline {
            color: BLACK,
            ..GeomHline::new(2.0)
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_vline_and_an_explicit_colour_is_unaffected() {
    let svg = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_vline(2.0)
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_vline_with(GeomVline {
            color: BLACK,
            ..GeomVline::new(2.0)
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_abline_and_an_explicit_colour_is_unaffected() {
    let svg = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_abline(1.0, 0.0)
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_abline_with(GeomAbline {
            color: BLACK,
            ..GeomAbline::new(1.0, 0.0)
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_errorbar_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("ymin", floats(&[0.5, 1.0])),
        ("ymax", floats(&[1.5, 2.5])),
    ]);
    let aes = || Aes::new().x("x").ymin("ymin").ymax("ymax");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_errorbar()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_errorbar_with(GeomErrorbar {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_errorbarh_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("y", floats(&[1.0, 2.0])),
        ("xmin", floats(&[0.5, 1.0])),
        ("xmax", floats(&[1.5, 2.5])),
    ]);
    let aes = || Aes::new().y("y").xmin("xmin").xmax("xmax");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_errorbarh()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_errorbarh_with(GeomErrorbarh {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_linerange_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("ymin", floats(&[0.5, 1.0])),
        ("ymax", floats(&[1.5, 2.5])),
    ]);
    let aes = || Aes::new().x("x").ymin("ymin").ymax("ymax");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_linerange()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_linerange_with(GeomLinerange {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_pointrange_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[1.0, 1.5])),
        ("ymin", floats(&[0.5, 1.0])),
        ("ymax", floats(&[1.5, 2.5])),
    ]);
    let aes = || Aes::new().x("x").y("y").ymin("ymin").ymax("ymax");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_pointrange()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("#FFFFFF"), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_pointrange_with(GeomPointrange {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("#000000"), "{explicit}");
}

#[test]
fn ink_recolours_geom_step_and_an_explicit_colour_is_unaffected() {
    let svg = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_step()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_step_with(GeomStep {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_spoke_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[1.0, 2.0])),
        ("angle", floats(&[0.3, 1.0])),
        ("radius", floats(&[1.0, 1.5])),
    ]);
    let aes = || Aes::new().x("x").y("y").angle("angle").radius("radius");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_spoke()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_spoke_with(GeomSpoke {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_rug_and_an_explicit_colour_is_unaffected() {
    let svg = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_rug()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_rug_with(GeomRug {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

// `GeomDotplot::color` is intentionally not covered: it's dead config (only
// `fill` is ever painted by `draw`, see `src/geom/dotplot.rs`), not a
// hard-coded visible default, so there's nothing for `ink` to recolour.

#[test]
fn ink_recolours_geom_bracket_default() {
    // `geom_bracket`/`geom_bracket_many` have no `_with`-style explicit-style
    // guard of their own (both go through `add_geom`, not `add_geom_with` —
    // a pre-existing gap shared with the brand-colour application, outside
    // #142's scope), so only the "ink recolours the default" half of the
    // usual pair applies here.
    let boxplot_data = frame(vec![
        ("x", strs(&["a", "a", "b", "b"])),
        ("y", floats(&[1.0, 2.0, 3.0, 4.0])),
    ]);

    let svg = GGPlot::new(boxplot_data)
        .aes(Aes::new().x("x").y("y"))
        .geom_boxplot()
        .geom_bracket("a", "b", 5.0, "*")
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");
}

#[test]
fn ink_recolours_geom_jitter_and_an_explicit_colour_is_unaffected() {
    let svg = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_jitter()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("#FFFFFF"), "{svg}");

    let explicit = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_jitter_with(GeomJitter {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("#000000"), "{explicit}");
}

#[test]
fn ink_recolours_geom_qq_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![(
        "y",
        floats(&[-1.5, -0.8, -0.2, 0.1, 0.4, 0.9, 1.3, 2.0]),
    )]);

    let svg = GGPlot::new(data.clone())
        .aes(Aes::new().y("y"))
        .geom_qq()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("#FFFFFF"), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(Aes::new().y("y"))
        .geom_qq_with(GeomQQ {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("#000000"), "{explicit}");
}

#[test]
fn ink_recolours_geom_censor_marks_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[1.0, 0.8, 0.6])),
        ("n_censor", floats(&[0.0, 1.0, 0.0])),
    ]);
    let aes = || Aes::new().x("x").y("y");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_censor_marks("n_censor")
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("#FFFFFF"), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_censor_marks_with(
            GeomCensorMarks {
                color: BLACK,
                ..Default::default()
            },
            "n_censor",
        )
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("#000000"), "{explicit}");
}

#[test]
fn ink_recolours_geom_freqpoly_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![(
        "x",
        floats(&(0..30).map(|i| i as f64 * 0.3).collect::<Vec<_>>()),
    )]);

    let svg = GGPlot::new(data.clone())
        .aes(Aes::new().x("x"))
        .geom_freqpoly()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_freqpoly_with(GeomFreqpoly {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_text_repel_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[1.0, 2.0, 1.5])),
        ("label", strs(&["a", "b", "c"])),
    ]);
    let aes = || Aes::new().x("x").y("y").label("label");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_text_repel()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("fill=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_text_repel_with(ggplot_rs::geom::repel::GeomTextRepel {
            color: BLACK,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("fill=\"#000000\""), "{explicit}");
}

#[test]
fn ink_and_paper_recolour_geom_crossbar_and_explicit_colours_are_unaffected() {
    let data = cat_y();
    let aes = || Aes::new().x("x").y("y").ymin("y").ymax("y");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_crossbar()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "outline: {svg}");
    assert!(svg.contains("fill=\"#000000\""), "fill: {svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_crossbar_with(GeomCrossbar {
            color: BLACK,
            fill: WHITE,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#000000\""), "{explicit}");
    assert!(explicit.contains("fill=\"#FFFFFF\""), "{explicit}");
}

#[test]
fn ink_and_paper_recolour_geom_label_repel_and_explicit_colours_are_unaffected() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[1.0, 2.0, 1.5])),
        ("label", strs(&["a", "b", "c"])),
    ]);
    let aes = || Aes::new().x("x").y("y").label("label");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_label_repel()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("fill=\"#FFFFFF\""), "text: {svg}");
    let rects: Vec<&str> = svg
        .match_indices("<rect ")
        .map(|(i, _)| {
            let rest = &svg[i..];
            &rest[..rest.find('>').map_or(rest.len(), |e| e + 1)]
        })
        .collect();
    assert!(
        rects.iter().any(|r| r.contains("fill=\"#000000\"")),
        "label background should be paper: {svg}"
    );

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_label_repel_with(ggplot_rs::geom::repel::GeomLabelRepel {
            color: BLACK,
            fill: WHITE,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("fill=\"#000000\""), "{explicit}");
    assert!(explicit.contains("fill=\"#FFFFFF\""), "{explicit}");
}

// --- outlines (grey `(50, 50, 50)` → ink) ----------------------------------

#[test]
fn ink_recolours_geom_rect_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("xmin", floats(&[0.0])),
        ("xmax", floats(&[1.0])),
        ("ymin", floats(&[0.0])),
        ("ymax", floats(&[1.0])),
    ]);
    let aes = || {
        Aes::new()
            .xmin("xmin")
            .xmax("xmax")
            .ymin("ymin")
            .ymax("ymax")
    };

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_rect()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");
    assert!(!svg.contains("#323232"), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_rect_with(GeomRect {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_tile_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[1.0]))]);
    let aes = || Aes::new().x("x").y("y");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_tile()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_tile_with(GeomTile {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_bar_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![("x", strs(&["a", "a", "b", "b", "b"]))]);
    let aes = || Aes::new().x("x");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_bar()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_bar_with(GeomBar {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_col_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![("x", strs(&["a", "b"])), ("y", floats(&[2.0, 3.0]))]);
    let aes = || Aes::new().x("x").y("y");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_col()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_col_with(GeomCol {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_polygon_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("x", floats(&[0.0, 1.0, 1.0, 0.0])),
        ("y", floats(&[0.0, 0.0, 1.0, 1.0])),
    ]);
    let aes = || Aes::new().x("x").y("y");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_polygon()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_polygon_with(GeomPolygon {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_violin_and_an_explicit_colour_is_unaffected() {
    let data = cat_y();
    let aes = || Aes::new().x("x").y("y");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_violin()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_violin_with(GeomViolin {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_density_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![(
        "x",
        floats(
            &(0..30)
                .map(|i| (i as f64 * 0.31).sin() * 2.0)
                .collect::<Vec<_>>(),
        ),
    )]);
    let aes = || Aes::new().x("x");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_density()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_density_with(GeomDensity {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_area_and_an_explicit_colour_is_unaffected() {
    let svg = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_area()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(xy())
        .aes(Aes::new().x("x").y("y"))
        .geom_area_with(GeomArea {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_count_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![
        ("x", floats(&[1.0, 1.0, 2.0, 2.0, 2.0])),
        ("y", floats(&[1.0, 1.0, 2.0, 2.0, 2.0])),
    ]);
    let aes = || Aes::new().x("x").y("y");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_count()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("#FFFFFF"), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_count_with(GeomCount {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("#323232"), "{explicit}");
}

#[test]
fn ink_recolours_geom_hex_and_an_explicit_colour_is_unaffected() {
    let svg = GGPlot::new(cloud())
        .aes(Aes::new().x("x").y("y"))
        .geom_hex()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(cloud())
        .aes(Aes::new().x("x").y("y"))
        .geom_hex_with(GeomHex {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_contour_and_an_explicit_colour_is_unaffected() {
    let svg = GGPlot::new(surface())
        .aes(Aes::new().x("x").y("y"))
        .geom_contour()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(surface())
        .aes(Aes::new().x("x").y("y"))
        .geom_contour_with(GeomContour {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_histogram_and_an_explicit_colour_is_unaffected() {
    let data = frame(vec![(
        "x",
        floats(&(0..30).map(|i| i as f64 * 0.2).collect::<Vec<_>>()),
    )]);
    let aes = || Aes::new().x("x");

    let svg = GGPlot::new(data.clone())
        .aes(aes())
        .geom_histogram()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(data)
        .aes(aes())
        .geom_histogram_with(GeomHistogram {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_density2d_and_an_explicit_colour_is_unaffected() {
    let svg = GGPlot::new(cloud())
        .aes(Aes::new().x("x").y("y"))
        .geom_density2d()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(cloud())
        .aes(Aes::new().x("x").y("y"))
        .geom_density2d_with(GeomDensity2d {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn ink_recolours_geom_bin2d_and_an_explicit_colour_is_unaffected() {
    let svg = GGPlot::new(cloud())
        .aes(Aes::new().x("x").y("y"))
        .geom_bin2d()
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");

    let explicit = GGPlot::new(cloud())
        .aes(Aes::new().x("x").y("y"))
        .geom_bin2d_with(GeomBin2d {
            color: GREY,
            ..Default::default()
        })
        .theme(ink_theme())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(explicit.contains("stroke=\"#323232\""), "{explicit}");
}

#[test]
fn default_theme_no_ink_paper_set_is_byte_identical_for_the_newly_covered_geoms() {
    // B2-7's headline guarantee extended to the geoms this PR adds coverage
    // for: `ElementGeom::default()` (nothing set) must not change a single
    // byte of output.
    let rect_data = frame(vec![
        ("xmin", floats(&[0.0])),
        ("xmax", floats(&[1.0])),
        ("ymin", floats(&[0.0])),
        ("ymax", floats(&[1.0])),
    ]);
    let plain = GGPlot::new(rect_data.clone())
        .aes(
            Aes::new()
                .xmin("xmin")
                .xmax("xmax")
                .ymin("ymin")
                .ymax("ymax"),
        )
        .geom_rect()
        .geom_hline(0.5)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let with_default_theme = GGPlot::new(rect_data)
        .aes(
            Aes::new()
                .xmin("xmin")
                .xmax("xmax")
                .ymin("ymin")
                .ymax("ymax"),
        )
        .geom_rect()
        .geom_hline(0.5)
        .theme(Theme::default())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(plain, with_default_theme);
}
