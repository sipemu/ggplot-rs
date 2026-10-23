# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- `GeomSmooth::gam()` (`method = "gam"`) now works without the `regression`
  feature, including for `wasm32-unknown-unknown` (#174). Previously the GAM
  smoother only existed behind `regression`, which pulls anofox-regression,
  faer, nalgebra, statrs and rand (~100 crates) and does not build for wasm.
  `StatSmooth` now carries two GAM backends: `compute_gam_regression`
  (unchanged, anofox-regression's `PSplineRegressor`, `regression` feature)
  and a new dependency-free `compute_gam_builtin` — a cubic B-spline basis,
  a second-order difference penalty (Eilers & Marx), λ chosen by
  generalised cross-validation, and a handful of dense k×k solves by
  Gaussian elimination, plus a t-based confidence ribbon built from the
  same linear algebra (`src/stat/gam.rs`). A small dispatcher
  (`StatSmooth::compute_gam`) picks the regression-backed path when the
  feature is enabled and the builtin path otherwise; no other API changes.
  This is not a breaking change — `gam()` simply works in more
  configurations than before. The builtin fit is checked against the
  regression-backed one on the same data (`src/stat/smooth.rs`'s
  `gam_backend_parity_tests`, `regression`-gated) and `tests/gam_smooth.rs`
  now runs against whichever backend is active. Ported from
  anofox-visualization's own `src/smooth.rs` (DataZooDE/anofox-visualization#318).

### Spatial

- `SfProjection` gains `EqualEarth { lon0 }` (Šavrič et al. 2018),
  `Albers { lat1, lat2, lat0, lon0 }` (spherical equal-area conic, with
  `SfProjection::albers_usa()`/`albers_europe()` presets) and
  `Utm { zone, south }` (ellipsoidal, Krüger series), alongside the existing
  `PlateCarree`/`Mercator`. `SfProjection::utm_for(lon, lat)` picks the zone
  and hemisphere for a point; `SfProjection::utm_zone(lon)` the zone alone.
  All three are tested against PROJ (`ST_Transform` in DuckDB spatial) to
  0.01 m absolute (≤1e-6 relative) over their usual range. Ported from
  anofox-visualization's `map/project.rs` (DataZooDE/anofox-visualization#318,
  #177). The `ggplot` CLI's `--projection` and the wasm `render_geo` JSON
  spec's `projection` field accept the new names (`equal-earth`,
  `albers-usa`, `albers-europe`, `utm:<zone>`/`utm:<zone>s`).
- **Breaking**: `SfProjection` no longer derives `Eq` (the new variants carry
  `f64` fields); `PartialEq`/`Debug`/`Clone`/`Copy` are unaffected, so this
  only matters to code that put `SfProjection` in an `Eq`-bound generic
  (none in this crate).
- `geom_sf_text()`/`geom_sf_label()` (+ `_with`): labels placed inside
  `geom_sf` polygons, ggplot2's `geom_sf_text`/`geom_sf_label`. An
  anchor is either the ring's centroid or its pole of inaccessibility
  (`SfLabelAnchor::Polylabel`, Mapbox's algorithm — the inside point
  farthest from the outline, so a C-shaped region's label stays in the
  spine rather than the centroid, which can fall outside; default).
  `.check_overlap(true)` drops a lower-priority label (the mapped `size`
  aesthetic if present, else the region's screen area, larger first)
  that would collide with one already placed or fall outside the panel —
  `geom_sf_text`'s own `StatSf` trains the X/Y scales on the labelled
  geometry's extent even with no `geom_sf` layer sharing the plot, like
  ggplot2's. `.halo(color, r)` draws a shadowtext-style halo (`r` px out
  in 8 directions, in `color`, behind the label) for legibility over a
  busy fill — a backend-agnostic approximation built from plain
  `draw_text` calls, so it needs no `DrawBackend`/`TextStyle` changes and
  works identically on the SVG/pixel/plotters backends. `geom_sf_label`
  additionally draws a background box, like `geom_label`.
  anofox-visualization's own label placement (`map/labels.rs`) can
  retire onto this (DataZooDE/anofox-visualization#318, #179).
- `ScaleFillBivariate` (biscale-style; `scale::bivariate::ScaleFillBivariate`)
  + `GeomBivariateLegend` (`GGPlot::bivariate_legend(..)`, feature `sf`): a
  fill scale mapping two already-binned variables (a `"{x}-{y}"` composite
  class, 1-indexed — the `biscale` R package's own convention; the binning
  itself stays the caller's job, as `ScaleFillBinned`'s break maths does)
  to an `nx × ny` palette, and its square legend guide with two axis
  captions. Each cell is `base − tx·(base − color_x) − ty·(base −
  color_y)` per channel (clamped): exactly `base` at low-low, exactly
  `color_x`/`color_y` at the pure-axis corners, compounding toward the
  high-high corner — `base` is white for light, or a theme's card colour
  "lifted" for dark (plan-maps.md §15's D10); a CIE76 ΔE test checks the
  9 cells stay distinguishable for both. `GeomBivariateLegend::from_scale`
  builds the legend from the same scale used for the fill, and (like
  `annotation_scale`) draws as a furniture layer, not through the ordinary
  per-aesthetic legend — `.legend_position(LegendPosition::None)` hides
  that one. Verified `geom_sf`'s ordinary Fill legend already follows an
  arbitrary custom theme (legend background/text/title colour, font
  family) with no new code needed — it rides the same generic
  discrete-legend path every other geom's Fill/Colour legend does, no
  sf-specific carve-out exists. anofox-visualization's own bivariate
  palette (planned, `docs/plans/plan-maps.md` M2d) can build on this
  (DataZooDE/anofox-visualization#318, #182).
- `geom_sf`'s `Polygon` now draws every ring (exterior + holes) as one
  shape under the even-odd fill rule, via a new
  `DrawBackend::draw_polygon_rings` (default impl: the old
  exterior-filled/holes-stroked-only approximation, for third-party
  backends; the SVG, pixel/`canvas` and plotters backends all override it
  with a real fill-rule/even-odd-scanline cut-out). A `MultiPolygon`'s
  parts already drew independently and keep doing so, each now correctly
  honouring its own holes. `GeomSf::simplify_px` (default `0.5`, builder
  `.simplify(px)`) simplifies every ring/line to the render resolution in
  screen space (Ramer-Douglas-Peucker) before drawing — a country
  boundary made of 10^5 points draws a fraction of that at a few hundred
  pixels across. Ported from anofox-visualization's `map/mod.rs`/
  `map/write.rs` (DataZooDE/anofox-visualization#318, #178).
  **Migration**: a polygon's holes were previously drawn as separately
  stroked (not filled-through) rings on top of the exterior fill — any
  snapshot test asserting on that exact (incorrect) look will need
  re-blessing; a hole now actually shows the panel background through
  it. `GeomSf { .. }` struct-literal callers gain the new `simplify_px`
  field (`..Default::default()` already covers it).
- `GGPlot::annotation_scale()`/`annotation_scale_with(..)` (`GeomAnnotationScale`,
  feature `sf`): a metric scale bar in a panel corner — ggspatial's
  `annotation_scale()`. A "nice" 1/2/5 × 10ⁿ distance is picked from the
  trained X scale's domain at the panel's centre height (correct for a
  non-conformal projection like Equal Earth/Albers too, not just a
  uniform one), capped at `max_width_px` (default 90) and omitted
  entirely below 12px. Assumes the X/Y domain is in metres (any `geom_sf`
  projection other than `SfProjection::PlateCarree`) — same constraint
  ggspatial's own `annotation_scale()` has for an unprojected CRS; it
  draws a (wrong, undetectable-as-wrong-by-magnitude) bar from the raw
  degree domain rather than guessing.
- `GGPlot::annotation_attribution(text)`/`_with(..)` (`GeomAttribution`,
  feature `sf`): a small, muted attribution text in a panel corner (e.g.
  `"© Natural Earth"`) — plain text only, no hyperlink (`DrawBackend` has
  no link primitive; out of scope for this change).
  anofox-visualization's own scale bar and attribution (`map/mod.rs`) can
  retire onto these (DataZooDE/anofox-visualization#318, #181).

Diagram primitives (#176): a node-link diagram (boxes with wrapped labels,
rounded corners, arrows between box edges) can now be drawn with geoms
only — no hand-rolled SVG. Tracked from
[DataZooDE/anofox-visualization#318](https://github.com/DataZooDE/anofox-visualization/issues/318),
which hand-rolled all three of these in its own `diagram.rs` for lack of
upstream support. No rendered output changes unless the new options are
used, but the new pub fields are an API break for exhaustive struct
literals (see **Migration**); cargo-semver-checks rates it major, i.e. the
next release needs a 0.x minor bump (0.27.0), not a patch.

**Migration:** code that builds `RectStyle`, `GeomRect`, `GeomTile`,
`GeomText` or `GeomLabel` with every field named (no
`..Default::default()`) must add the new fields — `corner_radius: 0.0` /
`radius: 0.0` / `wrap_width: None` / `line_height: 1.2` — or switch to
`..Default::default()`. Code using `::default()`, `..Default::default()`
or the `with_*` builders is unaffected.

### Added

- **Rounded rectangles** (ggplot2 `geom_label(label.r)`, ggchicklet
  `radius`): `RectStyle` gains a `corner_radius: f64` field (default `0.0`,
  a plain rectangle). `GeomRect`/`GeomTile` gain a `radius` field and
  `.with_radius(px)` builder method; `GeomLabel` gains the same for its
  background box. `svg_backend` emits a native `rx`/`ry`;
  `pixel_backend` (feature `canvas`) and `plotters_backend` (feature
  `plotters`), which have no rounded-rect primitive, approximate it as a
  polygon via the new `render::backend::rounded_rect_points` helper.
  `GeomRect` ignores `radius` under polar coords (drawn as an annulus
  sector, not a rect).
- **Text wrapping** (ggtext / `str_wrap` behaviour): `GeomText`/`GeomLabel`
  gain `.with_wrap(width_px)` (word-wraps to a max width, measured with
  `render::text::text_width`'s per-character metrics — the same table
  `axis_label_fit`/`check.overlap` already use, not a cruder estimate) and
  `.with_line_height(multiple_of_size)` (default `1.2`) for the gap between
  wrapped lines. `GeomLabel`'s background box grows to fit however many
  lines result, plus `padding`, and inherits the box's `radius`. The new
  `render::text::wrap_text`/`wrapped_block_width` do the wrapping and
  measure the widest resulting line. Unwrapped output (no `.with_wrap()`
  call) is byte-identical to before.
- **Arrow-to-box-edge** (arrows on segments/curves via `arrow()`, since
  0.20): no change needed to the arrow API itself — a segment/curve already
  draws to whatever endpoint its data gives it, tip included, so "attach to
  a box edge" was always just a matter of computing that endpoint. Added
  `geom::arrow::rect_edge_point(center, half_w, half_h, toward)`, a small
  public helper (ported from the consumer's hand-rolled `exit()`) that
  returns where a straight line from a box's centre toward a point leaves
  the box — feed its result as a `geom_segment`/`geom_curve` row's
  `(x, y)`/`(xend, yend)` instead of the box's raw centre. Documented with
  a worked example in its doc comment.
- `tests/diagram_primitives_176.rs`: an integration test building a 4-box
  node-link diagram (rounded boxes, a wrapped label, arrows landing exactly
  on computed box edges) through the public `GGPlot` builder API only,
  checked against the SVG output's actual `rx`, `<text>` count, and
  polyline endpoints — not just rendered and eyeballed.

### Scales / legend

- `ScaleFillBinned` (`scale_fill_binned(breaks, low, high)` /
  `scale_fill_binned_with(breaks, colors)`): a binned fill scale with
  *explicit* class breaks (`n + 1` increasing edges), one colour per bin —
  R's `scale_fill_steps(breaks = ...)`, backing a `::CLASSES` choropleth.
  The legend shows each class's trained count (`"10–50 (12)"`) and, when
  any trained value is `Value::Na`, a "no data" key in a configurable
  `na_color` (ggplot2's `na.value`, default grey50) — no changes needed to
  the existing discrete-legend drawing code, since both ride the same
  `Scale::breaks()`/`map_to_color()` round trip `ScaleColorSteps` already
  uses. The break *maths* (quantile/Fisher-Jenks/equal-width/custom edges)
  stays the caller's job — this only bins against the edges it's given.
  anofox-visualization's own map legend (`map/legend.rs`) can retire its
  stepped-class rendering onto this (DataZooDE/anofox-visualization#318,
  #180).
- **Fix**: `ScaleSizeContinuous::map_fraction_to_size` (used to size a
  graduated-symbol legend's keys) ignored `scale_size_area`'s area-
  proportional mapping entirely and always used the plain linear `range`
  — so a `scale_size_area` legend's circles didn't match the radii the
  data itself draws at. It now uses the same `max_size · √(|v|/denom)`
  formula `map_to_size` does.

- `PlotGrid::axis_collect` and [`AxisCollect`] (#175): align a column of
  stacked plots' plot areas and/or share one x domain across them —
  patchwork's `plot_layout(axes = ...)` idea, applied to the x axis and
  plot-area alignment specifically. `AxisCollect::Align` gives every cell
  in a column the same left/right plot-area inset, computed from the
  widest y-axis label requirement in that column (the same idea
  anofox-visualization's `track.rs` uses via a render-measure-render loop,
  done here as one layout pass instead). `AxisCollect::CollectX` adds a
  shared x domain — the union of every cell's own trained domain — with
  x-axis ticks/title/labels drawn only on the bottom cell of each column;
  a column whose x scale is discrete or untrained on any cell keeps
  `Align`'s plot-area alignment but warns and skips the domain union.
  Every aligned cell also switches to
  `axis_text_x_fit(AxisLabelFit::Shift)`, so an overflowing outer tick
  label can never perturb the alignment. Default `AxisCollect::Keep`:
  byte-identical output when not called. Tracked from
  DataZooDE/anofox-visualization#318 (visualization logic belongs in
  ggplot-rs, not the consumer); `track.rs` keeps its own
  render-measure-render approach for now.

## [0.26.0] — 2026-10-24

Minor bump: log and sqrt axis ticks now land on round numbers (an output
change for `scale_x_log10`/`scale_y_log10`/`scale_x_sqrt`/`scale_y_sqrt` and
their `log2`/`ln` siblings; see **Migration**).

### Fixed
- **Log and sqrt axes pick round ticks, not raw half-decades/squares**
  (#413, upstream issue DataZooDE/anofox-visualization#413). A
  `scale_y_log10()`/`scale_x_log10()` axis previously placed ticks at
  `extended_breaks()` computed directly in log space, which inverse-
  transform to half-decades like `316.23`/`3162.28` — unround numbers no
  label formatter can fix. Likewise `scale_y_sqrt()`/`scale_x_sqrt()` ticked
  at evenly-spaced sqrt-space values, which square back to `625`, `2500`,
  `5625`, etc.
  - Log axes (`Log10`/`Log2`/`Ln`) now use a `scales::log_breaks()`-style
    algorithm (`ScaleTransform::breaks_in_data_space`,
    `scale::util::log_breaks`): prefer whole powers of the base; when the
    domain spans too few decades for the requested tick count, fill in with
    evenly-spaced sub-multiples (`{1,2,5,...} * base^n`, as `log_sub_breaks`
    does in R's `scales`); fall back to `extended_breaks()` over the data
    range if neither gives enough ticks.
  - Sqrt axes now place ticks via `extended_breaks()` over the
    *untransformed* data range (matching `scales::sqrt_trans()`), not the
    sqrt-transformed range.
  - `ScaleContinuous::with_breaks(...)` (explicit breaks) is unaffected and
    still overrides this placement entirely.
  - This only changes `ScaleContinuous`/`scale_{x,y}_{log10,log2,ln,sqrt}`
    axes; `coord_trans` (`CoordTrans`), whose ticks come from the
    underlying linear scale and are warped at draw time, was already round
    and is unchanged.
  **Migration:** a plot using a log10/log2/ln/sqrt scale (directly or via
  `YSCALE`/`XSCALE` in a downstream consumer) now renders different tick
  positions and labels — round numbers instead of half-decades/squares.
  Nothing else about the API changes; a plot pinning tick positions with
  `.with_breaks(...)` is unaffected.

## [0.25.0] — 2026-10-23

Minor bump: `geom_area()`'s default stat is now `stat_align()`, as in
ggplot2 (an output change for areas with more than one series; see
**Migration**).

### Added
- `StatAlign` (`stat_align`, #172). Before stacking, it interpolates every
  group of a panel onto shared x positions: every x, each group's zero
  crossings, and a tiny step either side of them. It pads each group with
  `y = 0` just outside its range. A single group passes through unchanged;
  a group with one distinct x is dropped; repeated x become a vertical step.
  This follows ggplot2 4.0.3's `StatAlign`, checked against its
  `layer_data()` (`tests/geom_area_stat_align.rs`).

### Changed
- **`geom_area()` uses `stat_align()` by default** (#172, ggplot2 parity).
  Stacked areas whose series are sampled at different x no longer dip where
  one series has no row. This was the known limitation of 0.24.0.
  **Migration:** to keep the 0.24.0 output, a plot with several series per
  panel sets `.stat(StatIdentity)` on the layer. A single-series area is
  unchanged.

## [0.24.0] — 2026-10-23

Minor bump: `geom_col()` and `geom_area()` now stack by default, like
ggplot2 (an output change; see **Migration** below), plus legend-layout
and missing-value stacking fixes from the #167 review, each guarded by a
test.

### Changed

- **`geom_col()`'s default position is now `position_stack()`**, matching
  `geom_bar()`'s (#169) — ggplot2 defaults `geom_col` to `position_stack`
  too, and 0.22/0.23 left it at `PositionIdentity`, a parity gap called
  out in review #167 finding 3 and tracked as #169. **Migration**: a plot
  that calls `geom_col()` with more than one group (e.g. a mapped
  `fill`/`colour`) at the same x and relies on the old identity behaviour
  (bars drawn on top of each other from `y = 0`, not stacked) must now
  add an explicit `.position(PositionIdentity)` to keep that output; a
  plot that already set `.position(...)` explicitly, or that never has
  more than one group at a shared x, is unaffected. The `ggplot` CLI's
  `--geom col` changes the same way (`src/bin/ggplot.rs`, same
  `GGPlot::geom_col()` call). `tests/default_output_023.rs`'s
  `all_positive_stack` case (`geom_col()`'s default, two groups at one
  x) exercises this — see its updated expected hash and data-fn comment;
  `tests/issue169_geom_col_default_stack.rs` checks the default matches
  an explicit `PositionStack` and no longer matches `PositionIdentity`.
  `tests/integration.rs`'s `test_layer_position_override_build` switched
  from `geom_col()` to `geom_point()` to keep testing an override of an
  *identity* default.
- **`geom_area()`'s default position is now `position_stack()` too**,
  matching ggplot2 (review #167 finding 5 on #171) — the same parity gap
  as #169, landed in this same minor so every default-position break
  ships in one release instead of trickling in across several.
  **Migration**: a plot that calls `geom_area()` with more than one
  series (a mapped `fill`/`colour`/`group`) at the same x and relies on
  the old identity behaviour (bands drawn on top of each other from
  `y = 0`, not stacked) must now add an explicit
  `.position(PositionIdentity)`; a plot with only one series, or that
  already set `.position(...)` explicitly, is unaffected. The `ggplot`
  CLI's `--geom area` changes the same way. `tests/default_output_022.rs`'s
  `area` case changes too, for an unrelated reason worth calling out: its
  fixture (`band_data()`) happens to carry its own pre-existing `ymin`
  column (there for `geom_ribbon`'s case, not this one), which
  `PositionIdentity` always left untouched but `PositionStack` now
  overwrites with the (single-series, all-zero) baseline it computes —
  see that file's updated expected hash and docstring note.
  **Known limitation**: ggplot2 pairs `geom_area` with `stat_align`
  (each series interpolated onto the union of x before stacking);
  ggplot-rs stacks by exact x, so a series missing an x makes the series
  above it dip at that x (#172).
  `tests/geom_area_default_stack.rs` checks the default matches an
  explicit `PositionStack` and no longer matches `PositionIdentity`.

### Fixed

- A vertical legend's explicit `ncol` grid that auto-reduces its column
  count to fit (#159) now reclaims the panel width that reduction no
  longer needs, instead of keeping the wider, pre-reduction column
  count's reservation forever (#168, review #167 finding 5 on #171).
  Reclaiming it is a single bounded retry, not an unconditional
  recompute: the tighter reservation is re-checked against the same fit
  test the reduction loop uses, against *every* currently-planned guide
  — not only the guide(s) the reduction loop itself reduced — and is
  kept only if every one of them still fits under it. A follow-up review
  round caught that first cut checking only the reduced guides: `legend_x`
  moves for *every* guide sharing the row once the reservation tightens,
  not just the one(s) reduction touched, so a second guide that already
  fit at the old, wider `legend_x` (its base width estimate is a fixed
  `6·text_size`, unrelated to the reduced guide's own labels), or whose
  labels were truncated against that old `legend_x`, shifted right by
  the same amount with nothing re-checking it — up to 40px off-canvas
  with no warning in a 7,992-plot sweep, 7.3% of cases with two
  simultaneous guides. Checking every plan removed all of those. A grid
  that would overflow at the tighter reservation even after that (the
  pre-existing #159 limitation where an explicit `ncol(1)` grid has no
  fit check of its own) keeps the wider, already-safe reservation
  instead, so this can only shrink the reservation when doing so is
  provably safe, never regress a plot that already rendered correctly.
  `tests/legend_wrap.rs`'s `legend_panel_width_is_reclaimed_after_column_reduction`
  and `legend_panel_width_reservation_is_not_loosened_when_unsafe` cover
  the single-guide case (both sides); `two_guides_ncol_retighten_repro_a_stays_on_canvas`
  and `..._repro_b_stays_on_canvas` cover the two-guide regression
  (untouched guide and truncated guide respectively).
- A bottom/top legend mixing a colourbar (a continuous `colour`/`fill`
  scale) with a discrete guide now leaves the discrete guide only the
  row width left over after the colourbar's own fixed share, instead of
  splitting the *whole* row width among the discrete guides alone — the
  wrapped discrete grid could still run past the canvas once the
  colourbar's share was added back on top (review #167 finding 7).
  `tests/legend_wrap.rs`'s `bottom_legend_colourbar_leaves_room_for_the_discrete_guide`.
- A `PlotGrid`'s collected-legend path (`collect_legends(true)`) now
  propagates the warnings its own legend planning produces (e.g. an
  invalid `nrow(0)`/`ncol(0)` on the collected legend), instead of
  silently discarding them — unlike the same setting on a single plot's
  own legend, which already warned (review #167 finding 6).
  `tests/legend_wrap.rs`'s `plotgrid_collected_legend_propagates_its_own_warnings`.
- The legend bbox pre-pass used to measure a `PlotGrid`'s collected
  legend (`compose::capture_legend`'s internal `Measure` backend) now
  forwards a drawn label's `<title>` tooltip to its inner backend
  instead of silently dropping it (review #167 finding 4) — latent
  today (a collected legend's labels never get 0.23's truncate-with-
  `<title>` treatment yet), but would otherwise have resurfaced as soon
  as they did. `src/compose.rs`'s
  `measure_forwards_the_title_to_its_inner_backend` unit test.
- A missing (`NaN`/SQL `NULL`) `y` is now skipped in `position_stack`
  and `position_fill` (it contributes **0** to its bucket's running
  total/denominator) and its own `y`/`ymin`/`ymax` are left `NaN` — not
  drawn — matching ggplot2's `pos_stack`/`position_fill` exactly (review
  #167 finding 2/3 on #171). `±Inf` is treated as missing the same way.
  This only matters for `geom_line`/`geom_step`/
  `geom_path`, which keep a missing row as a line break instead of
  dropping it (`Geom::keeps_na_as_break`); every other geom already drops
  non-finite rows before `Stat`/`Position` ever see them. Also fixes a
  pre-existing, unrelated bug the same code touches: a `Value::Na` `y`
  (SQL `NULL`, as opposed to a float `NaN`) used to fall back to `0.0`
  via `as_f64().unwrap_or(0.0)` and get stacked as a *real* point at the
  top of its bucket — a line drawn straight through a row the "removed 1
  row" warning said was gone. `src/position/stack.rs`'s
  `missing_y_contributes_zero_and_does_not_poison_its_bucket` and
  `value_na_is_treated_like_float_nan_not_zero`, `src/position/fill.rs`'s
  sibling test, and `tests/position_stack_r.rs`'s
  `stacked_line_with_a_missing_y_leaves_other_series_intact_at_that_x`
  (an end-to-end `geom_line` regression, since `keeps_na_as_break` is
  what makes this reachable at all).

## [0.23.0] — 2026-10-22

Minor bump: per-panel dodge/jitter, negative stacking, legend wrap for
grids/multiple guides, vertical legend fit. Default output stays byte-
identical except for the deliberate changes under "Changed", each guarded
by a test.

### Added

- `DrawBackend::draw_text_with_title` (the default draws the text and
  drops the title; the native SVG backend writes the `<title>`) and `render::text::truncate_to_width` (a new `pub
  fn` in the public `render::text` module): draws (or measures) a text
  run with an attached full-text `<title>` tooltip, used by #159's label
  truncation below to keep a truncated label's full text available via
  hover even though the drawn text itself is shortened. The truncated
  label is a plain text run, not a `gg-mark`: an SVG consumer that treats
  "has a `<title>`" as "is a data mark" should select on `gg-mark` instead.
- `Position::group_order` (default `None`) and
  `PositionParams::group_order`: the layer-wide group order a faceted
  dodge/dodge2 ranks its groups by (#153, see Changed).
- `tests/default_output_023.rs`, on the same pattern as
  `tests/default_output_022.rs`: a battery of fixed plots exercising
  #153/#154/#155/#159's deliberate changes plus their byte-identical
  unaffected counterparts, hashed with FNV-1a. Wired into CI's `sf`,
  no-default-features test job alongside `default_output_022`. Extended
  (review #167) with a `PositionFill` mixed-sign case alongside
  `PositionStack`'s, and `tests/position_panels_r.rs` gained a dodge-
  order case with two panels listing their groups in opposite row order
  (#153 finding 2), and `tests/legend_wrap.rs` gained a `PlotGrid`
  collected-legend case with two simultaneous discrete guides (#155
  finding 1).

### Breaking

- `PositionParams` has a new public field, `group_order:
  Option<Vec<String>>` (#153). Code that builds `PositionParams` with a
  struct literal must add `group_order: None` (or use
  `..Default::default()`). `cargo semver-checks` flags this as
  `constructible_struct_adds_field`; it's the only API break.

### Deprecated

- `guide::legend::draw_legend` is deprecated (`#[deprecated(since = "0.23.0")]`)
  and will be removed in a later minor release: it's unused internally as
  of this release — #155's rerouting of `compose::capture_legend` to the
  crate-internal `draw_legend_planned` (so a `PlotGrid`'s collected legend
  gets the same wrap/grid treatment a single plot's legend does) removed
  `draw_legend`'s only caller. There is no direct public replacement
  (`draw_legend_planned` is `pub(crate)`); legends are drawn automatically
  by `GGPlot`'s own render methods, which was always the supported way to
  get a legend onto a plot.

### Changed

- A faceted `position_dodge`/`position_dodge2` now computes each panel's
  group *count*/offset from that panel's own rows instead of the rows
  pooled across panels (#153), matching ggplot2 4.0.3 (verified with
  `layer_data()`: a group missing from one panel's x level no longer
  shifts the groups that panel does have). Group *rank* — which group
  dodges left vs right — still comes from the whole layer's first-
  appearance order (review #167 finding 2), not from each panel's own
  row order, so every panel agrees, as in ggplot2 4.0.3. (ggplot2 ranks by
  the sorted `group` id, so the two still differ when the data's first
  appearance order isn't sorted; 0.22.2 behaved the same.) A panel
  whose rows happen to list two groups in a different order than
  another panel still draws a given group on the same side in both. A
  faceted `position_jitter`'s default width/height (`resolution(x) *
  0.4`) is now likewise measured per panel — also verified with
  `layer_data()`: two panels with very different x spacing get
  differently-scaled jitter even under shared, fixed facet scales. A
  plot with 2+ distinct facet panels and a dodge or jitter layer may
  render differently; an unfaceted plot, or one with a single panel, is
  unaffected (`tests/position_panels_r.rs`, including
  `facet_dodge_order_is_global_not_per_panel_first_appearance`).
- `position_stack`/`position_fill` now split a group's positive and
  negative values at each x into two independent stacks instead of one
  pooled running total (#154), matching ggplot2's internal `pos`/`neg`
  split (`collide`/`pos_stack`): positives stack upward from 0 using only
  the positive sum as the total (or `position_fill`'s normalisation
  denominator), negatives stack downward from 0 using only the
  (absolute) negative sum, each with its own `vjust` anchoring, verified
  against ggplot2 4.0.3 with `layer_data()` (a 4-group bar at one x with y = 3,
  -2, 4, -1: `position_stack(vjust = 0.5)` gives y/ymin/ymax (5.5, 4, 7),
  (-2, -3, -1), (2, 0, 4), (-0.5, -1, 0); `position_fill(vjust = 0)` gives
  (4/7, 4/7, 1), (-1, -1, -1/3), (0, 0, 4/7), (-1/3, -1/3, 0)). All-
  positive (or all-negative) data at an x is byte-identical — the other
  sign's bucket is simply empty (`src/position/stack.rs`,
  `src/position/fill.rs` unit tests).
- A plot's auto-wrap/grid legend layout (0.22) now applies to every
  simultaneous discrete guide independently, not only when the plot has
  exactly one (#155): two discrete guides (e.g. `colour` and `shape` both
  mapped to discrete variables) laid out side by side each wrap against
  an equal share of the available width, and a vertical explicit
  `nrow`/`ncol` grid on either reserves its own extra width. Because
  `GuideLegend` (the `.guides(...)` argument) is a single, global
  setting, an explicit `nrow`/`ncol` on it now applies to *every*
  discrete guide, including one sitting next to a continuous guide
  (e.g. a colourbar plus a discrete `shape`) — a discrete guide next to
  a continuous one now participates in auto-wrap/grid too. In 0.22 an
  explicit `nrow`/`ncol` was silently ignored as soon as the plot had
  two or more legend scales of any kind, discrete+discrete or
  discrete+continuous. A `PlotGrid`'s `collect_legends(true)` strip also
  now wraps its collected legend against the grid's own page size,
  instead of always drawing the plain, unwrapped path — previously the
  only legend eligible for wrap/grid was a single plot's single discrete
  guide, rendered directly (not collected into a grid strip); two
  simultaneous discrete guides collected this way are laid out side by
  side, each against its own equal share of the page width, the same as
  a single plot's legend (review #167 finding 1 — `capture_legend`
  measures the collected legend against a fake zero-width panel, so the
  step to the next guide must use the share `plan_all_legends` actually
  planned against, not that panel's own width). A plot with one discrete
  guide that already fit is unaffected (`tests/legend_wrap.rs`'s
  `two_discrete_guides_each_auto_wrap`,
  `plotgrid_collected_legend_auto_wraps` and
  `plotgrid_collected_legend_two_guides_sit_side_by_side` cover the new
  cases).
- A vertical legend's explicit `ncol` grid that still runs off the canvas
  after reserving `(cols - 1) * col_pitch` (0.22's "warn only" check) now
  tries harder before giving up (#159): it first reduces the column
  count toward 1, keeping the first count that fits; if even one column
  overflows, it truncates labels with an ellipsis instead (the full label
  stays available via a `<title>` tooltip on the label text, escaped the
  same way every other text run is — `DrawBackend::draw_text_with_title`, new,
  default no-op, SVG-only); only if it still doesn't fit after both does
  it warn, as before. A grid that already fits renders byte-identical to
  0.22.2 — no reduction, no truncation, no warning
  (`tests/legend_wrap.rs`'s `a_vertical_grid_that_already_fits_is_byte_identical`,
  plus the reduce/truncate/still-warns cases replacing
  `vertical_legend_ncol_grid_too_wide_for_the_canvas_warns`, which
  asserted the old "always warns" behaviour).

## [0.22.2] — 2026-10-22

Patch: decorative layers, fill value formatter, refline tooltips,
annotation ink, boxplot identity mapping.

### Added

- `GGPlot::decorative()` marks the most recently added layer as purely
  decorative (#161): its drawn marks keep their own visual style but carry
  no per-mark metadata — no `<title>`, no `data-x`/`data-series`/
  `data-value`/`data-fvalue`, and no `data_attr` attributes — so an SVG
  consumer that treats "has a non-empty `<title>`" as "is a selectable
  data point" doesn't pick it up (e.g. a boxplot's own mean dot, drawn by
  a separate `geom_point` layer purely to mark the spot). Implemented as a
  private delegating `Geom` wrapper rather than a new `Layer` field, so it
  adds no field to any public struct. A `.tooltip(...)` on a decorative
  layer has no visible effect and warns, like the existing has-no-effect
  warning. A decorative layer also defaults to hidden in the legend,
  unless `show_legend(true)` is called to opt back in. Its marks are tagged
  `data-part="decorative"` and `aria-hidden="true"` (keeping `data-geom`),
  and their class is `gg-mark gg-decorative` instead of the geom's own
  (`gg-point`, `gg-line`, …), so a stylesheet selecting on the geom class
  doesn't restyle them as data marks.
- `with_value_formatter` (same signature as
  `ScaleContinuous::with_value_formatter`) on every continuous colour/fill
  scale that implements `format_value`: `ScaleColorContinuous`,
  `ScaleColorGradientN` and `ScaleColorGradient2` (#164). Falls back to
  the plain locale-formatted number when unset (byte-identical default
  output).

### Fixed

- `geom_hline`/`geom_vline`/`geom_abline` now implement `Geom::
  consumes_tooltips` and read a layer's `tooltip()` template (e.g.
  `.tooltip("{xintercept}")`), replacing their own default `"x = …"`/
  `"y = …"` label the same way every other `consumes_tooltips` geom does;
  previously the template was silently dropped with a has-no-effect
  warning (#165). The default label (no template set) is unchanged.
  A mapped reference line's `data_attr` attributes no longer carry over
  onto the marks drawn after it (annotations, gridlines drawn on top,
  legend keys).
- `annotate_text`/`annotate_segment` now follow theme `element_geom` ink
  when their colour is still at its hard-coded default, the same as every
  other geom's default colour (#157, follow-up to #142) — previously they
  stayed near-black and nearly invisible under a white-on-black theme.
  `annotate_rect`'s fill is deliberately excluded, matching ggplot2
  4.0.3's own behaviour (checked directly: its fill stays the hard-coded
  grey under `element_geom(ink=, paper=)` too). Default theme (no ink set)
  is byte-identical.
- `geom_boxplot(stat = "identity")`: a raw, unmapped `ymin`/`ymax` column
  no longer satisfies the five-number-summary requirement, matching
  ggplot2 4.0.3 (checked directly) and the same "only when actually
  mapped" treatment `lower`/`middle`/`upper`/`outliers` already got (#160,
  follow-up to #158). Scoped to `GeomBoxplot` alone; the universal raw-y
  convention for other geoms is unchanged.

## [0.22.1] — 2026-10-21

Patch: discrete-axis auto-rotate (#162).

### Fixed

- The crowded-discrete-axis auto-rotate heuristic (0.22) counted every
  break in `breaks()`, including entries a `with_labels` override blanks
  (`""`) to thin a crowded axis while keeping every discrete position —
  over-triggering 45°/90° rotation over labels that would fit flat. The
  crowding count and the label-width estimate now only consider the
  labels actually drawn, measured with `render::text::text_width`
  (sipemu/ggplot-rs#162).
- Rotated bottom-axis labels — whether rotated by the auto-rotate
  heuristic or by an explicit `axis_text_x_angle` — could overlap the
  panel: the layout reserved a blind constant height that didn't grow with
  the label's real rotated extent, and the rotation anchor itself was
  placed at a fixed small offset from the axis line regardless of how
  much was reserved, so a long enough label's far end swung right back
  over the panel. `layout_built`'s reservation and `draw_x_axis`'s
  rotation-anchor placement now both derive from the same measured,
  per-label rotated bounding boxes (`render::text::rotated_label_rise_fall`),
  for both auto-rotated and explicitly-angled labels, on bottom and top
  axes alike.
- **Changed rendering:** auto-rotated crowded discrete axes now use −45°
  (was +45° in 0.22.0), ggplot2's `angle = 45, hjust = 1` look mirrored to
  this library's angle sign. Every rotated label, auto or explicit, is
  anchored so its near-axis end sits at its own tick, on bottom and top
  axes (`render::text::rotated_anchor_frac`). Crowded discrete axes
  therefore render differently from 0.22.0.

## [0.22.0] — 2026-10-20

Minor bump: layout and stats (upstream batch 2). Default output stays
byte-identical except for the deliberate changes under "Changed", each
guarded by `tests/default_output_022.rs`.

### Added

- `render::text::text_width(text, size)` and ggplot2's `check.overlap`
  priority order for axis labels, ported from the consumer's `ticks.rs` so
  ggplot-rs owns outer-label overflow and overlap thinning centrally.
- `Theme.axis_text_x_fit: AxisLabelFit { Clip, Shift, #[default] Reserve }`
  and `Theme.axis_text_x_check_overlap` (default `false`), with
  `GGPlot::axis_text_x_fit`/`axis_text_x_check_overlap` builders.
- `GuideLegend.byrow`/`max_rows`, with `with_byrow`/`with_max_rows`
  builders: an explicit `nrow`/`ncol` legend grid (column-major unless
  `byrow`), and an auto-wrapped horizontal legend that ends in a trailing
  "+N more" when it still doesn't fit `max_rows`.
- `Position::per_panel()` (default `false`) and `Position::validate()`
  (default no-op); `PositionStack`/`PositionFill` are per-panel.
- `PositionStack`/`PositionFill` gain a `vjust` field (default `1`, the
  previous top-anchored behaviour) with a `with_vjust` builder and free
  functions `position_stack()`/`position_fill()`, matching ggplot2's
  `position_stack(vjust =)`/`position_fill(vjust =)` (#147). `vjust != 1`
  also writes an `ymax` column holding the segment's top.

- `ScaleSizeContinuous::area(max_size)` / `GGPlot::scale_size_area(max_size)`
  (ggplot2's `scale_size_area()`, #150): size ∝ √(value) so that *area*
  (not radius) is proportional to the data, with 0 mapping to 0. NA/non-
  finite input maps to `None`. A non-finite or non-positive `max_size`
  warns and falls back to 6.
- `geom_boxplot(stat = "identity")`: new `Lower`/`Middle`/`Upper`/
  `Outliers` aesthetics (`Aes::lower`/`middle`/`upper`/`outliers`, D4
  extensions) let a layer supply the five-number summary directly instead
  of ggplot-rs computing it, matching ggplot2 (#149) — `ymin`/`ymax` are
  also required, as in ggplot2 4.0.3. `outliers` takes a comma list with
  optional brackets (`"7, 9"` or `"[7, 9]"`), or a single bare number; an
  unparsable part is skipped with one warning. The boxplot now consumes
  `tooltip()`/`data_attr()` (`consumes_tooltips` is true).
- `Geom::mapped_only_columns()` (D3, default empty — `GeomBoxplot` returns
  `lower`/`middle`/`upper`/`outliers`): a raw column of that name is
  dropped right after `resolve_mappings` unless the aesthetic was
  actually mapped, so a column that merely happens to share the name
  (e.g. a SQL alias) can't be mistaken for a mapped one, and so a raw
  `lower` is never drawn as a boxplot hinge. Scoped to `GeomBoxplot`
  alone — see the `Value::Na`/non-finite drop bullet under "Changed" for
  how an unmapped `lower`/`middle`/`upper` on any *other* geom is handled
  without removing it from the data (review #158's final check).
- `Geom::validate_layer(pre_stat_columns, stat_name)` (default no-op):
  stat-dependent validation beyond `required_aes`. Under
  `stat = "identity"`, `geom_boxplot` requires `lower`/`middle`/`upper`/
  `ymin`/`ymax` to be mapped (a `ValidationError` otherwise, matching
  ggplot2 4.0.3's own `stat_identity` boxplot requirement); under the
  default stat, it requires `y`.

### Changed

- Outer x-axis labels that would cross the SVG edge are kept on-canvas by
  default (allowed #1, #2 — `AxisLabelFit::Reserve`, the new default):
  the panel narrows by just the overflow if `Reserve` can reserve enough
  margin, and a label whose own tick has scrolled off the canvas is
  dropped rather than drawn off-page. A plot whose outer labels already
  fit is unaffected (`tests/axis_label_fit.rs`;
  `default_output_022`'s `wide_axis` case, re-blessed when the Reserve
  formula bug — finding 1 below — was fixed). The total reservation is
  now capped so the panel can never start beyond the canvas or shrink
  past its own minimum width (review #158's second re-review, LOW): a
  single, centred break whose label is wider than the whole canvas falls
  back to `Shift`'s draw-time adjustment for whatever still overflows
  past the cap (`tests/axis_label_fit.rs`).
- A single horizontal legend that overflows its one-line layout now
  auto-wraps to more rows instead of running off the panel (allowed #3,
  D7); beyond `max_rows` it ends in a trailing "+N more" with a build
  warning. A legend that already fits on one line is unaffected. A
  vertical `ncol` legend grid that still overflows the canvas after its
  width is reserved now also gets a build warning (review #158's second
  re-review, LOW; `tests/legend_wrap.rs`).
- Tooltip-template placeholders for every position column
  (`xmin`/`xmax`/`xend`/`ymin`/`ymax`/`yend`/`lower`/`middle`/`upper`,
  D9 allowed #7) are now formatted through the x/y scale, the same as the
  bare `x`/`y` they're derived from — previously they fell back to the
  plain display format, ignoring a custom formatter or a log-transformed
  scale's inverse transform. A `tooltip()` template that names one of
  these columns on a transformed scale, or with a custom number
  formatter, now shows a different (correct) value.
- `geom_boxplot` outliers now widen the trained y domain to include every
  outlier value instead of silently leaving it at the whiskers' extent
  (allowed #6; confirmed as a real bug with a failing test before the
  fix landed — see `tests/boxplot_identity.rs`). A boxplot whose outliers
  fall outside `ymin`/`ymax` renders with a taller y axis.

- A faceted `position_stack`/`position_fill` now computes per panel
  instead of on the rows pooled across panels before faceting, matching
  ggplot2 (#146). A plot with 2 or more distinct facet panels and a stack/
  fill layer may render differently; one panel (or none) is unaffected.
- `geom_text`/`geom_label` now honour a discrete `position_dodge`, via the
  same `XMapper` bar/col/point/errorbar/… already use (#148). A plot that
  dodges text/labels on a discrete x axis renders differently; text/
  labels without a dodge are unaffected.
- `geom_errorbar`'s whisker caps shrink by the dodge group count under a
  discrete `position_dodge`, so caps from adjacent groups no longer
  visually overlap (#152). An un-dodged errorbar is unaffected.
- A `Value::Na` (e.g. a SQL NULL) in a position column that is never
  legitimately discrete (`ymin`/`ymax`/`xend`/`yend`/`lower`/`middle`/
  `upper`/…, i.e. every position column except `x`/`y`, which may hold
  discrete category values) is now dropped with the standard "removed N
  row(s) containing non-finite values" warning, the same as `Float(NaN)`
  already was — previously it silently mapped to 0 with no warning
  (review #158 finding 4). This applies to every geom that uses one of
  these columns, not just `geom_boxplot`: e.g. `geom_errorbar`/
  `geom_pointrange`/`geom_ribbon` with a `Value::Na` `ymin` now drop that
  row with a warning instead of silently drawing it from 0 — matching
  ggplot2, which has always dropped a missing aesthetic's row this way.
  The new `lower`/`middle`/`upper` columns also drop a non-numeric,
  non-`Na` value (a stray string) the same way, since they have no
  legitimate discrete use. A plot whose position data has no `Na` is
  unaffected.
- `lower`/`middle`/`upper` are excluded from the above `Value::Na`/non-
  finite drop, the pre-stat y-scale transform, and the y-scale training
  *when the layer never mapped the corresponding aesthetic* — a raw,
  unmapped column under one of these names (e.g. a forecast's confidence-
  bound columns on a plain `geom_line`) is otherwise left completely
  untouched in the layer's data, so a `tooltip("{lower}")` placeholder or
  a `facet_wrap("upper")` on it still works, exactly as any other
  unmapped column would (review #158's final check — round 3's universal
  drop of these names from the data for every geom was itself a
  regression on both of those). `geom_boxplot`'s own hinges — mapped
  under `stat = "identity"`, or computed fresh by its default stat — are
  unaffected either way.

### Breaking

- `Theme` gains two new public fields, `axis_text_x_fit: AxisLabelFit` and
  `axis_text_x_check_overlap: bool` — an external `Theme { .. }`
  struct-literal pattern without `..Default::default()` (or
  `..theme_gray()`/another preset) no longer compiles.
- `GuideLegend` gains two new public fields, `byrow: bool` and
  `max_rows: Option<usize>` — same breakage for an external
  `GuideLegend { .. }` struct literal without `..Default::default()`.
  Neither struct is `#[non_exhaustive]`.
- `guide::axis::draw_x_axis` gained a `canvas: &Rect` parameter (the full
  SVG viewport, not just the panel) — a deviation from the plan, needed
  for the outer-label-fit edge checks (B3-1): an off-canvas label can't
  be detected from the panel rect alone. An external direct caller of
  `draw_x_axis` needs to pass it (`backend.total_area()`).
- `PositionStack`/`PositionFill` are now structs with a `vjust` field
  instead of unit structs — an external `PositionStack { .. }`/
  `PositionFill { .. }` struct-literal pattern no longer compiles (the
  bare value `PositionStack`/`PositionFill` still does, via a `const` of
  the same name — dodge.rs's existing `PositionDodge` precedent).
- New `Aesthetic` variants `Lower`/`Middle`/`Upper`/`Outliers`: an
  exhaustive external `match` on `Aesthetic` no longer compiles.
- `Geom::setup_data` gained a second parameter,
  `warnings: &mut Vec<String>` (a deviation from the plan, needed so
  `GeomBoxplot::setup_data` can warn on an unparsable outlier value
  through the existing warnings channel rather than a new one or
  `eprintln!`): `fn setup_data(&self, data: &mut DataFrame)` becomes
  `fn setup_data(&self, data: &mut DataFrame, warnings: &mut Vec<String>)`.
  An external `Geom` impl overriding it needs the new parameter.

- The `arrow` feature now pins `arrow = "59"` (was `"53"`), still with
  `default-features = false`. `GGPlot::new(batch)` and the `GGData` impl for
  `arrow::record_batch::RecordBatch` compile unchanged against arrow 59 —
  no source changes were needed in `src/data/source.rs`. This lets Arrow-
  native producers on arrow 59.x (e.g. the inventory-rs DuckDB/Parquet
  ingest path) hand `RecordBatch`es to ggplot-rs directly, without a copy
  through an intermediate arrow 53 batch (#123). The `polars` feature is
  unaffected: polars vendors its own `polars-arrow` crate rather than
  depending on the `arrow` crate, so the two can be enabled together with a
  single `arrow` v59.3.1 in the dependency tree. This is a semver-relevant
  change for `arrow`-feature users: code that names arrow-53-specific types
  (e.g. via `arrow = "53"` pinned directly) needs to move to arrow 59 too,
  since arrow does not guarantee cross-major type compatibility. Originally
  sipemu/ggplot-rs#143; folded into 0.22.0.

## [0.21.1] — 2026-10-20

Patch bump: additive and bug fixes only, no API break. Default output is
byte-identical (`ElementGeom` unset, as before).

### Fixed

- `element_geom` (`ink`/`paper`) now covers every other geom that hard-codes
  a black/`(50, 50, 50)`/white default, following 0.21.0's
  `Geom::apply_element_geom` pattern (gated on `explicit_style`, so an
  explicit `geom_*_with(...)` colour is untouched even when it equals the
  default): `GeomSegment`, `GeomCurve`, the reference lines (`GeomHline`/
  `GeomVline`/`GeomAbline`), `GeomErrorbar`/`GeomErrorbarh`/`GeomLinerange`/
  `GeomPointrange`, `GeomStep`, the outline colours of `GeomRect`/`GeomTile`/
  `GeomBar`/`GeomCol`/`GeomPolygon`/`GeomViolin`/`GeomDensity`/`GeomArea`/
  `GeomCount`/`GeomHex`/`GeomContour`/`GeomHistogram`/`GeomDensity2d`/
  `GeomBin2d`/`GeomSf` (feature `sf`), `GeomCrossbar` (outline + fill),
  `GeomTextRepel`, `GeomLabelRepel` (outline + fill), `GeomSpoke`, `GeomRug`,
  `GeomBracket`, `GeomJitter`, `GeomQQ`, `GeomCensorMarks` and
  `GeomFreqpoly`. `apply_element_geom` is still a no-op for: `GeomDotplot`'s
  `color` field (dead config — only `fill` is ever painted); `GeomQQLine`'s
  red default and `GeomCandlestick`/`GeomOhlc`'s green/red `up`/`down`
  (deliberate semantic colours, not neutral); `GeomQQBand`'s mid-grey
  `(153, 153, 153)` fill (doesn't match `ink`'s black/`(50, 50, 50)` line
  check or `paper`'s white-fill check — extending `ElementGeom` to a grey
  band is a bigger change, left for a follow-up); and `GeomCalendarBorder`,
  which already reads its colour from the dedicated
  `Theme::calendar_month_border` element rather than a hard-coded default.
  Closes #142.
- The discrete-style legend key for a continuous `Size` or `Alpha` aesthetic
  no longer collapses every key to the scale's minimum. `draw_discrete_
  legend_at` only had each break's rendered *label* (a `Value::Str`), not
  its original data value, and `ScaleSizeContinuous`/`ScaleAlphaContinuous`'s
  `map_to_size`/`map_to_alpha` can't parse a label string back to a number,
  so every key silently reported the scale's own fallback. 0.20 (#131) hit
  the same bug for `Linewidth` and fixed it with `Scale::map_fraction_to_
  size(fraction) -> Option<f64>`, taking the break's own `[0, 1]` fraction
  (already computed by `breaks()`) directly instead of round-tripping
  through the label. `ScaleSizeContinuous` now implements that same method,
  and the new `Scale::map_fraction_to_alpha(fraction) -> Option<f64>`
  (default `None`) does the equivalent for `ScaleAlphaContinuous` — alpha
  isn't a "size", so it gets its own trait method rather than reusing
  `map_fraction_to_size`. `draw_discrete_legend_at`'s `Size`/`Alpha` arms
  use the new methods (falling back to the old label round-trip for any
  other scale assigned to those aesthetics). Closes #132.

## [0.21.0] — 2026-10-20

Minor bump: the SVG mark contract. Default output is byte-identical except
for four deliberate changes: `fill-opacity`/`stroke-opacity` omitted at
1.000, `draw_line` splitting into separate `<polyline>`s at non-finite
vertices, every data mark gaining a `gg-mark gg-<kind>` class, and `GeomText`
honouring a theme/geom font family it previously ignored. `Layer` and
`Theme`/`ThemeUpdate` gain new public fields, so struct literals without
`..Default::default()` need them added.

### Added

- SVG data marks carry a stable class: every element a layer draws gets
  `class="gg-mark gg-<kind>"` (`gg-point`, `gg-line`, `gg-band`, `gg-bar`,
  `gg-text`, by primitive) and `data-geom="<geom name>"`, so a host can select
  marks without matching grid lines, axes or legends, which stay unclassed
  (#121). While a mark part is set (e.g. an arrow head), the part is the
  effective kind, so arrow heads carry `gg-mark gg-arrow` rather than their
  parent's `gg-band`/`gg-line`. `data-geom` on reference lines reads `hline`,
  `vline` or `abline`. New `DrawBackend::set_mark_geom` hook (default no-op),
  forwarded by `compose::Measure` as well as the panel backend adapter, so
  `PlotGrid` marks are classed too.
- New default `Geom::keeps_na_as_break` hook (default `false`; `GeomLine`/
  `GeomPath`/`GeomStep` return `true`): a row whose position is non-finite is
  no longer deleted from the built data for these geoms, so `draw_line`'s
  per-vertex split (below) actually has a gap to split on, instead of the
  row being spliced out and its finite neighbours silently joined across it.
- `GGPlot::svg_attr(name, value)`/`svg_attrs(iter)` — and the same API on
  `PlotGrid` — add caller attributes to the root `<svg>`: `data-*`/`aria-*`
  (the same charset-restricted suffix `Aes::data_attr` uses), or the fixed
  names `role`/`id`/`class`/`lang`. Anything else, or a name ggplot-rs
  already writes to the same root, is dropped with a build warning rather
  than silently overwriting or duplicating it.
- `GGPlot::svg_warnings_attr(bool)` (and on `PlotGrid`) emits `data-warnings`
  as a JSON array of the build warnings, `[]` when there are none.
- Per-mark formatted values: new default `Scale::format_value(&Value) ->
  Option<String>`, overridden by `ScaleContinuous` (`with_value_formatter`,
  falling back to the label formatter then the locale, and inverting the
  scale's own transform first — `scale_y_log10`'s `data-fvalue` shows
  `"100"`, not the transformed-space `"2"`), `ScaleDateTime` (`with_date_
  labels`, else the locale date format, else plain ISO — the same order
  the axis break labels already use), and the size/linewidth/colour-
  continuous scales (their own formatter). `GGPlot::svg_formatted_values
  (bool)` (default off) turns on `data-fvalue` (the value aesthetic's
  scale — y for most geoms, fill for hex/tile/bin2d) and `data-fx` (the x
  scale, written only when it differs from the plain `data-x`) on marks.
  New `DrawBackend::wants_formatted_values`/`set_mark_fvalue`/
  `set_mark_fx` hooks (all default no-ops). A vline/hline's `data-x` and
  tooltip show the matching scale's formatted value too (e.g. an ISO date
  on a datetime x) when the flag is on.
- `GGPlot::tooltip(template)`: a `{column}` tooltip template for the most
  recently added layer, replacing its default `<title>`. Each placeholder
  is filled from the *built* (post-stat) layer data — so it can reference
  a stat-computed column, e.g. a hex's `count` — formatted through the
  column's scale when it has one. An absent column renders empty, not
  literally `{column}`; `{{`/`}}` are literal braces.
- Geom text inherits the theme font: `GeomText`/`GeomLabel`/`GeomTextRepel`/
  `GeomLabelRepel`/`GeomBracket`/text annotations resolve their font family
  as the geom's own `fontfamily` (previously a dead field on `GeomText`/
  `GeomLabel`), else `theme.geom.family`, else `theme.text.family`, else
  `"sans-serif"`.
- `#[non_exhaustive] ElementGeom { ink, paper, accent, family }` (ggplot2
  4.0's `element_geom`), plus `Theme.geom`, `ThemeUpdate.geom`,
  `Theme::set_geom`/`with_ink_paper`, and `ThemeConfig`'s `geom.ink`/
  `geom.paper`/`geom.accent`/`geom.family` config keys. A new default
  `Geom::apply_element_geom` (a no-op) is called once per layer before the
  brand/primary colour, and only for a layer *not* configured explicitly
  via `geom_*_with(...)` (the same `explicit_style` gate the brand colour
  already uses — review PR#141 finding 4: a field-*value* comparison
  against the hard-coded default can't tell an explicitly-set colour that
  happens to equal it apart from one nobody touched, so the gate skips the
  call entirely instead). Covered geoms: `GeomPoint`/`GeomLine`/
  `GeomPath`/`GeomText`/`GeomLabel`/`GeomBoxplot` recolour their built-in
  black/`(50, 50, 50)`/white defaults from `ink`/`paper`, and `GeomSmooth`
  from `accent`. The legend key swatch colour is now
  `theme.geom.ink.unwrap_or((50, 50, 50))`. Every other geom (segment,
  curve, reference lines, errorbar/linerange/pointrange, step, rect/tile
  outlines, density/violin/area outlines, crossbar) does not yet honour
  `ElementGeom` — tracked at sipemu/ggplot-rs#142.
- `ScaleColorContinuous`/`ScaleColorGradient2`/`ScaleColorGradientN` gain
  `with_limits(lo, hi)`, fixing the colour domain (not just the legend) to
  a range independent of the trained data — e.g. `[-1, 1]` for a
  correlation matrix. New `Oob` enum (`Censor`, the default: an
  out-of-range value renders in the scale's `na_value`; `Squish`, opt-in
  via `with_oob`: clamped to the nearest limit).
- The same three scales gain `na_value` (default `(127, 127, 127)`,
  ggplot2's `"grey50"`) and a `with_na_value` builder: a genuinely missing
  (`NA`) or non-finite value, and a value censored by `Oob::Censor`, all
  render in `na_value` instead of a bare `None` the geom silently papers
  over with its own unrelated default colour.
- New default `Geom::consumes_tooltips` hook (default `false`; the 13
  geoms that actually read a per-row tooltip — bar/col/histogram/hex/
  line/path/point/polygon/rect/sf/segment/curve/tile — return `true`):
  `GGPlot::tooltip(template)` on a layer whose geom ignores tooltips now
  pushes a build warning instead of silently having no effect.

### Fixed

- `Scale::format_value` on `ScaleContinuous` now inverts the scale's own
  transform (`scale_x_log10`, etc.) before formatting: `data-fvalue` and a
  `{y}` tooltip-template placeholder on a log-scaled `y = 100` showed `"2"`
  (the transformed-space value the built data actually holds) instead of
  `"100"`, while the axis labels correctly showed the inverse already.
- `ScaleDateTime::format_value` now honours `with_date_labels` (falling
  back to the locale date format, then plain ISO) instead of always
  ISO/locale regardless of it — the axis break labels already did. This
  also un-breaks `data-fx` on a datetime x with `with_date_labels` set: it
  was suppressed as "equal to `data-x`" because the un-formatted ISO
  fallback happened to be the only thing compared.
- `PlotGrid` cells each honour their own `svg_formatted_values` flag
  (previously only a top-level `GGPlot::render_svg_native*` call applied
  it; a plot composed into a grid lost `data-fvalue`/`data-fx` even with
  the flag set on that cell).
- `ScaleColorContinuous`/`ScaleColorGradient2`/`ScaleColorGradientN`'s
  `with_limits(lo, hi)` with `lo > hi` (e.g. a typo) no longer panics
  under `Oob::Squish` (`f64::clamp` panics when `min > max`) — every
  value is treated as censored (`na_value`) regardless of the configured
  `Oob`, same as `Oob::Censor` already did, and `Scale::validate` now
  pushes a build warning naming the inverted limits.
- The root/`PlotGrid` `svg_attr` collision warning now distinguishes a
  name ggplot-rs itself writes (`data-plot`, `data-grid`, …) from a
  *second* `svg_attr`/`svg_attrs` call for the same name — the latter
  previously said "collides with an attribute ggplot-rs already writes
  to the root", which wrongly blamed ggplot-rs for the caller's own
  earlier call; it now says "was already set by an earlier svg_attr
  call".

### Changed

- **Missing colour values render grey** on continuous colour/fill scales (`na_value`, default grey50, as in ggplot2), also without limits. Previously NaN rendered black and NA the low end of the gradient.
- `fill-opacity`/`stroke-opacity` is written only when the value isn't
  `1.000` (the SVG default); it was previously written unconditionally on
  every mark.
- `draw_line` splits at a non-finite vertex into separate `<polyline>`
  marks instead of silently dropping the bad vertex and joining its finite
  neighbours across the gap (ggplot2's NA-break semantics) — for
  `geom_line`/`geom_path`/`geom_step` only (see `keeps_na_as_break` above).
  A run shorter than 2 points is dropped, the same as ggplot2 drops a
  one-observation path piece, rather than becoming an invisible 1-point
  polyline mark. `GeomStep` clamps only an *infinite* pixel coordinate to
  the panel edge (`stat_ecdf`'s `±Inf` padding); a `NaN` coordinate is left
  alone so it breaks the line instead of being clamped to an edge it was
  never meant to reach.
- `"geom"`, `"fvalue"` and `"fx"` join `"part"` as reserved `data_attr`
  names (alongside the attributes they already name): a mapping under any
  of these is rejected with a build warning, the same as `"part"` already
  was, so no element can ever carry two `data-geom`/`data-fvalue`/`data-fx`
  attributes.

### Breaking

- `Layer` gains a public `tooltip_template: Option<String>` field.
- `Theme` gains a public `geom: ElementGeom` field; `ThemeUpdate` gains a
  public `geom: Option<ElementGeom>` field. Struct literals for either
  without `..Default::default()` need the new field added.

## [0.20.0] — 2026-10-20

Minor bump: new public API (arrows, linewidth, hex binning in data units,
interactive aesthetics). `LineStyle` gains a `lineend` field, and
`GeomSegment`/`GeomCurve`/`GeomPath`/`GeomLine` gain `arrow`/`lineend` fields,
so struct literals without `..Default::default()` need them added.
`StatBinHex`'s fields and output columns are not back-compat with 0.19.0's.
Default output is otherwise unchanged, except hexbin cell geometry and curve
rendering (a native quadratic Bézier `<path>` instead of a flattened
polyline).

### Added

- `LineEnd { Butt, Round, Square }` and `LineStyle::lineend`; `DrawBackend`
  gains a default `draw_quadratic(p0, ctrl, p1, style)`, overridden by the
  SVG backend to emit a native `<path d="M.. Q..">`.
- `src/geom/arrow.rs`: `Arrow` (grid's `arrow()`: `angle`, `length`, `ends`,
  `kind`, plus the `position`/`length_per_linewidth` extensions) via the
  `arrow()` builder. `GeomSegment`/`GeomCurve`/`GeomPath`/`GeomLine` gain
  `arrow`/`lineend` fields and `.arrow() .lineend() .linewidth()` builders
  (curve also `.curvature() .ncp()`); segment and curve honour per-row
  mapped `linetype` and `alpha` (previously ignored).
- `Aesthetic::Linewidth` ("linewidth") with `Aes::linewidth()` /
  `Aes::after_stat_linewidth()`; `src/scale/linewidth.rs`'s
  `ScaleLinewidthContinuous` (range, limits/squish, transform, breaks,
  labels, locale); `GGPlot::scale_linewidth()`; a legend key drawn as a
  horizontal line of the mapped width. Segment/curve read it per row;
  path/line per point, drawing segment-by-segment when it varies within a
  group.
- `StatBinHex` rewritten onto a fixed pointy-top hex lattice
  (`src/stat/binhex::HexLattice`; cube-rounded point assignment, `locate()`,
  deterministic `binwidth`/`origin` rules) instead of floor-binned
  staggered rectangles anchored at the data minimum. New output columns:
  `count, density, ncount, ndensity, width, height, n, q, r` (`fill`
  stays as `count`, back-compat). `GeomHex` draws each hexagon's vertices
  in data space (from `width`/`height`, falling back to the data's
  resolution) and projects each individually — correct under any coord,
  including `coord_sf`.
- `src/scale/identity.rs`: `ScaleColorIdentity` reads `#RRGGBB`/`#RGB` hex
  strings directly, with no legend; `GGPlot::scale_colour_identity()` /
  `scale_color_identity()` / `scale_fill_identity()`.
- Interactive aesthetics: `Aesthetic::Tooltip` (`Aes::tooltip(col)`, a
  custom `<title>`) and `Aesthetic::DataAttr` (`Aes::data_attr(name, col)`,
  a custom `data-<name>` attribute; `name` must match `[a-z0-9][a-z0-9-]*`).
  `DrawBackend::set_mark_attrs`/`set_mark_part` (the latter also used by
  arrowheads: `data-part="arrow" aria-hidden="true"`, no `<title>`). A user
  attr replaces a built-in of the same name rather than duplicating it.
- `CoordSf::xlim`/`ylim` (zoom, no filtering) and `expand(bool)`, via a new
  default `Scale::set_expand(mult, add)`; `GGPlot::coord_sf_with(CoordSf)`
  and `svg_root_data(bool)` (default `true`; off omits `data-plot`/
  `data-domain`/etc from the root `<svg>`) — for a consumer overlaying a
  rendered fragment on its own basemap.

### Changed

- Hexbin cell geometry: `StatBinHex` no longer floor-bins staggered
  rectangles anchored at the data minimum, so cell positions differ from
  0.19.0 (deterministic, but not reproducible against old output). See
  "Added" above.
- `geom_curve` draws a native quadratic Bézier `<path d="M.. Q..">`
  instead of a flattened `<polyline>`; `ncp` is now ignored by SVG
  (documented) since a single control point fully determines the curve.

### Breaking

- `LineStyle` gains `lineend: LineEnd`; `GeomSegment`, `GeomCurve`,
  `GeomPath`, `GeomLine` gain `arrow: Option<Arrow>` and
  `lineend: LineEnd`; `StatBinHex`'s fields and output columns are not
  back-compat with 0.19.0. Struct literals need `..Default::default()`.
- `Aesthetic` gains `Linewidth`, `Tooltip` and `DataAttr(String)`
  variants — an exhaustive match on `Aesthetic` needs a catch-all or the
  new arms.
- `Aes` gains a private `warnings` field (build warnings from
  `Aes::data_attr`, e.g. an invalid or reserved name). A struct literal
  that names every field explicitly — `Aes { mappings, after_scale }`,
  without `..Default::default()` — no longer compiles (E0451, "field
  `warnings` of struct `Aes` is private"); add `..Default::default()`.
- `ggplot_rs::aes::{resolve_mappings, apply_after_stat}` (public
  re-exports of `aes::mapping`'s functions) gain a `&mut Vec<String>`
  warnings parameter: `resolve_mappings(data, mapping)` →
  `resolve_mappings(data, mapping, warnings)`, same for
  `apply_after_stat`. Needed so a `data_attr`/`tooltip` mapping that
  turns out invalid or reserved at this stage (not just at
  `Aes::data_attr` build time — e.g. one constructed by pushing
  directly into `Aes.mappings`) can still report a warning instead of
  resolving silently.

## [0.19.0] — 2026-10-16

Minor bump: new public API. `Theme` and `ThemeUpdate` gain two fields, so a
struct literal without `..Default::default()` needs them added. Default output
is unchanged, except that a calendar under a plot locale now uses it.

### Added

- `CoordPolar::expand(bool)` (default `true`) and the `Coord::discrete_expand`
  hook: with `expand(false)` bars on a discrete axis are not band-padded, so a
  `theta = "y"` pie with `width = 1` bars is solid at the centre (#117). A
  donut (`inner_radius`) keeps its hole.
- `Theme::calendar_month_border` (`geom_calendar`'s month outlines, default
  grey 1.2) and `Theme::legend_bar_border` (the continuous colourbar frame;
  `None` keeps the `legend_key.color` / dark-grey default), with
  `set_calendar_month_border` / `set_legend_bar_border` /
  `legend_bar_border_line`, `ThemeUpdate` fields and theme-config keys.
  `ElementLine::blank()` hides either (#118).
- `ScaleContinuous::with_localized_labels` (+ `LocalizedLabels`): custom-break
  labels resolved against the scale/plot locale;
  `stat::calendar::month_breaks_localized` and `weekday_labels`.

### Fixed

- `geom_calendar` weekday and month labels follow `GGPlot::locale` (de-DE
  `Mo.`/`Mi.`/`Fr.`, `Jan.`/`Feb.`/`März`; fr-FR likewise), whether the
  locale is set before or after the geom (#119).

## [0.18.0] — 2026-10-15

Minor bump: new public API, no breaking changes. Without a locale every label
is byte-identical to 0.17.1.

### Added — locale-aware number and date labels

- `ggplot_rs::locale::Locale` (re-exported as `ggplot_rs::Locale`, in the
  prelude and in `ggplot_rs::format`): decimal mark, thousands mark, minimum
  grouping digits, currency symbol + placement (`CurrencyPosition`) + spacing,
  percent suffix, short/long month and weekday names, date / month / time
  formats. Presets `Locale::en_us()`, `en_gb()`, `de_de()`, `fr_fr()`;
  `Locale::from_tag("de-DE")` parses BCP-47 / POSIX tags (`de_DE.UTF-8`,
  `en-GB`, …) and falls back to en-US (`try_from_tag` returns `None`
  instead); `FromStr`, `Default` (en-US). Helpers: `localize_number`,
  `parse_number`, `format_number`, `strftime`, `format_datetime`,
  `currency_affixes`, `with_decimal_mark` / `with_big_mark` /
  `with_min_grouping_digits` / `with_currency`.
- Formatters with explicit marks: `label_number_marks`, `label_comma_marks`,
  `label_percent_marks`, `label_currency_marks`; and in a locale's
  conventions: `label_number_locale`, `label_comma_locale`,
  `label_percent_locale`, `label_currency_locale` (de-DE `1.234,50 €`,
  en-US `$1,234.50`, en-GB `£1,234.50`, fr-FR `1 234,50 €`; de-DE / fr-FR
  percent `12,3 %`).
- `format::format_value_locale`, `scale::util::format_numbers_locale` /
  `format_number_locale` (joint break labelling keeps its distinct-labels
  logic, rendered with the locale's marks).
- `GGPlot::locale(Locale)`: plot-wide default for continuous position,
  colour, size and alpha scales (axis ticks and legends, e.g. `1.000` /
  `0,5`), secondary axes, and date/datetime scales (`Okt. 2026` for monthly
  breaks, `15. Okt. 2026` for daily ones, localised `%b %B %a %A`).
  `ScaleContinuous::with_locale` / `ScaleDateTime::with_locale` set it per
  scale (and win over the plot locale); custom `label_*` formatters win over
  both.
- `Scale::set_default_locale` / `Scale::locale` (default methods, no-op /
  `None`) and `ScaleSet::set_default_locale`.
- strftime subset gained `%a %A` (weekday) and `%-d %-m` (unpadded).

## [0.17.1] — 2026-10-09

### Fixed

- Continuous axis and legend breaks are labelled jointly
  (`scale::util::format_numbers`): on narrow ranges neighbouring breaks get
  enough decimals to stay distinct (`0.99, 0.992, 0.994` instead of
  `0.99, 0.99, 0.99`), and a non-zero break never prints as `0` (log ticks at
  `0.001`). Labels are unchanged wherever two decimals already suffice;
  custom `label_*` formatters are unaffected.

## [0.17.0] — 2026-10-08

### Breaking changes

- `Aesthetic` gained the variants `Xintercept`, `Yintercept`, `Slope` and
  `Intercept` (exhaustive matches on `Aesthetic` need new arms).
- `geom_hline` / `geom_vline` (constant and mapped) now train the y / x
  position scale on their intercepts, as in ggplot2: a reference line outside
  the data range widens the axis instead of being drawn off-panel.
- `geom_abline` is now drawn in data space (`y = intercept + slope · x` in the
  scales' — possibly transformed — units) and clipped to the panel; it used to
  interpret slope/intercept in normalized panel units.
- `StepDirection` gained the variant `Mid`.
- `PositionDodge` is now a struct with `width: Option<f64>` and `reverse:
  bool` (a same-named `const` keeps `.position(PositionDodge)` compiling);
  `PositionDodge2` gained the public fields `width` and `reverse` (construct
  it with `new` / `default` and the `with_*` builders). Both now dodge the
  groups present at each x (ggplot2's default `preserve = "total"`) instead
  of reserving a slot for every group, and key groups on group + fill +
  colour together.
- Reference-line geoms no longer inherit the plot-level mapping (ggplot2's
  `inherit.aes = FALSE`): a plot-level `color` no longer leaks into
  `geom_hline`. New `Geom::inherit_aes()` (default `true`) controls this.

- `StatQQ` / `StatQQLine` use the exact normal quantile in every build (they
  used the Abramowitz–Stegun approximation, |error| ≈ 4.5e-4, without
  `regression`), and `StatQQLine` needs 2 sample values instead of 4.

### Fixed

- `geom_errorbar`, `geom_linerange` and `geom_pointrange` collapsed to a
  point under `coord_flip` (only the x pixel of one end was used); every end
  is now transformed. `geom_errorbar` / `geom_linerange` honour a mapped
  `color`; an error bar is one polyline (cap–bar–cap) per row.

### Fixed — dodged bars

- `geom_col` / `geom_bar` with `position_dodge` on a **discrete** x now shift
  and narrow each bar by the stored dodge offset and the number of groups at
  that x (new `position::DODGE_N_COL`), so grouped bars sit side by side
  instead of on top of each other.

### Added — statistical geoms (reference lines, QQ, Cook's contours, KM/ECDF, dodge)

- **Data-mapped reference lines (U1).** `Aes::{xintercept, yintercept, slope,
  intercept}` plus `GGPlot::{geom_hline_aes, geom_vline_aes, geom_abline_aes}`
  (and `*_aes_with(geom, aes)` for custom default styling) draw one line per
  row of the layer data, per facet panel, coloured / linetyped by any mapped
  `color` / `linetype` / `alpha`. Constant lines are now one-row layers, so
  they appear in every facet panel. Lines carry `data-series` / `data-value`
  (the intercept) and a tooltip on the native SVG path. Non-finite intercepts
  are dropped with a warning; a numeric `xintercept` on a discrete axis sits
  between categories (1-based, as in ggplot2) without becoming a level, and a
  reference line added before a discrete layer no longer forces a continuous
  scale. `GeomHline::mapped()`, `GeomVline::mapped()`, `GeomAbline::mapped()`.
- **QQ plots against several distributions, with confidence bands (U2).**
  `QQDistribution::{Normal { mean, sd }, StudentT { df }, Exponential { rate },
  HalfNormal { sd }}` (ggplot2's `distribution` + `dparams`; constructors
  `normal()`, `t(df)`, `exponential()`, `half_normal()`), `StatQQDist`,
  `StatQQLineDist` (line through the `line_p` quartiles over the range of the
  theoretical quantiles; also emits `slope`/`intercept`) and `StatQQBand`
  (qqplotr's `stat_qq_band`: `QQBandType::Pointwise` normal-theory envelope or
  `QQBandType::Ks` DKW band, `level`), drawn by the new `GeomQQBand`. Builder
  methods `GGPlot::{stat_qq(dist), stat_qq_line(dist), stat_qq_band(band),
  geom_qq_band(), geom_qq_band_with(geom, band)}`. Validated against
  ggplot2 4.0 and qqplotr 0.0.7 to 1e-9 (`tests/qq_dist_r.rs`,
  `validation/generate_diagnostics.R`). QQ points now carry `data-x` /
  `data-value` / tooltips; `geom_qq_line` draws one line per group.
- **Cook's-distance contours (U4).** `GGPlot::stat_cooks_contour(p, levels)`
  / `geom_cooks_contour_with(GeomCooksContour)` draw R's `plot.lm(which = 5)`
  contours `±√(level · p · (1 − h) / h)` on a residuals-vs-leverage panel:
  dashed, clipped to the panel (geometrically — the native SVG has no clip
  paths), labelled with the level, in every facet panel, optionally limited
  to `with_h_range(lo, hi)`; they train no scale. Helpers
  `geom::cooks::{cooks_contour_y, cooks_distance}`. Checked against R
  (`tests/cooks_contour_r.rs`).
- **Kaplan–Meier / ECDF / horizontal-CI building blocks (U7).**
  `GeomStepribbon` (`geom_stepribbon()`, `geom_stepribbon_with(geom)`; step
  edges with `StepDirection::{Hv, Vh, Mid}`, ±Inf x to the panel edge) for KM
  confidence bands; censor marks `geom_censor_marks(censor_col)` /
  `GeomCensorMarks` + `StatCensored` (`+` glyphs at rows whose column is
  `> 0` / `true`); `stat_ecdf()` and `stat_ecdf_band(level)` / `StatEcdfBand`
  (simultaneous DKW band `F̂ ± √(ln(2/(1−level))/(2n))`, validated against R);
  `geom_errorbarh()` / `GeomErrorbarh` (`xmin`/`xmax`/`y`, one hoverable
  polyline per interval with `data-value="xmin xmax"`).
- `geom_step` draws one step line per group (colour / group / linetype) with
  mapped linetype and `data-series`, and gains `StepDirection::Mid`
  (ggplot2's `"mid"`). `stat_ecdf` ignores non-finite input values.
- **Multi-model coefficient forests (U8).** `position_dodge(width)` /
  `PositionDodge::new(width).with_reverse(bool)` and
  `PositionDodge2::new(padding).with_width(w).with_reverse(bool)` now work on
  a *discrete* x axis (terms): the per-row offset is stored in
  `.x_dodge_offset` (`position::DODGE_OFFSET_COL`) and applied by
  `geom_point`, `geom_pointrange`, `geom_errorbar`, `geom_linerange`,
  `geom_ribbon` and censor marks, so several models' intervals per term sit
  side by side — also under `coord_flip`. Interval geoms map `color` per row
  and carry `data-x` (term) / `data-series` (model) / `data-value` (estimate,
  else `"ymin ymax"`) plus a tooltip.
- `examples/diagnostics.rs` (runs with `--no-default-features`): a
  regression-diagnostic gallery on the native SVG path — QQ with pointwise +
  KS bands, QQ against t(5), residuals vs leverage with Cook's contours,
  Kaplan–Meier (`survival::lung`) with CI step ribbon and censor marks, ECDF
  with DKW band, dodged three-model coefficient forest, per-facet mapped ACF
  bounds — written to `assets/gallery/diagnostics/*.svg`.
- `ggplot_rs::stat::distribution`: dependency-free `qnorm` (AS 241), `dnorm`,
  `pt` / `qt` / `dt` (any `p`, any `df`) and `ln_gamma`, available in every
  feature configuration (the existing `stat::dist::qt` still returns the
  normal 0.975 quantile without `regression`).
- `geom_ribbon` draws one band per group (group / colour / fill) with a
  mapped `fill`, instead of a single polygon through every row.
- The native SVG backend draws every `PointShape` (square, triangle, diamond,
  `+`, `×`) instead of falling back to circles; `+`/`×` are one stroked
  `<path>` per point, so they keep their `data-*` hover attributes.

### Added — composition, label repel, GLM smooths, table brackets

- `geom_smooth(method = "glm")` families: `SmoothFamily::Binomial(link)`
  (`SmoothBinomialLink::{Logit, Probit, Cloglog}`, shorthand
  `SmoothFamily::binomial()`), `SmoothFamily::Gamma(link)`
  (`SmoothGammaLink::{Inverse, Log}`, shorthand `SmoothFamily::gamma()`) and
  `SmoothFamily::NegativeBinomial` (log link, θ by ML as `MASS::glm.nb`), plus
  the `GeomSmooth::glm(family)` builder. Bands follow ggplot2's
  `predictdf.glm`: `linkinv(η ± qnorm(0.975)·se(η))`, so they stay inside the
  response range. Validated against R `glm()`/`predict()` in
  `tests/glm_smooth_r.rs`.
- `GGPlot::geom_bracket_table(table, BracketTable)` — significance brackets
  from a **precomputed** test table (ggpubr `stat_pvalue_manual`), no test is
  recomputed. Reads the anofox `test` contract (`group1`, `group2`, `p_adj`
  falling back to `p_value` per row, `test_id`) plus optional `y_position` /
  `label` columns; label templates (`"p = {p_adj}"`, `{p}`, `{p.signif}` /
  `{stars}` with configurable cutpoints, any `{column}`), `hide_ns`, automatic
  stacking above the data for rows without `y_position`. Rows with missing or
  unknown groups are dropped with a build warning. Pure grammar: available
  without the `ggpubr` feature.
- Brackets (`geom_bracket*`) now carry host hover metadata: a `<title>`
  tooltip, `data-x="g1 vs g2"`, `data-series` (`test_id` for table brackets)
  and `data-value` (the p-value).
- `geom_text_repel` / `geom_label_repel` (`GeomTextRepel`, `GeomLabelRepel`,
  `RepelParams`, `RepelDirection`; ggrepel): deterministic, seeded label
  layout that avoids other labels and the labelled points and stays inside the
  panel; `nudge`, `box_padding`, `point_padding`, `force`/`force_pull`,
  `direction`, `max_iter` + `max_time` bounds, `max_overlaps` (dropped labels
  are reported as a warning), connecting segments beyond
  `min_segment_length`. Sweep-pruned collision checks; above `max_labels`
  (default 500) the force layout is skipped with a warning.
- `DrawBackend::warn` (default no-op): geoms can report draw-time warnings;
  the native SVG backend collects them (`SvgBackend::take_warnings`) and
  `render_svg_native_with_warnings` returns them after the build warnings.
- `PlotGrid` (`ggplot_rs::compose`, in the prelude): patchwork-style
  composition on the native SVG path — `PlotGrid::new().add(p).ncol(2)`, or
  `a | b` (side by side) and `a / b` (stacked; chains flatten, mixed operators
  nest), `add_spacer`, `nrow`/`byrow`, relative `widths`/`heights`, `spacing`,
  shared `title`/`subtitle`/`caption`, panel tags (`TagLevels::{Lower, Upper,
  Numeric, LowerRoman, UpperRoman, Custom}` + `tag_affixes`), and
  `collect_legends(true)` (each distinct legend drawn once to the right or
  bottom, identical legends de-duplicated). Renders one SVG
  (`render_svg_native[_with_size|_with_warnings|_at]`); every sub-plot is a
  nested `<svg>` that keeps its host attributes and adds
  `data-panel="<tag or index>"`; the root carries `data-grid="<rows> <cols>"`.
  Sub-plot warnings are returned prefixed `panel <tag>: `.
- `examples/regression_diagnostics.rs`: a plotters-free 2×2 diagnostics grid
  with repelled labels, table-driven brackets and a logistic GLM smooth.

### Changed

- `SmoothFamily::Poisson` bands are now formed on the link scale and mapped
  through `exp` (as R/ggplot2), instead of a response-scale interval.
- `ggpubr::ggarrange` now delegates to `PlotGrid`: cells are nested SVG
  fragments without a per-cell `xmlns`, positioned as `x="300.00"`, and carry
  `data-panel="<index>"`.
- The `regression` feature now requires `anofox-regression` ^0.5.17 — the
  version the anofox-statistics DuckDB extension uses — and GLM smooths use
  that extension's IRLS settings (tolerance 1e-8, ≤ 100 iterations), so SQL
  fits and plotted smooths agree.

## [0.16.0] — 2026-10-08

### Breaking changes

- **The library is now an `rlib` only** (was `["cdylib", "rlib"]`), which
  unblocks static / emscripten side-module consumers. The wasm-bindgen browser
  exports (`ggplot_rs::wasm::*`: `render_geo`, `render_plot`, `render_bar`,
  `render_hist`, `geo_bounds`, `scatter_frame`, `render_scatter_rgba`,
  `render_scatter_xy`) moved to the new, unpublished workspace crate
  `crates/ggplot-rs-wasm` (`cdylib`). Build the browser bundle with
  `wasm-pack build crates/ggplot-rs-wasm --target web --out-dir ../../web/pkg --out-name ggplot_rs`.
  The library no longer depends on `wasm-bindgen`.
- The `wasm` feature is now a **deprecated alias for `sf`** (kept so
  `features = ["wasm"]` dependents keep compiling); it no longer adds
  `wasm-bindgen`/`serde_json`.
- New default-on feature **`plotters`** (implies the new `png` feature) gates
  the plotters/`image`-backed APIs: `GGPlot::{render_svg, render_svg_with_size,
  render_png, render_png_with_size, save, save_with_size, ggsave}`,
  `ggpubr::{ggarrange_png, ggarrange_save_png}` and
  `render::plotters_backend`. Crates using `default-features = false` that call
  these must add `features = ["plotters"]`; the self-contained
  `render_svg_native*` path is always available. `canvas` now implies `png`.
  `cli` implies `plotters`.
- `DataFrame::add_column` no longer panics on a length mismatch: it pads the
  shorter side with `Value::Na` and records an issue
  (`DataFrame::issues()` / `validate()`); `GGPlot::try_build` and every
  `render_*`/`save*` then return `GGError::ValidationError`. This is reachable
  from `GGPlot::new(Vec<(String, Vec<Value>)>)` with ragged columns, which used
  to panic. New `DataFrame::try_add_column` rejects the column instead.
- `DataFrame::group_by` / `unique_values` keep `Value::Na` distinct from the
  literal string `"NA"` (they used to merge into one group).
- `label_number(…, prefix, …)` is now sign-aware: `-5` with prefix `"€"`
  renders `"-€5"` (was `"€-5"`), and a value rounding to zero drops its sign.
- `PositionJitter` gained a `seed: u64` field, and `width`/`height` are now
  `Option<f64>` (`None` = 0.4 × the data's resolution, as in ggplot2). Build it
  with `PositionJitter::new(w, h)`, `PositionJitter::default()`, `.with_width()`,
  `.with_height()` and `.with_seed()` instead of a struct literal.
  `GeomJitter::{width, height}` are likewise `Option<f64>` (default `None`).
- Jitter and `mean_cl_boot` now draw from an internal SplitMix64 stream instead
  of `rand::StdRng`: output is still deterministic, but the exact jittered
  positions / bootstrap intervals differ from 0.15.

### Added

- `ggplot_rs::format::format_value(&Value) -> String` — the public tooltip /
  data-label value formatter (previously the crate-private `geom::tip_value`);
  `ggplot_rs::format` also re-exports the `label_*` formatters.
- `label_currency(prefix, suffix, big_mark, accuracy)` — sign-aware currency
  labels with a configurable thousands mark (`"-€1.234.567"`). `label_dollar`
  is now implemented on top of it (output unchanged).
- `tests/native_svg.rs`: smoke coverage of the plotters-free SVG path across
  geoms/coords/facets/themes, run in every feature configuration.

- `Value::group_key() -> GroupKey` (injective w.r.t. missingness, borrows
  strings) and `Value::key_str() -> Cow<str>` (allocation-free form of
  `to_group_key`).
- `DataFrame::try_add_column`, `DataFrame::issues`, `DataFrame::validate`.

### Changed

- Minimal dependency tree (`default-features = false, features = ["sf"]`):
  38 crates → 3 (`indexmap`, `equivalent`, `hashbrown`). No plotters, image,
  ab_glyph, chrono, rand or getrandom.
- Removed the `rand` dependency (and the `wasm32`-only `getrandom` dependency)
  from the library; randomness comes from a small internal seeded PRNG.
- Default jitter amount is resolution-based (ggplot2's `0.4 * resolution(x)`):
  integer-spaced data keeps ±0.4, finely spaced continuous data is no longer
  smeared by a fixed ±0.4 data units.

### Performance

- `stat_density` switches to a binned estimator (linear binning onto a grid
  with spacing ≤ bw/50 + a kernel table truncated at ±8 bw) for groups with
  ≥ 2048 values: O(n + 512·window) instead of O(n·512) kernel evaluations,
  within ~2e-5 of the exact estimate relative to the peak. Smaller groups use
  the exact sum, unchanged.
- Layers without their own data borrow the plot data instead of cloning the
  whole frame per layer.
- Discrete scale training/lookups, `stat_count`, `position_stack` and
  `position_fill` use borrowed keys and hash lookups instead of allocating a
  `String` per value and scanning the distinct x values linearly per row
  (O(n·k) → O(n)).
- `DataFrame::group_by` builds borrowed keys (no per-row `String` clones).

### Fixed

- Empty manual scales (`scale_color_manual(vec![])`, `scale_fill_manual`,
  `scale_shape_manual`, `scale_linetype_manual`) no longer panic with a
  division by zero; the geom falls back to its default colour/shape/linetype.
- `format_epoch_secs` is total over `i64` (it overflowed for `i64::MIN`).
- `stat_binhex` / `geom_hex` output order is deterministic (row-major) instead
  of following `HashMap` iteration order, so identical data renders identical
  SVG.

- `label_number` / `label_currency` with an `accuracy` that keeps decimals no
  longer inserts thousands marks into the fractional part
  (`1234.5` @ 0.01 → `"1,234.50"`, was `"1,23,4.50"`-style garbage).

### Rendering robustness

- An empty plot (zero-row data, no columns, or no layers) renders an empty
  panel with its title/axes instead of failing with
  `ValidationError("geom_… requires aesthetic …")`.
- A layer whose stat produces no rows (a density/violin of one value, loess on
  two points, …) draws nothing; the other layers still render.
- Rows with non-finite position values (`NaN`, `±Inf`, or a transform that is
  undefined such as `log10(0)`) are dropped before stats, like ggplot2's
  "Removed n rows containing non-finite values". Stat output is filtered the
  same way.
- Build warnings are exposed: `BuiltPlot::warnings()` and
  `GGPlot::render_svg_native_with_warnings(w, h) -> (String, Vec<String>)`.
- The native SVG backend never writes `NaN`/`inf` into an attribute: marks with
  non-finite geometry are dropped and non-finite style values fall back to
  defaults.
- Density/violin bandwidth follows R's `bw.nrd0`, including its fallbacks for
  zero-spread samples (previously a constant sample produced NaN).
- A position scale's transform now applies to its whole column family
  (`ymin`/`ymax`/`yend`, …), not only `x`/`y`.
- Faceted panels keep per-mark hover metadata (tooltips / `data-x`).

### SVG host integration

- Hoverable marks carry `data-series` (colour/fill/group level) and
  `data-value` (the raw measured value; for stacked/filled bars the segment's
  own value) next to `data-x`. Stacked-bar tooltips now report the segment
  value rather than the cumulative stack top.
- The root `<svg>` carries the trained, expanded position domains:
  `data-domain="x0 x1 y0 y1"` (both axes continuous), `data-xdomain`,
  `data-ydomain`, `data-xlevels`/`data-ylevels` (JSON arrays for discrete axes)
  and `data-flip="true"` under `coord_flip`.
- `GGPlot::render_svg_native_at(x, y, w, h)` renders a nested, positioned
  `<svg x y width height viewBox>` fragment without `xmlns`, for composing
  dashboards without string surgery (`SvgBackend::finish_fragment`).

### Rect bounds

- `-Inf`/`Inf` (`f64::NEG_INFINITY`/`f64::INFINITY`) in `geom_rect` /
  `annotate("rect")` `xmin`/`xmax`/`ymin`/`ymax` extend the rect to the panel
  edge (ggplot2) and do not train scales — also on discrete axes. New
  `Geom::allows_infinite()` hook; other geoms still drop infinite positions.

### Theme / colour

- Theme presets (`theme_minimal()`, `theme_void()`, …) and `theme(t)` keep a
  previously set `primary_color` (unless the new theme sets its own), so call
  order no longer matters.
- `primary_color` no longer overrides colours of geoms added with
  `geom_*_with(...)` (`Layer::explicit_style`); default-configured geoms still
  take the brand colour.
- `ScaleColorDiscrete::sorted()`, `GGPlot::scale_fill_discrete_sorted()` and
  `scale_color_discrete_sorted()`: discrete colour levels in sorted order, so a
  level keeps its palette colour across charts regardless of data order.

### Tiles

- `geom_tile` on a continuous axis is sized by the data resolution (smallest
  gap between distinct values, like ggplot2's `resolution()`); `width`/`height`
  are now multiples of it (default 1 = abutting tiles). The tile extents
  (`xmin`/`xmax`/`ymin`/`ymax`) train the scales, so edge tiles no longer spill
  past the axes and no `with_expand(…)` workaround is needed. New
  `Geom::setup_data()` hook (ggplot2's `setup_data`). Free facet scales also
  train on extent columns.

### New chart types

- `geom_candlestick()` / `geom_ohlc()` (+ `_with`): financial charts from the
  new `open`/`high`/`low`/`close` aesthetics (`Aes::open()` …,
  `Aesthetic::{Open, High, Low, Close}`); configurable `up`/`down` colours;
  the y scale trains on high–low; hover carries `data-value` = close and
  `data-series` = `up`/`down` (or the mapped group).
- `coord_radar()` / `CoordRadar`: radar/spider charts — discrete x → spokes
  (first level at 12 o'clock), y → radius from 0, straight segments, rings at
  the y breaks, spokes and outside labels. `geom_polygon` now groups by
  `group`, else `color`/`fill` (no longer requires `group`), strokes with the
  mapped colour, and orders radar vertices by spoke; `geom_line` closes its
  path under radar and orders points by mapped x position. New
  `Coord::is_radar()` / `Coord::train_scales()` hooks.
- Gauges: `CoordPolar::with_span(start, end)` sweeps the theta range over a
  partial arc (e.g. `-PI/2..PI/2` for a half donut) and fits the arc to the
  panel; `geom_rect` draws annulus sectors under polar coords, so a gauge is
  `geom_rect` bands + a `geom_segment` needle.
- Calendar heatmaps: `geom_calendar()` / `geom_calendar_with(tile, monday_first)`
  and `StatCalendar` lay a date `x` (`DateTime`, epoch seconds or
  `YYYY-MM-DD`) out as week × weekday tiles with month labels, Mon/Wed/Fri
  labels, month boundary outlines and square cells; spans over
  `MAX_CALENDAR_YEARS` (50) are clipped to the most recent years with a
  warning. Tiles' hover key (`data-x`, tooltip) is the ISO date.
- `geom_step` accepts `±Inf` (stat_ecdf's padding reaches the panel edges
  again).
- Gallery: `candlestick`, `radar`, `gauge`, `calendar` examples.
