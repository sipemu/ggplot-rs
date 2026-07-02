//! Issue #21 — a computed stat is now estimated per facet panel, not pooled.

use ggplot_rs::prelude::*;

/// Two facet groups whose values are centred far apart. Before the fix, both
/// panels showed a slice of the *pooled* density (identical peaks); now each
/// panel's density is estimated on its own data.
fn two_group_data() -> Vec<(String, Vec<Value>)> {
    let mut x = Vec::new();
    let mut g = Vec::new();
    for i in 0..80 {
        // Group A centred near 0, group B centred near 20.
        let a = (i as f64 * 0.13).sin() * 2.0;
        let b = 20.0 + (i as f64 * 0.17).cos() * 2.0;
        x.push(Value::Float(a));
        g.push(Value::Str("A".into()));
        x.push(Value::Float(b));
        g.push(Value::Str("B".into()));
    }
    vec![("x".to_string(), x), ("g".to_string(), g)]
}

/// x-position of the maximum density in a panel's layer data.
fn density_peak(df: &ggplot_rs::data::DataFrame) -> f64 {
    let xs: Vec<f64> = df
        .column("x")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let ys: Vec<f64> = df
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let mut best = (f64::NEG_INFINITY, 0.0);
    for (x, y) in xs.iter().zip(ys.iter()) {
        if *y > best.0 {
            best = (*y, *x);
        }
    }
    best.1
}

#[test]
fn faceted_density_is_per_panel() {
    let built = GGPlot::new(two_group_data())
        .aes(Aes::new().x("x"))
        .geom_density()
        .facet_wrap("g", Some(2))
        .build();

    assert_eq!(built.panels_data.len(), 2, "two panels expected");
    let peak0 = density_peak(&built.panels_data[0][0]);
    let peak1 = density_peak(&built.panels_data[1][0]);

    // Group A peaks near 0, group B near 20 — a pooled estimate would give the
    // two panels (near-)identical peaks.
    assert!(
        (peak0 - peak1).abs() > 8.0,
        "panels should have distinct density peaks, got {peak0} and {peak1}"
    );
}

#[test]
fn faceted_density_renders() {
    let svg = GGPlot::new(two_group_data())
        .aes(Aes::new().x("x").fill("g"))
        .geom_density()
        .facet_wrap("g", Some(2))
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
