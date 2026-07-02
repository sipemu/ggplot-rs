//! Supplier delivery-reliability lead-time distribution (see issue #6).
//!
//! "Is a supplier hitting its contracted lead time, and what does its lead-time
//! *distribution* look like once you exclude delays that weren't its fault?"
//!
//! Deliberately **polars-free** — it feeds plain column-oriented `Value` data, so
//! it builds and runs with `--no-default-features` too. This is the shape a
//! stateless renderer (peacock) would use, receiving ACL-checked rows as columns.
//!
//! Run: `cargo run --example supplier_leadtime`  → writes PNGs to assets/gallery/.

use ggplot_rs::prelude::*;

const W: u32 = 720;
const H: u32 = 500;
const CONTRACT: f64 = 30.0; // contracted lead time (SLA), days

// Brand palette (issue #5): DataZoo teal + risk-ish accents.
const TEAL: (u8, u8, u8) = (26, 153, 136);
const CONTRACT_RED: (u8, u8, u8) = (200, 60, 60);
const P90_BLUE: (u8, u8, u8) = (60, 90, 200);
const MUTED: (u8, u8, u8) = (150, 150, 150);

fn out(name: &str) -> String {
    format!("assets/gallery/{name}.png")
}

/// Tiny deterministic PRNG so the example is reproducible without `rand`.
struct Lcg(u64);
impl Lcg {
    fn u01(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 32) as u32) as f64 / (u64::from(u32::MAX) as f64 + 1.0)
    }
    /// Approx N(0,1) via the central limit theorem (sum of 6 uniforms).
    fn normalish(&mut self) -> f64 {
        (0..6).map(|_| self.u01()).sum::<f64>() - 3.0
    }
}

struct Supplier {
    name: &'static str,
    mean: f64,
    sd: f64,
    seed: u64,
    n: usize,
}

/// One row per received purchase order.
struct Po {
    supplier: &'static str,
    lead: f64,
    attributable: bool,
}

fn simulate() -> Vec<Po> {
    let suppliers = [
        Supplier {
            name: "Meridian Components",
            mean: 26.0,
            sd: 5.0,
            seed: 11,
            n: 200,
        },
        Supplier {
            name: "Aurora Freight",
            mean: 34.0,
            sd: 8.0,
            seed: 23,
            n: 200,
        },
        Supplier {
            name: "Boreal Supply",
            mean: 28.0,
            sd: 3.0,
            seed: 37,
            n: 200,
        },
        Supplier {
            name: "Cobalt Metals",
            mean: 30.0,
            sd: 12.0,
            seed: 51,
            n: 200,
        },
    ];

    let mut rows = Vec::new();
    for s in suppliers {
        let mut rng = Lcg(s.seed);
        for _ in 0..s.n {
            // ~8% of deliveries are excused (external disruption) — and tend to be
            // much later, which is exactly why they must be excluded from the SLA view.
            let excused = rng.u01() < 0.08;
            let lead = if excused {
                s.mean + s.sd * rng.normalish() + 20.0 + 15.0 * rng.u01()
            } else {
                s.mean + s.sd * rng.normalish()
            }
            .max(1.0);
            rows.push(Po {
                supplier: s.name,
                lead,
                attributable: !excused,
            });
        }
    }
    rows
}

fn percentile(mut xs: Vec<f64>, p: f64) -> f64 {
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if xs.is_empty() {
        return 0.0;
    }
    let idx = ((xs.len() - 1) as f64 * p).round() as usize;
    xs[idx]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("assets/gallery")?;
    let rows = simulate();

    detail(&rows)?;
    ecdf(&rows)?;
    compare(&rows)?;

    println!("Supplier lead-time charts written to assets/gallery/");
    Ok(())
}

/// One supplier: density of *attributable* lead times, contract + p90 markers,
/// and excused deliveries shown as a muted rug (not folded into the estimate).
fn detail(rows: &[Po]) -> Result<(), Box<dyn std::error::Error>> {
    let name = "Meridian Components";
    let mine: Vec<&Po> = rows.iter().filter(|p| p.supplier == name).collect();

    let attributable: Vec<f64> = mine
        .iter()
        .filter(|p| p.attributable)
        .map(|p| p.lead)
        .collect();
    let p90 = percentile(attributable.clone(), 0.90);

    let dens_data: Vec<(String, Vec<Value>)> = vec![(
        "lead".to_string(),
        attributable.iter().map(|v| Value::Float(*v)).collect(),
    )];
    let excused_data: Vec<(String, Vec<Value>)> = vec![(
        "lead".to_string(),
        mine.iter()
            .filter(|p| !p.attributable)
            .map(|p| Value::Float(p.lead))
            .collect(),
    )];

    GGPlot::new(dens_data)
        .aes(Aes::new().x("lead"))
        .geom_density_with(GeomDensity {
            fill: TEAL,
            color: (20, 110, 98),
            alpha: 0.45,
            line_width: 1.5,
        })
        // SLA threshold — mass to the left is the on-time share.
        .geom_vline_with(GeomVline {
            xintercept: CONTRACT,
            color: CONTRACT_RED,
            width: 1.5,
            linetype: Linetype::Dashed,
            alpha: 1.0,
        })
        // p90 — the number to size safety stock against.
        .geom_vline_with(GeomVline {
            xintercept: p90,
            color: P90_BLUE,
            width: 1.5,
            linetype: Linetype::Dashed,
            alpha: 1.0,
        })
        // Excused deliveries, shown distinctly and kept out of the density.
        .geom_rug_with(GeomRug {
            color: MUTED,
            alpha: 0.7,
            length: 0.04,
            sides: "b".to_string(),
        })
        .layer_data(excused_data)
        .layer_aes(Aes::new().x("lead"))
        .annotate_text(&format!("contract {CONTRACT:.0}d"), CONTRACT + 1.0, 0.075)
        .annotate_text(&format!("p90 {p90:.0}d"), p90 + 1.0, 0.06)
        .annotate_text("<- excused (external)", 48.0, 0.008)
        .title(&format!("{name} — lead-time distribution"))
        .subtitle("attributable deliveries only; excused shown as rug")
        .xlab("Actual lead time (days)")
        .ylab("Density")
        .theme_minimal()
        .save_with_size(&out("supplier_leadtime"), W, H)?;
    Ok(())
}

/// ECDF for one supplier — P(lead <= contract) reads straight off the curve.
fn ecdf(rows: &[Po]) -> Result<(), Box<dyn std::error::Error>> {
    let name = "Meridian Components";
    let attributable: Vec<f64> = rows
        .iter()
        .filter(|p| p.supplier == name && p.attributable)
        .map(|p| p.lead)
        .collect();
    let on_time =
        attributable.iter().filter(|&&l| l <= CONTRACT).count() as f64 / attributable.len() as f64;
    let data: Vec<(String, Vec<Value>)> = vec![(
        "lead".to_string(),
        attributable.iter().map(|v| Value::Float(*v)).collect(),
    )];

    GGPlot::new(data)
        .aes(Aes::new().x("lead"))
        .geom_step()
        .stat(StatEcdf)
        .geom_vline_with(GeomVline {
            xintercept: CONTRACT,
            color: CONTRACT_RED,
            width: 1.5,
            linetype: Linetype::Dashed,
            alpha: 1.0,
        })
        .annotate_text(
            &format!("on-time rate {:.0}%", on_time * 100.0),
            CONTRACT + 1.0,
            0.15,
        )
        .title(&format!("{name} — on-time reliability (ECDF)"))
        .subtitle("cumulative share at the contract line = P(lead <= 30d)")
        .xlab("Actual lead time (days)")
        .ylab("Cumulative share")
        .theme_bw()
        .save_with_size(&out("supplier_leadtime_ecdf"), W, H)?;
    Ok(())
}

/// Compare suppliers: overlaid per-supplier densities against the contract line.
fn compare(rows: &[Po]) -> Result<(), Box<dyn std::error::Error>> {
    let lead: Vec<Value> = rows
        .iter()
        .filter(|p| p.attributable)
        .map(|p| Value::Float(p.lead))
        .collect();
    let supplier: Vec<Value> = rows
        .iter()
        .filter(|p| p.attributable)
        .map(|p| Value::Str(p.supplier.to_string()))
        .collect();
    let data: Vec<(String, Vec<Value>)> = vec![
        ("lead".to_string(), lead),
        ("supplier".to_string(), supplier),
    ];

    GGPlot::new(data)
        .aes(Aes::new().x("lead").fill("supplier").color("supplier"))
        .geom_density_with(GeomDensity {
            alpha: 0.35,
            line_width: 1.2,
            ..Default::default()
        })
        .geom_vline_with(GeomVline {
            xintercept: CONTRACT,
            color: CONTRACT_RED,
            width: 1.2,
            linetype: Linetype::Dashed,
            alpha: 1.0,
        })
        .scale_fill_brewer(PaletteName::Dark2)
        .scale_color_brewer(PaletteName::Dark2)
        .annotate_text("contract", CONTRACT + 1.0, 0.005)
        .title("Lead-time distributions by supplier")
        .subtitle("attributable deliveries; dashed line = contracted lead time")
        .xlab("Actual lead time (days)")
        .ylab("Density")
        .theme_minimal()
        .save_with_size(&out("supplier_leadtime_compare"), W, H)?;
    Ok(())
}
