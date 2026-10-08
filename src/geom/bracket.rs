use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, LineStyle, Linetype, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Significance bracket — a horizontal bar with two downward end ticks spanning
/// `[xmin, xmax]` at height `y`, captioned with a `label` above it (R's
/// `ggpubr::geom_bracket`). Typically annotates a pairwise-comparison p-value or
/// significance stars over a boxplot.
pub struct GeomBracket {
    pub color: (u8, u8, u8),
    pub line_width: f64,
    /// Length of the downward end ticks, in pixels.
    pub tip_length: f64,
    /// Label font size.
    pub label_size: f64,
}

impl Default for GeomBracket {
    fn default() -> Self {
        GeomBracket {
            color: (0, 0, 0),
            line_width: 1.0,
            tip_length: 8.0,
            label_size: 12.0,
        }
    }
}

impl Geom for GeomBracket {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let xmin_col = data
            .column("xmin")
            .ok_or(RenderError::MissingAesthetic("xmin".into()))?;
        let xmax_col = data
            .column("xmax")
            .ok_or(RenderError::MissingAesthetic("xmax".into()))?;
        let y_col = data
            .column("y")
            .ok_or(RenderError::MissingAesthetic("y".into()))?;
        let label_col = data.column("label");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let nxmin = x_scale.map(|s| s.map(&xmin_col[i])).unwrap_or(0.0);
            let nxmax = x_scale.map(|s| s.map(&xmax_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);

            let (px_min, py) = coord.transform((nxmin, ny), &plot_area);
            let (px_max, _) = coord.transform((nxmax, ny), &plot_area);

            // Host hover metadata: the label as tooltip, the comparison (or
            // its `test_id`) as series, the p-value as value.
            let label_text = label_col.map(|lc| lc[i].to_group_key()).unwrap_or_default();
            let comparison = format!(
                "{} vs {}",
                super::tip_value(&xmin_col[i]),
                super::tip_value(&xmax_col[i])
            );
            let series = data
                .column(BRACKET_SERIES_COL)
                .and_then(|c| c.get(i))
                .filter(|v| !v.is_na())
                .map(super::tip_value)
                .unwrap_or_else(|| comparison.clone());
            let value = data
                .column(BRACKET_P_COL)
                .and_then(|c| c.get(i))
                .and_then(super::raw_value);
            let tooltip = if label_text.is_empty() {
                comparison.clone()
            } else {
                format!("{comparison}: {label_text}")
            };
            super::set_mark(
                backend,
                Some(tooltip),
                Some(comparison),
                Some(series),
                value,
            );

            // Bar at `py` with end ticks pointing toward the data (+y is down in
            // screen space, so the ticks drop below the bar).
            let tip = self.tip_length;
            backend.draw_line(
                &[
                    (px_min, py + tip),
                    (px_min, py),
                    (px_max, py),
                    (px_max, py + tip),
                ],
                &LineStyle {
                    color: self.color,
                    alpha: 1.0,
                    width: self.line_width,
                    linetype: Linetype::Solid,
                },
            )?;
            super::clear_mark(backend);

            // Centered label just above the bar.
            if let Some(lc) = label_col {
                let text = lc[i].to_group_key();
                if !text.is_empty() {
                    let cx = (px_min + px_max) / 2.0;
                    backend.draw_text(
                        &text,
                        (cx, py - self.label_size * 0.3 - 2.0),
                        &TextStyle {
                            color: self.color,
                            size: self.label_size,
                            anchor: TextAnchor::Middle,
                            angle: 0.0,
                            family: None,
                            face: FontFace::Plain,
                        },
                    )?;
                }
            }
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Xmin, Aesthetic::Xmax, Aesthetic::Y]
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
        "bracket"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}

/// Layer column carrying each bracket's p-value (for `data-value`).
pub(crate) const BRACKET_P_COL: &str = ".bracket_p";
/// Layer column carrying each bracket's series key (`test_id`, for
/// `data-series`).
pub(crate) const BRACKET_SERIES_COL: &str = ".bracket_series";

/// Significance brackets from a **precomputed** test table — ggpubr's
/// `stat_pvalue_manual()` / `geom_bracket(data = test_table)`. Nothing is
/// recomputed: every bracket comes from one table row.
///
/// The defaults read the anofox `test` contract schema (`group1`, `group2`,
/// `p_adj`, `p_value`, `test_id`, …) and the ggpubr/rstatix columns
/// `y_position` / `label`:
///
/// - **groups**: `group1` / `group2` name the x categories the bracket spans.
///   Rows with a missing group, or a group that is not a level of the plot's
///   x column, are dropped with a warning.
/// - **p-value**: per row, `p_adj` when present and not `NULL`, else
///   `p_value` (override with [`p_column`](Self::p_column)).
/// - **label**: the [`label`](Self::label) template if set; otherwise a
///   non-null `label` column; otherwise `"p = {p}"`.
/// - **height**: a finite `y_position` is used as-is; rows without one are
///   stacked automatically above the data (and above any given positions),
///   `step_increase` × the data's y range apart.
///
/// Label templates substitute `{column}` with that row's value. Special keys:
/// `{p}` (the chosen p-value), `{p.signif}` / `{stars}` (significance stars by
/// [`cutpoints`](Self::cutpoints)) and `{p.format}` (`"p = 0.0123"`-style).
/// Numeric columns whose name starts with `p` are printed like p-values
/// (4 significant digits, scientific below 1e-4); other values as in
/// tooltips. Unknown keys are left verbatim; `NULL` prints as `NA`.
///
/// ```
/// use ggplot_rs::prelude::*;
/// use ggplot_rs::data::Value;
/// let tests = vec![
///     ("group1".to_string(), vec![Value::Str("a".into()), Value::Str("a".into())]),
///     ("group2".to_string(), vec![Value::Str("b".into()), Value::Str("c".into())]),
///     ("p_adj".to_string(), vec![Value::Float(0.003), Value::Float(0.2)]),
/// ];
/// let data = vec![
///     ("g".to_string(), ["a", "b", "c"].iter().map(|s| Value::Str(s.to_string())).collect()),
///     ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)]),
/// ];
/// let svg = GGPlot::new(data)
///     .aes(Aes::new().x("g").y("y"))
///     .geom_point()
///     .geom_bracket_table(tests, BracketTable::new().label("p = {p_adj} {stars}"))
///     .render_svg_native()
///     .unwrap();
/// assert!(svg.contains("p = 0.003 **"));
/// ```
#[derive(Clone)]
pub struct BracketTable {
    /// Label template (see the type docs); `None` = `label` column or `"p = {p}"`.
    pub label: Option<String>,
    /// Column naming the left group.
    pub group1: String,
    /// Column naming the right group.
    pub group2: String,
    /// Explicit p-value column; `None` = `p_adj`, falling back to `p_value`.
    pub p_column: Option<String>,
    /// Column with explicit bracket heights.
    pub y_position: String,
    /// Gap between auto-stacked brackets as a fraction of the data's y range.
    pub step_increase: f64,
    /// Drop non-significant rows (p above the last-but-one cutpoint, i.e.
    /// labelled `ns`).
    pub hide_ns: bool,
    /// Significance-star cutpoints `(upper bound, symbol)`, ascending; a p
    /// at or below a bound gets that symbol. Default (ggpubr):
    /// `≤1e-4 ****`, `≤0.001 ***`, `≤0.01 **`, `≤0.05 *`, else `ns`.
    pub cutpoints: Vec<(f64, String)>,
    /// Bracket appearance.
    pub geom: GeomBracketStyle,
}

/// Appearance of table-driven brackets (a cloneable mirror of
/// [`GeomBracket`]'s fields).
#[derive(Clone, Copy, Debug)]
pub struct GeomBracketStyle {
    pub color: (u8, u8, u8),
    pub line_width: f64,
    pub tip_length: f64,
    pub label_size: f64,
}

impl Default for GeomBracketStyle {
    fn default() -> Self {
        let g = GeomBracket::default();
        GeomBracketStyle {
            color: g.color,
            line_width: g.line_width,
            tip_length: g.tip_length,
            label_size: g.label_size,
        }
    }
}

impl From<GeomBracketStyle> for GeomBracket {
    fn from(s: GeomBracketStyle) -> Self {
        GeomBracket {
            color: s.color,
            line_width: s.line_width,
            tip_length: s.tip_length,
            label_size: s.label_size,
        }
    }
}

impl Default for BracketTable {
    fn default() -> Self {
        BracketTable {
            label: None,
            group1: "group1".into(),
            group2: "group2".into(),
            p_column: None,
            y_position: "y_position".into(),
            step_increase: 0.12,
            hide_ns: false,
            cutpoints: vec![
                (1e-4, "****".into()),
                (1e-3, "***".into()),
                (1e-2, "**".into()),
                (0.05, "*".into()),
                (f64::INFINITY, "ns".into()),
            ],
            geom: GeomBracketStyle::default(),
        }
    }
}

impl BracketTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Label template, e.g. `"p = {p_adj}"` or `"{p.signif}"`.
    pub fn label(mut self, template: &str) -> Self {
        self.label = Some(template.to_string());
        self
    }

    /// Label each bracket with significance stars only (`"{p.signif}"`).
    pub fn stars(self) -> Self {
        self.label("{p.signif}")
    }

    /// Read the p-value from this column instead of `p_adj`/`p_value`.
    pub fn p_column(mut self, col: &str) -> Self {
        self.p_column = Some(col.to_string());
        self
    }

    /// Use other column names for the compared groups.
    pub fn groups(mut self, group1: &str, group2: &str) -> Self {
        self.group1 = group1.to_string();
        self.group2 = group2.to_string();
        self
    }

    /// Read explicit heights from this column (default `y_position`).
    pub fn y_position(mut self, col: &str) -> Self {
        self.y_position = col.to_string();
        self
    }

    /// Spacing of auto-stacked brackets (fraction of the y range; default 0.12).
    pub fn step_increase(mut self, step: f64) -> Self {
        self.step_increase = step;
        self
    }

    /// Hide non-significant comparisons.
    pub fn hide_ns(mut self, hide: bool) -> Self {
        self.hide_ns = hide;
        self
    }

    /// Custom significance-star cutpoints `(upper bound, symbol)`, ascending.
    pub fn cutpoints(mut self, cutpoints: &[(f64, &str)]) -> Self {
        self.cutpoints = cutpoints.iter().map(|(c, s)| (*c, s.to_string())).collect();
        self
    }

    /// Bracket appearance.
    pub fn style(mut self, style: GeomBracketStyle) -> Self {
        self.geom = style;
        self
    }

    /// The star symbol for `p` (empty for a missing / non-finite p).
    pub fn signif(&self, p: f64) -> String {
        if !p.is_finite() {
            return String::new();
        }
        self.cutpoints
            .iter()
            .find(|(c, _)| p <= *c)
            .map(|(_, s)| s.clone())
            .unwrap_or_default()
    }

    /// Whether `p` lands in the last ("not significant") cutpoint class.
    fn is_ns(&self, p: f64) -> bool {
        match self.cutpoints.len() {
            0 => false,
            n => {
                let below = if n >= 2 {
                    self.cutpoints[n - 2].0
                } else {
                    f64::NEG_INFINITY
                };
                p.is_finite() && p > below
            }
        }
    }

    /// Resolve the table into bracket layer data (`xmin`, `xmax`, `y`,
    /// `label` + hover columns). `x_levels` are the plot's x categories (empty
    /// = unknown, no filtering); `y_range` the finite data range (for
    /// auto-stacking). Problems are appended to `warnings`.
    pub(crate) fn resolve(
        &self,
        table: &DataFrame,
        x_levels: &[String],
        y_range: Option<(f64, f64)>,
        warnings: &mut Vec<String>,
    ) -> Option<DataFrame> {
        const WHO: &str = "geom_bracket";
        if let Err(e) = table.validate() {
            warnings.push(format!(
                "{WHO}: test table rejected ({e}); no brackets drawn"
            ));
            return None;
        }
        let (Some(g1), Some(g2)) = (table.column(&self.group1), table.column(&self.group2)) else {
            warnings.push(format!(
                "{WHO}: test table needs '{}' and '{}' columns; no brackets drawn",
                self.group1, self.group2
            ));
            return None;
        };
        let p_of = |i: usize| -> Option<f64> {
            let get = |c: &str| {
                table
                    .column(c)
                    .and_then(|col| col.get(i))
                    .and_then(|v| v.as_f64())
                    .filter(|p| !p.is_nan())
            };
            match &self.p_column {
                Some(c) => get(c),
                None => get("p_adj").or_else(|| get("p_value")),
            }
        };

        let mut rows: Vec<(usize, f64)> = Vec::new(); // (row, p or NaN)
        let (mut missing, mut unknown, mut ns) = (0usize, Vec::new(), 0usize);
        for i in 0..table.nrows() {
            let (a, b) = (&g1[i], &g2[i]);
            if a.is_na() || b.is_na() {
                missing += 1;
                continue;
            }
            if !x_levels.is_empty() {
                let bad: Vec<String> = [a, b]
                    .iter()
                    .map(|v| v.to_group_key())
                    .filter(|k| !x_levels.contains(k))
                    .collect();
                if !bad.is_empty() {
                    unknown.extend(bad);
                    continue;
                }
            }
            let p = p_of(i).unwrap_or(f64::NAN);
            if self.hide_ns && self.is_ns(p) {
                ns += 1;
                continue;
            }
            rows.push((i, p));
        }
        if missing > 0 {
            warnings.push(format!(
                "{WHO}: removed {missing} row{} with a missing group",
                if missing == 1 { "" } else { "s" }
            ));
        }
        if !unknown.is_empty() {
            unknown.dedup();
            warnings.push(format!(
                "{WHO}: removed {} row{} comparing groups not on the x axis ({})",
                unknown.len(),
                if unknown.len() == 1 { "" } else { "s" },
                unknown.join(", ")
            ));
        }
        let _ = ns; // hidden on request: not a warning
        if rows.is_empty() {
            return None;
        }

        // Heights: explicit finite y_position, else auto-stacked.
        let given = |i: usize| {
            table
                .column(&self.y_position)
                .and_then(|c| c.get(i))
                .and_then(|v| v.as_f64())
                .filter(|y| y.is_finite())
        };
        let max_given = rows
            .iter()
            .filter_map(|(i, _)| given(*i))
            .fold(f64::NEG_INFINITY, f64::max);
        let (base, step) = match y_range {
            Some((lo, hi)) => {
                let span = hi - lo;
                let unit = if span > 0.0 {
                    span
                } else if hi != 0.0 {
                    hi.abs()
                } else {
                    1.0
                };
                let step = if self.step_increase.is_finite() && self.step_increase > 0.0 {
                    unit * self.step_increase
                } else {
                    unit * 0.12
                };
                (hi.max(max_given), step)
            }
            None if max_given.is_finite() => (max_given, max_given.abs().max(1.0) * 0.12),
            None => (0.0, 1.0),
        };
        let needs_auto = rows.iter().any(|(i, _)| given(*i).is_none());
        if needs_auto && y_range.is_none() && !max_given.is_finite() {
            warnings.push(format!(
                "{WHO}: no y_position and no y data to stack above; brackets placed from 0"
            ));
        }

        let mut out = DataFrame::new();
        let (mut xmin, mut xmax, mut y, mut label, mut pv, mut series) =
            (vec![], vec![], vec![], vec![], vec![], vec![]);
        let mut auto_k = 0usize;
        for (i, p) in rows {
            xmin.push(g1[i].clone());
            xmax.push(g2[i].clone());
            let yy = given(i).unwrap_or_else(|| {
                auto_k += 1;
                base + step * auto_k as f64
            });
            y.push(Value::Float(yy));
            label.push(Value::Str(self.label_for(table, i, p)));
            pv.push(if p.is_finite() {
                Value::Float(p)
            } else {
                Value::Na
            });
            series.push(
                table
                    .column("test_id")
                    .and_then(|c| c.get(i))
                    .cloned()
                    .unwrap_or(Value::Na),
            );
        }
        out.add_column("xmin".into(), xmin);
        out.add_column("xmax".into(), xmax);
        out.add_column("y".into(), y);
        out.add_column("label".into(), label);
        out.add_column(BRACKET_P_COL.into(), pv);
        out.add_column(BRACKET_SERIES_COL.into(), series);
        Some(out)
    }

    fn label_for(&self, table: &DataFrame, i: usize, p: f64) -> String {
        let template = match &self.label {
            Some(t) => t.clone(),
            None => match table.column("label").and_then(|c| c.get(i)) {
                Some(v) if !v.is_na() => return v.to_group_key(),
                _ => "p = {p}".to_string(),
            },
        };
        let mut out = String::with_capacity(template.len());
        let mut rest = template.as_str();
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            let Some(close) = after.find('}') else {
                out.push_str(&rest[open..]);
                rest = "";
                break;
            };
            let key = &after[..close];
            let fmt_p = |p: f64| {
                if p.is_finite() {
                    format_p_number(p)
                } else {
                    "NA".to_string()
                }
            };
            let sub = match key {
                "p" => Some(fmt_p(p)),
                "p.signif" | "stars" => Some(if p.is_finite() {
                    self.signif(p)
                } else {
                    "NA".to_string()
                }),
                "p.format" => Some(if p.is_finite() {
                    format!("p = {}", format_p_number(p))
                } else {
                    "p = NA".to_string()
                }),
                col => table.column(col).and_then(|c| c.get(i)).map(|v| match v {
                    Value::Na => "NA".to_string(),
                    v if col.starts_with('p') && v.as_f64().is_some() && !v.is_datetime() => {
                        fmt_p(v.as_f64().unwrap_or(f64::NAN))
                    }
                    v => crate::format::format_value(v),
                }),
            };
            match sub {
                Some(s) => out.push_str(&s),
                None => {
                    out.push('{');
                    out.push_str(key);
                    out.push('}');
                }
            }
            rest = &after[close + 1..];
        }
        out.push_str(rest);
        out
    }
}

/// A p-value for a label: 4 significant digits (trailing zeros dropped),
/// scientific below 1e-4, `< 2.2e-16` at R's floor.
pub fn format_p_number(p: f64) -> String {
    if !p.is_finite() {
        return "NA".to_string();
    }
    if p < 2.2e-16 {
        return "< 2.2e-16".to_string();
    }
    if p < 1e-4 {
        return format!("{p:.2e}");
    }
    let digits = (4 - 1 - p.abs().log10().floor() as i32).clamp(0, 12) as usize;
    let s = format!("{p:.digits$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod table_tests {
    use super::*;

    fn s(v: &str) -> Value {
        Value::Str(v.into())
    }

    fn table() -> DataFrame {
        let mut t = DataFrame::new();
        t.add_column("group1".into(), vec![s("a"), s("a"), s("b"), Value::Na]);
        t.add_column("group2".into(), vec![s("b"), s("c"), s("c"), s("c")]);
        t.add_column(
            "p_value".into(),
            vec![
                Value::Float(0.0004),
                Value::Float(0.03),
                Value::Float(0.4),
                Value::Float(0.01),
            ],
        );
        t.add_column(
            "p_adj".into(),
            vec![
                Value::Float(0.0012),
                Value::Na,
                Value::Float(0.9),
                Value::Na,
            ],
        );
        t
    }

    fn levels() -> Vec<String> {
        vec!["a".into(), "b".into(), "c".into()]
    }

    #[test]
    fn p_adj_falls_back_to_p_value_and_stacks() {
        let mut w = Vec::new();
        let out = BracketTable::new()
            .label("{p} {stars}")
            .resolve(&table(), &levels(), Some((0.0, 10.0)), &mut w)
            .unwrap();
        let labels: Vec<String> = out
            .column("label")
            .unwrap()
            .iter()
            .map(|v| v.to_group_key())
            .collect();
        assert_eq!(labels, vec!["0.0012 **", "0.03 *", "0.9 ns"]);
        let ys: Vec<f64> = out
            .column("y")
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert!(
            (ys[0] - 11.2).abs() < 1e-9
                && (ys[1] - 12.4).abs() < 1e-9
                && (ys[2] - 13.6).abs() < 1e-9
        );
        assert!(w.iter().any(|m| m.contains("missing group")), "{w:?}");
    }

    #[test]
    fn hide_ns_and_unknown_groups() {
        let mut t = table();
        t.add_column("y_position".into(), vec![Value::Float(20.0); 4]);
        let mut w = Vec::new();
        let out = BracketTable::new()
            .hide_ns(true)
            .resolve(&t, &["a".into(), "b".into()], Some((0.0, 1.0)), &mut w)
            .unwrap();
        // a–c and b–c compare a level not on the axis; a–b is kept.
        assert_eq!(out.nrows(), 1);
        assert_eq!(out.column("y").unwrap()[0].as_f64(), Some(20.0));
        assert!(w.iter().any(|m| m.contains("not on the x axis")), "{w:?}");
    }

    #[test]
    fn label_column_and_unknown_keys() {
        let mut t = table();
        t.add_column("label".into(), vec![s("custom"), Value::Na, s("x"), s("y")]);
        let mut w = Vec::new();
        let out = BracketTable::new().resolve(&t, &[], None, &mut w).unwrap();
        let labels: Vec<String> = out
            .column("label")
            .unwrap()
            .iter()
            .map(|v| v.to_group_key())
            .collect();
        assert_eq!(labels[0], "custom");
        assert_eq!(labels[1], "p = 0.03");
        let mut w = Vec::new();
        let out = BracketTable::new()
            .label("{nope} {group1}")
            .resolve(&t, &[], None, &mut w)
            .unwrap();
        assert_eq!(out.column("label").unwrap()[0].to_group_key(), "{nope} a");
    }

    #[test]
    fn formats_p_numbers() {
        assert_eq!(format_p_number(0.03), "0.03");
        assert_eq!(format_p_number(0.012345), "0.01235");
        assert_eq!(format_p_number(0.5), "0.5");
        assert_eq!(format_p_number(1.0), "1");
        assert_eq!(format_p_number(3e-5), "3.00e-5");
        assert_eq!(format_p_number(1e-20), "< 2.2e-16");
        assert_eq!(format_p_number(f64::NAN), "NA");
    }
}
