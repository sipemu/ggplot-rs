//! sipemu/ggplot-rs#329: `facet_levels` fixes facet_wrap's panel order and
//! keeps panels for levels with no data (ggplot2 factor levels, drop = FALSE).

use ggplot_rs::data::Value;
use ggplot_rs::facet::FacetScales;
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    // Week 2 first (as if sorted widest-first for drawing), no week 3.
    let weeks = [2, 2, 1, 1, 4];
    vec![
        (
            "x".to_string(),
            (0..5).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..5).map(|i| Value::Float((i * i) as f64)).collect(),
        ),
        (
            "week".to_string(),
            weeks.iter().map(|w| Value::Integer(*w)).collect(),
        ),
    ]
}

fn plot() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
}

fn labels(p: GGPlot) -> Vec<String> {
    p.build().panels.iter().map(|p| p.label.clone()).collect()
}

#[test]
fn default_is_first_appearance_order() {
    assert_eq!(labels(plot().facet_wrap("week", None)), ["2", "1", "4"]);
}

#[test]
fn levels_set_the_order_and_keep_empty_panels() {
    let p = plot()
        .facet_wrap("week", Some(2))
        .facet_levels(&["1", "2", "3", "1"]);
    let built = p.build();
    let names: Vec<_> = built.panels.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(names, ["1", "2", "3"], "ordered, deduplicated, week 3 kept");
    let rows: Vec<usize> = built.panels_data.iter().map(|l| l[0].nrows()).collect();
    assert_eq!(rows, [2, 2, 0], "week 4 is not a level: its row is dropped");
    assert_eq!(
        built
            .panels
            .iter()
            .map(|p| (p.row, p.col))
            .collect::<Vec<_>>(),
        [(0, 0), (0, 1), (1, 0)]
    );
}

#[test]
fn empty_panels_render_under_fixed_and_free_scales() {
    for scales in [FacetScales::Fixed, FacetScales::Free] {
        let svg = plot()
            .facet_wrap_free("week", None, scales)
            .facet_levels(&["1", "2", "3", "4", "5"])
            .render_svg_native_with_size(500, 400)
            .unwrap();
        assert!(!svg.contains("NaN"));
        for w in 1..=5 {
            assert!(svg.contains(&format!(">{w}</text>")), "panel {w} strip");
        }
    }
}
