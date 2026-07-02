//! Issue #21 — facet_grid space="free_x" proportional panel sizing.
use ggplot_rs::prelude::*;

#[test]
fn facet_grid_space_free_x_renders() {
    // Two columns with very different x-ranges.
    let mut x = vec![];
    let mut y = vec![];
    let mut c = vec![];
    for i in 0..10 {
        x.push(Value::Float(i as f64));
        y.push(Value::Float((i % 3) as f64));
        c.push(Value::Str("narrow".into()));
    }
    for i in 0..10 {
        x.push(Value::Float(i as f64 * 6.0));
        y.push(Value::Float((i % 3) as f64));
        c.push(Value::Str("wide".into()));
    }
    let svg = GGPlot::new(vec![
        ("x".to_string(), x),
        ("y".to_string(), y),
        ("c".to_string(), c),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .facet_grid_space(None, Some("c"), FacetScales::FreeX, FacetSpace::FreeX)
    .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn facet_grid_multi_two_col_vars() {
    // Columns = combination of `region` (2) x `tier` (2) = up to 4 panels.
    let mut x = vec![];
    let mut y = vec![];
    let mut region = vec![];
    let mut tier = vec![];
    for i in 0..24 {
        x.push(Value::Float((i % 6) as f64));
        y.push(Value::Float((i % 4) as f64));
        region.push(Value::Str(["N", "S"][(i / 2) % 2].into()));
        tier.push(Value::Str(["hi", "lo"][i % 2].into()));
    }
    let built = GGPlot::new(vec![
        ("x".into(), x),
        ("y".into(), y),
        ("region".into(), region),
        ("tier".into(), tier),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .facet_grid_multi(
        None,
        &["region", "tier"],
        FacetScales::Fixed,
        FacetSpace::Fixed,
    )
    .build();
    // 2 regions x 2 tiers = 4 column panels.
    assert_eq!(built.panels.len(), 4, "expected 4 combined-column panels");
}
