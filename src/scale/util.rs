/// Upper bound on label-start candidates per (q, j, k, z) in [`extended_breaks`].
/// Real inputs need a handful; only precision-degenerate ranges get near this.
const MAX_START_CANDIDATES: f64 = 10_000.0;

/// True when `[dmin, dmax]` is too narrow, relative to its magnitude, to place
/// distinct ticks on (floating-point noise around a constant value).
pub fn is_degenerate_range(dmin: f64, dmax: f64) -> bool {
    let range = (dmax - dmin).abs();
    range < f64::EPSILON || range <= 1e-10 * dmin.abs().max(dmax.abs())
}

/// Evenly spaced break values `start, start+step, …` up to `end` (inclusive,
/// with a small tolerance). Index-driven and capped, so it terminates even when
/// `step` is below the floating-point resolution of `start` (where repeated
/// `v += step` would never advance).
pub fn stepped_breaks(start: f64, end: f64, step: f64) -> Vec<f64> {
    const MAX_BREAKS: f64 = 1_000.0;
    if !(start.is_finite() && end.is_finite() && step.is_finite()) || step <= 0.0 {
        return vec![];
    }
    let n = ((end + step * 0.001 - start) / step).floor();
    if !(0.0..=MAX_BREAKS).contains(&n) {
        return vec![];
    }
    (0..=n as usize).map(|i| start + i as f64 * step).collect()
}

/// Extended-Wilkinson tick locations (Talbot, Lin & Hanrahan 2010), matching R's
/// `scales::extended_breaks()` / `labeling::extended()`. Returns "nice" break
/// values covering `[dmin, dmax]` with roughly `m` labels.
pub fn extended_breaks(dmin: f64, dmax: f64, m: usize) -> Vec<f64> {
    if !(dmin.is_finite() && dmax.is_finite()) || dmax <= dmin || m < 2 {
        return vec![];
    }
    // A range that is negligible relative to the magnitude of the data (e.g.
    // 1-ulp float noise from SUM/AVG) has no meaningful breaks, and the search
    // below would need more than 2^53 candidate starts to cover it.
    if is_degenerate_range(dmin, dmax) {
        return vec![];
    }
    // Preferred step mantissas and score weights (simplicity, coverage, density,
    // legibility) — the paper's defaults, as in the `labeling` package.
    const Q: [f64; 6] = [1.0, 5.0, 2.0, 2.5, 4.0, 3.0];
    const W: [f64; 4] = [0.25, 0.2, 0.5, 0.05];
    let m = m as f64;

    let simplicity = |qi: usize, j: f64, lmin: f64, lmax: f64, step: f64| {
        let eps = 1e-10;
        let modulo = lmin.rem_euclid(step);
        let v = if (modulo < eps || step - modulo < eps) && lmin <= 0.0 && lmax >= 0.0 {
            1.0
        } else {
            0.0
        };
        1.0 - qi as f64 / (Q.len() as f64 - 1.0) - j + v
    };
    let simplicity_max = |qi: usize, j: f64| 1.0 - qi as f64 / (Q.len() as f64 - 1.0) - j + 1.0;
    let coverage = |lmin: f64, lmax: f64| {
        let r = dmax - dmin;
        1.0 - 0.5 * ((dmax - lmax).powi(2) + (dmin - lmin).powi(2)) / (0.1 * r).powi(2)
    };
    let coverage_max = |span: f64| {
        let r = dmax - dmin;
        if span > r {
            let half = (span - r) / 2.0;
            1.0 - 0.5 * (half * half + half * half) / (0.1 * r).powi(2)
        } else {
            1.0
        }
    };
    let density = |k: f64, lmin: f64, lmax: f64| {
        let r = (k - 1.0) / (lmax - lmin);
        let rt = (m - 1.0) / (lmax.max(dmax) - lmin.min(dmin));
        2.0 - (r / rt).max(rt / r)
    };
    let density_max = |k: f64| {
        if k >= m {
            2.0 - (k - 1.0) / (m - 1.0)
        } else {
            1.0
        }
    };

    let mut best_score = -2.0;
    let mut best = (dmin, dmax, (dmax - dmin) / (m - 1.0));

    for j_i in 1..=4u32 {
        let j = j_i as f64;
        for (qi, &q) in Q.iter().enumerate() {
            let sm = simplicity_max(qi, j);
            if W[0] * sm + W[1] + W[2] + W[3] < best_score {
                break;
            }
            for k_i in 2..=(2.0 * m + 4.0) as usize {
                let k = k_i as f64;
                let dm = density_max(k);
                if W[0] * sm + W[1] + W[2] * dm + W[3] < best_score {
                    break;
                }
                let delta = (dmax - dmin) / (k + 1.0) / j / q;
                let z0 = delta.log10().ceil() as i32;
                for z in z0..(z0 + 6) {
                    let step = j * q * 10f64.powi(z);
                    let cm = coverage_max(step * (k - 1.0));
                    if W[0] * sm + W[1] * cm + W[2] * dm + W[3] < best_score {
                        break;
                    }
                    let min_start = (dmax / step).floor() * j - (k - 1.0) * j;
                    let max_start = (dmin / step).ceil() * j;
                    if min_start > max_start {
                        continue;
                    }
                    // Integer-driven so the loop always terminates: once
                    // `start` exceeds 2^53, `start += 1.0` no longer changes it.
                    let n_starts = max_start - min_start;
                    if !n_starts.is_finite() || n_starts > MAX_START_CANDIDATES {
                        continue;
                    }
                    for si in 0..=(n_starts as u64) {
                        let start = min_start + si as f64;
                        let lmin = start * step / j;
                        let lmax = lmin + step * (k - 1.0);
                        let s = simplicity(qi, j, lmin, lmax, step);
                        let c = coverage(lmin, lmax);
                        let g = density(k, lmin, lmax);
                        let score = W[0] * s + W[1] * c + W[2] * g + W[3];
                        if score > best_score {
                            best_score = score;
                            best = (lmin, lmax, step);
                        }
                    }
                }
            }
        }
    }

    let (lmin, lmax, step) = best;
    if step <= 0.0 {
        return vec![];
    }
    let n = ((lmax - lmin) / step).round() as usize;
    (0..=n).map(|i| lmin + i as f64 * step).collect()
}

/// Round ticks for a log-transformed axis, matching R's `scales::log_breaks()`:
/// prefers `base^n`; when the range spans too few decades for `m` ticks, mixes
/// in evenly-spaced sub-multiples of `base` (e.g. `{1,2,5} * 10^n` for base 10);
/// falls back to [`extended_breaks`] over the *data* range `[dmin, dmax]` when
/// neither gives enough ticks. `dmin`/`dmax` are in data (pre-transform) space
/// and must be finite and positive.
pub fn log_breaks(dmin: f64, dmax: f64, m: usize, base: f64) -> Vec<f64> {
    let m = m.max(2);
    if !(dmin.is_finite() && dmax.is_finite())
        || dmin <= 0.0
        || dmax <= 0.0
        || dmax < dmin
        || base <= 1.0
    {
        return extended_breaks(dmin, dmax, m);
    }
    if is_degenerate_range(dmin, dmax) {
        return vec![];
    }

    let log_min = dmin.log(base);
    let log_max = dmax.log(base);
    let min_e = log_min.floor();
    let max_e = log_max.ceil();
    if (max_e - min_e).abs() < 1e-9 {
        return vec![base.powf(min_e)];
    }

    let want = (m as i64 - 2).max(0) as f64;
    let exp_seq = |by: f64| -> Vec<f64> {
        let mut out = Vec::new();
        let mut e = min_e;
        while e <= max_e + 1e-9 {
            out.push(base.powf(e));
            e += by;
        }
        out
    };
    let count_relevant = |breaks: &[f64]| -> usize {
        let lo = base.powf(log_min) * (1.0 - 1e-9);
        let hi = base.powf(log_max) * (1.0 + 1e-9);
        breaks.iter().filter(|&&b| b >= lo && b <= hi).count()
    };

    let mut by = ((max_e - min_e) / m as f64).floor() + 1.0;
    loop {
        let breaks = exp_seq(by);
        if count_relevant(&breaks) as f64 >= want {
            return breaks;
        }
        if by <= 1.0 {
            break;
        }
        by -= 1.0;
    }

    log_sub_breaks(dmin, dmax, min_e, max_e, m, base)
}

/// Sub-decade ticks for [`log_breaks`] once whole-decade (or whole-`by`)
/// steps don't give enough of them — mirrors `scales:::log_sub_breaks()`.
fn log_sub_breaks(dmin: f64, dmax: f64, min_e: f64, max_e: f64, m: usize, base: f64) -> Vec<f64> {
    let want = (m as i64 - 2).max(0) as f64;
    let lo = dmin * (1.0 - 1e-9);
    let hi = dmax * (1.0 + 1e-9);

    if base <= 2.0 {
        let mut out = Vec::new();
        let mut e = min_e;
        while e <= max_e + 1e-9 {
            out.push(base.powf(e));
            e += 1.0;
        }
        return out;
    }

    let exps: Vec<i64> = ((min_e as i64)..=(max_e as i64)).collect();
    let mut steps: Vec<f64> = vec![1.0];
    // Integer sub-multiples strictly between 1 and `base` (R's
    // `seq_len(base)` filtered to `1 < candidate < base`).
    let mut candidates: Vec<f64> = (2..=(base.floor() as i64))
        .filter(|&x| (x as f64) < base)
        .map(|x| x as f64)
        .collect();

    // Greedily add whichever remaining candidate leaves the largest gap
    // (in log space) between consecutive steps, same as the R reference.
    let delta = |x: f64, steps: &[f64]| -> f64 {
        let mut vals: Vec<f64> = steps.to_vec();
        vals.push(x);
        vals.push(base);
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        vals.windows(2)
            .map(|w| w[1].log(base) - w[0].log(base))
            .fold(f64::INFINITY, f64::min)
    };

    let mut breaks: Vec<f64> = Vec::new();
    let mut relevant = 0usize;
    while !candidates.is_empty() {
        let (best_i, _) = candidates
            .iter()
            .enumerate()
            .map(|(i, &c)| (i, delta(c, &steps)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        steps.push(candidates.remove(best_i));
        breaks = exps
            .iter()
            .flat_map(|&e| steps.iter().map(move |&s| base.powf(e as f64) * s))
            .collect();
        relevant = breaks.iter().filter(|&&b| b >= lo && b <= hi).count();
        if relevant as f64 >= want {
            break;
        }
    }

    if relevant as f64 >= want && !breaks.is_empty() {
        breaks.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let first_in = breaks.iter().position(|&b| b >= lo);
        let last_in = breaks.iter().rposition(|&b| b <= hi);
        match (first_in, last_in) {
            (Some(fi), Some(li)) => {
                let start = fi.saturating_sub(1);
                let end = (li + 1).min(breaks.len() - 1);
                breaks[start..=end].to_vec()
            }
            _ => breaks,
        }
    } else {
        extended_breaks(dmin, dmax, m)
    }
}

/// Round step size to a "nice" value (1, 2, 5, 10, 20, 50, ...).
pub fn nice_step(raw: f64) -> f64 {
    let magnitude = 10f64.powf(raw.abs().log10().floor());
    let fraction = raw / magnitude;

    let nice = if fraction <= 1.5 {
        1.0
    } else if fraction <= 3.5 {
        2.0
    } else if fraction <= 7.5 {
        5.0
    } else {
        10.0
    };

    nice * magnitude
}

/// Format a number nicely, removing trailing zeros.
/// Format a set of break values together: like [`format_number`] (integers
/// verbatim, otherwise up to 2 decimals, trailing zeros trimmed), but with as
/// many extra decimals as needed so that distinct breaks get distinct labels
/// and no non-zero break prints as "0" — e.g. breaks 0.990/0.992/0.994 read
/// "0.99, 0.992, 0.994" instead of "0.99, 0.99, 0.99", and a log tick at 0.001
/// no longer reads "0". Output is unchanged whenever 2 decimals suffice.
pub fn format_numbers(values: &[f64]) -> Vec<String> {
    fn fmt(v: f64, decimals: usize) -> String {
        if v == v.round() && v.abs() < 1e10 {
            return format!("{}", v as i64);
        }
        let s = format!("{v:.decimals$}");
        let s = s.trim_end_matches('0').trim_end_matches('.');
        if s == "-0" {
            "0".to_string()
        } else {
            s.to_string()
        }
    }
    let ok = |labels: &[String]| {
        let distinct = values
            .iter()
            .zip(labels)
            .all(|(a, la)| values.iter().zip(labels).all(|(b, lb)| a == b || la != lb));
        let nonzero = values
            .iter()
            .zip(labels)
            .all(|(v, l)| *v == 0.0 || l != "0");
        distinct && nonzero
    };
    let mut labels = Vec::new();
    for decimals in 2..=12 {
        labels = values.iter().map(|&v| fmt(v, decimals)).collect();
        if ok(&labels) {
            break;
        }
    }
    labels
}

pub fn format_number(v: f64) -> String {
    if v == v.round() && v.abs() < 1e10 {
        format!("{}", v as i64)
    } else {
        let s = format!("{:.2}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// [`format_numbers`] rendered with the marks of `locale`: the same number of
/// decimals is chosen (so distinct breaks keep distinct labels), then the
/// labels get the locale's decimal mark and thousands grouping —
/// `[1000, 1500]` reads `"1.000", "1.500"` in de-DE.
pub fn format_numbers_locale(values: &[f64], locale: &crate::locale::Locale) -> Vec<String> {
    format_numbers(values)
        .into_iter()
        .map(|s| locale.localize_number(&s))
        .collect()
}

/// [`format_number`] rendered with the marks of `locale`.
pub fn format_number_locale(v: f64, locale: &crate::locale::Locale) -> String {
    locale.localize_number(&format_number(v))
}

/// [`format_numbers`] with an optional locale (`None` = default marks).
pub(crate) fn format_numbers_opt(
    values: &[f64],
    locale: Option<&crate::locale::Locale>,
) -> Vec<String> {
    match locale {
        Some(l) => format_numbers_locale(values, l),
        None => format_numbers(values),
    }
}

/// [`format_number`] with an optional locale (`None` = default marks).
pub(crate) fn format_number_opt(v: f64, locale: Option<&crate::locale::Locale>) -> String {
    match locale {
        Some(l) => format_number_locale(v, l),
        None => format_number(v),
    }
}

#[cfg(test)]
mod log_breaks_tests {
    use super::*;

    #[test]
    fn wide_range_ticks_whole_decades() {
        // A span of several decades: whole powers of 10 are plenty for 5 ticks.
        assert_eq!(
            log_breaks(1.0, 1000.0, 5, 10.0),
            vec![1.0, 10.0, 100.0, 1000.0]
        );
    }

    #[test]
    fn narrow_range_falls_back_to_sub_decade_steps() {
        // Issue #413: 1..170 spans barely two decades — too few for 5 ticks at
        // whole powers of 10 alone (ggplot2 fills in with 1-2-5-style steps).
        let breaks = log_breaks(1.0, 170.0, 5, 10.0);
        // Every tick must be a "round" number, never a half-decade like 316.23.
        for &b in &breaks {
            let digits = format!("{b}");
            assert!(!digits.contains('.'), "non-round log break: {b}");
        }
        assert!(breaks.contains(&1.0));
        assert!(breaks.iter().any(|&b| b >= 100.0));
    }

    #[test]
    fn base2_ticks_whole_powers_only() {
        assert_eq!(log_breaks(1.0, 8.0, 5, 2.0), vec![1.0, 2.0, 4.0, 8.0]);
    }

    #[test]
    fn degenerate_or_invalid_range_falls_back() {
        assert!(log_breaks(5.0, 5.0, 5, 10.0).is_empty());
        // Non-positive domain can't be logged — fall back to linear extended
        // breaks rather than NaN/garbage.
        assert_eq!(
            log_breaks(-1.0, 10.0, 5, 10.0),
            extended_breaks(-1.0, 10.0, 5)
        );
    }
}

#[cfg(test)]
mod locale_tests {
    use super::*;
    use crate::locale::Locale;

    #[test]
    fn joint_labels_keep_distinctness_with_locale_marks() {
        let de = Locale::de_de();
        assert_eq!(
            format_numbers_locale(&[0.99, 0.992, 0.994], &de),
            ["0,99", "0,992", "0,994"]
        );
        assert_eq!(
            format_numbers_locale(&[0.0, 1000.0, 2000.0, 3000.0], &de),
            ["0", "1.000", "2.000", "3.000"]
        );
        assert_eq!(
            format_numbers_locale(&[-1500.5, 0.0, 1500.5], &de),
            ["-1.500,5", "0", "1.500,5"]
        );
        assert_eq!(
            format_numbers_locale(&[1000.0, 1000.001, 1000.002], &de),
            ["1.000", "1.000,001", "1.000,002"]
        );
        assert_eq!(
            format_numbers_locale(&[0.001, 0.01, 0.1], &Locale::fr_fr()),
            ["0,001", "0,01", "0,1"]
        );
        let us = Locale::en_us();
        assert_eq!(
            format_numbers_locale(&[0.0, 2500.0, 5000.0], &us),
            ["0", "2,500", "5,000"]
        );
        assert_eq!(format_number_locale(1234.5678, &de), "1.234,57");
        // No locale: unchanged default output.
        assert_eq!(format_numbers_opt(&[0.0, 1000.0], None), ["0", "1000"]);
        assert_eq!(format_number_opt(1234.5, None), "1234.5");
    }
}
