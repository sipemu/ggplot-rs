//! sipemu/ggplot-rs#142: `element_geom` (ink) coverage for `GeomSf`'s default
//! `(50, 50, 50)` outline (feature `sf`). Split out from
//! `geom_ink_paper_coverage.rs` because `GeomSf` needs the `sf` feature and
//! isn't re-exported from the prelude.
#![cfg(feature = "sf")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::geom::sf::GeomSf;
use ggplot_rs::prelude::*;

fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

fn polygon() -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 3 0, 3 3, 0 3, 0 0))"]),
    );
    df
}

const WHITE: (u8, u8, u8) = (255, 255, 255);
const BLACK: (u8, u8, u8) = (0, 0, 0);

#[test]
fn ink_recolours_the_sf_outline_default() {
    let svg = GGPlot::new(polygon())
        .aes(Aes::new())
        .geom_sf()
        .theme(Theme::default().with_ink_paper(WHITE, BLACK))
        .render_svg_native()
        .expect("sf render");
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");
    assert!(!svg.contains("#323232"), "no built-in (50,50,50): {svg}");
}

#[test]
fn an_explicit_sf_outline_equal_to_the_default_is_unaffected_by_ink() {
    let svg = GGPlot::new(polygon())
        .aes(Aes::new())
        .geom_sf_with(GeomSf {
            color: (50, 50, 50),
            ..Default::default()
        })
        .theme(Theme::default().with_ink_paper(WHITE, BLACK))
        .render_svg_native()
        .expect("sf render");
    assert!(svg.contains("stroke=\"#323232\""), "{svg}");
}
