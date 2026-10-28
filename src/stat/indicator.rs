//! `StatIndicator`: technical-analysis indicators for price series, to
//! overlay on (or stack under) candlestick / line charts:
//! `geom_line().stat(StatIndicator::new(Indicator::Ema(20)))`.
//!
//! Rows are taken in `x` order. The price is `close` when mapped, else `y`;
//! ATR and the stochastic oscillator also need `high`/`low`, VWAP uses
//! `weight` as volume. Output keeps `x` and puts the indicator in `y`
//! (Bollinger bands also in `ymin`/`ymax`); warm-up rows without a value
//! are dropped. Smoothing follows the TA-Lib conventions: EMA seeded with
//! the SMA of its first `n` values, RSI and ATR with Wilder's smoothing.

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Which MACD series to emit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacdPart {
    Line,
    Signal,
    Histogram,
}

/// A technical indicator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Indicator {
    /// Simple moving average over `n` periods.
    Sma(usize),
    /// Exponential moving average (α = 2/(n+1), SMA-seeded).
    Ema(usize),
    /// Linearly weighted moving average.
    Wma(usize),
    /// Bollinger bands: SMA(n) ± k population standard deviations.
    Bollinger(usize, f64),
    /// MACD(fast, slow, signal).
    Macd(usize, usize, usize, MacdPart),
    /// Relative strength index (Wilder), 0–100.
    Rsi(usize),
    /// Average true range (Wilder); needs `high`/`low`/`close`.
    Atr(usize),
    /// Volume-weighted average price, cumulative; volume from `weight`.
    Vwap,
    /// Stochastic %K over `k` periods smoothed by an SMA of `d` (`d = 1`
    /// for raw %K); needs `high`/`low`/`close`. 0–100.
    Stochastic(usize, usize),
    /// ZigZag pivots: points where the price reverses by at least `pct` %.
    ZigZag(f64),
}

/// Indicator stat (see the module docs).
#[derive(Clone, Debug)]
pub struct StatIndicator {
    pub indicator: Indicator,
}

impl StatIndicator {
    pub fn new(indicator: Indicator) -> Self {
        StatIndicator { indicator }
    }
}

/// Simple moving average; `None` during the warm-up.
pub fn sma(v: &[f64], n: usize) -> Vec<Option<f64>> {
    let n = n.max(1);
    let mut out = vec![None; v.len()];
    let mut sum = 0.0;
    for i in 0..v.len() {
        sum += v[i];
        if i >= n {
            sum -= v[i - n];
        }
        if i + 1 >= n {
            out[i] = Some(sum / n as f64);
        }
    }
    out
}

/// EMA with α = 2/(n+1), seeded with the SMA of the first `n` values.
pub fn ema(v: &[f64], n: usize) -> Vec<Option<f64>> {
    smoothed(v, n, 2.0 / (n.max(1) as f64 + 1.0))
}

/// Exponential smoothing with factor `alpha`, seeded with an `n`-SMA.
fn smoothed(v: &[f64], n: usize, alpha: f64) -> Vec<Option<f64>> {
    let n = n.max(1);
    let mut out = vec![None; v.len()];
    if v.len() < n {
        return out;
    }
    let mut prev = v[..n].iter().sum::<f64>() / n as f64;
    out[n - 1] = Some(prev);
    for i in n..v.len() {
        prev += alpha * (v[i] - prev);
        out[i] = Some(prev);
    }
    out
}

/// Linearly weighted moving average (weights 1..n, newest heaviest).
pub fn wma(v: &[f64], n: usize) -> Vec<Option<f64>> {
    let n = n.max(1);
    let denom = (n * (n + 1)) as f64 / 2.0;
    (0..v.len())
        .map(|i| {
            (i + 1 >= n).then(|| {
                (0..n)
                    .map(|k| v[i + 1 - n + k] * (k + 1) as f64)
                    .sum::<f64>()
                    / denom
            })
        })
        .collect()
}

/// RSI with Wilder's smoothing (α = 1/n), seeded with the mean gain/loss of
/// the first `n` changes.
pub fn rsi(v: &[f64], n: usize) -> Vec<Option<f64>> {
    let n = n.max(1);
    let mut out = vec![None; v.len()];
    if v.len() <= n {
        return out;
    }
    let ch: Vec<f64> = v.windows(2).map(|w| w[1] - w[0]).collect();
    let (mut gain, mut loss) = (0.0, 0.0);
    for &c in &ch[..n] {
        gain += c.max(0.0);
        loss += (-c).max(0.0);
    }
    gain /= n as f64;
    loss /= n as f64;
    let value = |g: f64, l: f64| {
        if l == 0.0 {
            if g == 0.0 {
                50.0
            } else {
                100.0
            }
        } else {
            100.0 - 100.0 / (1.0 + g / l)
        }
    };
    out[n] = Some(value(gain, loss));
    for i in n..ch.len() {
        gain = (gain * (n as f64 - 1.0) + ch[i].max(0.0)) / n as f64;
        loss = (loss * (n as f64 - 1.0) + (-ch[i]).max(0.0)) / n as f64;
        out[i + 1] = Some(value(gain, loss));
    }
    out
}

impl Stat for StatIndicator {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let Some(x) = data.column("x") else {
            return DataFrame::new();
        };
        let num = |name: &str| data.column(name);
        let price_col = num("close").or_else(|| num("y"));
        let Some(price_col) = price_col else {
            return DataFrame::new();
        };
        let key = |v: &Value| v.as_f64().unwrap_or(f64::NAN);
        let mut order: Vec<usize> = (0..data.nrows())
            .filter(|&i| price_col[i].as_f64().is_some_and(f64::is_finite))
            .collect();
        order.sort_by(|&a, &b| key(&x[a]).total_cmp(&key(&x[b])));
        let take = |c: Option<&[Value]>| -> Option<Vec<f64>> {
            let c = c?;
            Some(
                order
                    .iter()
                    .map(|&i| c[i].as_f64().unwrap_or(f64::NAN))
                    .collect(),
            )
        };
        let p = take(Some(price_col)).unwrap_or_default();
        let (high, low) = (take(num("high")), take(num("low")));
        let n_rows = p.len();
        // (y, ymin, ymax) per sorted row.
        let mut val: Vec<(Option<f64>, Option<f64>, Option<f64>)> =
            vec![(None, None, None); n_rows];
        let set_y = |val: &mut Vec<(Option<f64>, Option<f64>, Option<f64>)>,
                     ys: Vec<Option<f64>>| {
            for (v, y) in val.iter_mut().zip(ys) {
                v.0 = y;
            }
        };
        match self.indicator {
            Indicator::Sma(n) => set_y(&mut val, sma(&p, n)),
            Indicator::Ema(n) => set_y(&mut val, ema(&p, n)),
            Indicator::Wma(n) => set_y(&mut val, wma(&p, n)),
            Indicator::Bollinger(n, k) => {
                let mid = sma(&p, n);
                for i in 0..n_rows {
                    if let Some(m) = mid[i] {
                        let w = &p[i + 1 - n.max(1)..=i];
                        let sd = (w.iter().map(|v| (v - m).powi(2)).sum::<f64>() / w.len() as f64)
                            .sqrt();
                        val[i] = (Some(m), Some(m - k * sd), Some(m + k * sd));
                    }
                }
            }
            Indicator::Macd(fast, slow, signal, part) => {
                let (f, s) = (ema(&p, fast), ema(&p, slow));
                let line: Vec<Option<f64>> =
                    f.iter().zip(&s).map(|(a, b)| Some((*a)? - (*b)?)).collect();
                // Signal: EMA of the MACD line over its defined stretch.
                let start = line.iter().position(Option::is_some).unwrap_or(n_rows);
                let defined: Vec<f64> = line[start..].iter().map(|v| v.unwrap_or(0.0)).collect();
                let mut sig = vec![None; start];
                sig.extend(ema(&defined, signal));
                let out: Vec<Option<f64>> = match part {
                    MacdPart::Line => line.iter().zip(&sig).map(|(l, s)| s.and(*l)).collect(),
                    MacdPart::Signal => sig,
                    MacdPart::Histogram => line
                        .iter()
                        .zip(&sig)
                        .map(|(l, s)| Some((*l)? - (*s)?))
                        .collect(),
                };
                set_y(&mut val, out);
            }
            Indicator::Rsi(n) => set_y(&mut val, rsi(&p, n)),
            Indicator::Atr(n) => {
                if let (Some(h), Some(l)) = (&high, &low) {
                    let tr: Vec<f64> = (0..n_rows)
                        .map(|i| {
                            let hl = h[i] - l[i];
                            if i == 0 {
                                hl
                            } else {
                                hl.max((h[i] - p[i - 1]).abs()).max((l[i] - p[i - 1]).abs())
                            }
                        })
                        .collect();
                    set_y(&mut val, smoothed(&tr, n, 1.0 / n.max(1) as f64));
                }
            }
            Indicator::Vwap => {
                let vol = take(num("weight")).unwrap_or_else(|| vec![1.0; n_rows]);
                let (mut pv, mut vv) = (0.0, 0.0);
                let out = (0..n_rows)
                    .map(|i| {
                        let typical = match (&high, &low) {
                            (Some(h), Some(l)) => (h[i] + l[i] + p[i]) / 3.0,
                            _ => p[i],
                        };
                        pv += typical * vol[i];
                        vv += vol[i];
                        (vv > 0.0).then(|| pv / vv)
                    })
                    .collect();
                set_y(&mut val, out);
            }
            Indicator::Stochastic(k, d) => {
                if let (Some(h), Some(l)) = (&high, &low) {
                    let k = k.max(1);
                    let raw: Vec<Option<f64>> = (0..n_rows)
                        .map(|i| {
                            (i + 1 >= k).then(|| {
                                let hh = h[i + 1 - k..=i].iter().copied().fold(f64::MIN, f64::max);
                                let ll = l[i + 1 - k..=i].iter().copied().fold(f64::MAX, f64::min);
                                if hh > ll {
                                    100.0 * (p[i] - ll) / (hh - ll)
                                } else {
                                    50.0
                                }
                            })
                        })
                        .collect();
                    let start = raw.iter().position(Option::is_some).unwrap_or(n_rows);
                    let defined: Vec<f64> = raw[start..].iter().map(|v| v.unwrap_or(0.0)).collect();
                    let mut out = vec![None; start];
                    out.extend(sma(&defined, d));
                    set_y(&mut val, out);
                }
            }
            Indicator::ZigZag(pct) => {
                let th = pct.abs() / 100.0;
                if n_rows > 0 {
                    // Track the running extreme; confirm a pivot when the
                    // price retraces `th` from it.
                    let mut pivots = vec![0usize];
                    let mut ext = 0usize;
                    let mut dir = 0i8;
                    for i in 1..n_rows {
                        match dir {
                            0 => {
                                if p[i] >= p[0] * (1.0 + th) {
                                    dir = 1;
                                    ext = i;
                                } else if p[i] <= p[0] * (1.0 - th) {
                                    dir = -1;
                                    ext = i;
                                }
                            }
                            1 => {
                                if p[i] > p[ext] {
                                    ext = i;
                                } else if p[i] <= p[ext] * (1.0 - th) {
                                    pivots.push(ext);
                                    dir = -1;
                                    ext = i;
                                }
                            }
                            _ => {
                                if p[i] < p[ext] {
                                    ext = i;
                                } else if p[i] >= p[ext] * (1.0 + th) {
                                    pivots.push(ext);
                                    dir = 1;
                                    ext = i;
                                }
                            }
                        }
                    }
                    if *pivots.last().unwrap() != ext {
                        pivots.push(ext);
                    }
                    for i in pivots {
                        val[i].0 = Some(p[i]);
                    }
                }
            }
        }
        let keep: Vec<usize> = (0..n_rows)
            .filter(|&i| val[i].0.is_some_and(f64::is_finite))
            .collect();
        let mut out = DataFrame::new();
        out.add_column(
            "x".into(),
            keep.iter().map(|&i| x[order[i]].clone()).collect(),
        );
        out.add_column(
            "y".into(),
            keep.iter()
                .map(|&i| Value::Float(val[i].0.unwrap()))
                .collect(),
        );
        if matches!(self.indicator, Indicator::Bollinger(..)) {
            out.add_column(
                "ymin".into(),
                keep.iter()
                    .map(|&i| Value::Float(val[i].1.unwrap_or(f64::NAN)))
                    .collect(),
            );
            out.add_column(
                "ymax".into(),
                keep.iter()
                    .map(|&i| Value::Float(val[i].2.unwrap_or(f64::NAN)))
                    .collect(),
            );
        }
        for c in ["color", "fill", "group", "linetype"] {
            if let Some(col) = data.column(c) {
                out.add_column(
                    c.into(),
                    keep.iter().map(|&i| col[order[i]].clone()).collect(),
                );
            }
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    /// Indicators run over the whole series.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "indicator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|a| (a - b).abs() < 1e-9)
    }

    #[test]
    fn moving_averages() {
        let v = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(
            sma(&v, 3),
            vec![None, None, Some(2.0), Some(3.0), Some(4.0)]
        );
        let e = ema(&v, 3);
        // Seed SMA 2, α = 0.5: 3, 4.
        assert!(close(e[2], 2.0) && close(e[3], 3.0) && close(e[4], 4.0));
        let w = wma(&v, 3);
        assert!(close(w[2], (1.0 + 4.0 + 9.0) / 6.0));
    }

    #[test]
    fn rsi_extremes_and_wilder() {
        let up: Vec<f64> = (0..20).map(f64::from).collect();
        assert!(rsi(&up, 14).iter().flatten().all(|&r| r == 100.0));
        let flat = vec![5.0; 20];
        assert!(rsi(&flat, 14).iter().flatten().all(|&r| r == 50.0));
        // Alternating +1/−1: average gain = average loss → 50.
        let zig: Vec<f64> = (0..30)
            .map(|i| if i % 2 == 0 { 10.0 } else { 11.0 })
            .collect();
        let r = rsi(&zig, 14);
        assert!(r[14].is_some_and(|v| (v - 50.0).abs() < 4.0));
    }
}
