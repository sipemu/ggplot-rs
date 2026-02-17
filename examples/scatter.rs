use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sepal_length: Vec<f64> = (0..50).map(|i| 4.5 + i as f64 * 0.05).collect();
    let sepal_width: Vec<f64> = (0..50)
        .map(|i| 2.0 + (i as f64 * 0.3).sin() + i as f64 * 0.02)
        .collect();
    let species: Vec<&str> = (0..50)
        .map(|i| match i % 3 {
            0 => "setosa",
            1 => "versicolor",
            _ => "virginica",
        })
        .collect();

    let df = df! {
        "sepal_length" => sepal_length,
        "sepal_width" => sepal_width,
        "species" => species,
    }?;

    GGPlot::new(df)
        .aes(Aes::new().x("sepal_length").y("sepal_width").color("species"))
        .geom_point()
        .title("Iris Scatter Plot")
        .xlab("Sepal Length")
        .ylab("Sepal Width")
        .save("scatter.svg")?;

    println!("Saved scatter.svg");
    Ok(())
}
