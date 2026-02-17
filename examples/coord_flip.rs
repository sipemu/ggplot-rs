use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "language" => ["Rust", "Python", "JavaScript", "Go", "TypeScript", "Java", "C++"],
        "satisfaction" => [92.0, 88.0, 85.0, 78.0, 82.0, 70.0, 75.0],
    }?;

    // Horizontal bar chart using coord_flip
    GGPlot::new(df)
        .aes(Aes::new().x("language").y("satisfaction"))
        .geom_col()
        .coord_flip()
        .title("Developer Satisfaction by Language")
        .xlab("Language")
        .ylab("Satisfaction Score")
        .save("coord_flip.svg")?;

    println!("Saved coord_flip.svg");
    Ok(())
}
