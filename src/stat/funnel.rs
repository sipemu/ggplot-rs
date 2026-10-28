//! `StatFunnel`: stage polygons for funnel and pyramid charts.
//!
//! Input: `x` (the stage; stages keep data order, repeated stages are summed)
//! and `y` (the stage's value). Output: polygon vertices — several rows per
//! stage sharing a `stage` key, in drawing order — as `fx`/`fy` in a layout
//! where the chart spans `fx ∈ [-0.5, 0.5]` (centred) and `fy ∈ [0, 1]`
//! (top = first stage). The layout lives outside `x`/`y` because the stage
//! mapping makes the x scale discrete; `x`/`y` keep the stage and value.
//! Per-row columns: `fx`, `fy`, `x`, `y`, `stage`, `value`, `pct_first`
//! (value as a share of the first stage), `pct_total` (share of the sum) and
//! the label anchor `label_y`.

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// How stage widths and heights encode the values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FunnelShape {
    /// Equal-height stages whose width is proportional to the value; each
    /// stage tapers to the next stage's width (a conversion funnel).
    Proportional,
    /// A fixed silhouette — full width at the top narrowing to `neck_width`
    /// at `neck_height` above the bottom — cut into stages whose *height* is
    /// proportional to the value (Highcharts' funnel).
    Silhouette { neck_width: f64, neck_height: f64 },
    /// A triangle with its apex on top, cut into stages whose height is
    /// proportional to the value (Highcharts' pyramid).
    Pyramid,
}

/// Funnel stat behind [`GeomFunnel`](crate::geom::funnel::GeomFunnel).
#[derive(Clone, Debug)]
pub struct StatFunnel {
    pub shape: FunnelShape,
    /// Vertical gap between stages as a fraction of the chart height.
    pub gap: f64,
}

impl Default for StatFunnel {
    fn default() -> Self {
        StatFunnel {
            shape: FunnelShape::Proportional,
            gap: 0.01,
        }
    }
}

impl StatFunnel {
    /// Half-width at height `y` (0 = bottom, 1 = top) of a silhouette shape.
    fn half_width(&self, y: f64) -> f64 {
        match self.shape {
            FunnelShape::Silhouette {
                neck_width,
                neck_height,
            } => {
                let nw = neck_width.clamp(0.0, 1.0);
                let nh = neck_height.clamp(0.0, 0.999);
                if y <= nh {
                    nw / 2.0
                } else {
                    (nw + (1.0 - nw) * (y - nh) / (1.0 - nh)) / 2.0
                }
            }
            FunnelShape::Pyramid => (1.0 - y) / 2.0,
            FunnelShape::Proportional => 0.5,
        }
    }
}

impl Stat for StatFunnel {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (Some(xc), Some(yc)) = (data.column("x"), data.column("y")) else {
            return DataFrame::new();
        };
        let mut stages: indexmap::IndexMap<String, (Value, f64)> = indexmap::IndexMap::new();
        for (x, y) in xc.iter().zip(yc) {
            let Some(v) = y.as_f64().filter(|v| v.is_finite() && *v >= 0.0) else {
                continue;
            };
            stages
                .entry(x.key_str().into_owned())
                .or_insert_with(|| (x.clone(), 0.0))
                .1 += v;
        }
        let stages: Vec<(Value, f64)> = stages.into_values().collect();
        let n = stages.len();
        let total: f64 = stages.iter().map(|s| s.1).sum();
        let max = stages.iter().map(|s| s.1).fold(0.0, f64::max);
        if n == 0 || total <= 0.0 {
            return DataFrame::new();
        }
        let first = stages[0].1;
        let gap = self.gap.clamp(0.0, 0.5 / n as f64);

        let mut cols: [Vec<f64>; 3] = Default::default(); // fx, fy, label_y
        let (mut stage, mut value, mut pct_first, mut pct_total) = (vec![], vec![], vec![], vec![]);
        let mut top = 1.0;
        for (i, (key, v)) in stages.iter().enumerate() {
            let h = match self.shape {
                FunnelShape::Proportional => 1.0 / n as f64,
                _ => v / total,
            };
            let (y1, y0) = (top, top - h);
            top = y0;
            let (y1g, y0g) = (y1 - gap / 2.0, (y0 + gap / 2.0).min(y1 - gap / 2.0));
            let poly: Vec<(f64, f64)> = match self.shape {
                FunnelShape::Proportional => {
                    let w_top = if max > 0.0 { v / max / 2.0 } else { 0.0 };
                    let w_bot = stages
                        .get(i + 1)
                        .map(|s| if max > 0.0 { s.1 / max / 2.0 } else { 0.0 })
                        .unwrap_or(w_top);
                    vec![(-w_top, y1g), (w_top, y1g), (w_bot, y0g), (-w_bot, y0g)]
                }
                _ => {
                    // Right edge top → bottom (through the neck corner when
                    // the stage spans it), then the left edge back up.
                    let mut ys = vec![y1g];
                    if let FunnelShape::Silhouette { neck_height, .. } = self.shape {
                        if neck_height < y1g && neck_height > y0g {
                            ys.push(neck_height);
                        }
                    }
                    ys.push(y0g);
                    let mut p: Vec<(f64, f64)> =
                        ys.iter().map(|&y| (self.half_width(y), y)).collect();
                    p.extend(ys.iter().rev().map(|&y| (-self.half_width(y), y)));
                    p
                }
            };
            for &(px, py) in &poly {
                cols[0].push(px);
                cols[1].push(py);
                cols[2].push((y1g + y0g) / 2.0);
                stage.push(key.clone());
                value.push(Value::Float(*v));
                pct_first.push(if first > 0.0 {
                    Value::Float(v / first)
                } else {
                    Value::Na
                });
                pct_total.push(Value::Float(v / total));
            }
        }
        let f = |v: &Vec<f64>| v.iter().map(|&x| Value::Float(x)).collect::<Vec<_>>();
        let mut out = DataFrame::new();
        out.add_column("fx".into(), f(&cols[0]));
        out.add_column("fy".into(), f(&cols[1]));
        out.add_column("x".into(), stage.clone());
        out.add_column("y".into(), value.clone());
        out.add_column("stage".into(), stage);
        out.add_column("value".into(), value);
        out.add_column("pct_first".into(), pct_first);
        out.add_column("pct_total".into(), pct_total);
        out.add_column("label_y".into(), f(&cols[2]));
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    /// Stages share one silhouette: every row of the panel at once.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "funnel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> DataFrame {
        let mut d = DataFrame::new();
        d.add_column(
            "x".into(),
            ["Visit", "Cart", "Buy", "Cart"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        );
        d.add_column(
            "y".into(),
            [1000.0, 300.0, 120.0, 100.0]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        );
        d
    }

    fn col(d: &DataFrame, c: &str) -> Vec<f64> {
        d.column(c)
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    }

    #[test]
    fn proportional_widths_and_shares() {
        let out = StatFunnel {
            gap: 0.0,
            ..Default::default()
        }
        .compute_group(&data(), &ScaleSet::new());
        assert_eq!(out.nrows(), 3 * 4);
        let x = col(&out, "fx");
        assert_eq!(
            &x[..4],
            &[-0.5, 0.5, 0.2, -0.2],
            "Visit tapers to Cart (400)"
        );
        assert_eq!(
            &x[8..12],
            &[-0.06, 0.06, 0.06, -0.06],
            "last stage is a bar"
        );
        let pf = col(&out, "pct_first");
        assert_eq!((pf[0], pf[4], pf[8]), (1.0, 0.4, 0.12));
    }

    #[test]
    fn silhouette_heights_are_proportional_and_pass_the_neck() {
        let stat = StatFunnel {
            shape: FunnelShape::Silhouette {
                neck_width: 0.3,
                neck_height: 0.25,
            },
            gap: 0.0,
        };
        let out = stat.compute_group(&data(), &ScaleSet::new());
        let y = col(&out, "fy");
        // Visit spans 1000/1520 of the height from the top.
        assert!((y[0] - 1.0).abs() < 1e-12);
        assert!((y[1] - (1.0 - 1000.0 / 1520.0)).abs() < 1e-12);
        // Buy (bottom 120/1520) sits inside the neck: constant width.
        let n = out.nrows();
        let x = col(&out, "fx");
        assert!((x[n - 1] + 0.15).abs() < 1e-12);
        assert!((stat.half_width(1.0) - 0.5).abs() < 1e-12);
    }
}
