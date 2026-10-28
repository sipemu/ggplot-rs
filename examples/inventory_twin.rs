//! Inventory digital twin: a stock projection as a nested quantile fan, the
//! historical on-hand trajectory, the reorder point, "today", and an alarm
//! strip below on the same time axis (#124).
//!
//! Polars-free, deterministic data shaped like inventory-rs's
//! `projection` / `trajectory` / `alarms` export tables.
//!
//! ```sh
//! cargo run --no-default-features --example inventory_twin   # SVG
//! cargo run --example inventory_twin                         # + PNG
//! ```

use ggplot_rs::compose::{AxisCollect, PlotGrid};
use ggplot_rs::data::Value;
use ggplot_rs::geom::line::GeomLine;
use ggplot_rs::geom::rect::GeomRect;
use ggplot_rs::geom::refline::{GeomHline, GeomVline};
use ggplot_rs::geom::ribbon::GeomRibbon;
use ggplot_rs::geom::step::GeomStep;
use ggplot_rs::prelude::*;
use ggplot_rs::render::backend::Linetype;

type Frame = Vec<(String, Vec<Value>)>;

const TODAY: f64 = 60.0;
const HORIZON: usize = 60;
const REORDER_POINT: f64 = 120.0;
const ORDER_QTY: f64 = 360.0;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&x| Value::Float(x)).collect()
}

fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

/// Daily demand: a weekly pattern plus deterministic jitter.
fn demand(day: usize) -> f64 {
    let weekly = [9.0, 10.0, 11.0, 10.0, 13.0, 7.0, 4.0][day % 7];
    weekly + ((day as f64 * 1.7).sin() * 2.0).round()
}

/// On-hand stock for days `0..=TODAY`: an (s, Q) policy with a 5-day lead
/// time.
fn trajectory() -> (Vec<f64>, Vec<f64>) {
    let (mut day, mut on_hand) = (vec![], vec![]);
    let (mut stock, mut arrival) = (300.0_f64, None::<usize>);
    for d in 0..=TODAY as usize {
        if arrival == Some(d) {
            stock += ORDER_QTY;
            arrival = None;
        }
        day.push(d as f64);
        on_hand.push(stock);
        stock = (stock - demand(d)).max(0.0);
        if stock < REORDER_POINT && arrival.is_none() {
            arrival = Some(d + 5);
        }
    }
    (day, on_hand)
}

/// Projected quantiles from today: the same (s, Q) policy applied to the
/// expected stock, with uncertainty growing like √t; stock never goes
/// below zero.
fn projection(start: f64) -> Frame {
    let z = [
        ("q05", -1.645),
        ("q10", -1.282),
        ("q25", -0.674),
        ("q50", 0.0),
        ("q75", 0.674),
        ("q90", 1.282),
        ("q95", 1.645),
    ];
    let mut cols: Vec<(String, Vec<Value>)> = vec![("day".into(), vec![])];
    cols.extend(z.iter().map(|(n, _)| (n.to_string(), vec![])));
    let (mut mean, mut arrival) = (start, None::<usize>);
    for t in 0..=HORIZON {
        let d = TODAY as usize + t;
        if arrival == Some(d) {
            mean += ORDER_QTY;
            arrival = None;
        }
        let sd = 11.0 * (t as f64).sqrt();
        cols[0].1.push(Value::Float(d as f64));
        for (i, (_, zq)) in z.iter().enumerate() {
            cols[i + 1].1.push(Value::Float((mean + zq * sd).max(0.0)));
        }
        mean -= demand(d);
        if mean < REORDER_POINT && arrival.is_none() {
            arrival = Some(d + 5);
        }
    }
    cols
}

fn column(frame: &Frame, name: &str) -> Vec<f64> {
    frame
        .iter()
        .find(|(c, _)| c == name)
        .unwrap()
        .1
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

/// Days where `flag` holds, as `[start, end)` spans.
fn spans(day: &[f64], flag: impl Fn(usize) -> bool) -> Vec<(f64, f64)> {
    let (mut out, mut start) = (vec![], None);
    for (i, &d) in day.iter().enumerate() {
        match (flag(i), start) {
            (true, None) => start = Some(d),
            (false, Some(s)) => {
                out.push((s, d));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, day[day.len() - 1] + 1.0));
    }
    out
}

/// Alarm spans with their kind: stock below the reorder point (history,
/// then the projected median) and a ≥ 5% stockout risk (projection).
fn alarms(history: &Frame, proj: &Frame) -> Frame {
    let (hd, on_hand) = (column(history, "day"), column(history, "on_hand"));
    let (pd, q05, q50) = (
        column(proj, "day"),
        column(proj, "q05"),
        column(proj, "q50"),
    );
    // History up to (excluding) today, then the projection from today.
    let day: Vec<f64> = hd[..hd.len() - 1].iter().chain(&pd).copied().collect();
    let level: Vec<f64> = on_hand[..on_hand.len() - 1]
        .iter()
        .chain(&q50)
        .copied()
        .collect();
    let mut rows: Vec<(f64, f64, &str)> = spans(&day, |i| level[i] < REORDER_POINT)
        .into_iter()
        .map(|(a, b)| (a, b, "Below reorder point"))
        .collect();
    rows.extend(
        spans(&pd, |i| q05[i] < 1.0)
            .into_iter()
            .map(|(a, b)| (a, b, "Stockout risk (5%)")),
    );
    let lane = |k: &str| if k.starts_with("Stockout") { 1.0 } else { 0.0 };
    let col =
        |f: &dyn Fn(&(f64, f64, &str)) -> f64| floats(&rows.iter().map(f).collect::<Vec<_>>());
    vec![
        ("start".into(), col(&|r| r.0)),
        ("end".into(), col(&|r| r.1)),
        ("lo".into(), col(&|r| lane(r.2) + 0.15)),
        ("hi".into(), col(&|r| lane(r.2) + 0.85)),
        (
            "alarm".into(),
            strs(&rows.iter().map(|r| r.2).collect::<Vec<_>>()),
        ),
    ]
}

fn twin(history: &Frame, proj: &Frame) -> GGPlot {
    let blue = (33, 102, 172);
    let band = |lo: &str, hi: &str, alpha: f64| {
        (
            GeomRibbon {
                fill: blue,
                alpha,
                ..Default::default()
            },
            Aes::new().x("day").ymin(lo).ymax(hi),
        )
    };
    let mut plot = GGPlot::new(proj.clone());
    // Widest band first, so the narrower ones darken towards the median.
    for (lo, hi, alpha) in [
        ("q05", "q95", 0.15),
        ("q10", "q90", 0.2),
        ("q25", "q75", 0.3),
    ] {
        let (geom, aes) = band(lo, hi, alpha);
        plot = plot.geom_ribbon_with(geom).layer_aes(aes);
    }
    plot.geom_line_with(GeomLine::default().color(blue).linewidth(1.2))
        .layer_aes(Aes::new().x("day").y("q50"))
        .geom_step_with(GeomStep {
            color: (40, 40, 40),
            width: 1.3,
            ..Default::default()
        })
        .layer_data(history.clone())
        .layer_aes(Aes::new().x("day").y("on_hand"))
        .geom_hline_with(GeomHline {
            color: (214, 96, 77),
            linetype: Linetype::Dashed,
            ..GeomHline::new(REORDER_POINT)
        })
        .geom_vline_with(GeomVline {
            color: (120, 120, 120),
            linetype: Linetype::Dotted,
            ..GeomVline::new(TODAY)
        })
        .annotate_text("today", TODAY + 4.0, 470.0)
        .annotate_text("reorder point", 6.0, REORDER_POINT + 22.0)
        .title("Inventory twin — SKU 4711")
        .subtitle("On-hand history, projected 50/80/90% bands and median")
        .ylab("Units on hand")
        .xlab("Day")
        .theme_minimal()
}

fn strip(alarms: &Frame) -> GGPlot {
    // The lanes' y positions carry no meaning: no y tick labels or grid.
    let mut theme = ggplot_rs::theme::presets::theme_minimal();
    theme.hide_y_guides();
    theme.axis_title_y.visible = true;
    GGPlot::new(alarms.clone())
        .aes(
            Aes::new()
                .xmin("start")
                .xmax("end")
                .ymin("lo")
                .ymax("hi")
                .fill("alarm"),
        )
        .geom_rect_with(GeomRect {
            line_width: 0.0,
            ..Default::default()
        })
        .scale_fill_manual(vec![
            ("Below reorder point", RGBAColor::new(244, 165, 130)),
            ("Stockout risk (5%)", RGBAColor::new(178, 24, 43)),
        ])
        .ylab("Alarms")
        .xlab("Day")
        .theme(theme)
        .legend_position(ggplot_rs::theme::LegendPosition::Bottom)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (day, on_hand) = trajectory();
    let start = *on_hand.last().unwrap();
    let history: Frame = vec![
        ("day".into(), floats(&day)),
        ("on_hand".into(), floats(&on_hand)),
    ];
    let proj = projection(start);
    let alarm_spans = alarms(&history, &proj);

    let svg = PlotGrid::new()
        .add(twin(&history, &proj))
        .add(strip(&alarm_spans))
        .ncol(1)
        .heights(&[4.0, 1.3])
        .axis_collect(AxisCollect::CollectX)
        .render_svg_native_with_size(820, 560)?;
    std::fs::write("inventory_twin.svg", svg)?;
    println!("wrote inventory_twin.svg");

    #[cfg(feature = "plotters")]
    {
        twin(&history, &proj).save_with_size("inventory_twin.png", 820, 460)?;
        println!("wrote inventory_twin.png");
    }
    Ok(())
}
