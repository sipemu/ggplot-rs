use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "fruit" => ["Apple", "Apple", "Apple", "Banana", "Banana",
                     "Cherry", "Cherry", "Cherry", "Cherry", "Date"],
    }?;

    GGPlot::new(df)
        .aes(Aes::new().x("fruit"))
        .geom_bar()
        .title("Fruit Counts")
        .xlab("Fruit")
        .ylab("Count")
        .save("bar_chart.svg")?;

    println!("Saved bar_chart.svg");
    Ok(())
}
