//! Per-layer SVG attributes (#221): [`GGPlot::layer_svg_attr`]
//! (crate::plot::GGPlot::layer_svg_attr) wraps a layer's geom at build time
//! in [`LayerSvgAttrs`], which hands the layer's validated attributes to the
//! backend ([`DrawBackend::set_layer_attrs`]) around the wrapped geom's own
//! `draw` and delegates every other [`Geom`] method unchanged — the same
//! wrapper approach as [`super::decorative`], for the same reason (no new
//! field on the public `Layer`/`BuiltLayer` structs).

use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::Position;
use crate::render::backend::DrawBackend;
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Aesthetic, Geom, GeomParams};

/// A geom plus its layer's validated `layer_svg_attr`s.
pub(crate) struct LayerSvgAttrs(pub(crate) Box<dyn Geom>, pub(crate) Vec<(String, String)>);

impl Geom for LayerSvgAttrs {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        backend.set_layer_attrs(self.1.clone());
        let result = self.0.draw(data, coord, scales, theme, backend);
        backend.set_layer_attrs(Vec::new());
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        self.0.required_aes()
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        self.0.default_stat()
    }

    fn default_position(&self) -> Box<dyn Position> {
        self.0.default_position()
    }

    fn default_params(&self) -> GeomParams {
        self.0.default_params()
    }

    fn name(&self) -> &str {
        self.0.name()
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.0.set_series_color(color);
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        self.0.apply_element_geom(el);
    }

    fn include_zero_baseline(&self) -> bool {
        self.0.include_zero_baseline()
    }

    fn allows_infinite(&self) -> bool {
        self.0.allows_infinite()
    }

    fn keeps_na_as_break(&self) -> bool {
        self.0.keeps_na_as_break()
    }

    fn consumes_tooltips(&self) -> bool {
        self.0.consumes_tooltips()
    }

    fn setup_data(&self, data: &mut DataFrame, warnings: &mut Vec<String>) {
        self.0.setup_data(data, warnings);
    }

    fn inherit_aes(&self) -> bool {
        self.0.inherit_aes()
    }

    fn mapped_only_columns(&self) -> &'static [&'static str] {
        self.0.mapped_only_columns()
    }

    fn validate_layer(&self, pre_stat_columns: &[String], stat_name: &str) -> Result<(), String> {
        self.0.validate_layer(pre_stat_columns, stat_name)
    }
}
