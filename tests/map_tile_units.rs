//! `GeomMapTile::units`: a domain in EPSG:3857 metres asks for the same tiles
//! as the same view in unit-sphere Mercator radians (anofox-visualization#760).
#![cfg(feature = "tiles")]
use ggplot_rs::geom::map_tile::{GeomMapTile, TileSource};
use ggplot_rs::prelude::*;
use std::sync::{Arc, Mutex};

const R: f64 = 6_378_137.0;

fn tiles(scale: f64) -> Vec<(u32, u32, u32)> {
    // roughly Germany: lon 6–15°, lat 47–55° in Mercator units × scale
    let merc = |lon: f64, lat: f64| {
        let (l, p) = (lon.to_radians(), lat.to_radians());
        (
            l * scale,
            (std::f64::consts::FRAC_PI_4 + p / 2.0).tan().ln() * scale,
        )
    };
    let (a, b) = (merc(6.0, 47.0), merc(15.0, 55.0));
    let data = vec![
        ("x".to_string(), vec![Value::Float(a.0), Value::Float(b.0)]),
        ("y".to_string(), vec![Value::Float(a.1), Value::Float(b.1)]),
    ];
    let seen = Arc::new(Mutex::new(Vec::new()));
    let s2 = seen.clone();
    let geom = GeomMapTile::new(TileSource::xyz(""), move |z, x, y| {
        s2.lock().unwrap().push((z, x, y));
        None
    })
    .units(scale);
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .annotation_map_tile_with(geom)
        .geom_point()
        .render_svg_native()
        .unwrap();
    let mut v = seen.lock().unwrap().clone();
    v.sort();
    v.dedup();
    v
}

#[test]
fn metres_ask_for_the_same_tiles_as_radians() {
    let unit = tiles(1.0);
    assert!(!unit.is_empty());
    assert_eq!(tiles(R), unit);
}
