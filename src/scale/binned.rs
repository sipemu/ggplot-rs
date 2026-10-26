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
use crate::render::backend::FillPattern;

use super::Scale;

/// `ScaleFillBinned::na_key` (sipemu/ggplot-rs#269): whether the legend's
/// "no data" key is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NaKey {
    /// Today's behaviour (unchanged default): the key appears only when
    /// at least one trained value was `Value::Na`.
    #[default]
    Auto,
    /// Always show the key, with its count (`"no data (0)"` when nothing
    /// is missing) — a plot rendered once per animation frame needs a
    /// legend that doesn't grow or shrink as the missing count changes
    /// from frame to frame.
    Always,
    /// Never show the key, even with `Value::Na` trained — the caller
    /// draws its own "no data" explanation elsewhere.
    Never,
}

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
    /// "no data" key. Default grey50 `(127, 127, 127)`. Overridden by
    /// `na_pattern` when that's set (#206).
    na_color: (u8, u8, u8),
    /// A pattern fill (a thin hatch, by default) for `Value::Na` and the
    /// legend's "no data" key, drawn instead of the flat `na_color` by
    /// `geom_sf`/`geom_polygon`/`geom_rect`/`geom_tile` — a classed
    /// choropleth's no-data cells then read as "we have no reading here"
    /// rather than a greyed-out *value* easily mistaken for one (#206).
    /// `None` (the default): `na_color` draws flat, as before.
    na_pattern: Option<FillPattern>,
    na_label: String,
    /// `na_key` (#269). Default `Auto` — today's `na_count > 0` rule.
    na_key: NaKey,
    /// `count_reserve` (#271) — layout-only digit-width reservation for
    /// the legend's auto-wrap sizing pass; the drawn label is unaffected.
    /// `None` (the default): unchanged, `breaks_for_layout` falls back to
    /// `breaks()`.
    count_reserve: Option<usize>,
    name: String,
    corrected: bool,
}

/// At least two increasing, finite edges: non-finite values dropped and
/// sorted; fewer than two left falls back to `[0.0, 1.0]` (a single
/// degenerate bin) rather than panicking.
///
/// Equal adjacent edges are kept rather than collapsed (sipemu/ggplot-rs#240):
/// a quantile split (or any other caller-supplied break set) can legitimately
/// repeat an edge, e.g. `[35.0, 150.0, 150.0]` for a class whose upper bin
/// holds only the maximum. Deduping such edges away used to merge two
/// classes into one and change the caller's requested class count; keeping
/// them instead produces a *zero-width* bin, which [`bin_index`] and
/// [`ScaleFillBinned::class_label`] give dedicated handling.
fn sorted_edges(breaks: Vec<f64>) -> (Vec<f64>, bool) {
    let mut v: Vec<f64> = breaks.into_iter().filter(|f| f.is_finite()).collect();
    v.sort_by(f64::total_cmp);
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
            na_pattern: None,
            na_label: "no data".to_string(),
            na_key: NaKey::Auto,
            count_reserve: None,
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
            na_pattern: None,
            na_label: "no data".to_string(),
            na_key: NaKey::Auto,
            count_reserve: None,
            name: String::new(),
            corrected,
        }
    }

    /// Colour for `Value::Na` and the legend's "no data" key (ggplot2's
    /// `na.value`). Default grey50 `(127, 127, 127)`. Overridden for
    /// drawing (not for the legend count) by [`na_pattern`](Self::na_pattern)
    /// when that's also set.
    pub fn na_color(mut self, color: (u8, u8, u8)) -> Self {
        self.na_color = color;
        self
    }

    /// A pattern fill for `Value::Na` and the legend's "no data" key,
    /// drawn instead of the flat [`na_color`](Self::na_color) — a classed
    /// choropleth's no-data cells read as "no reading", not a muted
    /// *value* (#206). `FillPattern::stripe(fg, bg)` is a reasonable
    /// default hatch; `bg` is what a pattern-less backend (`plotters`)
    /// falls back to.
    pub fn na_pattern(mut self, pattern: FillPattern) -> Self {
        self.na_pattern = Some(pattern);
        self
    }

    /// Label for the "no data" legend key. Default `"no data"`.
    pub fn na_label(mut self, label: &str) -> Self {
        self.na_label = label.to_string();
        self
    }

    /// Whether the "no data" legend key is shown (#269). Default
    /// [`NaKey::Auto`] (today's behaviour: shown only when at least one
    /// trained value was `Value::Na`). `NaKey::Always` keeps the key
    /// (with its count, down to `"no data (0)"`) even when nothing is
    /// missing — a plot rendered once per animation frame needs a
    /// legend whose key count doesn't change (and so doesn't resize)
    /// just because one particular frame happens to have no missing
    /// values. `NaKey::Never` hides it even with `Value::Na` trained.
    pub fn na_key(mut self, show: NaKey) -> Self {
        self.na_key = show;
        self
    }

    /// Lay the legend out — for auto-wrap sizing only, never for what's
    /// drawn — as if every class's trained count had `max_count`'s digit
    /// width (#271). Binned legend labels carry a count
    /// (`"10–50 (12)"`); across an animation's frames the same scale's
    /// counts change digit width while its breaks/colours stay fixed,
    /// which can change the legend's measured pitch enough to flip an
    /// auto-wrapped (horizontal) legend's row count and move the panel.
    /// `max_count` should be the largest count any frame can produce
    /// (e.g. the full dataset's count per class). Default `None`: off,
    /// byte-identical to before this existed.
    pub fn count_reserve(mut self, max_count: usize) -> Self {
        self.count_reserve = Some(max_count);
        self
    }

    /// Pad `label`'s trailing `" (count)"` so its digit run is at least
    /// `min_digits` wide (zero-padded), for [`breaks_for_layout`]'s
    /// `count_reserve` sizing pass — never for what's drawn. A label that
    /// doesn't end in `(<digits>)` (shouldn't happen for this scale's own
    /// breaks, but keeps this generic) is returned unchanged.
    ///
    /// [`breaks_for_layout`]: Scale::breaks_for_layout
    fn pad_trailing_count(label: &str, min_digits: usize) -> String {
        if label.ends_with(')') {
            if let Some(open) = label.rfind('(') {
                let inner = &label[open + 1..label.len() - 1];
                if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit()) {
                    let digits = min_digits.max(inner.len());
                    return format!("{}({inner:0>digits$})", &label[..open]);
                }
            }
        }
        label.to_string()
    }

    /// The bin index for `v` (clamped into range first).
    ///
    /// An ordinary bin `i` is half-open, `[breaks[i], breaks[i+1])`; the
    /// last bin is closed at its upper edge too, so the maximum
    /// trained/data value lands in it rather than falling just outside
    /// every bin. A **zero-width** bin (`breaks[i] == breaks[i+1]`, e.g.
    /// from a quantile split whose top class holds only the maximum,
    /// sipemu/ggplot-rs#240) is closed at *both* ends instead, so it
    /// captures values equal to its single shared edge rather than losing
    /// them to whichever neighbour the half-open rule would otherwise hand
    /// them to. Bins are scanned low to high and the first match wins, so
    /// when several consecutive bins share the same point the lowest-index
    /// one claims it (ggplot2's `cut(..., right = TRUE)` convention, with
    /// the first bin additionally closed on the left so the global minimum
    /// is never dropped).
    fn bin_index(&self, v: f64) -> usize {
        let n = self.colors.len();
        let v = v.clamp(self.breaks[0], self.breaks[n]);
        for i in 0..n {
            let (lo, hi) = (self.breaks[i], self.breaks[i + 1]);
            let hit = if i == n - 1 {
                v >= lo && v <= hi // last bin: closed at both ends
            } else if lo == hi {
                v == lo // zero-width bin: closed at both ends
            } else {
                v >= lo && v < hi // ordinary bin: half-open
            };
            if hit {
                return i;
            }
        }
        n - 1
    }

    /// `"{lo}–{hi} ({count})"` for bin `i`, or just `"{v} ({count})"` when
    /// `i` is a zero-width bin (`breaks[i] == breaks[i + 1]`, #240) — a
    /// range like "150–150" would misstate a bin that in fact holds only
    /// the single value 150.
    fn class_label(&self, i: usize) -> String {
        let (lo, hi) = (self.breaks[i], self.breaks[i + 1]);
        if lo == hi {
            format!("{} ({})", format_value(&Value::Float(lo)), self.counts[i])
        } else {
            format!(
                "{}–{} ({})",
                format_value(&Value::Float(lo)),
                format_value(&Value::Float(hi)),
                self.counts[i]
            )
        }
    }

    /// `"{na_label} ({na_count})"`.
    fn na_label_with_count(&self) -> String {
        format!("{} ({})", self.na_label, self.na_count)
    }

    /// Whether `breaks()` includes the "no data" key, per `na_key` (#269).
    fn na_key_shown(&self) -> bool {
        match self.na_key {
            NaKey::Auto => self.na_count > 0,
            NaKey::Always => true,
            NaKey::Never => false,
        }
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
        if self.na_key_shown() {
            out.push((1.0, self.na_label_with_count()));
        }
        out
    }

    /// `count_reserve` (#271): pad every key's trailing `(count)` to at
    /// least `count_reserve`'s digit width before measuring — the drawn
    /// labels from [`breaks`](Scale::breaks) are untouched. `None` (the
    /// default): identical to `breaks()`.
    fn breaks_for_layout(&self) -> Vec<(f64, String)> {
        match self.count_reserve {
            None => self.breaks(),
            Some(reserve) => {
                let min_digits = reserve.to_string().len();
                self.breaks()
                    .into_iter()
                    .map(|(pos, label)| (pos, Self::pad_trailing_count(&label, min_digits)))
                    .collect()
            }
        }
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

    /// This scale's configured `na_pattern`, if any — the caller (a
    /// geom's fill resolution, or the legend's "no data" key) only
    /// consults this for a row/key that's already otherwise headed for
    /// the flat `na_color` (`Value::Na`, or the "no data" legend-key
    /// label itself); every ordinary bin colour is unaffected (#206).
    fn na_fill_pattern(&self) -> Option<FillPattern> {
        self.na_pattern
    }

    fn is_na_break(&self, label: &str) -> bool {
        self.na_key_shown() && label == self.na_label_with_count()
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
    fn na_pattern_is_none_until_set() {
        let s = scale();
        assert_eq!(s.na_fill_pattern(), None);
    }

    #[test]
    fn na_pattern_is_reported_only_for_the_no_data_break_label() {
        let pattern = FillPattern::stripe((10, 20, 30), (200, 210, 220));
        let mut s = scale().na_pattern(pattern);
        s.train(&[Value::Na, Value::Float(5.0)]);
        assert_eq!(s.na_fill_pattern(), Some(pattern));
        assert!(s.is_na_break(&s.na_label_with_count()));
        // An ordinary bin's own label is never mistaken for the na key.
        let ordinary = s.class_label(0);
        assert!(!s.is_na_break(&ordinary));
    }

    #[test]
    fn no_na_break_when_nothing_trained_as_na() {
        // `na_count == 0`: `is_na_break` is false even for the label text
        // that *would* be the na key, since the legend never shows one.
        let s = scale().na_pattern(FillPattern::stripe((1, 2, 3), (4, 5, 6)));
        assert!(!s.is_na_break(&s.na_label_with_count()));
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

    // --- #240: duplicate (zero-width) class edges ----------------------

    /// A duplicate edge at the *maximum* — the motivating case from #240
    /// (a quantile split whose top class holds only the maximum, e.g.
    /// `[35, 150, 150]` over two cells 35 and 150).
    #[test]
    fn duplicate_edge_at_the_max_keeps_both_classes_and_colours() {
        let mut s = ScaleFillBinned::new(
            Aesthetic::Fill,
            vec![35.0, 150.0, 150.0],
            vec![(0, 0, 255), (255, 0, 0)],
        );
        assert!(
            !s.corrected,
            "a legitimate zero-width bin is not a fallback"
        );
        assert_eq!(s.colors.len(), 2, "caller's requested class count is kept");
        assert_eq!(
            s.colors,
            vec![(0, 0, 255), (255, 0, 0)],
            "distinct colours per class"
        );

        s.train(&[Value::Float(35.0), Value::Float(150.0)]);
        assert_eq!(s.counts, vec![1, 1], "each cell counted into its own class");
        assert_eq!(s.bin_index(35.0), 0);
        assert_eq!(
            s.bin_index(150.0),
            1,
            "the zero-width bin claims its own edge value"
        );
        assert_eq!(s.map_to_color(&Value::Float(150.0)), Some((255, 0, 0)));

        let breaks = s.breaks();
        let labels: Vec<&str> = breaks.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(
            labels,
            vec!["35–150 (1)", "150 (1)"],
            "the zero-width class's label/count is a single value, not a 150–150 range"
        );
    }

    /// A duplicate edge at the *minimum* — the lowest bin is zero-width and
    /// must still claim the global minimum (left-closed first bin).
    #[test]
    fn duplicate_edge_at_the_min_keeps_both_classes_and_colours() {
        let mut s = ScaleFillBinned::new(
            Aesthetic::Fill,
            vec![0.0, 0.0, 10.0],
            vec![(0, 0, 255), (255, 0, 0)],
        );
        assert!(!s.corrected);
        assert_eq!(s.colors.len(), 2);

        s.train(&[Value::Float(0.0), Value::Float(0.0), Value::Float(5.0)]);
        assert_eq!(s.counts, vec![2, 1]);
        assert_eq!(s.bin_index(0.0), 0, "the zero-width bin claims the minimum");
        assert_eq!(s.bin_index(5.0), 1);
        assert_eq!(s.map_to_color(&Value::Float(0.0)), Some((0, 0, 255)));

        let breaks = s.breaks();
        let labels: Vec<&str> = breaks.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(labels, vec!["0 (2)", "0–10 (1)"]);
    }

    /// A duplicate edge in the *middle* — between two otherwise ordinary
    /// classes.
    #[test]
    fn duplicate_edge_in_the_middle_keeps_all_classes_and_colours() {
        let mut s = ScaleFillBinned::new(
            Aesthetic::Fill,
            vec![0.0, 50.0, 50.0, 100.0],
            vec![(0, 0, 255), (0, 255, 0), (255, 0, 0)],
        );
        assert!(!s.corrected);
        assert_eq!(s.colors.len(), 3, "all three requested classes survive");
        assert_eq!(s.colors, vec![(0, 0, 255), (0, 255, 0), (255, 0, 0)]);

        s.train(&[Value::Float(25.0), Value::Float(50.0), Value::Float(75.0)]);
        assert_eq!(
            s.counts,
            vec![1, 1, 1],
            "the shared edge value goes to the middle (zero-width) bin"
        );
        assert_eq!(s.bin_index(25.0), 0);
        assert_eq!(
            s.bin_index(50.0),
            1,
            "the zero-width middle bin wins the tie over its neighbours"
        );
        assert_eq!(s.bin_index(75.0), 2);
        assert_eq!(s.map_to_color(&Value::Float(50.0)), Some((0, 255, 0)));

        let breaks = s.breaks();
        let labels: Vec<&str> = breaks.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(labels, vec!["0–50 (1)", "50 (1)", "50–100 (1)"]);
    }

    /// The fix must not perturb the ordinary (non-degenerate) case this
    /// crate already shipped: same inputs, same bins/colours/labels as
    /// before #240.
    #[test]
    fn non_degenerate_edges_are_unaffected() {
        let s = scale();
        assert_eq!(s.breaks, vec![0.0, 10.0, 50.0, 100.0]);
        assert_eq!(s.colors, vec![(0, 0, 255), (0, 255, 0), (255, 0, 0)]);
        assert_eq!(s.bin_index(0.0), 0);
        assert_eq!(s.bin_index(9.999), 0);
        assert_eq!(s.bin_index(10.0), 1);
        assert_eq!(s.bin_index(49.999), 1);
        assert_eq!(s.bin_index(50.0), 2);
        assert_eq!(s.bin_index(100.0), 2);
    }

    // ─── #269: `na_key` ──────────────────────────────────────────────

    #[test]
    fn na_key_always_lists_the_key_with_zero_count_when_nothing_is_missing() {
        let mut s = scale().na_key(NaKey::Always);
        s.train(&[Value::Float(1.0)]);
        let breaks = s.breaks();
        assert_eq!(breaks.len(), 4, "the no-data key is kept even at 0");
        let (_, label) = breaks.last().unwrap();
        assert_eq!(label, "no data (0)");
        // And it still round-trips to the na colour via `map_to_color`.
        assert_eq!(
            s.map_to_color(&Value::Str(label.clone())),
            Some((127, 127, 127))
        );
        assert!(s.is_na_break(label));
    }

    #[test]
    fn na_key_never_hides_it_even_with_na_present() {
        let mut s = scale().na_key(NaKey::Never);
        s.train(&[Value::Float(1.0), Value::Na, Value::Na]);
        let breaks = s.breaks();
        assert_eq!(breaks.len(), 3, "no trailing no-data entry despite 2 NAs");
        assert!(!s.is_na_break(&s.na_label_with_count()));
    }

    #[test]
    fn na_key_auto_is_unchanged() {
        // Default (no `.na_key(..)` call): identical to before #269.
        let mut without_na = scale();
        without_na.train(&[Value::Float(1.0)]);
        assert_eq!(without_na.breaks().len(), 3);

        let mut with_na = scale();
        with_na.train(&[Value::Float(1.0), Value::Na]);
        assert_eq!(with_na.breaks().len(), 4);
    }

    // ─── #271: `count_reserve` ───────────────────────────────────────

    #[test]
    fn count_reserve_pads_measured_labels_but_not_drawn_ones() {
        let mut few = scale().count_reserve(9999);
        few.train(&[Value::Float(1.0), Value::Float(20.0), Value::Float(75.0)]);
        let drawn = few.breaks();
        let labels: Vec<&str> = drawn.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(
            labels,
            vec!["0–10 (1)", "10–50 (1)", "50–100 (1)"],
            "drawn labels unaffected"
        );

        let layout = few.breaks_for_layout();
        let layout_labels: Vec<&str> = layout.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(
            layout_labels,
            vec!["0–10 (0001)", "10–50 (0001)", "50–100 (0001)"],
            "layout labels reserve the 4-digit width"
        );
    }

    /// The motivating case (#271): two scales sharing breaks/colours, one
    /// trained with single-digit counts and the other with four-digit
    /// counts, give *equal-length* `breaks_for_layout()` labels — the
    /// same character count `legend_pitch`'s `text_width` measures —
    /// once both reserve the wider digit width, even though the actual
    /// digits (and so the drawn labels) differ.
    #[test]
    fn count_reserve_gives_equal_length_layout_labels_across_differing_counts() {
        let mut few = scale().count_reserve(9999);
        few.train(&[Value::Float(1.0), Value::Float(20.0), Value::Float(75.0)]);

        let mut many = scale().count_reserve(9999);
        let batch: Vec<Value> = (0..1234)
            .flat_map(|_| [Value::Float(1.0), Value::Float(20.0), Value::Float(75.0)])
            .collect();
        many.train(&batch);

        let few_layout = few.breaks_for_layout();
        let many_layout = many.breaks_for_layout();
        for (f, m) in few_layout.iter().zip(many_layout.iter()) {
            assert_eq!(
                f.1.len(),
                m.1.len(),
                "layout labels must measure the same width: {f:?} vs {m:?}"
            );
        }
        // But the actually-drawn labels still differ, as they must.
        assert_ne!(few.breaks(), many.breaks());
    }

    #[test]
    fn count_reserve_is_none_by_default_and_layout_matches_breaks() {
        let mut s = scale();
        s.train(&[Value::Float(1.0), Value::Na]);
        assert_eq!(s.breaks_for_layout(), s.breaks());
    }

    #[test]
    fn count_reserve_never_shrinks_below_the_actual_count() {
        // A reservation smaller than the count actually trained is a
        // floor, not a cap — the real count's digit width still wins.
        let mut s = scale().count_reserve(1);
        let batch: Vec<Value> = (0..1234).map(|_| Value::Float(1.0)).collect();
        s.train(&batch);
        let layout = s.breaks_for_layout();
        assert!(layout[0].1.contains("(1234)"), "{layout:?}");
    }
}
