use crate::aes::Aesthetic;
use crate::data::DataFrame;
use crate::scale::ScaleSet;

use super::Stat;

/// Applies an arbitrary function to each group's data — ggplot2 4.0's
/// `stat_manual(fun)`. The function gets the group's rows (aesthetic
/// columns `x`, `y`, `colour` … plus the raw columns) and returns the rows to
/// draw, e.g. a centroid or a convex hull.
pub struct StatManual {
    pub fun: Box<dyn Fn(&DataFrame) -> DataFrame + Send + Sync>,
}

impl StatManual {
    pub fn new(fun: impl Fn(&DataFrame) -> DataFrame + Send + Sync + 'static) -> Self {
        StatManual { fun: Box::new(fun) }
    }
}

impl Stat for StatManual {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        (self.fun)(data)
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        Vec::new()
    }

    fn name(&self) -> &str {
        "manual"
    }
}
