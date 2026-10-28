//! `StatHeikinAshi`: Heikin-Ashi transform of OHLC rows, for
//! `geom_heikin_ashi` (candles over the smoothed values).
//!
//! Rows are taken in `x` order; for each period
//! `HA close = (o + h + l + c) / 4`, `HA open = (prev HA open + prev HA
//! close) / 2` (the first period: `(o + c) / 2`), `HA high = max(h, HA open,
//! HA close)` and `HA low = min(l, HA open, HA close)`. The `open`/`high`/
//! `low`/`close` columns are replaced; every other column is carried.

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Heikin-Ashi transform (see the module docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct StatHeikinAshi;

/// Heikin-Ashi `(open, high, low, close)` for consecutive OHLC periods.
pub fn heikin_ashi(ohlc: &[(f64, f64, f64, f64)]) -> Vec<(f64, f64, f64, f64)> {
    let mut out = Vec::with_capacity(ohlc.len());
    let mut prev: Option<(f64, f64)> = None;
    for &(o, h, l, c) in ohlc {
        let close = (o + h + l + c) / 4.0;
        let open = match prev {
            Some((po, pc)) => (po + pc) / 2.0,
            None => (o + c) / 2.0,
        };
        out.push((open, h.max(open).max(close), l.min(open).min(close), close));
        prev = Some((open, close));
    }
    out
}

impl Stat for StatHeikinAshi {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let cols: Option<[&[Value]; 5]> = (|| {
            Some([
                data.column("x")?,
                data.column("open")?,
                data.column("high")?,
                data.column("low")?,
                data.column("close")?,
            ])
        })();
        let Some([x, o, h, l, c]) = cols else {
            return DataFrame::new();
        };
        let key = |v: &Value| v.as_f64().unwrap_or(f64::NAN);
        let mut order: Vec<usize> = (0..data.nrows())
            .filter(|&i| {
                [&o[i], &h[i], &l[i], &c[i]]
                    .iter()
                    .all(|v| v.as_f64().is_some())
            })
            .collect();
        order.sort_by(|&a, &b| key(&x[a]).total_cmp(&key(&x[b])));
        let ha = heikin_ashi(
            &order
                .iter()
                .map(|&i| (key(&o[i]), key(&h[i]), key(&l[i]), key(&c[i])))
                .collect::<Vec<_>>(),
        );
        let mut out = DataFrame::new();
        for name in data.column_names() {
            let vals: Vec<Value> = match name {
                "open" => ha.iter().map(|v| Value::Float(v.0)).collect(),
                "high" => ha.iter().map(|v| Value::Float(v.1)).collect(),
                "low" => ha.iter().map(|v| Value::Float(v.2)).collect(),
                "close" => ha.iter().map(|v| Value::Float(v.3)).collect(),
                _ => {
                    let col = data.column(name).unwrap_or_default();
                    order.iter().map(|&i| col[i].clone()).collect()
                }
            };
            out.add_column(name.to_string(), vals);
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![
            Aesthetic::X,
            Aesthetic::Open,
            Aesthetic::High,
            Aesthetic::Low,
            Aesthetic::Close,
        ]
    }

    /// Each period depends on the previous one: the whole series at once.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "heikin_ashi"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_reference_formulas() {
        let ha = heikin_ashi(&[(10.0, 12.0, 9.0, 11.0), (11.0, 13.0, 10.0, 12.0)]);
        assert_eq!(ha[0], (10.5, 12.0, 9.0, 10.5));
        // open = (10.5 + 10.5) / 2, close = (11 + 13 + 10 + 12) / 4 = 11.5.
        assert_eq!(ha[1], (10.5, 13.0, 10.0, 11.5));
    }
}
