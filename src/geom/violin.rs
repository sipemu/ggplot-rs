use std::collections::HashMap;

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::ydensity::StatYDensity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// How violins are sized relative to each other (ggplot2's `scale`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViolinScale {
    /// Every violin has the same area (ggplot2's default).
    #[default]
    Area,
    /// Areas proportional to the number of observations.
    Count,
    /// Every violin has the same maximum width.
    Width,
}

/// Violin geometry — mirrored density polygon per group.
#[derive(Clone, Debug)]
pub struct GeomViolin {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
    /// Relative sizing of the violins (`scale`).
    pub scale: ViolinScale,
    /// Trim to the data range (`trim`, default true); passed to the stat.
    pub trim: bool,
    /// Fixed bandwidth (`bw`); `None` = `bw.nrd0`. Passed to the stat.
    pub bw: Option<f64>,
    /// Bandwidth multiplier (`adjust`). Passed to the stat.
    pub adjust: f64,
    /// Draw horizontal lines at these sample quantiles (`quantiles`).
    pub quantiles: Vec<f64>,
    /// Quantile line style (`quantile.linetype`).
    pub quantile_linetype: Linetype,
    /// Quantile line colour; `None` = the outline colour.
    pub quantile_color: Option<(u8, u8, u8)>,
}

impl Default for GeomViolin {
    fn default() -> Self {
        GeomViolin {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 0.7,
            line_width: 0.5,
            scale: ViolinScale::Area,
            trim: true,
            bw: None,
            adjust: 1.0,
            quantiles: Vec::new(),
            quantile_linetype: Linetype::Solid,
            quantile_color: None,
        }
    }
}

impl GeomViolin {
    /// `scale = "area" | "count" | "width"`.
    pub fn with_scale(mut self, scale: ViolinScale) -> Self {
        self.scale = scale;
        self
    }

    /// `trim = FALSE`: extend the density into the tails.
    pub fn untrimmed(mut self) -> Self {
        self.trim = false;
        self
    }

    /// `bw` — a fixed kernel bandwidth.
    pub fn with_bw(mut self, bw: f64) -> Self {
        self.bw = Some(bw);
        self
    }

    /// `adjust` — bandwidth multiplier.
    pub fn with_adjust(mut self, adjust: f64) -> Self {
        self.adjust = adjust;
        self
    }

    /// `quantiles = c(…)` — draw lines at these sample quantiles.
    pub fn with_quantiles(mut self, q: impl IntoIterator<Item = f64>) -> Self {
        self.quantiles = q.into_iter().collect();
        self
    }
}

impl Geom for GeomViolin {
    fn key_fill(&self) -> Option<((u8, u8, u8), f64)> {
        Some((self.fill, self.alpha))
    }

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
        let width_col = data
            .column("violinwidth")
            .ok_or(RenderError::MissingAesthetic("violinwidth".into()))?;
        let fill_col = data.column("fill");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // Half-width of a group's slot, in the X scale's mapped space. Mirror the
        // boxplot: discrete axes share the slot between all groups; continuous
        // axes use a small fixed fraction.
        let x_is_discrete = x_scale.map(|s| s.is_discrete()).unwrap_or(false);
        let max_half_width = if x_is_discrete {
            let n = x_scale.map(|s| s.breaks().len()).unwrap_or(1);
            0.75 / (n.max(1) as f64 * 2.5)
        } else {
            0.03
        };

        // A dodge (the default `position_dodge`) shifts each violin within its
        // x slot and shares the slot's width between the groups there (#404).
        let x_mapper = super::support::XMapper::new(data, scales);
        // `scale`: area → density / the largest density of all violins;
        // count → each violin's own peak × n / max n; width → own peak.
        let density_col = data.column("density");
        let n_col = data.column("n");
        let max_of = |c: Option<&[crate::data::Value]>| {
            c.map(|c| c.iter().filter_map(|v| v.as_f64()).fold(0.0_f64, f64::max))
                .unwrap_or(0.0)
        };
        let (max_density, max_n) = (max_of(density_col), max_of(n_col));
        let rel_width = |i: usize| -> f64 {
            let own = width_col[i].as_f64().unwrap_or(0.0);
            match self.scale {
                ViolinScale::Width => own,
                ViolinScale::Area => match density_col.and_then(|c| c[i].as_f64()) {
                    Some(d) if max_density > 0.0 => d / max_density,
                    _ => own,
                },
                ViolinScale::Count => match n_col.and_then(|c| c[i].as_f64()) {
                    Some(n) if max_n > 0.0 => own * n / max_n,
                    _ => own,
                },
            }
        };
        let series: Vec<Option<String>> = (0..data.nrows())
            .map(|i| super::series_key(data, i))
            .collect();

        // The stat vstacks one contiguous block of points per group; partition
        // rows back into groups (keyed by x and the group / fill / colour
        // level, so grouped violins at one x stay separate) and draw a
        // polygon each.
        let mut order: Vec<String> = Vec::new();
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, xv) in x_col.iter().enumerate() {
            let key = format!("{xv:?}\u{1f}{:?}", series[i]);
            if !groups.contains_key(&key) {
                order.push(key.clone());
                groups.insert(key.clone(), Vec::new());
            }
            groups.get_mut(&key).unwrap().push(i);
        }

        for key in &order {
            let idxs = &groups[key];
            let first = idxs[0];
            let nx = if x_scale.is_some() {
                x_mapper.map(&x_col[first], first)
            } else {
                0.5
            };
            let max_half_width = max_half_width / x_mapper.slot_n(first);
            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[first]))
                .unwrap_or(self.fill);
            let group = Some(super::tip_value(&x_col[first])).filter(|s| !s.is_empty());
            super::set_mark(
                backend,
                group.clone(),
                group,
                super::series_key(data, first),
                None,
            );
            super::set_mark_formatted(backend, scales, Some(&x_col[first]), &Aesthetic::Y, None);

            // Right side (top→bottom) then left side reversed to close the polygon.
            let mut right_side: Vec<(f64, f64)> = Vec::with_capacity(idxs.len());
            let mut left_side: Vec<(f64, f64)> = Vec::with_capacity(idxs.len());
            for &i in idxs {
                let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                let half = rel_width(i) * max_half_width;
                right_side.push(coord.transform((nx + half, ny), &plot_area));
                left_side.push(coord.transform((nx - half, ny), &plot_area));
            }

            let mut polygon = right_side;
            left_side.reverse();
            polygon.extend(left_side);

            if polygon.len() >= 3 {
                backend.draw_polygon(
                    &polygon,
                    &RectStyle {
                        fill: Some(fill_color),
                        stroke: Some(self.color),
                        stroke_width: self.line_width,
                        alpha: self.alpha,
                        clip: true,
                        corner_radius: 0.0,
                    },
                )?;
            }

            // Quantile lines across the violin at the stat's sample quantiles,
            // as wide as the violin at that y (linear interpolation).
            let quantiles = data
                .column("quantiles")
                .and_then(|c| c[first].as_str().map(str::to_string));
            if let Some(q) = quantiles {
                let mut profile: Vec<(f64, f64)> = idxs
                    .iter()
                    .filter_map(|&i| y_col[i].as_f64().map(|y| (y, rel_width(i))))
                    .collect();
                profile.sort_by(|a, b| a.0.total_cmp(&b.0));
                for qy in q.split(',').filter_map(|v| v.parse::<f64>().ok()) {
                    let Some(w) = interpolate(&profile, qy) else {
                        continue;
                    };
                    let half = w * max_half_width;
                    let ny = y_scale.map(|s| s.map(&crate::data::Value::Float(qy)));
                    let Some(ny) = ny else { continue };
                    backend.draw_line(
                        &[
                            coord.transform((nx - half, ny), &plot_area),
                            coord.transform((nx + half, ny), &plot_area),
                        ],
                        &LineStyle {
                            color: self.quantile_color.unwrap_or(self.color),
                            width: self.line_width.max(0.5),
                            alpha: 1.0,
                            linetype: self.quantile_linetype.clone(),
                            ..Default::default()
                        },
                    )?;
                }
            }
        }
        super::clear_mark(backend);

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatYDensity {
            trim: self.trim,
            bw: self.bw,
            adjust: self.adjust,
            quantiles: self.quantiles.clone(),
            ..Default::default()
        })
    }

    /// ggplot2's default: grouped violins sit side by side in their x slot.
    fn default_position(&self) -> Box<dyn Position> {
        Box::new(crate::position::dodge::PositionDodge)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "violin"
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (50, 50, 50), el.ink) {
            self.color = ink;
        }
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}

/// Width of a sorted `(y, width)` profile at `y` (linear), `None` outside it.
fn interpolate(profile: &[(f64, f64)], y: f64) -> Option<f64> {
    let (first, last) = (profile.first()?, profile.last()?);
    if y < first.0 || y > last.0 {
        return None;
    }
    let k = profile.partition_point(|p| p.0 < y);
    if k == 0 {
        return Some(first.1);
    }
    let (a, b) = (profile[k - 1], profile.get(k).copied().unwrap_or(*last));
    let t = if b.0 > a.0 {
        (y - a.0) / (b.0 - a.0)
    } else {
        0.0
    };
    Some(a.1 + t * (b.1 - a.1))
}
