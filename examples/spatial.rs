//! `geom_sf` — plot simple-features geometry, with and without a projection.
//!
//! Run: `cargo run --features sf --example spatial`
//!
//! Any source of WKT works. To plot a **shapefile** (or GeoJSON / GeoPackage /
//! FlatGeobuf), let DuckDB's `spatial` extension do the reading and hand
//! ggplot-rs WKT via `ST_AsText` — one shell command with the CLI:
//!
//! ```sh
//! ggplot-rs --spatial \
//!   --sql "SELECT ST_AsText(geom) AS geometry, name, pop_est
//!          FROM ST_Read('ne_110m_admin_0_countries.shp')" \
//!   --geom sf --fill pop_est --projection mercator -o world.png
//! ```

use ggplot_rs::geom::sf::GeomSf;
use ggplot_rs::prelude::*;
use ggplot_rs::spatial::SfProjection;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    choropleth()?;
    mercator_grid()?;
    Ok(())
}

/// A little "country" split into provinces, filled by population.
fn choropleth() -> Result<(), Box<dyn std::error::Error>> {
    let geometry = vec![
        "POLYGON ((0 0, 3 0, 3 2, 1 2.5, 0 2, 0 0))",
        "POLYGON ((3 0, 6 0, 6 3, 3 2, 3 0))",
        "POLYGON ((0 2, 1 2.5, 3 2, 3 5, 0 5, 0 2))",
        "POLYGON ((3 2, 6 3, 6 5, 3 5, 3 2))",
        "POLYGON ((6 0, 9 1, 8 4, 6 3, 6 0))",
        "POLYGON ((6 3, 8 4, 9 6, 6 5, 6 3))",
    ];
    let province = vec!["North", "East", "West", "Center", "Coast", "Cape"];
    let population = vec![4.2, 7.8, 3.1, 9.5, 5.4, 2.7];
    let df = df! {
        "geometry"   => geometry,
        "province"   => province,
        "population" => population,
    }?;

    GGPlot::new(df)
        .aes(Aes::new().fill("population"))
        .geom_sf()
        .scale_fill_viridis_c()
        .title("geom_sf — population by province")
        .theme_minimal()
        .save("spatial_choropleth.png")?;
    println!("wrote spatial_choropleth.png");
    Ok(())
}

/// A grid of equal 20°×20° lon/lat cells under Web Mercator — the poleward rows
/// stretch, the way they do on a real Mercator world map.
fn mercator_grid() -> Result<(), Box<dyn std::error::Error>> {
    let (mut geometry, mut lat_band) = (Vec::new(), Vec::new());
    for lat in (0..80).step_by(20) {
        for lon in (-40..60).step_by(20) {
            geometry.push(format!(
                "POLYGON (({lo} {la}, {lo1} {la}, {lo1} {la1}, {lo} {la1}, {lo} {la}))",
                lo = lon,
                lo1 = lon + 20,
                la = lat,
                la1 = lat + 20,
            ));
            lat_band.push(lat as f64);
        }
    }
    let df = df! { "geometry" => geometry, "lat_band" => lat_band }?;

    GGPlot::new(df)
        .aes(Aes::new().fill("lat_band"))
        .geom_sf_with(GeomSf::default().project(SfProjection::Mercator))
        .coord_sf()
        .scale_fill_viridis_c()
        .title("Web Mercator — equal 20° cells stretch poleward")
        .theme_minimal()
        .save("spatial_mercator.png")?;
    println!("wrote spatial_mercator.png");
    Ok(())
}
