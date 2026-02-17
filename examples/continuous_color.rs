use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate scatter data with a continuous variable for color
    let x: Vec<f64> = (0..200).map(|i| { let t = i as f64 * 0.05; t.cos() * (1.0 + t * 0.3) }).collect();
    let y: Vec<f64> = (0..200).map(|i| { let t = i as f64 * 0.05; t.sin() * (1.0 + t * 0.3) }).collect();
    let z: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();

    let df = df! {
        "x" => &x,
        "y" => &y,
        "z" => &z,
    }?;

    // Default blue-to-red gradient
    GGPlot::new(df.clone())
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .title("Continuous Color (default gradient)")
        .xlab("X")
        .ylab("Y")
        .save("continuous_color.svg")?;

    println!("Saved continuous_color.svg");

    // Custom gradient: dark blue to yellow
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .scale_color_gradient(
            RGBAColor::new(10, 30, 100),
            RGBAColor::new(255, 230, 50),
        )
        .title("Continuous Color (custom gradient)")
        .xlab("X")
        .ylab("Y")
        .save("continuous_color_custom.svg")?;

    println!("Saved continuous_color_custom.svg");
    Ok(())
}
