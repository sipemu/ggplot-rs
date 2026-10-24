//! #236: a mapped `size` legend drew every key as the same small circle —
//! `draw_legend_swatch`'s `Aesthetic::Size` arm clamped each key's radius
//! to half the *fixed* default swatch size (`theme.legend_key_width / 2`),
//! so any break mapping to a larger radius got clipped down to that same
//! ceiling. ggplot2 draws each key at its own break's mapped size and
//! grows the key box (and the legend band reserved for it) to fit the
//! largest one — `size_key_diameter` computes that largest diameter, fed
//! into the key box size everywhere it matters: the two per-row draw
//! paths, `estimate_legend_box` (#225's `Inside` justification) and
//! `PlotLayout`'s own `legend_size`/`legend_band` reservation.
#![cfg(feature = "sf")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::prelude::*;

fn sf_points_with_size() -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        vec![
            Value::Str("POINT (0 0)".into()),
            Value::Str("POINT (1 1)".into()),
            Value::Str("POINT (2 2)".into()),
        ],
    );
    df.add_column(
        "sz".into(),
        vec![
            Value::Float(100.0),
            Value::Float(200.0),
            Value::Float(300.0),
        ],
    );
    df
}

/// `(diameter, y)` for every `<rect width="D" height="D" .../>` where
/// width == height (a square legend key box — the panel's own marks are
/// `<circle>`s, not squares, so this only ever matches legend keys).
fn key_box_diameters(svg: &str) -> Vec<f64> {
    let mut out = Vec::new();
    for (start, _) in svg.match_indices("<rect ") {
        let end = svg[start..].find("/>").unwrap() + start;
        let tag = &svg[start..end];
        let w_start = match tag.find("width=\"") {
            Some(i) => i + 7,
            None => continue,
        };
        let w_end = tag[w_start..].find('"').unwrap() + w_start;
        let h_start = match tag.find("height=\"") {
            Some(i) => i + 8,
            None => continue,
        };
        let h_end = tag[h_start..].find('"').unwrap() + h_start;
        let w: f64 = tag[w_start..w_end].parse().unwrap();
        let h: f64 = tag[h_start..h_end].parse().unwrap();
        if (w - h).abs() < 1e-6 {
            out.push(w);
        }
    }
    out
}

/// Every legend key circle's radius — a `<circle>` with no `class`
/// attribute (panel marks always carry `class="gg-mark gg-point"`).
fn legend_key_radii(svg: &str) -> Vec<f64> {
    let mut out = Vec::new();
    for (start, _) in svg.match_indices("<circle ") {
        let end = svg[start..].find(['/', '>']).unwrap() + start;
        let tag = &svg[start..end];
        if tag.contains("class=") {
            continue; // a panel mark
        }
        let r_start = tag.find("r=\"").unwrap() + 3;
        let r_end = tag[r_start..].find('"').unwrap() + r_start;
        out.push(tag[r_start..r_end].parse().unwrap());
    }
    out
}

#[test]
fn size_legend_keys_are_monotone_and_not_all_identical() {
    let svg = GGPlot::new(sf_points_with_size())
        .aes(Aes::new().size("sz"))
        .geom_sf()
        .scale_size_area(10.0)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let radii = legend_key_radii(&svg);
    assert!(radii.len() >= 3, "{radii:?} in {svg}");
    // Strictly increasing — #236's bug clamped every key past a certain
    // size to the exact same radius.
    for pair in radii.windows(2) {
        assert!(
            pair[1] > pair[0] + 1e-6,
            "key radii must be strictly increasing: {radii:?}"
        );
    }
}

#[test]
fn the_largest_size_key_is_not_clamped_to_the_default_swatch_radius() {
    // The old bug: `size.min(swatch_size / 2.0)` with a *fixed*
    // `swatch_size` (the theme's default `legend_key_width`, 12px) capped
    // every key at radius 6. `scale_size_area(10.0)` asks for up to
    // radius 10 — well past that old ceiling.
    let svg = GGPlot::new(sf_points_with_size())
        .aes(Aes::new().size("sz"))
        .geom_sf()
        .scale_size_area(10.0)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let radii = legend_key_radii(&svg);
    let max_r = radii.iter().cloned().fold(0.0_f64, f64::max);
    const OLD_CEILING: f64 = 6.0;
    assert!(
        max_r > OLD_CEILING + 1.0,
        "largest key radius {max_r} should exceed the old fixed ceiling of {OLD_CEILING}: {radii:?}"
    );
}

#[test]
fn the_key_box_grows_to_fit_the_largest_key() {
    let svg = GGPlot::new(sf_points_with_size())
        .aes(Aes::new().size("sz"))
        .geom_sf()
        .scale_size_area(10.0)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let radii = legend_key_radii(&svg);
    let max_r = radii.iter().cloned().fold(0.0_f64, f64::max);
    let diameters = key_box_diameters(&svg);
    assert!(!diameters.is_empty(), "{svg}");
    let max_box = diameters.iter().cloned().fold(0.0_f64, f64::max);
    assert!(
        max_box >= 2.0 * max_r - 1e-6,
        "key box ({max_box}) must be at least as wide as the largest key's diameter ({})",
        2.0 * max_r
    );
    // And it must have actually grown past the theme's plain default
    // (12px) — otherwise this would just be coincidentally big enough.
    assert!(max_box > 12.0, "{diameters:?}");
}

#[test]
fn a_size_legend_whose_keys_already_fit_is_byte_identical() {
    // Small, in-range sizes (the default scale, no scale_size_area) never
    // needed the old clamp in the first place — #236's fix must not
    // change this common case's output at all.
    let data: Vec<(String, Vec<Value>)> = vec![
        ("x".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("sz".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ];
    let plot = || {
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y").size("sz"))
            .geom_point()
    };
    let a = plot().render_svg_native_with_size(300, 200).unwrap();
    let b = plot().render_svg_native_with_size(300, 200).unwrap();
    assert_eq!(a, b);
    let diameters = key_box_diameters(&a);
    for d in diameters {
        assert!(
            (d - 12.0).abs() < 1e-6,
            "an unscaled size legend's key box should stay the theme default (12px): {d}"
        );
    }
}
