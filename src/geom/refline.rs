//! Reference lines: `geom_hline`, `geom_vline`, `geom_abline`.
//!
//! Each geom draws one line per row of its layer data when the matching
//! aesthetic column is present (`yintercept`, `xintercept`, or
//! `slope`/`intercept` — ggplot2's data-mapped form, which also honours
//! per-row `color` / `linetype` / `alpha` and facet panels), and otherwise
//! falls back to the constant stored in the geom. As in ggplot2:
//!
//! - `xintercept` / `yintercept` train the x / y position scale (the line is
//!   always inside the panel); `slope` / `intercept` train nothing;
//! - reference lines do not inherit the plot-level mapping (`inherit.aes =
//!   FALSE`);
//! - `geom_abline` lives in data space (in the scales' transformed space for
//!   e.g. `scale_x_log10`), clipped to the panel.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::support::{clip_polyline_unit, map_position};
use super::{Geom, GeomParams};

/// Per-row line style: mapped `color`/`linetype`/`alpha`, else the defaults.
fn row_style(
    data: &DataFrame,
    i: usize,
    scales: &ScaleSet,
    color: (u8, u8, u8),
    width: f64,
    linetype: Linetype,
    alpha: f64,
) -> LineStyle {
    let color = data
        .column("color")
        .and_then(|c| c.get(i))
        .and_then(|v| scales.map_color(&Aesthetic::Color, v))
        .unwrap_or(color);
    let linetype = data
        .column("linetype")
        .and_then(|c| c.get(i))
        .and_then(|v| scales.map_linetype(v))
        .unwrap_or(linetype);
    let alpha = data
        .column("alpha")
        .and_then(|c| c.get(i))
        .and_then(|v| scales.map_alpha(v).or_else(|| v.as_f64()))
        .filter(|a| a.is_finite())
        .unwrap_or(alpha);
    LineStyle {
        color,
        alpha,
        width,
        linetype,
        ..Default::default()
    }
}

/// Hover metadata for one reference line: `data-series` (the row's colour /
/// group level) and `data-value` (the intercept), with a `"<series>: y = v"`
/// tooltip — replaced by the layer's `tooltip_template` (#165), e.g.
/// `.tooltip("{xintercept}")`, when one is set (`set_row_extras` only
/// overrides the tooltip when the internal tooltip column actually
/// exists, so a layer with no template keeps this default).
///
/// B2-4: with `svg_formatted_values` on, the value is formatted through the
/// matching position scale (`what == "x"` → the x scale, else y) instead of
/// the plain numeric display — in the tooltip, and (for a vline, which is
/// the only caller that passes an `axis`) in `data-x` itself: a raw
/// epoch-seconds number is not materially useful even un-formatted, so this
/// one replaces rather than just adding `data-fx` alongside it.
fn mark_line(
    backend: &mut dyn DrawBackend,
    scales: &ScaleSet,
    data: &DataFrame,
    i: Option<usize>,
    what: &str,
    value: &Value,
    axis: Option<String>,
) {
    let series = i.and_then(|i| super::series_key(data, i));
    let raw = super::raw_value(value);
    let aes = if what == "x" {
        Aesthetic::X
    } else {
        Aesthetic::Y
    };
    let formatted = if backend.wants_formatted_values() {
        scales.get(&aes).and_then(|s| s.format_value(value))
    } else {
        None
    };
    let shown = formatted.clone().unwrap_or_else(|| super::tip_value(value));
    let tip = raw.as_ref().map(|_| match &series {
        Some(s) => format!("{s}: {what} = {shown}"),
        None => format!("{what} = {shown}"),
    });
    let axis = axis.map(|_| formatted.unwrap_or(shown));
    super::set_mark(backend, tip, axis, series, raw);
    super::set_mark_formatted(
        backend,
        scales,
        (what == "x").then_some(value),
        &Aesthetic::Y,
        (what == "y").then_some(value),
    );
    // Call set_mark (above) first so a `tooltip()` template — rendered
    // into the same internal column `set_row_extras` reads — wins over
    // this geom's own default tooltip, same as every other
    // `consumes_tooltips` geom (point, segment, path, ...).
    if let Some(i) = i {
        super::set_row_extras(backend, data, i);
    }
}

/// The rows to draw: one per row of `col` (finite values only), or the
/// constant when the column is absent.
fn line_rows(data: &DataFrame, col: &str, constant: f64) -> Vec<(Option<usize>, Value)> {
    match data.column(col) {
        Some(c) => c
            .iter()
            .enumerate()
            .filter(|(_, v)| v.as_f64().is_some_and(f64::is_finite))
            .map(|(i, v)| (Some(i), v.clone()))
            .collect(),
        None if constant.is_finite() => vec![(None, Value::Float(constant))],
        None => vec![],
    }
}

/// Horizontal reference line spanning the panel width.
pub struct GeomHline {
    pub yintercept: f64,
    pub color: (u8, u8, u8),
    pub width: f64,
    pub linetype: Linetype,
    pub alpha: f64,
}

impl GeomHline {
    pub fn new(yintercept: f64) -> Self {
        GeomHline {
            yintercept,
            color: (0, 0, 0),
            width: 1.0,
            linetype: Linetype::Dashed,
            alpha: 1.0,
        }
    }

    /// A data-mapped hline: draws one line per row of the `yintercept`
    /// aesthetic (see [`GGPlot::geom_hline_aes`](crate::plot::GGPlot::geom_hline_aes)).
    pub fn mapped() -> Self {
        Self::new(f64::NAN)
    }
}

impl Geom for GeomHline {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let plot_area = backend.plot_area();
        let y_scale = scales.get(&Aesthetic::Y);
        for (row, value) in line_rows(data, "yintercept", self.yintercept) {
            let ny = map_position(y_scale, &value).unwrap_or(0.5);
            if !ny.is_finite() {
                continue;
            }
            let style = match row {
                Some(i) => row_style(
                    data,
                    i,
                    scales,
                    self.color,
                    self.width,
                    self.linetype,
                    self.alpha,
                ),
                None => LineStyle {
                    color: self.color,
                    alpha: self.alpha,
                    width: self.width,
                    linetype: self.linetype,
                    ..Default::default()
                },
            };
            mark_line(backend, scales, data, row, "y", &value, None);
            let pts = sample_segment(coord, &plot_area, (0.0, ny), (1.0, ny));
            backend.draw_line(&pts, &style)?;
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
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
        "hline"
    }
    fn inherit_aes(&self) -> bool {
        false
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (0, 0, 0), el.ink) {
            self.color = ink;
        }
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
}

/// Vertical reference line spanning the panel height.
pub struct GeomVline {
    pub xintercept: f64,
    pub color: (u8, u8, u8),
    pub width: f64,
    pub linetype: Linetype,
    pub alpha: f64,
}

impl GeomVline {
    pub fn new(xintercept: f64) -> Self {
        GeomVline {
            xintercept,
            color: (0, 0, 0),
            width: 1.0,
            linetype: Linetype::Dashed,
            alpha: 1.0,
        }
    }

    /// A data-mapped vline: one line per row of the `xintercept` aesthetic.
    pub fn mapped() -> Self {
        Self::new(f64::NAN)
    }
}

impl Geom for GeomVline {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        for (row, value) in line_rows(data, "xintercept", self.xintercept) {
            let nx = map_position(x_scale, &value).unwrap_or(0.5);
            if !nx.is_finite() {
                continue;
            }
            let style = match row {
                Some(i) => row_style(
                    data,
                    i,
                    scales,
                    self.color,
                    self.width,
                    self.linetype,
                    self.alpha,
                ),
                None => LineStyle {
                    color: self.color,
                    alpha: self.alpha,
                    width: self.width,
                    linetype: self.linetype,
                    ..Default::default()
                },
            };
            let axis = Some(super::tip_value(&value));
            mark_line(backend, scales, data, row, "x", &value, axis);
            let pts = sample_segment(coord, &plot_area, (nx, 0.0), (nx, 1.0));
            backend.draw_line(&pts, &style)?;
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
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
        "vline"
    }
    fn inherit_aes(&self) -> bool {
        false
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (0, 0, 0), el.ink) {
            self.color = ink;
        }
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
}

/// Line `y = intercept + slope · x` in data space, spanning the panel and
/// clipped to it.
pub struct GeomAbline {
    pub slope: f64,
    pub intercept: f64,
    pub color: (u8, u8, u8),
    pub width: f64,
    pub linetype: Linetype,
    pub alpha: f64,
}

impl GeomAbline {
    pub fn new(slope: f64, intercept: f64) -> Self {
        GeomAbline {
            slope,
            intercept,
            color: (0, 0, 0),
            width: 1.0,
            linetype: Linetype::Dashed,
            alpha: 1.0,
        }
    }

    /// A data-mapped abline: one line per row of the `slope` / `intercept`
    /// aesthetics (a missing one defaults to slope 1 / intercept 0).
    pub fn mapped() -> Self {
        Self::new(1.0, 0.0)
    }
}

/// The data-space x range spanned by the panel (normalized 0 → 1).
fn panel_x_range(scales: &ScaleSet) -> (f64, f64) {
    match scales.get(&Aesthetic::X) {
        Some(s) if s.is_discrete() => (0.5, s.breaks().len().max(1) as f64 + 0.5),
        Some(s) => s.expanded_domain().unwrap_or((0.0, 1.0)),
        None => (0.0, 1.0),
    }
}

/// The normalized (panel) polyline of `y = a + b·x` across the panel,
/// clipped to the unit square. Empty when the line misses the panel.
pub(crate) fn abline_runs(scales: &ScaleSet, slope: f64, intercept: f64) -> Vec<Vec<(f64, f64)>> {
    if !(slope.is_finite() && intercept.is_finite()) {
        return vec![];
    }
    let (x0, x1) = panel_x_range(scales);
    let y_scale = scales.get(&Aesthetic::Y);
    let ny = |x: f64| {
        let y = intercept + slope * x;
        match y_scale {
            Some(_) => map_position(y_scale, &Value::Float(y)).unwrap_or(f64::NAN),
            None => y,
        }
    };
    clip_polyline_unit(&[(0.0, ny(x0)), (1.0, ny(x1))])
}

/// Transform a normalized segment to pixels — densely sampled under polar /
/// radar coordinates (where a straight data line is curved), else its two
/// endpoints.
fn sample_segment(
    coord: &dyn Coord,
    plot_area: &crate::render::Rect,
    a: (f64, f64),
    b: (f64, f64),
) -> Vec<(f64, f64)> {
    let n = if coord.is_polar() || coord.is_radar() {
        64
    } else {
        1
    };
    (0..=n)
        .map(|k| {
            let t = k as f64 / n as f64;
            coord.transform((a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1)), plot_area)
        })
        .collect()
}

impl Geom for GeomAbline {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let plot_area = backend.plot_area();
        let slope_col = data.column("slope");
        let icpt_col = data.column("intercept");
        let rows: Vec<(Option<usize>, f64, f64)> = if slope_col.is_some() || icpt_col.is_some() {
            (0..data.nrows())
                .map(|i| {
                    let get = |c: Option<&[Value]>, d: f64| {
                        c.map_or(Some(d), |c| c.get(i).and_then(|v| v.as_f64()))
                    };
                    (i, get(slope_col, 1.0), get(icpt_col, 0.0))
                })
                .filter_map(|(i, s, a)| Some((Some(i), s?, a?)))
                .collect()
        } else {
            vec![(None, self.slope, self.intercept)]
        };
        for (row, slope, intercept) in rows {
            let style = match row {
                Some(i) => row_style(
                    data,
                    i,
                    scales,
                    self.color,
                    self.width,
                    self.linetype,
                    self.alpha,
                ),
                None => LineStyle {
                    color: self.color,
                    alpha: self.alpha,
                    width: self.width,
                    linetype: self.linetype,
                    ..Default::default()
                },
            };
            let series = row.and_then(|i| super::series_key(data, i));
            let label = format!(
                "y = {} + {}·x",
                super::tip_value(&Value::Float(intercept)),
                super::tip_value(&Value::Float(slope))
            );
            let tip = Some(match &series {
                Some(s) => format!("{s}: {label}"),
                None => label,
            });
            super::set_mark(
                backend,
                tip,
                None,
                series,
                super::raw_value(&Value::Float(slope)),
            );
            // Slope/intercept aren't on either position scale — reset only.
            super::set_mark_formatted(backend, scales, None, &Aesthetic::Y, None);
            // #165: a `tooltip()` template wins over the default
            // "y = a + b·x" label above, same as the other refline geoms.
            if let Some(i) = row {
                super::set_row_extras(backend, data, i);
            }
            for run in abline_runs(scales, slope, intercept) {
                let mut pts = Vec::new();
                for w in run.windows(2) {
                    let seg = sample_segment(coord, &plot_area, w[0], w[1]);
                    if !pts.is_empty() {
                        pts.pop();
                    }
                    pts.extend(seg);
                }
                backend.draw_line(&pts, &style)?;
            }
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
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
        "abline"
    }
    fn inherit_aes(&self) -> bool {
        false
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (0, 0, 0), el.ink) {
            self.color = ink;
        }
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod fixed_value_tests {
    use crate::data::Value;
    use crate::prelude::*;

    /// A plain `geom_hline`/`geom_vline`/`geom_abline` with a fixed value
    /// (no per-row data mapping) takes the "no row" branch of `row_style`'s
    /// match — just the geom's own configured colour/width/linetype/alpha,
    /// not a per-row override.
    #[test]
    fn fixed_value_reflines_render_without_a_data_mapping() {
        let svg = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_hline(0.5)
        .geom_vline(0.5)
        .geom_abline(1.0, 0.0)
        .render_svg_native_with_size(200, 200)
        .unwrap();

        assert!(!svg.contains("NaN"));
        // 3 reference lines + the data polyline-less point layer: at least
        // 3 extra polylines beyond gridlines/axes exist.
        assert!(svg.matches("<polyline").count() >= 3);
    }
}
