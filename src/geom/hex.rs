use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::binhex::{HexLattice, StatBinHex};
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Fallback hex cell size from the data's own resolution (the smallest gap
/// between distinct values, like ggplot2's `resolution()`), used when the
/// stat didn't supply `width`/`height` columns — e.g. a consumer feeding
/// `GeomHex` pre-binned cells through `StatIdentity`.
///
/// On a pointy-top hex lattice with cell width `w` and height `h` (as
/// [`HexLattice`] defines them — `w` is the horizontal spacing between
/// same-row centres, `h*sqrt(3)/2` the row spacing), the *observed*
/// resolution of pooled x/y values is not `w`/`h` themselves:
/// - the x resolution is `w/2`, because offset rows are staggered by `w/2`;
/// - the y resolution is the row spacing `h*sqrt(3)/2`.
///
/// So the cell size is the inverse of that: `w = 2*res(x)`,
/// `h = 2*res(y)/sqrt(3)`. Using the raw resolution directly (a previous
/// version of this fallback) drew cells at half their correct size and not
/// regular (`width != height`, even on a square lattice).
fn fallback_hex_size(xs: impl Iterator<Item = f64>, ys: impl Iterator<Item = f64>) -> (f64, f64) {
    let res_x = super::continuous_bar_half_width(xs, 2.0, 1.0);
    let res_y = super::continuous_bar_half_width(ys, 2.0, 1.0);
    (2.0 * res_x, 2.0 * res_y / 3.0_f64.sqrt())
}

/// Hexagonal binning geometry — draws a hexagon per cell, in data units
/// (not a fixed pixel size): each vertex is computed in data space from the
/// row's `width`/`height` (as [`StatBinHex`] outputs them) and only then
/// mapped and projected, so the lattice stays correct under any coord
/// (including `coord_sf`).
pub struct GeomHex {
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
}

impl Default for GeomHex {
    fn default() -> Self {
        GeomHex {
            color: (50, 50, 50),
            alpha: 1.0,
            line_width: 0.2,
        }
    }
}

impl GeomHex {
    pub fn color(mut self, color: (u8, u8, u8)) -> Self {
        self.color = color;
        self
    }

    /// Sets `line_width` (the hex outline's stroke width).
    pub fn linewidth(mut self, width: f64) -> Self {
        self.line_width = width;
        self
    }
}

impl Geom for GeomHex {
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
        let width_col = data.column("width");
        let height_col = data.column("height");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        let (fallback_w, fallback_h) = fallback_hex_size(
            x_col.iter().filter_map(|v| v.as_f64()),
            y_col.iter().filter_map(|v| v.as_f64()),
        );

        for i in 0..data.nrows() {
            let (Some(cx), Some(cy)) = (x_col[i].as_f64(), y_col[i].as_f64()) else {
                continue;
            };
            let width = width_col
                .and_then(|wc| wc[i].as_f64())
                .filter(|w| w.is_finite() && *w > 0.0)
                .unwrap_or(fallback_w);
            let height = height_col
                .and_then(|hc| hc[i].as_f64())
                .filter(|h| h.is_finite() && *h > 0.0)
                .unwrap_or(fallback_h);

            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[i]))
                .unwrap_or((97, 156, 255));

            super::set_mark(
                backend,
                fill_col.map(|c| format!("n = {}", super::tip_value(&c[i]))),
                None,
                None,
                fill_col.and_then(|c| super::raw_value(&c[i])),
            );
            super::set_mark_formatted(
                backend,
                scales,
                None,
                &Aesthetic::Fill,
                fill_col.map(|c| &c[i]),
            );
            super::set_row_extras(backend, data, i);

            // Vertices in data space, each individually mapped + projected —
            // correct under any coord, including `coord_sf`.
            let verts_data = HexLattice::new(width, height, (cx, cy)).vertices(0, 0);
            let vertices: Vec<(f64, f64)> = verts_data
                .iter()
                .map(|&(vx, vy)| {
                    let nx = x_scale
                        .map(|s| s.map(&crate::data::Value::Float(vx)))
                        .unwrap_or(0.0);
                    let ny = y_scale
                        .map(|s| s.map(&crate::data::Value::Float(vy)))
                        .unwrap_or(0.0);
                    coord.transform((nx, ny), &plot_area)
                })
                .collect();

            backend.draw_polygon(
                &vertices,
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
        super::clear_row_extras(backend);

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatBinHex::default())
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "hex"
    }

    fn consumes_tooltips(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::fallback_hex_size;
    use crate::data::Value;
    use crate::prelude::*;

    /// Review finding #5: `w = 2*res(x)`, `h = 2*res(y)/sqrt(3)` — not the
    /// raw resolution, which draws half-size, irregular hexagons.
    #[test]
    fn fallback_size_is_twice_the_resolution_scaled_for_the_lattice() {
        let (w, h) = fallback_hex_size([0.0, 2.0].into_iter(), [0.0, 3.0].into_iter());
        assert!((w - 4.0).abs() < 1e-9, "w={w}");
        let expected_h = 2.0 * 3.0 / 3.0_f64.sqrt();
        assert!(
            (h - expected_h).abs() < 1e-9,
            "h={h}, expected={expected_h}"
        );
    }

    #[test]
    fn fallback_size_gives_regular_hexagons_on_a_square_lattice() {
        // res(x) = res(y) = W/2 (x) is not how a real square lattice's raw
        // columns look, but the fallback must still reproduce w == h when
        // fed resolutions consistent with a *square* lattice: res(x) = W/2,
        // res(y) = W*sqrt(3)/2 (see HexLattice::centre's row spacing).
        let w_lattice = 5.0;
        let res_x = w_lattice / 2.0;
        let res_y = w_lattice * 3.0_f64.sqrt() / 2.0;
        let (w, h) = fallback_hex_size([0.0, res_x].into_iter(), [0.0, res_y].into_iter());
        assert!((w - w_lattice).abs() < 1e-9, "w={w}");
        assert!((h - w_lattice).abs() < 1e-9, "h={h}");
    }

    /// End-to-end: pre-binned cells through `StatIdentity` (no width/height
    /// columns) still draw regular hexagons at the full (not half) size.
    /// Uses `coord_sf` (feature `sf`) for a true isotropic (equal
    /// pixels-per-data-unit in x and y) panel: `coord_fixed`'s `ratio`
    /// compares the normalized `[0, 1]` axes, not the real data range, so
    /// it can't give this guarantee when x and y have different ranges (as
    /// they deliberately do here).
    #[test]
    #[cfg(feature = "sf")]
    fn geom_hex_without_width_height_columns_draws_regular_hexagons() {
        // x/y resolutions consistent with a *square* (w == h) source
        // lattice — res(y) = res(x)*sqrt(3), see `HexLattice::centre`'s row
        // spacing — so the fallback size comes out regular (w == h).
        let res_x = 5.0;
        let res_y = res_x * 3.0_f64.sqrt();
        let svg = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(res_x)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(res_y)]),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_hex_with(GeomHex::default())
        .stat(crate::stat::identity::StatIdentity)
        .coord_sf()
        .render_svg_native_with_size(400, 400)
        .unwrap();

        assert!(!svg.contains("NaN"));
        let poly = svg.split("<polygon").nth(1).expect("a hex polygon");
        let points_attr = poly
            .split("points=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        let pts: Vec<(f64, f64)> = points_attr
            .split(' ')
            .map(|p| {
                let mut it = p.split(',');
                let x: f64 = it.next().unwrap().parse().unwrap();
                let y: f64 = it.next().unwrap().parse().unwrap();
                (x, y)
            })
            .collect();
        assert_eq!(pts.len(), 6);
        let edge =
            |a: (f64, f64), b: (f64, f64)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        let edges: Vec<f64> = (0..6).map(|i| edge(pts[i], pts[(i + 1) % 6])).collect();
        let first = edges[0];
        for e in &edges {
            assert!((e - first).abs() < 0.05, "edges should be equal: {edges:?}");
        }
    }

    /// `.color()`/`.linewidth()` builders actually change the rendered hex
    /// outline, not just the struct's fields.
    #[test]
    fn color_and_linewidth_builders_change_the_rendered_outline() {
        let svg = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_hex_with(GeomHex::default().color((10, 20, 30)).linewidth(3.0))
        .render_svg_native_with_size(200, 200)
        .unwrap();
        assert!(svg.contains("stroke=\"#0A141E\""));
        assert!(svg.contains("stroke-width=\"3.00\""));
    }

    /// A row with a non-numeric x/y is skipped, not rendered as a
    /// degenerate hexagon at the origin or a NaN polygon.
    #[test]
    fn a_row_with_non_numeric_position_is_skipped() {
        let svg = GGPlot::new(vec![
            (
                "x".into(),
                vec![Value::Float(0.0), Value::Na, Value::Float(5.0)],
            ),
            (
                "y".into(),
                vec![Value::Float(0.0), Value::Float(1.0), Value::Float(5.0)],
            ),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_hex_with(GeomHex::default())
        .stat(crate::stat::identity::StatIdentity)
        .render_svg_native_with_size(200, 200)
        .unwrap();
        assert!(!svg.contains("NaN"));
        assert_eq!(svg.matches("<polygon").count(), 2);
    }
}
