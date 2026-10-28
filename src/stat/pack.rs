//! `StatPack`: packed-bubble / circle-packing layout (d3-hierarchy's
//! `packSiblings` front-chain algorithm by Wang et al., with Welzl's
//! smallest enclosing circle), deterministic.
//!
//! Each row is a circle with area ∝ `weight` (1 when unmapped). Without a
//! `subgroup` mapping all circles are packed together; with one, each
//! group is packed on its own, wrapped in its enclosing circle, and the
//! group circles are packed in turn (Highcharts' clustered packed bubble).
//!
//! Output: one row per circle (`part = "leaf"`, every input column carried
//! through) then one per group (`part = "group"`, `subgroup` set), each
//! with centre `x`/`y` and radius `r` in layout units (a group row's
//! `fill` is its first member's), plus constant
//! `xmin`/`xmax`/`ymin`/`ymax` framing the whole pack as a square.

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Circle {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

fn place(b: Circle, a: Circle, c: &mut Circle) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let d2 = dx * dx + dy * dy;
    if d2 > 0.0 {
        let a2 = (a.r + c.r).powi(2);
        let b2 = (b.r + c.r).powi(2);
        if a2 > b2 {
            let x = (d2 + b2 - a2) / (2.0 * d2);
            let y = (b2 / d2 - x * x).max(0.0).sqrt();
            c.x = b.x - x * dx - y * dy;
            c.y = b.y - x * dy + y * dx;
        } else {
            let x = (d2 + a2 - b2) / (2.0 * d2);
            let y = (a2 / d2 - x * x).max(0.0).sqrt();
            c.x = a.x + x * dx - y * dy;
            c.y = a.y + x * dy + y * dx;
        }
    } else {
        c.x = a.x + c.r;
        c.y = a.y;
    }
}

fn intersects(a: Circle, b: Circle) -> bool {
    let dr = a.r + b.r - 1e-6;
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    dr > 0.0 && dr * dr > dx * dx + dy * dy
}

/// Pack `circles` (radii set; positions overwritten) around the origin
/// without overlap; returns the radius of their enclosing circle, which is
/// centred on the origin afterwards.
pub(crate) fn pack_siblings(circles: &mut [Circle]) -> f64 {
    let n = circles.len();
    if n == 0 {
        return 0.0;
    }
    circles[0].x = 0.0;
    circles[0].y = 0.0;
    if n == 1 {
        return circles[0].r;
    }
    circles[0].x = -circles[1].r;
    circles[1].x = circles[0].r;
    circles[1].y = 0.0;
    if n == 2 {
        return circles[0].r + circles[1].r;
    }
    let (c0, c1) = (circles[0], circles[1]);
    place(c1, c0, &mut circles[2]);

    // Front chain as a doubly linked list over circle indices.
    let mut next = vec![usize::MAX; n];
    let mut prev = vec![usize::MAX; n];
    let (mut a, mut b) = (0usize, 1usize);
    next[0] = 1;
    prev[1] = 0;
    next[1] = 2;
    prev[2] = 1;
    next[2] = 0;
    prev[0] = 2;
    let score = |c: &[Circle], i: usize, next: &[usize]| {
        let (p, q) = (c[i], c[next[i]]);
        let ab = p.r + q.r;
        let dx = (p.x * q.r + q.x * p.r) / ab;
        let dy = (p.y * q.r + q.y * p.r) / ab;
        dx * dx + dy * dy
    };

    let mut i = 3;
    'pack: while i < n {
        let (ca, cb) = (circles[a], circles[b]);
        place(ca, cb, &mut circles[i]);
        let c = i;
        let (mut j, mut k) = (next[b], prev[a]);
        let (mut sj, mut sk) = (circles[b].r, circles[a].r);
        loop {
            if sj <= sk {
                if intersects(circles[j], circles[c]) {
                    b = j;
                    next[a] = b;
                    prev[b] = a;
                    continue 'pack;
                }
                sj += circles[j].r;
                j = next[j];
            } else {
                if intersects(circles[k], circles[c]) {
                    a = k;
                    next[a] = b;
                    prev[b] = a;
                    continue 'pack;
                }
                sk += circles[k].r;
                k = prev[k];
            }
            if j == next[k] {
                break;
            }
        }
        // Insert c between a and b.
        prev[c] = a;
        next[c] = b;
        next[a] = c;
        prev[b] = c;
        b = c;
        // New closest pair to the centroid.
        let mut best = score(circles, a, &next);
        let mut cur = next[c];
        while cur != b {
            let s = score(circles, cur, &next);
            if s < best {
                a = cur;
                best = s;
            }
            cur = next[cur];
        }
        b = next[a];
        i += 1;
    }

    let mut chain = vec![circles[b]];
    let mut cur = next[b];
    while cur != b {
        chain.push(circles[cur]);
        cur = next[cur];
    }
    let e = enclose(&chain);
    for c in circles.iter_mut() {
        c.x -= e.x;
        c.y -= e.y;
    }
    e.r
}

fn encloses_not(a: Circle, b: Circle) -> bool {
    let dr = a.r - b.r;
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    dr < 0.0 || dr * dr < dx * dx + dy * dy
}

fn encloses_weak(a: Circle, b: Circle) -> bool {
    let dr = a.r - b.r + a.r.max(b.r).max(1.0) * 1e-9;
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    dr > 0.0 && dr * dr > dx * dx + dy * dy
}

fn encloses_weak_all(a: Circle, set: &[Circle]) -> bool {
    set.iter().all(|&b| encloses_weak(a, b))
}

fn basis2(a: Circle, b: Circle) -> Circle {
    let (x21, y21, r21) = (b.x - a.x, b.y - a.y, b.r - a.r);
    let l = (x21 * x21 + y21 * y21).sqrt();
    if l == 0.0 {
        return if a.r >= b.r { a } else { b };
    }
    Circle {
        x: (a.x + b.x + x21 / l * r21) / 2.0,
        y: (a.y + b.y + y21 / l * r21) / 2.0,
        r: (l + a.r + b.r) / 2.0,
    }
}

fn basis3(a: Circle, b: Circle, c: Circle) -> Circle {
    let (x1, y1, r1) = (a.x, a.y, a.r);
    let (a2, a3, b2, b3) = (x1 - b.x, x1 - c.x, y1 - b.y, y1 - c.y);
    let (c2, c3) = (b.r - r1, c.r - r1);
    let d1 = x1 * x1 + y1 * y1 - r1 * r1;
    let d2 = d1 - b.x * b.x - b.y * b.y + b.r * b.r;
    let d3 = d1 - c.x * c.x - c.y * c.y + c.r * c.r;
    let ab = a3 * b2 - a2 * b3;
    let xa = (b2 * d3 - b3 * d2) / (ab * 2.0) - x1;
    let xb = (b3 * c2 - b2 * c3) / ab;
    let ya = (a3 * d2 - a2 * d3) / (ab * 2.0) - y1;
    let yb = (a2 * c3 - a3 * c2) / ab;
    let qa = xb * xb + yb * yb - 1.0;
    let qb = 2.0 * (r1 + xa * xb + ya * yb);
    let qc = xa * xa + ya * ya - r1 * r1;
    let r = -(if qa.abs() > 1e-6 {
        (qb + (qb * qb - 4.0 * qa * qc).sqrt()) / (2.0 * qa)
    } else {
        qc / qb
    });
    Circle {
        x: x1 + xa + xb * r,
        y: y1 + ya + yb * r,
        r,
    }
}

fn basis(set: &[Circle]) -> Circle {
    match set {
        [a] => *a,
        [a, b] => basis2(*a, *b),
        [a, b, c] => basis3(*a, *b, *c),
        _ => Circle {
            x: 0.0,
            y: 0.0,
            r: 0.0,
        },
    }
}

fn extend_basis(set: &[Circle], p: Circle) -> Vec<Circle> {
    if encloses_weak_all(p, set) {
        return vec![p];
    }
    for &b in set {
        if encloses_not(p, b) && encloses_weak_all(basis2(b, p), set) {
            return vec![b, p];
        }
    }
    for i in 0..set.len().saturating_sub(1) {
        for j in i + 1..set.len() {
            let (bi, bj) = (set[i], set[j]);
            if encloses_not(basis2(bi, bj), p)
                && encloses_not(basis2(bi, p), bj)
                && encloses_not(basis2(bj, p), bi)
                && encloses_weak_all(basis3(bi, bj, p), set)
            {
                return vec![bi, bj, p];
            }
        }
    }
    // Numerical corner case: fall back to the pair basis.
    vec![set[0], p]
}

/// Smallest circle enclosing `circles` (Welzl / d3's `packEnclose`), with a
/// fixed-seed shuffle so the result is deterministic.
pub(crate) fn enclose(circles: &[Circle]) -> Circle {
    let mut c = circles.to_vec();
    // Fisher–Yates with a fixed LCG (d3 uses a seeded LCG too).
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for i in (1..c.len()).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = (state >> 33) as usize % (i + 1);
        c.swap(i, j);
    }
    let (mut i, mut set, mut e): (usize, Vec<Circle>, Option<Circle>) = (0, Vec::new(), None);
    let mut guard = 0usize;
    while i < c.len() {
        let p = c[i];
        if e.is_some_and(|e| encloses_weak(e, p)) {
            i += 1;
        } else {
            set = extend_basis(&set, p);
            e = Some(basis(&set));
            i = 0;
            guard += 1;
            if guard > 10 * c.len() * c.len() + 100 {
                break;
            }
        }
    }
    e.unwrap_or(Circle {
        x: 0.0,
        y: 0.0,
        r: 0.0,
    })
}

/// One packed group: its subgroup value, `(row, radius)` members, their
/// circles and the enclosing radius.
type PackedGroup = (Value, Vec<(usize, f64)>, Vec<Circle>, f64);

/// Packed-bubble stat behind [`GeomPackedBubble`](crate::geom::pack::GeomPackedBubble).
#[derive(Clone, Debug)]
pub struct StatPack {
    /// Pack larger circles first (tighter, more regular packs).
    pub sort: bool,
    /// Gap between circles as a fraction of the mean leaf radius.
    pub padding: f64,
}

impl Default for StatPack {
    fn default() -> Self {
        StatPack {
            sort: true,
            padding: 0.08,
        }
    }
}

impl Stat for StatPack {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let weight = data.column("weight");
        let rows: Vec<(usize, f64)> = (0..data.nrows())
            .filter_map(|i| {
                let w = match weight {
                    Some(c) => c[i].as_f64()?,
                    None => 1.0,
                };
                (w.is_finite() && w > 0.0).then_some((i, w.sqrt()))
            })
            .collect();
        if rows.is_empty() {
            return DataFrame::new();
        }
        let mean_r = rows.iter().map(|r| r.1).sum::<f64>() / rows.len() as f64;
        let pad = self.padding * mean_r;

        let sub = data.column("subgroup");
        let mut groups: indexmap::IndexMap<String, (Value, Vec<(usize, f64)>)> =
            indexmap::IndexMap::new();
        for &(i, r) in &rows {
            let (key, val) = match sub {
                Some(c) => (c[i].key_str().into_owned(), c[i].clone()),
                None => (String::new(), Value::Na),
            };
            groups
                .entry(key)
                .or_insert_with(|| (val, Vec::new()))
                .1
                .push((i, r));
        }

        let pack = |members: &mut Vec<(usize, f64)>| -> (Vec<Circle>, f64) {
            if self.sort {
                members.sort_by(|a, b| b.1.total_cmp(&a.1));
            }
            let mut circles: Vec<Circle> = members
                .iter()
                .map(|&(_, r)| Circle {
                    x: 0.0,
                    y: 0.0,
                    r: r + pad / 2.0,
                })
                .collect();
            let outer = pack_siblings(&mut circles);
            for c in circles.iter_mut() {
                c.r -= pad / 2.0;
            }
            (circles, outer)
        };

        let mut leaves: Vec<(usize, Circle)> = Vec::new();
        let mut group_rows: Vec<(Value, Circle, usize)> = Vec::new();
        let grouped = sub.is_some();
        let mut packed: Vec<PackedGroup> = groups
            .into_values()
            .map(|(val, mut m)| {
                let (c, outer) = pack(&mut m);
                (val, m, c, outer)
            })
            .collect();
        if grouped {
            let mut outers: Vec<Circle> = packed
                .iter()
                .map(|g| Circle {
                    x: 0.0,
                    y: 0.0,
                    r: g.3 + pad * 1.5,
                })
                .collect();
            pack_siblings(&mut outers);
            for (g, o) in packed.iter_mut().zip(&outers) {
                for c in g.2.iter_mut() {
                    c.x += o.x;
                    c.y += o.y;
                }
                group_rows.push((g.0.clone(), Circle { r: g.3, ..*o }, g.1[0].0));
            }
        }
        for g in &packed {
            for (&(i, _), &c) in g.1.iter().zip(&g.2) {
                leaves.push((i, c));
            }
        }
        let all: Vec<Circle> = leaves
            .iter()
            .map(|l| l.1)
            .chain(group_rows.iter().map(|g| g.1))
            .collect();
        let half = all
            .iter()
            .map(|c| c.x.abs().max(c.y.abs()) + c.r)
            .fold(0.0, f64::max);

        let n = leaves.len() + group_rows.len();
        let mut out = DataFrame::new();
        let mut part = vec![Value::Str("leaf".into()); leaves.len()];
        part.extend(std::iter::repeat_n(
            Value::Str("group".into()),
            group_rows.len(),
        ));
        out.add_column("part".into(), part);
        out.add_column("x".into(), all.iter().map(|c| Value::Float(c.x)).collect());
        out.add_column("y".into(), all.iter().map(|c| Value::Float(c.y)).collect());
        out.add_column("r".into(), all.iter().map(|c| Value::Float(c.r)).collect());
        for (c, v) in [
            ("xmin", -half),
            ("xmax", half),
            ("ymin", -half),
            ("ymax", half),
        ] {
            out.add_column(c.into(), vec![Value::Float(v); n]);
        }
        for name in data.column_names() {
            if out.has_column(name) {
                continue;
            }
            let col = data.column(name).unwrap_or_default();
            let mut vals: Vec<Value> = leaves.iter().map(|l| col[l.0].clone()).collect();
            if name == "subgroup" {
                vals.extend(group_rows.iter().map(|g| g.0.clone()));
            } else if name == "fill" {
                // Unfilled, but must not train an NA level: first member's.
                vals.extend(group_rows.iter().map(|g| col[g.2].clone()));
            } else {
                vals.extend(std::iter::repeat_n(Value::Na, group_rows.len()));
            }
            out.add_column(name.to_string(), vals);
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        Vec::new()
    }

    /// One pack per panel.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "pack"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_overlap(c: &[Circle]) {
        for i in 0..c.len() {
            for j in i + 1..c.len() {
                let d = (c[i].x - c[j].x).hypot(c[i].y - c[j].y);
                assert!(d >= c[i].r + c[j].r - 1e-6, "{i} and {j} overlap: {d}");
            }
        }
    }

    #[test]
    fn siblings_never_overlap_and_fit_their_enclosure() {
        let mut c: Vec<Circle> = (1..=40)
            .map(|k| Circle {
                x: 0.0,
                y: 0.0,
                r: ((k * 37 % 11) as f64 + 1.0).sqrt(),
            })
            .collect();
        let outer = pack_siblings(&mut c);
        no_overlap(&c);
        for k in &c {
            assert!(k.x.hypot(k.y) + k.r <= outer + 1e-6);
        }
        // Reasonably tight: the enclosure isn't far above the area bound.
        let area: f64 = c.iter().map(|k| k.r * k.r).sum();
        assert!(outer * outer < 2.2 * area, "{outer}");
    }

    #[test]
    fn enclose_three_known_circles() {
        let e = enclose(&[
            Circle {
                x: -1.0,
                y: 0.0,
                r: 1.0,
            },
            Circle {
                x: 1.0,
                y: 0.0,
                r: 1.0,
            },
            Circle {
                x: 0.0,
                y: 0.0,
                r: 0.5,
            },
        ]);
        assert!((e.r - 2.0).abs() < 1e-9 && e.x.abs() < 1e-9 && e.y.abs() < 1e-9);
    }

    #[test]
    fn grouped_leaves_sit_inside_their_group_circle() {
        let mut d = DataFrame::new();
        let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
        d.add_column("subgroup".into(), s(&["A", "A", "A", "B", "B", "C"]));
        d.add_column(
            "weight".into(),
            [9.0, 4.0, 1.0, 16.0, 2.0, 5.0]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        );
        let out = StatPack::default().compute_group(&d, &ScaleSet::new());
        assert_eq!(out.nrows(), 6 + 3);
        let f = |c: &str, i: usize| out.column(c).unwrap()[i].as_f64().unwrap();
        let sub = |i: usize| out.column("subgroup").unwrap()[i].key_str().into_owned();
        let leaves: Vec<Circle> = (0..6)
            .map(|i| Circle {
                x: f("x", i),
                y: f("y", i),
                r: f("r", i),
            })
            .collect();
        no_overlap(&leaves);
        for i in 0..6 {
            let g = (6..9).find(|&g| sub(g) == sub(i)).unwrap();
            let d = (f("x", i) - f("x", g)).hypot(f("y", i) - f("y", g));
            assert!(
                d + f("r", i) <= f("r", g) + 1e-6,
                "leaf {i} outside its group"
            );
        }
        // Area ∝ weight: radius 3 vs 2 for weights 9 and 4.
        let r: Vec<f64> = (0..3).map(|i| f("r", i)).collect();
        assert!((r[0] / r[1] - 1.5).abs() < 0.05, "{r:?}");
    }
}
