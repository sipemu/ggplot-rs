//! Fixed-domain gradient/colour-continuous scales (anofox-visualization
//! PR #324): `with_limits(lo, hi)` on `ScaleColorContinuous`,
//! `ScaleColorGradient2` and `ScaleColorGradientN` fixes the *colour*
//! domain (not just the legend) to `[lo, hi]`, with ggplot2's out-of-bounds
//! semantics: `Oob::Censor` (default) drops the colour for an out-of-range
//! value, `Oob::Squish` (opt-in via `with_oob`) clamps it to the nearest
//! limit instead.

use ggplot_rs::aes::Aesthetic;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::Scale;

fn c(v: f64) -> Value {
    Value::Float(v)
}

#[test]
fn without_limits_the_domain_still_trains_from_data() {
    let mut s = ScaleColorContinuous::new(Aesthetic::Fill)
        .with_colors(RGBAColor::new(0, 0, 255), RGBAColor::new(255, 0, 0));
    s.train(&[c(-5.0), c(5.0)]);
    assert_eq!(s.domain(), Some((-5.0, 5.0)));
    assert_eq!(s.map_to_color(&c(-5.0)), Some((0, 0, 255)));
    assert_eq!(s.map_to_color(&c(5.0)), Some((255, 0, 0)));
}

#[test]
fn with_limits_the_domain_is_fixed_and_training_does_not_widen_it() {
    let mut s = ScaleColorContinuous::new(Aesthetic::Fill)
        .with_colors(RGBAColor::new(0, 0, 255), RGBAColor::new(255, 0, 0))
        .with_limits(-1.0, 1.0);
    // Data far outside [-1, 1] must not widen the fixed domain.
    s.train(&[c(-100.0), c(100.0)]);
    assert_eq!(s.domain(), Some((-1.0, 1.0)));
    // The *colours* reflect the fixed domain too, not just the legend.
    assert_eq!(s.map_to_color(&c(-1.0)), Some((0, 0, 255)));
    assert_eq!(s.map_to_color(&c(0.0)), Some((127, 0, 127)));
    assert_eq!(s.map_to_color(&c(1.0)), Some((255, 0, 0)));
}

#[test]
fn oob_censor_is_the_default_and_drops_out_of_range_colours() {
    let s = ScaleColorContinuous::new(Aesthetic::Fill)
        .with_colors(RGBAColor::new(0, 0, 255), RGBAColor::new(255, 0, 0))
        .with_limits(-1.0, 1.0);
    assert_eq!(s.map_to_color(&c(0.5)), Some((191, 0, 63)));
    assert_eq!(s.map_to_color(&c(1.5)), None, "oob: no colour by default");
    assert_eq!(s.map_to_color(&c(-1.5)), None);
}

#[test]
fn oob_squish_clamps_to_the_nearest_limit() {
    let s = ScaleColorContinuous::new(Aesthetic::Fill)
        .with_colors(RGBAColor::new(0, 0, 255), RGBAColor::new(255, 0, 0))
        .with_limits(-1.0, 1.0)
        .with_oob(Oob::Squish);
    assert_eq!(s.map_to_color(&c(1.5)), Some((255, 0, 0)), "squished to hi");
    assert_eq!(
        s.map_to_color(&c(-1.5)),
        Some((0, 0, 255)),
        "squished to lo"
    );
    assert_eq!(s.map_to_color(&c(50.0)), s.map_to_color(&c(1.0)));
}

#[test]
fn gradient2_respects_limits_and_oob_around_the_midpoint() {
    let s = ScaleColorGradient2::new(Aesthetic::Fill)
        .with_colors(
            RGBAColor::new(0, 0, 255),
            RGBAColor::new(255, 255, 255),
            RGBAColor::new(255, 0, 0),
        )
        .with_midpoint(0.0)
        .with_limits(-1.0, 1.0);
    assert_eq!(s.map_to_color(&c(0.0)), Some((255, 255, 255)));
    assert_eq!(s.map_to_color(&c(-1.0)), Some((0, 0, 255)));
    assert_eq!(s.map_to_color(&c(1.0)), Some((255, 0, 0)));
    assert_eq!(s.map_to_color(&c(2.0)), None, "censored by default");

    let squished = s.with_oob(Oob::Squish);
    assert_eq!(squished.map_to_color(&c(2.0)), Some((255, 0, 0)));
    assert_eq!(squished.map_to_color(&c(-2.0)), Some((0, 0, 255)));
}

#[test]
fn gradientn_respects_limits_and_oob() {
    let stops = vec![
        (0.0, RGBAColor::new(0, 0, 255)),
        (0.5, RGBAColor::new(255, 255, 255)),
        (1.0, RGBAColor::new(255, 0, 0)),
    ];
    let s = ScaleColorGradientN::new(Aesthetic::Fill, stops).with_limits(0.0, 10.0);
    assert_eq!(s.map_to_color(&c(0.0)), Some((0, 0, 255)));
    assert_eq!(s.map_to_color(&c(10.0)), Some((255, 0, 0)));
    assert_eq!(s.map_to_color(&c(20.0)), None, "censored by default");

    let squished = s.with_oob(Oob::Squish);
    assert_eq!(squished.map_to_color(&c(20.0)), Some((255, 0, 0)));
    assert_eq!(squished.map_to_color(&c(-20.0)), Some((0, 0, 255)));
}

#[test]
fn a_fixed_domain_tile_plot_colours_a_correlation_matrix_correctly() {
    use ggplot_rs::data::Value;
    let data: Vec<(String, Vec<Value>)> = vec![
        (
            "x".into(),
            vec![Value::Str("a".into()), Value::Str("b".into())],
        ),
        (
            "y".into(),
            vec![Value::Str("a".into()), Value::Str("a".into())],
        ),
        ("v".into(), vec![c(1.0), c(-1.0)]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_tile()
        .scale_fill(
            ScaleColorContinuous::new(Aesthetic::Fill)
                .with_colors(RGBAColor::new(0, 0, 255), RGBAColor::new(255, 0, 0))
                .with_limits(-1.0, 1.0),
        )
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("fill=\"#FF0000\""), "{svg}");
    assert!(svg.contains("fill=\"#0000FF\""), "{svg}");
}
