//! A nested treemap keeps its group rows (rectangles + group labels): the
//! stat's group rows must not be dropped as "missing values".
use ggplot_rs::prelude::*;

#[test]
fn group_rows_survive_the_build() {
    let s = |v: &[&str]| {
        v.iter()
            .map(|x| Value::Str(x.to_string()))
            .collect::<Vec<_>>()
    };
    let data = vec![
        (
            "x".to_string(),
            s(&["Web", "Shop", "Web", "Shop", "Partner"]),
        ),
        (
            "g".to_string(),
            s(&["North", "North", "South", "South", "West"]),
        ),
        (
            "w".to_string(),
            [420.0, 260.0, 310.0, 180.0, 70.0]
                .iter()
                .map(|&f| Value::Float(f))
                .collect(),
        ),
    ];
    let plot = GGPlot::new(data)
        .aes(Aes::new().weight("w").label("x").subgroup("g").fill("g"))
        .geom_treemap();
    let (svg, warnings) = plot.render_svg_native_with_warnings(560, 360).unwrap();
    if let Ok(p) = std::env::var("TM_SVG") {
        std::fs::write(p, &svg).unwrap();
    }
    assert!(warnings.is_empty(), "{warnings:?}");
    for g in ["North", "South", "West"] {
        assert!(
            svg.contains(&format!(">{g}<")) || svg.contains(&format!(">\n{g}")) || svg.contains(g),
            "group label {g}"
        );
    }
}
