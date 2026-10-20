//! B3-7: `geom_errorbar` caps shrink under a discrete `position_dodge`
//! (allowed #8).

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".into(),
            vec![Value::Str("A".into()), Value::Str("A".into())],
        ),
        (
            "grp".into(),
            vec![Value::Str("G1".into()), Value::Str("G2".into())],
        ),
        ("ymin".into(), vec![Value::Float(1.0), Value::Float(1.5)]),
        ("ymax".into(), vec![Value::Float(3.0), Value::Float(3.5)]),
    ]
}

/// Measure the horizontal extent of an errorbar's top cap (the first
/// segment of its polyline) from the rendered `<polyline points="...">`.
fn cap_widths(svg: &str) -> Vec<f64> {
    svg.match_indices("<polyline points=\"")
        .filter_map(|(start, _)| {
            let rest = &svg[start..];
            let after = rest.strip_prefix("<polyline points=\"")?;
            let end = after.find('"')?;
            let points = &after[..end];
            let coords: Vec<(f64, f64)> = points
                .split_whitespace()
                .map(|p| {
                    let mut it = p.split(',');
                    let x: f64 = it.next().unwrap().parse().unwrap();
                    let y: f64 = it.next().unwrap().parse().unwrap();
                    (x, y)
                })
                .collect();
            // An errorbar polyline is cap(2pts), bar, cap(2pts) = 6 points.
            if coords.len() == 6 {
                Some((coords[0].0 - coords[1].0).abs())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn two_dodged_groups_halve_the_cap_width() {
    let plain = GGPlot::new(data())
        .aes(Aes::new().x("x").ymin("ymin").ymax("ymax"))
        .geom_errorbar()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let dodged = GGPlot::new(data())
        .aes(Aes::new().x("x").ymin("ymin").ymax("ymax").group("grp"))
        .geom_errorbar()
        .position(position_dodge(0.9))
        .render_svg_native_with_size(400, 300)
        .unwrap();

    let plain_caps = cap_widths(&plain);
    let dodged_caps = cap_widths(&dodged);
    // 2 rows at the same x, so 2 bars either way; without a dodge they
    // overlap (same cap width), with one they sit side by side.
    assert_eq!(plain_caps.len(), 2, "{plain}");
    assert_eq!(dodged_caps.len(), 2, "{dodged}");

    let plain_w = plain_caps[0];
    assert!(
        (plain_caps[1] - plain_w).abs() < 1e-6,
        "both un-dodged caps share the same width"
    );
    for w in dodged_caps {
        assert!(
            (w - plain_w / 2.0).abs() < 0.02,
            "dodged cap {w} should be half the plain cap {plain_w}"
        );
    }
}

/// An un-dodged errorbar is unaffected (no `DODGE_N_COL`, so the cap
/// falls back to the full `cap_width`) — covered redundantly by
/// `default_output_022`'s "errorbar" case staying byte-identical, and
/// directly here.
#[test]
fn undodged_errorbar_keeps_the_full_cap_width() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").ymin("ymin").ymax("ymax"))
        .geom_errorbar_with(GeomErrorbar {
            cap_width: 0.05,
            ..GeomErrorbar::default()
        })
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let caps = cap_widths(&svg);
    assert_eq!(caps.len(), 2);
    assert!((caps[0] - caps[1]).abs() < 1e-6, "no dodge column present");
}
