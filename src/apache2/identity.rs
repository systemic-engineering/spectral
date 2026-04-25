//! Actor identity types. Name, BiasChain, bias resolution.

use terni::{Imperfect, Loss};

/// Loss accumulated during name resolution.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NamingLoss {
    pub candidates_considered: u32,
    pub candidates_rejected: u32,
}

impl Loss for NamingLoss {
    fn zero() -> Self {
        NamingLoss { candidates_considered: 0, candidates_rejected: 0 }
    }

    fn total() -> Self {
        NamingLoss { candidates_considered: u32::MAX, candidates_rejected: u32::MAX }
    }

    fn is_zero(&self) -> bool {
        self.candidates_considered == 0 && self.candidates_rejected == 0
    }

    fn combine(self, other: Self) -> Self {
        NamingLoss {
            candidates_considered: self.candidates_considered + other.candidates_considered,
            candidates_rejected: self.candidates_rejected + other.candidates_rejected,
        }
    }
}

/// A resolved actor name, or silence.
#[derive(Debug, Clone, PartialEq)]
pub enum Name {
    Named(String, NamingLoss),
    Silent,
}

impl Name {
    pub fn is_named(&self) -> bool {
        matches!(self, Name::Named(_, _))
    }

    pub fn is_silent(&self) -> bool {
        matches!(self, Name::Silent)
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Name::Named(s, _) => Some(s),
            Name::Silent => None,
        }
    }

    pub fn loss(&self) -> Option<&NamingLoss> {
        match self {
            Name::Named(_, loss) if !loss.is_zero() => Some(loss),
            _ => None,
        }
    }
}

impl From<Imperfect<String, (), NamingLoss>> for Name {
    fn from(imp: Imperfect<String, (), NamingLoss>) -> Self {
        match imp {
            Imperfect::Success(s) => Name::Named(s, NamingLoss::zero()),
            Imperfect::Partial(s, loss) => Name::Named(s, loss),
            Imperfect::Failure(_, _) => Name::Silent,
        }
    }
}

/// An ordered chain of bias terms for actor resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct BiasChain {
    ordering: Vec<String>,
}

impl BiasChain {
    pub fn new(ordering: Vec<String>) -> Self {
        BiasChain { ordering }
    }

    pub fn len(&self) -> usize {
        self.ordering.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ordering.is_empty()
    }

    pub fn first(&self) -> Option<&str> {
        self.ordering.first().map(|s| s.as_str())
    }

    pub fn position(&self, name: &str) -> Option<usize> {
        self.ordering.iter().position(|s| s == name)
    }

    pub fn ordering(&self) -> &[String] {
        &self.ordering
    }
}
