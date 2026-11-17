//! What a built plot draws, as numbers: per layer (and facet panel) and per
//! series, how many marks, the x range, the y minimum / maximum and where,
//! the first and last y, a band's extent, the series' shape (its largest
//! move against the trend, its peaks) and which series leads at each x. A
//! caller that writes about a chart (a title, an answer) can quote these
//! instead of guessing them.
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
    /// Two or more series over the same x values: which one is highest at
    /// each shared x — `(series, how many x)`, most first — and how many x
    /// they share. "A is always the highest" holds only when A has them all.
    pub leaders: Option<Leaders>,
}

/// Who is highest at each shared x: `(series, how many x)` most first, and
/// how many x the series share.
pub type Leaders = (Vec<(String, usize)>, usize);

/// A move of a series between two of its marks: `(y, x)` at each end.
#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    pub from: (f64, String),
    pub to: (f64, String),
}

/// One series of a layer (all its rows when the layer has no series column).
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesSummary {
    /// The series' value (its colour / fill / group), `None` for a single one.
    pub key: Option<String>,
    /// Rows with a finite y.
    pub n: usize,
    /// The x extent: the smallest and largest of a numeric or date x, the
    /// first and last (in the order drawn) of a discrete one.
    pub x_from: String,
    pub x_to: String,
    pub y_first: f64,
    pub y_last: f64,
    /// `(y, x)` of the smallest and largest y (the first, on a tie).
    pub y_min: (f64, String),
    pub y_max: (f64, String),
    /// `(lowest ymin, highest ymax)` of a band (ribbon, smooth, interval).
    pub band: Option<(f64, f64)>,
    /// The band's width (ymax − ymin) at `x_from` and at `x_to`: whether
    /// the uncertainty widens or narrows along x.
    pub band_width: Option<(f64, f64)>,
    /// Every mark's `(x label, y)`, in the order drawn.
    pub points: Vec<(String, f64)>,
    /// How y goes with a numeric x on a point layer (a scatter, a bubble
    /// chart) of at least [`MIN_COR_POINTS`] marks.
    pub cor: Option<Correlation>,
    /// Along an ordered x (at least [`MIN_SHAPE_POINTS`] marks): the
    /// largest move against the series' net direction — the deepest fall of
    /// a series that ends higher than it starts, the biggest rise of one
    /// that ends lower — when it is at least a tenth of the y range. A
    /// series that "rises steadily" has none worth naming.
    pub counter_move: Option<Move>,
    /// Along an ordered x: the local maxima that stand out by at least a
    /// quarter of the y range (their prominence), highest first. Two or
    /// more: the series peaks more than once.
    pub peaks: Vec<(f64, String)>,
}

/// A series needs this many marks for its shape (counter move, peaks).
pub const MIN_SHAPE_POINTS: usize = 4;

/// A point layer needs this many marks for a correlation.
pub const MIN_COR_POINTS: usize = 5;

/// How two things go together in a scatter: y with x, and a bubble's size
/// with x and with y.
#[derive(Debug, Clone, PartialEq)]
pub struct Correlation {
    /// Pearson's r of x and y.
    pub r: f64,
    /// Spearman's ρ of x and y (ranks: any monotone trend).
    pub rho: f64,
    /// Spearman's ρ of the size with x and with y, for a bubble chart.
    pub size_rho: Option<(f64, f64)>,
}

/// The summaries of every layer, panel by panel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlotSummary {
    pub layers: Vec<LayerSummary>,
    /// Two series over the same x values: their difference per x.
    pub compare: Option<Comparison>,
    /// A faceted plot whose panels each draw one series of the same layer
    /// over the same x: which panel is highest at each shared x, as
    /// [`LayerSummary::leaders`], with the layer's geom.
    pub panel_leaders: Option<(String, Leaders)>,
}

/// `b − a` per shared x, for a plot of exactly two series (2025 against
/// 2024, actual against plan): where the gap is largest and smallest.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub a: String,
    pub b: String,
    /// `(b − a, (b − a) / |a| in %, x)` of the largest and smallest difference.
    pub largest: (f64, Option<f64>, String),
    pub smallest: (f64, Option<f64>, String),
    /// The same by the % change, when that ranks the x values differently.
    pub largest_pct: Option<(f64, Option<f64>, String)>,
    pub smallest_pct: Option<(f64, Option<f64>, String)>,
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
        // a layer repeating the one before it (a line's point markers) is
        // one entry: `line+point`
        let mut merged: Vec<LayerSummary> = Vec::new();
        for l in layers {
            match merged.last_mut() {
                Some(m) if m.panel == l.panel && m.series == l.series => {
                    m.geom = format!("{}+{}", m.geom, l.geom);
                }
                _ => merged.push(l),
            }
        }
        let compare = compare(&merged);
        let panel_leaders = faceted.then(|| panel_leaders(&merged)).flatten();
        PlotSummary {
            layers: merged,
            compare,
            panel_leaders,
        }
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
                // a ribbon is its band: its y is only the lower edge
                if let (true, Some((lo, hi))) = (l.geom == "ribbon", s.band) {
                    write!(
                        f,
                        " {} mark{}, x {} … {}, band {} … {}",
                        s.n,
                        if s.n == 1 { "" } else { "s" },
                        s.x_from,
                        s.x_to,
                        num(lo),
                        num(hi)
                    )?;
                    write_width(f, s)?;
                    continue;
                }
                write!(
                    f,
                    " {} mark{}, x {} … {}; y first {}, last {}, min {} at {}, max {} at {}",
                    s.n,
                    if s.n == 1 { "" } else { "s" },
                    s.x_from,
                    s.x_to,
                    num(s.y_first),
                    num(s.y_last),
                    num(s.y_min.0),
                    s.y_min.1,
                    num(s.y_max.0),
                    s.y_max.1
                )?;
                if let Some((lo, hi)) = s.band {
                    write!(f, ", band {} … {}", num(lo), num(hi))?;
                    write_width(f, s)?;
                }
                if let Some(m) = &s.counter_move {
                    let word = if m.to.0 < m.from.0 { "fall" } else { "rise" };
                    write!(
                        f,
                        "; against the trend: largest {word} {} at {} → {} at {}",
                        num(m.from.0),
                        m.from.1,
                        num(m.to.0),
                        m.to.1
                    )?;
                }
                if s.peaks.len() > 1 {
                    let list: Vec<String> = s.peaks[..s.peaks.len().min(3)]
                        .iter()
                        .map(|(y, x)| format!("{} at {x}", num(*y)))
                        .collect();
                    let more = if s.peaks.len() > 3 { ", …" } else { "" };
                    write!(f, "; {} peaks: {}{more}", s.peaks.len(), list.join(", "))?;
                }
                if let Some(c) = &s.cor {
                    write!(
                        f,
                        "; y {} with x (r {}, ρ {})",
                        trend(c.rho),
                        num(round2(c.r)),
                        num(round2(c.rho))
                    )?;
                    if let Some((sx, sy)) = c.size_rho {
                        write!(
                            f,
                            "; size {} with x (ρ {}), {} with y (ρ {})",
                            trend(sx),
                            num(round2(sx)),
                            trend(sy),
                            num(round2(sy))
                        )?;
                    }
                }
            }
            if l.series.len() > shown {
                write!(f, "; +{} more series", l.series.len() - shown)?;
            }
            if let Some((top, shared)) = &l.leaders {
                let list: Vec<String> = top.iter().map(|(k, c)| format!("{k} {c}")).collect();
                write!(
                    f,
                    "; highest at each of {shared} shared x: {}",
                    list.join(", ")
                )?;
            }
        }
        if let Some((geom, (top, shared))) = &self.panel_leaders {
            let list: Vec<String> = top.iter().map(|(k, c)| format!("{k} {c}")).collect();
            write!(
                f,
                "\n{geom} across panels: highest at each of {shared} shared x: {}",
                list.join(", ")
            )?;
        }
        if let Some(c) = &self.compare {
            let one = |(d, p, x): &(f64, Option<f64>, String)| {
                let sign = if *d >= 0.0 { "+" } else { "" };
                match p {
                    Some(p) => format!("{sign}{} ({sign}{} %) at {x}", num(*d), num(*p)),
                    None => format!("{sign}{} at {x}", num(*d)),
                }
            };
            write!(
                f,
                "\n{} − {}: largest {}, smallest {}",
                c.b,
                c.a,
                one(&c.largest),
                one(&c.smallest)
            )?;
            if let (Some(l), Some(s)) = (&c.largest_pct, &c.smallest_pct) {
                write!(f, "; by %: largest {}, smallest {}", one(l), one(s))?;
            }
        }
        Ok(())
    }
}

/// The plot's series outside bands and ribbons; exactly two named ones over
/// the same (at least two) x values are compared.
fn compare(layers: &[LayerSummary]) -> Option<Comparison> {
    let series: Vec<&SeriesSummary> = layers
        .iter()
        .filter(|l| l.panel.is_none() && l.geom != "ribbon" && l.geom != "smooth")
        .flat_map(|l| &l.series)
        .collect();
    let [a, b] = series.as_slice() else {
        return None;
    };
    let (ka, kb) = (a.key.clone()?, b.key.clone()?);
    if ka == kb || a.points.len() < 2 || a.points.len() != b.points.len() {
        return None;
    }
    let diffs: Vec<(f64, Option<f64>, String)> = a
        .points
        .iter()
        .map(|(x, ya)| {
            let yb = b.points.iter().find(|(xb, _)| xb == x)?.1;
            let d = yb - ya;
            Some((d, (*ya != 0.0).then(|| 100.0 * d / ya.abs()), x.clone()))
        })
        .collect::<Option<_>>()?;
    let pick = |better: fn(f64, f64) -> bool| {
        diffs
            .iter()
            .fold(None::<&(f64, Option<f64>, String)>, |m, d| match m {
                Some(m) if !better(d.0, m.0) => Some(m),
                _ => Some(d),
            })
            .cloned()
    };
    let largest = pick(|x, y| x > y)?;
    let smallest = pick(|x, y| x < y)?;
    let by_pct = |better: fn(f64, f64) -> bool| {
        diffs
            .iter()
            .filter_map(|d| d.1.map(|p| (p, d)))
            .fold(
                None::<(f64, &(f64, Option<f64>, String))>,
                |m, (p, d)| match m {
                    Some(m) if !better(p, m.0) => Some(m),
                    _ => Some((p, d)),
                },
            )
            .map(|(_, d)| d.clone())
    };
    let (lp, sp) = (by_pct(|x, y| x > y), by_pct(|x, y| x < y));
    // only when % ranks them differently (else it repeats the line above)
    let differs = lp.as_ref().map(|l| &l.2) != Some(&largest.2)
        || sp.as_ref().map(|s| &s.2) != Some(&smallest.2);
    Some(Comparison {
        a: ka,
        b: kb,
        largest,
        smallest,
        largest_pct: lp.filter(|_| differs),
        smallest_pct: sp.filter(|_| differs),
    })
}

/// A correlation in words: how strongly, and which way.
fn trend(rho: f64) -> &'static str {
    match (rho.abs(), rho > 0.0) {
        (a, _) if a < 0.2 => "shows no clear trend",
        (a, true) if a < 0.4 => "rises weakly",
        (a, false) if a < 0.4 => "falls weakly",
        (a, true) if a < 0.7 => "rises moderately",
        (a, false) if a < 0.7 => "falls moderately",
        (_, true) => "rises strongly",
        (_, false) => "falls strongly",
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Pearson's r, `None` when either side doesn't vary.
fn pearson(a: &[f64], b: &[f64]) -> Option<f64> {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        sab += (x - ma) * (y - mb);
        saa += (x - ma) * (x - ma);
        sbb += (y - mb) * (y - mb);
    }
    (saa > 0.0 && sbb > 0.0).then(|| (sab / (saa * sbb).sqrt()).clamp(-1.0, 1.0))
}

/// Average ranks (ties share their mean rank).
fn ranks(v: &[f64]) -> Vec<f64> {
    let mut ix: Vec<usize> = (0..v.len()).collect();
    ix.sort_by(|&i, &j| v[i].total_cmp(&v[j]));
    let mut r = vec![0.0; v.len()];
    let mut k = 0;
    while k < ix.len() {
        let mut e = k;
        while e + 1 < ix.len() && v[ix[e + 1]] == v[ix[k]] {
            e += 1;
        }
        let avg = (k + e) as f64 / 2.0 + 1.0;
        for &i in &ix[k..=e] {
            r[i] = avg;
        }
        k = e + 1;
    }
    r
}

fn spearman(a: &[f64], b: &[f64]) -> Option<f64> {
    pearson(&ranks(a), &ranks(b))
}

/// `, width 16 at a → 25.3 at b (widens)` for a band.
fn write_width(f: &mut std::fmt::Formatter<'_>, s: &SeriesSummary) -> std::fmt::Result {
    let Some((a, b)) = s.band_width else {
        return Ok(());
    };
    let trend = if (b - a).abs() <= 0.02 * a.abs().max(b.abs()) {
        "about even"
    } else if b > a {
        "widens"
    } else {
        "narrows"
    };
    write!(
        f,
        ", width {} at {} → {} at {} ({trend})",
        num(a),
        s.x_from,
        num(b),
        s.x_to
    )
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
            // a numeric / date x: its extent, not the row order (a scatter's
            // rows are in no x order)
            let xs: Vec<(usize, f64)> = pts
                .iter()
                .filter(|(i, _)| !matches!(x[*i], Value::Str(_)))
                .filter_map(|(i, _)| x[*i].as_f64().map(|v| (*i, v)))
                .collect();
            let (ia, ib) = if xs.len() == pts.len() {
                let a = xs.iter().fold(xs[0], |a, b| if b.1 < a.1 { *b } else { a });
                let b = xs.iter().fold(xs[0], |a, b| if b.1 > a.1 { *b } else { a });
                (a.0, b.0)
            } else {
                (i0, i1)
            };
            let (x_from, x_to) = (xl(ia), xl(ib));
            let width = |i: usize| {
                let lo = ymin?.get(i)?.as_f64()?;
                let hi = ymax?.get(i)?.as_f64()?;
                (lo.is_finite() && hi.is_finite()).then_some(hi - lo)
            };
            let band_width = band
                .as_ref()
                .and_then(|b| b.as_ref())
                .and_then(|_| width(ia).zip(width(ib)))
                .filter(|_| ia != ib);
            // a scatter or bubble chart: how y (and the size) go with x
            let cor =
                (geom.starts_with("point") && xs.len() == pts.len() && pts.len() >= MIN_COR_POINTS)
                    .then(|| {
                        let xv: Vec<f64> = xs.iter().map(|p| p.1).collect();
                        let yv: Vec<f64> = pts.iter().map(|p| p.1).collect();
                        let size_rho = d.column("size").and_then(|sc| {
                            let sv: Vec<f64> = pts
                                .iter()
                                .map(|(i, _)| sc[*i].as_f64().filter(|v| v.is_finite()))
                                .collect::<Option<_>>()?;
                            Some((spearman(&sv, &xv)?, spearman(&sv, &yv)?))
                        });
                        Some(Correlation {
                            r: pearson(&xv, &yv)?,
                            rho: spearman(&xv, &yv)?,
                            size_rho,
                        })
                    })
                    .flatten();
            // the shape along x: a numeric / date x in x order, a discrete
            // one as drawn; a scatter (point layer) has no line to follow
            let ordered: Vec<(f64, String)> = if geom.starts_with("point") {
                Vec::new()
            } else if xs.len() == pts.len() {
                let mut v: Vec<(f64, f64, usize)> = xs
                    .iter()
                    .zip(&pts)
                    .map(|(xv, p)| (xv.1, p.1, p.0))
                    .collect();
                v.sort_by(|a, b| a.0.total_cmp(&b.0));
                v.into_iter().map(|(_, yv, i)| (yv, xl(i))).collect()
            } else {
                pts.iter().map(|(i, v)| (*v, xl(*i))).collect()
            };
            let (counter_move, peaks) = shape(&ordered);
            Some(SeriesSummary {
                counter_move,
                peaks,
                key: key.map(|_| k),
                n: pts.len(),
                x_from,
                x_to,
                y_first: y0,
                y_last: y1,
                y_min: (lo.1, xl(lo.0)),
                y_max: (hi.1, xl(hi.0)),
                band: band.flatten(),
                band_width,
                points: pts.iter().map(|(i, v)| (xl(*i), *v)).collect(),
                cor,
            })
        })
        .collect();
    let leaders = leaders(&series);
    (!series.is_empty()).then(|| LayerSummary {
        geom: geom.to_string(),
        panel: None,
        series,
        leaders,
    })
}

/// The largest move against the net direction and the prominent peaks of
/// `ys` (in x order), see [`SeriesSummary::counter_move`] / [`SeriesSummary::peaks`].
fn shape(ys: &[(f64, String)]) -> (Option<Move>, Vec<(f64, String)>) {
    let n = ys.len();
    if n < MIN_SHAPE_POINTS {
        return (None, Vec::new());
    }
    let (lo, hi) = ys
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), (y, _)| {
            (a.min(*y), b.max(*y))
        });
    let range = hi - lo;
    if range <= 0.0 {
        return (None, Vec::new());
    }
    // against the net direction: a fall when it ends higher (or level), a rise when lower
    let up = ys[n - 1].0 >= ys[0].0;
    let (mut best, mut ext, mut at) = (0.0, ys[0].0, 0);
    let mut mv = None;
    for (j, (y, _)) in ys.iter().enumerate() {
        let d = if up { ext - y } else { y - ext };
        if d > best {
            best = d;
            mv = Some((at, j));
        }
        if (up && *y > ext) || (!up && *y < ext) {
            ext = *y;
            at = j;
        }
    }
    let counter_move = mv.filter(|_| best >= 0.1 * range).map(|(a, b)| Move {
        from: ys[a].clone(),
        to: ys[b].clone(),
    });
    // peaks by prominence: how far it stands above the higher of the two
    // lowest points between it and the next higher mark on either side
    let mut peaks = Vec::new();
    if n <= 5000 {
        for i in 0..n {
            let y = ys[i].0;
            let left_ok = i == 0 || ys[i - 1].0 < y;
            let right_ok = i + 1 == n || ys[i + 1].0 <= y;
            if !(left_ok && right_ok) || (i == 0 && n > 1 && ys[1].0 == y) {
                continue;
            }
            let side = |it: &mut dyn Iterator<Item = usize>| {
                let mut m = y;
                for k in it {
                    if ys[k].0 > y {
                        return m;
                    }
                    m = m.min(ys[k].0);
                }
                m
            };
            let l = side(&mut (0..i).rev());
            let r = side(&mut (i + 1..n));
            // an end of the series counts only by its inner side
            let base = if i == 0 {
                r
            } else if i + 1 == n {
                l
            } else {
                l.max(r)
            };
            if y - base >= 0.25 * range {
                peaks.push(ys[i].clone());
            }
        }
        peaks.sort_by(|a, b| b.0.total_cmp(&a.0));
    }
    (counter_move, peaks)
}

/// The first layer of each panel, when every panel draws it as one series:
/// those series, keyed by panel, compared like the series of one layer.
fn panel_leaders(layers: &[LayerSummary]) -> Option<(String, Leaders)> {
    let geom = &layers.first()?.geom;
    let mut seen = Vec::new();
    let mut series = Vec::new();
    for l in layers.iter().filter(|l| &l.geom == geom) {
        let p = l.panel.clone()?;
        if seen.contains(&p) || l.series.len() != 1 {
            return None;
        }
        seen.push(p.clone());
        series.push(SeriesSummary {
            key: Some(p),
            ..l.series[0].clone()
        });
    }
    Some((geom.clone(), leaders(&series)?))
}

/// Which series is highest at each x two or more series share.
fn leaders(series: &[SeriesSummary]) -> Option<Leaders> {
    if series.len() < 2 || series.iter().any(|s| s.key.is_none()) {
        return None;
    }
    let maps: Vec<std::collections::HashMap<&str, f64>> = series
        .iter()
        .map(|s| s.points.iter().map(|(x, y)| (x.as_str(), *y)).collect())
        .collect();
    let mut count: Vec<usize> = vec![0; series.len()];
    let mut shared = 0;
    for (x, _) in &series[0].points {
        let ys: Option<Vec<f64>> = maps.iter().map(|m| m.get(x.as_str()).copied()).collect();
        let Some(ys) = ys else { continue };
        shared += 1;
        let top = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        for (k, y) in ys.iter().enumerate() {
            if *y == top {
                count[k] += 1;
            }
        }
    }
    if shared < 3 {
        return None;
    }
    let mut out: Vec<(String, usize)> = series
        .iter()
        .zip(count)
        .filter(|(_, c)| *c > 0)
        .map(|(s, c)| (s.key.clone().unwrap_or_default(), c))
        .collect();
    out.sort_by_key(|o| std::cmp::Reverse(o.1));
    Some((out, shared))
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
    fn a_scatter_says_how_y_and_the_size_go_with_x() {
        let f = |v: &[f64]| v.iter().map(|x| Value::Float(*x)).collect::<Vec<_>>();
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let y = [2.0, 3.0, 5.0, 4.0, 7.0, 8.0];
        // the size falls as y rises (big districts sit low)
        let size = [90.0, 80.0, 50.0, 60.0, 20.0, 10.0];
        let df = vec![
            ("x".to_string(), f(&x)),
            ("y".to_string(), f(&y)),
            ("s".to_string(), f(&size)),
        ];
        let built = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y").size("s"))
            .geom_point()
            .build();
        let c = built.summary().layers[0].series[0].cor.clone().unwrap();
        assert!(c.r > 0.9 && c.rho > 0.9, "{c:?}");
        let (sx, sy) = c.size_rho.unwrap();
        assert!(sx < -0.9 && sy < -0.9, "{c:?}");
        let text = built.summary_text();
        assert!(
            text.contains("; y rises strongly with x (r 0.95, ρ 0.94)"),
            "{text}"
        );
        assert!(
            text.contains("size falls strongly with x (ρ -0.94), falls strongly with y (ρ -1)"),
            "{text}"
        );

        // too few marks, or a line: no correlation
        let df = vec![("x".to_string(), f(&x[..4])), ("y".to_string(), f(&y[..4]))];
        let few = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .build();
        assert!(few.summary().layers[0].series[0].cor.is_none());
        let df = vec![("x".to_string(), f(&x)), ("y".to_string(), f(&y))];
        let line = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y"))
            .geom_line()
            .build();
        assert!(line.summary().layers[0].series[0].cor.is_none());
        assert!(!line.summary_text().contains(" with x"));
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
            (a.n, a.x_from.as_str(), a.x_to.as_str()),
            (3, "2023-11-15", "2023-11-17")
        );
        assert_eq!(a.y_max, (5.0, "2023-11-16".to_string()));
        assert_eq!(a.y_min, (1.0, "2023-11-17".to_string()));
        let smooth = &s.layers[1];
        assert_eq!(smooth.geom, "smooth");
        // the fitted grid's x is a date too, and its band is reported
        assert!(
            smooth.series[0].x_from.starts_with("2023-11-15"),
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
    fn scatter_x_extent_ribbon_band_and_markers_merged() {
        let df = vec![
            ("x".to_string(), [5.0, 1.0, 9.0].map(Value::Float).to_vec()),
            ("y".to_string(), [2.0, 4.0, 3.0].map(Value::Float).to_vec()),
            ("lo".to_string(), [1.0, 3.0, 2.0].map(Value::Float).to_vec()),
            ("hi".to_string(), [3.0, 6.0, 4.0].map(Value::Float).to_vec()),
        ];
        let t = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y").ymin("lo").ymax("hi"))
            .geom_ribbon()
            .geom_line()
            .geom_point()
            .build()
            .summary_text();
        let lines: Vec<&str> = t.lines().collect();
        // widths: x=1 → 6−3 = 3, x=9 → 4−2 = 2
        assert_eq!(
            lines[0], "ribbon: 3 marks, x 1 … 9, band 1 … 6, width 3 at 1 → 2 at 9 (narrows)",
            "{t}"
        );
        assert!(
            lines[1].starts_with("line+point: 3 marks, x 1 … 9; y first"),
            "{t}"
        );
        assert_eq!(lines.len(), 2, "{t}");
    }

    #[test]
    fn two_series_over_the_same_x_are_compared() {
        let m: Vec<Value> = (1..=3).map(|i| Value::Float(i as f64)).collect();
        let df = vec![
            ("x".to_string(), [m.clone(), m].concat()),
            (
                "y".to_string(),
                [100.0, 200.0, 50.0, 130.0, 220.0, 51.0]
                    .map(Value::Float)
                    .to_vec(),
            ),
            (
                "g".to_string(),
                strs(&["2024", "2024", "2024", "2025", "2025", "2025"]),
            ),
        ];
        let t = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_line()
            .build()
            .summary_text();
        assert!(
            t.ends_with("2025 − 2024: largest +30 (+30 %) at 1, smallest +1 (+2 %) at 3"),
            "{t}"
        );
    }

    #[test]
    fn a_comparison_by_percent_when_it_ranks_differently() {
        let df = vec![
            (
                "x".to_string(),
                [1.0, 2.0, 1.0, 2.0].map(Value::Float).to_vec(),
            ),
            (
                "y".to_string(),
                [10.0, 200.0, 15.0, 260.0].map(Value::Float).to_vec(),
            ),
            ("g".to_string(), strs(&["a", "a", "b", "b"])),
        ];
        let t = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_line()
            .build()
            .summary_text();
        assert!(
            t.ends_with("by %: largest +5 (+50 %) at 1, smallest +60 (+30 %) at 2"),
            "{t}"
        );
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

    fn line(ys: &[f64]) -> String {
        let df = vec![
            (
                "x".to_string(),
                (1..=ys.len()).map(|i| Value::Float(i as f64)).collect(),
            ),
            (
                "y".to_string(),
                ys.iter().map(|v| Value::Float(*v)).collect(),
            ),
        ];
        GGPlot::new(df)
            .aes(Aes::new().x("x").y("y"))
            .geom_line()
            .build()
            .summary_text()
    }

    #[test]
    fn a_rise_with_a_dip_names_the_dip() {
        let t = line(&[1.0, 2.0, 5.0, 3.0, 4.0, 6.0]);
        assert!(
            t.contains("; against the trend: largest fall 5 at 3 → 3 at 4"),
            "{t}"
        );
        let t = line(&[9.0, 8.0, 4.0, 7.0, 3.0, 1.0]);
        assert!(t.contains("largest rise 4 at 3 → 7 at 4"), "{t}");
    }

    #[test]
    fn a_steady_or_nearly_steady_series_has_no_counter_move() {
        assert!(!line(&[1.0, 2.0, 3.0, 4.0, 5.0]).contains("against the trend"));
        // a wobble under a tenth of the range is not named
        assert!(!line(&[0.0, 50.0, 49.0, 100.0]).contains("against the trend"));
    }

    #[test]
    fn two_peaks_are_named_highest_first() {
        let t = line(&[0.0, 10.0, 0.0, 0.0, 6.0, 0.0, 0.0]);
        assert!(t.contains("; 2 peaks: 10 at 2, 6 at 5"), "{t}");
        // a small bump is not a peak
        assert!(!line(&[0.0, 10.0, 0.0, 1.0, 0.0]).contains("peaks"));
    }

    #[test]
    fn a_scatter_has_no_shape() {
        let df = vec![
            (
                "x".to_string(),
                vec![3.0, 1.0, 2.0, 5.0, 4.0]
                    .into_iter()
                    .map(Value::Float)
                    .collect(),
            ),
            (
                "y".to_string(),
                vec![1.0, 5.0, 2.0, 4.0, 3.0]
                    .into_iter()
                    .map(Value::Float)
                    .collect(),
            ),
        ];
        let s = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .build()
            .summary();
        assert!(s.layers[0].series[0].counter_move.is_none());
        assert!(s.layers[0].series[0].peaks.is_empty());
    }

    #[test]
    fn the_highest_series_per_x_is_counted() {
        let df = vec![
            (
                "x".to_string(),
                strs(&["a", "b", "c", "d", "a", "b", "c", "d"]),
            ),
            (
                "y".to_string(),
                vec![5.0, 6.0, 7.0, 1.0, 4.0, 5.0, 6.0, 2.0]
                    .into_iter()
                    .map(Value::Float)
                    .collect(),
            ),
            (
                "g".to_string(),
                strs(&["WI", "WI", "WI", "WI", "CA", "CA", "CA", "CA"]),
            ),
        ];
        let t = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_line()
            .build()
            .summary_text();
        assert!(
            t.contains("; highest at each of 4 shared x: WI 3, CA 1"),
            "{t}"
        );
    }

    #[test]
    fn facet_panels_are_compared_per_x() {
        let df = vec![
            (
                "x".to_string(),
                vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0]
                    .into_iter()
                    .map(Value::Float)
                    .collect(),
            ),
            (
                "y".to_string(),
                vec![5.0, 1.0, 7.0, 4.0, 6.0, 2.0]
                    .into_iter()
                    .map(Value::Float)
                    .collect(),
            ),
            (
                "store".to_string(),
                strs(&["WI", "WI", "WI", "CA", "CA", "CA"]),
            ),
        ];
        let t = GGPlot::new(df)
            .aes(Aes::new().x("x").y("y"))
            .geom_line()
            .facet_wrap("store", None)
            .build()
            .summary_text();
        assert!(
            t.contains("\nline across panels: highest at each of 3 shared x: WI 2, CA 1"),
            "{t}"
        );
    }

    #[test]
    fn at_most_three_peaks_are_listed() {
        let t = line(&[0.0, 9.0, 0.0, 8.0, 0.0, 7.0, 0.0, 10.0, 0.0]);
        assert!(t.contains("; 4 peaks: 10 at 8, 9 at 2, 8 at 4, …"), "{t}");
    }
}
