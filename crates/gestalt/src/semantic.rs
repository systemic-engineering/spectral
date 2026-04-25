//! Semantic: the shared meaning vocabulary.
//! What things mean — roles, intents, callout kinds, ref kinds, marks.

use std::collections::BTreeSet;

/// Semantic role a node plays in document structure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Claim,
    Evidence,
    Example,
    Aside,
    Defining,
    Instruction,
    Summary,
    Transition,
}

/// Metadata on a block. Each variant is one `<!-- ... -->` comment line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Meta {
    /// Block identifier: `<!-- id: hook -->`
    Id(String),
    /// Semantic role: `<!-- role: claim -->`
    Role(Role),
    /// Generic extension: `<!-- key: value -->`
    Extension { key: String, value: String },
}

/// Formatting annotations on text spans.
/// A set, not a nesting order. Strong(Emph(x)) == Emph(Strong(x)).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mark {
    Strong,
    Emphasis,
    Strikethrough,
    Highlight,
    Superscript,
    Subscript,
}

/// Author's own aside with semantic kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalloutKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

/// Unified reference kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefKind {
    Footnote,
    Wiki,
    Citation,
    CrossRef,
}

/// How you point at a location in a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Address {
    Named(String),
    Line(i64),
    Point { line: i64, col: i64 },
    Span { line: i64, from: i64, to: i64 },
    LineRange { from: i64, to: i64 },
}

/// Math display mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MathDisplay {
    InlineMath,
    DisplayMath,
}

/// A set of marks on a text span.
pub type MarkSet = BTreeSet<Mark>;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_set_is_order_independent() {
        let mut a = MarkSet::new();
        a.insert(Mark::Strong);
        a.insert(Mark::Emphasis);

        let mut b = MarkSet::new();
        b.insert(Mark::Emphasis);
        b.insert(Mark::Strong);

        assert_eq!(a, b);
    }

    #[test]
    fn meta_id_round_trip() {
        let m = Meta::Id("hook".into());
        assert!(matches!(m, Meta::Id(ref s) if s == "hook"));
    }

    #[test]
    fn meta_role_round_trip() {
        let m = Meta::Role(Role::Claim);
        assert!(matches!(m, Meta::Role(Role::Claim)));
    }

    #[test]
    fn meta_extension_round_trip() {
        let m = Meta::Extension { key: "foo".into(), value: "bar".into() };
        assert!(matches!(m, Meta::Extension { ref key, ref value } if key == "foo" && value == "bar"));
    }

    #[test]
    fn callout_kinds_distinct() {
        assert_ne!(CalloutKind::Note, CalloutKind::Tip);
        assert_ne!(CalloutKind::Warning, CalloutKind::Caution);
    }

    #[test]
    fn ref_kinds_distinct() {
        assert_ne!(RefKind::Footnote, RefKind::Wiki);
    }

    #[test]
    fn math_display_distinct() {
        assert_ne!(MathDisplay::InlineMath, MathDisplay::DisplayMath);
    }
}
