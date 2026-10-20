# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
