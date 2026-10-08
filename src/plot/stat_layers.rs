//! `GGPlot` builder methods for statistical-diagnostic layers: QQ plots
//! against several distributions with confidence bands, step ribbons and
//! ECDF bands, censor marks, horizontal error bars and Cook's-distance
//! contours.

use super::GGPlot;
use crate::data::{DataFrame, Value};
use crate::geom::censor::{GeomCensorMarks, StatCensored};
use crate::geom::cooks::GeomCooksContour;
use crate::geom::errorbarh::GeomErrorbarh;
use crate::geom::qq::{GeomQQ, GeomQQBand, GeomQQLine};
use crate::geom::ribbon::GeomStepribbon;
use crate::geom::step::GeomStep;
use crate::stat::ecdf::{StatEcdf, StatEcdfBand};
use crate::stat::qq::{QQDistribution, StatQQBand, StatQQDist, StatQQLineDist};

impl GGPlot {
    /// QQ points of `y` against a theoretical `distribution` (ggplot2's
    /// `stat_qq(distribution = …, dparams = …)`): `x` = quantiles at R's
    /// `ppoints(n)`, `y` = the sorted sample, per group / panel.
    pub fn stat_qq(self, distribution: QQDistribution) -> Self {
        self.add_geom(GeomQQ::default())
            .stat(StatQQDist::new(distribution))
    }

    /// QQ reference line through the sample / theoretical 1st and 3rd
    /// quartiles (ggplot2's `stat_qq_line`), for any distribution.
    pub fn stat_qq_line(self, distribution: QQDistribution) -> Self {
        self.add_geom(GeomQQLine::default())
            .stat(StatQQLineDist::new(distribution))
    }

    /// QQ confidence envelope (qqplotr's `stat_qq_band`): pointwise
    /// normal-theory or KS band around the quartile line, drawn as a ribbon.
    /// Add it *before* `geom_qq` so the points sit on top.
    pub fn stat_qq_band(self, band: StatQQBand) -> Self {
        self.add_geom(GeomQQBand::default()).stat(band)
    }

    /// [`stat_qq_band`](Self::stat_qq_band) with the default 95% pointwise
    /// band against the standard normal.
    pub fn geom_qq_band(self) -> Self {
        self.stat_qq_band(StatQQBand::default())
    }

    /// A QQ band with custom fill / alpha.
    pub fn geom_qq_band_with(self, geom: GeomQQBand, band: StatQQBand) -> Self {
        self.add_geom_with(geom).stat(band)
    }

    /// Cook's-distance contours for a residuals-vs-leverage plot (`x` =
    /// leverage, `y` = standardized residual), as in R's `plot.lm(which = 5)`:
    /// dashed curves `±√(level · p · (1 − h) / h)` for a model with `p`
    /// parameters, clipped to the panel, labelled with the level, in every
    /// facet panel. They don't train the scales.
    ///
    /// ```
    /// # use ggplot_rs::prelude::*;
    /// let obs: Vec<(String, Vec<Value>)> = vec![
    ///     ("leverage".into(), vec![Value::Float(0.05), Value::Float(0.3)]),
    ///     ("std_residual".into(), vec![Value::Float(-1.2), Value::Float(2.1)]),
    /// ];
    /// let svg = GGPlot::new(obs)
    ///     .aes(Aes::new().x("leverage").y("std_residual"))
    ///     .geom_point()
    ///     .stat_cooks_contour(3, &[0.5, 1.0])
    ///     .render_svg_native()
    ///     .unwrap();
    /// assert!(svg.contains("Cook&#39;s distance = 0.5"));
    /// ```
    pub fn stat_cooks_contour(self, p: usize, levels: &[f64]) -> Self {
        self.geom_cooks_contour_with(GeomCooksContour::new(p, levels))
    }

    /// Cook's-distance contours with custom styling / leverage range.
    pub fn geom_cooks_contour_with(self, geom: GeomCooksContour) -> Self {
        // A one-row placeholder frame (no position columns): the curves are
        // computed from the trained scales at draw time, in every panel.
        let mut data = DataFrame::new();
        data.add_column(".cooks".into(), vec![Value::Float(0.0)]);
        self.add_geom_with(geom).layer_data(data)
    }

    /// Step ribbon between `ymin` and `ymax` (pammtools' `geom_stepribbon`,
    /// `direction = "hv"`): the confidence band of a Kaplan–Meier curve.
    pub fn geom_stepribbon(self) -> Self {
        self.add_geom(GeomStepribbon::default())
    }

    /// Step ribbon with custom fill / alpha / direction (`Hv`, `Vh`, `Mid`).
    pub fn geom_stepribbon_with(self, geom: GeomStepribbon) -> Self {
        self.add_geom_with(geom)
    }

    /// Empirical CDF of `x` as a step line (ggplot2's `stat_ecdf()`), one
    /// per group, padded to the panel edges.
    pub fn stat_ecdf(self) -> Self {
        self.add_geom(GeomStep::default()).stat(StatEcdf)
    }

    /// Simultaneous DKW confidence band for the ECDF of `x` at `level`
    /// (`F̂ ± √(ln(2/(1−level))/(2n))`, clamped to [0, 1]) as a step ribbon.
    /// Add it before [`stat_ecdf`](Self::stat_ecdf) so the line is on top.
    pub fn stat_ecdf_band(self, level: f64) -> Self {
        self.add_geom(GeomStepribbon::default())
            .stat(StatEcdfBand::new(level))
    }

    /// Censor marks (`+`) at the rows whose `censor_col` is `> 0` / `true`,
    /// at the layer's `x` (time) and `y` (survival) — e.g. a Kaplan–Meier
    /// table with an `n_censor` column. Coloured like the curves when the
    /// plot maps `color`.
    pub fn geom_censor_marks(self, censor_col: &str) -> Self {
        self.add_geom(GeomCensorMarks::default())
            .stat(StatCensored::new(censor_col))
    }

    /// Censor marks with custom size / colour / shape.
    pub fn geom_censor_marks_with(self, geom: GeomCensorMarks, censor_col: &str) -> Self {
        self.add_geom_with(geom).stat(StatCensored::new(censor_col))
    }

    /// Horizontal error bars from `xmin` to `xmax` at `y`.
    pub fn geom_errorbarh(self) -> Self {
        self.add_geom(GeomErrorbarh::default())
    }

    /// Horizontal error bars with custom styling.
    pub fn geom_errorbarh_with(self, geom: GeomErrorbarh) -> Self {
        self.add_geom_with(geom)
    }
}
