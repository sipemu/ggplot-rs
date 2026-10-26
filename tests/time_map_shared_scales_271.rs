//! #269/#270/#271: the three gaps anofox-visualization's time maps
//! (M2e/M2f) need from a `GGPlot` rendered once per animation frame with
//! shared scales — a stable "no data" legend key, a fixed size domain,
//! and a legend whose auto-wrap row count doesn't depend on which
//! frame's counts happen to be measured.
//!
//! The last part is proven two ways: a regression guard showing the
//! *un-reserved* legend really does move (so this isn't testing nothing),
//! and the fix showing `na_key(Always)` + `count_reserve` together keep
//! the panel rect identical.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::binned::{NaKey, ScaleFillBinned};
use ggplot_rs::scale::size::ScaleSizeContinuous;
use ggplot_rs::theme::LegendPosition;

fn tile_data(values: &[Option<f64>]) -> Vec<(String, Vec<Value>)> {
    let x: Vec<Value> = (0..values.len()).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = values.iter().map(|_| Value::Float(0.0)).collect();
    let v: Vec<Value> = values
        .iter()
        .map(|o| o.map(Value::Float).unwrap_or(Value::Na))
        .collect();
    vec![("x".into(), x), ("y".into(), y), ("v".into(), v)]
}

fn data_plot_rect(svg: &str) -> [f64; 4] {
    let start = svg.find("data-plot=\"").expect("data-plot attr") + "data-plot=\"".len();
    let rest = &svg[start..];
    let end = rest.find('"').expect("closing quote");
    let mut it = rest[..end]
        .split_whitespace()
        .map(|s| s.parse::<f64>().unwrap());
    [
        it.next().unwrap(),
        it.next().unwrap(),
        it.next().unwrap(),
        it.next().unwrap(),
    ]
}

fn binned_plot(values: &[Option<f64>], reserve: Option<usize>, na_always: bool) -> GGPlot {
    let mut scale = ScaleFillBinned::new(
        Aesthetic::Fill,
        vec![0.0, 10.0, 50.0, 100.0],
        vec![(240, 240, 200), (150, 170, 100), (30, 90, 30)],
    );
    if na_always {
        scale = scale.na_key(NaKey::Always);
    }
    if let Some(r) = reserve {
        scale = scale.count_reserve(r);
    }
    GGPlot::new(tile_data(values))
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_tile()
        .scale_fill(scale)
        .legend_position(LegendPosition::Top)
}

/// Regression guard: *without* the new knobs, a frame with no missing
/// values and small counts, versus a frame with missing values and much
/// larger counts, moves the panel at a width near the 390px budget —
/// exactly what sipemu/ggplot-rs#271 was filed against.
#[test]
fn without_na_key_and_count_reserve_the_panel_can_move() {
    let few = vec![Some(5.0), Some(25.0), Some(75.0)];
    let mut many = Vec::new();
    for _ in 0..1234 {
        many.push(Some(5.0));
        many.push(Some(25.0));
        many.push(Some(75.0));
        many.push(None); // introduces the "no data" key too
    }

    let r_few = data_plot_rect(
        &binned_plot(&few, None, false)
            .render_svg_native_with_size(330, 600)
            .unwrap(),
    );
    let r_many = data_plot_rect(
        &binned_plot(&many, None, false)
            .render_svg_native_with_size(330, 600)
            .unwrap(),
    );
    assert_ne!(
        r_few, r_many,
        "sanity check: the un-reserved legend was expected to move here"
    );
}

/// The fix: `na_key(Always)` (#269) keeps the "no data" key present in
/// both frames, and `count_reserve` (#271) reserves its measured width
/// for the widest expected count — the panel rect is now identical at
/// both a comfortable width and the narrow one where the regression
/// guard above showed movement.
#[test]
fn na_key_always_and_count_reserve_keep_the_panel_stable_across_frames() {
    let few = vec![Some(5.0), Some(25.0), Some(75.0)];
    let mut many = Vec::new();
    for _ in 0..1234 {
        many.push(Some(5.0));
        many.push(Some(25.0));
        many.push(Some(75.0));
        many.push(None);
    }
    // Reserve for the worst case across every frame: every class and the
    // na key could in principle reach the full 1234*3 row count.
    let reserve = Some(1234 * 3);

    for (w, h) in [(1200u32, 800u32), (330u32, 600u32)] {
        let svg_few = binned_plot(&few, reserve, true)
            .render_svg_native_with_size(w, h)
            .unwrap();
        let svg_many = binned_plot(&many, reserve, true)
            .render_svg_native_with_size(w, h)
            .unwrap();
        let r_few = data_plot_rect(&svg_few);
        let r_many = data_plot_rect(&svg_many);
        assert_eq!(
            r_few, r_many,
            "panel moved at {w}x{h}: few={r_few:?} many={r_many:?}"
        );
        // The "no data" key is present in both, even though `few` has no
        // missing values at all.
        assert!(svg_few.contains("no data (0)"), "{svg_few}");
        assert!(svg_many.contains("no data ("), "{svg_many}");
    }
}

/// #270: a size encoding shared across two "frames" of different data,
/// under the same `with_limits`, gives identical radii for the same
/// value and an identical legend — exactly the comparability a time
/// map's graduated-symbol frames need.
#[test]
fn size_with_limits_is_comparable_across_two_frames_worth_of_data() {
    let frame_a = ScaleSizeContinuous::new()
        .with_range(2.0, 12.0)
        .with_limits(0.0, 1000.0);
    let frame_b = ScaleSizeContinuous::new()
        .with_range(2.0, 12.0)
        .with_limits(0.0, 1000.0);

    let mut a = frame_a;
    let mut b = frame_b;
    a.train(&[Value::Float(10.0), Value::Float(200.0)]);
    b.train(&[Value::Float(999.0), Value::Float(1.0), Value::Float(500.0)]);

    use ggplot_rs::scale::Scale;
    for v in [0.0, 200.0, 500.0, 1000.0] {
        assert_eq!(
            a.map_to_size(&Value::Float(v)),
            b.map_to_size(&Value::Float(v)),
            "value {v} must size identically regardless of which frame trained it"
        );
    }
    assert_eq!(a.breaks(), b.breaks());
}
