#[cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
use plotters::prelude::IntoDrawingArea;

use crate::aes::Aes;
use crate::annotate::Annotation;
use crate::build::PlotBuilder;
use crate::coord::cartesian::CoordCartesian;
use crate::coord::fixed::CoordFixed;
use crate::coord::flip::CoordFlip;
use crate::coord::polar::CoordPolar;
use crate::coord::Coord;
use crate::data::{DataFrame, GGData, Value};
use crate::facet::{Facet, FacetLabeller, FacetScales, FacetSpace};
use crate::geom::area::GeomArea;
use crate::geom::bar::GeomBar;
use crate::geom::bin2d::GeomBin2d;
use crate::geom::blank::GeomBlank;
use crate::geom::boxplot::GeomBoxplot;
use crate::geom::col::GeomCol;
use crate::geom::contour::GeomContour;
use crate::geom::count::GeomCount;
use crate::geom::crossbar::GeomCrossbar;
use crate::geom::curve::GeomCurve;
use crate::geom::density::GeomDensity;
use crate::geom::density2d::GeomDensity2d;
use crate::geom::dotplot::GeomDotplot;
use crate::geom::errorbar::GeomErrorbar;
use crate::geom::freqpoly::GeomFreqpoly;
use crate::geom::hex::GeomHex;
use crate::geom::histogram::GeomHistogram;
use crate::geom::jitter::GeomJitter;
use crate::geom::line::GeomLine;
use crate::geom::linerange::GeomLinerange;
use crate::geom::path::GeomPath;
use crate::geom::point::GeomPoint;
use crate::geom::pointrange::GeomPointrange;
use crate::geom::polygon::GeomPolygon;
use crate::geom::qq::{GeomQQ, GeomQQLine};
use crate::geom::rect::GeomRect;
use crate::geom::refline::{GeomAbline, GeomHline, GeomVline};
use crate::geom::ribbon::GeomRibbon;
use crate::geom::rug::GeomRug;
use crate::geom::segment::GeomSegment;
use crate::geom::smooth::GeomSmooth;
use crate::geom::spoke::GeomSpoke;
use crate::geom::step::GeomStep;
use crate::geom::text::{GeomLabel, GeomText};
use crate::geom::tile::GeomTile;
use crate::geom::violin::GeomViolin;
use crate::geom::{Geom, GeomParams};
use crate::position::Position;
use crate::render::layout::PlotLayout;
#[cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
use crate::render::plotters_backend::PlottersAdapter;
use crate::render::renderer::PlotRenderer;
use crate::render::RenderError;
use crate::scale::continuous::ScaleContinuous;
use crate::scale::transform::ScaleTransform;
use crate::scale::Scale;
use crate::stat::Stat;
use crate::theme::Theme;

// Builder methods for the 0.17 statistical-graphics layers (QQ, step ribbons,
// ECDF bands, Cook's contours, horizontal error bars).
mod stat_layers;

/// Labels for the plot.
#[derive(Clone, Debug, Default)]
pub struct Labels {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub x: Option<String>,
    pub y: Option<String>,
    pub caption: Option<String>,
    /// Corner tag (R's `labs(tag = ...)`), e.g. "A" for figure panels.
    pub tag: Option<String>,
}

/// A single layer in the plot.
pub struct Layer {
    pub data: Option<DataFrame>,
    pub mapping: Aes,
    pub geom: Box<dyn Geom>,
    pub stat: Box<dyn Stat>,
    pub position: Box<dyn Position>,
    pub params: GeomParams,
    pub show_legend: Option<bool>,
    /// The geom was configured explicitly (`geom_*_with(...)`), so its colours
    /// are deliberate and the theme's `primary_color` must not override them.
    pub explicit_style: bool,
    /// A `{column}` tooltip template ([`GGPlot::tooltip`]), rendered per row
    /// from the *built* layer data (post-stat) once that's available.
    pub tooltip_template: Option<String>,
}

/// The top-level plot specification — builder pattern.
pub struct GGPlot {
    pub(crate) data: DataFrame,
    pub(crate) mapping: Aes,
    pub(crate) layers: Vec<Layer>,
    pub(crate) scales: Vec<Box<dyn Scale>>,
    pub(crate) coord: Box<dyn Coord>,
    pub(crate) theme: Theme,
    pub(crate) labels: Labels,
    pub(crate) facet: Facet,
    pub(crate) annotations: Vec<Annotation>,
    pub(crate) guide_legend: crate::guide::config::GuideLegend,
    /// Warnings raised while specifying the plot (e.g. a clipped calendar
    /// span); prepended to the build warnings.
    pub(crate) warnings: Vec<String>,
    /// [`GGPlot::legend_bottom_when_narrow`]'s share, if set (#287).
    pub(crate) legend_bottom_share: Option<f64>,
    /// Panel aspect ratio used when the theme sets none (helpers such as
    /// `geom_calendar` need square cells; survives theme presets).
    pub(crate) default_aspect_ratio: Option<f64>,
    /// Hide axes and grid whatever theme is set (layout-only charts such as
    /// `geom_sankey`, whose x/y carry no data meaning); survives presets.
    pub(crate) hide_position_guides: bool,
    /// Hide only the y guides whatever theme is set (`geom_timeline`).
    pub(crate) hide_y_guides: bool,
    /// Plot-wide default locale for axis / legend labels ([`GGPlot::locale`]).
    pub(crate) locale: Option<crate::locale::Locale>,
    /// Whether the rendered root `<svg>` carries `data-plot`/`data-domain`/…
    /// ([`svg_root_data`](Self::svg_root_data)). Default `true`; a consumer
    /// overlaying a fragment on its own basemap (`render_svg_native_at`)
    /// turns it off so the fragment adds no root attributes of its own.
    pub(crate) svg_root_data: bool,
    /// Extra root `<svg>` attributes requested via [`svg_attr`](Self::svg_attr)/
    /// [`svg_attrs`](Self::svg_attrs), validated and applied at render time.
    pub(crate) root_attrs: Vec<(String, String)>,
    /// Whether to emit a `data-warnings` JSON array of the build warnings
    /// on the root `<svg>` ([`svg_warnings_attr`](Self::svg_warnings_attr)).
    /// Default off; byte-identical output when off.
    pub(crate) svg_warnings_attr: bool,
    /// Whether to emit `data-fvalue`/`data-fx` on marks
    /// ([`svg_formatted_values`](Self::svg_formatted_values)). Default off;
    /// byte-identical output when off.
    pub(crate) svg_formatted_values: bool,
    /// #221: validated [`layer_svg_attr`](Self::layer_svg_attr)s as
    /// `(layer index, name, value)`, applied by wrapping that layer's geom
    /// at build time.
    pub(crate) layer_svg_attrs: Vec<(usize, String, String)>,
    /// #255: accessible name for the rendered root `<svg>`
    /// ([`svg_title`](Self::svg_title)) — a `<title>` first child plus
    /// `role="img"`/`aria-labelledby`. Default `None`; byte-identical
    /// output when not set.
    pub(crate) svg_title: Option<String>,
    /// #255: accessible description for the rendered root `<svg>`
    /// ([`svg_desc`](Self::svg_desc)) — a `<desc>` first child plus
    /// `role="img"`/`aria-describedby`. Default `None`; byte-identical
    /// output when not set.
    pub(crate) svg_desc: Option<String>,
}

impl GGPlot {
    /// Create a new plot with the given data source.
    ///
    /// Never panics on malformed input: e.g. column-oriented data whose columns
    /// have different lengths is padded with `NA` and reported as a
    /// [`GGError::ValidationError`] by [`try_build`](Self::try_build) and the
    /// `render_*`/`save*` methods.
    pub fn new(data: impl GGData) -> Self {
        GGPlot {
            data: data.into_dataframe(),
            mapping: Aes::default(),
            layers: Vec::new(),
            scales: Vec::new(),
            coord: Box::new(CoordCartesian::new()),
            theme: Theme::default(),
            labels: Labels::default(),
            facet: Facet::default(),
            annotations: Vec::new(),
            guide_legend: crate::guide::config::GuideLegend::default(),
            warnings: Vec::new(),
            legend_bottom_share: None,
            default_aspect_ratio: None,
            hide_position_guides: false,
            hide_y_guides: false,
            locale: None,
            svg_root_data: true,
            root_attrs: Vec::new(),
            svg_warnings_attr: false,
            svg_formatted_values: false,
            layer_svg_attrs: Vec::new(),
            svg_title: None,
            svg_desc: None,
        }
    }

    /// Set the plot-level aesthetic mapping.
    pub fn aes(mut self, mapping: Aes) -> Self {
        self.warnings.extend(mapping.warnings.iter().cloned());
        self.mapping = mapping;
        self
    }

    // ─── Geom shortcuts ──────────────────────────────────────────

    pub fn geom_point(self) -> Self {
        self.add_geom(GeomPoint::default())
    }

    pub fn geom_point_with(self, geom: GeomPoint) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_line(self) -> Self {
        self.add_geom(GeomLine::default())
    }

    pub fn geom_line_with(self, geom: GeomLine) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_bar(self) -> Self {
        self.add_geom(GeomBar::default())
    }

    pub fn geom_bar_with(self, geom: GeomBar) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_histogram(self) -> Self {
        self.add_geom(GeomHistogram::default())
    }

    pub fn geom_histogram_with(self, geom: GeomHistogram) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_boxplot(self) -> Self {
        self.add_geom(GeomBoxplot::default())
    }

    pub fn geom_boxplot_with(self, geom: GeomBoxplot) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_smooth(self) -> Self {
        self.add_geom(GeomSmooth::default())
    }

    pub fn geom_smooth_with(self, geom: GeomSmooth) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_col(self) -> Self {
        self.add_geom(GeomCol::default())
    }

    pub fn geom_col_with(self, geom: GeomCol) -> Self {
        self.add_geom_with(geom)
    }

    /// Horizontal reference line at a constant `yintercept`. Like ggplot2 the
    /// intercept trains the y scale (the line is always visible) and the line
    /// appears in every facet panel.
    pub fn geom_hline(self, yintercept: f64) -> Self {
        self.add_refline(
            GeomHline::new(yintercept),
            false,
            &[("yintercept", yintercept)],
        )
    }

    /// Add a horizontal reference line with custom styling (color/linetype/width).
    pub fn geom_hline_with(self, geom: GeomHline) -> Self {
        let y = geom.yintercept;
        self.add_refline(geom, true, &[("yintercept", y)])
    }

    /// Data-mapped horizontal lines (ggplot2's `geom_hline(aes(yintercept =
    /// …))`): one line per row of the layer data (the plot data unless
    /// [`layer_data`](Self::layer_data) follows), per facet panel, styled by
    /// any `color`/`linetype`/`alpha` in `mapping`. The plot-level mapping is
    /// not inherited.
    ///
    /// ```
    /// # use ggplot_rs::prelude::*;
    /// let thresholds: Vec<(String, Vec<Value>)> = vec![
    ///     ("bound".into(), vec![Value::Float(-0.2), Value::Float(0.2)]),
    /// ];
    /// let svg = GGPlot::new(vec![
    ///         ("lag".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ///         ("acf".to_string(), vec![Value::Float(0.5), Value::Float(0.1)]),
    ///     ])
    ///     .aes(Aes::new().x("lag").y("acf"))
    ///     .geom_col()
    ///     .geom_hline_aes(Aes::new().yintercept("bound"))
    ///     .layer_data(thresholds)
    ///     .render_svg_native()
    ///     .unwrap();
    /// assert!(svg.contains("data-value=\"0.2\""));
    /// ```
    pub fn geom_hline_aes(self, mapping: Aes) -> Self {
        self.add_geom(GeomHline::mapped()).layer_aes(mapping)
    }

    /// [`geom_hline_aes`](Self::geom_hline_aes) with custom default styling.
    pub fn geom_hline_aes_with(self, geom: GeomHline, mapping: Aes) -> Self {
        self.add_geom_with(geom).layer_aes(mapping)
    }

    /// Vertical reference line at a constant `xintercept` (trains the x
    /// scale, appears in every panel).
    pub fn geom_vline(self, xintercept: f64) -> Self {
        self.add_refline(
            GeomVline::new(xintercept),
            false,
            &[("xintercept", xintercept)],
        )
    }

    /// Add a vertical reference line with custom styling (color/linetype/width).
    pub fn geom_vline_with(self, geom: GeomVline) -> Self {
        let x = geom.xintercept;
        self.add_refline(geom, true, &[("xintercept", x)])
    }

    /// Data-mapped vertical lines (`aes(xintercept = …)`), one per row; see
    /// [`geom_hline_aes`](Self::geom_hline_aes).
    pub fn geom_vline_aes(self, mapping: Aes) -> Self {
        self.add_geom(GeomVline::mapped()).layer_aes(mapping)
    }

    /// [`geom_vline_aes`](Self::geom_vline_aes) with custom default styling.
    pub fn geom_vline_aes_with(self, geom: GeomVline, mapping: Aes) -> Self {
        self.add_geom_with(geom).layer_aes(mapping)
    }

    /// Line `y = intercept + slope · x` in data space, clipped to the panel.
    /// It does not train any scale (as in ggplot2).
    pub fn geom_abline(self, slope: f64, intercept: f64) -> Self {
        self.add_refline(
            GeomAbline::new(slope, intercept),
            false,
            &[("slope", slope), ("intercept", intercept)],
        )
    }

    /// Add a slope/intercept reference line with custom styling.
    pub fn geom_abline_with(self, geom: GeomAbline) -> Self {
        let (b, a) = (geom.slope, geom.intercept);
        self.add_refline(geom, true, &[("slope", b), ("intercept", a)])
    }

    /// Data-mapped ablines (`aes(slope = …, intercept = …)`), one per row; a
    /// missing aesthetic defaults to slope 1 / intercept 0.
    pub fn geom_abline_aes(self, mapping: Aes) -> Self {
        self.add_geom(GeomAbline::mapped()).layer_aes(mapping)
    }

    /// [`geom_abline_aes`](Self::geom_abline_aes) with custom default styling.
    pub fn geom_abline_aes_with(self, geom: GeomAbline, mapping: Aes) -> Self {
        self.add_geom_with(geom).layer_aes(mapping)
    }

    /// A constant reference line as ggplot2 builds it: a one-row layer frame
    /// holding the constants, mapped to their aesthetics.
    fn add_refline(
        self,
        geom: impl Geom + 'static,
        explicit: bool,
        values: &[(&str, f64)],
    ) -> Self {
        let mut data = DataFrame::new();
        let mut mapping = Aes::new();
        for (col, v) in values {
            data.add_column(col.to_string(), vec![Value::Float(*v)]);
            mapping = match *col {
                "xintercept" => mapping.xintercept(col),
                "yintercept" => mapping.yintercept(col),
                "slope" => mapping.slope(col),
                _ => mapping.intercept(col),
            };
        }
        let plot = if explicit {
            self.add_geom_with(geom)
        } else {
            self.add_geom(geom)
        };
        plot.layer_aes(mapping).layer_data(data)
    }

    pub fn geom_text(self) -> Self {
        self.add_geom(GeomText::default())
    }

    pub fn geom_text_with(self, geom: GeomText) -> Self {
        self.add_geom_with(geom)
    }

    /// Annotate with a correlation coefficient + p-value (`ggpubr::stat_cor()`).
    /// Adds a text layer whose statistic computes Pearson correlation via
    /// anofox-statistics; use [`GGPlot::stat_cor_with`] to pick Spearman or set
    /// the label position.
    #[cfg(feature = "ggpubr")]
    pub fn stat_cor(self) -> Self {
        self.geom_text().stat(crate::stat::cor::StatCor::default())
    }

    /// [`GGPlot::stat_cor`] with an explicit [`StatCor`](crate::stat::cor::StatCor)
    /// (method / label position).
    #[cfg(feature = "ggpubr")]
    pub fn stat_cor_with(self, stat: crate::stat::cor::StatCor) -> Self {
        self.geom_text().stat(stat)
    }

    /// Annotate grouped data with a group-comparison p-value
    /// (`ggpubr::stat_compare_means()`). Adds a text layer whose statistic
    /// compares y across the discrete x groups (Wilcoxon for two groups,
    /// Kruskal-Wallis for more) via anofox-statistics.
    #[cfg(feature = "ggpubr")]
    pub fn stat_compare_means(self) -> Self {
        self.geom_text()
            .stat(crate::stat::compare_means::StatCompareMeans::default())
    }

    /// [`GGPlot::stat_compare_means`] with an explicit
    /// [`StatCompareMeans`](crate::stat::compare_means::StatCompareMeans).
    #[cfg(feature = "ggpubr")]
    pub fn stat_compare_means_with(
        self,
        stat: crate::stat::compare_means::StatCompareMeans,
    ) -> Self {
        self.geom_text().stat(stat)
    }

    /// Auto-draw pairwise significance brackets (`ggpubr::stat_compare_means`
    /// with `comparisons`). For each `(group_a, group_b)` pair it runs a
    /// two-sample test (Wilcoxon by default) on the plot's y-values, formats the
    /// p-value, and stacks a labelled [`geom_bracket`](GGPlot::geom_bracket)
    /// above the data. Uses the plot's x/y mapping and data.
    #[cfg(feature = "ggpubr")]
    pub fn stat_compare_means_pairwise(self, comparisons: &[(&str, &str)]) -> Self {
        self.stat_compare_means_pairwise_with(
            crate::stat::compare_means::CompareMethod::Auto,
            comparisons,
        )
    }

    /// [`stat_compare_means_pairwise`](GGPlot::stat_compare_means_pairwise) with
    /// an explicit test method.
    #[cfg(feature = "ggpubr")]
    pub fn stat_compare_means_pairwise_with(
        self,
        method: crate::stat::compare_means::CompareMethod,
        comparisons: &[(&str, &str)],
    ) -> Self {
        // Resolve the x/y source columns from the plot mapping + data.
        let col_for = |a: crate::aes::Aesthetic| {
            self.mapping
                .mappings
                .iter()
                .find(|m| m.aesthetic == a)
                .map(|m| m.column.clone())
        };
        let (xcol, ycol) = match (
            col_for(crate::aes::Aesthetic::X),
            col_for(crate::aes::Aesthetic::Y),
        ) {
            (Some(x), Some(y)) => (x, y),
            _ => return self,
        };
        let (xc, yc) = match (self.data.column(&xcol), self.data.column(&ycol)) {
            (Some(x), Some(y)) => (x, y),
            _ => return self,
        };

        // Bucket y by x category (first-seen order) and find the data range.
        let mut groups: Vec<(String, Vec<f64>)> = Vec::new();
        let mut ymax = f64::NEG_INFINITY;
        let mut ymin = f64::INFINITY;
        for (xv, yv) in xc.iter().zip(yc.iter()) {
            let y = match yv.as_f64() {
                Some(y) if y.is_finite() => y,
                _ => continue,
            };
            ymax = ymax.max(y);
            ymin = ymin.min(y);
            let key = xv.to_group_key();
            if let Some(g) = groups.iter_mut().find(|(k, _)| *k == key) {
                g.1.push(y);
            } else {
                groups.push((key, vec![y]));
            }
        }
        if groups.len() < 2 {
            return self;
        }
        let step = if ymax > ymin {
            (ymax - ymin) * 0.12
        } else {
            1.0
        };

        // One stacked bracket per resolvable comparison.
        let (mut xmin_v, mut xmax_v, mut y_v, mut label_v) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut placed = 0usize;
        for (a, b) in comparisons {
            let ga = groups.iter().find(|(k, _)| k == a).map(|g| &g.1);
            let gb = groups.iter().find(|(k, _)| k == b).map(|g| &g.1);
            let (ga, gb) = match (ga, gb) {
                (Some(x), Some(y)) if x.len() >= 2 && y.len() >= 2 => (x, y),
                _ => continue,
            };
            let p = match crate::stat::compare_means::pairwise_p(method, ga, gb) {
                Some(p) => p,
                None => continue,
            };
            xmin_v.push(Value::Str((*a).to_string()));
            xmax_v.push(Value::Str((*b).to_string()));
            y_v.push(Value::Float(ymax + step * (placed as f64 + 1.0)));
            label_v.push(Value::Str(crate::stat::cor::format_p_value(p)));
            placed += 1;
        }
        if xmin_v.is_empty() {
            return self;
        }

        let data = vec![
            ("xmin".to_string(), xmin_v),
            ("xmax".to_string(), xmax_v),
            ("y".to_string(), y_v),
            ("label".to_string(), label_v),
        ];
        self.add_geom(crate::geom::bracket::GeomBracket::default())
            .layer_data(data)
            .layer_aes(Aes::new().xmin("xmin").xmax("xmax").y("y").label("label"))
    }

    /// Draw a significance bracket over the plot (`ggpubr::geom_bracket`): a bar
    /// spanning the categorical positions `xmin`..`xmax` at height `y`, captioned
    /// with `label` (e.g. a p-value or significance stars). Pairs with a boxplot
    /// and [`stat_compare_means`](GGPlot::stat_compare_means).
    pub fn geom_bracket(self, xmin: &str, xmax: &str, y: f64, label: &str) -> Self {
        self.geom_bracket_many(
            crate::geom::bracket::GeomBracket::default(),
            &[(xmin, xmax, y, label)],
        )
    }

    /// Draw several significance brackets at once with a configured
    /// [`GeomBracket`](crate::geom::bracket::GeomBracket). Each tuple is
    /// `(xmin, xmax, y, label)`.
    pub fn geom_bracket_many(
        self,
        geom: crate::geom::bracket::GeomBracket,
        brackets: &[(&str, &str, f64, &str)],
    ) -> Self {
        let xmin = brackets
            .iter()
            .map(|b| Value::Str(b.0.to_string()))
            .collect::<Vec<_>>();
        let xmax = brackets
            .iter()
            .map(|b| Value::Str(b.1.to_string()))
            .collect::<Vec<_>>();
        let y = brackets
            .iter()
            .map(|b| Value::Float(b.2))
            .collect::<Vec<_>>();
        let label = brackets
            .iter()
            .map(|b| Value::Str(b.3.to_string()))
            .collect::<Vec<_>>();
        let data = vec![
            ("xmin".to_string(), xmin),
            ("xmax".to_string(), xmax),
            ("y".to_string(), y),
            ("label".to_string(), label),
        ];
        self.add_geom(geom)
            .layer_data(data)
            .layer_aes(Aes::new().xmin("xmin").xmax("xmax").y("y").label("label"))
    }

    /// Significance brackets from a precomputed test table (ggpubr's
    /// `stat_pvalue_manual` / `geom_bracket(data = …)`) — e.g. the anofox
    /// `test` contract output (`group1`, `group2`, `p_adj`/`p_value`, optional
    /// `y_position`, `label`). No test is recomputed. See
    /// [`BracketTable`](crate::geom::bracket::BracketTable) for the column
    /// rules, label templates (`"p = {p_adj}"`, `"{p.signif}"`) and the
    /// automatic stacking of rows without a `y_position`. Rows that cannot be
    /// drawn are dropped with a build warning.
    pub fn geom_bracket_table(
        mut self,
        table: impl GGData,
        spec: crate::geom::bracket::BracketTable,
    ) -> Self {
        let table = table.into_dataframe();
        let col_for = |a: crate::aes::Aesthetic| {
            self.mapping
                .mappings
                .iter()
                .find(|m| m.aesthetic == a)
                .map(|m| m.column.clone())
        };
        // The plot's x categories (to reject unknown groups) and finite y
        // range (to stack brackets above the data).
        let x_levels: Vec<String> = col_for(crate::aes::Aesthetic::X)
            .and_then(|c| self.data.column(&c))
            .filter(|col| col.iter().any(|v| matches!(v, Value::Str(_))))
            .map(|col| {
                let mut seen = std::collections::HashSet::new();
                col.iter()
                    .filter(|v| !v.is_na())
                    .map(|v| v.to_group_key())
                    .filter(|k| seen.insert(k.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let y_range = col_for(crate::aes::Aesthetic::Y)
            .and_then(|c| self.data.column(&c))
            .and_then(|col| {
                let (lo, hi) = col
                    .iter()
                    .filter_map(|v| v.as_f64())
                    .filter(|v| v.is_finite())
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                        (lo.min(v), hi.max(v))
                    });
                (lo <= hi).then_some((lo, hi))
            });
        let mut warnings = Vec::new();
        let resolved = spec.resolve(&table, &x_levels, y_range, &mut warnings);
        self.warnings.extend(warnings);
        match resolved {
            Some(data) => self
                .add_geom(crate::geom::bracket::GeomBracket::from(spec.geom))
                .layer_data(data)
                .layer_aes(Aes::new().xmin("xmin").xmax("xmax").y("y").label("label")),
            None => self,
        }
    }

    /// Text labels that repel each other and their points
    /// (`ggrepel::geom_text_repel`) — deterministic (seeded) layout; see
    /// [`GeomTextRepel`](crate::geom::repel::GeomTextRepel). Requires `x`,
    /// `y` and `label`.
    pub fn geom_text_repel(self) -> Self {
        self.add_geom(crate::geom::repel::GeomTextRepel::default())
    }

    /// [`geom_text_repel`](Self::geom_text_repel) with a configured geom
    /// (padding, nudge, `max_overlaps`, seed, …).
    pub fn geom_text_repel_with(self, geom: crate::geom::repel::GeomTextRepel) -> Self {
        self.add_geom_with(geom)
    }

    /// Boxed labels that repel each other and their points
    /// (`ggrepel::geom_label_repel`).
    pub fn geom_label_repel(self) -> Self {
        self.add_geom(crate::geom::repel::GeomLabelRepel::default())
    }

    /// [`geom_label_repel`](Self::geom_label_repel) with a configured geom.
    pub fn geom_label_repel_with(self, geom: crate::geom::repel::GeomLabelRepel) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_label(self) -> Self {
        self.add_geom(GeomLabel::default())
    }

    pub fn geom_label_with(self, geom: GeomLabel) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_area(self) -> Self {
        self.add_geom(GeomArea::default())
    }

    pub fn geom_area_with(self, geom: GeomArea) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_ribbon(self) -> Self {
        self.add_geom(GeomRibbon::default())
    }

    pub fn geom_ribbon_with(self, geom: GeomRibbon) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_errorbar(self) -> Self {
        self.add_geom(GeomErrorbar::default())
    }

    pub fn geom_errorbar_with(self, geom: GeomErrorbar) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_segment(self) -> Self {
        self.add_geom(GeomSegment::default())
    }

    pub fn geom_segment_with(self, geom: GeomSegment) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_density(self) -> Self {
        self.add_geom(GeomDensity::default())
    }

    pub fn geom_density_with(self, geom: GeomDensity) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_rug(self) -> Self {
        self.add_geom(GeomRug::default())
    }

    pub fn geom_rug_with(self, geom: GeomRug) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_jitter(self) -> Self {
        self.add_geom(GeomJitter::default())
    }

    pub fn geom_jitter_with(self, geom: GeomJitter) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_path(self) -> Self {
        self.add_geom(GeomPath::default())
    }

    pub fn geom_path_with(self, geom: GeomPath) -> Self {
        self.add_geom_with(geom)
    }

    /// Add a confidence-ellipse layer (default 95%) as a path per group.
    pub fn stat_ellipse(self) -> Self {
        self.geom_path()
            .stat(crate::stat::ellipse::StatEllipse::default())
    }

    /// Add a confidence-ellipse layer at the given level (0, 1).
    pub fn stat_ellipse_level(self, level: f64) -> Self {
        self.geom_path()
            .stat(crate::stat::ellipse::StatEllipse::new(level))
    }

    /// Add a quantile-regression line for each `tau` as a separate path layer
    /// (R's `stat_quantile`). Backed by anofox-regression (feature `regression`).
    #[cfg(feature = "regression")]
    pub fn stat_quantile(mut self, taus: &[f64]) -> Self {
        for &tau in taus {
            self = self
                .geom_path()
                .stat(crate::stat::quantile::StatQuantile::new(tau));
        }
        self
    }

    /// Quantile-regression lines at the quartiles (0.25, 0.5, 0.75).
    #[cfg(feature = "regression")]
    pub fn geom_quantile(self) -> Self {
        self.stat_quantile(&[0.25, 0.5, 0.75])
    }

    pub fn geom_step(self) -> Self {
        self.add_geom(GeomStep::default())
    }

    pub fn geom_step_with(self, geom: GeomStep) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_freqpoly(self) -> Self {
        self.add_geom(GeomFreqpoly::default())
    }

    pub fn geom_freqpoly_with(self, geom: GeomFreqpoly) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_linerange(self) -> Self {
        self.add_geom(GeomLinerange::default())
    }

    pub fn geom_linerange_with(self, geom: GeomLinerange) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_pointrange(self) -> Self {
        self.add_geom(GeomPointrange::default())
    }

    pub fn geom_pointrange_with(self, geom: GeomPointrange) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_crossbar(self) -> Self {
        self.add_geom(GeomCrossbar::default())
    }

    pub fn geom_crossbar_with(self, geom: GeomCrossbar) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_spoke(self) -> Self {
        self.add_geom(GeomSpoke::default())
    }

    pub fn geom_spoke_with(self, geom: GeomSpoke) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_rect(self) -> Self {
        self.add_geom(GeomRect::default())
    }

    pub fn geom_rect_with(self, geom: GeomRect) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_tile(self) -> Self {
        self.add_geom(GeomTile::default())
    }

    pub fn geom_tile_with(self, geom: GeomTile) -> Self {
        self.add_geom_with(geom)
    }

    /// Dense regular grid of filled cells (heatmap/raster) from x, y, fill.
    pub fn geom_raster(self) -> Self {
        self.add_geom(crate::geom::raster::GeomRaster::default())
    }

    pub fn geom_raster_with(self, geom: crate::geom::raster::GeomRaster) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_polygon(self) -> Self {
        self.add_geom(GeomPolygon::default())
    }

    pub fn geom_polygon_with(self, geom: GeomPolygon) -> Self {
        self.add_geom_with(geom)
    }

    /// Render simple-features geometry from a WKT `geometry` column (feature `sf`).
    #[cfg(feature = "sf")]
    pub fn geom_sf(self) -> Self {
        self.add_geom(crate::geom::sf::GeomSf::default())
    }

    #[cfg(feature = "sf")]
    pub fn geom_sf_with(self, geom: crate::geom::sf::GeomSf) -> Self {
        self.add_geom_with(geom)
    }

    /// Labels placed inside `geom_sf` polygons (feature `sf`): `label`
    /// aesthetic, `geometry` column — ggplot2's `geom_sf_text`.
    #[cfg(feature = "sf")]
    pub fn geom_sf_text(self) -> Self {
        self.add_geom(crate::geom::sf_text::GeomSfText::default())
    }

    #[cfg(feature = "sf")]
    pub fn geom_sf_text_with(self, geom: crate::geom::sf_text::GeomSfText) -> Self {
        self.add_geom_with(geom)
    }

    /// Like [`geom_sf_text`](Self::geom_sf_text), on a background box —
    /// ggplot2's `geom_sf_label`.
    #[cfg(feature = "sf")]
    pub fn geom_sf_label(self) -> Self {
        self.add_geom(crate::geom::sf_text::GeomSfLabel::default())
    }

    #[cfg(feature = "sf")]
    pub fn geom_sf_label_with(self, geom: crate::geom::sf_text::GeomSfLabel) -> Self {
        self.add_geom_with(geom)
    }

    /// A metric scale bar in a panel corner (feature `sf`) — ggspatial's
    /// `annotation_scale()`. Assumes the X/Y domain is in metres (any
    /// `geom_sf` projection other than `SfProjection::PlateCarree`).
    #[cfg(feature = "sf")]
    pub fn annotation_scale(self) -> Self {
        self.add_geom(crate::geom::annotation_scale::GeomAnnotationScale::default())
    }

    #[cfg(feature = "sf")]
    pub fn annotation_scale_with(
        self,
        geom: crate::geom::annotation_scale::GeomAnnotationScale,
    ) -> Self {
        self.add_geom_with(geom)
    }

    /// A north arrow in a panel corner (feature `sf`) — see
    /// [`GeomNorthArrow`](crate::geom::annotation_scale::GeomNorthArrow).
    #[cfg(feature = "sf")]
    pub fn annotation_north_arrow_with(
        self,
        geom: crate::geom::annotation_scale::GeomNorthArrow,
    ) -> Self {
        self.add_geom_with(geom)
    }

    /// A small, muted attribution text in a panel corner (feature `sf`) —
    /// e.g. `"© Natural Earth"`.
    #[cfg(feature = "sf")]
    pub fn annotation_attribution(self, text: impl Into<String>) -> Self {
        self.add_geom(crate::geom::annotation_scale::GeomAttribution::new(text))
    }

    #[cfg(feature = "sf")]
    pub fn annotation_attribution_with(
        self,
        geom: crate::geom::annotation_scale::GeomAttribution,
    ) -> Self {
        self.add_geom_with(geom)
    }

    /// The square `n × m` guide for a
    /// [`ScaleFillBivariate`](crate::scale::bivariate::ScaleFillBivariate)
    /// fill scale (feature `sf`): two axis captions, biscale's own legend
    /// shape — drawn as a furniture layer, not through the ordinary
    /// per-aesthetic legend (set `.legend_position(LegendPosition::None)`
    /// to hide that one). [`GeomBivariateLegend::from_scale`] builds this
    /// from the same scale used for the fill, so the swatches match.
    #[cfg(feature = "sf")]
    pub fn bivariate_legend(
        self,
        geom: crate::geom::bivariate_legend::GeomBivariateLegend,
    ) -> Self {
        self.add_geom(geom)
    }

    /// Like [`Self::bivariate_legend`], but with each cell's row count
    /// drawn centred in its swatch — build with
    /// [`GeomBivariateLegend::counts`](crate::geom::bivariate_legend::GeomBivariateLegend::counts)
    /// (#259).
    #[cfg(feature = "sf")]
    pub fn bivariate_legend_counts(
        self,
        geom: crate::geom::bivariate_legend::GeomBivariateLegendCounts,
    ) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_curve(self) -> Self {
        self.add_geom(GeomCurve::default())
    }

    pub fn geom_curve_with(self, geom: GeomCurve) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_violin(self) -> Self {
        self.add_geom(GeomViolin::default())
    }

    pub fn geom_violin_with(self, geom: GeomViolin) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_dotplot(self) -> Self {
        self.add_geom(GeomDotplot::default())
    }

    pub fn geom_dotplot_with(self, geom: GeomDotplot) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_qq(self) -> Self {
        self.add_geom(GeomQQ::default())
    }

    pub fn geom_qq_with(self, geom: GeomQQ) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_qq_line(self) -> Self {
        self.add_geom(GeomQQLine::default())
    }

    pub fn geom_qq_line_with(self, geom: GeomQQLine) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_bin2d(self) -> Self {
        self.add_geom(GeomBin2d::default())
    }

    pub fn geom_bin2d_with(self, geom: GeomBin2d) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_hex(self) -> Self {
        self.add_geom(GeomHex::default())
    }

    pub fn geom_hex_with(self, geom: GeomHex) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_count(self) -> Self {
        self.add_geom(GeomCount::default())
    }

    pub fn geom_count_with(self, geom: GeomCount) -> Self {
        self.add_geom_with(geom)
    }

    pub fn geom_contour(self) -> Self {
        self.add_geom(GeomContour::default())
    }

    pub fn geom_contour_with(self, geom: GeomContour) -> Self {
        self.add_geom_with(geom)
    }

    /// Filled contour bands from gridded (x, y, z) data — draws polygons filled by
    /// band level. Pair with a continuous fill scale (e.g. `scale_fill_viridis_c`).
    pub fn geom_contour_filled(self) -> Self {
        self.add_geom(GeomPolygon {
            line_width: 0.0,
            alpha: 1.0,
            ..GeomPolygon::default()
        })
        .stat(crate::stat::contour_filled::StatContourFilled::default())
    }

    pub fn geom_density2d(self) -> Self {
        self.add_geom(GeomDensity2d::default())
    }

    pub fn geom_density2d_with(self, geom: GeomDensity2d) -> Self {
        self.add_geom_with(geom)
    }

    /// Waterfall (bridge) chart: map `x` (steps, in data order) and `y` (each
    /// step's change). Bars float from the running total and are filled by
    /// direction (Increase / Decrease / Total, with a legend); dashed
    /// connectors join consecutive bars. See [`geom_waterfall_with`](Self::geom_waterfall_with)
    /// for subtotals and a closing total.
    pub fn geom_waterfall(self) -> Self {
        self.geom_waterfall_with(crate::geom::waterfall::GeomWaterfall::default())
    }

    /// [`geom_waterfall`](Self::geom_waterfall) with explicit colours,
    /// `totals` (subtotal x levels) and `final_total`. Unless `fill` is
    /// already mapped at plot level, the layer maps `after_stat(fill =
    /// direction)`; unless a fill scale is already set, a manual fill scale
    /// with the geom's `up`/`down`/`total` colours is added (a later
    /// `scale_fill_*` call replaces it).
    pub fn geom_waterfall_with(self, geom: crate::geom::waterfall::GeomWaterfall) -> Self {
        use crate::scale::color::RGBAColor;
        use crate::stat::waterfall::{WATERFALL_DECREASE, WATERFALL_INCREASE, WATERFALL_TOTAL};
        let rgba = |(r, g, b): (u8, u8, u8)| RGBAColor::new(r, g, b);
        let mut values = vec![
            (WATERFALL_INCREASE, rgba(geom.up)),
            (WATERFALL_DECREASE, rgba(geom.down)),
        ];
        if geom.has_totals() {
            values.push((WATERFALL_TOTAL, rgba(geom.total)));
        }
        let fill_mapped = self
            .mapping
            .get_mapping(&crate::aes::Aesthetic::Fill)
            .is_some();
        let has_fill_scale = self
            .scales
            .iter()
            .any(|s| s.aesthetic() == crate::aes::Aesthetic::Fill);
        let mut plot = self.add_geom_with(geom);
        if !fill_mapped {
            plot = plot.layer_aes(Aes::new().after_stat_fill("direction"));
            if !has_fill_scale {
                plot = plot.scale_fill_manual(values);
            }
        }
        plot
    }

    /// Pareto chart: map `x` (categories) and `y` (values, summed per
    /// category). Bars are sorted descending and a cumulative line with
    /// points rises to the grand total, read against a 0–100 % right axis.
    pub fn geom_pareto(self) -> Self {
        self.add_pareto(crate::geom::pareto::GeomPareto::default(), false)
    }

    /// [`geom_pareto`](Self::geom_pareto) with explicit styling, sorting or a
    /// threshold line. Unless a y scale is already set, a continuous y scale
    /// with a [`SecAxis::percent_of_max`](crate::scale::sec_axis::SecAxis::percent_of_max)
    /// secondary axis ("Cumulative %") is added; a later `scale_y_continuous`
    /// replaces it.
    pub fn geom_pareto_with(self, geom: crate::geom::pareto::GeomPareto) -> Self {
        self.add_pareto(geom, true)
    }

    fn add_pareto(self, geom: crate::geom::pareto::GeomPareto, explicit: bool) -> Self {
        let has_y_scale = self
            .scales
            .iter()
            .any(|s| s.aesthetic() == crate::aes::Aesthetic::Y);
        let plot = if explicit {
            self.add_geom_with(geom)
        } else {
            self.add_geom(geom)
        };
        if has_y_scale {
            plot
        } else {
            plot.scale_y_continuous(ScaleContinuous::new().with_sec_axis(
                crate::scale::sec_axis::SecAxis::percent_of_max().with_name("Cumulative %"),
            ))
        }
    }

    /// Sankey diagram from an edge list: map `from`, `to` and optionally
    /// `weight` (`Aes::new().from("src").to("dst").weight("flow")`). Nodes
    /// are placed in columns by their depth, sized by their flow and
    /// coloured by name through the fill scale (no legend — nodes are
    /// labelled); links take their source's colour. Axes and grid are
    /// hidden, whatever the theme.
    pub fn geom_sankey(self) -> Self {
        self.add_sankey(crate::geom::sankey::GeomSankey::default(), false)
    }

    /// [`geom_sankey`](Self::geom_sankey) with explicit layout and styling.
    pub fn geom_sankey_with(self, geom: crate::geom::sankey::GeomSankey) -> Self {
        self.add_sankey(geom, true)
    }

    fn add_sankey(self, geom: crate::geom::sankey::GeomSankey, explicit: bool) -> Self {
        let mut plot = if explicit {
            self.add_geom_with(geom)
        } else {
            self.add_geom(geom)
        };
        plot.hide_position_guides = true;
        plot.layer_aes(Aes::new().after_stat_fill("node"))
            .show_legend(false)
    }

    /// Chord diagram / dependency wheel from an edge list: map `from`, `to`
    /// and optionally `weight`. Each node is an arc sized by its total flow
    /// (in + out); each link a ribbon through the centre, coloured by its
    /// source. Nodes are coloured by name through the fill scale (no legend
    /// — they are labelled); axes and grid are hidden and the panel is kept
    /// square so the circle stays round.
    pub fn geom_chord(self) -> Self {
        self.add_chord(crate::geom::chord::GeomChord::default(), false)
    }

    /// [`geom_chord`](Self::geom_chord) with explicit padding and styling.
    pub fn geom_chord_with(self, geom: crate::geom::chord::GeomChord) -> Self {
        self.add_chord(geom, true)
    }

    fn add_chord(self, geom: crate::geom::chord::GeomChord, explicit: bool) -> Self {
        let (ex, ey) = geom.extents();
        let mut plot = if explicit {
            self.add_geom_with(geom)
        } else {
            self.add_geom(geom)
        };
        plot.hide_position_guides = true;
        plot.default_aspect_ratio = Some(ey / ex);
        plot.layer_aes(Aes::new().after_stat_fill("node"))
            .show_legend(false)
    }

    /// Arc diagram from an edge list: map `from`, `to` and optionally
    /// `weight`. Nodes sit on a line (sized by total flow), links are arcs
    /// whose width is their weight. Coloured by node name through the fill
    /// scale (no legend — nodes are labelled); axes and grid are hidden.
    pub fn geom_arcdiagram(self) -> Self {
        self.add_arcdiagram(crate::geom::arcdiagram::GeomArcdiagram::default(), false)
    }

    /// [`geom_arcdiagram`](Self::geom_arcdiagram) with explicit ordering,
    /// direction and styling.
    pub fn geom_arcdiagram_with(self, geom: crate::geom::arcdiagram::GeomArcdiagram) -> Self {
        self.add_arcdiagram(geom, true)
    }

    fn add_arcdiagram(self, geom: crate::geom::arcdiagram::GeomArcdiagram, explicit: bool) -> Self {
        let mut plot = if explicit {
            self.add_geom_with(geom)
        } else {
            self.add_geom(geom)
        };
        plot.hide_position_guides = true;
        plot.layer_aes(Aes::new().after_stat_fill("node"))
            .show_legend(false)
    }

    /// Squarified treemap: map `weight` (tile area) and optionally
    /// `subgroup` (tiles nest inside group rectangles), `fill` and `label`
    /// (`Aes::new().weight("sales").subgroup("region").label("country")`).
    /// When `subgroup` is mapped at plot level and `fill` is not, tiles are
    /// filled by subgroup. The panel aspect matches the layout so tiles come
    /// out square; the position scales get no expansion and the axes and
    /// grid are hidden.
    pub fn geom_treemap(self) -> Self {
        self.add_treemap(crate::geom::treemap::GeomTreemap::default(), false)
    }

    /// [`geom_treemap`](Self::geom_treemap) with explicit layout and styling.
    pub fn geom_treemap_with(self, geom: crate::geom::treemap::GeomTreemap) -> Self {
        self.add_treemap(geom, true)
    }

    fn add_treemap(self, geom: crate::geom::treemap::GeomTreemap, explicit: bool) -> Self {
        use crate::aes::Aesthetic;
        let aspect = geom.aspect;
        let has = |plot: &Self, aes: Aesthetic| plot.scales.iter().any(|s| s.aesthetic() == aes);
        let (has_x, has_y) = (has(&self, Aesthetic::X), has(&self, Aesthetic::Y));
        let fill_by = match (
            self.mapping.get_mapping(&Aesthetic::Fill),
            self.mapping.get_mapping(&Aesthetic::Subgroup),
        ) {
            (None, Some(sub)) => Some(sub.to_string()),
            _ => None,
        };
        let mut plot = if explicit {
            self.add_geom_with(geom)
        } else {
            self.add_geom(geom)
        };
        plot.hide_position_guides = true;
        plot.default_aspect_ratio = Some(1.0 / aspect.max(1e-6));
        if let Some(sub) = fill_by {
            plot = plot.layer_aes(Aes::new().fill(&sub));
        }
        if !has_x {
            plot = plot.scale_x_continuous(ScaleContinuous::new().with_expand(0.0, 0.0));
        }
        if !has_y {
            plot = plot.scale_y_continuous(ScaleContinuous::new().with_expand(0.0, 0.0));
        }
        plot
    }

    /// Sunburst over the hierarchy columns `levels` (outermost first),
    /// sized by `weight` (1 per row when unmapped): concentric rings, one per
    /// level, each node's angle ∝ its value and nested in its parent's.
    /// Sets `coord_polar`, colours each branch by its top-level ancestor
    /// (lighter further out; no legend — sectors are labelled), drops scale
    /// expansion and hides the position guides.
    pub fn geom_sunburst<S: Into<String>>(self, levels: impl IntoIterator<Item = S>) -> Self {
        let geom = crate::geom::partition::GeomPartition::sunburst(levels);
        // The hole is `hole` level-widths of the `levels.len() + hole` radius.
        let depth = geom.levels.len().max(1) as f64;
        let inner = geom.hole / (geom.hole + depth);
        let mut plot = self.geom_partition_with(geom);
        plot.coord = Box::new(CoordPolar::new().inner_radius(inner));
        plot
    }

    /// Icicle chart over `levels`: the partition of
    /// [`geom_sunburst`](Self::geom_sunburst) in Cartesian coordinates, root
    /// level on top.
    pub fn geom_icicle<S: Into<String>>(self, levels: impl IntoIterator<Item = S>) -> Self {
        self.geom_partition_with(crate::geom::partition::GeomPartition::icicle(levels))
    }

    /// A partition layer with explicit settings (sunburst under
    /// `coord_polar`, icicle otherwise); same defaults as the shortcuts
    /// except the coordinate system, which is left as is.
    pub fn geom_partition_with(self, geom: crate::geom::partition::GeomPartition) -> Self {
        use crate::aes::Aesthetic;
        let has = |plot: &Self, aes: Aesthetic| plot.scales.iter().any(|s| s.aesthetic() == aes);
        let (has_x, has_y) = (has(&self, Aesthetic::X), has(&self, Aesthetic::Y));
        let fill_mapped = self.mapping.get_mapping(&Aesthetic::Fill).is_some();
        let mut plot = self.add_geom_with(geom);
        plot.hide_position_guides = true;
        if !fill_mapped {
            plot = plot
                .layer_aes(Aes::new().after_stat_fill("root"))
                .show_legend(false);
        }
        if !has_x {
            plot = plot.scale_x_continuous(ScaleContinuous::new().with_expand(0.0, 0.0));
        }
        if !has_y {
            plot = plot.scale_y_continuous(ScaleContinuous::new().with_expand(0.0, 0.0));
        }
        plot
    }

    /// Funnel chart: map `x` (stage, in data order; repeats are summed) and
    /// `y` (value). Equal-height stages whose width is proportional to the
    /// value, each tapering to the next; labels show the value and the
    /// conversion from the first stage. Stages are filled by name (no
    /// legend); axes and grid are hidden. See
    /// [`GeomFunnel::silhouette`](crate::geom::funnel::GeomFunnel::silhouette)
    /// for the Highcharts-style fixed shape.
    pub fn geom_funnel(self) -> Self {
        self.geom_funnel_with(crate::geom::funnel::GeomFunnel::default())
    }

    /// Pyramid chart: apex on top, stage heights proportional to the values.
    pub fn geom_pyramid(self) -> Self {
        self.geom_funnel_with(crate::geom::funnel::GeomFunnel::pyramid())
    }

    /// [`geom_funnel`](Self::geom_funnel) with an explicit shape and styling.
    pub fn geom_funnel_with(self, geom: crate::geom::funnel::GeomFunnel) -> Self {
        let fill_mapped = self
            .mapping
            .get_mapping(&crate::aes::Aesthetic::Fill)
            .is_some();
        let mut plot = self.add_geom_with(geom);
        plot.hide_position_guides = true;
        if !fill_mapped {
            plot = plot
                .layer_aes(Aes::new().after_stat_fill("stage"))
                .show_legend(false);
        }
        plot
    }

    /// Streamgraph: [`geom_area`](Self::geom_area) stacked around a
    /// wiggle-minimising baseline ([`StackOffset::Wiggle`](crate::position::stack::StackOffset::Wiggle)).
    /// Map `x`, `y` (non-negative) and `fill` (the series). For a
    /// symmetric ThemeRiver use
    /// `.geom_area().position(position_stack().with_offset(StackOffset::Silhouette))`.
    pub fn geom_stream(self) -> Self {
        self.geom_area().position(
            crate::position::stack::position_stack()
                .with_offset(crate::position::stack::StackOffset::Wiggle),
        )
    }

    /// Lollipop chart: a stem from 0 to `y` capped with a dot, per `x`
    /// (`coord_flip` for horizontal lollipops).
    pub fn geom_lollipop(self) -> Self {
        self.add_geom(crate::geom::lollipop::GeomLollipop::default())
    }

    pub fn geom_lollipop_with(self, geom: crate::geom::lollipop::GeomLollipop) -> Self {
        self.add_geom_with(geom)
    }

    /// Dumbbell chart: a segment from `ymin` to `ymax` per `x`, with a dot at
    /// each end (low / high colours); `coord_flip` for the usual horizontal
    /// layout.
    pub fn geom_dumbbell(self) -> Self {
        self.add_geom(crate::geom::lollipop::GeomDumbbell::default())
    }

    pub fn geom_dumbbell_with(self, geom: crate::geom::lollipop::GeomDumbbell) -> Self {
        self.add_geom_with(geom)
    }

    /// Bullet chart (horizontal): map `x` (KPI) and `y` (actual); configure
    /// the target column and range bands on the geom, e.g.
    /// `GeomBullet::default().with_target("goal").with_ranges([60.0, 80.0, 100.0])`.
    /// Sets `coord_flip`; use [`geom_bullet_vertical`](Self::geom_bullet_vertical)
    /// for upright bullets.
    pub fn geom_bullet(self, geom: crate::geom::bullet::GeomBullet) -> Self {
        self.add_geom_with(geom).coord_flip()
    }

    /// [`geom_bullet`](Self::geom_bullet) without the flip (vertical bars).
    pub fn geom_bullet_vertical(self, geom: crate::geom::bullet::GeomBullet) -> Self {
        self.add_geom_with(geom)
    }

    /// Waffle chart: map `fill` (category) and `weight` (units per row);
    /// units fill a grid `rows` high, one contiguous block per category.
    /// Axes and grid are hidden; units stay square at any panel aspect.
    pub fn geom_waffle(self, rows: usize) -> Self {
        self.geom_waffle_with(crate::geom::waffle::GeomWaffle::waffle(rows))
    }

    /// Parliament (hemicycle) chart: map `fill` (party) and `weight`
    /// (seats); seats on concentric arcs, parties left to right in data
    /// order. `rows = 0` picks the number of arcs from the seat count.
    pub fn geom_parliament(self, rows: usize) -> Self {
        self.geom_waffle_with(crate::geom::waffle::GeomWaffle::parliament(rows))
    }

    /// A waffle / parliament layer with explicit settings.
    pub fn geom_waffle_with(self, geom: crate::geom::waffle::GeomWaffle) -> Self {
        let mut plot = self.add_geom_with(geom);
        plot.hide_position_guides = true;
        plot
    }

    /// Packed bubble chart: circles with area ∝ `weight`, packed without
    /// overlap; with `subgroup` they cluster per group (outlined). Fills by
    /// subgroup when `subgroup` is mapped at plot level and `fill` is not.
    /// The panel is square and the axes and grid are hidden.
    pub fn geom_packed_bubble(self) -> Self {
        self.geom_packed_bubble_with(crate::geom::pack::GeomPackedBubble::default())
    }

    /// [`geom_packed_bubble`](Self::geom_packed_bubble) with explicit styling.
    pub fn geom_packed_bubble_with(self, geom: crate::geom::pack::GeomPackedBubble) -> Self {
        use crate::aes::Aesthetic;
        let fill_by = match (
            self.mapping.get_mapping(&Aesthetic::Fill),
            self.mapping.get_mapping(&Aesthetic::Subgroup),
        ) {
            (None, Some(sub)) => Some(sub.to_string()),
            _ => None,
        };
        let mut plot = self.add_geom_with(geom);
        plot.hide_position_guides = true;
        plot.default_aspect_ratio = Some(1.0);
        if let Some(sub) = fill_by {
            plot = plot.layer_aes(Aes::new().fill(&sub));
        }
        plot
    }

    /// Network graph from an edge list: map `from`, `to` and optionally
    /// `weight`. Force-directed layout (seeded, deterministic), edges with
    /// width ∝ weight, nodes sized by degree and labelled. Square panel,
    /// axes and grid hidden. See [`GeomNetwork`](crate::geom::network::GeomNetwork)
    /// for directed edges, other layouts and colouring by component.
    pub fn geom_network(self) -> Self {
        self.add_network(crate::geom::network::GeomNetwork::default(), false)
    }

    /// [`geom_network`](Self::geom_network) with explicit settings.
    pub fn geom_network_with(self, geom: crate::geom::network::GeomNetwork) -> Self {
        self.add_network(geom, true)
    }

    fn add_network(self, geom: crate::geom::network::GeomNetwork, explicit: bool) -> Self {
        let by_component = geom.color_components;
        let mut plot = if explicit {
            self.add_geom_with(geom)
        } else {
            self.add_geom(geom)
        };
        plot.hide_position_guides = true;
        plot.default_aspect_ratio = Some(1.0);
        if by_component {
            plot = plot
                .layer_aes(Aes::new().after_stat_fill("component"))
                .show_legend(false);
        }
        plot
    }

    /// Tree diagram from parent → child links: map `from` (parent) and `to`
    /// (child). Tidy layout (leaves evenly spaced, parents centred over
    /// their children), top-down with curved links. See
    /// [`GeomTree::org_chart`](crate::geom::tree::GeomTree::org_chart) and
    /// other orientations via [`geom_tree_with`](Self::geom_tree_with).
    /// Axes and grid are hidden.
    pub fn geom_tree(self) -> Self {
        self.geom_tree_with(crate::geom::tree::GeomTree::default())
    }

    /// Dendrogram from [`dendrogram_links`](crate::stat::tree::dendrogram_links)
    /// output (`from`, `to`, `height`): step links at the merge heights,
    /// leaves labelled.
    pub fn geom_dendrogram(self) -> Self {
        self.geom_tree_with(crate::geom::tree::GeomTree::dendrogram())
    }

    /// A tree layer with explicit layout and styling.
    pub fn geom_tree_with(self, geom: crate::geom::tree::GeomTree) -> Self {
        let radial = geom.orientation == crate::stat::tree::TreeOrientation::Radial;
        let mut plot = self.add_geom_with(geom);
        plot.hide_position_guides = true;
        if radial {
            plot.default_aspect_ratio = Some(1.0);
        }
        plot
    }

    /// Area-proportional Venn / Euler diagram: map `sets` (`"A"`, `"B"`,
    /// `"A&B"`, … — venn.js-style inclusive sizes) and `weight` (size).
    /// Exact for two and three sets' pairwise overlaps, approximate beyond.
    /// Circles are filled by set name (no legend — regions are labelled);
    /// square panel, axes and grid hidden.
    pub fn geom_venn(self) -> Self {
        self.geom_venn_with(crate::geom::venn::GeomVenn::default())
    }

    /// [`geom_venn`](Self::geom_venn) with explicit styling.
    pub fn geom_venn_with(self, geom: crate::geom::venn::GeomVenn) -> Self {
        let mut plot = self.add_geom_with(geom);
        plot.hide_position_guides = true;
        plot.default_aspect_ratio = Some(1.0);
        plot.layer_aes(Aes::new().after_stat_fill("set"))
            .show_legend(false)
    }

    /// Word cloud: map `label` (word) and `weight` (frequency); optionally
    /// `color`. Largest words placed first along a spiral from the centre,
    /// no overlaps, deterministic; words that don't fit are dropped. Axes
    /// and grid are hidden.
    pub fn geom_wordcloud(self) -> Self {
        self.geom_wordcloud_with(crate::geom::wordcloud::GeomWordcloud::default())
    }

    /// [`geom_wordcloud`](Self::geom_wordcloud) with explicit sizes,
    /// rotation share, seed and palette.
    pub fn geom_wordcloud_with(self, geom: crate::geom::wordcloud::GeomWordcloud) -> Self {
        let mut plot = self.add_geom_with(geom);
        plot.hide_position_guides = true;
        plot
    }

    /// Variwide chart: map `x` (category), `y` (height) and `weight` (bar
    /// width). Unless an x scale is set, the x axis is continuous over the
    /// column layout with each category named under its column (computed
    /// from the plot data, so map `x`/`weight` at plot level).
    pub fn geom_variwide(self) -> Self {
        self.geom_marimekko_with(crate::geom::marimekko::GeomMarimekko::variwide())
    }

    /// Marimekko (mosaic) chart: map `x` (column), `fill` (segment) and
    /// `weight` (value). Column widths are their totals' shares, segments
    /// their share of the column (y axis 0–100 %).
    pub fn geom_marimekko(self) -> Self {
        self.geom_marimekko_with(crate::geom::marimekko::GeomMarimekko::marimekko())
    }

    /// A variwide / Marimekko layer with explicit settings.
    pub fn geom_marimekko_with(self, geom: crate::geom::marimekko::GeomMarimekko) -> Self {
        use crate::aes::Aesthetic;
        let has = |plot: &Self, aes: Aesthetic| plot.scales.iter().any(|s| s.aesthetic() == aes);
        let (has_x, has_y) = (has(&self, Aesthetic::X), has(&self, Aesthetic::Y));
        // Column centres and names from the plot data (same layout as the stat).
        let axis = self.mapping.get_mapping(&Aesthetic::X).and_then(|xc| {
            let x = self.data.column(xc)?;
            let w = self
                .mapping
                .get_mapping(&Aesthetic::Weight)
                .and_then(|wc| self.data.column(wc));
            let totals = crate::stat::marimekko::column_totals(x, w);
            let widths: Vec<f64> = totals.iter().map(|t| t.1).collect();
            let spans = crate::stat::marimekko::column_spans(&widths, geom.gap);
            Some((
                spans
                    .iter()
                    .map(|s| (s.0 + s.1) / 2.0)
                    .collect::<Vec<f64>>(),
                totals.into_iter().map(|t| t.0).collect::<Vec<String>>(),
            ))
        });
        let mosaic = geom.mosaic;
        let mut plot = self.add_geom_with(geom);
        if !has_x {
            let mut sx = ScaleContinuous::new().with_expand(0.0, 0.0);
            if let Some((breaks, labels)) = axis {
                sx = sx.with_breaks(breaks).with_labels(labels);
            }
            plot = plot.scale_x_continuous(sx);
        }
        if mosaic && !has_y {
            plot = plot.scale_y_continuous(
                ScaleContinuous::new()
                    .with_expand(0.0, 0.0)
                    .with_label_formatter(|v| format!("{}%", (v * 100.0).round())),
            );
        }
        plot
    }

    /// Timeline: map `x` (event time, e.g. a date), `label`, and optionally
    /// `xend` (span end) and `color`. Events sit on a central baseline with
    /// boxed labels stacked on alternating levels so they never overlap;
    /// the y axis and horizontal grid are hidden.
    pub fn geom_timeline(self) -> Self {
        let mut plot = self.add_geom(crate::geom::timeline::GeomTimeline::default());
        plot.hide_y_guides = true;
        plot
    }

    /// [`geom_timeline`](Self::geom_timeline) with explicit styling.
    pub fn geom_timeline_with(self, geom: crate::geom::timeline::GeomTimeline) -> Self {
        let mut plot = self.add_geom_with(geom);
        plot.hide_y_guides = true;
        plot
    }

    /// Candlestick chart: map `x` and `open`/`high`/`low`/`close`
    /// (`Aes::new().x("date").open("o").high("h").low("l").close("c")`).
    pub fn geom_candlestick(self) -> Self {
        self.add_geom(crate::geom::candlestick::GeomCandlestick::default())
    }

    pub fn geom_candlestick_with(self, geom: crate::geom::candlestick::GeomCandlestick) -> Self {
        self.add_geom_with(geom)
    }

    /// OHLC bar chart (high–low bar with open/close ticks); same aesthetics as
    /// [`geom_candlestick`](Self::geom_candlestick).
    pub fn geom_ohlc(self) -> Self {
        self.add_geom(crate::geom::candlestick::GeomOhlc::default())
    }

    pub fn geom_ohlc_with(self, geom: crate::geom::candlestick::GeomOhlc) -> Self {
        self.add_geom_with(geom)
    }

    /// Calendar heatmap (GitHub/ECharts style): the plot's `x` (a date —
    /// `DateTime`, epoch seconds or `"YYYY-MM-DD"`) is laid out as week
    /// columns × weekday rows (Sunday on top) via [`StatCalendar`], coloured
    /// by `fill`, with month labels along x, Mon/Wed/Fri along y, month
    /// boundary outlines and square cells. Spans over
    /// [`MAX_CALENDAR_YEARS`] are clipped to the most recent years (with a
    /// build warning). Uses the plot-level data and `x` mapping.
    ///
    /// [`StatCalendar`]: crate::stat::calendar::StatCalendar
    /// [`MAX_CALENDAR_YEARS`]: crate::stat::calendar::MAX_CALENDAR_YEARS
    pub fn geom_calendar(self) -> Self {
        self.geom_calendar_with(
            GeomTile {
                color: (255, 255, 255),
                line_width: 1.5,
                ..Default::default()
            },
            false,
        )
    }

    /// [`geom_calendar`](Self::geom_calendar) with a custom cell style and
    /// optionally Monday-first weeks.
    pub fn geom_calendar_with(mut self, tile: GeomTile, monday_first: bool) -> Self {
        use crate::stat::calendar::{
            day_number, month_boundaries, month_breaks, month_breaks_localized, weekday_labels,
            CalendarGrid, StatCalendar, MAX_CALENDAR_YEARS,
        };
        let grid = self
            .mapping
            .get_mapping(&crate::aes::Aesthetic::X)
            .and_then(|c| self.data.column(c))
            .and_then(|col| {
                CalendarGrid::from_days(col.iter().filter_map(day_number), monday_first)
            });
        let Some((grid, clipped)) = grid else {
            // No dates: an empty calendar layer (renders an empty panel).
            return self.add_geom_with(tile).stat(StatCalendar {
                grid: None,
                monday_first,
            });
        };
        if clipped {
            self.warnings.push(format!(
                "geom_calendar: dates more than {MAX_CALENDAR_YEARS} years before the newest were dropped"
            ));
        }
        let (xb, _) = month_breaks(&grid);
        // Rows: y = 6 - weekday; label Mon/Wed/Fri.
        let yb = if monday_first {
            vec![6.0, 4.0, 2.0]
        } else {
            vec![5.0, 3.0, 1.0]
        };
        let segs = month_boundaries(&grid);
        let col = |f: fn(&(f64, f64, f64, f64)) -> f64| -> Vec<Value> {
            segs.iter().map(|s| Value::Float(f(s))).collect()
        };
        let boundaries = vec![
            ("x".to_string(), col(|s| s.0)),
            ("y".to_string(), col(|s| s.1)),
            ("xend".to_string(), col(|s| s.2)),
            ("yend".to_string(), col(|s| s.3)),
        ];
        self.default_aspect_ratio = Some(7.0 / grid.n_weeks().max(1) as f64);
        let plot = self
            .add_geom_with(tile)
            .stat(StatCalendar {
                grid: Some(grid),
                monday_first,
            })
            .scale_x_continuous(
                ScaleContinuous::new()
                    .with_breaks(xb)
                    // Month / weekday names follow the plot locale.
                    .with_localized_labels(move |l| month_breaks_localized(&grid, l).1)
                    .with_expand(0.0, 0.0),
            )
            .scale_y_continuous(
                ScaleContinuous::new()
                    .with_breaks(yb)
                    .with_localized_labels(weekday_labels)
                    .with_expand(0.0, 0.0),
            );
        if segs.is_empty() {
            return plot;
        }
        // Styled by the theme's `calendar_month_border` at draw time.
        plot.add_geom_with(crate::geom::segment::GeomCalendarBorder)
            .layer_data(boundaries)
            .layer_aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
            .show_legend(false)
    }

    pub fn geom_blank(self) -> Self {
        self.add_geom(GeomBlank)
    }

    /// Add a geom configured explicitly by the caller: its colours win over
    /// the theme's `primary_color`.
    fn add_geom_with(self, geom: impl Geom + 'static) -> Self {
        let mut plot = self.add_geom(geom);
        if let Some(layer) = plot.layers.last_mut() {
            layer.explicit_style = true;
        }
        plot
    }

    fn add_geom(mut self, geom: impl Geom + 'static) -> Self {
        let stat = geom.default_stat();
        let position = geom.default_position();
        let params = geom.default_params();
        self.layers.push(Layer {
            data: None,
            mapping: Aes::default(),
            geom: Box::new(geom),
            stat,
            position,
            params,
            show_legend: None,
            explicit_style: false,
            tooltip_template: None,
        });
        self
    }

    // ─── Layer-level overrides ──────────────────────────────────

    /// Override the stat for the most recently added layer.
    pub fn stat(mut self, stat: impl Stat + 'static) -> Self {
        if let Some(layer) = self.layers.last_mut() {
            layer.stat = Box::new(stat);
        }
        self
    }

    /// Override the position for the most recently added layer.
    pub fn position(mut self, pos: impl Position + 'static) -> Self {
        if let Some(layer) = self.layers.last_mut() {
            layer.position = Box::new(pos);
        }
        self
    }

    /// Override the data for the most recently added layer.
    pub fn layer_data(mut self, data: impl GGData) -> Self {
        if let Some(layer) = self.layers.last_mut() {
            layer.data = Some(data.into_dataframe());
        }
        self
    }

    /// Override the aesthetic mapping for the most recently added layer.
    pub fn layer_aes(mut self, mapping: Aes) -> Self {
        self.warnings.extend(mapping.warnings.iter().cloned());
        if let Some(layer) = self.layers.last_mut() {
            layer.mapping = mapping;
        }
        self
    }

    /// A `{column}` tooltip template for the most recently added layer
    /// (B2-5), replacing its default `<title>`. Each placeholder is filled
    /// from the *built* layer data (so it can reference a stat-computed
    /// column, e.g. a hex's `count`) at that row, formatted through the
    /// column's scale when it has one
    /// ([`Scale::format_value`](crate::scale::Scale::format_value)) — the same
    /// `data-fvalue` logic, but written into the tooltip text. A column
    /// that doesn't exist on this layer renders as empty, not literally
    /// `{column}`. `{{`/`}}` are literal braces. The result is escaped the
    /// same way any other tooltip is — this is plain text, not markup.
    pub fn tooltip(mut self, template: &str) -> Self {
        if let Some(layer) = self.layers.last_mut() {
            layer.tooltip_template = Some(template.to_string());
        }
        self
    }

    /// Control whether the most recently added layer contributes to the legend.
    /// `true` = always show, `false` = always hide, default (None) = auto.
    pub fn show_legend(mut self, show: bool) -> Self {
        if let Some(layer) = self.layers.last_mut() {
            layer.show_legend = Some(show);
        }
        self
    }

    /// Mark the most recently added layer as purely decorative (#161): its
    /// drawn marks keep their own visual style but carry **no** per-mark
    /// metadata — no `<title>`, no `data-x`/`data-series`/`data-value`/
    /// `data-fvalue`, and no [`data_attr`](crate::aes::Aes::data_attr)
    /// attributes — so an SVG consumer that treats "has a non-empty
    /// `<title>`" as "is a selectable data point" doesn't pick it up. Meant
    /// for a layer that annotates another layer's mark rather than drawing
    /// data in its own right (e.g. a boxplot's own mean dot, drawn by a
    /// separate `geom_point` layer purely to mark the spot the box's own
    /// tooltip already names). Each mark also carries
    /// `data-part="decorative"` and `aria-hidden="true"` (`data-geom`
    /// still names the real geom), so a consumer can select decorative
    /// marks positively rather than only infer them from what they lack.
    /// Their class is `gg-mark gg-decorative`, not the geom's own
    /// (`gg-point`, `gg-line`, …).
    ///
    /// A `.tooltip(...)` template on a decorative layer has no visible
    /// effect — the geom's marks carry no tooltip regardless — and warns,
    /// like the existing has-no-effect warning for a geom that doesn't
    /// read tooltips at all.
    ///
    /// A decorative layer also defaults to hidden in the legend (it's
    /// annotating, not its own series), unless
    /// [`show_legend`](Self::show_legend)`(true)` is called — before or
    /// after `decorative()` — to opt back in; an explicit
    /// `show_legend(false)` is left untouched either way.
    pub fn decorative(mut self) -> Self {
        if let Some(layer) = self.layers.last_mut() {
            let geom = std::mem::replace(&mut layer.geom, Box::new(GeomBlank));
            layer.geom = Box::new(crate::geom::decorative::Decorative(geom));
            if layer.show_legend.is_none() {
                layer.show_legend = Some(false);
            }
        }
        self
    }

    /// Add one SVG attribute to **every mark** of the most recently added
    /// layer (#221) — e.g. to make a label layer drawn over node boxes inert,
    /// so hover/click and screen readers pass through to the box below:
    ///
    /// ```
    /// # use ggplot_rs::{aes::Aes, data::Value, GGPlot};
    /// # let data = vec![("x".to_string(), vec![Value::Float(1.0)]),
    /// #   ("y".to_string(), vec![Value::Float(1.0)]),
    /// #   ("l".to_string(), vec![Value::Str("a".into())])];
    /// let svg = GGPlot::new(data)
    ///     .aes(Aes::new().x("x").y("y").label("l"))
    ///     .geom_text()
    ///     .layer_svg_attr("aria-hidden", "true")
    ///     .layer_svg_attr("pointer-events", "none")
    ///     .layer_svg_attr("class", "node-label")
    ///     .render_svg_native_with_size(200, 200)
    ///     .unwrap();
    /// assert!(svg.contains(r#"aria-hidden="true" pointer-events="none""#));
    /// ```
    ///
    /// The allow-list is deliberately small: `class` (space-separated
    /// tokens, each `[A-Za-z][A-Za-z0-9_-]*`, appended to the mark's own
    /// `gg-mark gg-<kind>` class; invalid tokens are dropped), `aria-hidden`
    /// (`"true"`/`"false"`), `pointer-events` (an SVG keyword such as
    /// `"none"`), and `data-*` (`Aes::data_attr`'s name rules; a row's own
    /// `data_attr` of the same name wins). Anything else — in particular
    /// `style` and `on*` handlers, so a strict CSP without `unsafe-inline`
    /// still holds — is ignored with a build warning, as is a bad value or a
    /// call before any layer. A later call with the same name replaces the
    /// earlier value. Values are escaped on output. For a per-row class use
    /// [`Aes::class`](crate::aes::Aes::class); to wrap several layers' marks
    /// per key in one `<g>`, [`Aes::group_wrap`](crate::aes::Aes::group_wrap).
    /// SVG backend only.
    pub fn layer_svg_attr(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let (name, value) = (name.into(), value.into());
        let Some(idx) = self.layers.len().checked_sub(1) else {
            self.warnings.push(format!(
                "layer_svg_attr(\"{name}\") called before any layer was added — ignored"
            ));
            return self;
        };
        if let Some((n, v)) =
            crate::render::svg_backend::validate_layer_svg_attr(&name, &value, &mut self.warnings)
        {
            self.layer_svg_attrs
                .retain(|(i, k, _)| !(*i == idx && *k == n));
            self.layer_svg_attrs.push((idx, n, v));
        }
        self
    }

    /// [`layer_svg_attr`](Self::layer_svg_attr) for several attributes at once.
    pub fn layer_svg_attrs<N: Into<String>, V: Into<String>>(
        self,
        attrs: impl IntoIterator<Item = (N, V)>,
    ) -> Self {
        attrs
            .into_iter()
            .fold(self, |p, (n, v)| p.layer_svg_attr(n, v))
    }

    // ─── Scales ──────────────────────────────────────────────────

    pub fn scale_x_continuous(mut self, s: ScaleContinuous) -> Self {
        let s = s.for_aesthetic(crate::aes::Aesthetic::X);
        self.scales.push(Box::new(s));
        self
    }

    pub fn scale_y_continuous(mut self, s: ScaleContinuous) -> Self {
        let s = s.for_aesthetic(crate::aes::Aesthetic::Y);
        self.scales.push(Box::new(s));
        self
    }

    pub fn scale_x_discrete(mut self, s: crate::scale::discrete::ScaleDiscrete) -> Self {
        let s = s.for_aesthetic(crate::aes::Aesthetic::X);
        self.scales.push(Box::new(s));
        self
    }

    pub fn scale_y_discrete(mut self, s: crate::scale::discrete::ScaleDiscrete) -> Self {
        let s = s.for_aesthetic(crate::aes::Aesthetic::Y);
        self.scales.push(Box::new(s));
        self
    }

    pub fn scale_color(mut self, s: impl Scale + 'static) -> Self {
        self.scales.push(Box::new(s));
        self
    }

    pub fn scale_fill(mut self, s: impl Scale + 'static) -> Self {
        self.scales.push(Box::new(s));
        self
    }

    pub fn scale_color_manual(self, values: Vec<(&str, crate::scale::color::RGBAColor)>) -> Self {
        let s = crate::scale::manual::ScaleManual::new(crate::aes::Aesthetic::Color, values);
        self.scale_color(s)
    }

    pub fn scale_fill_manual(self, values: Vec<(&str, crate::scale::color::RGBAColor)>) -> Self {
        let s = crate::scale::manual::ScaleManual::new(crate::aes::Aesthetic::Fill, values);
        self.scale_fill(s)
    }

    /// The `colour`/`color` column already holds `#RRGGBB`/`#RGB` hex
    /// strings, used as-is — no legend (R's `scale_colour_identity`).
    pub fn scale_colour_identity(self) -> Self {
        let s = crate::scale::identity::ScaleColorIdentity::new(crate::aes::Aesthetic::Color);
        self.scale_color(s)
    }

    /// American-spelling alias for [`scale_colour_identity`](Self::scale_colour_identity).
    pub fn scale_color_identity(self) -> Self {
        self.scale_colour_identity()
    }

    /// The `fill` column already holds `#RRGGBB`/`#RGB` hex strings, used
    /// as-is — no legend (R's `scale_fill_identity`).
    pub fn scale_fill_identity(self) -> Self {
        let s = crate::scale::identity::ScaleColorIdentity::new(crate::aes::Aesthetic::Fill);
        self.scale_fill(s)
    }

    pub fn scale_color_viridis(self) -> Self {
        use crate::scale::color::ScaleColorDiscrete;
        use crate::scale::palettes::PaletteName;
        let s = ScaleColorDiscrete::new(crate::aes::Aesthetic::Color)
            .with_named_palette(&PaletteName::Viridis);
        self.scale_color(s)
    }

    pub fn scale_color_brewer(self, name: crate::scale::palettes::PaletteName) -> Self {
        use crate::scale::color::ScaleColorDiscrete;
        let s = ScaleColorDiscrete::new(crate::aes::Aesthetic::Color).with_named_palette(&name);
        self.scale_color(s)
    }

    pub fn scale_color_gradient(
        self,
        low: crate::scale::color::RGBAColor,
        high: crate::scale::color::RGBAColor,
    ) -> Self {
        use crate::scale::color::ScaleColorContinuous;
        let s = ScaleColorContinuous::new(crate::aes::Aesthetic::Color).with_colors(low, high);
        self.scale_color(s)
    }

    pub fn scale_fill_gradient(
        self,
        low: crate::scale::color::RGBAColor,
        high: crate::scale::color::RGBAColor,
    ) -> Self {
        use crate::scale::color::ScaleColorContinuous;
        let s = ScaleColorContinuous::new(crate::aes::Aesthetic::Fill).with_colors(low, high);
        self.scale_fill(s)
    }

    pub fn scale_color_gradient2(
        self,
        low: crate::scale::color::RGBAColor,
        mid: crate::scale::color::RGBAColor,
        high: crate::scale::color::RGBAColor,
    ) -> Self {
        use crate::scale::gradient::ScaleColorGradient2;
        let s = ScaleColorGradient2::new(crate::aes::Aesthetic::Color).with_colors(low, mid, high);
        self.scale_color(s)
    }

    pub fn scale_fill_gradient2(
        self,
        low: crate::scale::color::RGBAColor,
        mid: crate::scale::color::RGBAColor,
        high: crate::scale::color::RGBAColor,
    ) -> Self {
        use crate::scale::gradient::ScaleColorGradient2;
        let s = ScaleColorGradient2::new(crate::aes::Aesthetic::Fill).with_colors(low, mid, high);
        self.scale_fill(s)
    }

    pub fn scale_fill_viridis(self) -> Self {
        use crate::scale::color::ScaleColorDiscrete;
        use crate::scale::palettes::PaletteName;
        let s = ScaleColorDiscrete::new(crate::aes::Aesthetic::Fill)
            .with_named_palette(&PaletteName::Viridis);
        self.scale_fill(s)
    }

    /// Continuous viridis color scale (for numeric data).
    pub fn scale_color_viridis_c(self) -> Self {
        use crate::scale::gradient_n::ScaleColorGradientN;
        let s = ScaleColorGradientN::viridis(crate::aes::Aesthetic::Color);
        self.scale_color(s)
    }

    /// Continuous viridis fill scale (for numeric data).
    pub fn scale_fill_viridis_c(self) -> Self {
        use crate::scale::gradient_n::ScaleColorGradientN;
        let s = ScaleColorGradientN::viridis(crate::aes::Aesthetic::Fill);
        self.scale_fill(s)
    }

    /// N-stop continuous color gradient.
    pub fn scale_color_gradientn(self, stops: Vec<(f64, crate::scale::color::RGBAColor)>) -> Self {
        use crate::scale::gradient_n::ScaleColorGradientN;
        let s = ScaleColorGradientN::new(crate::aes::Aesthetic::Color, stops);
        self.scale_color(s)
    }

    /// N-stop continuous fill gradient.
    pub fn scale_fill_gradientn(self, stops: Vec<(f64, crate::scale::color::RGBAColor)>) -> Self {
        use crate::scale::gradient_n::ScaleColorGradientN;
        let s = ScaleColorGradientN::new(crate::aes::Aesthetic::Fill, stops);
        self.scale_fill(s)
    }

    /// Binned (stepped) two-colour continuous colour scale — buckets the mapped
    /// variable into `n_bins` bins, each a discrete colour, with a stepped legend.
    pub fn scale_color_steps(
        self,
        low: crate::scale::color::RGBAColor,
        high: crate::scale::color::RGBAColor,
        n_bins: usize,
    ) -> Self {
        let s = crate::scale::steps::ScaleColorSteps::two(
            crate::aes::Aesthetic::Color,
            (low.r, low.g, low.b),
            (high.r, high.g, high.b),
            n_bins,
        );
        self.scale_color(s)
    }

    /// Binned N-stop continuous colour scale.
    pub fn scale_color_stepsn(
        self,
        stops: Vec<crate::scale::color::RGBAColor>,
        n_bins: usize,
    ) -> Self {
        let s = crate::scale::steps::ScaleColorSteps::new(
            crate::aes::Aesthetic::Color,
            stops.iter().map(|c| (c.r, c.g, c.b)).collect(),
            n_bins,
        );
        self.scale_color(s)
    }

    /// Binned ColorBrewer colour scale (R's `scale_color_fermenter`).
    pub fn scale_color_fermenter(
        self,
        name: crate::scale::palettes::PaletteName,
        n_bins: usize,
    ) -> Self {
        let stops = crate::scale::palettes::palette(&name)
            .iter()
            .map(|c| (c.r, c.g, c.b))
            .collect();
        let s =
            crate::scale::steps::ScaleColorSteps::new(crate::aes::Aesthetic::Color, stops, n_bins);
        self.scale_color(s)
    }

    /// Binned (stepped) two-colour continuous fill scale.
    pub fn scale_fill_steps(
        self,
        low: crate::scale::color::RGBAColor,
        high: crate::scale::color::RGBAColor,
        n_bins: usize,
    ) -> Self {
        let s = crate::scale::steps::ScaleColorSteps::two(
            crate::aes::Aesthetic::Fill,
            (low.r, low.g, low.b),
            (high.r, high.g, high.b),
            n_bins,
        );
        self.scale_fill(s)
    }

    /// Binned fill scale with *explicit* class breaks (`n + 1` increasing
    /// edges, `n` colours interpolated `low`→`high`) — R's
    /// `scale_fill_steps(breaks = ...)`, backing a `::CLASSES` choropleth
    /// (sipemu/ggplot-rs#180). The legend shows each class's trained count
    /// (`"10–50 (12)"`) and, when any value is `Value::Na`, a "no data" key
    /// in `na_color` (default grey50) — see
    /// [`ScaleFillBinned`](crate::scale::binned::ScaleFillBinned) for the
    /// `.na_color(..)`/`.na_label(..)` builders. The break maths itself
    /// (quantile/Jenks/equal-width/custom edges) is the caller's job; this
    /// only bins against the edges it's given.
    pub fn scale_fill_binned(
        self,
        breaks: Vec<f64>,
        low: crate::scale::color::RGBAColor,
        high: crate::scale::color::RGBAColor,
    ) -> Self {
        let s = crate::scale::binned::ScaleFillBinned::two(
            crate::aes::Aesthetic::Fill,
            breaks,
            (low.r, low.g, low.b),
            (high.r, high.g, high.b),
        );
        self.scale_fill(s)
    }

    /// Like [`scale_fill_binned`](Self::scale_fill_binned), with an
    /// explicit colour per bin (`colors.len()` should be
    /// `breaks.len() - 1`) instead of a two-colour interpolation.
    pub fn scale_fill_binned_with(
        self,
        breaks: Vec<f64>,
        colors: Vec<crate::scale::color::RGBAColor>,
    ) -> Self {
        let s = crate::scale::binned::ScaleFillBinned::new(
            crate::aes::Aesthetic::Fill,
            breaks,
            colors.iter().map(|c| (c.r, c.g, c.b)).collect(),
        );
        self.scale_fill(s)
    }

    /// Binned ColorBrewer fill scale.
    pub fn scale_fill_fermenter(
        self,
        name: crate::scale::palettes::PaletteName,
        n_bins: usize,
    ) -> Self {
        let stops = crate::scale::palettes::palette(&name)
            .iter()
            .map(|c| (c.r, c.g, c.b))
            .collect();
        let s =
            crate::scale::steps::ScaleColorSteps::new(crate::aes::Aesthetic::Fill, stops, n_bins);
        self.scale_fill(s)
    }

    /// Default discrete fill palette with levels in sorted order, so a level
    /// keeps its colour across charts regardless of data order. For a custom
    /// palette: `scale_fill(ScaleColorDiscrete::new(Aesthetic::Fill)
    /// .with_palette(p).sorted())`.
    pub fn scale_fill_discrete_sorted(self) -> Self {
        use crate::scale::color::ScaleColorDiscrete;
        self.scale_fill(ScaleColorDiscrete::new(crate::aes::Aesthetic::Fill).sorted())
    }

    /// Default discrete colour palette with levels in sorted order (see
    /// [`scale_fill_discrete_sorted`](Self::scale_fill_discrete_sorted)).
    pub fn scale_color_discrete_sorted(self) -> Self {
        use crate::scale::color::ScaleColorDiscrete;
        self.scale_color(ScaleColorDiscrete::new(crate::aes::Aesthetic::Color).sorted())
    }

    pub fn scale_fill_brewer(self, name: crate::scale::palettes::PaletteName) -> Self {
        use crate::scale::color::ScaleColorDiscrete;
        let s = ScaleColorDiscrete::new(crate::aes::Aesthetic::Fill).with_named_palette(&name);
        self.scale_fill(s)
    }

    pub fn scale_linetype_manual(
        self,
        values: Vec<(&str, crate::render::backend::Linetype)>,
    ) -> Self {
        let s = crate::scale::linetype_manual::ScaleLinetypeManual::new(values);
        self.scale_color(s)
    }

    pub fn scale_shape_manual(
        self,
        values: Vec<(&str, crate::render::backend::PointShape)>,
    ) -> Self {
        let s = crate::scale::shape_manual::ScaleShapeManual::new(values);
        self.scale_color(s)
    }

    pub fn scale_color_grey(self) -> Self {
        let s = crate::scale::grey::ScaleColorGrey::new(crate::aes::Aesthetic::Color);
        self.scale_color(s)
    }

    pub fn scale_fill_grey(self) -> Self {
        let s = crate::scale::grey::ScaleColorGrey::new(crate::aes::Aesthetic::Fill);
        self.scale_fill(s)
    }

    pub fn scale_color_grey_with(self, s: crate::scale::grey::ScaleColorGrey) -> Self {
        self.scale_color(s)
    }

    pub fn scale_fill_grey_with(self, s: crate::scale::grey::ScaleColorGrey) -> Self {
        self.scale_fill(s)
    }

    pub fn scale_x_reverse(self) -> Self {
        self.scale_x_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Reverse))
    }

    pub fn scale_y_reverse(self) -> Self {
        self.scale_y_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Reverse))
    }

    pub fn scale_x_datetime(mut self, s: crate::scale::datetime::ScaleDateTime) -> Self {
        let s = s.for_aesthetic(crate::aes::Aesthetic::X);
        self.scales.push(Box::new(s));
        self
    }

    pub fn scale_y_datetime(mut self, s: crate::scale::datetime::ScaleDateTime) -> Self {
        let s = s.for_aesthetic(crate::aes::Aesthetic::Y);
        self.scales.push(Box::new(s));
        self
    }

    pub fn scale_size(mut self, s: crate::scale::size::ScaleSizeContinuous) -> Self {
        self.scales.push(Box::new(s));
        self
    }

    /// `scale_size_area(max_size)` (ggplot2, B3-6): size scale where
    /// *area*, not radius, is proportional to the value, and 0 maps to 0.
    /// See [`ScaleSizeContinuous::area`](crate::scale::size::ScaleSizeContinuous::area).
    pub fn scale_size_area(mut self, max_size: f64) -> Self {
        self.scales.push(Box::new(
            crate::scale::size::ScaleSizeContinuous::new().area(max_size),
        ));
        self
    }

    pub fn scale_alpha(mut self, s: crate::scale::alpha::ScaleAlphaContinuous) -> Self {
        self.scales.push(Box::new(s));
        self
    }

    /// Continuous linewidth scale for the `linewidth` aesthetic (segment,
    /// curve, path, line) — see
    /// [`ScaleLinewidthContinuous`](crate::scale::linewidth::ScaleLinewidthContinuous).
    pub fn scale_linewidth(mut self, s: crate::scale::linewidth::ScaleLinewidthContinuous) -> Self {
        self.scales.push(Box::new(s));
        self
    }

    pub fn xlim(self, min: f64, max: f64) -> Self {
        self.scale_x_continuous(ScaleContinuous::new().with_limits(min, max))
    }

    pub fn ylim(self, min: f64, max: f64) -> Self {
        self.scale_y_continuous(ScaleContinuous::new().with_limits(min, max))
    }

    pub fn scale_x_log10(self) -> Self {
        self.scale_x_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Log10))
    }

    pub fn scale_y_log10(self) -> Self {
        self.scale_y_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Log10))
    }

    pub fn scale_x_sqrt(self) -> Self {
        self.scale_x_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Sqrt))
    }

    pub fn scale_y_sqrt(self) -> Self {
        self.scale_y_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Sqrt))
    }

    pub fn scale_x_log2(self) -> Self {
        self.scale_x_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Log2))
    }

    pub fn scale_y_log2(self) -> Self {
        self.scale_y_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Log2))
    }

    pub fn scale_x_ln(self) -> Self {
        self.scale_x_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Ln))
    }

    pub fn scale_y_ln(self) -> Self {
        self.scale_y_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Ln))
    }

    /// Logit-transformed x axis (for proportions in (0, 1)).
    pub fn scale_x_logit(self) -> Self {
        self.scale_x_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Logit))
    }

    pub fn scale_y_logit(self) -> Self {
        self.scale_y_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Logit))
    }

    /// Probit-transformed x axis (inverse normal CDF, for proportions in (0, 1)).
    pub fn scale_x_probit(self) -> Self {
        self.scale_x_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Probit))
    }

    pub fn scale_y_probit(self) -> Self {
        self.scale_y_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Probit))
    }

    /// Sign-preserving pseudo-log x axis (handles zero and negative values).
    pub fn scale_x_pseudo_log(self) -> Self {
        self.scale_x_continuous(ScaleContinuous::new().with_transform(ScaleTransform::PseudoLog))
    }

    pub fn scale_y_pseudo_log(self) -> Self {
        self.scale_y_continuous(ScaleContinuous::new().with_transform(ScaleTransform::PseudoLog))
    }

    /// Reciprocal (1/x) x axis.
    pub fn scale_x_reciprocal(self) -> Self {
        self.scale_x_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Reciprocal))
    }

    pub fn scale_y_reciprocal(self) -> Self {
        self.scale_y_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Reciprocal))
    }

    /// Exponential x axis (labels spaced logarithmically).
    pub fn scale_x_exp(self) -> Self {
        self.scale_x_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Exp))
    }

    pub fn scale_y_exp(self) -> Self {
        self.scale_y_continuous(ScaleContinuous::new().with_transform(ScaleTransform::Exp))
    }

    /// Box–Cox x axis with the given lambda (x > 0).
    pub fn scale_x_boxcox(self, lambda: f64) -> Self {
        self.scale_x_continuous(
            ScaleContinuous::new().with_transform(ScaleTransform::BoxCox(lambda)),
        )
    }

    pub fn scale_y_boxcox(self, lambda: f64) -> Self {
        self.scale_y_continuous(
            ScaleContinuous::new().with_transform(ScaleTransform::BoxCox(lambda)),
        )
    }

    // ─── Faceting ─────────────────────────────────────────────────

    pub fn facet_wrap(mut self, var: &str, ncol: Option<usize>) -> Self {
        self.facet = Facet::Wrap {
            var: var.to_string(),
            ncol,
            scales: FacetScales::Fixed,
            labeller: FacetLabeller::default(),
        };
        self
    }

    pub fn facet_wrap_free(mut self, var: &str, ncol: Option<usize>, scales: FacetScales) -> Self {
        self.facet = Facet::Wrap {
            var: var.to_string(),
            ncol,
            scales,
            labeller: FacetLabeller::default(),
        };
        self
    }

    pub fn facet_wrap_labeller(
        mut self,
        var: &str,
        ncol: Option<usize>,
        labeller: FacetLabeller,
    ) -> Self {
        self.facet = Facet::Wrap {
            var: var.to_string(),
            ncol,
            scales: FacetScales::Fixed,
            labeller,
        };
        self
    }

    pub fn facet_grid(mut self, row: Option<&str>, col: Option<&str>) -> Self {
        self.facet = Facet::Grid {
            row_var: row.map(String::from),
            col_var: col.map(String::from),
            scales: FacetScales::Fixed,
            labeller: FacetLabeller::default(),
            space: FacetSpace::Fixed,
        };
        self
    }

    pub fn facet_grid_free(
        mut self,
        row: Option<&str>,
        col: Option<&str>,
        scales: FacetScales,
    ) -> Self {
        self.facet = Facet::Grid {
            row_var: row.map(String::from),
            col_var: col.map(String::from),
            scales,
            labeller: FacetLabeller::default(),
            space: FacetSpace::Fixed,
        };
        self
    }

    /// `facet_grid` over **multiple** column variables (R's `rows ~ b + c`):
    /// the columns become the combination of the given variables' values.
    /// Also accepts free scales + proportional `space` for full parity.
    pub fn facet_grid_multi(
        mut self,
        row: Option<&str>,
        cols: &[&str],
        scales: FacetScales,
        space: FacetSpace,
    ) -> Self {
        let col_var = if cols.len() > 1 {
            // Build a synthetic column that is the interaction of the col vars.
            let n = self.data.nrows();
            let mut combined = Vec::with_capacity(n);
            for i in 0..n {
                let parts: Vec<String> = cols
                    .iter()
                    .map(|c| {
                        self.data
                            .column(c)
                            .and_then(|col| col.get(i))
                            .map(|v| v.to_group_key())
                            .unwrap_or_default()
                    })
                    .collect();
                combined.push(crate::data::Value::Str(parts.join(" . ")));
            }
            let name = "__facet_cols__".to_string();
            self.data.add_column(name.clone(), combined);
            Some(name)
        } else {
            cols.first().map(|s| s.to_string())
        };
        self.facet = Facet::Grid {
            row_var: row.map(String::from),
            col_var,
            scales,
            labeller: FacetLabeller::default(),
            space,
        };
        self
    }

    /// `facet_grid` with proportional panel sizing (R's `space =`): panels are
    /// sized to their data range. Typically paired with free scales.
    pub fn facet_grid_space(
        mut self,
        row: Option<&str>,
        col: Option<&str>,
        scales: FacetScales,
        space: FacetSpace,
    ) -> Self {
        self.facet = Facet::Grid {
            row_var: row.map(String::from),
            col_var: col.map(String::from),
            scales,
            labeller: FacetLabeller::default(),
            space,
        };
        self
    }

    pub fn facet_grid_labeller(
        mut self,
        row: Option<&str>,
        col: Option<&str>,
        labeller: FacetLabeller,
    ) -> Self {
        self.facet = Facet::Grid {
            row_var: row.map(String::from),
            col_var: col.map(String::from),
            scales: FacetScales::Fixed,
            labeller,
            space: FacetSpace::Fixed,
        };
        self
    }

    // ─── Coordinates ─────────────────────────────────────────────

    pub fn coord_flip(mut self) -> Self {
        self.coord = Box::new(CoordFlip);
        self
    }

    pub fn coord_fixed(mut self, ratio: f64) -> Self {
        self.coord = Box::new(CoordFixed::new(ratio));
        self
    }

    /// Spatial coordinate system (feature `sf`): equal aspect derived from the
    /// data extent, so projected geometry keeps its shape. Pair with a `geom_sf`
    /// projection (e.g. Mercator) for a conformal map.
    #[cfg(feature = "sf")]
    pub fn coord_sf(mut self) -> Self {
        self.coord = Box::new(crate::coord::sf::CoordSf::new());
        self
    }

    /// Spatial coordinate system with explicit settings (`xlim`/`ylim`/
    /// `expand`) — see [`CoordSf`](crate::coord::sf::CoordSf). Used by
    /// consumers overlaying a fragment on their own projected basemap: a
    /// fixed pixel window via `xlim`/`ylim` and `expand(false)`.
    #[cfg(feature = "sf")]
    pub fn coord_sf_with(mut self, coord: crate::coord::sf::CoordSf) -> Self {
        self.coord = Box::new(coord);
        self
    }

    /// Whether the rendered root `<svg>` carries `data-plot`/`data-domain`/
    /// `data-xdomain`/… (see [`root_data_attrs`](crate::render::svg_backend::root_data_attrs)).
    /// Default `true`. A consumer that overlays a rendered fragment
    /// (`render_svg_native_at`) on its own basemap — which already has its
    /// own root attributes and window — turns this off so the fragment adds
    /// none of its own.
    pub fn svg_root_data(mut self, on: bool) -> Self {
        self.svg_root_data = on;
        self
    }

    /// Add one extra attribute to the root `<svg>` element — a host
    /// identifier, an ARIA label, a CSS hook, and so on. Allowed names:
    /// `data-*`, `aria-*`, `role`, `id`, `class`, `lang`; anything else, or
    /// a name ggplot-rs already writes to the root (`data-plot`,
    /// `data-domain`, …, or `data-warnings` when
    /// [`svg_warnings_attr`](Self::svg_warnings_attr) is also on), is
    /// ignored with a build warning instead of being silently dropped or
    /// duplicated. Values are escaped on output. PlotGrid has the same API
    /// ([`PlotGrid::svg_attr`](crate::compose::PlotGrid::svg_attr)).
    pub fn svg_attr(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.root_attrs.push((name.into(), value.into()));
        self
    }

    /// [`svg_attr`](Self::svg_attr) for several attributes at once.
    pub fn svg_attrs(mut self, attrs: impl IntoIterator<Item = (String, String)>) -> Self {
        self.root_attrs.extend(attrs);
        self
    }

    /// Emit a `data-warnings` attribute on the root `<svg>` — a JSON array
    /// of the build warnings (rows dropped for non-finite positions, an
    /// `svg_attr` name rejected, …), `[]` when there are none. Default off,
    /// and omitted entirely when off (byte-identical to not calling this).
    pub fn svg_warnings_attr(mut self, on: bool) -> Self {
        self.svg_warnings_attr = on;
        self
    }

    /// Emit `data-fvalue`/`data-fx` on marks: the value aesthetic's raw
    /// data value (y for most geoms, fill for hex/tile) and the x value,
    /// each formatted through its scale (currency/locale numbers, ISO or
    /// locale dates — [`Scale::format_value`](crate::scale::Scale::format_value))
    /// rather than the plain display string `data-value`/`data-x` already
    /// carry. `data-fx` is omitted when it would just duplicate `data-x`.
    /// Default off, and omitted entirely when off (byte-identical to not
    /// calling this).
    pub fn svg_formatted_values(mut self, on: bool) -> Self {
        self.svg_formatted_values = on;
        self
    }

    /// Give the rendered root `<svg>` an accessible name: a `<title>`
    /// written (escaped) as its first child, plus `role="img"` and
    /// `aria-labelledby` pointing at that `<title>`'s id — the SVG
    /// accessibility practice of naming an image via `role="img"` +
    /// `aria-labelledby`/`-describedby` rather than a bare `<title>`,
    /// which some screen readers treat as a tooltip instead of a name.
    /// Pairs with [`svg_desc`](Self::svg_desc) for a longer description;
    /// either may be set alone. Default off (`None`), so output stays
    /// byte-identical when this is never called.
    pub fn svg_title(mut self, title: impl Into<String>) -> Self {
        self.svg_title = Some(title.into());
        self
    }

    /// Give the rendered root `<svg>` an accessible description: a
    /// `<desc>` written (escaped) as a first child (after `<title>`, if
    /// [`svg_title`](Self::svg_title) is also set), plus `role="img"` and
    /// `aria-describedby` pointing at that `<desc>`'s id — e.g. a data
    /// summary like "4 regions; highest Bavaria (5.2); 1 without data".
    /// Default off (`None`), so output stays byte-identical when this is
    /// never called.
    pub fn svg_desc(mut self, desc: impl Into<String>) -> Self {
        self.svg_desc = Some(desc.into());
        self
    }

    /// Transform the coordinate space at draw time (R's `coord_trans`) — stats are
    /// computed on raw data but drawn on non-linear axes. Pass a per-axis
    /// [`ScaleTransform`] (e.g. `Some(ScaleTransform::Log10)`), `None` to leave an
    /// axis linear.
    pub fn coord_trans(mut self, x: Option<ScaleTransform>, y: Option<ScaleTransform>) -> Self {
        self.coord = Box::new(crate::coord::trans::CoordTrans::new(x, y));
        self
    }

    /// `coord_trans` on the y-axis only.
    pub fn coord_trans_y(self, y: ScaleTransform) -> Self {
        self.coord_trans(None, Some(y))
    }

    /// `coord_trans` on the x-axis only.
    pub fn coord_trans_x(self, x: ScaleTransform) -> Self {
        self.coord_trans(Some(x), None)
    }

    /// Zoom into a region without filtering data (unlike xlim/ylim which filter).
    pub fn coord_cartesian_zoom(
        mut self,
        xlim: Option<(f64, f64)>,
        ylim: Option<(f64, f64)>,
    ) -> Self {
        let mut c = CoordCartesian::new();
        if let Some((min, max)) = xlim {
            c = c.xlim(min, max);
        }
        if let Some((min, max)) = ylim {
            c = c.ylim(min, max);
        }
        self.coord = Box::new(c);
        self
    }

    pub fn coord_polar(mut self) -> Self {
        self.coord = Box::new(CoordPolar::new());
        self
    }

    /// Radar / spider chart coordinates: discrete `x` → spokes, `y` → distance
    /// from the centre, straight segments, ring/spoke guides. Pair with
    /// `geom_polygon` (closed series) and `geom_point`.
    pub fn coord_radar(mut self) -> Self {
        self.coord = Box::new(crate::coord::radar::CoordRadar::new());
        self
    }

    pub fn coord_radar_with(mut self, coord: crate::coord::radar::CoordRadar) -> Self {
        self.coord = Box::new(coord);
        self
    }

    pub fn coord_polar_with(mut self, coord: CoordPolar) -> Self {
        self.coord = Box::new(coord);
        self
    }

    // ─── Theme ───────────────────────────────────────────────────

    /// Replace the theme. A `primary_color` set earlier is kept unless the new
    /// theme sets its own, so preset/`primary_color` call order doesn't matter.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.apply_preset(theme);
        self
    }

    fn apply_preset(&mut self, theme: Theme) {
        let primary = self.theme.primary;
        self.theme = theme;
        if self.theme.primary.is_none() {
            self.theme.primary = primary;
        }
    }

    /// Rotate the x-axis tick labels by `degrees` (R's
    /// `guides(x = guide_axis(angle = ...))` / `axis.text.x = element_text(angle)`).
    /// Useful for long category labels. Call after any `theme_*()` preset.
    pub fn axis_text_x_angle(mut self, degrees: f64) -> Self {
        self.theme.axis_text_x.angle = degrees;
        self
    }

    /// Rotate the y-axis tick labels by `degrees`.
    pub fn axis_text_y_angle(mut self, degrees: f64) -> Self {
        self.theme.axis_text_y.angle = degrees;
        self
    }

    /// Stagger x-axis tick labels across `n` rows to avoid overlap (R's
    /// `guides(x = guide_axis(n.dodge = n))`). `1` = no dodging.
    pub fn axis_text_x_dodge(mut self, n: usize) -> Self {
        self.theme.axis_text_x_dodge = n.max(1);
        self
    }

    /// Fix the panel's height:width ratio (R's `aspect.ratio`). Call after any
    /// `theme_*()` preset.
    pub fn aspect_ratio(mut self, ratio: f64) -> Self {
        self.theme.aspect_ratio = Some(ratio);
        self
    }

    /// Draw gridlines on top of the data layers (R's `panel.ontop`).
    pub fn panel_ontop(mut self) -> Self {
        self.theme.panel_ontop = true;
        self
    }

    /// Draw minor tick marks between major ticks (R's `axis.minor.ticks`).
    pub fn axis_minor_ticks(mut self) -> Self {
        self.theme.axis_minor_ticks = true;
        self
    }

    /// How an outer x-axis label that would cross the SVG edge is handled
    /// (B3-1, D6): [`AxisLabelFit::Reserve`](crate::theme::AxisLabelFit::Reserve) (default), `Shift`, or `Clip`
    /// (the 0.21 behaviour). Use `Shift` for panels that must stay
    /// pixel-aligned with other elements (e.g. a host's own aligned strip).
    pub fn axis_text_x_fit(mut self, fit: crate::theme::AxisLabelFit) -> Self {
        self.theme.axis_text_x_fit = fit;
        self
    }

    /// Thin crowded x-axis labels in ggplot2's `check.overlap` priority
    /// order (B3-1, D5). Off by default.
    pub fn axis_text_x_check_overlap(mut self, on: bool) -> Self {
        self.theme.axis_text_x_check_overlap = on;
        self
    }

    /// Align title/subtitle/caption to the panel or the whole plot width
    /// (R's `plot.title.position`).
    pub fn title_position(mut self, pos: crate::theme::TitlePosition) -> Self {
        self.theme.title_position = pos;
        self
    }

    /// Corner for the `tag()` label (R's `plot.tag.position`).
    pub fn tag_position(mut self, pos: crate::theme::TagPosition) -> Self {
        self.theme.tag_position = pos;
        self
    }

    /// Force the legend key layout direction (R's `legend.direction`).
    pub fn legend_direction(mut self, dir: crate::theme::LegendDirection) -> Self {
        self.theme.legend_direction = Some(dir);
        self
    }

    /// Set the brand/primary color used as the default for single-series geoms
    /// that have no color/fill aesthetic mapped. Composes with any theme — one
    /// render process can serve different tenants' brands at render time.
    /// Place the legend inside the panel at panel-relative coordinates
    /// (0..1, 0..1) — `(0,0)` bottom-left, `(1,1)` top-right (R's
    /// `legend.position = c(x, y)`).
    pub fn legend_position_inside(mut self, x: f64, y: f64) -> Self {
        self.theme.legend_position = crate::theme::LegendPosition::Inside(x, y);
        self
    }

    /// Set the legend position (R's `legend.position`, e.g. `Top`/`Bottom`).
    pub fn legend_position(mut self, pos: crate::theme::LegendPosition) -> Self {
        self.theme.legend_position = pos;
        self
    }

    /// Move a [`Right`](crate::theme::LegendPosition::Right) legend to
    /// `Bottom` (drawn horizontally) when it would take more than `share`
    /// (0..1) of the panel's width — the width the panel has with no legend
    /// at all (#287). The band is roughly fixed in size, so on a narrow
    /// render it can take most of the panel; a render wide enough keeps
    /// `Right`, unchanged. Off by default; no effect on any other position.
    pub fn legend_bottom_when_narrow(mut self, share: f64) -> Self {
        self.legend_bottom_share = Some(share);
        self
    }

    pub fn primary_color(mut self, color: (u8, u8, u8)) -> Self {
        self.theme.primary = Some(color);
        self
    }

    pub fn theme_minimal(mut self) -> Self {
        self.apply_preset(crate::theme::presets::theme_minimal());
        self
    }

    pub fn theme_bw(mut self) -> Self {
        self.apply_preset(crate::theme::presets::theme_bw());
        self
    }

    pub fn theme_gray(mut self) -> Self {
        self.apply_preset(crate::theme::presets::theme_gray());
        self
    }

    pub fn theme_classic(mut self) -> Self {
        self.apply_preset(crate::theme::presets::theme_classic());
        self
    }

    /// Apply the publication-ready `theme_pubr()` (ggpubr style).
    pub fn theme_pubr(mut self) -> Self {
        self.apply_preset(crate::theme::presets::theme_pubr());
        self
    }

    pub fn theme_linedraw(mut self) -> Self {
        self.apply_preset(crate::theme::presets::theme_linedraw());
        self
    }

    pub fn theme_light(mut self) -> Self {
        self.apply_preset(crate::theme::presets::theme_light());
        self
    }

    pub fn theme_dark(mut self) -> Self {
        self.apply_preset(crate::theme::presets::theme_dark());
        self
    }

    pub fn theme_void(mut self) -> Self {
        self.apply_preset(crate::theme::presets::theme_void());
        self
    }

    /// Apply incremental theme modifications on top of the current theme.
    /// Like R's `+ theme(axis.text.x = element_text(...))`.
    pub fn theme_update(mut self, update: crate::theme::ThemeUpdate) -> Self {
        self.theme = self.theme.update(update);
        self
    }

    // ─── Guides ──────────────────────────────────────────────────

    /// Configure legend guide (title, ncol, reverse).
    pub fn guides(mut self, guide: crate::guide::config::GuideLegend) -> Self {
        self.guide_legend = guide;
        self
    }

    // ─── Labels ──────────────────────────────────────────────────

    pub fn labs(mut self, labels: Labels) -> Self {
        if labels.title.is_some() {
            self.labels.title = labels.title;
        }
        if labels.subtitle.is_some() {
            self.labels.subtitle = labels.subtitle;
        }
        if labels.x.is_some() {
            self.labels.x = labels.x;
        }
        if labels.y.is_some() {
            self.labels.y = labels.y;
        }
        if labels.caption.is_some() {
            self.labels.caption = labels.caption;
        }
        if labels.tag.is_some() {
            self.labels.tag = labels.tag;
        }
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.labels.title = Some(title.to_string());
        self
    }

    /// Corner tag label (R's `labs(tag = ...)`), drawn at the top-left — handy
    /// for labelling figure panels ("A", "B", …).
    pub fn tag(mut self, tag: &str) -> Self {
        self.labels.tag = Some(tag.to_string());
        self
    }

    pub fn subtitle(mut self, subtitle: &str) -> Self {
        self.labels.subtitle = Some(subtitle.to_string());
        self
    }

    pub fn xlab(mut self, label: &str) -> Self {
        self.labels.x = Some(label.to_string());
        self
    }

    pub fn ylab(mut self, label: &str) -> Self {
        self.labels.y = Some(label.to_string());
        self
    }

    pub fn caption(mut self, caption: &str) -> Self {
        self.labels.caption = Some(caption.to_string());
        self
    }

    // ─── Annotations ──────────────────────────────────────────────

    /// Add an annotation to the plot.
    pub fn annotate(mut self, annotation: Annotation) -> Self {
        self.annotations.push(annotation);
        self
    }

    /// Add a text annotation at data coordinates.
    pub fn annotate_text(self, label: &str, x: f64, y: f64) -> Self {
        self.annotate(Annotation::text(label, x, y))
    }

    /// Add a rectangle annotation at data coordinates.
    pub fn annotate_rect(self, xmin: f64, xmax: f64, ymin: f64, ymax: f64) -> Self {
        self.annotate(Annotation::rect(xmin, xmax, ymin, ymax))
    }

    /// Add a segment annotation between data coordinates.
    pub fn annotate_segment(self, x: f64, y: f64, xend: f64, yend: f64) -> Self {
        self.annotate(Annotation::segment(x, y, xend, yend))
    }

    /// Default [`Locale`](crate::locale::Locale) for the plot's axis and
    /// legend labels: every continuous position / colour / size / alpha scale
    /// labels its default breaks with the locale's decimal and thousands marks
    /// (`1.000`, `2,5` in de-DE), and date/datetime scales use its month names
    /// and date formats (`Okt. 2026`, `15. Okt. 2026`). A scale's own
    /// `with_locale` and custom `label_*` formatters take precedence.
    /// Without this call labels are unchanged (`.` decimal mark, no grouping,
    /// ISO dates).
    ///
    /// ```
    /// use ggplot_rs::prelude::*;
    /// let svg = GGPlot::new(vec![
    ///         ("x".to_string(), vec![Value::Float(0.0), Value::Float(3000.0)]),
    ///         ("y".to_string(), vec![Value::Float(0.5), Value::Float(2.5)]),
    ///     ])
    ///     .aes(Aes::new().x("x").y("y"))
    ///     .geom_point()
    ///     .locale(Locale::from_tag("de-DE"))
    ///     .render_svg_native()
    ///     .unwrap();
    /// assert!(svg.contains(">1.000<") && svg.contains(">1,5<"));
    /// ```
    pub fn locale(mut self, locale: crate::locale::Locale) -> Self {
        self.locale = Some(locale);
        self
    }

    // ─── Build and Render ────────────────────────────────────────

    /// Build the plot without rendering, returning errors on validation failure.
    pub fn try_build(self) -> Result<crate::build::BuiltPlot, GGError> {
        PlotBuilder::build(self)
    }

    /// Build the plot without rendering (analogous to R's ggplot_build()).
    /// Returns the fully computed BuiltPlot with layer data ready for inspection.
    /// Panics on validation errors — use `try_build()` for error handling.
    pub fn build(self) -> crate::build::BuiltPlot {
        self.try_build().expect("plot build failed")
    }

    /// Build and save the plot to a file. Format determined by extension.
    /// (Requires the `plotters` feature, on by default; native only — wasm has
    /// no filesystem/plotters backend.)
    #[cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
    pub fn save(self, path: &str) -> Result<(), GGError> {
        self.save_with_size(path, 800, 600)
    }

    /// Build and save with custom dimensions. (Feature `plotters`; native only.)
    #[cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
    pub fn save_with_size(self, path: &str, w: u32, h: u32) -> Result<(), GGError> {
        let (built, layout) = self.prepare(w, h)?;

        // Determine backend from file extension
        let ext = path.rsplit('.').next().unwrap_or("svg").to_lowercase();

        match ext.as_str() {
            "svg" => {
                let backend = plotters::prelude::SVGBackend::new(path, (w, h));
                Self::render_into(backend.into_drawing_area(), &built, &layout)?;
            }
            "png" | "bmp" | "gif" | "jpeg" | "jpg" | "tiff" => {
                let backend = plotters::prelude::BitMapBackend::new(path, (w, h));
                Self::render_into(backend.into_drawing_area(), &built, &layout)?;
            }
            _ => {
                return Err(GGError::UnsupportedFormat(ext));
            }
        }

        Ok(())
    }

    /// Render the plot to an in-memory SVG document (default 800x600).
    ///
    /// Unlike [`save`](Self::save), this writes nothing to disk — handy for
    /// serving charts from a web/MCP service. Requires the `plotters` feature
    /// (on by default); without it use [`render_svg_native`](Self::render_svg_native).
    #[cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
    pub fn render_svg(self) -> Result<String, GGError> {
        self.render_svg_with_size(800, 600)
    }

    /// Render the plot to an in-memory SVG document with custom dimensions.
    /// (Feature `plotters`; native only — on wasm or without `plotters` use
    /// [`render_svg_native_with_size`](Self::render_svg_native_with_size).)
    #[cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
    pub fn render_svg_with_size(self, w: u32, h: u32) -> Result<String, GGError> {
        let (built, layout) = self.prepare(w, h)?;
        let mut buf = String::new();
        {
            let backend = plotters::prelude::SVGBackend::with_string(&mut buf, (w, h));
            Self::render_into(backend.into_drawing_area(), &built, &layout)?;
        }
        Ok(buf)
    }

    /// Render to SVG through the self-contained [`SvgBackend`](crate::render::svg_backend::SvgBackend)
    /// — a plotters-free path that drives the same `PlotRenderer`, proving the
    /// `DrawBackend` abstraction (and needing no glyph rasterization).
    pub fn render_svg_native(self) -> Result<String, GGError> {
        self.render_svg_native_with_size(800, 600)
    }

    /// [`render_svg_native`](Self::render_svg_native) with an explicit size.
    ///
    /// The root `<svg>` carries `data-plot="x y w h"` (the panel rect) and the
    /// trained position domains (`data-domain`, `data-xdomain`, `data-ydomain`,
    /// `data-xlevels`, `data-ylevels` — see
    /// [`root_data_attrs`](crate::render::svg_backend::root_data_attrs)).
    /// Hoverable marks carry `data-x`, `data-series` and `data-value`.
    pub fn render_svg_native_with_size(self, w: u32, h: u32) -> Result<String, GGError> {
        Ok(self.render_native(w, h)?.0.finish())
    }

    /// Like [`render_svg_native_with_size`](Self::render_svg_native_with_size),
    /// but also returns the build warnings (rows dropped for non-finite
    /// positions, layers whose stat produced no data, …) — see
    /// [`BuiltPlot::warnings`](crate::build::BuiltPlot::warnings).
    pub fn render_svg_native_with_warnings(
        self,
        w: u32,
        h: u32,
    ) -> Result<(String, Vec<String>), GGError> {
        let (backend, warnings, _) = self.render_native(w, h)?;
        Ok((backend.finish(), warnings))
    }

    /// Render a `w`×`h` chart as a *nested* `<svg x=.. y=.. width=.. height=..
    /// viewBox="0 0 w h">` fragment for composition into a larger SVG (a
    /// dashboard): no `xmlns` (inherited from the parent), positioned at
    /// `(x, y)` in the parent's user space. No string surgery needed:
    ///
    /// ```ignore
    /// let mut page = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg" width="900" height="400">"#);
    /// page += &plot_a.render_svg_native_at(0.0, 0.0, 450, 400)?;
    /// page += &plot_b.render_svg_native_at(450.0, 0.0, 450, 400)?;
    /// page += "</svg>";
    /// ```
    pub fn render_svg_native_at(self, x: f64, y: f64, w: u32, h: u32) -> Result<String, GGError> {
        Ok(self.render_native(w, h)?.0.finish_fragment(x, y))
    }

    /// Like [`render_svg_native_with_size`](Self::render_svg_native_with_size),
    /// but also returns the panel rect in pixels `[x, y, w, h]` — enough to
    /// overlay a WebGL/canvas layer that draws marks in data coordinates.
    pub fn render_svg_area_with_size(self, w: u32, h: u32) -> Result<(String, [f64; 4]), GGError> {
        let (backend, _, pa) = self.render_native(w, h)?;
        Ok((backend.finish(), [pa.x, pa.y, pa.width, pa.height]))
    }

    /// Shared native-SVG pipeline: build, lay out, render into an
    /// [`SvgBackend`](crate::render::svg_backend::SvgBackend) (not yet
    /// finished), returning it with the build warnings and the panel rect.
    fn render_native(
        self,
        w: u32,
        h: u32,
    ) -> Result<
        (
            crate::render::svg_backend::SvgBackend,
            Vec<String>,
            crate::render::Rect,
        ),
        GGError,
    > {
        let svg_root_data = self.svg_root_data;
        let root_attrs = self.root_attrs.clone();
        let svg_warnings_attr = self.svg_warnings_attr;
        let svg_formatted_values = self.svg_formatted_values;
        let svg_title = self.svg_title.clone();
        let svg_desc = self.svg_desc.clone();
        let (built, layout) = self.prepare(w, h)?;
        let pa = layout.plot_area.clone();
        let mut backend = crate::render::svg_backend::SvgBackend::new(w, h, pa.clone());
        backend.set_formatted_values(svg_formatted_values);
        backend.set_svg_title(svg_title.clone());
        backend.set_svg_desc(svg_desc.clone());
        let existing_root_attrs = if svg_root_data {
            let attrs = crate::render::svg_backend::root_data_attrs(&built);
            backend.set_root_attrs(attrs.clone());
            // `data-plot` itself is a literal in `root_open`, not part of
            // `root_data_attrs`'s output, but it is written whenever
            // `svg_root_data` is on — reserve it too.
            let mut reserved = attrs;
            // `data-viewport`/`data-panels` likewise come from the backend
            // (a letterboxing coord / fixed-aspect facets).
            for name in ["data-plot", "data-viewport", "data-panels"] {
                reserved.push((name.to_string(), String::new()));
            }
            reserved
        } else {
            backend.without_plot_attr();
            Vec::new()
        };
        PlotRenderer::render(&built, &mut backend).map_err(GGError::Render)?;
        let mut warnings = built.warnings;
        warnings.extend(backend.take_warnings());

        let mut reserved = existing_root_attrs;
        if svg_warnings_attr {
            reserved.push(("data-warnings".to_string(), String::new()));
        }
        // #255: `role`/`aria-labelledby`/`aria-describedby` are written
        // directly by `SvgBackend::root_open` when a title/desc was set —
        // reserve them so a colliding `svg_attr` call is dropped with a
        // warning instead of duplicating the attribute.
        if svg_title.is_some() || svg_desc.is_some() {
            reserved.push(("role".to_string(), String::new()));
        }
        if svg_title.is_some() {
            reserved.push(("aria-labelledby".to_string(), String::new()));
        }
        if svg_desc.is_some() {
            reserved.push(("aria-describedby".to_string(), String::new()));
        }
        let extra = crate::render::svg_backend::validate_root_attrs(
            &root_attrs,
            &reserved,
            "plot",
            &mut warnings,
        );
        backend.extend_root_attrs(extra);
        if svg_warnings_attr {
            backend.extend_root_attrs(vec![(
                "data-warnings".to_string(),
                crate::render::svg_backend::warnings_json(&warnings),
            )]);
        }
        Ok((backend, warnings, pa))
    }

    /// Render to a raw RGBA pixel buffer via the self-contained raster
    /// [`PixelBackend`](crate::render::pixel_backend::PixelBackend) (feature
    /// `canvas`) — fast for large-N and wasm-compatible. Returns `(w, h, rgba)`,
    /// ready for a `<canvas>` `putImageData`.
    #[cfg(feature = "canvas")]
    pub fn render_rgba_with_size(self, w: u32, h: u32) -> Result<(u32, u32, Vec<u8>), GGError> {
        let (built, layout) = self.prepare(w, h)?;
        let mut backend =
            crate::render::pixel_backend::PixelBackend::new(w, h, layout.plot_area.clone());
        PlotRenderer::render(&built, &mut backend).map_err(GGError::Render)?;
        let (ow, oh) = backend.dimensions();
        Ok((ow, oh, backend.into_rgba()))
    }

    /// Like [`render_rgba_with_size`](Self::render_rgba_with_size), but also
    /// returns the panel rect in pixels `[x, y, w, h]` — enough to map a canvas
    /// pixel back to data coordinates for interactive hover.
    #[cfg(feature = "canvas")]
    pub fn render_rgba_area_with_size(
        self,
        w: u32,
        h: u32,
    ) -> Result<(Vec<u8>, [f64; 4]), GGError> {
        let (built, layout) = self.prepare(w, h)?;
        let pa = layout.plot_area.clone();
        let mut backend = crate::render::pixel_backend::PixelBackend::new(w, h, pa.clone());
        PlotRenderer::render(&built, &mut backend).map_err(GGError::Render)?;
        Ok((backend.into_rgba(), [pa.x, pa.y, pa.width, pa.height]))
    }

    /// Render to PNG bytes via the raster [`PixelBackend`](crate::render::pixel_backend::PixelBackend)
    /// (feature `canvas`). Unlike [`render_png`](Self::render_png) this needs no
    /// plotters bitmap backend, so it also works on wasm.
    #[cfg(feature = "canvas")]
    pub fn render_png_raster_with_size(self, w: u32, h: u32) -> Result<Vec<u8>, GGError> {
        let (built, layout) = self.prepare(w, h)?;
        let mut backend =
            crate::render::pixel_backend::PixelBackend::new(w, h, layout.plot_area.clone());
        PlotRenderer::render(&built, &mut backend).map_err(GGError::Render)?;
        backend.into_png().map_err(GGError::Render)
    }

    /// Render the plot to in-memory PNG bytes (default 800x600).
    ///
    /// Returns a fully-encoded PNG, ready to write to an HTTP response or
    /// embed as a data URI — no temp files involved. Requires the `plotters`
    /// feature (on by default); see also `render_png_raster_with_size`
    /// (feature `canvas`).
    #[cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
    pub fn render_png(self) -> Result<Vec<u8>, GGError> {
        self.render_png_with_size(800, 600)
    }

    /// Render the plot to in-memory PNG bytes with custom dimensions. (Feature
    /// `plotters`; native only — PNG needs the plotters bitmap backend.)
    #[cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
    pub fn render_png_with_size(self, w: u32, h: u32) -> Result<Vec<u8>, GGError> {
        let (built, layout) = self.prepare(w, h)?;

        // plotters' BitMapBackend draws into a raw RGB buffer; we then encode
        // that buffer to PNG via the `image` crate.
        let mut rgb = vec![0u8; (w as usize) * (h as usize) * 3];
        {
            let backend = plotters::prelude::BitMapBackend::with_buffer(&mut rgb, (w, h));
            Self::render_into(backend.into_drawing_area(), &built, &layout)?;
        }

        let img = image::RgbImage::from_raw(w, h, rgb).ok_or_else(|| {
            GGError::Render(RenderError::BackendError(
                "PNG buffer size mismatch".to_string(),
            ))
        })?;
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageOutputFormat::Png)
            .map_err(|e| GGError::Render(RenderError::BackendError(format!("{:?}", e))))?;
        Ok(out.into_inner())
    }

    /// Shared pipeline: build the plot, apply label overrides, compute layout.
    pub(crate) fn prepare(
        self,
        w: u32,
        h: u32,
    ) -> Result<(crate::build::BuiltPlot, PlotLayout), GGError> {
        let (mut built, meta) = self.build_for_render()?;
        let layout = Self::layout_built(&mut built, &meta, w, h);
        Ok((built, layout))
    }

    /// The size-independent half of [`prepare`](Self::prepare): build the plot,
    /// resolve theme inheritance and apply axis-label overrides.
    pub(crate) fn build_for_render(self) -> Result<(crate::build::BuiltPlot, RenderMeta), GGError> {
        let plot = self;

        let has_title = plot.labels.title.is_some();
        let has_subtitle = plot.labels.subtitle.is_some();
        let has_caption = plot.labels.caption.is_some();
        let has_legend = plot.has_legend_mapping();
        let legend_bottom_share = plot.legend_bottom_share;
        let x_label = plot.labels.x.clone();
        let y_label = plot.labels.y.clone();

        let mut built = PlotBuilder::build(plot)?;

        // Resolve R-style theme inheritance (root `text` → child text elements).
        built.theme.resolve_inheritance();

        // Apply user label overrides to scales
        if let Some(ref label) = x_label {
            if let Some(s) = built.scales.get_mut(&crate::aes::Aesthetic::X) {
                s.set_name(label);
            }
        }
        if let Some(ref label) = y_label {
            if let Some(s) = built.scales.get_mut(&crate::aes::Aesthetic::Y) {
                s.set_name(label);
            }
        }

        Ok((
            built,
            RenderMeta {
                has_title,
                has_subtitle,
                has_caption,
                has_legend,
                legend_bottom_share,
            },
        ))
    }

    /// The size-dependent half of [`prepare`](Self::prepare): auto-tune axis
    /// labels for `w`×`h` and compute the layout.
    pub(crate) fn layout_built(
        built: &mut crate::build::BuiltPlot,
        meta: &RenderMeta,
        w: u32,
        h: u32,
    ) -> PlotLayout {
        let RenderMeta {
            has_title,
            has_subtitle,
            has_caption,
            has_legend,
            legend_bottom_share,
        } = *meta;
        let x_axis_top = built
            .scales
            .get(&crate::aes::Aesthetic::X)
            .map(|s| s.axis_position_opposite())
            .unwrap_or(false);

        // #287: decided first, so every legend pass below plans for the
        // position actually drawn, on this render's own layout.
        if let Some(share) = legend_bottom_share {
            if has_legend
                && matches!(
                    built.theme.legend_position,
                    crate::theme::LegendPosition::Right
                )
            {
                let panel_width = |pos| {
                    let mut theme = built.theme.clone();
                    theme.legend_position = pos;
                    PlotLayout::compute_with_extras(
                        w as f64,
                        h as f64,
                        &theme,
                        has_title,
                        has_subtitle,
                        has_caption,
                        has_legend,
                        x_axis_top,
                        None,
                        None,
                        crate::render::layout::LayoutExtras::default(),
                    )
                    .plot_area
                    .width
                };
                // Against the panel it would have with no legend at all
                // (the clamped minimum panel width hides the real band).
                let free = panel_width(crate::theme::LegendPosition::None);
                if panel_width(crate::theme::LegendPosition::Right) < (1.0 - share) * free {
                    built.theme.legend_position = crate::theme::LegendPosition::Bottom;
                }
            }
        }

        // Auto-tune categorical axis labels so they stay legible. A flip swaps
        // which trained scale sits on each visual axis: the bottom (x) axis reads
        // the Y scale when flipped, the left (y) axis reads the X scale.
        let flipped = built.coord.is_flipped();
        let h_aes = if flipped {
            crate::aes::Aesthetic::Y
        } else {
            crate::aes::Aesthetic::X
        };
        let v_aes = if flipped {
            crate::aes::Aesthetic::X
        } else {
            crate::aes::Aesthetic::Y
        };

        // The bottom-axis break labels that are actually drawn — a
        // `with_labels` override can blank some entries ("" ) to thin a
        // crowded discrete axis while keeping every position (sipemu/
        // ggplot-rs#162); a blanked entry is never drawn, so it never
        // competes for horizontal room or vertical drop below. Shared by
        // the auto-rotate decision and the rotated-label height reservation
        // just below, so both agree on what's actually on the canvas.
        let drawn_x_labels: Vec<(f64, String)> = built
            .scales
            .get(&h_aes)
            .map(|hs| {
                hs.breaks()
                    .into_iter()
                    .filter(|(_, label)| !label.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        let h_is_discrete = built
            .scales
            .get(&h_aes)
            .map(|hs| hs.is_discrete())
            .unwrap_or(false);

        // Crowded discrete bottom-axis labels overlap when drawn flat. Rotate
        // them — −45° if that de-collides, otherwise vertical (270°) — only
        // when no explicit angle was set (angle.abs() < 1.0 reads as
        // "unset"). Sized from the real, measured widths of the labels that
        // are actually drawn (`render::text::text_width`), both for the
        // label-width estimate and the crowding count `n` — #162's bug was
        // counting every break, blanked or not, which over-triggered the
        // rotation. −45° (not +45°) is ggplot2's own idiom (`angle = 45,
        // hjust = 1`, mirrored to this library's angle sign, sipemu/
        // ggplot-rs#163): it ends each label at its own tick and slants
        // down-left, rather than +45°'s down-right slant, which — anchored
        // the same way — would start under the *previous* category.
        if built.theme.axis_text_x.visible
            && built.theme.axis_text_x.angle.abs() < 1.0
            && h_is_discrete
            && !drawn_x_labels.is_empty()
        {
            let font = built.theme.axis_text_x.size;
            let max_w = drawn_x_labels
                .iter()
                .map(|(_, l)| crate::render::text::text_width(l, font))
                .fold(0.0_f64, f64::max);
            let n = drawn_x_labels.len() as f64;
            let spacing = 0.9 * w as f64 / n; // ~px between adjacent *drawn* labels
            if max_w > spacing {
                // 45° shrinks the horizontal footprint by cos45; if that
                // still doesn't fit, go vertical (270° = reads bottom-to-
                // top, hanging below the axis like the y-axis title).
                let vertical = max_w * std::f64::consts::FRAC_1_SQRT_2 > spacing;
                built.theme.axis_text_x.angle = if vertical { 270.0 } else { -45.0 };
            }
        }

        // Whenever the bottom-axis labels end up rotated — by the
        // auto-rotate decision just above, or by an explicit
        // `axis_text_x_angle` call — reserve the real bottom space their
        // rotated bounding boxes need, so they never overlap the panel
        // (#162 also asked for this: an explicit angle reserved nothing).
        // `rotated_label_rise_fall` is the same helper `draw_x_axis` calls
        // to place its rotation anchor — `rise` must be reserved (as well
        // as placed) or a long enough label's far end swings right back up
        // over the panel regardless of how much height is reserved here;
        // `fall` just needs reserving so the label stays on-canvas. Both
        // here and in `draw_x_axis`, the anchor fraction must come from
        // `rotated_anchor_frac(angle, dir)` (sipemu/ggplot-rs#163) — not a
        // blind `1.0` — or the reservation is sized for the wrong end of
        // the label; `dir` matches `draw_x_axis`'s own tick direction.
        let x_label_height = if built.theme.axis_text_x.visible
            && built.theme.axis_text_x.angle.abs() > 10.0
            && !drawn_x_labels.is_empty()
        {
            let font = built.theme.axis_text_x.size;
            let angle = built.theme.axis_text_x.angle;
            let dir = if x_axis_top { -1.0 } else { 1.0 };
            let (rise, fall) = crate::render::text::rotated_label_rise_fall(
                drawn_x_labels.iter().map(|(_, l)| l.as_str()),
                font,
                angle,
                crate::render::text::rotated_anchor_frac(angle, dir),
            );
            Some(built.theme.legend_spacing / 2.0 + font / 2.0 + rise + fall)
        } else {
            None
        };

        // Reserve the left-axis width from the actual label lengths so long
        // category names (a flipped bar chart's rows) aren't clipped. Never
        // shrinks below the default; grows up to 35% of the width.
        let y_label_width = if built.theme.axis_text_y.visible {
            built.scales.get(&v_aes).map(|vs| {
                let max_ch = vs
                    .breaks()
                    .iter()
                    .map(|(_, l)| l.chars().count())
                    .max()
                    .unwrap_or(0);
                let measured = max_ch as f64 * built.theme.axis_text_y.size * 0.6 + 4.0;
                let default = built.theme.axis_text_y.size * 3.5 + 4.0;
                measured.max(default).min(0.35 * w as f64)
            })
        } else {
            None
        };

        // B3-1 (D6): reserve just enough left/right panel margin to keep
        // the outer x-axis labels on-canvas under `AxisLabelFit::Reserve`,
        // for a single (non-faceted) panel on a coord that supports it
        // (not polar/radar/sf, where a label bbox doesn't map to a simple
        // panel margin). A faceted plot skips the layout-level reservation
        // and relies on `draw_x_axis`'s own draw-time shift instead — so
        // does `AxisLabelFit::Shift` (the panel must keep its size) and
        // `Clip` (the 0.21 behaviour, no correction at all). A plot whose
        // outer labels already fit never grows `extras`, so it stays
        // byte-identical.
        let mut extras = crate::render::layout::LayoutExtras::default();
        // A secondary y axis draws its ticks, labels and title right of the
        // panel: reserve that width so they stay on-canvas (and clear of a
        // right-hand legend).
        if let (Some(vs), Some(sec)) = (
            built.scales.get(&crate::aes::Aesthetic::Y),
            built.scales.sec_axis(&crate::aes::Aesthetic::Y),
        ) {
            let theme = &built.theme;
            let max_ch = match sec.own_ticks(vs) {
                Some(t) => t.iter().map(|(_, l)| l.chars().count()).max(),
                None => vs.breaks().iter().map(|(_, l)| l.chars().count() + 2).max(),
            }
            .unwrap_or(0);
            let text = if theme.axis_text_y.visible {
                (max_ch as f64 * theme.axis_text_y.size * 0.6).max(theme.axis_text_y.size * 3.5)
            } else {
                0.0
            };
            let title = if !sec.name.is_empty() && theme.axis_title_y.visible {
                theme.axis_title_y.size * 1.5
            } else {
                0.0
            };
            extras.right += theme.axis_ticks_length + theme.legend_spacing + text + title;
        }
        let can_reserve = built.facet.is_none()
            && built.theme.axis_text_x.visible
            && built.theme.axis_text_x_fit == crate::theme::AxisLabelFit::Reserve
            && !built.coord.is_polar()
            && built.coord.supports_axis_reserve();
        if can_reserve {
            if let Some(hs) = built.scales.get(&h_aes) {
                let breaks = hs.breaks();
                if let (Some(first), Some(last)) = (breaks.first(), breaks.last()) {
                    let angle = built.theme.axis_text_x.angle;
                    let anchor = if angle.abs() > 10.0 {
                        let dir = if x_axis_top { -1.0 } else { 1.0 };
                        crate::render::text::rotated_anchor_frac(angle, dir)
                    } else {
                        0.5
                    };
                    let size = built.theme.axis_text_x.size;
                    const EDGE: f64 = 1.0;
                    for _ in 0..2 {
                        let layout = PlotLayout::compute_with_extras(
                            w as f64,
                            h as f64,
                            &built.theme,
                            has_title,
                            has_subtitle,
                            has_caption,
                            has_legend,
                            x_axis_top,
                            y_label_width,
                            x_label_height,
                            extras,
                        );
                        let area = &layout.plot_area;
                        let edge_y = area.y + area.height;
                        let to_point = |pos: f64| if flipped { (0.0, pos) } else { (pos, 0.0) };
                        let px_first = built.coord.transform(to_point(first.0), area).0;
                        let px_last = built.coord.transform(to_point(last.0), area).0;
                        let bbox_first = crate::render::text::label_bbox(
                            &first.1, size, px_first, edge_y, angle, anchor,
                        );
                        let bbox_last = crate::render::text::label_bbox(
                            &last.1, size, px_last, edge_y, angle, anchor,
                        );
                        let deficit_left = (EDGE - bbox_first[0]).max(0.0);
                        let deficit_right = (bbox_last[2] - (w as f64 - EDGE)).max(0.0);
                        if deficit_left <= 0.0 && deficit_right <= 0.0 {
                            break;
                        }
                        // Growing the left margin by `eL` (holding `eR` fixed)
                        // shifts a label at normalized position `p` right by
                        // `eL·(1 − p)`; growing the right margin by `eR`
                        // shifts it left by `eR·p` (the panel maps [0, 1] to
                        // [plot_x + eL, plot_x + width − eR]). Review #158
                        // finding 1: the original code divided by `p_first`
                        // and `1 − p_last` — inverted, which over-reserved
                        // by about `1/p_last` (≈20× at a typical p_last≈0.95).
                        let (p_first, p_last) = (first.0, last.0);
                        if deficit_left > 0.0 && deficit_right > 0.0 {
                            // Both ends overflow: solve the 2×2 system so
                            // each margin accounts for its effect on *both*
                            // labels, rather than correcting them
                            // independently and overshooting.
                            let (a, b, c, d) = (1.0 - p_first, p_first, 1.0 - p_last, p_last);
                            let det = a * d - b * c;
                            if det.abs() > 0.01 {
                                extras.left +=
                                    ((deficit_left * d + deficit_right * b) / det).max(0.0);
                                extras.right +=
                                    ((deficit_right * a + deficit_left * c) / det).max(0.0);
                            } else {
                                // Degenerate (first and last break nearly
                                // coincide in normalized space): fall back
                                // to the independent, clamped formulas.
                                extras.left += deficit_left / (1.0 - p_first).max(0.05);
                                extras.right += deficit_right / p_last.max(0.05);
                            }
                        } else if deficit_left > 0.0 {
                            extras.left += deficit_left / (1.0 - p_first).max(0.05);
                        } else if deficit_right > 0.0 {
                            extras.right += deficit_right / p_last.max(0.05);
                        }
                        // Cap the total reservation so the panel can never
                        // start beyond the canvas or be squeezed past the
                        // same minimum width `compute_with_extras` already
                        // floors it to (review #158's second re-review,
                        // LOW): a single, centred break whose label is
                        // wider than the whole canvas has `p_first ==
                        // p_last == 0.5`, so both `eL` and `eR` grow by
                        // roughly the same (large) deficit independently,
                        // and together they could place `plot_x` past `w`
                        // entirely. Past this cap, Reserve simply stops
                        // growing and `draw_x_axis`'s own draw-time shift
                        // — which always runs for Reserve, the same as for
                        // `Shift` — takes over for whatever's still
                        // outside, exactly as D6 already documents.
                        const MIN_PANEL_WIDTH: f64 = 50.0;
                        let cap = (w as f64 - MIN_PANEL_WIDTH).max(0.0);
                        if extras.left + extras.right > cap {
                            let scale = if extras.left + extras.right > 0.0 {
                                cap / (extras.left + extras.right)
                            } else {
                                0.0
                            };
                            extras.left *= scale;
                            extras.right *= scale;
                        }
                    }
                }
            }
        }

        // B3-2 (D7): a discrete legend's `nrow`/`ncol` grid, or an
        // overflowing horizontal legend's auto-wrap, reserves extra legend
        // rows/width. Several simultaneous discrete guides (e.g. `colour`
        // and `shape` both mapped to discrete variables) each get their
        // own independent plan (sipemu/ggplot-rs#155) — previously this
        // only ran for exactly one discrete guide.
        if has_legend
            && !matches!(
                built.theme.legend_position,
                crate::theme::LegendPosition::None
            )
        {
            let legend_aes =
                crate::guide::legend::legend_scales_for(&built.scales, &built.suppressed_aes);
            // #236: a mapped `size` legend's largest key can need a bigger
            // box than the theme's flat defaults — independent of B3-2's
            // discrete-guide wrap planning below, so computed unconditionally.
            extras.legend_max_key_size = legend_aes
                .iter()
                .filter_map(|aes| {
                    let scale = built.scales.get(aes)?;
                    let breaks = scale.breaks();
                    Some(crate::guide::legend::size_key_diameter(
                        aes,
                        &built.scales,
                        scale,
                        &breaks,
                    ))
                })
                .fold(0.0_f64, f64::max);
            // #545: a continuous, non-colourbar guide (`Size`/
            // `Linewidth`/… sampled breaks) needs the same B3-2 wrap/grid
            // reservation pass a discrete guide gets — only a true
            // colourbar (continuous `Color`/`Fill`) is exempt, since it
            // draws as a fixed-width bar instead of sampled keys.
            // Previously this gate only fired for a discrete guide, so a
            // plot mapping *only* a continuous `Size`/`Linewidth` (no
            // discrete guide alongside it) skipped the whole reservation
            // pass and its legend could run straight off the panel/canvas
            // with no wrap, shrink, or "+N more" to catch it.
            let has_wrap_guide = legend_aes.iter().any(|a| {
                let is_discrete = built
                    .scales
                    .get(a)
                    .map(|s| s.is_discrete())
                    .unwrap_or(false);
                is_discrete || !crate::guide::legend::is_colorbar_aesthetic(a)
            });
            if has_wrap_guide {
                let layout0 = PlotLayout::compute_with_extras(
                    w as f64,
                    h as f64,
                    &built.theme,
                    has_title,
                    has_subtitle,
                    has_caption,
                    has_legend,
                    x_axis_top,
                    y_label_width,
                    x_label_height,
                    extras,
                );
                // `y` is unused here (only `legend_x0`/`is_horizontal` feed
                // the avail-width check below), so #233's `Top` row-count
                // parameter can't matter — `extras.legend_extra_rows` isn't
                // even known yet at this point in the pass. Likewise #225's
                // `Inside` justification only matters when `legend_x0` is
                // actually used for an `Inside` position combined with an
                // explicit `nrow`/`ncol` grid (`Inside` otherwise never
                // reaches this discrete-guide-wrap check), too rare to
                // bother estimating a real size hint for here.
                let (legend_x0, _, mut is_horizontal) = crate::guide::legend::legend_position(
                    &built.theme,
                    &layout0.plot_area,
                    extras.legend_extra_rows,
                    (0.0, 0.0),
                    &layout0.total,
                );
                if let Some(dir) = built.theme.legend_direction {
                    is_horizontal = matches!(dir, crate::theme::LegendDirection::Horizontal);
                }
                let avail_width =
                    (layout0.plot_area.x + layout0.plot_area.width - legend_x0).max(0.0);
                let mut legend_warnings = Vec::new();
                let mut plans = crate::guide::legend::plan_all_legends(
                    &built.scales,
                    &built.guide_legend,
                    &built.theme,
                    &built.suppressed_aes,
                    avail_width,
                    layout0.plot_area.height,
                    is_horizontal,
                    &mut legend_warnings,
                );
                built.warnings.extend(legend_warnings);

                if !plans.is_empty() {
                    // Guides laid out side by side (a horizontal legend
                    // position) share one row band — its height is set
                    // by whichever guide wrapped to the most rows.
                    // Guides stacked vertically each add their own full
                    // height, so their extra rows sum.
                    extras.legend_extra_rows = if is_horizontal {
                        plans
                            .values()
                            .map(|p| p.rows.saturating_sub(1))
                            .max()
                            .unwrap_or(0)
                    } else {
                        plans.values().map(|p| p.rows.saturating_sub(1)).sum()
                    };
                    // Review #158 finding 3: a vertical explicit grid
                    // with ncol > 1 needs extra legend band width for
                    // its later columns, or they run off the canvas
                    // (cols is always 1, col_pitch 0, for the horizontal
                    // auto-wrap path, so this is a no-op there). Several
                    // guides take the widest requirement among them.
                    extras.legend_extra_width = plans
                        .values()
                        .map(|p| p.cols.saturating_sub(1) as f64 * p.col_pitch)
                        .fold(0.0_f64, f64::max);

                    // #159: a vertical multi-column grid that still runs
                    // off the canvas even after reserving the extra width
                    // above now (1) tries reducing its column count
                    // toward 1 — the first count that fits wins; (2) if
                    // even one column still overflows, truncates labels
                    // with an ellipsis (keeping the full label available
                    // via a `<title>`); and only (3) warns if it still
                    // doesn't fit after both. A grid that already fits is
                    // untouched — no reduction, no truncation, no warning
                    // (review #158's second re-review, LOW, is the
                    // original "warn only" version of this check).
                    if !is_horizontal {
                        let final_layout = PlotLayout::compute_with_extras(
                            w as f64,
                            h as f64,
                            &built.theme,
                            has_title,
                            has_subtitle,
                            has_caption,
                            has_legend,
                            x_axis_top,
                            y_label_width,
                            x_label_height,
                            extras,
                        );
                        // `y` is unused here too, but `extras` is final by
                        // this point, so pass its real `legend_extra_rows`
                        // for consistency with #233's `Top` formula. #225's
                        // `Inside` size hint is skipped here for the same
                        // reason as the pass above.
                        let (legend_x_final, _, _) = crate::guide::legend::legend_position(
                            &built.theme,
                            &final_layout.plot_area,
                            extras.legend_extra_rows,
                            (0.0, 0.0),
                            &final_layout.total,
                        );
                        let swatch_and_gap =
                            built.theme.legend_key_width + built.theme.legend_spacing;
                        let text_size = built.theme.legend_text.size;
                        // A plain fn, not a closure: #168's retightening
                        // pass below re-runs this same check at a second
                        // `legend_x` (the narrower reservation's), so
                        // `legend_x` has to be a parameter rather than
                        // captured.
                        fn right_edge_of(
                            legend_x: f64,
                            swatch_and_gap: f64,
                            text_size: f64,
                            plan: &crate::guide::legend::LegendPlan,
                            breaks: &[(f64, String)],
                        ) -> f64 {
                            plan.entries
                                .iter()
                                .filter_map(|e| breaks.get(e.break_idx).map(|(_, l)| (e, l)))
                                .map(|(e, label)| {
                                    let text =
                                        e.label_override.as_deref().unwrap_or(label.as_str());
                                    legend_x
                                        + e.x_offset
                                        + swatch_and_gap
                                        + crate::render::text::text_width(text, text_size)
                                })
                                .fold(0.0_f64, f64::max)
                        }

                        let mut updates: Vec<(
                            crate::aes::Aesthetic,
                            crate::guide::legend::LegendPlan,
                        )> = Vec::new();
                        // #168: guides narrowed below by *column
                        // reduction* (not label truncation) are
                        // candidates for the retightening pass after this
                        // loop — a truncated guide already picked the
                        // narrowest column count, so there's no reserved
                        // width left to give back.
                        let mut reduced_aes: Vec<crate::aes::Aesthetic> = Vec::new();
                        for (aes, plan) in &plans {
                            if plan.cols <= 1 {
                                continue;
                            }
                            let Some(scale) = built.scales.get(aes) else {
                                continue;
                            };
                            let breaks = scale.breaks();
                            if right_edge_of(
                                legend_x_final,
                                swatch_and_gap,
                                text_size,
                                plan,
                                &breaks,
                            ) <= w as f64
                            {
                                continue; // already fits — untouched.
                            }

                            // Step 1: reduce columns toward 1; the first
                            // count that fits wins.
                            let n = breaks.len();
                            let mut reduced: Vec<crate::guide::legend::LegendPlan> = Vec::new();
                            for c in (1..plan.cols).rev() {
                                let mut candidate_guide = built.guide_legend.clone();
                                candidate_guide.ncol = Some(c);
                                candidate_guide.nrow = None;
                                let mut tmp_warnings = Vec::new();
                                let candidate_guide =
                                    candidate_guide.validated(n, &mut tmp_warnings);
                                if let Some(cand) = crate::guide::legend::plan_legend(
                                    &breaks,
                                    &candidate_guide,
                                    &built.theme,
                                    avail_width,
                                    layout0.plot_area.height,
                                    false,
                                ) {
                                    reduced.push(cand);
                                }
                            }
                            if let Some(fits) = reduced.iter().find(|c| {
                                right_edge_of(legend_x_final, swatch_and_gap, text_size, c, &breaks)
                                    <= w as f64
                            }) {
                                extras.legend_extra_rows =
                                    extras.legend_extra_rows.max(fits.rows.saturating_sub(1));
                                updates.push((aes.clone(), fits.clone()));
                                reduced_aes.push(aes.clone());
                                continue;
                            }

                            // Step 2: no column count fit — truncate
                            // labels (starting from the narrowest, 1-
                            // column, candidate) with an ellipsis, each
                            // to whatever room its own position leaves.
                            let base = reduced.last().cloned().unwrap_or_else(|| plan.clone());
                            extras.legend_extra_rows =
                                extras.legend_extra_rows.max(base.rows.saturating_sub(1));
                            let mut truncated = base.clone();
                            for e in &mut truncated.entries {
                                let Some((_, label)) = breaks.get(e.break_idx) else {
                                    continue;
                                };
                                let avail = (w as f64
                                    - (legend_x_final + e.x_offset + swatch_and_gap))
                                    .max(0.0);
                                if let Some(short) =
                                    crate::render::text::truncate_to_width(label, avail, text_size)
                                {
                                    e.label_override = Some(short);
                                }
                            }

                            // Step 3: only warn if it still doesn't fit
                            // after both the reduction and the
                            // truncation above.
                            let truncated_edge = right_edge_of(
                                legend_x_final,
                                swatch_and_gap,
                                text_size,
                                &truncated,
                                &breaks,
                            );
                            if truncated_edge > w as f64 {
                                built.warnings.push(format!(
                                    "legend: the {}-column grid still doesn't fit the {w}px chart (needs about {} px)",
                                    truncated.cols,
                                    truncated_edge.ceil() as i64
                                ));
                            }
                            updates.push((aes.clone(), truncated));
                        }
                        for (aes, new_plan) in &updates {
                            plans.insert(aes.clone(), new_plan.clone());
                        }

                        // #168: `extras.legend_extra_width` above still
                        // reserves the *original*, pre-reduction column
                        // count's width — that stale, wider reservation
                        // is an accident of ordering (reserve → layout →
                        // fit? → reduce), not a requirement, so the panel
                        // stays narrower than it needs to be once a guide
                        // above got fewer columns.
                        //
                        // Tighten it to the reduced guides' new column
                        // counts and re-check fit once more at the
                        // narrower reservation (a bounded retry, not an
                        // unbounded fixed point: a tighter reservation
                        // moves `legend_x_final` right — less headroom
                        // before the canvas edge — which could in
                        // principle call for yet another round of
                        // reduction; rather than chase reserve → layout →
                        // fit → reduce → reserve → … indefinitely, this
                        // keeps the tighter reservation only if every
                        // guide still fits under it, and falls back to
                        // the wider, already-proven-safe reservation
                        // otherwise — so retightening can only ever
                        // help, never regress a plot that fit before
                        // this pass existed).
                        //
                        // Review #167 finding 1 on #171 (HIGH, fixed):
                        // the retry must re-check *every* plan in
                        // `plans`, not just `reduced_aes`. `legend_x`
                        // moves for every guide sharing the row, not
                        // only the one(s) that got reduced — a second
                        // guide that already fit at the old, wider
                        // `legend_x_final` (its own width is a fixed
                        // `6·text_size` estimate, unrelated to the
                        // reduced guide's labels) or whose labels were
                        // truncated against that old `legend_x_final`
                        // shifts right by the same amount and was never
                        // re-checked, so it could run off the canvas
                        // with no warning (the warnings were computed
                        // against the old x too). `reduced_aes` is kept
                        // only to gate *entering* this block at all —
                        // nothing to reclaim if nothing was reduced.
                        if !reduced_aes.is_empty() {
                            let original_reserve = extras.legend_extra_width;
                            let tighter_reserve = plans
                                .values()
                                .map(|p| p.cols.saturating_sub(1) as f64 * p.col_pitch)
                                .fold(0.0_f64, f64::max);
                            if tighter_reserve < original_reserve {
                                let mut retry_extras = extras;
                                retry_extras.legend_extra_width = tighter_reserve;
                                let retry_layout = PlotLayout::compute_with_extras(
                                    w as f64,
                                    h as f64,
                                    &built.theme,
                                    has_title,
                                    has_subtitle,
                                    has_caption,
                                    has_legend,
                                    x_axis_top,
                                    y_label_width,
                                    x_label_height,
                                    retry_extras,
                                );
                                // `y` is unused here too; `retry_extras`
                                // only tightened `legend_extra_width`, so
                                // its `legend_extra_rows` is still the same
                                // real value. #225's `Inside` size hint is
                                // skipped for the same reason as above.
                                let (retry_x, _, _) = crate::guide::legend::legend_position(
                                    &built.theme,
                                    &retry_layout.plot_area,
                                    retry_extras.legend_extra_rows,
                                    (0.0, 0.0),
                                    &retry_layout.total,
                                );
                                let all_still_fit = plans.iter().all(|(aes, plan)| {
                                    let Some(scale) = built.scales.get(aes) else {
                                        return true;
                                    };
                                    let breaks = scale.breaks();
                                    right_edge_of(retry_x, swatch_and_gap, text_size, plan, &breaks)
                                        <= w as f64
                                });
                                if all_still_fit {
                                    extras.legend_extra_width = tighter_reserve;
                                }
                            }
                        }
                    }
                    for (aes, plan) in &plans {
                        if let Some(ov) = &plan.overflow {
                            let n = built
                                .scales
                                .get(aes)
                                .map(|s| s.breaks().len())
                                .unwrap_or(ov.dropped);
                            let shown = n.saturating_sub(ov.dropped);
                            built.warnings.push(format!(
                                "legend: {} of {n} entries don't fit the chart; the legend shows {shown} and \"+{} more\"",
                                ov.dropped, ov.dropped
                            ));
                        }
                    }
                }
                // Review #158 finding 2: store these exact plans so
                // `draw_legend_planned` draws them verbatim instead of
                // recomputing against the (shorter) post-reservation
                // panel height, which could warn "+7 more" while
                // drawing "+15 more".
                built.legend_plans = plans;
            }
        }

        // #211: a continuous colourbar at `Top`/`Bottom` (or
        // `legend_direction(Horizontal)`) draws wide × short instead of
        // its usual tall × narrow bar (see `draw_continuous_legend_at`),
        // with its tick labels below rather than beside it — reserve the
        // extra band height that needs beyond the flat discrete-row
        // height `legend_band` already assumes. Independent of the
        // `has_wrap_guide` block above (a plot can have a continuous
        // colourbar with no other guide at all), so computed
        // unconditionally here, right before the layout that actually
        // matters reads `extras`.
        if has_legend
            && !matches!(
                built.theme.legend_position,
                crate::theme::LegendPosition::None
            )
        {
            let is_horizontal = match built.theme.legend_direction {
                Some(dir) => matches!(dir, crate::theme::LegendDirection::Horizontal),
                None => matches!(
                    built.theme.legend_position,
                    crate::theme::LegendPosition::Top | crate::theme::LegendPosition::Bottom
                ),
            };
            if is_horizontal {
                let legend_aes =
                    crate::guide::legend::legend_scales_for(&built.scales, &built.suppressed_aes);
                let continuous_colourbar = legend_aes.iter().find_map(|aes| {
                    if !matches!(
                        aes,
                        crate::aes::Aesthetic::Color | crate::aes::Aesthetic::Fill
                    ) {
                        return None;
                    }
                    let scale = built.scales.get(aes)?;
                    if scale.is_discrete() {
                        return None;
                    }
                    Some(scale)
                });
                if let Some(scale) = continuous_colourbar {
                    let title = built
                        .guide_legend
                        .title
                        .as_deref()
                        .unwrap_or_else(|| scale.name());
                    let bar_height = crate::guide::legend::continuous_horizontal_bar_height(
                        &built.theme,
                        !title.is_empty(),
                    );
                    let key_h = built
                        .theme
                        .legend_key_height
                        .max(extras.legend_max_key_size);
                    extras.continuous_horizontal_legend_extra_height =
                        (bar_height - key_h).max(0.0);
                }
            }
        }

        PlotLayout::compute_with_extras(
            w as f64,
            h as f64,
            &built.theme,
            has_title,
            has_subtitle,
            has_caption,
            has_legend,
            x_axis_top,
            y_label_width,
            x_label_height,
            extras,
        )
    }

    /// Fill the background, render the built plot, and flush — for any plotters
    /// backend. (Native only.)
    #[cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
    fn render_into<DB>(
        area: plotters::drawing::DrawingArea<DB, plotters::coord::Shift>,
        built: &crate::build::BuiltPlot,
        layout: &PlotLayout,
    ) -> Result<(), GGError>
    where
        DB: plotters::prelude::DrawingBackend,
        DB::ErrorType: 'static,
    {
        area.fill(&plotters::prelude::WHITE)
            .map_err(|e| GGError::Render(RenderError::BackendError(format!("{:?}", e))))?;
        let mut adapter = PlottersAdapter::new(&area, layout.plot_area.clone());
        PlotRenderer::render(built, &mut adapter).map_err(GGError::Render)?;
        area.present()
            .map_err(|e| GGError::Render(RenderError::BackendError(format!("{:?}", e))))?;
        Ok(())
    }

    /// Save with physical dimensions (inches) and DPI. (Feature `plotters`;
    /// native only.)
    #[cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
    pub fn ggsave(
        self,
        path: &str,
        width_inches: f64,
        height_inches: f64,
        dpi: f64,
    ) -> Result<(), GGError> {
        let w = (width_inches * dpi) as u32;
        let h = (height_inches * dpi) as u32;
        self.save_with_size(path, w, h)
    }

    /// Does this plot have a legend to make room for? (#201) A plot-level
    /// `.aes(...)` mapping always counts; so does a *layer's own*
    /// `.layer_aes(...)` mapping — e.g. `geom_hex`'s usual pattern of
    /// mapping `fill` to a raw column only on that one layer, via
    /// `StatIdentity` after pre-binning, rather than on the plot as a
    /// whole. Checking only `self.mapping` (as before) missed that case
    /// entirely: the legend still drew (the render path always draws
    /// whatever guides the trained scales produce, regardless of this
    /// flag), just with no panel margin ever reserved for it, so it ran
    /// straight off the canvas edge. A layer that opts out with
    /// `show_legend(false)` doesn't count towards its own mapping, mirroring
    /// `PlotBuilder::compute_suppressed_aes`'s later, more precise
    /// (post-build, cross-layer) suppression check — this one only needs to
    /// decide *whether to reserve space at all*, not exactly how much.
    fn has_legend_mapping(&self) -> bool {
        // #241: delegates to `guide::legend::is_legend_aesthetic` (the
        // same list `draw_legend_planned` itself draws from) rather than
        // keeping its own copy — a second, independently-maintained list
        // here had drifted out of sync, missing `Linewidth`.
        use crate::guide::legend::is_legend_aesthetic;
        let plot_level = self
            .mapping
            .mappings
            .iter()
            .any(|m| is_legend_aesthetic(&m.aesthetic));
        plot_level
            || self.layers.iter().any(|layer| {
                if layer.show_legend == Some(false) {
                    return false;
                }
                let effective = if layer.geom.inherit_aes() {
                    self.mapping.merge(&layer.mapping)
                } else {
                    layer.mapping.clone()
                };
                effective
                    .mappings
                    .iter()
                    .any(|m| is_legend_aesthetic(&m.aesthetic))
            })
    }
}

/// Size-independent facts about a built plot that its layout needs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RenderMeta {
    pub(crate) has_title: bool,
    pub(crate) has_subtitle: bool,
    pub(crate) has_caption: bool,
    pub(crate) has_legend: bool,
    pub(crate) legend_bottom_share: Option<f64>,
}

/// Top-level error type.
#[derive(Debug)]
#[non_exhaustive]
pub enum GGError {
    Render(RenderError),
    UnsupportedFormat(String),
    Io(std::io::Error),
    ValidationError(String),
}

impl std::fmt::Display for GGError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GGError::Render(e) => write!(f, "Render error: {e}"),
            GGError::UnsupportedFormat(ext) => write!(f, "Unsupported output format: {ext}"),
            GGError::Io(e) => write!(f, "IO error: {e}"),
            GGError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
        }
    }
}

impl std::error::Error for GGError {}
