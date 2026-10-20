//! sipemu/ggplot-rs#132: the discrete-style legend for a continuous `Size`
//! or `Alpha` aesthetic used to collapse every key to the scale's minimum,
//! because `draw_discrete_legend_at` only had each break's rendered *label*
//! (a `Value::Str`), not its original data value, and
//! `map_to_size`/`map_to_alpha` can't parse a label string back to a number.
//! 0.20 fixed the same bug for `Linewidth` with `Scale::map_fraction_to_size`
//! (see `linewidth_arrow.rs`); this is the equivalent fix for `Size` (reusing
//! `map_fraction_to_size`) and `Alpha` (the new `map_fraction_to_alpha`).

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::Scale;

fn training_values() -> Vec<f64> {
    vec![0.0, 2.5, 5.0, 7.5, 10.0]
}

fn breaks_data(aes_col: &str) -> Vec<(String, Vec<Value>)> {
    let vs = training_values();
    vec![
        ("x".into(), vs.iter().map(|_| Value::Float(0.0)).collect()),
        (
            "y".into(),
            (0..vs.len()).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            aes_col.into(),
            vs.iter().map(|&v| Value::Float(v)).collect(),
        ),
    ]
}

/// The markup added once a plot gets a legend — same technique as
/// `linewidth_arrow.rs`'s `legend_has_n_keys_with_increasing_widths`: render
/// with and without the legend and diff from the first byte that differs.
fn legend_markup(with_legend: &str, without_legend: &str) -> String {
    let common = with_legend
        .bytes()
        .zip(without_legend.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    assert!(
        with_legend.len() > without_legend.len(),
        "expected the legend to add markup"
    );
    with_legend[common..].to_string()
}

fn circle_radii(svg: &str) -> Vec<f64> {
    svg.split("<circle ")
        .skip(1)
        .filter_map(|s| s.split("r=\"").nth(1))
        .filter_map(|s| s.split('"').next())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect()
}

/// `fill-opacity="…"` values from the `<rect …>` legend-key *swatches*
/// specifically (`fill="#323232"`, the legend code's hard-coded
/// `theme.geom.ink.unwrap_or((50, 50, 50))` for an `Alpha` key) — not the
/// legend-key *background* rect drawn just before each swatch (always fully
/// opaque, so it'd otherwise pollute every other entry). No `fill-opacity`
/// attribute at all means fully opaque (`opacity_attr` omits it when alpha
/// rounds to `1.000`).
fn alpha_swatch_opacities(svg: &str) -> Vec<f64> {
    svg.split("<rect ")
        .skip(1)
        .filter_map(|s| {
            let tag_end = s.find('>').unwrap_or(s.len());
            let tag = &s[..tag_end];
            if !tag.contains("fill=\"#323232\"") {
                return None;
            }
            Some(match tag.split("fill-opacity=\"").nth(1) {
                Some(rest) => rest.split('"').next().unwrap().parse::<f64>().unwrap(),
                None => 1.0,
            })
        })
        .collect()
}

#[test]
fn size_legend_keys_scale_with_their_break_not_all_the_minimum() {
    // Kept under `legend_key_width / 2` (6.0 at the default theme) so the
    // legend swatch's own radius clamp never kicks in and masks the fix.
    let range = (1.0, 5.5);
    let plot = || {
        GGPlot::new(breaks_data("v"))
            .aes(Aes::new().x("x").y("y").size("v"))
            .geom_point()
            .scale_size(ScaleSizeContinuous::new().with_range(range.0, range.1))
    };
    let with_legend = plot().render_svg_native_with_size(400, 400).unwrap();
    let without_legend = plot()
        .show_legend(false)
        .render_svg_native_with_size(400, 400)
        .unwrap();
    let markup = legend_markup(&with_legend, &without_legend);
    let radii = circle_radii(&markup);

    // The expected radius per break, computed the same way the fix does:
    // the break's own `[0, 1]` fraction straight into `map_fraction_to_size`
    // — not a hardcoded break count, since `breaks()`'s "nice" step choice
    // doesn't have to match the number of trained values.
    let mut scale = ScaleSizeContinuous::new().with_range(range.0, range.1);
    scale.train(
        &training_values()
            .iter()
            .map(|&v| Value::Float(v))
            .collect::<Vec<_>>(),
    );
    let expected: Vec<f64> = scale
        .breaks()
        .iter()
        .map(|(fraction, _)| scale.map_fraction_to_size(*fraction).unwrap())
        .collect();

    assert_eq!(
        radii.len(),
        expected.len(),
        "one legend key per break: got {radii:?}, expected {expected:?}"
    );
    for (got, want) in radii.iter().zip(&expected) {
        assert!(
            (got - want).abs() < 0.01,
            "legend key radius {got} should match map_fraction_to_size's {want}: {radii:?} vs {expected:?}"
        );
    }

    // Before the fix every key reported `map_size(Value::Str(label))`,
    // `None` for `ScaleSizeContinuous`, and so fell back to the hard-coded
    // 3.0 swatch radius for every key — never varying with the break.
    for i in 1..radii.len() {
        assert!(
            radii[i] > radii[i - 1],
            "legend key radii should increase with the break value: {radii:?}"
        );
    }
}

#[test]
fn alpha_legend_keys_scale_with_their_break_not_all_the_minimum() {
    let range = (0.2, 0.9);
    let plot = || {
        GGPlot::new(breaks_data("v"))
            .aes(Aes::new().x("x").y("y").alpha("v"))
            .geom_point()
            .scale_alpha(ScaleAlphaContinuous::new().with_range(range.0, range.1))
    };
    let with_legend = plot().render_svg_native_with_size(400, 400).unwrap();
    let without_legend = plot()
        .show_legend(false)
        .render_svg_native_with_size(400, 400)
        .unwrap();
    let markup = legend_markup(&with_legend, &without_legend);
    let alphas = alpha_swatch_opacities(&markup);

    let mut scale = ScaleAlphaContinuous::new().with_range(range.0, range.1);
    scale.train(
        &training_values()
            .iter()
            .map(|&v| Value::Float(v))
            .collect::<Vec<_>>(),
    );
    let expected: Vec<f64> = scale
        .breaks()
        .iter()
        .map(|(fraction, _)| scale.map_fraction_to_alpha(*fraction).unwrap())
        .collect();

    assert_eq!(
        alphas.len(),
        expected.len(),
        "one legend key swatch per break: got {alphas:?}, expected {expected:?}"
    );
    for (got, want) in alphas.iter().zip(&expected) {
        assert!(
            (got - want).abs() < 0.001,
            "legend key opacity {got} should match map_fraction_to_alpha's {want}: {alphas:?} vs {expected:?}"
        );
    }

    // Before the fix every key reported `map_alpha(Value::Str(label))`,
    // `None` for `ScaleAlphaContinuous`, and so fell back to the hard-coded
    // 1.0 swatch alpha for every key — never varying with the break.
    for i in 1..alphas.len() {
        assert!(
            alphas[i] > alphas[i - 1] - 1e-9,
            "legend key opacity should not decrease with the break value: {alphas:?}"
        );
    }
    assert!(
        alphas[alphas.len() - 1] - alphas[0] > 0.3,
        "the first and last key should differ well beyond rounding: {alphas:?}"
    );
}

#[test]
fn scale_size_continuous_map_fraction_to_size_matches_the_configured_range() {
    let s = ScaleSizeContinuous::new().with_range(2.0, 20.0);
    assert_eq!(s.map_fraction_to_size(0.0), Some(2.0));
    assert!((s.map_fraction_to_size(1.0).unwrap() - 20.0).abs() < 1e-9);
    assert!((s.map_fraction_to_size(0.5).unwrap() - 11.0).abs() < 1e-9);
    // Out-of-[0,1] fractions are clamped, same as `ScaleLinewidthContinuous`.
    assert_eq!(s.map_fraction_to_size(-1.0), Some(2.0));
    assert!((s.map_fraction_to_size(2.0).unwrap() - 20.0).abs() < 1e-9);
}

#[test]
fn scale_alpha_continuous_map_fraction_to_alpha_matches_the_configured_range() {
    let s = ScaleAlphaContinuous::new().with_range(0.2, 0.9);
    assert_eq!(s.map_fraction_to_alpha(0.0), Some(0.2));
    assert!((s.map_fraction_to_alpha(1.0).unwrap() - 0.9).abs() < 1e-9);
    let mid = s.map_fraction_to_alpha(0.5).unwrap();
    assert!((mid - 0.55).abs() < 1e-9, "{mid}");
    assert_eq!(s.map_fraction_to_alpha(-1.0), Some(0.2));
    assert!((s.map_fraction_to_alpha(2.0).unwrap() - 0.9).abs() < 1e-9);
}
