//! Issue #46 — element_line linetype (dashed/dotted theme lines).
//! The backend renders non-solid lines by segmenting them, so a dashed grid
//! emits more path elements than a solid one.
use ggplot_rs::prelude::*;

fn svg_with_grid(lt: Linetype) -> String {
    let d = vec![
        (
            "x".to_string(),
            (0..12).map(|i| Value::Float(i as f64)).collect::<Vec<_>>(),
        ),
        (
            "y".to_string(),
            (0..12)
                .map(|i| Value::Float((i % 3) as f64))
                .collect::<Vec<_>>(),
        ),
    ];
    let mut t = theme_minimal();
    t.panel_grid_major.linetype = lt;
    t.panel_grid_minor.visible = false; // isolate the major grid
    GGPlot::new(d)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme(t)
        .render_svg()
        .expect("render")
}

fn segments(svg: &str) -> usize {
    svg.matches("<polyline").count() + svg.matches("<line").count() + svg.matches("<path").count()
}

#[test]
fn dashed_grid_emits_more_segments_than_solid() {
    let solid = segments(&svg_with_grid(Linetype::Solid));
    let dashed = segments(&svg_with_grid(Linetype::Dashed));
    let dotted = segments(&svg_with_grid(Linetype::Dotted));
    assert!(
        dashed > solid && dotted > solid,
        "non-solid grids should emit more segments (solid={solid} dashed={dashed} dotted={dotted})"
    );
}
