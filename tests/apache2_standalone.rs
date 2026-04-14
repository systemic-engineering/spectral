//! Standalone tests for apache2 types: Loss, Signal, Identity, Runtime.

use spectral::apache2::loss::{InitLoss, ObserveLoss};
use spectral::apache2::signal::{Signal, SignalKind};
use spectral::apache2::identity::{Name, NamingLoss, BiasChain};
use spectral::apache2::runtime::Runtime;
use terni::{Imperfect, Loss};

// ── InitLoss monoid laws ──────────────────────────────────────────────

#[test]
fn init_loss_zero() {
    let z = InitLoss::zero();
    assert_eq!(z.grammars_compiled, 0);
    assert_eq!(z.grammars_with_warnings, 0);
    assert!(z.is_zero());
}

#[test]
fn init_loss_combine_adds() {
    let a = InitLoss { grammars_compiled: 3, grammars_with_warnings: 1 };
    let b = InitLoss { grammars_compiled: 2, grammars_with_warnings: 2 };
    let c = a.combine(b);
    assert_eq!(c.grammars_compiled, 5);
    assert_eq!(c.grammars_with_warnings, 3);
}

#[test]
fn init_loss_zero_is_identity() {
    let a = InitLoss { grammars_compiled: 4, grammars_with_warnings: 1 };
    assert_eq!(a.clone().combine(InitLoss::zero()), a);
    assert_eq!(InitLoss::zero().combine(a.clone()), a);
}

#[test]
fn init_loss_total() {
    let t = InitLoss::total();
    assert_eq!(t.grammars_compiled, u32::MAX);
    assert_eq!(t.grammars_with_warnings, u32::MAX);
    assert!(!t.is_zero());
}

// ── ObserveLoss monoid laws ───────────────────────────────────────────

#[test]
fn observe_loss_zero() {
    let z = ObserveLoss::zero();
    assert_eq!(z.dark_dimensions, 0);
    assert!(z.is_zero());
}

#[test]
fn observe_loss_combine_takes_max() {
    let a = ObserveLoss { dark_dimensions: 3 };
    let b = ObserveLoss { dark_dimensions: 7 };
    let c = a.combine(b);
    assert_eq!(c.dark_dimensions, 7);
}

#[test]
fn observe_loss_zero_is_identity() {
    let a = ObserveLoss { dark_dimensions: 5 };
    assert_eq!(a.clone().combine(ObserveLoss::zero()), a);
    assert_eq!(ObserveLoss::zero().combine(a.clone()), a);
}

#[test]
fn observe_loss_total() {
    let t = ObserveLoss::total();
    assert_eq!(t.dark_dimensions, 16);
    assert!(!t.is_zero());
}

// ── Signal ────────────────────────────────────────────────────────────

#[test]
fn signal_construction() {
    let s = Signal::new(SignalKind::Tick, "hello".to_string());
    assert_eq!(s.kind(), &SignalKind::Tick);
    assert_eq!(s.payload(), "hello");
}

#[test]
fn signal_kind_variants() {
    let kinds = vec![
        SignalKind::Init,
        SignalKind::Tick,
        SignalKind::Tock,
        SignalKind::Crystal,
        SignalKind::Observe,
    ];
    // All five variants exist and are distinct
    for (i, a) in kinds.iter().enumerate() {
        for (j, b) in kinds.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b);
            }
        }
    }
}

// ── Identity: Name ────────────────────────────────────────────────────

#[test]
fn name_from_imperfect_success_is_named() {
    let imp: Imperfect<String, (), NamingLoss> = Imperfect::Success("alice".to_string());
    let name: Name = Name::from(imp);
    assert!(name.is_named());
    assert!(!name.is_silent());
    assert_eq!(name.text(), Some("alice"));
    assert!(name.loss().is_none());
}

#[test]
fn name_from_imperfect_partial_is_named_with_loss() {
    let loss = NamingLoss { candidates_considered: 10, candidates_rejected: 7 };
    let imp: Imperfect<String, (), NamingLoss> = Imperfect::Partial("bob".to_string(), loss.clone());
    let name: Name = Name::from(imp);
    assert!(name.is_named());
    assert_eq!(name.text(), Some("bob"));
    assert_eq!(name.loss(), Some(&loss));
}

#[test]
fn name_from_imperfect_failure_is_silent() {
    let loss = NamingLoss { candidates_considered: 5, candidates_rejected: 5 };
    let imp: Imperfect<String, (), NamingLoss> = Imperfect::Failure((), loss);
    let name: Name = Name::from(imp);
    assert!(name.is_silent());
    assert!(!name.is_named());
    assert_eq!(name.text(), None);
    assert!(name.loss().is_none());
}

// ── Identity: BiasChain ───────────────────────────────────────────────

#[test]
fn bias_chain_ordering() {
    let chain = BiasChain::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    assert_eq!(chain.len(), 3);
    assert!(!chain.is_empty());
    assert_eq!(chain.first(), Some("a"));
    assert_eq!(chain.position("b"), Some(1));
    assert_eq!(chain.position("z"), None);
    assert_eq!(chain.ordering(), &["a", "b", "c"]);
}

#[test]
fn bias_chain_empty() {
    let chain = BiasChain::new(vec![]);
    assert!(chain.is_empty());
    assert_eq!(chain.len(), 0);
    assert_eq!(chain.first(), None);
}

// ── Runtime trait ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
struct TestLoss(u32);

impl Loss for TestLoss {
    fn zero() -> Self { TestLoss(0) }
    fn total() -> Self { TestLoss(u32::MAX) }
    fn is_zero(&self) -> bool { self.0 == 0 }
    fn combine(self, other: Self) -> Self { TestLoss(self.0 + other.0) }
}

struct TestRuntime { counter: u32 }

impl Runtime for TestRuntime {
    type State = u32;
    type Signal = String;
    type Error = String;
    type L = TestLoss;

    fn tick(&mut self, signal: Self::Signal) -> Imperfect<Self::State, Self::Error, Self::L> {
        self.counter += 1;
        if signal == "fail" {
            Imperfect::Failure("failed".to_string(), TestLoss(1))
        } else if signal == "warn" {
            Imperfect::Partial(self.counter, TestLoss(1))
        } else {
            Imperfect::Success(self.counter)
        }
    }
}

#[test]
fn runtime_returns_success() {
    let mut rt = TestRuntime { counter: 0 };
    let result = rt.tick("go".to_string());
    assert_eq!(result, Imperfect::Success(1));
}

#[test]
fn runtime_returns_partial() {
    let mut rt = TestRuntime { counter: 0 };
    let result = rt.tick("warn".to_string());
    assert_eq!(result, Imperfect::Partial(1, TestLoss(1)));
}

#[test]
fn runtime_returns_failure() {
    let mut rt = TestRuntime { counter: 0 };
    let result = rt.tick("fail".to_string());
    assert_eq!(result, Imperfect::Failure("failed".to_string(), TestLoss(1)));
}

// ── NamingLoss monoid ─────────────────────────────────────────────────

#[test]
fn naming_loss_zero() {
    let z = NamingLoss::zero();
    assert_eq!(z.candidates_considered, 0);
    assert_eq!(z.candidates_rejected, 0);
    assert!(z.is_zero());
}

#[test]
fn naming_loss_combine() {
    let a = NamingLoss { candidates_considered: 3, candidates_rejected: 1 };
    let b = NamingLoss { candidates_considered: 5, candidates_rejected: 2 };
    let c = a.combine(b);
    assert_eq!(c.candidates_considered, 8);
    assert_eq!(c.candidates_rejected, 3);
}
