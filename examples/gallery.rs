//! Generates the plot gallery shown in the README.
//!
//! Run with: `cargo run --example gallery`
//! Writes PNGs to `assets/gallery/`.

use ggplot_rs::prelude::*;
use polars::prelude::*;
use std::f64::consts::PI;

const W: u32 = 640;
const H: u32 = 480;
// Smaller thumbnails for the theme gallery.
const TW: u32 = 480;
const TH: u32 = 340;

fn out(name: &str) -> String {
    format!("assets/gallery/{name}.png")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("assets/gallery")?;

    scatter()?;
    smooth()?;
    histogram()?;
    bar()?;
    boxplot()?;
    violin()?;
    continuous_color()?;
    facet()?;
    density()?;
    contour_filled()?;
    hexbin()?;
    heatmap()?;
    jitter()?;
    ribbon()?;
    area_stack()?;
    polar()?;
    ecdf()?;
    qq()?;
    #[cfg(feature = "sf")]
    spatial()?;
    themes()?;

    println!("Gallery written to assets/gallery/");
    Ok(())
}

/// Grouped scatter with a qualitative Brewer palette.
fn scatter() -> Result<(), Box<dyn std::error::Error>> {
    let n = 150;
    let x: Vec<f64> = (0..n).map(|i| 4.5 + i as f64 * 0.02).collect();
    let y: Vec<f64> = (0..n)
        .map(|i| 2.5 + (i as f64 * 0.15).sin() + (i % 3) as f64 * 0.6)
        .collect();
    let species: Vec<&str> = (0..n)
        .map(|i| ["setosa", "versicolor", "virginica"][i % 3])
        .collect();

    let df = df! { "x" => x, "y" => y, "species" => species }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("species"))
        .geom_point()
        .scale_color_brewer(PaletteName::Set1)
        .title("Grouped Scatter")
        .xlab("Sepal Length")
        .ylab("Sepal Width")
        .theme_minimal()
        .save_with_size(&out("scatter"), W, H)?;
    Ok(())
}

/// Points overlaid with a LOESS trend line and confidence band.
fn smooth() -> Result<(), Box<dyn std::error::Error>> {
    let n = 120;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 * 0.1;
            (t * 0.6).sin() * 3.0 + t * 0.2 + ((i * 7919 % 100) as f64 / 100.0 - 0.5) * 1.5
        })
        .collect();

    let df = df! { "x" => x, "y" => y }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth {
            method: SmoothMethod::Loess { span: 0.5 },
            ..Default::default()
        })
        .title("LOESS Smoothing")
        .xlab("x")
        .ylab("y")
        .theme_bw()
        .save_with_size(&out("smooth"), W, H)?;
    Ok(())
}

/// Histogram of an approximately-normal sample.
fn histogram() -> Result<(), Box<dyn std::error::Error>> {
    let values: Vec<f64> = (0..1500)
        .map(|i: i32| {
            let r: f64 = (0..6)
                .map(|k| ((i * (1237 + k * 311) + 5678) % 1000) as f64 / 1000.0)
                .sum();
            (r - 3.0) * 2.0
        })
        .collect();

    let df = df! { "measurement" => values }?;
    GGPlot::new(df)
        .aes(Aes::new().x("measurement"))
        .geom_histogram_with(GeomHistogram {
            bins: 30,
            ..Default::default()
        })
        .title("Histogram")
        .xlab("Value")
        .ylab("Count")
        .theme_minimal()
        .save_with_size(&out("histogram"), W, H)?;
    Ok(())
}

/// Bar chart of category counts with a fill palette.
fn bar() -> Result<(), Box<dyn std::error::Error>> {
    let mut fruit: Vec<&str> = Vec::new();
    for (f, c) in [
        ("Apple", 8),
        ("Banana", 5),
        ("Cherry", 11),
        ("Date", 3),
        ("Elder", 7),
    ] {
        for _ in 0..c {
            fruit.push(f);
        }
    }
    let df = df! { "fruit" => fruit }?;
    GGPlot::new(df)
        .aes(Aes::new().x("fruit").fill("fruit"))
        .geom_bar()
        .scale_fill_brewer(PaletteName::Set2)
        .title("Bar Chart")
        .xlab("Fruit")
        .ylab("Count")
        .theme_minimal()
        .save_with_size(&out("bar"), W, H)?;
    Ok(())
}

/// Grouped boxplots.
fn boxplot() -> Result<(), Box<dyn std::error::Error>> {
    let n = 240;
    let group: Vec<&str> = (0..n).map(|i| ["A", "B", "C", "D"][i % 4]).collect();
    let value: Vec<f64> = (0..n)
        .map(|i| {
            let base = (i % 4) as f64 * 1.5;
            base + (i as f64 * 0.4).sin() * 1.2 + ((i * 6151 % 100) as f64 / 100.0 - 0.5) * 2.0
        })
        .collect();

    let df = df! { "group" => group, "value" => value }?;
    GGPlot::new(df)
        .aes(Aes::new().x("group").y("value"))
        .geom_boxplot_with(GeomBoxplot {
            fill: (70, 130, 180),
            ..Default::default()
        })
        .title("Boxplot")
        .xlab("Group")
        .ylab("Value")
        .theme_bw()
        .save_with_size(&out("boxplot"), W, H)?;
    Ok(())
}

/// Violin plots of grouped distributions.
fn violin() -> Result<(), Box<dyn std::error::Error>> {
    let n = 360;
    let group: Vec<&str> = (0..n).map(|i| ["X", "Y", "Z"][i % 3]).collect();
    let value: Vec<f64> = (0..n)
        .map(|i| {
            let g = (i % 3) as f64;
            g * 2.0 + (i as f64 * 0.5).sin() * 1.5 + ((i * 4231 % 100) as f64 / 100.0 - 0.5) * 2.5
        })
        .collect();

    let df = df! { "group" => group, "value" => value }?;
    GGPlot::new(df)
        .aes(Aes::new().x("group").y("value").fill("group"))
        .geom_violin()
        .scale_fill_brewer(PaletteName::Accent)
        .title("Violin")
        .xlab("Group")
        .ylab("Value")
        .theme_minimal()
        .save_with_size(&out("violin"), W, H)?;
    Ok(())
}

/// Spiral scatter coloured by a continuous variable (viridis).
fn continuous_color() -> Result<(), Box<dyn std::error::Error>> {
    let n = 400;
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 * 0.05;
            t.cos() * (1.0 + t * 0.12)
        })
        .collect();
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 * 0.05;
            t.sin() * (1.0 + t * 0.12)
        })
        .collect();
    let z: Vec<f64> = (0..n).map(|i| i as f64 * 0.05).collect();

    let df = df! { "x" => x, "y" => y, "z" => z }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .scale_color_viridis_c()
        .title("Continuous Color (viridis)")
        .xlab("x")
        .ylab("y")
        .theme_minimal()
        .save_with_size(&out("continuous_color"), W, H)?;
    Ok(())
}

/// Faceted scatter, one panel per group.
fn facet() -> Result<(), Box<dyn std::error::Error>> {
    let n = 180;
    let x: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).cos() * 3.0).collect();
    let y: Vec<f64> = (0..n)
        .map(|i| (i as f64 * 0.1).sin() * 3.0 + (i % 3) as f64)
        .collect();
    let species: Vec<&str> = (0..n)
        .map(|i| ["setosa", "versicolor", "virginica"][i % 3])
        .collect();

    let df = df! { "x" => x, "y" => y, "species" => species }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("species"))
        .geom_point()
        .facet_wrap("species", Some(3))
        .scale_color_brewer(PaletteName::Set1)
        .title("Facet Wrap")
        .xlab("x")
        .ylab("y")
        .theme_bw()
        .save_with_size(&out("facet"), W, H)?;
    Ok(())
}

/// Overlapping density curves by group.
fn density() -> Result<(), Box<dyn std::error::Error>> {
    let n = 600;
    let group: Vec<&str> = (0..n).map(|i| ["Group 1", "Group 2"][i % 2]).collect();
    let value: Vec<f64> = (0..n)
        .map(|i| {
            let shift = (i % 2) as f64 * 2.5;
            let t = i as f64 * 0.05;
            shift + (t.sin() + (t * 1.7).cos()) + ((i * 3319 % 100) as f64 / 100.0 - 0.5) * PI
        })
        .collect();

    let df = df! { "value" => value, "group" => group }?;
    GGPlot::new(df)
        .aes(Aes::new().x("value").fill("group").color("group"))
        .geom_density()
        .scale_fill_brewer(PaletteName::Set1)
        .scale_color_brewer(PaletteName::Set1)
        .title("Density by Group")
        .xlab("Value")
        .ylab("Density")
        .theme_minimal()
        .save_with_size(&out("density"), W, H)?;
    Ok(())
}

/// Filled contour bands from a gridded surface.
fn contour_filled() -> Result<(), Box<dyn std::error::Error>> {
    let (mut x, mut y, mut z) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..40 {
        for j in 0..40 {
            let xv = i as f64 * 0.25 - 5.0;
            let yv = j as f64 * 0.25 - 5.0;
            x.push(xv);
            y.push(yv);
            z.push((xv * 0.6).sin() * (yv * 0.6).cos() + (-(xv * xv + yv * yv) / 30.0).exp());
        }
    }
    let df = df! { "x" => x, "y" => y, "z" => z }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y"))
        .geom_contour_filled()
        .scale_fill_viridis_c()
        .title("Filled Contours")
        .theme_minimal()
        .save_with_size(&out("contour_filled"), W, H)?;
    Ok(())
}

/// Hexagonal binning of a 2-D point cloud.
fn hexbin() -> Result<(), Box<dyn std::error::Error>> {
    let n = 4000;
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64;
            (t * 0.017).sin() * 2.0 + ((i * 7919 % 1000) as f64 / 1000.0 - 0.5) * 3.0
        })
        .collect();
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64;
            (t * 0.017).cos() * 2.0 + ((i * 6323 % 1000) as f64 / 1000.0 - 0.5) * 3.0
        })
        .collect();
    let df = df! { "x" => x, "y" => y }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y"))
        .geom_hex()
        .scale_fill_viridis_c()
        .title("Hex Binning")
        .theme_minimal()
        .save_with_size(&out("hexbin"), W, H)?;
    Ok(())
}

/// Heatmap of a gridded value with `geom_tile`.
fn heatmap() -> Result<(), Box<dyn std::error::Error>> {
    let (mut x, mut y, mut fill) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..14 {
        for j in 0..14 {
            x.push(i as f64);
            y.push(j as f64);
            fill.push((i as f64 * 0.5).sin() * (j as f64 * 0.5).cos());
        }
    }
    let df = df! { "x" => x, "y" => y, "fill" => fill }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").fill("fill"))
        .geom_tile()
        .scale_fill_viridis_c()
        .title("Heatmap")
        .theme_minimal()
        .save_with_size(&out("heatmap"), W, H)?;
    Ok(())
}

/// Jittered categorical scatter (`geom_jitter`).
fn jitter() -> Result<(), Box<dyn std::error::Error>> {
    let n = 300;
    let group: Vec<&str> = (0..n).map(|i| ["Control", "Low", "High"][i % 3]).collect();
    let value: Vec<f64> = (0..n)
        .map(|i| {
            let base = (i % 3) as f64 * 1.5;
            base + ((i * 5701 % 1000) as f64 / 1000.0 - 0.5) * 2.0
        })
        .collect();
    let df = df! { "group" => group, "value" => value }?;
    GGPlot::new(df)
        .aes(Aes::new().x("group").y("value").color("group"))
        .geom_jitter()
        .scale_color_brewer(PaletteName::Dark2)
        .title("Jittered Points")
        .theme_minimal()
        .save_with_size(&out("jitter"), W, H)?;
    Ok(())
}

/// A confidence band (`geom_ribbon`) under a line.
fn ribbon() -> Result<(), Box<dyn std::error::Error>> {
    let n = 80;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.15).collect();
    let y: Vec<f64> = x.iter().map(|v| v.sin() + v * 0.1).collect();
    let ymin: Vec<f64> = y.iter().map(|v| v - 0.4).collect();
    let ymax: Vec<f64> = y.iter().map(|v| v + 0.4).collect();
    let df = df! { "x" => x, "y" => y, "ymin" => ymin, "ymax" => ymax }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").ymin("ymin").ymax("ymax"))
        .geom_ribbon()
        .geom_line()
        .primary_color((49, 130, 189))
        .title("Ribbon + Line")
        .theme_minimal()
        .save_with_size(&out("ribbon"), W, H)?;
    Ok(())
}

/// Stacked areas by group.
fn area_stack() -> Result<(), Box<dyn std::error::Error>> {
    let n = 40;
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut g = Vec::new();
    for grp in ["North", "South", "East"] {
        for i in 0..n {
            x.push(i as f64);
            let base = match grp {
                "North" => 2.0,
                "South" => 3.0,
                _ => 1.5,
            };
            y.push(base + (i as f64 * 0.2).sin().abs() * base);
            g.push(grp);
        }
    }
    let df = df! { "x" => x, "y" => y, "g" => g }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").fill("g"))
        .geom_area()
        .scale_fill_brewer(PaletteName::Set2)
        .title("Stacked Area")
        .theme_minimal()
        .save_with_size(&out("area"), W, H)?;
    Ok(())
}

/// Radial "rose" chart — bars in polar coordinates (`coord_polar`).
fn polar() -> Result<(), Box<dyn std::error::Error>> {
    let day = vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let value = vec![18.0, 23.0, 20.0, 28.0, 33.0, 45.0, 39.0];
    let df = df! { "day" => day, "value" => value }?;
    // Hide the angular axis text (it overlaps at the base of a rose chart);
    // the legend already maps day → colour.
    let mut theme = theme_minimal();
    theme.axis_text_x.visible = false;
    GGPlot::new(df)
        .aes(Aes::new().x("day").y("value").fill("day"))
        .geom_col()
        .coord_polar()
        .scale_fill_brewer(PaletteName::Set3)
        .title("Polar Bars (rose chart)")
        .theme(theme)
        .save_with_size(&out("polar"), W, H)?;
    Ok(())
}

/// Empirical cumulative distribution, drawn as a step padded to the panel edges.
fn ecdf() -> Result<(), Box<dyn std::error::Error>> {
    let n = 300;
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 * 0.05;
            (t.sin() + (t * 1.7).cos()) * 1.5 + ((i * 3319 % 100) as f64 / 100.0 - 0.5) * 2.0
        })
        .collect();
    let df = df! { "x" => x }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x"))
        .geom_step()
        .stat(ggplot_rs::stat::ecdf::StatEcdf)
        .primary_color((49, 130, 189))
        .title("Empirical CDF (stat_ecdf)")
        .xlab("Value")
        .ylab("F(x)")
        .theme_minimal()
        .save_with_size(&out("ecdf"), W, H)?;
    Ok(())
}

/// Normal quantile-quantile plot with a reference line (`geom_qq`).
fn qq() -> Result<(), Box<dyn std::error::Error>> {
    let n = 200;
    // A heavy-tailed sample so the points bow away from the reference line.
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let u = ((i * 2749 + 13) % 1000) as f64 / 1000.0 - 0.5;
            u * u * u * 30.0 + u * 4.0
        })
        .collect();
    let df = df! { "y" => y }?;
    GGPlot::new(df)
        .aes(Aes::new().y("y"))
        .geom_qq()
        .geom_qq_line()
        .primary_color((197, 90, 17))
        .title("Q-Q Plot (stat_qq)")
        .xlab("Theoretical")
        .ylab("Sample")
        .theme_bw()
        .save_with_size(&out("qq"), W, H)?;
    Ok(())
}

/// A simple-features choropleth (`geom_sf`) — provinces filled by a value.
#[cfg(feature = "sf")]
fn spatial() -> Result<(), Box<dyn std::error::Error>> {
    let geometry = vec![
        "POLYGON ((0 0, 3 0, 3 2, 1 2.5, 0 2, 0 0))",
        "POLYGON ((3 0, 6 0, 6 3, 3 2, 3 0))",
        "POLYGON ((0 2, 1 2.5, 3 2, 3 5, 0 5, 0 2))",
        "POLYGON ((3 2, 6 3, 6 5, 3 5, 3 2))",
        "POLYGON ((6 0, 9 1, 8 4, 6 3, 6 0))",
        "POLYGON ((6 3, 8 4, 9 6, 6 5, 6 3))",
        "POLYGON ((0 5, 3 5, 4 7, 1 8, 0 7, 0 5))",
        "POLYGON ((3 5, 6 5, 6 7, 4 7, 3 5))",
    ];
    let population = vec![4.2, 7.8, 3.1, 9.5, 5.4, 2.7, 6.6, 8.9];
    let df = df! { "geometry" => geometry, "population" => population }?;

    // Clean map look: drop the axis text (coordinates aren't meaningful here).
    let mut theme = theme_minimal();
    theme.axis_text_x.visible = false;
    theme.axis_text_y.visible = false;
    GGPlot::new(df)
        .aes(Aes::new().fill("population"))
        .geom_sf()
        .scale_fill_viridis_c()
        .title("Choropleth")
        .theme(theme)
        .save_with_size(&out("spatial"), W, H)?;
    Ok(())
}

/// The same plot rendered under every built-in theme.
fn themes() -> Result<(), Box<dyn std::error::Error>> {
    let n = 90;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = (0..n)
        .map(|i| (i as f64 * 0.1).sin() + (i % 3) as f64 * 0.5)
        .collect();
    let g: Vec<&str> = (0..n).map(|i| ["A", "B", "C"][i % 3]).collect();
    let df = df! { "x" => x, "y" => y, "g" => g }?;

    type ThemeFn = fn() -> Theme;
    let variants: [(&str, ThemeFn); 8] = [
        ("gray", theme_gray),
        ("bw", theme_bw),
        ("minimal", theme_minimal),
        ("classic", theme_classic),
        ("light", theme_light),
        ("dark", theme_dark),
        ("linedraw", theme_linedraw),
        ("void", theme_void),
    ];
    for (name, make) in variants {
        GGPlot::new(df.clone())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_point()
            .theme(make())
            .scale_color_brewer(PaletteName::Dark2)
            .title(&format!("theme_{name}"))
            .save_with_size(&out(&format!("theme_{name}")), TW, TH)?;
    }
    Ok(())
}
