use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::DrawBackend;
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Invisible geometry that extends scale ranges without drawing anything.
/// Useful for ensuring axes include specific values.
#[derive(Default)]
pub struct GeomBlank;

impl Geom for GeomBlank {
    fn draw(
        &self,
        _data: &DataFrame,
        _coord: &dyn Coord,
        _scales: &ScaleSet,
        _theme: &Theme,
        _backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatIdentity)
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "blank"
    }
}
