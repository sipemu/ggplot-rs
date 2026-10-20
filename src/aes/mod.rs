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
            Aesthetic::Open => "open",
            Aesthetic::High => "high",
            Aesthetic::Low => "low",
            Aesthetic::Close => "close",
            Aesthetic::Xintercept => "xintercept",
            Aesthetic::Yintercept => "yintercept",
            Aesthetic::Slope => "slope",
            Aesthetic::Intercept => "intercept",
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
/// through the built-ins list `set_mark_attrs` dedups against.
const RESERVED_DATA_ATTR_NAMES: &[&str] = &["part"];

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
        for ok in ["x", "row", "data-ish", "a1", "0a", "fvalue"] {
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
