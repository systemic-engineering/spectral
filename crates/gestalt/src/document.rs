//! Document primitives — inline spans and enumerations.
//!
//! Span is the inline content unit. ListStyle and ColumnAlign are
//! shared enumerations used by both the encoder and the domain types.

use crate::semantic::{Address, Mark, MarkSet, MathDisplay, RefKind};

// --- Inline / Span ---

/// Inline content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Span {
    /// The primary leaf. Text with optional formatting marks.
    TextSpan { text: String, marks: MarkSet },
    /// Opaque inline code.
    CodeSpan(String),
    /// Opaque math expression.
    MathSpan {
        content: String,
        display: MathDisplay,
    },
    /// Container — URL scopes over children.
    LinkSpan {
        url: String,
        title: String,
        children: Vec<Span>,
    },
    /// Structured alt text.
    ImageSpan {
        url: String,
        title: String,
        alt: Vec<Span>,
    },
    /// Unified reference.
    RefSpan {
        target: Address,
        kind: RefKind,
        display: Vec<Span>,
    },
    /// Emoji with preserved shortcode semantics.
    EmojiSpan { shortcode: String, unicode: String },
    /// Access-gated content.
    SpoilerSpan(Vec<Span>),
    /// Explicit hard break.
    HardBreak,
}

// --- Enumerations ---

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListStyle {
    Ordered,
    Unordered,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnAlign {
    Left,
    Center,
    Right,
    Default,
}

// --- Convenience constructors ---

impl Span {
    /// Plain text with no marks.
    pub fn plain(text: impl Into<String>) -> Self {
        Span::TextSpan {
            text: text.into(),
            marks: MarkSet::new(),
        }
    }

    /// Text with a single mark.
    pub fn marked(text: impl Into<String>, mark: Mark) -> Self {
        let mut marks = MarkSet::new();
        marks.insert(mark);
        Span::TextSpan {
            text: text.into(),
            marks,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_plain_has_no_marks() {
        let s = Span::plain("hello");
        assert!(matches!(s, Span::TextSpan { ref text, ref marks } if text == "hello" && marks.is_empty()));
    }

    #[test]
    fn span_marked_has_one_mark() {
        let s = Span::marked("hello", Mark::Strong);
        assert!(matches!(s, Span::TextSpan { ref marks, .. } if marks.contains(&Mark::Strong)));
    }

    #[test]
    fn list_styles_distinct() {
        assert_ne!(ListStyle::Ordered, ListStyle::Unordered);
    }

    #[test]
    fn column_aligns_distinct() {
        assert_ne!(ColumnAlign::Left, ColumnAlign::Right);
        assert_ne!(ColumnAlign::Center, ColumnAlign::Default);
    }

    #[test]
    fn span_code_round_trip() {
        let s = Span::CodeSpan("let x = 1".into());
        assert!(matches!(s, Span::CodeSpan(ref c) if c == "let x = 1"));
    }

    #[test]
    fn span_hard_break() {
        let s = Span::HardBreak;
        assert!(matches!(s, Span::HardBreak));
    }
}
