pub mod bin;
pub mod bin2d;
pub mod bindot;
pub mod binhex;
pub mod boxplot;
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

    /// Name for debug/display.
    fn name(&self) -> &str;
}
