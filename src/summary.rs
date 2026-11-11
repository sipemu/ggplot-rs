//! What a built plot draws, as numbers: per layer (and facet panel) and per
//! series, how many marks, the x range, the y minimum / maximum and where,
//! the first and last y, and a band's extent. A caller that writes about a
//! chart (a title, an answer) can quote these instead of guessing them.
//!
//! ```
//! use ggplot_rs::prelude::*;
//! let df = vec![
//!     ("x".to_string(), vec![Value::Str("a".into()), Value::Str("b".into())]),
//!     ("y".to_string(), vec![Value::Float(3.0), Value::Float(5.0)]),
//! ];
//! let built = GGPlot::new(df).aes(Aes::new().x("x").y("y")).geom_col().build();
//! assert_eq!(built.summary_text(), "col: 2 marks, x a … b; y first 3, last 5, min 3 at a, max 5 at b");
//! ```

use crate::build::BuiltPlot;
use crate::data::{format_epoch_secs, DataFrame, Value};

/// Columns that name a layer's series, in order of preference.
const SERIES_COLS: &[&str] = &["colour", "color", "fill", "group"];
/// At most this many series per layer are listed; the rest are counted.
pub const MAX_SERIES: usize = 12;

/// One layer of one panel.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerSummary {
    /// The geom's name (`line`, `col`, `smooth`, …).
    pub geom: String,
    /// The facet panel's label, when the plot is faceted.
    pub panel: Option<String>,
    pub series: Vec<SeriesSummary>,
}

/// One series of a layer (all its rows when the layer has no series column).
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesSummary {
    /// The series' value (its colour / fill / group), `None` for a single one.
    pub key: Option<String>,
    /// Rows with a finite y.
    pub n: usize,
    /// First and last x, in row order (the order the marks are drawn in).
    pub x_first: String,
    pub x_last: String,
    pub y_first: f64,
    pub y_last: f64,
    /// `(y, x)` of the smallest and largest y (the first, on a tie).
    pub y_min: (f64, String),
    pub y_max: (f64, String),
    /// `(lowest ymin, highest ymax)` of a band (ribbon, smooth, interval).
    pub band: Option<(f64, f64)>,
}

/// The summaries of every layer, panel by panel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlotSummary {
    pub layers: Vec<LayerSummary>,
}

impl BuiltPlot {
    /// What the plot draws, as numbers (see the [module docs](crate::summary)).
    pub fn summary(&self) -> PlotSummary {
        // a date x stays a date in the stat output too (a smooth's grid is
        // plain numbers): format those as dates as well
        let date_x = self.layers.iter().any(|l| {
            l.data
                .column("x")
                .is_some_and(|c| c.iter().any(|v| v.is_datetime()))
        });
        let mut layers = Vec::new();
        let faceted = self.panels.len() > 1 && !self.panels_data.is_empty();
        if faceted {
            for (p, data) in self.panels.iter().zip(&self.panels_data) {
                for (l, d) in self.layers.iter().zip(data) {
                    if let Some(s) = layer_summary(l.geom.name(), d, date_x) {
                        layers.push(LayerSummary {
                            panel: Some(p.label.clone()),
                            ..s
                        });
                    }
                }
            }
        } else {
            for l in &self.layers {
                layers.extend(layer_summary(l.geom.name(), &l.data, date_x));
            }
        }
        PlotSummary { layers }
    }

    /// [`summary`](Self::summary) as text, one line per layer and panel.
    pub fn summary_text(&self) -> String {
        self.summary().to_string()
    }
}

impl std::fmt::Display for PlotSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, l) in self.layers.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            match &l.panel {
                Some(p) => write!(f, "{} [{p}]:", l.geom)?,
                None => write!(f, "{}:", l.geom)?,
            }
            let shown = l.series.len().min(MAX_SERIES);
            for (j, s) in l.series[..shown].iter().enumerate() {
                if j > 0 {
                    write!(f, ";")?;
                }
                if let Some(k) = &s.key {
                    write!(f, " {k} —")?;
                }
                write!(
                    f,
                    " {} mark{}, x {} … {}; y first {}, last {}, min {} at {}, max {} at {}",
                    s.n,
                    if s.n == 1 { "" } else { "s" },
                    s.x_first,
                    s.x_last,
                    num(s.y_first),
                    num(s.y_last),
                    num(s.y_min.0),
                    s.y_min.1,
                    num(s.y_max.0),
                    s.y_max.1
                )?;
                if let Some((lo, hi)) = s.band {
                    write!(f, ", band {} … {}", num(lo), num(hi))?;
                }
            }
            if l.series.len() > shown {
                write!(f, "; +{} more series", l.series.len() - shown)?;
            }
        }
        Ok(())
    }
}

fn layer_summary(geom: &str, d: &DataFrame, date_x: bool) -> Option<LayerSummary> {
    let x = d.column("x")?;
    // a stacked layer keeps each mark's own value in `.y_raw`
    let y = d.column(".y_raw").or_else(|| d.column("y"))?;
    let key = SERIES_COLS.iter().find_map(|c| d.column(c));
    let (ymin, ymax) = (d.column("ymin"), d.column("ymax"));
    // a band is drawn only by a layer whose ymin/ymax differ from its y
    // (a bar's ymin = 0 is its base, not a band)
    let banded = matches!(
        geom,
        "ribbon" | "smooth" | "errorbar" | "linerange" | "pointrange" | "crossbar"
    );
    let mut order: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for i in 0..d.nrows() {
        let k = key.map(|k| label(&k[i], false)).unwrap_or_default();
        let at = match order.iter().position(|o| *o == k) {
            Some(at) => at,
            None => {
                order.push(k);
                rows.push(Vec::new());
                order.len() - 1
            }
        };
        rows[at].push(i);
    }
    let series: Vec<SeriesSummary> = order
        .into_iter()
        .zip(rows)
        .filter_map(|(k, ix)| {
            let pts: Vec<(usize, f64)> = ix
                .into_iter()
                .filter_map(|i| y[i].as_f64().filter(|v| v.is_finite()).map(|v| (i, v)))
                .collect();
            let (&(i0, y0), &(i1, y1)) = (pts.first()?, pts.last()?);
            let lo = pts
                .iter()
                .fold(pts[0], |a, b| if b.1 < a.1 { *b } else { a });
            let hi = pts
                .iter()
                .fold(pts[0], |a, b| if b.1 > a.1 { *b } else { a });
            let xl = |i: usize| label(&x[i], date_x);
            let band = banded.then(|| {
                let f = |c: Option<&[Value]>, pick: fn(f64, f64) -> f64| {
                    c.and_then(|c| {
                        pts.iter()
                            .filter_map(|(i, _)| c[*i].as_f64().filter(|v| v.is_finite()))
                            .reduce(pick)
                    })
                };
                f(ymin, f64::min).zip(f(ymax, f64::max))
            });
            Some(SeriesSummary {
                key: key.map(|_| k),
                n: pts.len(),
                x_first: xl(i0),
                x_last: xl(i1),
                y_first: y0,
                y_last: y1,
                y_min: (lo.1, xl(lo.0)),
                y_max: (hi.1, xl(hi.0)),
                band: band.flatten(),
            })
        })
        .collect();
    (!series.is_empty()).then(|| LayerSummary {
        geom: geom.to_string(),
        panel: None,
        series,
    })
}

/// A value as the summary shows it: a date as `YYYY-MM-DD` (with the time
/// when it has one), a number trimmed, text as is.
fn label(v: &Value, date_x: bool) -> String {
    let date = |secs: f64| {
        let s = format_epoch_secs(secs.round() as i64);
        s.strip_suffix(" 00:00:00").map(str::to_string).unwrap_or(s)
    };
    match v {
        Value::DateTime(_) => date(v.as_f64().unwrap_or(0.0)),
        Value::Str(s) => s.clone(),
        Value::Na => "NA".into(),
        _ => match v.as_f64() {
            Some(n) if date_x => date(n),
            Some(n) => num(n),
            None => v.key_str().into_owned(),
        },
    }
}

/// A number with at most 4 significant decimals, no trailing zeros.
fn num(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    let digits = (4 - n.abs().log10().floor() as i32).clamp(0, 6) as usize;
    let s = format!("{n:.digits$}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    fn strs(v: &[&str]) -> Vec<Value> {
        v.iter().map(|s| Value::Str(s.to_string())).collect()
    }

    #[test]
    fn series_dates_and_a_smooth_band() {
        let day = |d: i64| Value::from_timestamp(1_700_006_400 + d * 86_400);
        let df = vec![
            (
                "x".to_string(),
                vec![day(0), day(1), day(2), day(0), day(1), day(2)],
            ),
            (
                "y".to_string(),
                [3.0, 5.0, 1.0, 2.0, 2.5, 4.0].map(Value::Float).to_vec(),
            ),
            ("g".to_string(), strs(&["A", "A", "A", "B", "B", "B"])),
        ];
        let s = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_line()
            .geom_smooth()
            .build()
            .summary();
        let line = &s.layers[0];
        assert_eq!(line.geom, "line");
        assert_eq!(line.series.len(), 2);
        let a = &line.series[0];
        assert_eq!(a.key.as_deref(), Some("A"));
        assert_eq!(
            (a.n, a.x_first.as_str(), a.x_last.as_str()),
            (3, "2023-11-15", "2023-11-17")
        );
        assert_eq!(a.y_max, (5.0, "2023-11-16".to_string()));
        assert_eq!(a.y_min, (1.0, "2023-11-17".to_string()));
        let smooth = &s.layers[1];
        assert_eq!(smooth.geom, "smooth");
        // the fitted grid's x is a date too, and its band is reported
        assert!(
            smooth.series[0].x_first.starts_with("2023-11-15"),
            "{smooth:?}"
        );
        assert!(smooth.series[0].band.is_some());
    }

    #[test]
    fn stacked_bars_report_their_own_values_and_facets_their_panel() {
        let df = vec![
            ("x".to_string(), strs(&["W1", "W1", "W2", "W2"])),
            (
                "y".to_string(),
                [3.0, 5.0, 7.0, 2.0].map(Value::Float).to_vec(),
            ),
            ("c".to_string(), strs(&["a", "b", "a", "b"])),
        ];
        let s = GGPlot::new(df.clone())
            .aes(Aes::new().x("x").y("y").fill("c"))
            .geom_col()
            .position(PositionStack)
            .build()
            .summary();
        let b = &s.layers[0].series[1];
        assert_eq!(b.key.as_deref(), Some("b"));
        assert_eq!(
            (b.y_first, b.y_last),
            (5.0, 2.0),
            "own values, not the stack tops"
        );
        assert_eq!(b.band, None, "a bar's base is not a band");

        let s = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y"))
            .geom_col()
            .facet_wrap("c", None)
            .build()
            .summary();
        let panels: Vec<_> = s.layers.iter().map(|l| l.panel.clone().unwrap()).collect();
        assert_eq!(panels, ["a", "b"]);
        assert_eq!(s.layers[1].series[0].y_max, (5.0, "W1".to_string()));
    }

    #[test]
    fn many_series_are_counted() {
        let n = 15;
        let df = vec![
            (
                "x".to_string(),
                (0..n).map(|i| Value::Float(i as f64)).collect(),
            ),
            (
                "y".to_string(),
                (0..n).map(|i| Value::Float(i as f64 * 1.5)).collect(),
            ),
            (
                "g".to_string(),
                (0..n).map(|i| Value::Str(format!("s{i}"))).collect(),
            ),
        ];
        let t = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_point()
            .build()
            .summary_text();
        assert!(t.ends_with("; +3 more series"), "{t}");
        assert!(t.contains("s1 — 1 mark, x 1 … 1; y first 1.5"), "{t}");
    }
}
