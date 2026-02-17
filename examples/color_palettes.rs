use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate grouped scatter data
    let groups = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"];
    let mut x_vals = Vec::new();
    let mut y_vals = Vec::new();
    let mut group_vals = Vec::new();

    for (gi, &group) in groups.iter().enumerate() {
        for j in 0..20 {
            let base_x = gi as f64 * 2.0 + 1.0;
            let base_y = (gi as f64 + 1.0) * 3.0;
            let r = ((gi * 20 + j) * 7 + 13) % 17;
            x_vals.push(base_x + (r as f64 / 17.0 - 0.5) * 2.0);
            y_vals.push(base_y + ((r * 3 + 5) % 11) as f64 / 11.0 * 4.0 - 2.0);
            group_vals.push(group);
        }
    }

    let df = df! {
        "x" => &x_vals,
        "y" => &y_vals,
        "group" => &group_vals,
    }?;

    // Viridis palette
    GGPlot::new(df.clone())
        .aes(Aes::new().x("x").y("y").color("group"))
        .geom_point()
        .scale_color_viridis()
        .title("Viridis Palette")
        .save("palette_viridis.svg")?;

    println!("Saved palette_viridis.svg");

    // Brewer Set1 palette
    GGPlot::new(df.clone())
        .aes(Aes::new().x("x").y("y").color("group"))
        .geom_point()
        .scale_color_brewer(PaletteName::Set1)
        .title("Brewer Set1 Palette")
        .save("palette_brewer_set1.svg")?;

    println!("Saved palette_brewer_set1.svg");

    // Brewer Dark2 palette
    GGPlot::new(df.clone())
        .aes(Aes::new().x("x").y("y").color("group"))
        .geom_point()
        .scale_color_brewer(PaletteName::Dark2)
        .title("Brewer Dark2 Palette")
        .save("palette_brewer_dark2.svg")?;

    println!("Saved palette_brewer_dark2.svg");

    // Manual colors
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("group"))
        .geom_point()
        .scale_color_manual(vec![
            ("Alpha", RGBAColor::new(255, 0, 0)),
            ("Beta", RGBAColor::new(0, 180, 0)),
            ("Gamma", RGBAColor::new(0, 0, 255)),
            ("Delta", RGBAColor::new(255, 165, 0)),
            ("Epsilon", RGBAColor::new(128, 0, 128)),
        ])
        .title("Manual Color Scale")
        .save("palette_manual.svg")?;

    println!("Saved palette_manual.svg");
    Ok(())
}
