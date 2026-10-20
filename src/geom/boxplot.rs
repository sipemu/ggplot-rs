use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, PointShape, PointStyle, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::boxplot::StatBoxplot;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Boxplot geometry.
pub struct GeomBoxplot {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
}

impl Default for GeomBoxplot {
    fn default() -> Self {
        GeomBoxplot {
            fill: (255, 255, 255),
            color: (50, 50, 50),
            width: 0.75,
            alpha: 1.0,
        }
    }
}

/// Parse a boxplot `outliers` cell (B3-5, D4 extension): a comma list,
/// optionally wrapped in `[...]`/`(...)` (`"7, 9"` or `"[7, 9]"`).
/// Returns the successfully-parsed values and whether any part was
/// unparsable (skipped).
fn parse_outliers(s: &str) -> (Vec<f64>, bool) {
    let trimmed = s.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .or_else(|| trimmed.strip_prefix('(').and_then(|t| t.strip_suffix(')')))
        .unwrap_or(trimmed);
    let mut out = Vec::new();
    let mut bad = false;
    for part in inner.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.parse::<f64>() {
            Ok(v) if v.is_finite() => out.push(v),
            _ => bad = true,
        }
    }
    (out, bad)
}

impl Geom for GeomBoxplot {
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
        let lower_col = data.column("lower");
        let middle_col = data.column("middle");
        let upper_col = data.column("upper");
        let ymin_col = data.column("ymin");
        let ymax_col = data.column("ymax");
        let outliers_col = data.column("outliers");
        let fill_col = data.column("fill");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        let x_is_discrete = x_scale.map(|s| s.is_discrete()).unwrap_or(false);

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.5);

            let half_width = if x_is_discrete {
                let n = x_scale.map(|s| s.breaks().len()).unwrap_or(1);
                self.width / (n.max(1) as f64 * 2.5)
            } else {
                0.03
            };

            // Get boxplot values
            let lower = lower_col.and_then(|c| c[i].as_f64()).unwrap_or(0.0);
            let middle = middle_col.and_then(|c| c[i].as_f64()).unwrap_or(0.0);
            let upper = upper_col.and_then(|c| c[i].as_f64()).unwrap_or(0.0);
            let ymin = ymin_col.and_then(|c| c[i].as_f64()).unwrap_or(lower);
            let ymax = ymax_col.and_then(|c| c[i].as_f64()).unwrap_or(upper);

            let map_y = |v: f64| y_scale.map(|s| s.map(&Value::Float(v))).unwrap_or(0.0);

            // Hover tooltip for the whole box: group + median (IQR) — the
            // geom's own default. B3-5: `set_row_extras` below overrides it
            // with a mapped `Aes::tooltip`/`data_attr`, if any, same as
            // every other tooltip-consuming geom.
            let group = super::tip_value(&x_col[i]);
            let r = |v: f64| (v * 100.0).round() / 100.0;
            let tip = if group.is_empty() {
                format!("median {} ({}–{})", r(middle), r(lower), r(upper))
            } else {
                format!("{group}: median {} ({}–{})", r(middle), r(lower), r(upper))
            };
            let middle_value = Value::Float(middle);
            super::set_mark(
                backend,
                Some(tip),
                Some(group.clone()).filter(|g| !g.is_empty()),
                super::series_key(data, i).or(Some(group).filter(|g| !g.is_empty())),
                super::raw_value(&middle_value),
            );
            super::set_mark_formatted(
                backend,
                scales,
                Some(&x_col[i]),
                &Aesthetic::Y,
                Some(&middle_value),
            );
            super::set_row_extras(backend, data, i);

            // Box (IQR) — honour a mapped `fill` (e.g. by group), else the default.
            let box_fill = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[i]))
                .unwrap_or(self.fill);
            let (box_left, box_top) = coord.transform((nx - half_width, map_y(upper)), &plot_area);
            let (box_right, box_bottom) =
                coord.transform((nx + half_width, map_y(lower)), &plot_area);
            backend.draw_rect(
                (box_left, box_top),
                (box_right, box_bottom),
                &RectStyle {
                    fill: Some(box_fill),
                    stroke: Some(self.color),
                    stroke_width: 1.0,
                    alpha: self.alpha,
                    clip: true,
                },
            )?;

            // Median line
            let (med_left, med_y) = coord.transform((nx - half_width, map_y(middle)), &plot_area);
            let (med_right, _) = coord.transform((nx + half_width, map_y(middle)), &plot_area);
            backend.draw_line(
                &[(med_left, med_y), (med_right, med_y)],
                &LineStyle {
                    color: self.color,
                    width: 2.0,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;

            // Lower whisker
            let (_, whisker_bottom) = coord.transform((nx, map_y(ymin)), &plot_area);
            let (center_x, _) = coord.transform((nx, map_y(lower)), &plot_area);
            backend.draw_line(
                &[(center_x, box_bottom), (center_x, whisker_bottom)],
                &LineStyle {
                    color: self.color,
                    width: 1.0,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;
            // Lower whisker cap
            let (wl, _) = coord.transform((nx - half_width * 0.5, map_y(ymin)), &plot_area);
            let (wr, _) = coord.transform((nx + half_width * 0.5, map_y(ymin)), &plot_area);
            backend.draw_line(
                &[(wl, whisker_bottom), (wr, whisker_bottom)],
                &LineStyle {
                    color: self.color,
                    width: 1.0,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;

            // Upper whisker
            let (_, whisker_top) = coord.transform((nx, map_y(ymax)), &plot_area);
            backend.draw_line(
                &[(center_x, box_top), (center_x, whisker_top)],
                &LineStyle {
                    color: self.color,
                    width: 1.0,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;
            // Upper whisker cap
            let (wl, _) = coord.transform((nx - half_width * 0.5, map_y(ymax)), &plot_area);
            let (wr, _) = coord.transform((nx + half_width * 0.5, map_y(ymax)), &plot_area);
            backend.draw_line(
                &[(wl, whisker_top), (wr, whisker_top)],
                &LineStyle {
                    color: self.color,
                    width: 1.0,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;

            // Outliers — `setup_data` has already normalised this cell to a
            // clean, bracket-free "v1, v2" list (any unparsable part was
            // warned on and dropped there), so a plain split/parse suffices.
            if let Some(oc) = outliers_col {
                if let Value::Str(s) = &oc[i] {
                    for part in s.split(',') {
                        if let Ok(val) = part.trim().parse::<f64>() {
                            let ny = map_y(val);
                            let (ox, oy) = coord.transform((nx, ny), &plot_area);
                            backend.draw_circle(
                                (ox, oy),
                                2.0,
                                &PointStyle {
                                    color: self.color,
                                    alpha: 1.0,
                                    filled: false,
                                    shape: PointShape::Circle,
                                },
                            )?;
                        }
                    }
                }
            }
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        // B3-5: `x` is always required; whether `y` or `lower`/`middle`/
        // `upper` is required depends on the stat (identity vs the default
        // `StatBoxplot`) — see `validate_layer`, which build.rs calls
        // separately because `required_aes` has no access to the stat.
        vec![Aesthetic::X]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatBoxplot)
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "boxplot"
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (50, 50, 50), el.ink) {
            self.color = ink;
        }
        if let (true, Some(paper)) = (self.fill == (255, 255, 255), el.paper) {
            self.fill = paper;
        }
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }

    fn mapped_only_columns(&self) -> &'static [&'static str] {
        &["lower", "middle", "upper", "outliers"]
    }

    fn validate_layer(&self, pre_stat_columns: &[String], stat_name: &str) -> Result<(), String> {
        let has = |c: &str| pre_stat_columns.iter().any(|x| x == c);
        if stat_name == "identity" {
            // ggplot2 4.0.3's own stat="identity" boxplot requires the
            // full five-number summary (plan §0's R table) — review #158
            // finding 5: this previously only required lower/middle/upper,
            // silently rendering whisker-less boxes with no ymin/ymax.
            for c in ["lower", "middle", "upper", "ymin", "ymax"] {
                if !has(c) {
                    return Err(format!(
                        "geom_boxplot requires aesthetic '{c}' under stat=\"identity\" but it was not provided"
                    ));
                }
            }
        } else if !has("y") {
            return Err("geom_boxplot requires aesthetic 'y' but it was not provided".to_string());
        }
        Ok(())
    }

    fn consumes_tooltips(&self) -> bool {
        true
    }

    /// B3-5: normalise the `outliers` cell (strip brackets, drop unparsable
    /// parts with one warning) and widen `ymin_final`/`ymax_final` to
    /// include every outlier, so the y scale trains on the full extent
    /// (allowed #6) instead of silently clipping outliers that fall
    /// outside the whiskers.
    fn setup_data(&self, data: &mut DataFrame, warnings: &mut Vec<String>) {
        if !data.has_column("outliers") {
            return;
        }
        let n = data.nrows();
        let outliers_col = data
            .column("outliers")
            .map(|c| c.to_vec())
            .unwrap_or_default();
        let ymin_col = data.column("ymin").map(|c| c.to_vec());
        let ymax_col = data.column("ymax").map(|c| c.to_vec());

        let mut cleaned = Vec::with_capacity(n);
        let mut ymin_final = Vec::with_capacity(n);
        let mut ymax_final = Vec::with_capacity(n);
        let mut had_unparsable = false;

        for i in 0..n {
            let (vals, cleaned_cell) = match outliers_col.get(i) {
                Some(Value::Str(s)) => {
                    let (vals, bad) = parse_outliers(s);
                    had_unparsable |= bad;
                    let joined = vals
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    (vals, Value::Str(joined))
                }
                // Review #158 finding 5: a numeric `outliers` cell (a
                // single value, not a comma list) is a single outlier, not
                // silently ignored. Normalised to the same `Value::Str`
                // shape as the parsed-list case, so `draw`'s plain
                // split/parse keeps working unchanged.
                Some(v) if v.as_f64().is_some_and(f64::is_finite) => {
                    let f = v.as_f64().unwrap();
                    (vec![f], Value::Str(f.to_string()))
                }
                Some(other) => (Vec::new(), other.clone()),
                None => (Vec::new(), Value::Na),
            };
            cleaned.push(cleaned_cell);

            let mut lo = ymin_col
                .as_ref()
                .and_then(|c| c.get(i))
                .and_then(|v| v.as_f64());
            let mut hi = ymax_col
                .as_ref()
                .and_then(|c| c.get(i))
                .and_then(|v| v.as_f64());
            for v in &vals {
                lo = Some(lo.map_or(*v, |l| l.min(*v)));
                hi = Some(hi.map_or(*v, |h| h.max(*v)));
            }
            ymin_final.push(lo.map(Value::Float).unwrap_or(Value::Na));
            ymax_final.push(hi.map(Value::Float).unwrap_or(Value::Na));
        }

        if had_unparsable {
            warnings.push(
                "geom_boxplot: outliers(...) contained an unparsable value; skipped".to_string(),
            );
        }

        data.add_column("outliers".to_string(), cleaned);
        // Review #158 finding 5: an internal-prefix name (never a raw
        // source column can forge it — see `BOXPLOT_YMIN_FINAL`'s doc),
        // so only this geom's own `setup_data` can ever produce these,
        // not a SQL alias/leftover column on an unrelated geom.
        data.add_column(crate::aes::BOXPLOT_YMIN_FINAL.to_string(), ymin_final);
        data.add_column(crate::aes::BOXPLOT_YMAX_FINAL.to_string(), ymax_final);
    }
}
