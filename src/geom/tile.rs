use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Tile geometry — rectangle centered at (x, y).
///
/// On a continuous axis a tile spans `width` × the data *resolution* (the
/// smallest gap between distinct x values, like ggplot2's `resolution()`;
/// `1` when there is a single value), so tiles on a 10-unit grid are 10 wide
/// and abut. Their extents (`xmin`/`xmax`/`ymin`/`ymax`) train the position
/// scales, so edge tiles stay inside the panel without manual expansion. On a
/// discrete axis a tile fills `width` × its category slot.
pub struct GeomTile {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    /// Tile width as a multiple of the x resolution (default 1 = abutting).
    pub width: f64,
    /// Tile height as a multiple of the y resolution (default 1 = abutting).
    pub height: f64,
    pub line_width: f64,
}

/// ggplot2's `resolution(x, zero = FALSE)`: the smallest gap between distinct
/// finite values, or 1 when there are fewer than two.
pub(crate) fn resolution(values: &[crate::data::Value]) -> f64 {
    let mut xs: Vec<f64> = values
        .iter()
        .filter_map(|v| v.as_f64())
        .filter(|f| f.is_finite())
        .collect();
    xs.sort_by(|a, b| a.total_cmp(b));
    xs.dedup_by(|a, b| (*a - *b).abs() <= 1e-9 * a.abs().max(1.0));
    let gap = xs
        .windows(2)
        .map(|w| w[1] - w[0])
        .fold(f64::INFINITY, f64::min);
    if gap.is_finite() && gap > 0.0 {
        gap
    } else {
        1.0
    }
}

/// Add `lo`/`hi` extent columns `centre ± mult·resolution/2` for a numeric
/// column (skipped for discrete data or when the extents already exist).
pub(crate) fn add_extents(data: &mut DataFrame, centre: &str, lo: &str, hi: &str, mult: f64) {
    if data.has_column(lo) || data.has_column(hi) {
        return;
    }
    let Some(col) = data.column(centre) else {
        return;
    };
    if col
        .iter()
        .any(|v| matches!(v, crate::data::Value::Str(_) | crate::data::Value::Bool(_)))
    {
        return;
    }
    let half = mult * resolution(col) / 2.0;
    let (los, his): (Vec<_>, Vec<_>) = col
        .iter()
        .map(|v| match v.as_f64() {
            Some(c) => (
                crate::data::Value::Float(c - half),
                crate::data::Value::Float(c + half),
            ),
            None => (crate::data::Value::Na, crate::data::Value::Na),
        })
        .unzip();
    data.add_column(lo.to_string(), los);
    data.add_column(hi.to_string(), his);
}

impl Default for GeomTile {
    fn default() -> Self {
        GeomTile {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 1.0,
            width: 1.0,
            height: 1.0,
            line_width: 0.5,
        }
    }
}

impl Geom for GeomTile {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let x_col = data
            .column("x")
            .ok_or(RenderError::MissingAesthetic("x".into()))?;
        let y_col = data
            .column("y")
            .ok_or(RenderError::MissingAesthetic("y".into()))?;
        let fill_col = data.column("fill");
        // A `label` (or the fill value) becomes each tile's hover tooltip.
        let label_col = data.column("label").or(fill_col);

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // Continuous extents come from `setup_data` (resolution-sized).
        let xmin_col = data.column("xmin");
        let xmax_col = data.column("xmax");
        let ymin_col = data.column("ymin");
        let ymax_col = data.column("ymax");
        let ext = |c: Option<&[crate::data::Value]>, i: usize, fallback: f64| {
            c.and_then(|c| c[i].as_f64()).unwrap_or(fallback)
        };
        // Discrete axes (categorical heatmap): centre the tile on the category
        // and size it to the break spacing; continuous axes use data ± half.
        let x_disc = x_scale.map(|s| s.is_discrete()).unwrap_or(false);
        let y_disc = y_scale.map(|s| s.is_discrete()).unwrap_or(false);
        let nbx = x_scale.map(|s| s.breaks().len()).unwrap_or(1).max(1) as f64;
        let nby = y_scale.map(|s| s.breaks().len()).unwrap_or(1).max(1) as f64;

        for i in 0..data.nrows() {
            let (nxmin, nxmax) = if x_disc {
                let nc = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                let hw = self.width / (nbx * 2.0);
                (nc - hw, nc + hw)
            } else {
                let cx = x_col[i].as_f64().unwrap_or(0.0);
                let (lo, hi) = (
                    ext(xmin_col, i, cx - self.width / 2.0),
                    ext(xmax_col, i, cx + self.width / 2.0),
                );
                (
                    x_scale
                        .map(|s| s.map(&crate::data::Value::Float(lo)))
                        .unwrap_or(0.0),
                    x_scale
                        .map(|s| s.map(&crate::data::Value::Float(hi)))
                        .unwrap_or(0.0),
                )
            };
            let (nymin, nymax) = if y_disc {
                let nc = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                let hh = self.height / (nby * 2.0);
                (nc - hh, nc + hh)
            } else {
                let cy = y_col[i].as_f64().unwrap_or(0.0);
                let (lo, hi) = (
                    ext(ymin_col, i, cy - self.height / 2.0),
                    ext(ymax_col, i, cy + self.height / 2.0),
                );
                (
                    y_scale
                        .map(|s| s.map(&crate::data::Value::Float(lo)))
                        .unwrap_or(0.0),
                    y_scale
                        .map(|s| s.map(&crate::data::Value::Float(hi)))
                        .unwrap_or(0.0),
                )
            };

            let (left, top) = coord.transform((nxmin, nymax), &plot_area);
            let (right, bottom) = coord.transform((nxmax, nymin), &plot_area);

            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[i]))
                .unwrap_or(self.fill);

            // Hover tooltip: "x, y: value".
            let xs = super::tip_value(&x_col[i]);
            let ys = super::tip_value(&y_col[i]);
            let tip = match label_col
                .map(|c| super::tip_value(&c[i]))
                .filter(|s| !s.is_empty())
            {
                Some(v) => format!("{xs}, {ys}: {v}"),
                None => format!("{xs}, {ys}"),
            };
            super::set_mark(
                backend,
                Some(tip),
                Some(xs),
                Some(ys),
                fill_col.and_then(|c| super::raw_value(&c[i])),
            );

            backend.draw_rect(
                (left, top.min(bottom)),
                (right, top.max(bottom)),
                &RectStyle {
                    fill: Some(fill_color),
                    stroke: Some(self.color),
                    stroke_width: self.line_width,
                    alpha: self.alpha,
                    clip: true,
                },
            )?;
        }
        super::clear_mark(backend);

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
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
        "tile"
    }

    fn setup_data(&self, data: &mut DataFrame) {
        add_extents(data, "x", "xmin", "xmax", self.width);
        add_extents(data, "y", "ymin", "ymax", self.height);
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
