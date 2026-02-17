use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate approximate normal distribution using central limit theorem
    let values: Vec<f64> = (0..1000)
        .map(|i: i32| {
            // Sum of pseudo-random uniform values → approx normal by CLT
            let r1 = ((i * 1237 + 5678) % 1000) as f64 / 1000.0;
            let r2 = ((i * 8731 + 4321) % 1000) as f64 / 1000.0;
            let r3 = ((i * 4567 + 8901) % 1000) as f64 / 1000.0;
            let r4 = ((i * 6543 + 2109) % 1000) as f64 / 1000.0;
            let r5 = ((i * 3571 + 7654) % 1000) as f64 / 1000.0;
            let r6 = ((i * 9137 + 3456) % 1000) as f64 / 1000.0;
            // Sum of 6 uniforms: mean=3, std≈0.745; normalize to mean=0, std≈1.5
            (r1 + r2 + r3 + r4 + r5 + r6 - 3.0) * 2.0
        })
        .collect();

    let df = df! {
        "measurement" => values,
    }?;

    GGPlot::new(df)
        .aes(Aes::new().x("measurement"))
        .geom_histogram_with(GeomHistogram {
            bins: 25,
            ..Default::default()
        })
        .title("Distribution of Measurements")
        .xlab("Value")
        .ylab("Frequency")
        .save("histogram.svg")?;

    println!("Saved histogram.svg");
    Ok(())
}
