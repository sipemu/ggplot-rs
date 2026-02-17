pub mod area;
pub mod bar;
pub mod boxplot;
pub mod col;
pub mod density;
pub mod errorbar;
pub mod histogram;
pub mod line;
pub mod point;
pub mod refline;
pub mod ribbon;
pub mod rug;
pub mod segment;
pub mod smooth;
pub mod text;

use std::collections::HashMap;

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::Position;
use crate::render::backend::DrawBackend;
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::Stat;
use crate::theme::Theme;

/// Fixed (non-mapped) visual parameters for a geom.
#[derive(Clone, Debug, Default)]
pub struct GeomParams {
    pub values: HashMap<String, f64>,
    pub color: Option<(u8, u8, u8)>,
    pub fill: Option<(u8, u8, u8)>,
    pub alpha: Option<f64>,
}

/// Trait for geometric objects that draw data on the plot.
pub trait Geom: Send + Sync {
    /// Draw this geometry.
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError>;

    /// Required aesthetics.
    fn required_aes(&self) -> Vec<Aesthetic>;

    /// Default stat for this geom.
    fn default_stat(&self) -> Box<dyn Stat>;

    /// Default position adjustment.
    fn default_position(&self) -> Box<dyn Position>;

    /// Non-mapped visual defaults.
    fn default_params(&self) -> GeomParams;

    /// Name for debug/display.
    fn name(&self) -> &str;
}
