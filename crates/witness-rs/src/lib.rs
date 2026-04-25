//! Witness — operation witnessing for spectral.
//!
//! A witness observes a spectral operation (diff, commit, merge) and produces
//! a signed attestation. The attestation is content-addressed: same operation
//! = same witness Oid.

use prism_core::oid::{Addressable, Oid};

/// What kind of operation was witnessed.
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Diff,
    Commit,
    Merge,
    Fork,
}

/// Visibility of the attestation.
#[derive(Clone, Debug, PartialEq)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

/// A witness attestation. Content-addressed proof that an operation occurred.
#[derive(Clone, Debug, PartialEq)]
pub struct Attestation {
    /// The operation that was witnessed.
    pub operation: Operation,
    /// Oid of the input (before).
    pub before: Oid,
    /// Oid of the output (after).
    pub after: Oid,
    /// Who witnessed it.
    pub witness: String,
    /// Visibility constraint.
    pub visibility: Visibility,
}

impl Addressable for Attestation {
    fn oid(&self) -> Oid {
        let content = format!(
            "attestation:{:?}:{}:{}:{}",
            self.operation, self.before, self.after, self.witness
        );
        Oid::hash(content.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_attestation() -> Attestation {
        Attestation {
            operation: Operation::Commit,
            before: Oid::hash(b"before-state"),
            after: Oid::hash(b"after-state"),
            witness: "reed@systemic.engineer".into(),
            visibility: Visibility::Public,
        }
    }

    #[test]
    fn attestation_is_content_addressed() {
        let a = sample_attestation();
        let b = sample_attestation();
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn different_operation_different_oid() {
        let a = sample_attestation();
        let mut b = sample_attestation();
        b.operation = Operation::Diff;
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn different_before_different_oid() {
        let a = sample_attestation();
        let mut b = sample_attestation();
        b.before = Oid::hash(b"different");
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn different_after_different_oid() {
        let a = sample_attestation();
        let mut b = sample_attestation();
        b.after = Oid::hash(b"different");
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn different_witness_different_oid() {
        let a = sample_attestation();
        let mut b = sample_attestation();
        b.witness = "other@example.com".into();
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn oid_is_not_dark() {
        let a = sample_attestation();
        assert!(!a.oid().is_dark());
    }

    #[test]
    fn visibility_variants() {
        assert_ne!(Visibility::Public, Visibility::Private);
        assert_ne!(Visibility::Protected, Visibility::Private);
    }

    #[test]
    fn operation_variants() {
        assert_ne!(Operation::Diff, Operation::Commit);
        assert_ne!(Operation::Merge, Operation::Fork);
    }
}
