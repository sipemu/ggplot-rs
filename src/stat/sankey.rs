//! `StatSankey`: node/link layout for a Sankey diagram from an edge list.
//!
//! Input: one row per link with `from`, `to` and an optional `weight` (1 when
//! unmapped); repeated (from, to) pairs are summed. Nodes are placed in
//! columns by their longest path from a source (Highcharts' layout), sized by
//! `max(inflow, outflow)`, stacked with padding and centred per column; a few
//! barycentre sweeps reorder each column to reduce crossings.
//!
//! Output: one row per node (`part = "node"`) then one per link
//! (`part = "link"`), both as boxes in layout units: x is the column index,
//! y the flow value. A node is `xmin..xmax × ymin..ymax`; a link leaves its
//! source at `x = xmin, ymin..ymax` and enters its target at
//! `x = xmax, yend_min..yend_max`. Also `node` (node name; the source for a
//! link), `from`, `to`, `value` and `column`.

use std::collections::HashMap;

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Sankey layout stat behind [`GeomSankey`](crate::geom::sankey::GeomSankey).
#[derive(Clone, Debug)]
pub struct StatSankey {
    /// Node width in column units (columns are 1 apart).
    pub node_width: f64,
    /// Vertical gap between nodes as a fraction of the tallest column's flow.
    pub node_padding: f64,
    /// Move nodes without outgoing links to the last column (d3's
    /// `sankeyJustify`); `false` keeps them at their depth.
    pub justify: bool,
    /// Barycentre sweeps used to reorder nodes (0 keeps first-appearance order).
    pub iterations: usize,
}

impl Default for StatSankey {
    fn default() -> Self {
        StatSankey {
            node_width: 0.12,
            node_padding: 0.08,
            justify: false,
            iterations: 6,
        }
    }
}

struct Layout {
    names: Vec<String>,
    column: Vec<usize>,
    value: Vec<f64>,
    top: Vec<f64>,
    /// Aggregated links: (from, to, value).
    links: Vec<(usize, usize, f64)>,
}

impl StatSankey {
    fn layout(&self, data: &DataFrame) -> Option<Layout> {
        let (from, to) = (data.column("from")?, data.column("to")?);
        let weight = data.column("weight");
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut names = Vec::new();
        let mut id = |v: &Value| -> Option<usize> {
            if v.is_na() {
                return None;
            }
            let key = v.key_str().into_owned();
            Some(*index.entry(key.clone()).or_insert_with(|| {
                names.push(key);
                names.len() - 1
            }))
        };
        let mut pairs: indexmap::IndexMap<(usize, usize), f64> = indexmap::IndexMap::new();
        for i in 0..data.nrows() {
            let (Some(a), Some(b)) = (id(&from[i]), id(&to[i])) else {
                continue;
            };
            let w = match weight {
                Some(c) => match c[i].as_f64() {
                    Some(w) if w.is_finite() && w > 0.0 => w,
                    _ => continue,
                },
                None => 1.0,
            };
            if a != b {
                *pairs.entry((a, b)).or_insert(0.0) += w;
            }
        }
        let n = names.len();
        if n == 0 {
            return None;
        }
        let links: Vec<(usize, usize, f64)> =
            pairs.into_iter().map(|((a, b), w)| (a, b, w)).collect();

        // Column = longest path from a source (capped, so a cycle terminates).
        let mut column = vec![0usize; n];
        for _ in 0..n {
            let mut changed = false;
            for &(a, b, _) in &links {
                if column[b] < column[a] + 1 && column[a] + 1 < n {
                    column[b] = column[a] + 1;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let (mut inflow, mut outflow) = (vec![0.0; n], vec![0.0; n]);
        for &(a, b, w) in &links {
            outflow[a] += w;
            inflow[b] += w;
        }
        let last = column.iter().copied().max().unwrap_or(0);
        if self.justify {
            for k in 0..n {
                if outflow[k] == 0.0 {
                    column[k] = last;
                }
            }
        }
        let value: Vec<f64> = (0..n).map(|k| f64::max(inflow[k], outflow[k])).collect();

        let mut cols: Vec<Vec<usize>> = vec![Vec::new(); last + 1];
        for k in 0..n {
            cols[column[k]].push(k);
        }
        let col_total = |c: &Vec<usize>| c.iter().map(|&k| value[k]).sum::<f64>();
        let max_total = cols.iter().map(col_total).fold(0.0, f64::max);
        let pad = self.node_padding * max_total;
        let height = cols
            .iter()
            .map(|c| col_total(c) + pad * c.len().saturating_sub(1) as f64)
            .fold(0.0, f64::max);

        let mut top = vec![0.0; n];
        let stack = |c: &Vec<usize>, top: &mut Vec<f64>| {
            let h = col_total(c) + pad * c.len().saturating_sub(1) as f64;
            let mut y = height - (height - h) / 2.0;
            for &k in c {
                top[k] = y;
                y -= value[k] + pad;
            }
        };
        for c in &cols {
            stack(c, &mut top);
        }
        for _ in 0..self.iterations {
            for forward in [true, false] {
                let order: Vec<usize> = if forward {
                    (1..cols.len()).collect()
                } else {
                    (0..cols.len().saturating_sub(1)).rev().collect()
                };
                for ci in order {
                    let center = |k: usize| top[k] - value[k] / 2.0;
                    let mut keyed: Vec<(f64, usize)> = cols[ci]
                        .iter()
                        .map(|&k| {
                            let (mut sw, mut sy) = (0.0, 0.0);
                            for &(a, b, w) in &links {
                                let other = if forward && b == k {
                                    a
                                } else if !forward && a == k {
                                    b
                                } else {
                                    continue;
                                };
                                sw += w;
                                sy += w * center(other);
                            }
                            (if sw > 0.0 { sy / sw } else { center(k) }, k)
                        })
                        .collect();
                    // Higher barycentre first (y grows upward; stacking runs top-down).
                    keyed.sort_by(|a, b| b.0.total_cmp(&a.0));
                    cols[ci] = keyed.into_iter().map(|(_, k)| k).collect();
                    stack(&cols[ci], &mut top);
                }
            }
        }
        Some(Layout {
            names,
            column,
            value,
            top,
            links,
        })
    }
}

impl Stat for StatSankey {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let Some(l) = self.layout(data) else {
            return DataFrame::new();
        };
        let n = l.names.len();
        let half = self.node_width / 2.0;
        let center = |k: usize| l.top[k] - l.value[k] / 2.0;

        let mut rows: Vec<[f64; 6]> = Vec::new();
        let mut text: Vec<[String; 4]> = Vec::new();
        let mut value = Vec::new();
        let mut column = Vec::new();
        for k in 0..n {
            let x = l.column[k] as f64;
            let (lo, hi) = (l.top[k] - l.value[k], l.top[k]);
            rows.push([x - half, x + half, lo, hi, lo, hi]);
            let name = l.names[k].clone();
            text.push(["node".into(), name.clone(), name.clone(), name]);
            value.push(l.value[k]);
            column.push(l.column[k] as f64);
        }
        // Outgoing links stack down from the source's top in target order,
        // incoming links down from the target's top in source order.
        let mut out_off = vec![0.0; n];
        let mut in_off = vec![0.0; n];
        let mut by_source: Vec<usize> = (0..l.links.len()).collect();
        by_source.sort_by(|&i, &j| {
            let (a1, b1, _) = l.links[i];
            let (a2, b2, _) = l.links[j];
            a1.cmp(&a2).then(center(b2).total_cmp(&center(b1)))
        });
        let mut src_band = vec![(0.0, 0.0); l.links.len()];
        for i in by_source {
            let (a, _, w) = l.links[i];
            let hi = l.top[a] - out_off[a];
            src_band[i] = (hi - w, hi);
            out_off[a] += w;
        }
        let mut by_target: Vec<usize> = (0..l.links.len()).collect();
        by_target.sort_by(|&i, &j| {
            let (a1, b1, _) = l.links[i];
            let (a2, b2, _) = l.links[j];
            b1.cmp(&b2).then(center(a2).total_cmp(&center(a1)))
        });
        let mut dst_band = vec![(0.0, 0.0); l.links.len()];
        for i in by_target {
            let (_, b, w) = l.links[i];
            let hi = l.top[b] - in_off[b];
            dst_band[i] = (hi - w, hi);
            in_off[b] += w;
        }
        for (i, &(a, b, w)) in l.links.iter().enumerate() {
            let (x0, x1) = (l.column[a] as f64 + half, l.column[b] as f64 - half);
            rows.push([
                x0,
                x1,
                src_band[i].0,
                src_band[i].1,
                dst_band[i].0,
                dst_band[i].1,
            ]);
            let (fa, tb) = (l.names[a].clone(), l.names[b].clone());
            text.push(["link".into(), fa.clone(), fa, tb]);
            value.push(w);
            column.push(l.column[a] as f64);
        }

        let num = |k: usize| rows.iter().map(|r| Value::Float(r[k])).collect::<Vec<_>>();
        let txt = |k: usize| {
            text.iter()
                .map(|r| Value::Str(r[k].clone()))
                .collect::<Vec<_>>()
        };
        let mut out = DataFrame::new();
        out.add_column("part".into(), txt(0));
        out.add_column("xmin".into(), num(0));
        out.add_column("xmax".into(), num(1));
        out.add_column("ymin".into(), num(2));
        out.add_column("ymax".into(), num(3));
        out.add_column("yend_min".into(), num(4));
        out.add_column("yend_max".into(), num(5));
        out.add_column("node".into(), txt(1));
        out.add_column("from".into(), txt(2));
        out.add_column("to".into(), txt(3));
        out.add_column(
            "value".into(),
            value.into_iter().map(Value::Float).collect(),
        );
        out.add_column(
            "column".into(),
            column.into_iter().map(Value::Float).collect(),
        );
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::From, Aesthetic::To]
    }

    /// The layout needs every link of the panel at once.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "sankey"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edges(e: &[(&str, &str, f64)]) -> DataFrame {
        let mut d = DataFrame::new();
        d.add_column(
            "from".into(),
            e.iter().map(|r| Value::Str(r.0.to_string())).collect(),
        );
        d.add_column(
            "to".into(),
            e.iter().map(|r| Value::Str(r.1.to_string())).collect(),
        );
        d.add_column(
            "weight".into(),
            e.iter().map(|r| Value::Float(r.2)).collect(),
        );
        d
    }

    #[test]
    fn columns_values_and_conservation() {
        let d = edges(&[
            ("Coal", "Power", 40.0),
            ("Gas", "Power", 30.0),
            ("Gas", "Heat", 20.0),
            ("Power", "Homes", 50.0),
            ("Power", "Industry", 20.0),
            ("Gas", "Power", 5.0),
        ]);
        let l = StatSankey::default().layout(&d).unwrap();
        let idx = |n: &str| l.names.iter().position(|x| x == n).unwrap();
        assert_eq!(l.column[idx("Coal")], 0);
        assert_eq!(l.column[idx("Power")], 1);
        assert_eq!(l.column[idx("Homes")], 2);
        assert_eq!(l.column[idx("Heat")], 1);
        assert_eq!(l.value[idx("Power")], 75.0, "max(in 75, out 70)");
        assert_eq!(l.value[idx("Gas")], 55.0, "repeated pair summed");
        assert_eq!(l.links.len(), 5);

        let out = StatSankey::default().compute_group(&d, &ScaleSet::new());
        assert_eq!(out.nrows(), 6 + 5);
        // Every link band lies inside its source and target nodes.
        let f = |c: &str| -> Vec<f64> {
            out.column(c)
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        };
        let (ymin, ymax, emin, emax) = (f("ymin"), f("ymax"), f("yend_min"), f("yend_max"));
        let s = |c: &str, i: usize| out.column(c).unwrap()[i].key_str().into_owned();
        let node_row = |n: &str| (0..6).find(|&i| s("node", i) == n).unwrap();
        for i in 6..11 {
            let (a, b) = (node_row(&s("from", i)), node_row(&s("to", i)));
            assert!(ymin[i] >= ymin[a] - 1e-9 && ymax[i] <= ymax[a] + 1e-9);
            assert!(emin[i] >= ymin[b] - 1e-9 && emax[i] <= ymax[b] + 1e-9);
        }
    }

    #[test]
    fn justify_moves_sinks_to_the_last_column() {
        let d = edges(&[("A", "B", 1.0), ("B", "C", 1.0), ("A", "D", 1.0)]);
        let l = StatSankey {
            justify: true,
            ..Default::default()
        }
        .layout(&d)
        .unwrap();
        let idx = |n: &str| l.names.iter().position(|x| x == n).unwrap();
        assert_eq!(l.column[idx("D")], 2);
    }

    #[test]
    fn cycles_terminate() {
        let d = edges(&[("A", "B", 1.0), ("B", "A", 1.0)]);
        assert!(StatSankey::default().layout(&d).is_some());
    }
}
