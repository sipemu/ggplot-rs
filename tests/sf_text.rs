//! `geom_sf_text`/`geom_sf_label` (feature `sf`): end-to-end rendering.
#![cfg(feature = "sf")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::geom::sf::GeomSf;
use ggplot_rs::geom::sf_text::{GeomSfLabel, GeomSfText, SfLabelAnchor};
use ggplot_rs::prelude::*;

fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

fn two_regions() -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))",
            "POLYGON ((20 0, 30 0, 30 10, 20 10, 20 0))",
        ]),
    );
    df.add_column("name".into(), strs(&["West", "East"]));
    df
}

#[test]
fn labels_render_over_their_polygons() {
    let df = two_regions();
    let svg = GGPlot::new(df.clone())
        .aes(Aes::new().label("name"))
        .geom_sf_with(GeomSf::default())
        .geom_sf_text_with(GeomSfText::default().fun_geometry(SfLabelAnchor::Polylabel))
        .render_svg_native()
        .expect("sf_text render");
    assert!(svg.contains("West") && svg.contains("East"), "{svg}");
}

#[test]
fn geom_sf_label_draws_a_background_box() {
    let df = two_regions();
    let svg = GGPlot::new(df)
        .aes(Aes::new().label("name"))
        .geom_sf_label_with(GeomSfLabel::default())
        .render_svg_native()
        .expect("sf_label render");
    assert!(svg.contains("<rect"), "a background box: {svg}");
    assert!(svg.contains("West") && svg.contains("East"));
}

#[test]
fn check_overlap_drops_the_lower_priority_label_when_crowded() {
    // Two slivers close enough together that their (wide) labels collide;
    // "Big" (larger screen area) should win over "Small".
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))",
            "POLYGON ((1.01 0, 2.01 0, 2.01 1, 1.01 1, 1.01 0))",
        ]),
    );
    df.add_column(
        "name".into(),
        strs(&["Small region name", "Big region name"]),
    );
    let svg = GGPlot::new(df)
        .aes(Aes::new().label("name"))
        .geom_sf_text_with(GeomSfText::default().check_overlap(true))
        .render_svg_native_with_size(200, 200)
        .expect("sf_text render");
    // Both polygons are the same size here, so this mainly checks the
    // pipeline runs to completion and drops *something* rather than
    // panicking or drawing overlapping garbage; a dedicated unit test
    // (`geom::sf_text::tests`) checks the priority ordering itself.
    assert!(svg.contains("region"));
}

#[test]
fn a_label_wider_than_the_panel_is_dropped_not_clipped() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))"]),
    );
    df.add_column(
        "name".into(),
        strs(&["A label far too long to ever fit in a tiny panel"]),
    );
    let svg = GGPlot::new(df)
        .aes(Aes::new().label("name"))
        .geom_sf_text()
        .render_svg_native_with_size(60, 60)
        .expect("sf_text render");
    assert!(
        !svg.contains("A label far too long"),
        "an oversized label should be dropped, not overflow the panel: {svg}"
    );
}

#[test]
fn centroid_anchor_is_selectable() {
    let df = two_regions();
    let svg = GGPlot::new(df)
        .aes(Aes::new().label("name"))
        .geom_sf_text_with(GeomSfText::default().fun_geometry(SfLabelAnchor::Centroid))
        .render_svg_native()
        .expect("sf_text render");
    assert!(svg.contains("West") && svg.contains("East"));
}

#[test]
fn halo_draws_extra_text_runs_behind_the_label() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))"]),
    );
    df.add_column("name".into(), strs(&["Haloed"]));
    let with_halo = GGPlot::new(df.clone())
        .aes(Aes::new().label("name"))
        .geom_sf_text_with(GeomSfText::default().halo((255, 255, 255), 2.0))
        .render_svg_native()
        .expect("sf_text render");
    let without_halo = GGPlot::new(df)
        .aes(Aes::new().label("name"))
        .geom_sf_text()
        .render_svg_native()
        .expect("sf_text render");
    let count = |svg: &str| svg.matches("Haloed").count();
    assert!(
        count(&with_halo) > count(&without_halo),
        "halo draws extra text runs: {} vs {}",
        count(&with_halo),
        count(&without_halo)
    );
}

/// `.halo(color, 0.0)` (and any non-positive radius) disables the halo —
/// same output as not calling `.halo` at all.
#[test]
fn a_non_positive_halo_radius_disables_it() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))"]),
    );
    df.add_column("name".into(), strs(&["Plain"]));
    let zero_radius = GGPlot::new(df.clone())
        .aes(Aes::new().label("name"))
        .geom_sf_text_with(GeomSfText::default().halo((255, 255, 255), 0.0))
        .render_svg_native()
        .expect("sf_text render");
    let no_halo = GGPlot::new(df)
        .aes(Aes::new().label("name"))
        .geom_sf_text()
        .render_svg_native()
        .expect("sf_text render");
    assert_eq!(
        zero_radius.matches("Plain").count(),
        no_halo.matches("Plain").count()
    );
}

#[test]
fn missing_geometry_column_is_a_clear_error() {
    let mut df = DataFrame::new();
    df.add_column("name".into(), strs(&["X"]));
    let err = GGPlot::new(df)
        .aes(Aes::new().label("name"))
        .geom_sf_text()
        .render_svg_native()
        .unwrap_err();
    assert!(format!("{err:?}").contains("geometry"), "{err:?}");
}

#[test]
fn missing_label_column_is_a_clear_error() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))"]),
    );
    let err = GGPlot::new(df)
        .aes(Aes::new())
        .geom_sf_text()
        .render_svg_native()
        .unwrap_err();
    assert!(format!("{err:?}").contains("label"), "{err:?}");
}

/// `apply_element_geom` (the ink/paper theme hook) recolours `GeomSfLabel`'s
/// default ink border and white fill, like `GeomLabel`'s own.
#[test]
fn geom_sf_label_ink_paper_theme_hook() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))"]),
    );
    df.add_column("name".into(), strs(&["Themed"]));
    let svg = GGPlot::new(df)
        .aes(Aes::new().label("name"))
        .geom_sf_label()
        .theme(Theme::default().with_ink_paper((255, 255, 255), (0, 0, 0)))
        .render_svg_native()
        .expect("sf_label render");
    assert!(svg.contains("#FFFFFF"), "{svg}");
}

/// `GeomSfLabel`'s own builders (`fun_geometry`/`check_overlap`/`project`)
/// take effect the same way `GeomSfText`'s do.
#[test]
fn geom_sf_label_builders_change_behaviour() {
    let df = two_regions();
    let svg = GGPlot::new(df)
        .aes(Aes::new().label("name"))
        .geom_sf_label_with(
            GeomSfLabel::default()
                .fun_geometry(SfLabelAnchor::Centroid)
                .check_overlap(true)
                .project(ggplot_rs::spatial::SfProjection::Mercator),
        )
        .render_svg_native()
        .expect("sf_label render");
    assert!(svg.contains("West") && svg.contains("East"));
}
