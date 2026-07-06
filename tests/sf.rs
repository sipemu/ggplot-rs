//! geom_sf (feature `sf`): simple-features rendering + scale training.
#![cfg(feature = "sf")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::geom::sf::StatSf;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::scale_set::ScaleSet;
use ggplot_rs::stat::Stat;

fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

#[test]
fn stat_sf_adds_bounds_from_geometry() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 4 0, 4 3, 0 3, 0 0))", "POINT (10 8)"]),
    );
    let out = StatSf.compute_group(&df, &ScaleSet::new());
    // Original column preserved + extent columns added.
    assert!(out.has_column("geometry"));
    for c in ["xmin", "xmax", "ymin", "ymax"] {
        assert!(out.has_column(c), "missing {c}");
    }
    assert_eq!(out.column("xmin").unwrap()[0].as_f64(), Some(0.0));
    assert_eq!(out.column("xmax").unwrap()[0].as_f64(), Some(4.0));
    assert_eq!(out.column("xmax").unwrap()[1].as_f64(), Some(10.0));
    assert_eq!(out.column("ymax").unwrap()[1].as_f64(), Some(8.0));
}

#[test]
fn polygon_choropleth_renders_with_fill() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "POLYGON ((0 0, 3 0, 3 3, 0 3, 0 0))",
            "POLYGON ((3 0, 6 0, 6 3, 3 3, 3 0))",
        ]),
    );
    df.add_column("pop".into(), vec![Value::Float(2.0), Value::Float(9.0)]);

    let svg = GGPlot::new(df)
        .aes(Aes::new().fill("pop"))
        .geom_sf()
        .scale_fill_viridis_c()
        .render_svg()
        .expect("sf render");
    assert!(
        svg.contains("<polygon") || svg.contains("<path"),
        "polygons drawn"
    );
}

#[test]
fn mixed_geometry_types_render() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "POLYGON ((0 0, 5 0, 5 5, 0 5, 0 0))",
            "LINESTRING (0 0, 5 5, 2 5)",
            "MULTIPOINT ((1 1), (4 2), (2 4))",
        ]),
    );
    assert!(GGPlot::new(df)
        .aes(Aes::new())
        .geom_sf()
        .render_svg()
        .is_ok());
}

#[test]
fn scales_train_over_geometry_extent() {
    // A single polygon spanning x in [10, 20] should give an x-axis that covers
    // it, i.e. the built layer exposes xmin/xmax matching the geometry.
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((10 5, 20 5, 20 12, 10 12, 10 5))"]),
    );
    let built = GGPlot::new(df).aes(Aes::new()).geom_sf().build();
    let d = &built.layers[0].data;
    assert_eq!(d.column("xmin").unwrap()[0].as_f64(), Some(10.0));
    assert_eq!(d.column("ymax").unwrap()[0].as_f64(), Some(12.0));
}
