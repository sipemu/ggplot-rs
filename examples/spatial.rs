//! `geom_sf` — a simple choropleth from WKT geometry.
//!
//! Run with: `cargo run --features sf --example spatial`

use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A little "country" split into provinces (hand-drawn WKT polygons). Any
    // source of WKT works — a `geometry` column of `MULTIPOLYGON`s from a
    // shapefile/GeoJSON export drops straight in.
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
        .save("spatial.png")?;

    println!("wrote spatial.png");
    Ok(())
}
