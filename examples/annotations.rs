use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sales data with a notable spike
    let month: Vec<f64> = (1..=12).map(|i| i as f64).collect();
    let sales = vec![120.0, 135.0, 150.0, 180.0, 210.0, 310.0, 280.0, 250.0, 190.0, 170.0, 155.0, 140.0];

    let df = df! {
        "month" => month,
        "sales" => sales,
    }?;

    GGPlot::new(df)
        .aes(Aes::new().x("month").y("sales"))
        .geom_line()
        .geom_point()
        // Highlight the peak region
        .annotate_rect(4.5, 7.5, 100.0, 320.0)
        // Label the peak
        .annotate_text("Summer Peak", 6.0, 330.0)
        // Draw an arrow-like segment pointing to the max
        .annotate_segment(7.5, 330.0, 6.2, 312.0)
        .title("Monthly Sales with Annotations")
        .xlab("Month")
        .ylab("Sales ($)")
        .save("annotations.svg")?;

    println!("Saved annotations.svg");
    Ok(())
}
