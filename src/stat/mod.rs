pub mod bin;
pub mod bin2d;
pub mod bindot;
pub mod binhex;
pub mod boxplot;
#[cfg(feature = "ggpubr")]
pub mod compare_means;
pub mod contour;
pub mod contour_filled;
#[cfg(feature = "ggpubr")]
pub mod cor;
pub mod count;
pub mod density;
pub mod density2d;
pub mod dist;
pub mod ecdf;
pub mod ellipse;
pub mod function;
pub mod identity;
pub mod loess;
pub mod marching_squares;
pub mod qq;
#[cfg(feature = "regression")]
pub mod quantile;
pub mod smooth;
pub mod sum;
pub mod summary;
pub mod summary2d;
pub mod summary_bin;
pub mod ydensity;

use crate::aes::{Aes, Aesthetic};
use crate::data::DataFrame;
use crate::scale::ScaleSet;

/// Trait for statistical transformations.
pub trait Stat: Send + Sync {
    /// Transform data for a single group.
    fn compute_group(&self, data: &DataFrame, scales: &ScaleSet) -> DataFrame;

    /// Required aesthetics this stat needs.
    fn required_aes(&self) -> Vec<Aesthetic>;

    /// Default aesthetic mappings this stat produces.
    fn default_aes(&self) -> Aes {
        Aes::default()
    }

    /// If true, the stat receives every row of a panel at once (grouped only by
    /// facet variables) instead of once per aesthetic group. Needed for
    /// cross-group comparisons such as `stat_compare_means`, which must see all
    /// groups together. Defaults to per-group behaviour.
    fn panelwise(&self) -> bool {
        false
    }

    /// Name for debug/display.
    fn name(&self) -> &str;
}

/// R's `bw.nrd0`: Silverman's rule of thumb, `0.9 · min(sd, IQR/1.34) · n^-0.2`,
/// with R's fallbacks when the spread is zero — `sd`, then `|x[0]|`, then `1` —
/// so a constant sample still gets a positive bandwidth (never 0 → NaN).
pub(crate) fn bw_nrd0(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 {
        return 1.0;
    }
    let nf = n as f64;
    let mean = values.iter().sum::<f64>() / nf;
    let sd = (values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (nf - 1.0)).sqrt();
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let q = |p: f64| {
        let h = (n - 1) as f64 * p;
        let lo = h.floor() as usize;
        let hi = (lo + 1).min(n - 1);
        sorted[lo] + (h - lo as f64) * (sorted[hi] - sorted[lo])
    };
    let iqr = q(0.75) - q(0.25);
    let mut lo = sd.min(iqr / 1.34);
    for fallback in [sd, values[0].abs(), 1.0] {
        if lo > 0.0 && lo.is_finite() {
            break;
        }
        lo = fallback;
    }
    0.9 * lo * nf.powf(-0.2)
}
