//! `coord_quickmap` / `coord_map` (#411): map coordinates for plain
//! longitude / latitude data (no `sf` geometry).

use crate::render::Rect;

use super::{AxisSpan, Coord};

/// Which map approximation [`CoordMap`] uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapProjection {
    /// ggplot2's `coord_quickmap()`: linear axes, with the aspect ratio set so
    /// one degree of latitude and of longitude have their true relative
    /// length at the middle latitude (`1 / cos(lat)`).
    Quick,
    /// `coord_map("mercator")`: latitude stretched as Mercator
    /// (`ln tan(π/4 + φ/2)`), equal aspect in projected units.
    Mercator,
}

/// Map coordinates over `x` = longitude, `y` = latitude (degrees), letterboxed
/// to the map's aspect ratio.
pub struct CoordMap {
    pub projection: MapProjection,
    x_span: Option<AxisSpan>,
    y_span: Option<AxisSpan>,
}

impl CoordMap {
    pub fn new(projection: MapProjection) -> Self {
        CoordMap {
            projection,
            x_span: None,
            y_span: None,
        }
    }

    fn ratio(&self) -> f64 {
        let (Some(x), Some(y)) = (self.x_span, self.y_span) else {
            return 1.0;
        };
        let dx = (x.max - x.min).abs();
        if dx < f64::EPSILON {
            return 1.0;
        }
        match self.projection {
            MapProjection::Quick => {
                let mid = ((y.min + y.max) / 2.0).to_radians().cos().abs().max(1e-6);
                (y.max - y.min).abs() / dx / mid
            }
            MapProjection::Mercator => (mercator(y.max) - mercator(y.min)).abs() / dx,
        }
    }

    /// Mercator-warp a normalized y (within the data span; margins linear).
    fn warp_y(&self, n: f64) -> f64 {
        let Some(s) = self
            .y_span
            .filter(|s| s.max > s.min && (s.pmax - s.pmin).abs() > 1e-12)
        else {
            return n;
        };
        if self.projection != MapProjection::Mercator {
            return n;
        }
        let v = s.min + (n - s.pmin) / (s.pmax - s.pmin) * (s.max - s.min);
        if v < s.min || v > s.max {
            return n;
        }
        let (a, b) = (mercator(s.min), mercator(s.max));
        s.pmin + (mercator(v) - a) / (b - a) * (s.pmax - s.pmin)
    }
}

/// Mercator y in degree-like units (so it shares units with longitude).
fn mercator(lat: f64) -> f64 {
    let phi = lat.clamp(-85.05, 85.05).to_radians();
    (std::f64::consts::FRAC_PI_4 + phi / 2.0)
        .tan()
        .ln()
        .to_degrees()
}

impl Coord for CoordMap {
    fn transform(&self, point: (f64, f64), plot_area: &Rect) -> (f64, f64) {
        let r = self
            .data_rect(plot_area)
            .expect("coord_map always letterboxes");
        let (nx, ny) = (point.0, self.warp_y(point.1));
        (r.x + nx * r.width, r.y + (1.0 - ny) * r.height)
    }

    fn set_domains(&mut self, x: Option<AxisSpan>, y: Option<AxisSpan>) {
        self.x_span = x;
        self.y_span = y;
    }

    fn fixed_aspect(&self) -> Option<f64> {
        Some(self.ratio())
    }

    fn data_rect(&self, plot_area: &Rect) -> Option<Rect> {
        let data_aspect = self.ratio();
        let pixel_aspect = plot_area.height / plot_area.width;
        let (w, h, off_x, off_y) = if pixel_aspect > data_aspect {
            let h = plot_area.width * data_aspect;
            (plot_area.width, h, 0.0, (plot_area.height - h) / 2.0)
        } else {
            let w = plot_area.height / data_aspect;
            (w, plot_area.height, (plot_area.width - w) / 2.0, 0.0)
        };
        Some(Rect {
            x: plot_area.x + off_x,
            y: plot_area.y + off_y,
            width: w,
            height: h,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(min: f64, max: f64) -> AxisSpan {
        AxisSpan {
            min,
            max,
            pmin: 0.0,
            pmax: 1.0,
        }
    }

    #[test]
    fn quickmap_aspect_is_one_over_cos_lat() {
        // ggplot2: coord_quickmap at 60°N squeezes longitude by cos(60°) = 0.5.
        let mut c = CoordMap::new(MapProjection::Quick);
        c.set_domains(Some(span(0.0, 10.0)), Some(span(55.0, 65.0)));
        assert!((c.ratio() - 2.0).abs() < 1e-9, "{}", c.ratio());
    }

    #[test]
    fn mercator_stretches_high_latitudes() {
        let mut c = CoordMap::new(MapProjection::Mercator);
        c.set_domains(Some(span(0.0, 10.0)), Some(span(0.0, 60.0)));
        // 30°N sits below the middle of a 0–60° Mercator map.
        let mid = c.warp_y(0.5);
        let want = mercator(30.0) / mercator(60.0);
        assert!((mid - want).abs() < 1e-9 && mid < 0.5, "{mid}");
    }
}
