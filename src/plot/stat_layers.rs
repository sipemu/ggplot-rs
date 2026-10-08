//! `GGPlot` builder methods for statistical-diagnostic layers: QQ plots
//! against several distributions with confidence bands, step ribbons and
//! ECDF bands, censor marks, horizontal error bars and Cook's-distance
//! contours.

use super::GGPlot;
use crate::geom::qq::{GeomQQ, GeomQQBand, GeomQQLine};
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
}
