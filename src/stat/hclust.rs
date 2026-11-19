//! `hclust`: agglomerative hierarchical clustering of a distance matrix — the
//! leaf order a heatmap's rows and columns follow so that similar items sit
//! side by side (R's `heatmap()`, `corrplot(order = "hclust")`,
//! `ggcorrplot(hc.order = TRUE)`), and the merges a dendrogram draws
//! ([`dendrogram_links`](super::tree::dendrogram_links) takes them as they
//! are).
//!
//! The merges follow R's `hclust` convention: merge `k` (1-based) joins two
//! children, `-(i + 1)` for item `i` and `m` for merge `m`. Ties merge the
//! pair found first (lowest indices), so the result is deterministic.
//!
//! The leaf order goes one step beyond R's: when two clusters merge, each may
//! be reversed so that the two leaves that meet are the closest of the four
//! possible pairs — a cheap leaf-ordering pass that keeps neighbours
//! across a merge similar too (the heatmap's blocks join smoothly).

/// How the distance between two clusters is measured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Linkage {
    /// The mean distance over all pairs (UPGMA, R's `"average"`).
    #[default]
    Average,
    /// The largest pairwise distance (R's `"complete"`).
    Complete,
    /// The smallest pairwise distance (R's `"single"`).
    Single,
}

/// The result of [`hclust`].
#[derive(Clone, Debug, PartialEq)]
pub struct Hclust {
    /// One pair of children per merge (see the module docs for the
    /// numbering), `n - 1` of them.
    pub merge: Vec<(i64, i64)>,
    /// The distance each merge happened at, non-decreasing for average,
    /// complete and single linkage.
    pub heights: Vec<f64>,
    /// The items in leaf order (a permutation of `0..n`).
    pub order: Vec<usize>,
}

/// Cluster `n` items from their `n × n` distance matrix (symmetric; only the
/// upper triangle is read). A missing or non-finite distance counts as the
/// largest finite one; a ragged row's missing entries too. No items: an
/// empty result; one item: no merges and the order `[0]`.
pub fn hclust(dist: &[Vec<f64>], linkage: Linkage) -> Hclust {
    let n = dist.len();
    let raw = |i: usize, j: usize| {
        let (a, b) = if i < j { (i, j) } else { (j, i) };
        dist[a].get(b).copied().filter(|d| d.is_finite())
    };
    let worst = (0..n)
        .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
        .filter_map(|(i, j)| raw(i, j))
        .fold(0.0_f64, f64::max);
    let d = |i: usize, j: usize| raw(i, j).unwrap_or(worst);

    // Active clusters: their id in the merge numbering, size and leaves (in
    // leaf order), and the current cluster-to-cluster distances.
    let mut ids: Vec<i64> = (0..n).map(|i| -(i as i64 + 1)).collect();
    let mut size: Vec<usize> = vec![1; n];
    let mut leaves: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
    let mut alive: Vec<bool> = vec![true; n];
    let mut cd: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| if i == j { 0.0 } else { d(i, j) }).collect())
        .collect();
    let mut merge = Vec::with_capacity(n.saturating_sub(1));
    let mut heights = Vec::with_capacity(n.saturating_sub(1));
    for k in 1..n {
        // the closest pair of live clusters, the first found on a tie
        let mut best: Option<(usize, usize, f64)> = None;
        for i in 0..n {
            if !alive[i] {
                continue;
            }
            for j in i + 1..n {
                if alive[j] && best.is_none_or(|(_, _, b)| cd[i][j] < b) {
                    best = Some((i, j, cd[i][j]));
                }
            }
        }
        let Some((a, b, h)) = best else { break };
        // R lists a singleton before a merge, two singletons by item and two
        // merges by age
        let pair = if (ids[a] < 0) == (ids[b] < 0) {
            if ids[a] < 0 {
                (ids[a].max(ids[b]), ids[a].min(ids[b]))
            } else {
                (ids[a].min(ids[b]), ids[a].max(ids[b]))
            }
        } else {
            (ids[a].min(ids[b]), ids[a].max(ids[b]))
        };
        merge.push(pair);
        heights.push(h);
        // the leaf order: orient both halves so the leaves that meet are
        // the closest of the four candidates
        let (la, lb) = (
            std::mem::take(&mut leaves[a]),
            std::mem::take(&mut leaves[b]),
        );
        let ends = |l: &[usize]| (l[0], l[l.len() - 1]);
        let ((a0, a1), (b0, b1)) = (ends(&la), ends(&lb));
        let options = [
            (false, false, d(a1, b0)),
            (false, true, d(a1, b1)),
            (true, false, d(a0, b0)),
            (true, true, d(a0, b1)),
        ];
        let (ra, rb, _) =
            options
                .iter()
                .copied()
                .fold(options[0], |acc, o| if o.2 < acc.2 { o } else { acc });
        let mut joined: Vec<usize> = if ra {
            la.into_iter().rev().collect()
        } else {
            la
        };
        if rb {
            joined.extend(lb.into_iter().rev());
        } else {
            joined.extend(lb);
        }
        // the new cluster lives on in slot a
        for x in 0..n {
            if !alive[x] || x == a || x == b {
                continue;
            }
            let (da, db) = (cd[a][x], cd[b][x]);
            let v = match linkage {
                Linkage::Average => {
                    (da * size[a] as f64 + db * size[b] as f64) / (size[a] + size[b]) as f64
                }
                Linkage::Complete => da.max(db),
                Linkage::Single => da.min(db),
            };
            cd[a][x] = v;
            cd[x][a] = v;
        }
        alive[b] = false;
        size[a] += size[b];
        ids[a] = k as i64;
        leaves[a] = joined;
    }
    let order = (0..n)
        .find(|&i| alive[i])
        .map(|i| leaves[i].clone())
        .unwrap_or_default();
    Hclust {
        merge,
        heights,
        order,
    }
}

/// The leaf order of `n` items from their `n × n` *similarity* matrix (a
/// correlation's absolute value, a normalized mutual information): distance
/// = largest similarity − similarity, average linkage.
pub fn similarity_order(sim: &[Vec<f64>]) -> Vec<usize> {
    let n = sim.len();
    let top = sim
        .iter()
        .enumerate()
        .flat_map(|(i, r)| {
            r.iter()
                .enumerate()
                .filter(move |(j, _)| *j != i)
                .map(|(_, v)| *v)
        })
        .filter(|v| v.is_finite())
        .fold(f64::NEG_INFINITY, f64::max);
    let dist: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let s = sim[i].get(j).copied().unwrap_or(f64::NAN);
                    if s.is_finite() {
                        top - s
                    } else {
                        f64::NAN
                    }
                })
                .collect()
        })
        .collect();
    hclust(&dist, Linkage::Average).order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(rows: &[&[f64]]) -> Vec<Vec<f64>> {
        rows.iter().map(|r| r.to_vec()).collect()
    }

    #[test]
    fn empty_and_single() {
        assert_eq!(hclust(&[], Linkage::Average).order, Vec::<usize>::new());
        let one = hclust(&m(&[&[0.0]]), Linkage::Average);
        assert_eq!(one.order, vec![0]);
        assert!(one.merge.is_empty());
    }

    // R: hclust(as.dist(d), "average") on the same matrix
    #[test]
    fn merges_and_heights_as_r() {
        let d = m(&[
            &[0.0, 2.0, 6.0, 10.0, 9.0],
            &[2.0, 0.0, 5.0, 9.0, 8.0],
            &[6.0, 5.0, 0.0, 4.0, 5.0],
            &[10.0, 9.0, 4.0, 0.0, 3.0],
            &[9.0, 8.0, 5.0, 3.0, 0.0],
        ]);
        let h = hclust(&d, Linkage::Average);
        assert_eq!(h.merge, vec![(-1, -2), (-4, -5), (-3, 2), (1, 3)]);
        assert_eq!(h.heights[..3], [2.0, 3.0, 4.5]);
        // (6 + 10 + 9 + 5 + 9 + 8) / 6
        assert!((h.heights[3] - 47.0 / 6.0).abs() < 1e-12);
        // the two blocks side by side, 1 next to 2 and 2 next to 3
        assert_eq!(h.order, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn complete_and_single() {
        let d = m(&[&[0.0, 1.0, 4.0], &[1.0, 0.0, 2.0], &[4.0, 2.0, 0.0]]);
        assert_eq!(hclust(&d, Linkage::Complete).heights, vec![1.0, 4.0]);
        assert_eq!(hclust(&d, Linkage::Single).heights, vec![1.0, 2.0]);
    }

    // shuffled blocks come back together
    #[test]
    fn similarity_order_groups_blocks() {
        // items 0, 2, 4 are one block, 1 and 3 another
        let s = |i: usize, j: usize| {
            if i == j {
                1.0
            } else if i % 2 == j % 2 {
                0.8
            } else {
                0.1
            }
        };
        let sim: Vec<Vec<f64>> = (0..5).map(|i| (0..5).map(|j| s(i, j)).collect()).collect();
        let o = similarity_order(&sim);
        let pos = |x: usize| o.iter().position(|&v| v == x).unwrap();
        let mut evens = [pos(0), pos(2), pos(4)];
        evens.sort();
        assert_eq!(
            evens[2] - evens[0],
            2,
            "the even block is contiguous: {o:?}"
        );
    }

    #[test]
    fn missing_distances_count_as_the_largest() {
        let d = m(&[
            &[0.0, f64::NAN, 1.0],
            &[f64::NAN, 0.0, 3.0],
            &[1.0, 3.0, 0.0],
        ]);
        let h = hclust(&d, Linkage::Single);
        assert_eq!(h.merge[0], (-1, -3));
        assert_eq!(h.heights, vec![1.0, 3.0]);
    }
}
