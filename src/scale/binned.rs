//! Binned fill/colour scale with *explicit* class breaks (R's
//! `scale_fill_steps`/`scale_fill_fermenter` generalised to arbitrary,
//! caller-supplied edges — ggplot2's `scale_fill_binned` with `breaks` set
//! "manually"), backing a `::CLASSES` choropleth (sipemu/ggplot-rs#180).
//!
//! The break *maths* — quantile/Fisher-Jenks/equal-width/custom edges —
//! stays the caller's job (anofox-visualization's `map/classes.rs`); this
//! scale only bins against the edges it's given, tallies how many trained
//! values fall in each bin, and draws a stepped legend with those counts
//! plus a "no data" key for `Value::Na` (ggplot2's `na.value`).

use std::collections::HashSet;

use crate::aes::Aesthetic;
use crate::data::Value;
use crate::format::format_value;

use super::Scale;

/// A coarse, order-sensitive fingerprint of a `train()` batch (FNV-1a over
/// each value's length/discriminant/bits). See [`ScaleFillBinned::seen`]'s
/// doc for why this exists.
fn signature(values: &[Value]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |x: u64| {
        h ^= x;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    };
    mix(values.len() as u64);
    for v in values {
        match v {
            Value::Na => mix(1),
            _ => match v.as_f64() {
                Some(f) => {
                    mix(2);
                    mix(f.to_bits());
                }
                None => mix(3),
            },
        }
    }
    h
}

/// Binned fill/colour scale: `n` classes from `n + 1` increasing edges,
/// each with its own colour. See the [module docs](self) for how it
/// relates to `scale_fill_steps`/`::CLASSES`.
#[derive(Clone, Debug)]
pub struct ScaleFillBinned {
    aesthetic: Aesthetic,
    /// `n + 1` increasing edges (`colors.len() == n`).
    breaks: Vec<f64>,
    colors: Vec<(u8, u8, u8)>,
    /// Trained-value count per bin, same indices as `colors`.
    counts: Vec<usize>,
    /// Trained `Value::Na` count.
    na_count: usize,
    /// Fingerprints of every batch already folded into `counts`/`na_count`
    /// (see [`signature`]). `Scale::train` can run more than once over the
    /// *same* layer data — `build.rs`'s own "final scale training pass"
    /// re-trains every layer's already-built data on top of the per-layer
    /// pass `build_layer` just did, and faceting clones a scale set and
    /// retrains it again per panel — which every other scale in this crate
    /// tolerates for free because re-expanding a min/max domain with data
    /// it has already seen is a no-op. A running *count* isn't naturally
    /// idempotent that way, so this scale has to recognise "I've already
    /// counted this exact batch" itself and skip it, rather than double
    /// (or triple) class counts and the no-data count.
    seen: HashSet<u64>,
    /// ggplot2's `na.value` — colour for `Value::Na` and for the legend's
    /// "no data" key. Default grey50 `(127, 127, 127)`.
    na_color: (u8, u8, u8),
    na_label: String,
    name: String,
    corrected: bool,
}

/// At least two increasing, finite edges: non-finite values dropped,
/// sorted, exact duplicates collapsed; fewer than two left falls back to
/// `[0.0, 1.0]` (a single degenerate bin) rather than panicking.
fn sorted_edges(breaks: Vec<f64>) -> (Vec<f64>, bool) {
    let mut v: Vec<f64> = breaks.into_iter().filter(|f| f.is_finite()).collect();
    v.sort_by(f64::total_cmp);
    v.dedup();
    if v.len() >= 2 {
        (v, false)
    } else {
        (vec![0.0, 1.0], true)
    }
}

/// Linear RGB interpolation at `t` in `[0, 1]`.
fn lerp_color(low: (u8, u8, u8), high: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let lerp = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
    (
        lerp(low.0, high.0),
        lerp(low.1, high.1),
        lerp(low.2, high.2),
    )
}

/// `n` colours evenly interpolated from `low` to `high` (`n == 1` picks the
/// midpoint).
fn default_colors(n: usize, low: (u8, u8, u8), high: (u8, u8, u8)) -> Vec<(u8, u8, u8)> {
    (0..n)
        .map(|i| {
            let t = if n <= 1 {
                0.5
            } else {
                i as f64 / (n - 1) as f64
            };
            lerp_color(low, high, t)
        })
        .collect()
}

/// This crate's default two-colour ramp (matches [`super::steps::ScaleColorSteps`]'s).
const DEFAULT_LOW: (u8, u8, u8) = (50, 50, 200);
const DEFAULT_HIGH: (u8, u8, u8) = (200, 50, 50);

impl ScaleFillBinned {
    /// `n + 1` increasing `breaks` with an explicit colour per bin
    /// (`colors.len()` should be `breaks.len() - 1`; a mismatch falls back
    /// to the default two-colour ramp and is reported by
    /// [`validate`](Scale::validate)).
    pub fn new(aesthetic: Aesthetic, breaks: Vec<f64>, colors: Vec<(u8, u8, u8)>) -> Self {
        let (breaks, edges_corrected) = sorted_edges(breaks);
        let n = breaks.len() - 1;
        let (colors, colors_corrected) = if colors.len() == n {
            (colors, false)
        } else {
            (default_colors(n, DEFAULT_LOW, DEFAULT_HIGH), true)
        };
        ScaleFillBinned {
            aesthetic,
            breaks,
            colors,
            counts: vec![0; n],
            na_count: 0,
            seen: HashSet::new(),
            na_color: (127, 127, 127),
            na_label: "no data".to_string(),
            name: String::new(),
            corrected: edges_corrected || colors_corrected,
        }
    }

    /// `n + 1` increasing `breaks`, coloured by interpolating `low`→`high`
    /// across the `n` bins (R's `scale_fill_steps(breaks = ...)`).
    pub fn two(
        aesthetic: Aesthetic,
        breaks: Vec<f64>,
        low: (u8, u8, u8),
        high: (u8, u8, u8),
    ) -> Self {
        let (breaks, corrected) = sorted_edges(breaks);
        let n = breaks.len() - 1;
        ScaleFillBinned {
            aesthetic,
            colors: default_colors(n, low, high),
            counts: vec![0; n],
            breaks,
            na_count: 0,
            seen: HashSet::new(),
            na_color: (127, 127, 127),
            na_label: "no data".to_string(),
            name: String::new(),
            corrected,
        }
    }

    /// Colour for `Value::Na` and the legend's "no data" key (ggplot2's
    /// `na.value`). Default grey50 `(127, 127, 127)`.
    pub fn na_color(mut self, color: (u8, u8, u8)) -> Self {
        self.na_color = color;
        self
    }

    /// Label for the "no data" legend key. Default `"no data"`.
    pub fn na_label(mut self, label: &str) -> Self {
        self.na_label = label.to_string();
        self
    }

    /// The half-open `[breaks[i], breaks[i+1])` bin index for `v`
    /// (clamped into range first; the last bin is closed at its upper
    /// edge, so the maximum trained/data value lands in it rather than
    /// falling just outside every bin).
    fn bin_index(&self, v: f64) -> usize {
        let n = self.colors.len();
        let v = v.clamp(self.breaks[0], self.breaks[n]);
        for (i, w) in self.breaks[1..n].iter().enumerate() {
            if v < *w {
                return i;
            }
        }
        n - 1
    }

    /// `"{lo}–{hi} ({count})"` for bin `i`.
    fn class_label(&self, i: usize) -> String {
        format!(
            "{}–{} ({})",
            format_value(&Value::Float(self.breaks[i])),
            format_value(&Value::Float(self.breaks[i + 1])),
            self.counts[i]
        )
    }

    /// `"{na_label} ({na_count})"`.
    fn na_label_with_count(&self) -> String {
        format!("{} ({})", self.na_label, self.na_count)
    }
}

impl Scale for ScaleFillBinned {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    fn train(&mut self, values: &[Value]) {
        // See `seen`'s doc: the pipeline can submit the exact same layer's
        // data to `train` more than once, which every other scale's
        // min/max expansion tolerates for free but a running count can't.
        if !self.seen.insert(signature(values)) {
            return;
        }
        for v in values {
            match v {
                Value::Na => self.na_count += 1,
                _ => {
                    if let Some(f) = v.as_f64() {
                        if f.is_finite() {
                            let i = self.bin_index(f);
                            self.counts[i] += 1;
                        }
                    }
                }
            }
        }
    }

    /// Not used for position — like [`super::steps::ScaleColorSteps`], the
    /// legend looks a value's colour up by its break label instead.
    fn map(&self, _value: &Value) -> f64 {
        0.0
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        let n = self.colors.len();
        let (lo0, hi_n) = (self.breaks[0], self.breaks[n]);
        let span = (hi_n - lo0).max(f64::EPSILON);
        let mut out: Vec<(f64, String)> = (0..n)
            .map(|i| {
                let center = (self.breaks[i] + self.breaks[i + 1]) / 2.0;
                ((center - lo0) / span, self.class_label(i))
            })
            .collect();
        if self.na_count > 0 {
            out.push((1.0, self.na_label_with_count()));
        }
        out
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn is_discrete(&self) -> bool {
        true
    }

    fn map_to_color(&self, value: &Value) -> Option<(u8, u8, u8)> {
        if matches!(value, Value::Na) {
            return Some(self.na_color);
        }
        if let Some(f) = value.as_f64() {
            if f.is_finite() {
                return Some(self.colors[self.bin_index(f)]);
            }
        }
        if let Value::Str(s) = value {
            if s == &self.na_label_with_count() {
                return Some(self.na_color);
            }
            for i in 0..self.colors.len() {
                if self.class_label(i) == *s {
                    return Some(self.colors[i]);
                }
            }
        }
        None
    }

    fn domain(&self) -> Option<(f64, f64)> {
        let n = self.colors.len();
        Some((self.breaks[0], self.breaks[n]))
    }

    fn validate(&self, warnings: &mut Vec<String>) {
        if self.corrected {
            warnings.push(
                "scale_fill_binned: breaks need >= 2 increasing finite edges, with one colour \
                 per bin — falling back to a default range/palette"
                    .to_string(),
            );
        }
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        self.counts.iter_mut().for_each(|c| *c = 0);
        self.na_count = 0;
        self.seen.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scale() -> ScaleFillBinned {
        ScaleFillBinned::new(
            Aesthetic::Fill,
            vec![0.0, 10.0, 50.0, 100.0],
            vec![(0, 0, 255), (0, 255, 0), (255, 0, 0)],
        )
    }

    #[test]
    fn bins_are_half_open_except_the_last() {
        let s = scale();
        assert_eq!(s.bin_index(0.0), 0);
        assert_eq!(s.bin_index(9.999), 0);
        assert_eq!(s.bin_index(10.0), 1); // right edge moves to the next bin...
        assert_eq!(s.bin_index(49.999), 1);
        assert_eq!(s.bin_index(50.0), 2);
        assert_eq!(s.bin_index(100.0), 2); // ...except the very last, which is closed
        assert_eq!(
            s.bin_index(1000.0),
            2,
            "out-of-range clamps to the last bin"
        );
        assert_eq!(
            s.bin_index(-50.0),
            0,
            "out-of-range clamps to the first bin"
        );
    }

    #[test]
    fn train_tallies_counts_and_na_separately() {
        let mut s = scale();
        s.train(&[
            Value::Float(1.0),
            Value::Float(5.0),
            Value::Float(20.0),
            Value::Float(99.0),
            Value::Float(100.0),
            Value::Na,
            Value::Na,
        ]);
        assert_eq!(s.counts, vec![2, 1, 2]);
        assert_eq!(s.na_count, 2);
    }

    /// `build.rs`'s own "final scale training pass" (and, for a faceted
    /// plot, `compute_panel_scales`) re-submits a layer's already-trained
    /// data to `Scale::train` a second time — every other scale tolerates
    /// that for free (re-expanding a min/max domain with data it's already
    /// seen is a no-op); a running count must recognise the repeat itself.
    #[test]
    fn training_the_same_batch_twice_does_not_double_count() {
        let batch = [Value::Float(1.0), Value::Float(20.0), Value::Na];
        let mut s = scale();
        s.train(&batch);
        s.train(&batch); // the pipeline's redundant re-training pass
        assert_eq!(s.counts, vec![1, 1, 0]);
        assert_eq!(s.na_count, 1);
        // A genuinely different batch afterward still counts normally.
        s.train(&[Value::Float(60.0)]);
        assert_eq!(s.counts, vec![1, 1, 1]);
    }

    #[test]
    fn breaks_label_each_class_with_its_count_and_na_with_its_own() {
        let mut s = scale();
        s.train(&[Value::Float(1.0), Value::Float(20.0), Value::Na]);
        let breaks = s.breaks();
        let labels: Vec<&str> = breaks.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(
            labels,
            vec!["0–10 (1)", "10–50 (1)", "50–100 (0)", "no data (1)"]
        );
    }

    #[test]
    fn no_na_key_when_nothing_is_missing() {
        let mut s = scale();
        s.train(&[Value::Float(1.0)]);
        assert_eq!(s.breaks().len(), 3, "no trailing no-data entry");
    }

    #[test]
    fn map_to_color_matches_the_bin_and_roundtrips_legend_labels() {
        let mut s = scale();
        s.train(&[Value::Float(1.0), Value::Float(20.0), Value::Na]);
        assert_eq!(s.map_to_color(&Value::Float(5.0)), Some((0, 0, 255)));
        assert_eq!(s.map_to_color(&Value::Float(20.0)), Some((0, 255, 0)));
        assert_eq!(s.map_to_color(&Value::Float(75.0)), Some((255, 0, 0)));
        assert_eq!(s.map_to_color(&Value::Na), Some((127, 127, 127)));
        for (_, label) in s.breaks() {
            let via_label = s.map_to_color(&Value::Str(label.clone()));
            assert!(via_label.is_some(), "{label} should round-trip to a colour");
        }
    }

    #[test]
    fn na_color_and_label_are_configurable() {
        let mut s = scale().na_color((10, 20, 30)).na_label("missing");
        s.train(&[Value::Na]);
        assert_eq!(s.map_to_color(&Value::Na), Some((10, 20, 30)));
        assert!(s.breaks().iter().any(|(_, l)| l == "missing (1)"));
        assert_eq!(
            s.map_to_color(&Value::Str("missing (1)".to_string())),
            Some((10, 20, 30))
        );
    }

    #[test]
    fn two_interpolates_colors_across_bins() {
        let s = ScaleFillBinned::two(
            Aesthetic::Fill,
            vec![0.0, 25.0, 50.0, 75.0, 100.0],
            (0, 0, 0),
            (255, 255, 255),
        );
        assert_eq!(s.colors[0], (0, 0, 0));
        assert_eq!(s.colors[3], (255, 255, 255));
        assert!(s.colors[1].0 < s.colors[2].0);
    }

    #[test]
    fn mismatched_colors_fall_back_and_warn() {
        let s = ScaleFillBinned::new(
            Aesthetic::Fill,
            vec![0.0, 10.0, 20.0],
            vec![(0, 0, 0)], // 2 bins need 2 colours, only 1 given
        );
        assert_eq!(s.colors.len(), 2);
        let mut warnings = Vec::new();
        s.validate(&mut warnings);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn too_few_breaks_falls_back_to_a_single_bin_rather_than_panicking() {
        let s = ScaleFillBinned::new(Aesthetic::Fill, vec![5.0], vec![]);
        assert_eq!(s.breaks.len(), 2);
        assert_eq!(s.colors.len(), 1);
        let mut warnings = Vec::new();
        s.validate(&mut warnings);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn domain_spans_the_outer_edges() {
        let s = scale();
        assert_eq!(s.domain(), Some((0.0, 100.0)));
    }

    #[test]
    fn reset_training_clears_counts_but_keeps_breaks_and_colors() {
        let mut s = scale();
        s.train(&[Value::Float(1.0), Value::Na]);
        s.reset_training();
        assert_eq!(s.counts, vec![0, 0, 0]);
        assert_eq!(s.na_count, 0);
        assert_eq!(s.breaks, vec![0.0, 10.0, 50.0, 100.0]);
    }
}
