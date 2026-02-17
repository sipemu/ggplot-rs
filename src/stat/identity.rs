use crate::aes::Aesthetic;
use crate::data::DataFrame;
use crate::scale::ScaleSet;

use super::Stat;

/// Passthrough stat — no transformation.
pub struct StatIdentity;

impl Stat for StatIdentity {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        data.clone()
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![]
    }

    fn name(&self) -> &str {
        "identity"
    }
}
