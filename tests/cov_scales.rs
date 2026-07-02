//! Coverage-focused tests for `src/scale/`.
//!
//! Two complementary approaches:
//!  (a) DIRECT — construct scale objects from the public API and call their
//!      public `Scale` trait / inherent methods across a range of inputs,
//!      including edge cases (below/above/on gradient stops, unmapped discrete
//!      levels, NaN/negative transform inputs, empty domains).
//!  (b) RENDER — for logic only reachable through the render pipeline, build a
//!      `GGPlot` that uses the scale via the matching aesthetic and assert that
//!      `render_svg()` returns `Ok`.

use ggplot_rs::prelude::*;
use ggplot_rs::scale::palettes::palette;
use ggplot_rs::scale::Scale;

// ─── helpers ────────────────────────────────────────────────────────────────

fn col(name: &str, vals: Vec<Value>) -> (String, Vec<Value>) {
    (name.to_string(), vals)
}

fn floats(vals: &[f64]) -> Vec<Value> {
    vals.iter().map(|&v| Value::Float(v)).collect()
}

fn strs(vals: &[&str]) -> Vec<Value> {
    vals.iter().map(|&s| Value::Str(s.to_string())).collect()
}

/// Numeric x/y with a 4-level discrete grouping column `g`.
fn discrete_data() -> Vec<(String, Vec<Value>)> {
    vec![
        col("x", floats(&[1.0, 2.0, 3.0, 4.0])),
        col("y", floats(&[1.0, 4.0, 9.0, 16.0])),
        col("g", strs(&["a", "b", "c", "d"])),
    ]
}

/// Numeric x/y with a continuous colour/fill column `z`.
fn continuous_data() -> Vec<(String, Vec<Value>)> {
    vec![
        col("x", floats(&[1.0, 2.0, 3.0, 4.0])),
        col("y", floats(&[1.0, 4.0, 9.0, 16.0])),
        col("z", floats(&[-5.0, 0.0, 5.0, 10.0])),
    ]
}

fn all_palettes() -> Vec<PaletteName> {
    use PaletteName::*;
    vec![
        Set1, Set2, Set3, Dark2, Paired, Pastel1, Pastel2, Accent, Blues, Greens, Reds, Oranges,
        Purples, Greys, YlOrRd, YlGnBu, BuGn, BuPu, GnBu, OrRd, PuRd, RdPu, YlGn, YlOrBr, RdBu,
        Spectral, PiYG, PRGn, BrBG, PuOr, RdGy, RdYlBu, RdYlGn, Viridis, Magma, Plasma, Inferno,
    ]
}

const BLACK: (u8, u8, u8) = (0, 0, 0);
const RED: (u8, u8, u8) = (255, 0, 0);
const WHITE: (u8, u8, u8) = (255, 255, 255);

// ─── (a) DIRECT: ScaleColorGradientN ────────────────────────────────────────

#[test]
fn gradient_n_stops_and_edges() {
    let mut g = ScaleColorGradientN::new(
        Aesthetic::Color,
        vec![
            (1.0, RGBAColor::new(255, 255, 255)),
            (0.0, RGBAColor::new(0, 0, 0)),
            (0.5, RGBAColor::new(255, 0, 0)),
        ],
    );
    // Untrained domain is empty.
    assert!(g.domain().is_none());
    assert!(g.breaks().is_empty());

    g.train(&floats(&[0.0, 10.0]));
    assert_eq!(g.domain(), Some((0.0, 10.0)));

    // Exactly on the (sorted) stops.
    assert_eq!(g.map_to_color(&Value::Float(0.0)), Some(BLACK)); // t = 0.0
    assert_eq!(g.map_to_color(&Value::Float(5.0)), Some(RED)); // t = 0.5
    assert_eq!(g.map_to_color(&Value::Float(10.0)), Some(WHITE)); // t = 1.0

    // Below the first / above the last stop clamp to the endpoints.
    assert_eq!(g.map_to_color(&Value::Float(-100.0)), Some(BLACK));
    assert_eq!(g.map_to_color(&Value::Float(1000.0)), Some(WHITE));

    // Interpolated midpoint between black and red (t = 0.25).
    let (r, gg, b) = g.map_to_color(&Value::Float(2.5)).unwrap();
    assert_eq!((gg, b), (0, 0));
    assert!(r > 100 && r < 160);

    // Non-numeric value maps to t = 0.0 → first stop.
    assert_eq!(g.map_to_color(&Value::Str("nope".into())), Some(BLACK));

    // breaks() populated after training.
    assert!(!g.breaks().is_empty());

    g.reset_training();
    assert!(g.domain().is_none());
}

#[test]
fn gradient_n_empty_and_single_stop() {
    // Empty stops → mid-grey fallback.
    let mut empty = ScaleColorGradientN::new(Aesthetic::Fill, vec![]);
    empty.train(&floats(&[0.0, 1.0]));
    assert_eq!(
        empty.map_to_color(&Value::Float(0.5)),
        Some((127, 127, 127))
    );

    // Single stop → that colour everywhere.
    let mut single =
        ScaleColorGradientN::new(Aesthetic::Fill, vec![(0.5, RGBAColor::new(10, 20, 30))]);
    single.train(&floats(&[0.0, 10.0]));
    assert_eq!(single.map_to_color(&Value::Float(0.0)), Some((10, 20, 30)));
    assert_eq!(single.map_to_color(&Value::Float(10.0)), Some((10, 20, 30)));
}

#[test]
fn gradient_n_zero_range_maps_middle() {
    let mut g = ScaleColorGradientN::viridis(Aesthetic::Color);
    g.train(&floats(&[7.0, 7.0])); // min == max
    assert_eq!(g.map(&Value::Float(7.0)), 0.5);
    // single-value break.
    assert_eq!(g.breaks().len(), 1);
}

#[test]
fn gradient_n_perceptual_endpoints() {
    // Each perceptual constructor: endpoints via map_to_color after training [0,1].
    type Rgb = (u8, u8, u8);
    let cases: Vec<(ScaleColorGradientN, Rgb, Rgb)> = vec![
        (
            ScaleColorGradientN::viridis(Aesthetic::Color),
            (68, 1, 84),
            (253, 231, 37),
        ),
        (
            ScaleColorGradientN::magma(Aesthetic::Color),
            (0, 0, 4),
            (252, 253, 191),
        ),
        (
            ScaleColorGradientN::plasma(Aesthetic::Color),
            (13, 8, 135),
            (240, 249, 33),
        ),
        (
            ScaleColorGradientN::inferno(Aesthetic::Color),
            (0, 0, 4),
            (252, 255, 164),
        ),
    ];
    for (mut g, lo, hi) in cases {
        g.train(&floats(&[0.0, 1.0]));
        assert_eq!(g.map_to_color(&Value::Float(0.0)), Some(lo));
        assert_eq!(g.map_to_color(&Value::Float(1.0)), Some(hi));
    }
}

// ─── (a) DIRECT: ScaleManual ────────────────────────────────────────────────

#[test]
fn manual_mapping_fallback_and_wrap() {
    let mut m = ScaleManual::new(
        Aesthetic::Color,
        vec![
            ("a", RGBAColor::new(1, 0, 0)),
            ("b", RGBAColor::new(0, 1, 0)),
            ("c", RGBAColor::new(0, 0, 1)),
        ],
    );
    assert!(m.is_discrete());
    assert_eq!(m.aesthetic(), Aesthetic::Color);

    m.train(&strs(&["a", "b", "c"]));
    assert_eq!(m.map_to_color(&Value::Str("a".into())), Some((1, 0, 0)));
    assert_eq!(m.map_to_color(&Value::Str("c".into())), Some((0, 0, 1)));
    assert_eq!(m.map(&Value::Str("b".into())), 1.0);

    // Unmapped level falls back to index 0.
    assert_eq!(m.map_to_color(&Value::Str("zzz".into())), Some((1, 0, 0)));
    assert_eq!(m.map(&Value::Str("zzz".into())), 0.0);

    // Training a level with no matching colour triggers the wrap-around branch
    // (idx 3 % 3 == 0 → first colour).
    m.train(&strs(&["d"]));
    assert_eq!(m.map_to_color(&Value::Str("d".into())), Some((1, 0, 0)));

    assert_eq!(m.breaks().len(), 4);

    m.reset_training();
    assert!(m.breaks().is_empty());
}

// ─── (a) DIRECT: ScaleColorDiscrete / ScaleColorContinuous ──────────────────

#[test]
fn color_discrete_levels_and_wrap() {
    let mut s = ScaleColorDiscrete::new(Aesthetic::Color).with_named_palette(&PaletteName::Set1);
    s.train(&strs(&["x", "y", "z"]));
    assert_eq!(
        s.levels(),
        &["x".to_string(), "y".to_string(), "z".to_string()]
    );
    assert!(s.is_discrete());

    // Set1[0] == (228, 26, 28)
    assert_eq!(s.map_to_color(&Value::Str("x".into())), Some((228, 26, 28)));
    // color_for_value / color_for_index agree.
    let c0 = s.color_for_value(&Value::Str("x".into()));
    assert_eq!((c0.r, c0.g, c0.b), (228, 26, 28));

    // Unmapped level → index 0.
    assert_eq!(
        s.map_to_color(&Value::Str("missing".into())),
        Some((228, 26, 28))
    );

    // color_for_index wraps beyond palette length.
    let wrapped = s.color_for_index(9); // Set1 has 9 colours → 9 % 9 == 0
    assert_eq!((wrapped.r, wrapped.g, wrapped.b), (228, 26, 28));

    // Custom palette via with_palette.
    let mut cp = ScaleColorDiscrete::new(Aesthetic::Fill)
        .with_palette(vec![RGBAColor::new(9, 8, 7), RGBAColor::new(6, 5, 4)]);
    cp.train(&strs(&["p", "q"]));
    assert_eq!(cp.map_to_color(&Value::Str("q".into())), Some((6, 5, 4)));

    s.reset_training();
    assert!(s.levels().is_empty());
}

#[test]
fn color_continuous_endpoints_and_domain() {
    let mut s = ScaleColorContinuous::new(Aesthetic::Color)
        .with_colors(RGBAColor::new(0, 0, 255), RGBAColor::new(255, 0, 0));
    assert!(s.domain().is_none());

    s.train(&floats(&[0.0, 100.0]));
    assert_eq!(s.domain(), Some((0.0, 100.0)));
    assert_eq!(s.map(&Value::Float(0.0)), 0.0);
    assert_eq!(s.map(&Value::Float(100.0)), 1.0);
    assert_eq!(s.map_to_color(&Value::Float(0.0)), Some((0, 0, 255)));
    assert_eq!(s.map_to_color(&Value::Float(100.0)), Some((255, 0, 0)));

    // Midpoint interpolation.
    let mid = s.color_at(0.5);
    assert_eq!((mid.r, mid.b), (127, 127));

    // Non-numeric maps to 0.0.
    assert_eq!(s.map(&Value::Str("x".into())), 0.0);
    assert!(!s.breaks().is_empty());

    s.reset_training();
    assert!(s.domain().is_none());
    assert!(s.breaks().is_empty());
}

#[test]
fn color_continuous_zero_range() {
    let mut s = ScaleColorContinuous::new(Aesthetic::Fill);
    s.train(&floats(&[3.0, 3.0]));
    assert_eq!(s.map(&Value::Float(3.0)), 0.5);
    assert_eq!(s.breaks().len(), 1);
}

// ─── (a) DIRECT: palettes ───────────────────────────────────────────────────

#[test]
fn every_palette_non_empty() {
    for name in all_palettes() {
        let colors = palette(&name);
        assert!(!colors.is_empty(), "palette {name:?} was empty");
    }
}

// ─── (a) DIRECT: ScaleColorGradient2 (diverging) ────────────────────────────

#[test]
fn gradient2_diverging_around_midpoint() {
    let mut s = ScaleColorGradient2::new(Aesthetic::Fill)
        .with_colors(
            RGBAColor::new(0, 0, 255),
            RGBAColor::new(255, 255, 255),
            RGBAColor::new(255, 0, 0),
        )
        .with_midpoint(0.0);
    assert!(s.domain().is_none());

    s.train(&floats(&[-10.0, 10.0]));
    assert_eq!(s.domain(), Some((-10.0, 10.0)));

    // At the midpoint → mid colour (white).
    assert_eq!(s.map_to_color(&Value::Float(0.0)), Some(WHITE));
    // Extremes → low / high colours.
    assert_eq!(s.map_to_color(&Value::Float(-10.0)), Some((0, 0, 255)));
    assert_eq!(s.map_to_color(&Value::Float(10.0)), Some(RED));

    // Below midpoint blends low→mid; above blends mid→high.
    let below = s.map_to_color(&Value::Float(-5.0)).unwrap();
    assert!(below.2 > below.0); // still bluish
    let above = s.map_to_color(&Value::Float(5.0)).unwrap();
    assert!(above.0 > above.2); // still reddish

    // Non-numeric returns None (map_to_color uses `?`).
    assert_eq!(s.map_to_color(&Value::Str("x".into())), None);
    assert!(!s.breaks().is_empty());
}

// ─── (a) DIRECT: ScaleColorGrey ─────────────────────────────────────────────

#[test]
fn grey_range_and_single_level() {
    let mut s = ScaleColorGrey::new(Aesthetic::Color).with_range(0.0, 1.0);
    s.train(&strs(&["a", "b", "c"]));
    assert!(s.is_discrete());
    // First level → start (0.0 → black), last → end (1.0 → white).
    assert_eq!(s.map_to_color(&Value::Str("a".into())), Some((0, 0, 0)));
    assert_eq!(
        s.map_to_color(&Value::Str("c".into())),
        Some((255, 255, 255))
    );
    // Grey: r == g == b.
    let (r, g, b) = s.map_to_color(&Value::Str("b".into())).unwrap();
    assert_eq!((r, g), (r, b));
    assert_eq!(s.breaks().len(), 3);

    // Single-level → midpoint grey branch.
    let mut one = ScaleColorGrey::new(Aesthetic::Fill).with_range(0.2, 0.8);
    one.train(&strs(&["only"]));
    let (gr, _, _) = one.map_to_color(&Value::Str("only".into())).unwrap();
    assert_eq!(gr, ((0.5_f64) * 255.0) as u8);

    s.reset_training();
    assert!(s.breaks().is_empty());
}

// ─── (a) DIRECT: ScaleTransform variants ────────────────────────────────────

#[test]
fn transform_apply_inverse_all_variants() {
    // Forward.
    assert_eq!(ScaleTransform::Identity.apply(5.0), 5.0);
    assert_eq!(ScaleTransform::Log10.apply(1000.0), 3.0);
    assert_eq!(ScaleTransform::Log2.apply(8.0), 3.0);
    assert_eq!(ScaleTransform::Ln.apply(1.0), 0.0);
    assert_eq!(ScaleTransform::Sqrt.apply(9.0), 3.0);
    assert_eq!(ScaleTransform::Reverse.apply(4.0), -4.0);

    // Inverse.
    assert_eq!(ScaleTransform::Identity.inverse(5.0), 5.0);
    assert_eq!(ScaleTransform::Log10.inverse(2.0), 100.0);
    assert_eq!(ScaleTransform::Log2.inverse(3.0), 8.0);
    assert_eq!(ScaleTransform::Ln.inverse(0.0), 1.0);
    assert_eq!(ScaleTransform::Sqrt.inverse(2.0), 4.0);
    assert_eq!(ScaleTransform::Reverse.inverse(-4.0), 4.0);

    // is_identity.
    assert!(ScaleTransform::Identity.is_identity());
    assert!(!ScaleTransform::Log10.is_identity());
}

#[test]
fn transform_edge_inputs() {
    // Non-positive inputs to log transforms → -inf.
    assert_eq!(ScaleTransform::Log10.apply(0.0), f64::NEG_INFINITY);
    assert_eq!(ScaleTransform::Log10.apply(-5.0), f64::NEG_INFINITY);
    assert_eq!(ScaleTransform::Log2.apply(-1.0), f64::NEG_INFINITY);
    assert_eq!(ScaleTransform::Ln.apply(-1.0), f64::NEG_INFINITY);
    // Negative sqrt → NaN.
    assert!(ScaleTransform::Sqrt.apply(-4.0).is_nan());

    // transform_value coerces non-finite results to Na.
    assert!(matches!(
        ScaleTransform::Log10.transform_value(&Value::Float(-1.0)),
        Value::Na
    ));
    assert!(matches!(
        ScaleTransform::Sqrt.transform_value(&Value::Float(-1.0)),
        Value::Na
    ));
    // Finite result → Float.
    match ScaleTransform::Log10.transform_value(&Value::Float(100.0)) {
        Value::Float(f) => assert_eq!(f, 2.0),
        other => panic!("expected Float, got {other:?}"),
    }
    // Non-numeric passes through unchanged.
    match ScaleTransform::Log10.transform_value(&Value::Str("s".into())) {
        Value::Str(s) => assert_eq!(s, "s"),
        other => panic!("expected Str, got {other:?}"),
    }
}

// ─── (a) DIRECT: ScaleContinuous ────────────────────────────────────────────

#[test]
fn continuous_limits_expand_breaks_secaxis() {
    // filter_limits set only via with_limits.
    let s = ScaleContinuous::new()
        .for_aesthetic(Aesthetic::X)
        .with_name("xaxis")
        .with_limits(0.0, 10.0);
    assert_eq!(s.filter_limits(), Some((0.0, 10.0)));
    assert_eq!(s.domain(), Some((0.0, 10.0)));
    assert_eq!(s.name(), "xaxis");

    // Plain trained scale has no OOB filter.
    let mut plain = ScaleContinuous::new();
    plain.train(&floats(&[0.0, 100.0]));
    assert!(plain.filter_limits().is_none());
    assert!(!plain.breaks().is_empty());
    // Expanded range means data min maps just inside (0,1), not to 0.
    let at_min = plain.map(&Value::Float(0.0));
    assert!(at_min > 0.0 && at_min < 0.1);

    // Untrained → empty breaks.
    let untrained = ScaleContinuous::new();
    assert!(untrained.breaks().is_empty());

    // Custom breaks + labels.
    let mut cb = ScaleContinuous::new()
        .with_breaks(vec![0.0, 50.0, 100.0])
        .with_labels(vec!["low".into(), "mid".into(), "high".into()]);
    cb.train(&floats(&[0.0, 100.0]));
    let breaks = cb.breaks();
    assert_eq!(breaks.len(), 3);
    assert_eq!(breaks[0].1, "low");
    assert_eq!(breaks[2].1, "high");

    // Custom breaks without labels use the formatter/inverse path.
    let mut cb2 = ScaleContinuous::new().with_breaks(vec![10.0, 20.0]);
    cb2.train(&floats(&[0.0, 30.0]));
    assert_eq!(cb2.breaks().len(), 2);

    // Label formatter applied to generated breaks.
    let mut lf = ScaleContinuous::new().with_label_formatter(label_percent);
    lf.train(&floats(&[0.0, 1.0]));
    assert!(lf.breaks().iter().any(|(_, l)| l.ends_with('%')));

    // Secondary axis round-trips through the trait method.
    let sa = ScaleContinuous::new().with_sec_axis(SecAxis::new(|c| c * 2.0).with_name("double"));
    assert!(sa.sec_axis().is_some());
    assert_eq!(sa.sec_axis().unwrap().transform_value(21.0), 42.0);
}

#[test]
fn continuous_transform_labels_and_zero_range() {
    // Log10 scale: labels show the inverse (original) values.
    let mut s = ScaleContinuous::new().with_transform(ScaleTransform::Log10);
    // Values already in transformed space (log10 of 1..1000).
    s.train(&floats(&[0.0, 3.0]));
    match s.transform(&Value::Float(100.0)) {
        Value::Float(f) => assert_eq!(f, 2.0),
        other => panic!("expected Float, got {other:?}"),
    }
    assert!(!s.breaks().is_empty());

    // set_limits overrides the trained domain.
    let mut z = ScaleContinuous::new();
    z.set_limits(5.0, 5.0); // zero range
    assert_eq!(z.map(&Value::Float(5.0)), 0.5);
    assert_eq!(z.breaks().len(), 1);
}

// ─── (a) DIRECT: ScaleDiscrete ──────────────────────────────────────────────

#[test]
fn discrete_positions_labels_limits() {
    let mut s = ScaleDiscrete::new()
        .for_aesthetic(Aesthetic::X)
        .with_name("cat")
        .with_labels(vec!["A".into(), "B".into()]);
    s.train(&strs(&["a", "b"]));
    assert!(s.is_discrete());
    // Evenly spaced positions at (i + 0.5) / n.
    assert_eq!(s.map(&Value::Str("a".into())), 0.25);
    assert_eq!(s.map(&Value::Str("b".into())), 0.75);
    let breaks = s.breaks();
    assert_eq!(breaks.len(), 2);
    assert_eq!(breaks[0].1, "A");

    // Unmapped value → middle (0.5).
    assert_eq!(s.map(&Value::Str("missing".into())), 0.5);

    // Empty scale maps to 0.5 and yields no breaks.
    let empty = ScaleDiscrete::new();
    assert_eq!(empty.map(&Value::Str("x".into())), 0.5);
    assert!(empty.breaks().is_empty());

    // Limits filter + order; values outside limits map to 0.5.
    let mut lim = ScaleDiscrete::new().with_limits(vec!["b", "a"]);
    lim.train(&strs(&["a", "b", "c"]));
    assert_eq!(lim.map(&Value::Str("b".into())), 0.25); // first in limits
    assert_eq!(lim.map(&Value::Str("c".into())), 0.5); // not in limits
    assert_eq!(lim.breaks().len(), 2);
    // reset_training keeps limits-provided levels.
    lim.reset_training();
    assert_eq!(lim.breaks().len(), 2);
}

// ─── (a) DIRECT: ScaleDateTime ──────────────────────────────────────────────

#[test]
fn datetime_breaks_across_step_sizes() {
    const DAY: f64 = 86400.0;
    const YEAR: f64 = 365.25 * DAY;

    // A range covering every branch of nice_datetime_step.
    let ranges = [
        3.0,          // seconds
        200.0,        // minutes
        4.0 * 3600.0, // hours
        3.0 * DAY,    // days
        40.0 * DAY,   // weeks/months
        3.0 * YEAR,   // years
        400.0 * YEAR, // > 100-year fallback
    ];
    for r in ranges {
        let mut s = ScaleDateTime::new().for_aesthetic(Aesthetic::X);
        s.train(&floats(&[0.0, r]));
        assert!(
            !s.breaks().is_empty(),
            "expected breaks for datetime range {r}"
        );
        // map is monotone within the expanded range.
        assert!(s.map(&Value::Float(0.0)) < s.map(&Value::Float(r)));
    }
}

#[test]
fn datetime_untrained_single_and_limits() {
    // Untrained → no breaks.
    let s = ScaleDateTime::new();
    assert!(s.breaks().is_empty());

    // Single value (zero range) → one centred break.
    let mut one = ScaleDateTime::new().with_name("t");
    one.train(&floats(&[1_600_000_000.0]));
    assert_eq!(one.map(&Value::Float(1_600_000_000.0)), 0.5);
    assert_eq!(one.breaks().len(), 1);
    assert_eq!(one.name(), "t");

    // set_limits marks the scale trained.
    let mut lim = ScaleDateTime::new();
    lim.set_limits(0.0, 10.0 * 86400.0);
    assert!(!lim.breaks().is_empty());

    // Non-numeric maps to 0.0.
    assert_eq!(one.map(&Value::Str("x".into())), 0.0);

    one.reset_training();
    assert!(one.breaks().is_empty());
}

// ─── (a) DIRECT: label formatters ───────────────────────────────────────────

#[test]
fn label_formatters_values() {
    assert_eq!(label_comma(1_234_567.0), "1,234,567");
    assert_eq!(label_comma(-5000.0), "-5,000");
    assert_eq!(label_comma(1234.5), "1,234.5");

    assert_eq!(label_percent(0.5), "50%");
    assert_eq!(label_percent(0.123), "12.3%");

    assert_eq!(label_dollar(1000.0), "$1,000");
    assert_eq!(label_dollar(-500.0), "-$500");

    assert_eq!(label_scientific(12345.0), "1.23e4");
    assert_eq!(label_scientific(0.0), "0");
    assert_eq!(label_scientific(100.0), "1e2");
}

// ─── (b) RENDER: every named palette ────────────────────────────────────────

#[test]
fn render_every_brewer_palette() {
    for name in all_palettes() {
        let svg = GGPlot::new(discrete_data())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_point()
            .scale_color_brewer(name.clone())
            .render_svg();
        assert!(svg.is_ok(), "scale_color_brewer({name:?}) failed to render");
    }
}

#[test]
fn render_fill_brewer_and_grey() {
    let svg = GGPlot::new(discrete_data())
        .aes(Aes::new().x("x").y("y").fill("g"))
        .geom_col()
        .scale_fill_brewer(PaletteName::Spectral)
        .render_svg();
    assert!(svg.is_ok());

    let svg = GGPlot::new(discrete_data())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .scale_color_grey_with(ScaleColorGrey::new(Aesthetic::Color).with_range(0.1, 0.9))
        .render_svg();
    assert!(svg.is_ok());

    let svg = GGPlot::new(discrete_data())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .scale_color_grey()
        .render_svg();
    assert!(svg.is_ok());
}

// ─── (b) RENDER: continuous colour scales ───────────────────────────────────

#[test]
fn render_continuous_color_scales() {
    // viridis (discrete + continuous), gradient, gradient2, gradientn.
    let builders: Vec<Box<dyn Fn() -> Result<String, GGError>>> = vec![
        Box::new(|| {
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y").color("z"))
                .geom_point()
                .scale_color_viridis_c()
                .render_svg()
        }),
        Box::new(|| {
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y").color("z"))
                .geom_point()
                .scale_color_viridis()
                .render_svg()
        }),
        Box::new(|| {
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y").color("z"))
                .geom_point()
                .scale_color_gradient(RGBAColor::new(0, 0, 255), RGBAColor::new(255, 0, 0))
                .render_svg()
        }),
        Box::new(|| {
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y").color("z"))
                .geom_point()
                .scale_color_gradient2(
                    RGBAColor::new(0, 0, 255),
                    RGBAColor::new(255, 255, 255),
                    RGBAColor::new(255, 0, 0),
                )
                .render_svg()
        }),
        Box::new(|| {
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y").color("z"))
                .geom_point()
                .scale_color_gradientn(vec![
                    (0.0, RGBAColor::new(0, 0, 0)),
                    (0.5, RGBAColor::new(255, 0, 0)),
                    (1.0, RGBAColor::new(255, 255, 255)),
                ])
                .render_svg()
        }),
        Box::new(|| {
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y").fill("z"))
                .geom_col()
                .scale_fill_viridis_c()
                .render_svg()
        }),
    ];
    for (i, b) in builders.iter().enumerate() {
        assert!(b().is_ok(), "continuous color builder #{i} failed");
    }
}

// ─── (b) RENDER: position-scale features ────────────────────────────────────

#[test]
fn render_sec_axis_and_formatters() {
    let svg = GGPlot::new(continuous_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_continuous(
            ScaleContinuous::new()
                .for_aesthetic(Aesthetic::Y)
                .with_sec_axis(SecAxis::new(|v| v * 9.0 / 5.0 + 32.0).with_name("F")),
        )
        .render_svg();
    assert!(svg.is_ok());

    let svg = GGPlot::new(continuous_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_continuous(
            ScaleContinuous::new()
                .for_aesthetic(Aesthetic::Y)
                .with_label_formatter(label_dollar),
        )
        .scale_x_continuous(
            ScaleContinuous::new()
                .for_aesthetic(Aesthetic::X)
                .with_breaks(vec![1.0, 2.0, 3.0, 4.0])
                .with_labels(vec![
                    "one".into(),
                    "two".into(),
                    "three".into(),
                    "four".into(),
                ])
                .with_expand(0.2, 1.0),
        )
        .render_svg();
    assert!(svg.is_ok());
}

#[test]
fn render_transforms_and_xlim() {
    let variants: Vec<Box<dyn Fn() -> Result<String, GGError>>> = vec![
        Box::new(|| {
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y"))
                .geom_point()
                .scale_x_log10()
                .render_svg()
        }),
        Box::new(|| {
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y"))
                .geom_point()
                .scale_x_log2()
                .render_svg()
        }),
        Box::new(|| {
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y"))
                .geom_point()
                .scale_x_ln()
                .render_svg()
        }),
        Box::new(|| {
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y"))
                .geom_point()
                .scale_x_sqrt()
                .render_svg()
        }),
        Box::new(|| {
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y"))
                .geom_point()
                .scale_x_reverse()
                .render_svg()
        }),
        Box::new(|| {
            // xlim filters OOB data before stats.
            GGPlot::new(continuous_data())
                .aes(Aes::new().x("x").y("y"))
                .geom_point()
                .xlim(1.5, 3.5)
                .render_svg()
        }),
    ];
    for (i, v) in variants.iter().enumerate() {
        assert!(v().is_ok(), "transform/xlim builder #{i} failed");
    }
}

// ─── (b) RENDER: datetime scale ─────────────────────────────────────────────

#[test]
fn render_datetime_scale() {
    let data = vec![
        col(
            "x",
            vec![
                Value::DateTime(0),
                Value::DateTime(90 * 86400),
                Value::DateTime(180 * 86400),
                Value::DateTime(365 * 86400),
            ],
        ),
        col("y", floats(&[1.0, 2.0, 3.0, 4.0])),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .scale_x_datetime(
            ScaleDateTime::new()
                .for_aesthetic(Aesthetic::X)
                .with_name("date"),
        )
        .render_svg();
    assert!(svg.is_ok());
}
