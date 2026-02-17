use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate noisy sine wave data
    let x: Vec<f64> = (0..80).map(|i| (i as f64) * 0.1).collect();
    let y: Vec<f64> = (0..80)
        .map(|i| {
            let xv = (i as f64) * 0.1;
            let noise = ((i * 17 + 3) % 11) as f64 / 11.0 - 0.5; // deterministic pseudo-noise
            (xv * 0.8).sin() * 2.0 + noise * 1.5
        })
        .collect();

    let df = df! {
        "x" => &x,
        "y" => &y,
    }?;

    // Linear smooth (default)
    GGPlot::new(df.clone())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth()
        .title("Linear Smooth (method = lm)")
        .save("smooth_lm.svg")?;

    println!("Saved smooth_lm.svg");

    // LOESS smooth
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth::default().loess(0.3))
        .title("LOESS Smooth (span = 0.3)")
        .save("smooth_loess.svg")?;

    println!("Saved smooth_loess.svg");
    Ok(())
}
