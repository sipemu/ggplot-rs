use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Exponential growth data (e.g., population, bacteria count)
    let year: Vec<f64> = (0..30).map(|i| 1990.0 + i as f64).collect();
    let count: Vec<f64> = (0..30)
        .map(|i| {
            let base = 100.0 * (1.15_f64).powi(i); // 15% growth per year
            let noise = ((i * 37 + 7) % 13) as f64 / 13.0 * 0.2 + 0.9;
            base * noise
        })
        .collect();

    let df = df! {
        "year" => &year,
        "count" => &count,
    }?;

    // Linear scale — exponential curve
    GGPlot::new(df.clone())
        .aes(Aes::new().x("year").y("count"))
        .geom_point()
        .geom_line()
        .title("Exponential Growth (Linear Scale)")
        .xlab("Year")
        .ylab("Count")
        .save("log_scale_linear.svg")?;

    println!("Saved log_scale_linear.svg");

    // Log10 y-axis — should appear roughly linear
    GGPlot::new(df)
        .aes(Aes::new().x("year").y("count"))
        .geom_point()
        .geom_line()
        .scale_y_log10()
        .title("Exponential Growth (Log10 Y Scale)")
        .xlab("Year")
        .ylab("Count (log10)")
        .save("log_scale_log10.svg")?;

    println!("Saved log_scale_log10.svg");
    Ok(())
}
