//! A tiny, dependency-free, seeded pseudo-random number generator.
//!
//! ggplot-rs only needs randomness for *reproducible* visual noise (jitter) and
//! resampling (bootstrap confidence intervals) — never for anything
//! security-relevant. A SplitMix64 stream is fast, has good statistical quality
//! for this purpose, and keeps the crate free of `rand`/`getrandom` (which would
//! otherwise need the browser crypto API on `wasm32`).

/// SplitMix64 generator (Steele, Lea & Flood 2014).
#[derive(Clone, Debug)]
pub(crate) struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Create a generator from a seed. Equal seeds yield equal streams.
    pub(crate) fn new(seed: u64) -> Self {
        SplitMix64 { state: seed }
    }

    /// Next raw 64-bit output.
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform `f64` in `[0, 1)` (53 random mantissa bits).
    pub(crate) fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform `f64` in `[lo, hi)`. Returns `lo` for an empty/degenerate range.
    pub(crate) fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        if hi.partial_cmp(&lo) != Some(std::cmp::Ordering::Greater) {
            return lo;
        }
        lo + (hi - lo) * self.next_f64()
    }

    /// Uniform integer in `[0, n)` (Lemire's multiply-shift; `n > 0`).
    pub(crate) fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        ((self.next_u64() as u128 * n as u128) >> 64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_equal_seeds() {
        let mut a = SplitMix64::new(7);
        let mut b = SplitMix64::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        let mut c = SplitMix64::new(8);
        assert_ne!(SplitMix64::new(7).next_u64(), c.next_u64());
    }

    #[test]
    fn known_reference_values() {
        // Reference outputs of SplitMix64 seeded with 1234567.
        let mut r = SplitMix64::new(1_234_567);
        assert_eq!(r.next_u64(), 6_457_827_717_110_365_317);
        assert_eq!(r.next_u64(), 3_203_168_211_198_807_973);
    }

    #[test]
    fn ranges_are_respected_and_roughly_uniform() {
        let mut r = SplitMix64::new(42);
        let n = 20_000;
        let mut sum = 0.0;
        let mut hist = [0usize; 10];
        for _ in 0..n {
            let x = r.range_f64(-2.0, 3.0);
            assert!((-2.0..3.0).contains(&x));
            sum += x;
            let k = r.below(10);
            assert!(k < 10);
            hist[k] += 1;
        }
        let mean = sum / n as f64;
        assert!((mean - 0.5).abs() < 0.05, "mean {mean}");
        for h in hist {
            assert!((1_700..2_300).contains(&h), "bucket {h}");
        }
        assert_eq!(r.range_f64(1.0, 1.0), 1.0);
        assert_eq!(r.range_f64(1.0, f64::NAN), 1.0);
    }
}
