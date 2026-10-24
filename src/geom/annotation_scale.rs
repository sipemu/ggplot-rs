//! `annotation_scale()` / `annotation_attribution()` (feature `sf`) —
//! ggspatial's map furniture: a metric scale bar computed from the
//! projection at the map's centre, and a small attribution text in a
//! corner. Ported from anofox-visualization's `map/mod.rs` (DataZooDE/
//! anofox-visualization#318).
//!
//! Both read the trained X position scale's domain directly rather than
//! any data column, so they add no required aesthetic and no row loop —
//! a plot adds at most one of each, like a legend. The scale bar assumes
//! the X/Y domain is in metres (any `geom_sf` projection other than
//! `SfProjection::PlateCarree`, which stays in degrees and has no single
//! metric scale) — same constraint ggspatial's own `annotation_scale()`
//! has for an unprojected CRS.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Which panel corner a scale bar / attribution sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MapCorner {
    #[default]
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

/// A metric scale bar (ggspatial's `annotation_scale()`): a "nice" 1/2/5 ×
/// 10ⁿ distance at most `max_width_px` long, computed from the trained X
/// scale's domain at the panel's centre height.
pub struct GeomAnnotationScale {
    pub corner: MapCorner,
    pub max_width_px: f64,
    pub color: (u8, u8, u8),
    pub text_size: f64,
    pub line_width: f64,
}

impl Default for GeomAnnotationScale {
    fn default() -> Self {
        GeomAnnotationScale {
            corner: MapCorner::default(),
            max_width_px: 90.0,
            color: (80, 80, 80),
            text_size: 8.0,
            line_width: 1.0,
        }
    }
}

impl GeomAnnotationScale {
    pub fn corner(mut self, corner: MapCorner) -> Self {
        self.corner = corner;
        self
    }

    /// The scale bar's maximum length in pixels (it picks the largest
    /// "nice" 1/2/5 × 10ⁿ distance that still fits). Default `90.0`.
    pub fn max_width_px(mut self, px: f64) -> Self {
        self.max_width_px = px.max(1.0);
        self
    }
}

impl Geom for GeomAnnotationScale {
    fn draw(
        &self,
        _data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let plot_area = backend.plot_area();
        let Some((px, label)) = scale_bar_geometry(coord, scales, &plot_area, self.max_width_px)
        else {
            return Ok(());
        };
        let (x0, y0) = corner_origin(self.corner, &plot_area, px, self.text_size);
        // A staple/I-beam: two short end-ticks joining a horizontal run,
        // like ggspatial's own scale bar.
        backend.draw_line(
            &[(x0, y0), (x0, y0 + 3.0), (x0 + px, y0 + 3.0), (x0 + px, y0)],
            &LineStyle {
                color: self.color,
                alpha: 1.0,
                width: self.line_width,
                linetype: Linetype::Solid,
                ..Default::default()
            },
        )?;
        backend.draw_text(
            &label,
            (x0 + px / 2.0, y0 - 5.0),
            &TextStyle {
                color: self.color,
                size: self.text_size,
                anchor: TextAnchor::Middle,
                angle: 0.0,
                family: None,
                face: crate::render::backend::FontFace::Plain,
            },
        )?;
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
        "annotation_scale"
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (80, 80, 80), el.ink) {
            self.color = ink;
        }
    }
}

/// A small, muted attribution text (ggspatial's `annotation_scale()`
/// sibling convention: `"© Natural Earth"` etc.) in a panel corner.
pub struct GeomAttribution {
    pub text: String,
    pub corner: MapCorner,
    pub color: (u8, u8, u8),
    pub text_size: f64,
}

impl GeomAttribution {
    pub fn new(text: impl Into<String>) -> Self {
        GeomAttribution {
            text: text.into(),
            corner: MapCorner::BottomRight,
            color: (130, 130, 130),
            text_size: 7.5,
        }
    }

    pub fn corner(mut self, corner: MapCorner) -> Self {
        self.corner = corner;
        self
    }
}

impl Geom for GeomAttribution {
    fn draw(
        &self,
        _data: &DataFrame,
        _coord: &dyn Coord,
        _scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        if self.text.is_empty() {
            return Ok(());
        }
        let plot_area = backend.plot_area();
        let tw = crate::render::text::text_width(&self.text, self.text_size);
        let (anchor, x) = match self.corner {
            MapCorner::BottomLeft | MapCorner::TopLeft => (TextAnchor::Start, plot_area.x + 4.0),
            MapCorner::BottomRight | MapCorner::TopRight => {
                (TextAnchor::End, plot_area.x + plot_area.width - 4.0)
            }
        };
        let y = match self.corner {
            MapCorner::BottomLeft | MapCorner::BottomRight => plot_area.y + plot_area.height - 4.0,
            MapCorner::TopLeft | MapCorner::TopRight => plot_area.y + 4.0 + self.text_size / 2.0,
        };
        let _ = tw; // width only needed if a background chip is added later
        backend.draw_text(
            &self.text,
            (x, y),
            &TextStyle {
                color: self.color,
                size: self.text_size,
                anchor,
                angle: 0.0,
                family: None,
                face: crate::render::backend::FontFace::Plain,
            },
        )
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
        "attribution"
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (130, 130, 130), el.ink) {
            self.color = ink;
        }
    }
}

/// `(bar length in px, "50 km"/"300 m" label)`, or `None` when the X scale
/// isn't trained, has a degenerate domain, or the nicest bar would be
/// under 12px (too small to show). Computed from two domain points mapped
/// through the full scale → coord pipeline at the panel's mid-height, so
/// it reflects the actual on-screen scale at the map's centre — correct
/// for a non-conformal projection (Equal Earth, Albers) too, not just a
/// uniform one.
fn scale_bar_geometry(
    coord: &dyn Coord,
    scales: &ScaleSet,
    plot_area: &crate::render::Rect,
    max_width_px: f64,
) -> Option<(f64, String)> {
    let x_scale = scales.get(&Aesthetic::X)?;
    let (d0, d1) = x_scale.domain()?;
    if !(d0.is_finite() && d1.is_finite()) || d1 <= d0 {
        return None;
    }
    let y_scale = scales.get(&Aesthetic::Y);
    let ny = y_scale
        .and_then(|s| s.domain())
        .map(|(y0, y1)| {
            let mid = Value::Float((y0 + y1) / 2.0);
            y_scale.unwrap().map(&mid)
        })
        .unwrap_or(0.5);
    let p0 = coord.transform((x_scale.map(&Value::Float(d0)), ny), plot_area);
    let p1 = coord.transform((x_scale.map(&Value::Float(d1)), ny), plot_area);
    let total_px = (p1.0 - p0.0).hypot(p1.1 - p0.1);
    if !(total_px.is_finite()) || total_px <= 0.0 {
        return None;
    }
    let px_per_unit = total_px / (d1 - d0);
    // px per km, assuming the domain unit is metres.
    let px_per_km = px_per_unit * 1000.0;
    if !(px_per_km.is_finite()) || px_per_km <= 0.0 {
        return None;
    }
    let max_km = max_width_px / px_per_km;
    if !(max_km.is_finite()) || max_km <= 0.0 {
        return None;
    }
    let mag = 10f64.powf(max_km.log10().floor());
    let km = [5.0, 2.0, 1.0]
        .iter()
        .map(|f| f * mag)
        .find(|k| *k <= max_km)
        .unwrap_or(mag);
    let px = km * px_per_km;
    if !px.is_finite() || px < 12.0 {
        return None;
    }
    let label = if km >= 1.0 {
        format!("{} km", km.round() as i64)
    } else {
        format!("{} m", (km * 1000.0).round() as i64)
    };
    Some((px, label))
}

fn corner_origin(
    corner: MapCorner,
    plot_area: &crate::render::Rect,
    bar_px: f64,
    text_size: f64,
) -> (f64, f64) {
    let pad_x = 6.0;
    let pad_y = 8.0 + text_size;
    match corner {
        MapCorner::BottomLeft => (plot_area.x + pad_x, plot_area.y + plot_area.height - pad_y),
        MapCorner::BottomRight => (
            plot_area.x + plot_area.width - pad_x - bar_px,
            plot_area.y + plot_area.height - pad_y,
        ),
        MapCorner::TopLeft => (plot_area.x + pad_x, plot_area.y + pad_y),
        MapCorner::TopRight => (
            plot_area.x + plot_area.width - pad_x - bar_px,
            plot_area.y + pad_y,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::cartesian::CoordCartesian;
    use crate::scale::continuous::ScaleContinuous;
    use crate::scale::Scale;

    fn trained_scales(d0: f64, d1: f64) -> ScaleSet {
        let mut scales = ScaleSet::new();
        let mut x = ScaleContinuous::new().for_aesthetic(Aesthetic::X);
        x.train(&[Value::Float(d0), Value::Float(d1)]);
        scales.add(Box::new(x));
        let mut y = ScaleContinuous::new().for_aesthetic(Aesthetic::Y);
        y.train(&[Value::Float(0.0), Value::Float(1000.0)]);
        scales.add(Box::new(y));
        scales
    }

    fn panel() -> crate::render::Rect {
        crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        }
    }

    #[test]
    fn picks_a_nice_round_distance() {
        // 400,000 m across a 400px-wide-ish panel: ~1000 m/px before
        // margins; a 90px max bar should land on a tidy 1/2/5 figure.
        let scales = trained_scales(0.0, 400_000.0);
        let coord = CoordCartesian::default();
        let (px, label) = scale_bar_geometry(&coord, &scales, &panel(), 90.0).unwrap();
        assert!((12.0..=90.0).contains(&px), "px = {px}");
        assert!(
            label.ends_with(" km") || label.ends_with(" m"),
            "label = {label}"
        );
        // The label's leading number should be one of the "nice" values.
        let n: f64 = label.split_whitespace().next().unwrap().parse().unwrap();
        let mag = 10f64.powf(n.log10().floor());
        assert!(
            [1.0, 2.0, 5.0].contains(&(n / mag).round()),
            "not a nice 1/2/5 figure: {label}"
        );
    }

    #[test]
    fn too_small_a_bar_is_omitted() {
        // A huge domain over a tiny max width collapses the nicest bar
        // under 12px.
        let scales = trained_scales(0.0, 50_000_000.0);
        let coord = CoordCartesian::default();
        assert!(scale_bar_geometry(&coord, &scales, &panel(), 5.0).is_none());
    }

    #[test]
    fn untrained_scale_has_no_bar() {
        let scales = ScaleSet::new();
        let coord = CoordCartesian::default();
        assert!(scale_bar_geometry(&coord, &scales, &panel(), 90.0).is_none());
    }

    #[test]
    fn corner_placement_matches_each_corner() {
        let p = panel();
        let (x_bl, y_bl) = corner_origin(MapCorner::BottomLeft, &p, 50.0, 8.0);
        let (x_br, y_br) = corner_origin(MapCorner::BottomRight, &p, 50.0, 8.0);
        let (x_tl, y_tl) = corner_origin(MapCorner::TopLeft, &p, 50.0, 8.0);
        let (x_tr, y_tr) = corner_origin(MapCorner::TopRight, &p, 50.0, 8.0);
        assert!(x_bl < x_br, "left < right x");
        assert!(y_tl < y_bl, "top < bottom y");
        assert_eq!(x_tl, x_bl);
        assert_eq!(x_tr, x_br);
        assert_eq!(y_tl, y_tr);
        assert_eq!(y_bl, y_br);
    }

    #[test]
    fn default_corner_is_bottom_left() {
        assert_eq!(MapCorner::default(), MapCorner::BottomLeft);
    }
}
