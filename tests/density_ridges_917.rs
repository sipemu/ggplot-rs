//! `geom_density_ridges` (anofox-visualization#917): one ridge per level.
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let mut x = Vec::new();
    let mut y = Vec::new();
    for (m, shift) in [("Jan", 0.0), ("Feb", 2.0), ("Mar", 4.0), ("Apr", 3.0)] {
        for i in 0..60 {
            let v =
                shift + ((i * 37 % 60) as f64 / 10.0) - 3.0 + if i % 3 == 0 { 1.5 } else { 0.0 };
            x.push(Value::Float(v));
            y.push(Value::Str(m.to_string()));
        }
    }
    vec![("x".into(), x), ("m".into(), y)]
}

fn svg() -> String {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("m").fill("m"))
        .geom_density_ridges()
        .scale_y_discrete(
            ScaleDiscrete::new()
                .with_limits(vec!["Jan", "Feb", "Mar", "Apr"])
                .with_expand(0.0, 1.1),
        )
        .render_svg_native()
        .unwrap()
}

#[test]
fn a_ridge_per_level_back_to_front() {
    let s = svg();
    if let Ok(p) = std::env::var("RIDGES_SVG") {
        std::fs::write(p, &s).unwrap();
    }
    let ridges: Vec<&str> = s
        .split("<polygon ")
        .skip(1)
        .filter(|p| p.contains("data-x="))
        .collect();
    assert_eq!(ridges.len(), 4, "{s}");
    // drawn from the top row down, so the lowest ridge is in front
    let order: Vec<&str> = ridges
        .iter()
        .filter_map(|p| p.split("data-x=\"").nth(1).map(|r| &r[..3]))
        .collect();
    assert_eq!(order, ["Apr", "Mar", "Feb", "Jan"], "{s}");
}

#[test]
fn discrete_expansion_moves_levels_and_ticks_together() {
    use ggplot_rs::scale::Scale;
    let mut sc = ScaleDiscrete::new().with_expand(1.0, 1.0);
    sc.train(&[Value::Str("a".into()), Value::Str("b".into())]);
    // 2 levels + 1 + 1 slots: a at 1.5/4, b at 2.5/4
    assert!((sc.map(&Value::Str("a".into())) - 0.375).abs() < 1e-12);
    let b: Vec<f64> = sc.breaks().iter().map(|(p, _)| *p).collect();
    assert!((b[1] - 0.625).abs() < 1e-12, "{b:?}");
}
