# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased] — 0.16.0

### Breaking changes

- `PositionJitter` gained a `seed: u64` field, and `width`/`height` are now
  `Option<f64>` (`None` = 0.4 × the data's resolution, as in ggplot2). Build it
  with `PositionJitter::new(w, h)`, `PositionJitter::default()`, `.with_width()`,
  `.with_height()` and `.with_seed()` instead of a struct literal.
  `GeomJitter::{width, height}` are likewise `Option<f64>` (default `None`).
- Jitter and `mean_cl_boot` now draw from an internal SplitMix64 stream instead
  of `rand::StdRng`: output is still deterministic, but the exact jittered
  positions / bootstrap intervals differ from 0.15.

### Changed

- Removed the `rand` dependency (and the `wasm32`-only `getrandom` dependency)
  from the library; randomness comes from a small internal seeded PRNG.
- Default jitter amount is resolution-based (ggplot2's `0.4 * resolution(x)`):
  integer-spaced data keeps ±0.4, finely spaced continuous data is no longer
  smeared by a fixed ±0.4 data units.
