use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate two overlapping distributions using deterministic pseudo-random values
    let value: Vec<f64> = (0..200_u64)
        .map(|i| {
            let r = ((i.wrapping_mul(1103515245).wrapping_add(12345)) % (1 << 16)) as f64
                / (1u64 << 16) as f64;
            if i < 100 {
                3.0 + r * 4.0
            } else {
                5.0 + r * 4.0
            }
        })
        .collect();
    let group: Vec<&str> = (0..200)
        .map(|i| if i < 100 { "Group A" } else { "Group B" })
        .collect();

    let df = df! {
        "value" => value,
        "group" => group,
    }?;

    GGPlot::new(df)
        .aes(Aes::new().x("value").color("group"))
        .geom_density()
        .title("Density Plot by Group")
        .xlab("Value")
        .ylab("Density")
        .save("density.svg")?;

    println!("Saved density.svg");
    Ok(())
}
