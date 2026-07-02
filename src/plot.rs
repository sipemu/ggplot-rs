use plotters::prelude::IntoDrawingArea;

use crate::aes::Aes;
use crate::annotate::Annotation;
use crate::build::PlotBuilder;
use crate::coord::cartesian::CoordCartesian;
use crate::coord::fixed::CoordFixed;
use crate::coord::flip::CoordFlip;
use crate::coord::polar::CoordPolar;
use crate::coord::Coord;
use crate::data::{DataFrame, GGData};
use crate::facet::{Facet, FacetLabeller, FacetScales};
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
use crate::render::plotters_backend::PlottersAdapter;
use crate::render::renderer::PlotRenderer;
use crate::render::RenderError;
use crate::scale::continuous::ScaleContinuous;
use crate::scale::transform::ScaleTransform;
use crate::scale::Scale;
use crate::stat::Stat;
use crate::theme::Theme;

/// Labels for the plot.
#[derive(Clone, Debug, Default)]
pub struct Labels {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub x: Option<String>,
    pub y: Option<String>,
    pub caption: Option<String>,
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
}

impl GGPlot {
    /// Create a new plot with the given data source.
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
        }
    }

    /// Set the plot-level aesthetic mapping.
    pub fn aes(mut self, mapping: Aes) -> Self {
        self.mapping = mapping;
        self
    }

    // ─── Geom shortcuts ──────────────────────────────────────────

    pub fn geom_point(self) -> Self {
        self.add_geom(GeomPoint::default())
    }

    pub fn geom_point_with(self, geom: GeomPoint) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_line(self) -> Self {
        self.add_geom(GeomLine::default())
    }

    pub fn geom_line_with(self, geom: GeomLine) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_bar(self) -> Self {
        self.add_geom(GeomBar::default())
    }

    pub fn geom_bar_with(self, geom: GeomBar) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_histogram(self) -> Self {
        self.add_geom(GeomHistogram::default())
    }

    pub fn geom_histogram_with(self, geom: GeomHistogram) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_boxplot(self) -> Self {
        self.add_geom(GeomBoxplot::default())
    }

    pub fn geom_boxplot_with(self, geom: GeomBoxplot) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_smooth(self) -> Self {
        self.add_geom(GeomSmooth::default())
    }

    pub fn geom_smooth_with(self, geom: GeomSmooth) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_col(self) -> Self {
        self.add_geom(GeomCol::default())
    }

    pub fn geom_col_with(self, geom: GeomCol) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_hline(self, yintercept: f64) -> Self {
        self.add_geom(GeomHline::new(yintercept))
    }

    /// Add a horizontal reference line with custom styling (color/linetype/width).
    pub fn geom_hline_with(self, geom: GeomHline) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_vline(self, xintercept: f64) -> Self {
        self.add_geom(GeomVline::new(xintercept))
    }

    /// Add a vertical reference line with custom styling (color/linetype/width).
    pub fn geom_vline_with(self, geom: GeomVline) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_abline(self, slope: f64, intercept: f64) -> Self {
        self.add_geom(GeomAbline::new(slope, intercept))
    }

    /// Add a slope/intercept reference line with custom styling.
    pub fn geom_abline_with(self, geom: GeomAbline) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_text(self) -> Self {
        self.add_geom(GeomText::default())
    }

    pub fn geom_text_with(self, geom: GeomText) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_label(self) -> Self {
        self.add_geom(GeomLabel::default())
    }

    pub fn geom_label_with(self, geom: GeomLabel) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_area(self) -> Self {
        self.add_geom(GeomArea::default())
    }

    pub fn geom_area_with(self, geom: GeomArea) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_ribbon(self) -> Self {
        self.add_geom(GeomRibbon::default())
    }

    pub fn geom_ribbon_with(self, geom: GeomRibbon) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_errorbar(self) -> Self {
        self.add_geom(GeomErrorbar::default())
    }

    pub fn geom_errorbar_with(self, geom: GeomErrorbar) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_segment(self) -> Self {
        self.add_geom(GeomSegment::default())
    }

    pub fn geom_segment_with(self, geom: GeomSegment) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_density(self) -> Self {
        self.add_geom(GeomDensity::default())
    }

    pub fn geom_density_with(self, geom: GeomDensity) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_rug(self) -> Self {
        self.add_geom(GeomRug::default())
    }

    pub fn geom_rug_with(self, geom: GeomRug) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_jitter(self) -> Self {
        self.add_geom(GeomJitter::default())
    }

    pub fn geom_jitter_with(self, geom: GeomJitter) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_path(self) -> Self {
        self.add_geom(GeomPath::default())
    }

    pub fn geom_path_with(self, geom: GeomPath) -> Self {
        self.add_geom(geom)
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
        self.add_geom(geom)
    }

    pub fn geom_freqpoly(self) -> Self {
        self.add_geom(GeomFreqpoly::default())
    }

    pub fn geom_freqpoly_with(self, geom: GeomFreqpoly) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_linerange(self) -> Self {
        self.add_geom(GeomLinerange::default())
    }

    pub fn geom_linerange_with(self, geom: GeomLinerange) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_pointrange(self) -> Self {
        self.add_geom(GeomPointrange::default())
    }

    pub fn geom_pointrange_with(self, geom: GeomPointrange) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_crossbar(self) -> Self {
        self.add_geom(GeomCrossbar::default())
    }

    pub fn geom_crossbar_with(self, geom: GeomCrossbar) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_spoke(self) -> Self {
        self.add_geom(GeomSpoke::default())
    }

    pub fn geom_spoke_with(self, geom: GeomSpoke) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_rect(self) -> Self {
        self.add_geom(GeomRect::default())
    }

    pub fn geom_rect_with(self, geom: GeomRect) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_tile(self) -> Self {
        self.add_geom(GeomTile::default())
    }

    pub fn geom_tile_with(self, geom: GeomTile) -> Self {
        self.add_geom(geom)
    }

    /// Dense regular grid of filled cells (heatmap/raster) from x, y, fill.
    pub fn geom_raster(self) -> Self {
        self.add_geom(crate::geom::raster::GeomRaster::default())
    }

    pub fn geom_raster_with(self, geom: crate::geom::raster::GeomRaster) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_polygon(self) -> Self {
        self.add_geom(GeomPolygon::default())
    }

    pub fn geom_polygon_with(self, geom: GeomPolygon) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_curve(self) -> Self {
        self.add_geom(GeomCurve::default())
    }

    pub fn geom_curve_with(self, geom: GeomCurve) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_violin(self) -> Self {
        self.add_geom(GeomViolin::default())
    }

    pub fn geom_violin_with(self, geom: GeomViolin) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_dotplot(self) -> Self {
        self.add_geom(GeomDotplot::default())
    }

    pub fn geom_dotplot_with(self, geom: GeomDotplot) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_qq(self) -> Self {
        self.add_geom(GeomQQ::default())
    }

    pub fn geom_qq_with(self, geom: GeomQQ) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_qq_line(self) -> Self {
        self.add_geom(GeomQQLine::default())
    }

    pub fn geom_qq_line_with(self, geom: GeomQQLine) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_bin2d(self) -> Self {
        self.add_geom(GeomBin2d::default())
    }

    pub fn geom_bin2d_with(self, geom: GeomBin2d) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_hex(self) -> Self {
        self.add_geom(GeomHex::default())
    }

    pub fn geom_hex_with(self, geom: GeomHex) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_count(self) -> Self {
        self.add_geom(GeomCount::default())
    }

    pub fn geom_count_with(self, geom: GeomCount) -> Self {
        self.add_geom(geom)
    }

    pub fn geom_contour(self) -> Self {
        self.add_geom(GeomContour::default())
    }

    pub fn geom_contour_with(self, geom: GeomContour) -> Self {
        self.add_geom(geom)
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
        self.add_geom(geom)
    }

    pub fn geom_blank(self) -> Self {
        self.add_geom(GeomBlank)
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
        if let Some(layer) = self.layers.last_mut() {
            layer.mapping = mapping;
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

    pub fn scale_alpha(mut self, s: crate::scale::alpha::ScaleAlphaContinuous) -> Self {
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

    pub fn coord_polar_with(mut self, coord: CoordPolar) -> Self {
        self.coord = Box::new(coord);
        self
    }

    // ─── Theme ───────────────────────────────────────────────────

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the brand/primary color used as the default for single-series geoms
    /// that have no color/fill aesthetic mapped. Composes with any theme — one
    /// render process can serve different tenants' brands at render time.
    pub fn primary_color(mut self, color: (u8, u8, u8)) -> Self {
        self.theme.primary = Some(color);
        self
    }

    pub fn theme_minimal(mut self) -> Self {
        self.theme = crate::theme::presets::theme_minimal();
        self
    }

    pub fn theme_bw(mut self) -> Self {
        self.theme = crate::theme::presets::theme_bw();
        self
    }

    pub fn theme_gray(mut self) -> Self {
        self.theme = crate::theme::presets::theme_gray();
        self
    }

    pub fn theme_classic(mut self) -> Self {
        self.theme = crate::theme::presets::theme_classic();
        self
    }

    pub fn theme_linedraw(mut self) -> Self {
        self.theme = crate::theme::presets::theme_linedraw();
        self
    }

    pub fn theme_light(mut self) -> Self {
        self.theme = crate::theme::presets::theme_light();
        self
    }

    pub fn theme_dark(mut self) -> Self {
        self.theme = crate::theme::presets::theme_dark();
        self
    }

    pub fn theme_void(mut self) -> Self {
        self.theme = crate::theme::presets::theme_void();
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
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.labels.title = Some(title.to_string());
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
    pub fn save(self, path: &str) -> Result<(), GGError> {
        self.save_with_size(path, 800, 600)
    }

    /// Build and save with custom dimensions.
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
    /// serving charts from a web/MCP service.
    pub fn render_svg(self) -> Result<String, GGError> {
        self.render_svg_with_size(800, 600)
    }

    /// Render the plot to an in-memory SVG document with custom dimensions.
    pub fn render_svg_with_size(self, w: u32, h: u32) -> Result<String, GGError> {
        let (built, layout) = self.prepare(w, h)?;
        let mut buf = String::new();
        {
            let backend = plotters::prelude::SVGBackend::with_string(&mut buf, (w, h));
            Self::render_into(backend.into_drawing_area(), &built, &layout)?;
        }
        Ok(buf)
    }

    /// Render the plot to in-memory PNG bytes (default 800x600).
    ///
    /// Returns a fully-encoded PNG, ready to write to an HTTP response or
    /// embed as a data URI — no temp files involved.
    pub fn render_png(self) -> Result<Vec<u8>, GGError> {
        self.render_png_with_size(800, 600)
    }

    /// Render the plot to in-memory PNG bytes with custom dimensions.
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
    fn prepare(self, w: u32, h: u32) -> Result<(crate::build::BuiltPlot, PlotLayout), GGError> {
        let plot = self;

        let has_title = plot.labels.title.is_some();
        let has_subtitle = plot.labels.subtitle.is_some();
        let has_caption = plot.labels.caption.is_some();
        let has_legend = plot.has_legend_mapping();
        let x_label = plot.labels.x.clone();
        let y_label = plot.labels.y.clone();

        let mut built = PlotBuilder::build(plot)?;

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

        let layout = PlotLayout::compute_full(
            w as f64,
            h as f64,
            &built.theme,
            has_title,
            has_subtitle,
            has_caption,
            has_legend,
        );

        Ok((built, layout))
    }

    /// Fill the background, render the built plot, and flush — for any backend.
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

    /// Save with physical dimensions (inches) and DPI.
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

    fn has_legend_mapping(&self) -> bool {
        self.mapping.mappings.iter().any(|m| {
            matches!(
                m.aesthetic,
                crate::aes::Aesthetic::Color
                    | crate::aes::Aesthetic::Fill
                    | crate::aes::Aesthetic::Shape
                    | crate::aes::Aesthetic::Linetype
                    | crate::aes::Aesthetic::Size
                    | crate::aes::Aesthetic::Alpha
            )
        })
    }
}

/// Top-level error type.
#[derive(Debug)]
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
