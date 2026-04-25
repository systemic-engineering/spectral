//! Loss type re-exports and spectral-specific loss types.

pub use terni::{Loss, ConvergenceLoss, ApertureLoss, RoutingLoss};

/// Loss accumulated during grammar initialization.
/// Combines by addition. Total is MAX.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InitLoss {
    pub grammars_compiled: u32,
    pub grammars_with_warnings: u32,
}

impl Loss for InitLoss {
    fn zero() -> Self {
        InitLoss { grammars_compiled: 0, grammars_with_warnings: 0 }
    }

    fn total() -> Self {
        InitLoss { grammars_compiled: u32::MAX, grammars_with_warnings: u32::MAX }
    }

    fn is_zero(&self) -> bool {
        self.grammars_compiled == 0 && self.grammars_with_warnings == 0
    }

    fn combine(self, other: Self) -> Self {
        InitLoss {
            grammars_compiled: self.grammars_compiled + other.grammars_compiled,
            grammars_with_warnings: self.grammars_with_warnings + other.grammars_with_warnings,
        }
    }
}

/// Loss from observation: dimensions that couldn't be measured.
/// Combines by max. Total is 16.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObserveLoss {
    pub dark_dimensions: u32,
}

impl Loss for ObserveLoss {
    fn zero() -> Self {
        ObserveLoss { dark_dimensions: 0 }
    }

    fn total() -> Self {
        ObserveLoss { dark_dimensions: 16 }
    }

    fn is_zero(&self) -> bool {
        self.dark_dimensions == 0
    }

    fn combine(self, other: Self) -> Self {
        ObserveLoss {
            dark_dimensions: self.dark_dimensions.max(other.dark_dimensions),
        }
    }
}
