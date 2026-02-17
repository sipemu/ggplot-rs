use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate data with two grouping variables
    let sepal_length: Vec<f64> = (0..120)
        .map(|i| 4.0 + (i as f64) * 0.04 + (i as f64 * 0.5).sin() * 0.3)
        .collect();
    let sepal_width: Vec<f64> = (0..120)
        .map(|i| 2.0 + (i as f64) * 0.015 + (i as f64 * 0.3).cos() * 0.4)
        .collect();
    let species: Vec<&str> = (0..120)
        .map(|i| match i % 3 {
            0 => "setosa",
            1 => "versicolor",
            _ => "virginica",
        })
        .collect();
    let region: Vec<&str> = (0..120)
        .map(|i| if i % 2 == 0 { "North" } else { "South" })
        .collect();

    let df = df! {
        "sepal_length" => sepal_length,
        "sepal_width" => sepal_width,
        "species" => species,
        "region" => region,
    }?;

    // facet_wrap: one variable, automatic grid layout
    GGPlot::new(df.clone())
        .aes(Aes::new().x("sepal_length").y("sepal_width").color("species"))
        .geom_point()
        .facet_wrap("species", Some(2))
        .title("Facet Wrap by Species")
        .xlab("Sepal Length")
        .ylab("Sepal Width")
        .save("facet_wrap.svg")?;

    println!("Saved facet_wrap.svg");

    // facet_grid: two variables, row ~ col layout
    GGPlot::new(df)
        .aes(Aes::new().x("sepal_length").y("sepal_width").color("species"))
        .geom_point()
        .facet_grid(Some("region"), Some("species"))
        .title("Facet Grid: Region ~ Species")
        .xlab("Sepal Length")
        .ylab("Sepal Width")
        .save("facet_grid.svg")?;

    println!("Saved facet_grid.svg");
    Ok(())
}
