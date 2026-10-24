pub mod expr;
pub mod mapping;

pub use mapping::{apply_after_stat, resolve_mappings};

/// All supported aesthetic channels.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Aesthetic {
    X,
    Y,
    Color,
    Fill,
    Size,
    Shape,
    Alpha,
    Linetype,
    Group,
    Ymin,
    Ymax,
    Xmin,
    Xmax,
    Label,
    Weight,
    Xend,
    Yend,
    Angle,
    Radius,
    /// Stroke width for segment/curve/path/line (R's `linewidth`).
    Linewidth,
    /// A custom `<title>` hover tooltip, replacing the geom's own. Routed
    /// through [`crate::aes::TOOLTIP_INTERNAL_NAME`] in `resolve_mappings`,
    /// the same way `DataAttr` is, so an unrelated raw column literally
    /// named `"tooltip"` can never be mistaken for a mapped one.
    Tooltip,
    /// A custom `data-<name>` attribute on the mark. The `String` is the
    /// full attribute name (`"data-<name>"`). [`Aes::data_attr`] validates
    /// `name` against `[a-z0-9][a-z0-9-]*` (excluding reserved names) before
    /// constructing this; a `DataAttr` built directly — bypassing that
    /// builder — is re-validated in `resolve_mappings`
    /// (`aes::validated_data_attr_internal_name`), the single authoritative
    /// gate regardless of how the mapping was created.
    DataAttr(String),
    /// CSS class tokens on the mark (#221, [`Aes::class`]): a
    /// space-separated list, each token matching `[A-Za-z][A-Za-z0-9_-]*`
    /// (invalid tokens are dropped with a build warning). Routed through
    /// [`CLASS_INTERNAL_NAME`] the same way `Tooltip` is. No scale, no
    /// legend.
    Class,
    /// A caller-named group wrapper (#221, [`Aes::group_wrap`]): every mark
    /// whose row has the same value — across all layers that map the same
    /// name — is drawn inside one `<g data-<name>="<value>">`. The `String`
    /// is the full attribute name (`"data-<name>"`), validated like
    /// [`DataAttr`](Self::DataAttr)'s. No scale, no legend.
    GroupWrap(String),
    /// Financial OHLC aesthetics (`geom_candlestick` / `geom_ohlc`); they
    /// share the y position scale.
    Open,
    High,
    Low,
    Close,
    /// Reference-line aesthetics (`geom_hline` / `geom_vline` /
    /// `geom_abline`). `xintercept`/`yintercept` share (and train) the x / y
    /// position scale, as in ggplot2; `slope`/`intercept` get no scale.
    Xintercept,
    Yintercept,
    Slope,
    Intercept,
    /// Boxplot `stat="identity"` aesthetics (B3-5; ggplot2 extension, D4):
    /// the box's lower/middle/upper hinges. Live on the y scale (D-style,
    /// like `ymin`/`ymax`) — [`crate::scale::ScaleSet::ensure_scale`] gives
    /// them no scale of their own.
    Lower,
    Middle,
    Upper,
    /// A comma-separated list of outlier values, optionally bracketed
    /// (`"7, 9"` or `"[7, 9]"`) — ggplot2 extension, D4. Unparsable parts
    /// are skipped with one warning. Lives on the y scale, like `Lower`/
    /// `Middle`/`Upper`.
    Outliers,
}

impl Aesthetic {
    /// The canonical column name used in the working DataFrame after aes evaluation.
    pub fn col_name(&self) -> &str {
        match self {
            Aesthetic::X => "x",
            Aesthetic::Y => "y",
            Aesthetic::Color => "color",
            Aesthetic::Fill => "fill",
            Aesthetic::Size => "size",
            Aesthetic::Shape => "shape",
            Aesthetic::Alpha => "alpha",
            Aesthetic::Linetype => "linetype",
            Aesthetic::Group => "group",
            Aesthetic::Ymin => "ymin",
            Aesthetic::Ymax => "ymax",
            Aesthetic::Xmin => "xmin",
            Aesthetic::Xmax => "xmax",
            Aesthetic::Label => "label",
            Aesthetic::Weight => "weight",
            Aesthetic::Xend => "xend",
            Aesthetic::Yend => "yend",
            Aesthetic::Angle => "angle",
            Aesthetic::Radius => "radius",
            Aesthetic::Linewidth => "linewidth",
            Aesthetic::Tooltip => "tooltip",
            Aesthetic::DataAttr(name) => name,
            Aesthetic::Class => "class",
            Aesthetic::GroupWrap(name) => name,
            Aesthetic::Open => "open",
            Aesthetic::High => "high",
            Aesthetic::Low => "low",
            Aesthetic::Close => "close",
            Aesthetic::Xintercept => "xintercept",
            Aesthetic::Yintercept => "yintercept",
            Aesthetic::Slope => "slope",
            Aesthetic::Intercept => "intercept",
            Aesthetic::Lower => "lower",
            Aesthetic::Middle => "middle",
            Aesthetic::Upper => "upper",
            Aesthetic::Outliers => "outliers",
        }
    }
}

/// When an aesthetic mapping should be resolved.
#[derive(Clone, Debug, PartialEq)]
pub enum MappingStage {
    /// Resolve before stat computation (default — maps from raw data columns).
    BeforeStat,
    /// Resolve after stat computation (maps from stat-computed columns like `density`, `count`).
    AfterStat,
}

/// Maps a source column to an aesthetic channel.
#[derive(Clone, Debug)]
pub struct AesMapping {
    pub column: String,
    pub aesthetic: Aesthetic,
    pub stage: MappingStage,
}

/// A post-scale aesthetic derivation (R's `after_scale`): the `target` color
/// aesthetic is set to the `source` aesthetic's *mapped* color, adjusted in
/// lightness (`+` toward white, `-` toward black, in `-1.0..=1.0`).
#[derive(Clone, Debug)]
pub struct AfterScaleSpec {
    pub target: Aesthetic,
    pub source: Aesthetic,
    pub lightness: f64,
}

/// Builder for aesthetic mappings.
#[derive(Clone, Debug, Default)]
pub struct Aes {
    pub mappings: Vec<AesMapping>,
    /// Post-scale color derivations (`after_scale`).
    pub after_scale: Vec<AfterScaleSpec>,
    /// Non-fatal build warnings (e.g. an invalid [`data_attr`](Self::data_attr)
    /// name), merged into [`GGPlot`](crate::plot::GGPlot)'s warnings by
    /// `aes()`/`layer_aes()`.
    pub(crate) warnings: Vec<String>,
}

impl Aes {
    pub fn new() -> Self {
        Self::default()
    }

    fn push(mut self, col: &str, aesthetic: Aesthetic) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic,
            stage: MappingStage::BeforeStat,
        });
        self
    }

    fn push_after_stat(mut self, col: &str, aesthetic: Aesthetic) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic,
            stage: MappingStage::AfterStat,
        });
        self
    }

    // ─── after_scale() — post-scale color derivation ────────────

    /// Set `fill` to the mapped `color` with adjusted lightness (R's
    /// `aes(fill = after_scale(...))`). `lightness` in `-1.0..=1.0`: positive
    /// lightens toward white, negative darkens toward black.
    pub fn after_scale_fill_from_color(mut self, lightness: f64) -> Self {
        self.after_scale.push(AfterScaleSpec {
            target: Aesthetic::Fill,
            source: Aesthetic::Color,
            lightness,
        });
        self
    }

    /// Set `color` to the mapped `fill` with adjusted lightness — useful for a
    /// darker border around a filled shape (`color = after_scale(darken(fill))`).
    pub fn after_scale_color_from_fill(mut self, lightness: f64) -> Self {
        self.after_scale.push(AfterScaleSpec {
            target: Aesthetic::Color,
            source: Aesthetic::Fill,
            lightness,
        });
        self
    }

    /// `stage(start, after_stat)`: map `aesthetic` from `start` before the stat
    /// and re-map it from `after_stat` afterwards (R's `stage()`).
    pub fn stage(mut self, aesthetic: Aesthetic, start: &str, after_stat: &str) -> Self {
        self.mappings.push(AesMapping {
            column: start.to_string(),
            aesthetic: aesthetic.clone(),
            stage: MappingStage::BeforeStat,
        });
        self.mappings.push(AesMapping {
            column: after_stat.to_string(),
            aesthetic,
            stage: MappingStage::AfterStat,
        });
        self
    }

    pub fn x(self, col: &str) -> Self {
        self.push(col, Aesthetic::X)
    }
    pub fn y(self, col: &str) -> Self {
        self.push(col, Aesthetic::Y)
    }
    pub fn color(self, col: &str) -> Self {
        self.push(col, Aesthetic::Color)
    }
    pub fn fill(self, col: &str) -> Self {
        self.push(col, Aesthetic::Fill)
    }
    pub fn size(self, col: &str) -> Self {
        self.push(col, Aesthetic::Size)
    }
    pub fn shape(self, col: &str) -> Self {
        self.push(col, Aesthetic::Shape)
    }
    pub fn alpha(self, col: &str) -> Self {
        self.push(col, Aesthetic::Alpha)
    }
    pub fn group(self, col: &str) -> Self {
        self.push(col, Aesthetic::Group)
    }
    pub fn ymin(self, col: &str) -> Self {
        self.push(col, Aesthetic::Ymin)
    }
    pub fn ymax(self, col: &str) -> Self {
        self.push(col, Aesthetic::Ymax)
    }
    pub fn label(self, col: &str) -> Self {
        self.push(col, Aesthetic::Label)
    }
    pub fn weight(self, col: &str) -> Self {
        self.push(col, Aesthetic::Weight)
    }
    pub fn xend(self, col: &str) -> Self {
        self.push(col, Aesthetic::Xend)
    }
    pub fn yend(self, col: &str) -> Self {
        self.push(col, Aesthetic::Yend)
    }
    pub fn xmin(self, col: &str) -> Self {
        self.push(col, Aesthetic::Xmin)
    }
    pub fn xmax(self, col: &str) -> Self {
        self.push(col, Aesthetic::Xmax)
    }
    pub fn angle(self, col: &str) -> Self {
        self.push(col, Aesthetic::Angle)
    }
    pub fn radius(self, col: &str) -> Self {
        self.push(col, Aesthetic::Radius)
    }
    pub fn linetype(self, col: &str) -> Self {
        self.push(col, Aesthetic::Linetype)
    }
    /// Stroke width for segment/curve/path/line (R's `linewidth`).
    pub fn linewidth(self, col: &str) -> Self {
        self.push(col, Aesthetic::Linewidth)
    }
    /// Boxplot `stat="identity"` lower hinge (B3-5).
    pub fn lower(self, col: &str) -> Self {
        self.push(col, Aesthetic::Lower)
    }
    /// Boxplot `stat="identity"` middle (median) line (B3-5).
    pub fn middle(self, col: &str) -> Self {
        self.push(col, Aesthetic::Middle)
    }
    /// Boxplot `stat="identity"` upper hinge (B3-5).
    pub fn upper(self, col: &str) -> Self {
        self.push(col, Aesthetic::Upper)
    }
    /// Boxplot `stat="identity"` outlier values, as a comma list with
    /// optional brackets (`"7, 9"` or `"[7, 9]"`) (B3-5).
    pub fn outliers(self, col: &str) -> Self {
        self.push(col, Aesthetic::Outliers)
    }

    /// A custom `<title>` hover tooltip from `col`, replacing the geom's
    /// own. `ensure_scale` skips it (no scale, no legend): computing stats
    /// drop the column (identity-stat layers keep it), as `linetype`/`alpha`
    /// do.
    pub fn tooltip(self, col: &str) -> Self {
        self.push(col, Aesthetic::Tooltip)
    }

    /// A custom `data-<name>` attribute on the mark, from `col`. `name` must
    /// match `[a-z0-9][a-z0-9-]*` and not be a name ggplot-rs itself writes
    /// via another mechanism (currently just `"part"` — see
    /// `DrawBackend::set_mark_part`); an invalid or reserved name is skipped
    /// with a build warning (surfaced via
    /// [`GGPlot::aes`](crate::plot::GGPlot::aes) / `layer_aes`). A user attr
    /// replaces a built-in of the same name (`data-x`, `data-series`,
    /// `data-value`) — never duplicated.
    pub fn data_attr(mut self, name: &str, col: &str) -> Self {
        if !is_valid_data_attr_name(name) {
            let reason = if RESERVED_DATA_ATTR_NAMES.contains(&name) {
                "reserved (written by ggplot-rs itself)"
            } else {
                "invalid (must match [a-z0-9][a-z0-9-]*)"
            };
            self.warnings
                .push(format!("aes: data_attr name {name:?} is {reason}; skipped"));
            return self;
        }
        self.push(col, Aesthetic::DataAttr(format!("data-{name}")))
    }

    /// CSS class tokens on each mark, from `col` (#221): the row's value is
    /// split on whitespace and every token must match
    /// `[A-Za-z][A-Za-z0-9_-]*`; an invalid token is dropped (with one build
    /// warning per layer), the rest are kept. The tokens are appended to the
    /// mark's own `class="gg-mark gg-<kind>"` — there is never a second
    /// `class` attribute, and never an inline `style` (so a strict CSP with
    /// no `unsafe-inline` still holds: style the classes from an external
    /// stylesheet). Read by every geom that reads [`data_attr`](Self::data_attr),
    /// plus `geom_text`/`geom_label`. For one constant class on a whole
    /// layer use [`GGPlot::layer_svg_attr`](crate::plot::GGPlot::layer_svg_attr)
    /// `("class", "…")` instead.
    pub fn class(self, col: &str) -> Self {
        self.push(col, Aesthetic::Class)
    }

    /// Wrap marks in caller-named groups (#221): every mark whose row has
    /// the same `col` value is drawn inside one `<g data-<name>="<value>">`
    /// — across *all* layers that map `group_wrap` with the same `name` — so
    /// client code can address (or tween) e.g. a diagram node's box, label
    /// and badge as one element: `querySelector('[data-of="node-7"]')`.
    ///
    /// `name` follows [`data_attr`](Self::data_attr)'s rules
    /// (`[a-z0-9][a-z0-9-]*`, not reserved); a leading `data-` is accepted
    /// and stripped, so `group_wrap("of", …)` and `group_wrap("data-of", …)`
    /// are the same. An invalid name is skipped with a build warning. A row
    /// whose value is NA is drawn ungrouped.
    ///
    /// Paint order: a group is placed where its *first* mark would have
    /// been drawn, and later marks of the same group are appended inside
    /// it. So with a box layer then a label layer, both grouped by node,
    /// the output is `<g data-of="a">box-a label-a</g><g data-of="b">box-b
    /// label-b</g>` — node `b`'s box now paints over node `a`'s label,
    /// which only matters if they overlap. Ungrouped marks keep their
    /// positions. SVG backend only; raster backends ignore grouping.
    pub fn group_wrap(mut self, name: &str, col: &str) -> Self {
        let suffix = name.strip_prefix("data-").unwrap_or(name);
        if !is_valid_data_attr_name(suffix) {
            let reason = if RESERVED_DATA_ATTR_NAMES.contains(&suffix) {
                "reserved (written by ggplot-rs itself)"
            } else {
                "invalid (must match [a-z0-9][a-z0-9-]*)"
            };
            self.warnings.push(format!(
                "aes: group_wrap name {name:?} is {reason}; skipped"
            ));
            return self;
        }
        self.push(col, Aesthetic::GroupWrap(format!("data-{suffix}")))
    }
    /// Opening price (`geom_candlestick` / `geom_ohlc`).
    pub fn open(self, col: &str) -> Self {
        self.push(col, Aesthetic::Open)
    }
    /// Period high (`geom_candlestick` / `geom_ohlc`).
    pub fn high(self, col: &str) -> Self {
        self.push(col, Aesthetic::High)
    }
    /// Period low (`geom_candlestick` / `geom_ohlc`).
    pub fn low(self, col: &str) -> Self {
        self.push(col, Aesthetic::Low)
    }
    /// Closing price (`geom_candlestick` / `geom_ohlc`).
    pub fn close(self, col: &str) -> Self {
        self.push(col, Aesthetic::Close)
    }
    /// Data-mapped `geom_vline` position (one line per row).
    pub fn xintercept(self, col: &str) -> Self {
        self.push(col, Aesthetic::Xintercept)
    }
    /// Data-mapped `geom_hline` position (one line per row).
    pub fn yintercept(self, col: &str) -> Self {
        self.push(col, Aesthetic::Yintercept)
    }
    /// Data-mapped `geom_abline` slope (one line per row).
    pub fn slope(self, col: &str) -> Self {
        self.push(col, Aesthetic::Slope)
    }
    /// Data-mapped `geom_abline` intercept (one line per row).
    pub fn intercept(self, col: &str) -> Self {
        self.push(col, Aesthetic::Intercept)
    }

    // ─── after_stat() mappings ──────────────────────────────────
    // Map stat-computed columns (e.g., `density`, `count`, `ncount`, `ndensity`)
    // to an aesthetic. These are resolved after the stat step in the build pipeline.

    /// Map a stat-computed column to the y aesthetic (e.g., `after_stat_y("density")`).
    pub fn after_stat_y(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::Y)
    }

    /// Map a stat-computed column to the x aesthetic.
    pub fn after_stat_x(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::X)
    }

    /// Map a stat-computed column to the fill aesthetic.
    pub fn after_stat_fill(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::Fill)
    }

    /// Map a stat-computed column to the color aesthetic.
    pub fn after_stat_color(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::Color)
    }

    /// Map a stat-computed column to the size aesthetic.
    pub fn after_stat_size(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::Size)
    }

    /// Map a stat-computed column to the alpha aesthetic.
    pub fn after_stat_alpha(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::Alpha)
    }

    /// Map a stat-computed column to the linewidth aesthetic.
    pub fn after_stat_linewidth(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::Linewidth)
    }

    /// Get the column mapped to a specific aesthetic.
    pub fn get_mapping(&self, aes: &Aesthetic) -> Option<&str> {
        self.mappings
            .iter()
            .find(|m| m.aesthetic == *aes)
            .map(|m| m.column.as_str())
    }

    /// Merge another Aes into this one. The other's mappings override on conflict.
    pub fn merge(&self, other: &Aes) -> Aes {
        let mut result = self.clone();
        for m in &other.mappings {
            // Remove existing mapping for same aesthetic
            result
                .mappings
                .retain(|existing| existing.aesthetic != m.aesthetic);
            result.mappings.push(m.clone());
        }
        result.warnings.extend(other.warnings.iter().cloned());
        result
    }
}

/// Names ggplot-rs itself writes via a mechanism other than `data_attr`'s
/// `set_mark_attrs` dedup — reserved so a user mapping can never collide and
/// produce a duplicate attribute. `"part"` is `DrawBackend::set_mark_part`
/// (arrowheads: `data-part="arrow"`), written directly in `push_mark`, not
/// through the built-ins list `set_mark_attrs` dedups against. `"geom"` is
/// `SvgBackend::mark_class`'s `data-geom="<layer geom>"`, written the same
/// way, always last, alongside the `gg-mark` class. `"fvalue"`/`"fx"` are
/// `DrawBackend::set_mark_fvalue`/`set_mark_fx` (B2-4), written the same way.
const RESERVED_DATA_ATTR_NAMES: &[&str] = &["part", "geom", "fvalue", "fx"];

/// `[a-z0-9][a-z0-9-]*`, excluding [`RESERVED_DATA_ATTR_NAMES`] — a valid,
/// non-colliding HTML custom-data-attribute suffix.
pub(crate) fn is_valid_data_attr_name(name: &str) -> bool {
    if RESERVED_DATA_ATTR_NAMES.contains(&name) {
        return false;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Internal storage namespace for validated interactive-aesthetic columns
/// (`data_attr`, `tooltip`) in the built DataFrame. Starts with a NUL byte,
/// which can't appear in any column name `resolve_mappings` carries over
/// from a caller's raw source data: that loop explicitly drops a raw column
/// whose name contains a NUL (or this prefix) rather than carrying it
/// through unchanged — see its doc. So in practice no caller-supplied
/// column collides with a name under this prefix, but the real guarantee is
/// that drop, not merely that a legitimate column "wouldn't" be named this
/// way (`GGPlot::new` accepts any `String`, including one with a NUL byte
/// in it, deliberately crafted to collide).
pub(crate) const INTERNAL_PREFIX: &str = "\u{0}ggplot-rs:";

/// Internal storage prefix for validated `data_attr` columns — see
/// [`INTERNAL_PREFIX`]. `geom::set_row_extras` looks for *only* this
/// prefix instead of scanning for anything that merely looks like a
/// `data-*` column, which would also match an unrelated raw source column
/// a caller never mapped through `Aes::data_attr` (the injection this
/// guards against: see `validated_data_attr_internal_name`).
pub(crate) const DATA_ATTR_INTERNAL_PREFIX: &str = "\u{0}ggplot-rs:data-attr:";

/// Internal storage name for a validated `Aes::tooltip` mapping — see
/// [`INTERNAL_PREFIX`]. Without this, an unmapped raw column literally
/// named `"tooltip"` would drive every mark's hover title (the same class
/// of bug `DATA_ATTR_INTERNAL_PREFIX` fixes for `data_attr`); routing it
/// through the same internal-prefix scheme closes that for tooltips too.
pub(crate) const TOOLTIP_INTERNAL_NAME: &str = "\u{0}ggplot-rs:tooltip";

/// Internal storage name for a validated `Aes::class` mapping (#221) — see
/// [`INTERNAL_PREFIX`]; same reasoning as [`TOOLTIP_INTERNAL_NAME`].
pub(crate) const CLASS_INTERNAL_NAME: &str = "\u{0}ggplot-rs:class";

/// Internal storage prefix for a validated `Aes::group_wrap` mapping
/// (#221), followed by the full `data-<name>` attribute — see
/// [`INTERNAL_PREFIX`]; same reasoning as [`DATA_ATTR_INTERNAL_PREFIX`].
pub(crate) const GROUP_WRAP_INTERNAL_PREFIX: &str = "\u{0}ggplot-rs:group-wrap:";

/// One CSS class token (#221): `^[A-Za-z][A-Za-z0-9_-]*$`. Deliberately
/// narrower than CSS allows (no escapes, no leading digit or `-`), so a
/// token can never break out of the attribute or need escaping.
pub(crate) fn is_valid_class_token(token: &str) -> bool {
    let mut chars = token.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Split `value` on whitespace into `(valid tokens, rejected tokens)` —
/// see [`is_valid_class_token`]. Duplicate valid tokens are kept once.
pub(crate) fn split_class_tokens(value: &str) -> (Vec<String>, Vec<String>) {
    let (mut ok, mut bad) = (Vec::<String>::new(), Vec::new());
    for t in value.split_whitespace() {
        if !is_valid_class_token(t) {
            bad.push(t.to_string());
        } else if !ok.iter().any(|o| o == t) {
            ok.push(t.to_string());
        }
    }
    (ok, bad)
}

/// Validate a `group_wrap` mapping's target (`"data-<name>"`, as stored in
/// [`Aesthetic::GroupWrap`]) and return its internal column name — the
/// same single authoritative gate [`validated_data_attr_internal_name`] is.
pub(crate) fn validated_group_wrap_internal_name(target: &str) -> Option<String> {
    let suffix = target.strip_prefix("data-")?;
    is_valid_data_attr_name(suffix).then(|| format!("{GROUP_WRAP_INTERNAL_PREFIX}{target}"))
}

/// Internal storage names for `GeomBoxplot::setup_data`'s outlier-extent
/// columns — see [`INTERNAL_PREFIX`] (review #158 finding 5: these used to
/// be plain `"ymin_final"`/`"ymax_final"`, trained into the y scale for
/// *every* layer at build Step 8b regardless of which geom produced them;
/// `resolve_mappings` carries over any raw source column with a
/// not-otherwise-mapped canonical name, so a raw column literally named
/// `ymin_final` — e.g. a SQL alias — on an unrelated geom like `geom_point`
/// silently widened its y domain. Routing them through the internal-prefix
/// scheme means only `GeomBoxplot::setup_data` itself can ever produce
/// these columns; a raw column with this name is already stripped by
/// `is_forged_internal_name` before it reaches the built data.
pub(crate) const BOXPLOT_YMIN_FINAL: &str = "\u{0}ggplot-rs:ymin_final";
pub(crate) const BOXPLOT_YMAX_FINAL: &str = "\u{0}ggplot-rs:ymax_final";

/// Validate a `data_attr` mapping's target (`"data-<name>"`, as stored in
/// [`Aesthetic::DataAttr`]) and, if valid, return the internal column name
/// it should be written under. This is the single authoritative gate: it
/// runs in `resolve_mappings` regardless of *how* the `DataAttr` mapping was
/// created — the validated `Aes::data_attr` builder, or a `DataAttr`
/// constructed directly and pushed into the public `Aes.mappings` field,
/// bypassing the builder's own check.
pub(crate) fn validated_data_attr_internal_name(data_attr_target: &str) -> Option<String> {
    let suffix = data_attr_target.strip_prefix("data-")?;
    if is_valid_data_attr_name(suffix) {
        Some(format!("{DATA_ATTR_INTERNAL_PREFIX}{data_attr_target}"))
    } else {
        None
    }
}

#[cfg(test)]
mod data_attr_name_tests {
    use super::{is_valid_data_attr_name, validated_data_attr_internal_name};

    #[test]
    fn accepts_lowercase_alnum_and_hyphen() {
        for ok in ["x", "row", "data-ish", "a1", "0a", "value"] {
            assert!(is_valid_data_attr_name(ok), "{ok} should be valid");
        }
    }

    #[test]
    fn rejects_empty_uppercase_and_other_punctuation() {
        for bad in ["", "-x", "X", "a_b", "a.b", "a b", "déjà"] {
            assert!(!is_valid_data_attr_name(bad), "{bad} should be invalid");
        }
    }

    #[test]
    fn rejects_the_reserved_part_name() {
        assert!(!is_valid_data_attr_name("part"));
        assert_eq!(validated_data_attr_internal_name("data-part"), None);
    }

    #[test]
    fn rejects_the_reserved_geom_fvalue_and_fx_names() {
        // B2-0's `data-geom` and B2-4's `data-fvalue`/`data-fx` are written
        // directly in `push_mark`, the same way `data-part` is — a
        // `data_attr` mapping under any of these names would duplicate it.
        for name in ["geom", "fvalue", "fx"] {
            assert!(!is_valid_data_attr_name(name), "{name} should be reserved");
            assert_eq!(
                validated_data_attr_internal_name(&format!("data-{name}")),
                None
            );
        }
    }

    #[test]
    fn rejects_a_malformed_injection_attempt_even_with_the_data_prefix() {
        // Space, `=`, parens: none of these are legal in the suffix.
        assert_eq!(
            validated_data_attr_internal_name("data-a onmouseover=alert(1) b"),
            None
        );
    }

    #[test]
    fn valid_target_maps_to_the_internal_prefix_plus_itself() {
        let internal = validated_data_attr_internal_name("data-orig").unwrap();
        assert!(internal.starts_with(super::DATA_ATTR_INTERNAL_PREFIX));
        assert!(internal.ends_with("data-orig"));
    }
}

#[cfg(test)]
mod aesthetic_and_aes_tests {
    use super::{Aes, Aesthetic};

    /// `col_name()` for `Tooltip`/`DataAttr` is never actually used for
    /// storage (both are special-cased in `aes::mapping`), but it should
    /// still return a sane, documented fallback for a direct caller.
    #[test]
    fn col_name_for_tooltip_and_data_attr() {
        assert_eq!(Aesthetic::Tooltip.col_name(), "tooltip");
        assert_eq!(
            Aesthetic::DataAttr("data-orig".to_string()).col_name(),
            "data-orig"
        );
    }

    #[test]
    fn data_attr_warning_text_distinguishes_invalid_from_reserved() {
        let invalid = Aes::new().data_attr("Not Valid!", "col");
        assert_eq!(invalid.warnings.len(), 1);
        assert!(invalid.warnings[0].contains("invalid"));
        assert!(!invalid.warnings[0].contains("reserved"));

        let reserved = Aes::new().data_attr("part", "col");
        assert_eq!(reserved.warnings.len(), 1);
        assert!(reserved.warnings[0].contains("reserved"));
    }

    #[test]
    fn after_stat_linewidth_maps_a_stat_computed_column() {
        let aes = Aes::new().after_stat_linewidth("weight");
        assert_eq!(aes.get_mapping(&Aesthetic::Linewidth), Some("weight"));
    }
}
