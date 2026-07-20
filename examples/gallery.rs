//! Generates the plot gallery shown in the README.
//!
//! Run with: `cargo run --features sf --example gallery`
//! Writes PNGs to `assets/gallery/`.
//!
//! Data is drawn from a seeded RNG with realistic distributions so each plot
//! resembles a genuine analysis (natural point clouds, real group differences)
//! rather than an obviously-synthetic wave.

use ggplot_rs::prelude::*;
use polars::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const W: u32 = 640;
const H: u32 = 480;
// Smaller thumbnails for the theme gallery.
const TW: u32 = 480;
const TH: u32 = 340;

fn out(name: &str) -> String {
    format!("assets/gallery/{name}.png")
}

fn seeded(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

/// Approximate standard-normal draw (Irwin–Hall: sum of 12 uniforms − 6).
fn randn(r: &mut StdRng) -> f64 {
    (0..12).map(|_| r.gen::<f64>()).sum::<f64>() - 6.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Iris-like sample: 50 flowers per species with realistic sepal length/width
/// (means/SDs close to Fisher's iris), so the three species form distinct,
/// naturally-scattered clusters.
fn iris(seed: u64) -> (Vec<f64>, Vec<f64>, Vec<&'static str>) {
    // (species, len mean, len sd, width mean, width sd)
    let params = [
        ("setosa", 5.01, 0.35, 3.43, 0.38),
        ("versicolor", 5.94, 0.52, 2.77, 0.31),
        ("virginica", 6.59, 0.64, 2.97, 0.32),
    ];
    let mut r = seeded(seed);
    let (mut x, mut y, mut sp) = (Vec::new(), Vec::new(), Vec::new());
    for (name, lm, ls, wm, ws) in params {
        for _ in 0..50 {
            x.push(round2(lm + randn(&mut r) * ls));
            y.push(round2(wm + randn(&mut r) * ws));
            sp.push(name);
        }
    }
    (x, y, sp)
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

/// Grouped scatter of iris sepal dimensions with a qualitative palette.
fn scatter() -> Result<(), Box<dyn std::error::Error>> {
    let (x, y, species) = iris(1);
    let df = df! { "x" => x, "y" => y, "species" => species }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("species"))
        .geom_point()
        .scale_color_brewer(PaletteName::Set1)
        .title("Iris Sepal Dimensions")
        .xlab("Sepal Length (cm)")
        .ylab("Sepal Width (cm)")
        .theme_minimal()
        .save_with_size(&out("scatter"), W, H)?;
    Ok(())
}

/// Points overlaid with a LOESS trend line and confidence band.
fn smooth() -> Result<(), Box<dyn std::error::Error>> {
    let n = 120;
    let mut r = seeded(7);
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&t| (t * 0.6).sin() * 3.0 + t * 0.2 + randn(&mut r) * 0.8)
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

/// Histogram of a normally-distributed sample.
fn histogram() -> Result<(), Box<dyn std::error::Error>> {
    let mut r = seeded(11);
    let values: Vec<f64> = (0..1500).map(|_| randn(&mut r) * 2.0 + 10.0).collect();

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

/// Grouped boxplots — four treatments with distinct medians and spreads.
fn boxplot() -> Result<(), Box<dyn std::error::Error>> {
    // (group, mean, sd)
    let params = [("A", 4.0, 0.8), ("B", 6.2, 1.3), ("C", 5.4, 0.6), ("D", 7.1, 1.0)];
    let mut r = seeded(2);
    let (mut group, mut value) = (Vec::new(), Vec::new());
    for (name, mean, sd) in params {
        for _ in 0..60 {
            group.push(name);
            value.push(round2(mean + randn(&mut r) * sd));
        }
    }

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

/// Violin plots — bimodal distributions a boxplot would hide.
fn violin() -> Result<(), Box<dyn std::error::Error>> {
    // Each group mixes two normals so the violin shows two lobes.
    let params = [
        ("X", -2.0, 2.5),
        ("Y", -1.0, 3.5),
        ("Z", 0.0, 4.0),
    ];
    let mut r = seeded(3);
    let (mut group, mut value) = (Vec::new(), Vec::new());
    for (name, lo, hi) in params {
        for k in 0..120 {
            group.push(name);
            let center = if k % 2 == 0 { lo } else { hi };
            value.push(round2(center + randn(&mut r) * 0.7));
        }
    }

    let df = df! { "group" => group, "value" => value }?;
    GGPlot::new(df)
        .aes(Aes::new().x("group").y("value").fill("group"))
        .geom_violin()
        .scale_fill_brewer(PaletteName::Accent)
        .title("Violin (bimodal groups)")
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

/// Faceted iris scatter, one panel per species.
fn facet() -> Result<(), Box<dyn std::error::Error>> {
    let (x, y, species) = iris(4);
    let df = df! { "x" => x, "y" => y, "species" => species }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").color("species"))
        .geom_point()
        .facet_wrap("species", Some(3))
        .scale_color_brewer(PaletteName::Set1)
        .title("Facet Wrap")
        .xlab("Sepal Length (cm)")
        .ylab("Sepal Width (cm)")
        .theme_bw()
        .save_with_size(&out("facet"), W, H)?;
    Ok(())
}

/// Overlapping density curves for two separated groups.
fn density() -> Result<(), Box<dyn std::error::Error>> {
    let mut r = seeded(5);
    let (mut value, mut group) = (Vec::new(), Vec::new());
    for _ in 0..400 {
        value.push(round2(randn(&mut r) * 1.0 - 1.0));
        group.push("Group 1");
    }
    for _ in 0..400 {
        value.push(round2(randn(&mut r) * 1.2 + 2.0));
        group.push("Group 2");
    }

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

/// Hexagonal binning of a dense bivariate-normal cloud.
fn hexbin() -> Result<(), Box<dyn std::error::Error>> {
    let n = 6000;
    let mut r = seeded(9);
    // Correlated 2-D Gaussian: y shares part of x's draw.
    let (mut x, mut y) = (Vec::new(), Vec::new());
    for _ in 0..n {
        let a = randn(&mut r);
        let b = randn(&mut r);
        x.push(a * 2.0);
        y.push(a * 1.1 + b * 1.4);
    }
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

/// Jittered dose–response scatter (`geom_jitter`).
fn jitter() -> Result<(), Box<dyn std::error::Error>> {
    let params = [("Control", 2.0, 0.6), ("Low", 3.6, 0.7), ("High", 5.1, 0.8)];
    let mut r = seeded(6);
    let (mut group, mut value) = (Vec::new(), Vec::new());
    for (name, mean, sd) in params {
        for _ in 0..100 {
            group.push(name);
            value.push(round2(mean + randn(&mut r) * sd));
        }
    }
    let df = df! { "group" => group, "value" => value }?;
    GGPlot::new(df)
        .aes(Aes::new().x("group").y("value").color("group"))
        .geom_jitter()
        .scale_color_brewer(PaletteName::Dark2)
        .title("Jittered Points")
        .xlab("Dose")
        .ylab("Response")
        .theme_minimal()
        .save_with_size(&out("jitter"), W, H)?;
    Ok(())
}

/// A confidence band (`geom_ribbon`) under a line.
fn ribbon() -> Result<(), Box<dyn std::error::Error>> {
    let n = 80;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.15).collect();
    let y: Vec<f64> = x.iter().map(|v| v.sin() + v * 0.1).collect();
    // Uncertainty widens with x.
    let ymin: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(t, v)| v - (0.25 + t * 0.05))
        .collect();
    let ymax: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(t, v)| v + (0.25 + t * 0.05))
        .collect();
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

/// Stacked areas — three regions over time.
fn area_stack() -> Result<(), Box<dyn std::error::Error>> {
    let n = 40;
    let mut r = seeded(8);
    let (mut x, mut y, mut g) = (Vec::new(), Vec::new(), Vec::new());
    for (grp, base, amp) in [("North", 2.0, 1.2), ("South", 3.0, 1.6), ("East", 1.5, 0.9)] {
        // A smooth seasonal swell plus mild noise, always positive.
        for i in 0..n {
            x.push(i as f64);
            let seasonal = (i as f64 * 0.3).sin().abs() * amp;
            y.push(round2((base + seasonal + randn(&mut r) * 0.2).max(0.1)));
            g.push(grp);
        }
    }
    let df = df! { "x" => x, "y" => y, "g" => g }?;
    GGPlot::new(df)
        .aes(Aes::new().x("x").y("y").fill("g"))
        .geom_area()
        .scale_fill_brewer(PaletteName::Set2)
        .title("Stacked Area")
        .xlab("Month")
        .ylab("Volume")
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

/// Empirical cumulative distribution of a normal sample.
fn ecdf() -> Result<(), Box<dyn std::error::Error>> {
    let mut r = seeded(12);
    let x: Vec<f64> = (0..300).map(|_| round2(randn(&mut r) * 1.5)).collect();
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
    // A heavy-tailed (Student-t-like) sample so the points bow away from the
    // reference line at both ends.
    let mut r = seeded(13);
    let y: Vec<f64> = (0..200)
        .map(|_| {
            let z = randn(&mut r);
            // Inflate the tails.
            round2(z * (1.0 + 0.35 * z * z))
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

/// The same iris scatter rendered under every built-in theme.
fn themes() -> Result<(), Box<dyn std::error::Error>> {
    let (x, y, g) = iris(14);
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
