//! Observation primitives. Read-only spectral-db access.

use terni::Imperfect;
use super::loss::ObserveLoss;

/// A single observation with a label and total dimension count.
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    label: String,
    total_dimensions: u32,
}

impl Observation {
    pub fn new(label: String, total_dimensions: u32) -> Self {
        Observation { label, total_dimensions }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn dimensions(&self) -> u32 {
        self.total_dimensions
    }

    /// Measure with N observed dimensions.
    /// All observed = Success. Some dark = Partial(obs, ObserveLoss). None = Failure.
    pub fn measure(&self, observed: u32) -> Imperfect<&Observation, String, ObserveLoss> {
        if observed >= self.total_dimensions {
            Imperfect::Success(self)
        } else if observed > 0 {
            let dark = self.total_dimensions - observed;
            Imperfect::Partial(self, ObserveLoss { dark_dimensions: dark })
        } else {
            Imperfect::Failure(
                "no dimensions observed".to_string(),
                ObserveLoss { dark_dimensions: self.total_dimensions },
            )
        }
    }
}
