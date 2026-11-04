pub mod grid;
pub mod wrap;

use crate::render::Rect;

/// Whether facet panels share axes.
#[derive(Clone, Debug)]
pub enum FacetScales {
    Fixed,
    FreeX,
    FreeY,
    Free,
}

/// Whether facet panels are sized proportionally to their data range
/// (R's `space =`). `Fixed` = equal-size panels (default).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum FacetSpace {
    #[default]
    Fixed,
    FreeX,
    FreeY,
    Free,
}

impl FacetSpace {
    pub fn free_x(&self) -> bool {
        matches!(self, FacetSpace::FreeX | FacetSpace::Free)
    }
    pub fn free_y(&self) -> bool {
        matches!(self, FacetSpace::FreeY | FacetSpace::Free)
    }
}

/// Which side of a `facet_wrap` panel its strip sits on (ggplot2's
/// `strip.position`, #412).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StripPosition {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

/// Move `facet_grid` strips to the opposite side (ggplot2's `switch`,
/// #412): `X` puts the column strips below the grid, `Y` the row strips on
/// its left, `Both` does both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FacetSwitch {
    #[default]
    None,
    X,
    Y,
    Both,
}

impl FacetSwitch {
    pub fn x(self) -> bool {
        matches!(self, FacetSwitch::X | FacetSwitch::Both)
    }
    pub fn y(self) -> bool {
        matches!(self, FacetSwitch::Y | FacetSwitch::Both)
    }
}

/// Which facet panels get axes (ggplot2's `facet_*(axes =, axis.labels
/// =)`, #412). Default: axes on the outer panels only (`"margins"`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FacetAxes {
    /// An x axis under every panel (`axes = "all_x"`).
    pub all_x: bool,
    /// A y axis left of every panel (`axes = "all_y"`).
    pub all_y: bool,
    /// With `all_x`, tick labels only on the bottom panels
    /// (`axis.labels = "margins"` / `"all_y"`).
    pub margin_labels_x: bool,
    /// With `all_y`, tick labels only on the left panels.
    pub margin_labels_y: bool,
}

impl FacetAxes {
    /// `axes = "all"`: x and y axes on every panel, labelled.
    pub fn all() -> Self {
        FacetAxes {
            all_x: true,
            all_y: true,
            ..Self::default()
        }
    }

    /// `axes = "all_x"`.
    pub fn all_x() -> Self {
        FacetAxes {
            all_x: true,
            ..Self::default()
        }
    }

    /// `axes = "all_y"`.
    pub fn all_y() -> Self {
        FacetAxes {
            all_y: true,
            ..Self::default()
        }
    }

    /// `axis.labels = "margins"`: the inner axes get ticks but no labels.
    pub fn margin_labels(mut self) -> Self {
        self.margin_labels_x = true;
        self.margin_labels_y = true;
        self
    }
}

/// How facet strip labels are formatted.
#[derive(Clone, Default)]
pub enum FacetLabeller {
    /// Show just the value (default).
    #[default]
    Value,
    /// Show "var: value".
    Both,
    /// Custom formatting function: fn(variable_name, value) -> label.
    Custom(fn(&str, &str) -> String),
    /// The value, wrapped onto lines of at most this many characters
    /// (ggplot2's `label_wrap_gen(width)`, #412).
    Wrap(usize),
    /// A template with `{var}` and `{value}` placeholders, e.g.
    /// `"n = {value}"` (ggplot2's `label_bquote(cols = n == .(n))` /
    /// `as_labeller` glue strings, #412).
    Template(String),
    /// A labeller per facet variable (ggplot2's `labeller(var1 = …, var2 =
    /// …)`, #412); variables not listed use [`Value`](Self::Value).
    PerVar(Vec<(String, FacetLabeller)>),
    /// Fixed `value → label` lookups (ggplot2's `as_labeller(c(a = "…"))`,
    /// #412); values not listed show as they are.
    Lookup(Vec<(String, String)>),
}

/// Greedy word wrap at `width` characters (`strwrap`), lines joined by `\n`.
pub fn wrap_label(text: &str, width: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(l) if l.chars().count() + 1 + word.chars().count() <= width.max(1) => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines.join("\n")
}

impl FacetLabeller {
    /// `labeller(var = …)` builder: one labeller per variable (#412).
    pub fn per_var(per: Vec<(&str, FacetLabeller)>) -> Self {
        FacetLabeller::PerVar(per.into_iter().map(|(v, l)| (v.to_string(), l)).collect())
    }

    /// `as_labeller(c(value = "label"))` (#412).
    pub fn lookup(map: Vec<(&str, &str)>) -> Self {
        FacetLabeller::Lookup(
            map.into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }

    pub fn format(&self, var: &str, value: &str) -> String {
        match self {
            FacetLabeller::Value => value.to_string(),
            FacetLabeller::Both => format!("{var}: {value}"),
            FacetLabeller::Custom(f) => f(var, value),
            FacetLabeller::Wrap(w) => wrap_label(value, *w),
            FacetLabeller::Template(t) => t.replace("{var}", var).replace("{value}", value),
            FacetLabeller::PerVar(per) => per
                .iter()
                .find(|(v, _)| v == var)
                .map_or_else(|| value.to_string(), |(_, l)| l.format(var, value)),
            FacetLabeller::Lookup(map) => map
                .iter()
                .find(|(k, _)| k == value)
                .map_or_else(|| value.to_string(), |(_, l)| l.clone()),
        }
    }
}

/// A single panel in a faceted layout.
#[derive(Clone, Debug)]
pub struct Panel {
    pub row: usize,
    pub col: usize,
    pub label: String,
    pub row_label: Option<String>,
    pub col_label: Option<String>,
    pub rect: Rect,
}

/// Facet specification.
#[derive(Clone, Default)]
pub enum Facet {
    #[default]
    None,
    Wrap {
        var: String,
        ncol: Option<usize>,
        scales: FacetScales,
        labeller: FacetLabeller,
        /// Explicit panel order (ggplot2's factor levels): every level gets
        /// a panel, even with no data (`drop = FALSE`), and rows whose value
        /// isn't listed are dropped. `None` = values in first-appearance
        /// order. Set with [`GGPlot::facet_levels`](crate::GGPlot::facet_levels).
        levels: Option<Vec<String>>,
        /// Fill panels column by column (ggplot2's `dir = "v"`, #412).
        vertical: bool,
        /// First row at the top (default; ggplot2's `as.table = TRUE`).
        as_table: bool,
        /// The side each panel's strip sits on (`strip.position`, #412).
        strip_position: StripPosition,
    },
    Grid {
        row_var: Option<String>,
        col_var: Option<String>,
        scales: FacetScales,
        labeller: FacetLabeller,
        #[doc = "Proportional panel sizing (R's `space =`)."]
        space: FacetSpace,
        /// Column strips on top and row strips on the right, unless
        /// switched (`switch`, #412).
        switch: FacetSwitch,
        /// Fixed row / column level order: every level gets its row /
        /// column, even with no data (`drop = FALSE`); unlisted values are
        /// dropped (#412). `None` = values in first-appearance order.
        row_levels: Option<Vec<String>>,
        col_levels: Option<Vec<String>>,
        /// An extra "(all)" row and column pooling every row / column
        /// (ggplot2's `margins = TRUE`, #412).
        margins: bool,
    },
}

impl Facet {
    pub fn is_none(&self) -> bool {
        matches!(self, Facet::None)
    }

    pub fn labeller(&self) -> &FacetLabeller {
        match self {
            Facet::None => &FacetLabeller::Value,
            Facet::Wrap { labeller, .. } => labeller,
            Facet::Grid { labeller, .. } => labeller,
        }
    }
}

#[cfg(test)]
mod labeller_tests {
    use super::FacetLabeller;

    #[test]
    fn template_per_var_and_lookup() {
        assert_eq!(
            FacetLabeller::Template("{var} = {value}".into()).format("cyl", "4"),
            "cyl = 4"
        );
        let l = FacetLabeller::per_var(vec![
            ("cyl", FacetLabeller::Both),
            (
                "am",
                FacetLabeller::lookup(vec![("0", "automatic"), ("1", "manual")]),
            ),
        ]);
        assert_eq!(l.format("cyl", "6"), "cyl: 6");
        assert_eq!(l.format("am", "1"), "manual");
        assert_eq!(l.format("am", "2"), "2");
        assert_eq!(l.format("gear", "3"), "3");
    }
}
