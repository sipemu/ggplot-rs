use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Kernel density estimation on X per group, for ridgeline plots
/// ([`GeomDensityRidges`](crate::geom::density_ridges::GeomDensityRidges)).
/// Outputs: y (the group's value, e.g. a discrete label, kept as-is), x
/// (evaluation points over the group's range ± 3 bandwidths, so each ridge
/// falls to zero at its ends) and `height` (the density normalised to a peak
/// of 1 per group).
pub struct StatXDensityRidges {
    pub n_points: usize,
}

impl Default for StatXDensityRidges {
    fn default() -> Self {
        StatXDensityRidges { n_points: 256 }
    }
}

impl Stat for StatXDensityRidges {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let Some(x_col) = data.column("x") else {
            return DataFrame::new();
        };
        let values: Vec<f64> = x_col
            .iter()
            .filter_map(|v| v.as_f64())
            .filter(|f| f.is_finite())
            .collect();
        if values.len() < 2 {
            return DataFrame::new();
        }
        let group_y = data
            .column("y")
            .and_then(|c| c.first())
            .cloned()
            .unwrap_or(Value::Float(0.0));
        let n = values.len() as f64;
        let bw = super::bw_nrd0(&values);
        let lo = values.iter().cloned().fold(f64::INFINITY, f64::min) - 3.0 * bw;
        let hi = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 3.0 * bw;
        let m = self.n_points.max(2);
        let step = (hi - lo) / (m - 1) as f64;
        let dens: Vec<(f64, f64)> = (0..m)
            .map(|i| {
                let x = lo + i as f64 * step;
                let d = values
                    .iter()
                    .map(|xi| {
                        let u = (x - xi) / bw;
                        (-(u * u) / 2.0).exp()
                    })
                    .sum::<f64>()
                    / (n * bw * (2.0 * std::f64::consts::PI).sqrt());
                (x, d)
            })
            .collect();
        let peak = dens.iter().map(|(_, d)| *d).fold(0.0_f64, f64::max);
        let k = if peak > 0.0 { 1.0 / peak } else { 1.0 };
        let mut result = DataFrame::new();
        result.add_column(
            "x".into(),
            dens.iter().map(|(x, _)| Value::Float(*x)).collect(),
        );
        result.add_column("y".into(), vec![group_y; m]);
        result.add_column(
            "height".into(),
            dens.iter().map(|(_, d)| Value::Float(d * k)).collect(),
        );
        for col_name in &["color", "fill", "group"] {
            if let Some(first) = data.column(col_name).and_then(|c| c.first()) {
                result.add_column(col_name.to_string(), vec![first.clone(); m]);
            }
        }
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "xdensity_ridges"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_ridge_per_group_peaks_at_one_and_ends_near_zero() {
        let mut data = DataFrame::new();
        data.add_column(
            "x".into(),
            (0..40).map(|i| Value::Float((i % 10) as f64)).collect(),
        );
        data.add_column("y".into(), vec![Value::Str("A".into()); 40]);
        let r = StatXDensityRidges::default().compute_group(&data, &ScaleSet::new());
        let h: Vec<f64> = r
            .column("height")
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        let peak = h.iter().cloned().fold(0.0_f64, f64::max);
        assert!((peak - 1.0).abs() < 1e-9, "{peak}");
        assert!(
            h[0] < 0.05 && h[h.len() - 1] < 0.05,
            "{} {}",
            h[0],
            h[h.len() - 1]
        );
        assert!(r
            .column("y")
            .unwrap()
            .iter()
            .all(|v| *v == Value::Str("A".into())));
    }

    #[test]
    fn too_few_values_give_nothing() {
        let mut data = DataFrame::new();
        data.add_column("x".into(), vec![Value::Float(1.0)]);
        data.add_column("y".into(), vec![Value::Str("A".into())]);
        assert_eq!(
            StatXDensityRidges::default()
                .compute_group(&data, &ScaleSet::new())
                .nrows(),
            0
        );
    }
}
