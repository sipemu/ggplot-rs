//! `annotation_scale()`/`annotation_attribution()` (feature `sf`): end to
//! end through `geom_sf`.
#![cfg(feature = "sf")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::geom::annotation_scale::{GeomAnnotationScale, GeomAttribution, MapCorner};
use ggplot_rs::geom::sf::GeomSf;
use ggplot_rs::prelude::*;
use ggplot_rs::spatial::SfProjection;

fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

/// A ~500km-wide square (UTM-projected degrees chosen so it comes out to a
/// clean, large metric extent) so the scale bar has plenty of room to
/// render a sensible "nice" distance.
fn wide_region() -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((9 48, 15 48, 15 53, 9 53, 9 48))"]),
    );
    df
}

#[test]
fn scale_bar_and_attribution_render_over_a_projected_map() {
    let svg = GGPlot::new(wide_region())
        .aes(Aes::new())
        .geom_sf_with(GeomSf::default().project(SfProjection::utm(32, false)))
        .annotation_scale()
        .annotation_attribution("Made with Natural Earth")
        .render_svg_native_with_size(700, 600)
        .expect("annotation_scale render");
    assert!(svg.contains(" km") || svg.contains(" m<"), "{svg}");
    assert!(svg.contains("Made with Natural Earth"), "{svg}");
}

/// `annotation_scale` has no way to see which projection a `geom_sf` layer
/// used — it only reads the trained X scale's domain and assumes metres,
/// the same assumption ggspatial's own `annotation_scale()` makes (and
/// misreads the same way given an unprojected/geographic CRS). With
/// `SfProjection::PlateCarree` (lon/lat degrees, unchanged) a six-degree-
/// wide domain reads as "6 metres", nicest-bar-rounds to an (wrong, but
/// documented) small label — not suppressed, since there's no reliable way
/// to tell "a real 1m-scale close-up" from "a mis-scaled degree domain" by
/// magnitude alone.
#[test]
fn an_unprojected_plate_carree_map_still_draws_a_bar_from_the_raw_degree_domain() {
    let svg = GGPlot::new(wide_region())
        .aes(Aes::new())
        .geom_sf()
        .annotation_scale()
        .render_svg_native_with_size(700, 600)
        .expect("render");
    assert!(
        svg.contains("data-geom=\"annotation_scale\""),
        "a bar still draws (from the degree domain, not metres): {svg}"
    );
}

#[test]
fn each_corner_places_the_scale_bar_in_a_different_spot() {
    let mut positions = Vec::new();
    for corner in [
        MapCorner::BottomLeft,
        MapCorner::BottomRight,
        MapCorner::TopLeft,
        MapCorner::TopRight,
    ] {
        let svg = GGPlot::new(wide_region())
            .aes(Aes::new())
            .geom_sf_with(GeomSf::default().project(SfProjection::utm(32, false)))
            .annotation_scale_with(GeomAnnotationScale::default().corner(corner))
            .render_svg_native_with_size(700, 600)
            .expect("render");
        let polyline_start = svg.find("<polyline").map(|i| svg[i..].to_string());
        positions.push(polyline_start);
    }
    // Every corner's scale-bar markup should differ from every other's.
    for i in 0..positions.len() {
        for j in (i + 1)..positions.len() {
            assert_ne!(
                positions[i], positions[j],
                "corners {i} and {j} look identical"
            );
        }
    }
}

#[test]
fn attribution_corner_moves_the_text_anchor() {
    let left = GGPlot::new(wide_region())
        .aes(Aes::new())
        .geom_sf()
        .annotation_attribution_with(GeomAttribution::new("Attr").corner(MapCorner::BottomLeft))
        .render_svg_native()
        .expect("render");
    let right = GGPlot::new(wide_region())
        .aes(Aes::new())
        .geom_sf()
        .annotation_attribution_with(GeomAttribution::new("Attr").corner(MapCorner::BottomRight))
        .render_svg_native()
        .expect("render");
    assert!(left.contains("text-anchor=\"start\""), "{left}");
    assert!(right.contains("text-anchor=\"end\""), "{right}");
}

#[test]
fn empty_attribution_text_draws_nothing() {
    let svg = GGPlot::new(wide_region())
        .aes(Aes::new())
        .geom_sf()
        .annotation_attribution("")
        .render_svg_native()
        .expect("render");
    assert!(!svg.contains("data-geom=\"attribution\""), "{svg}");
}

#[test]
fn annotation_scale_ink_theme_hook_recolours_the_default() {
    let svg = GGPlot::new(wide_region())
        .aes(Aes::new())
        .geom_sf_with(GeomSf::default().project(SfProjection::utm(32, false)))
        .annotation_scale()
        .theme(Theme::default().with_ink_paper((255, 255, 255), (0, 0, 0)))
        .render_svg_native_with_size(700, 600)
        .expect("render");
    assert!(svg.contains("#FFFFFF"), "{svg}");
}
