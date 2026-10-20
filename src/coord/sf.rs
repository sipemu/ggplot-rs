//! `coord_sf` — spatial coordinate system (feature `sf`).

use crate::aes::Aesthetic;
use crate::render::Rect;
use crate::scale::ScaleSet;

use super::{AxisSpan, Coord};

/// Equal-aspect coordinate system for maps: one map-unit in x covers the same
/// number of pixels as one map-unit in y, so projected geometry keeps its shape.
/// The aspect is derived from the trained data extent (via [`Coord::set_domains`]).
/// Pair it with a `geom_sf` projection (e.g. Mercator) for a conformal map.
///
/// `xlim`/`ylim` (zoom, no filtering — like `coord_cartesian`) plus
/// `expand(false)` pin an exact data window to the panel: useful for a
/// consumer overlaying a rendered fragment on its own basemap, at the same
/// window as its `Frame`.
pub struct CoordSf {
    x_span: Option<AxisSpan>,
    y_span: Option<AxisSpan>,
    xlim: Option<(f64, f64)>,
    ylim: Option<(f64, f64)>,
    expand: bool,
}

impl Default for CoordSf {
    fn default() -> Self {
        CoordSf {
            x_span: None,
            y_span: None,
            xlim: None,
            ylim: None,
            expand: true,
        }
    }
}

impl CoordSf {
    pub fn new() -> Self {
        Self::default()
    }

    /// Zoom the x axis to `[min, max]` (data coordinates) — clips the
    /// viewport without filtering data, like `coord_cartesian`'s `xlim`.
    pub fn xlim(mut self, min: f64, max: f64) -> Self {
        self.xlim = Some((min, max));
        self
    }

    /// Zoom the y axis to `[min, max]` (data coordinates).
    pub fn ylim(mut self, min: f64, max: f64) -> Self {
        self.ylim = Some((min, max));
        self
    }

    /// Whether the position scales get their default expansion padding.
    /// `false` pins `xlim`/`ylim` to the panel exactly (no margin) — see
    /// [`Scale::set_expand`](crate::scale::Scale::set_expand). Default `true`.
    pub fn expand(mut self, on: bool) -> Self {
        self.expand = on;
        self
    }

    /// Data y-range / x-range → the panel aspect that makes x and y isometric.
    fn ratio(&self) -> f64 {
        match (&self.x_span, &self.y_span) {
            (Some(x), Some(y)) if (x.max - x.min).abs() > f64::EPSILON => {
                ((y.max - y.min) / (x.max - x.min)).abs()
            }
            _ => 1.0,
        }
    }
}

impl Coord for CoordSf {
    fn set_domains(&mut self, x: Option<AxisSpan>, y: Option<AxisSpan>) {
        self.x_span = x;
        self.y_span = y;
    }

    fn transform(&self, point: (f64, f64), plot_area: &Rect) -> (f64, f64) {
        // Letterbox the panel to the data aspect ratio (like CoordFixed).
        let data_aspect = self.ratio();
        let pixel_aspect = plot_area.height / plot_area.width;
        let (eff_w, eff_h, off_x, off_y) = if pixel_aspect > data_aspect {
            let h = plot_area.width * data_aspect;
            (plot_area.width, h, 0.0, (plot_area.height - h) / 2.0)
        } else {
            let w = plot_area.height / data_aspect;
            (w, plot_area.height, (plot_area.width - w) / 2.0, 0.0)
        };
        let (nx, ny) = point;
        let px = plot_area.x + off_x + nx * eff_w;
        let py = plot_area.y + off_y + (1.0 - ny) * eff_h;
        (px, py)
    }

    fn zoom_x(&self) -> Option<(f64, f64)> {
        self.xlim
    }

    fn zoom_y(&self) -> Option<(f64, f64)> {
        self.ylim
    }

    fn train_scales(&mut self, scales: &mut ScaleSet) {
        if !self.expand {
            if let Some(s) = scales.get_mut(&Aesthetic::X) {
                s.set_expand(0.0, 0.0);
            }
            if let Some(s) = scales.get_mut(&Aesthetic::Y) {
                s.set_expand(0.0, 0.0);
            }
        }
    }

    // Letterboxed, non-degree units: a label bbox doesn't correspond to a
    // simple left/right panel margin here (B3-1).
    fn supports_axis_reserve(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::data::Value;
    use crate::prelude::*;
    use crate::theme::LegendPosition;

    #[test]
    fn fixed_xlim_ylim_expand_false_fills_the_panel_with_no_margin() {
        let svg = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(100.0)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(50.0)]),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme(crate::theme::presets::theme_void())
        .legend_position(LegendPosition::None)
        .coord_sf_with(
            CoordSf::new()
                .xlim(0.0, 100.0)
                .ylim(0.0, 50.0)
                .expand(false),
        )
        .render_svg_native_with_size(400, 200)
        .unwrap();

        assert!(svg.contains("data-plot=\"0.00 0.00 400.00 200.00\""));
    }

    #[test]
    fn data_origin_and_extent_map_to_the_expected_corners() {
        let plot = || {
            GGPlot::new(vec![
                ("x".into(), vec![Value::Float(0.0), Value::Float(100.0)]),
                ("y".into(), vec![Value::Float(0.0), Value::Float(50.0)]),
            ])
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .theme(crate::theme::presets::theme_void())
            .legend_position(LegendPosition::None)
            .coord_sf_with(
                CoordSf::new()
                    .xlim(0.0, 100.0)
                    .ylim(0.0, 50.0)
                    .expand(false),
            )
        };
        let svg = plot().render_svg_native_with_size(400, 200).unwrap();
        // (0,0) -> (0, 200); (100,50) -> (400, 0).
        assert!(svg.contains("cx=\"0.00\" cy=\"200.00\""));
        assert!(svg.contains("cx=\"400.00\" cy=\"0.00\""));
    }

    #[test]
    fn a_segment_crossing_the_window_still_renders() {
        let svg = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(-50.0)]),
            ("y".into(), vec![Value::Float(25.0)]),
            ("xend".into(), vec![Value::Float(50.0)]),
            ("yend".into(), vec![Value::Float(25.0)]),
        ])
        .aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
        .geom_segment()
        .theme(crate::theme::presets::theme_void())
        .legend_position(LegendPosition::None)
        .coord_sf_with(
            CoordSf::new()
                .xlim(0.0, 100.0)
                .ylim(0.0, 50.0)
                .expand(false),
        )
        .render_svg_native_with_size(400, 200)
        .unwrap();
        assert!(svg.contains("<polyline"));
        assert!(!svg.contains("NaN"));
    }

    #[test]
    fn svg_root_data_false_omits_data_plot_and_data_domain() {
        let plot = || {
            GGPlot::new(vec![
                ("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
                ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ])
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .coord_sf()
        };
        let with_data = plot().render_svg_native_with_size(200, 200).unwrap();
        assert!(with_data.contains("data-plot="));

        let without_data = plot()
            .svg_root_data(false)
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert!(!without_data.contains("data-plot="));
        assert!(!without_data.contains("data-domain="));
    }
}
