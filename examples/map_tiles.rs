//! A raster basemap under `geom_sf` (#325): `annotation_map_tile` with a
//! tile fetcher. ggplot-rs does no networking — the fetcher supplies the
//! tile bytes. Here it either reads a local XYZ cache (`TILE_DIR=/path`
//! holding `{z}/{x}/{y}.png`) or, by default, draws synthetic tiles: a pale
//! sea with a 5° graticule computed per pixel from the tile's own
//! Web-Mercator bounds, so the cities landing on the right grid cells shows
//! the tiles and the projected geometry line up.
//!
//! ```sh
//! cargo run --no-default-features --features tiles,plotters --example map_tiles
//! ```

use std::f64::consts::PI;

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::map_tile::{tile_bounds, TileSource};
use ggplot_rs::geom::sf::GeomSf;
use ggplot_rs::spatial::SfProjection;
use ggplot_rs::GGPlot;

/// A 256 px tile: pale background, 5° graticule, a darker line every 10°.
fn synthetic_tile(z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
    let (x0, yt, x1, yb) = tile_bounds(z, x as i64, y as i64);
    let size = 256u32;
    let deg_per_px = (x1 - x0).to_degrees() / size as f64;
    let img = image::RgbImage::from_fn(size, size, |px, py| {
        let mx = x0 + (px as f64 + 0.5) / size as f64 * (x1 - x0);
        let my = yt + (py as f64 + 0.5) / size as f64 * (yb - yt);
        let lon = mx.to_degrees();
        let lat = (2.0 * my.exp().atan() - PI / 2.0).to_degrees();
        // Vertical px per degree latitude varies with latitude.
        let lat_per_px = deg_per_px * lat.to_radians().cos();
        let near = |v: f64, step: f64, tol: f64| {
            let r = v.rem_euclid(step);
            r < tol || step - r < tol
        };
        if near(lon, 10.0, deg_per_px) || near(lat, 10.0, lat_per_px) {
            image::Rgb([150, 170, 190])
        } else if near(lon, 5.0, deg_per_px * 0.6) || near(lat, 5.0, lat_per_px * 0.6) {
            image::Rgb([200, 212, 224])
        } else {
            image::Rgb([233, 240, 246])
        }
    });
    let mut out = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut out),
        image::ImageOutputFormat::Png,
    )
    .ok()?;
    Some(out)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cities = [
        ("Berlin", 13.40, 52.52, 3.7),
        ("Paris", 2.35, 48.86, 2.1),
        ("London", -0.13, 51.51, 8.9),
        ("Madrid", -3.70, 40.42, 3.3),
        ("Rome", 12.50, 41.90, 2.8),
        ("Vienna", 16.37, 48.21, 1.9),
        ("Warsaw", 21.01, 52.23, 1.8),
        ("Stockholm", 18.07, 59.33, 1.0),
        ("Lisbon", -9.14, 38.72, 0.5),
        ("Athens", 23.73, 37.98, 0.7),
    ];
    let data = vec![
        (
            "geometry".to_string(),
            cities
                .iter()
                .map(|(_, lon, lat, _)| Value::Str(format!("POINT ({lon} {lat})")))
                .collect::<Vec<_>>(),
        ),
        (
            "city".to_string(),
            cities.iter().map(|c| Value::Str(c.0.to_string())).collect(),
        ),
        (
            "population".to_string(),
            cities.iter().map(|c| Value::Float(c.3)).collect(),
        ),
    ];

    type Fetch = std::sync::Arc<dyn Fn(u32, u32, u32) -> Option<Vec<u8>> + Send + Sync>;
    let (source, fetch): (TileSource, Fetch) = match std::env::var("TILE_DIR") {
        Ok(dir) => (
            TileSource::xyz("{z}/{x}/{y}.png").attribution("Tiles: local cache"),
            std::sync::Arc::new(move |z, x, y| {
                std::fs::read(format!("{dir}/{z}/{x}/{y}.png")).ok()
            }),
        ),
        Err(_) => (
            TileSource::xyz("synthetic").attribution("Synthetic graticule tiles"),
            std::sync::Arc::new(synthetic_tile),
        ),
    };

    // Map units on the axes mean nothing to a reader: drop the tick labels.
    let mut theme = ggplot_rs::theme::presets::theme_minimal();
    theme.axis_text_x.visible = false;
    theme.axis_text_y.visible = false;
    let plot = || {
        let fetch = fetch.clone();
        GGPlot::new(data.clone())
            .aes(Aes::new().label("city").fill("population"))
            .geom_sf_with(GeomSf::default().project(SfProjection::Mercator))
            .annotation_map_tile(source.clone(), move |z, x, y| fetch(z, x, y))
            .coord_sf()
            .scale_fill_viridis_c()
            .title("Map tiles (annotation_map_tile)")
            .theme(theme.clone())
    };
    std::fs::write(
        "map_tiles.svg",
        plot().render_svg_native_with_size(640, 480)?,
    )?;
    println!("wrote map_tiles.svg");
    #[cfg(feature = "plotters")]
    {
        plot().save_with_size("map_tiles.png", 640, 480)?;
        println!("wrote map_tiles.png");
    }
    Ok(())
}
