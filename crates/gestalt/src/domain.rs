//! domain — vocabulary as trait.
//!
//! A Domain defines the language — what nodes exist in a gestalt tree.
//! Document is the primary implementation. Form and spectral visualization
//! blocks compose with it.

use crate::document::{ColumnAlign, ListStyle, Span};
use crate::semantic::{CalloutKind, Meta};
use prism_core::oid::Oid;
use std::borrow::Cow;

/// The tree's vocabulary. Defines the domain's language.
pub trait Domain: Clone + std::fmt::Debug + PartialEq + Eq {
    type Language: Clone + std::fmt::Debug + PartialEq + Eq + Encode;

    fn id() -> &'static str;
    fn local_name(kind: &Self::Language) -> Cow<'static, str>;
}

/// Encode a domain language node to bytes for content addressing.
pub trait Encode {
    fn encode(&self) -> Vec<u8>;
}

/// The document vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document;

// ---------------------------------------------------------------------------
// Gestalt<D> — the unit of meaning
// ---------------------------------------------------------------------------

/// The unit of meaning. Content and metadata, nothing else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gestalt<D: Domain = Document> {
    pub domain: D,
    pub head: Vec<Meta>,
    pub body: Vec<Node<D>>,
}

impl Gestalt<Document> {
    /// Empty document gestalt.
    pub fn empty() -> Self {
        Gestalt {
            domain: Document,
            head: vec![],
            body: vec![],
        }
    }

    /// Document gestalt from nodes.
    pub fn from_nodes(body: Vec<Node<Document>>) -> Self {
        Gestalt {
            domain: Document,
            head: vec![],
            body,
        }
    }
}

impl<D: Domain> Gestalt<D> {
    /// Content-addressed identity. Derived from domain id + child OIDs.
    pub fn oid(&self) -> Oid {
        let child_oids: String = self
            .body
            .iter()
            .map(|n| n.oid().to_string())
            .collect::<Vec<_>>()
            .join(":");
        Oid::hash(format!("{}:{}", D::id(), child_oids).as_bytes())
    }
}

/// What kind of document node this is.
/// Carries only variant-specific data. No children, no meta — those are on Node<D>.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentKind {
    Section { level: usize, title: Vec<Span> },
    Paragraph { content: Vec<Span> },
    CodeBlock { language: String, content: String },
    Quote { attribution: Option<Vec<Span>> },
    Callout { kind: CalloutKind, title: String },
    List { style: ListStyle, start: usize },
    ListItem { checked: Option<bool> },
    DefinitionList,
    Table { columns: Vec<ColumnAlign> },
    Figure { caption: Option<Vec<Span>> },
    Separator,
    Breath,
    RawBlock { content: String, format: String },
    Embedded(Box<Gestalt<Document>>),
}

impl Encode for DocumentKind {
    fn encode(&self) -> Vec<u8> {
        let s = match self {
            DocumentKind::Section { level, title } => {
                format!("{} {}", "#".repeat(*level), crate::encode::spans(title))
            }
            DocumentKind::Paragraph { content } => crate::encode::spans(content),
            DocumentKind::CodeBlock { language, content } => {
                format!("```{}\n{}\n```", language, content)
            }
            DocumentKind::Quote { attribution } => match attribution {
                Some(spans) => format!("> — {}", crate::encode::spans(spans)),
                None => ">".into(),
            },
            DocumentKind::Callout { kind, title } => {
                let k = match kind {
                    CalloutKind::Note => "NOTE",
                    CalloutKind::Tip => "TIP",
                    CalloutKind::Important => "IMPORTANT",
                    CalloutKind::Warning => "WARNING",
                    CalloutKind::Caution => "CAUTION",
                };
                format!("> [!{}] {}", k, title)
            }
            DocumentKind::List { style, start } => {
                let s = match style {
                    ListStyle::Ordered => "ol",
                    ListStyle::Unordered => "ul",
                };
                format!("{}:{}", s, start)
            }
            DocumentKind::ListItem { checked } => match checked {
                Some(true) => "[x]".into(),
                Some(false) => "[ ]".into(),
                None => "li".into(),
            },
            DocumentKind::DefinitionList => "dl".into(),
            DocumentKind::Table { columns } => {
                format!("table:{}", columns.len())
            }
            DocumentKind::Figure { caption } => match caption {
                Some(spans) => format!("figure:{}", crate::encode::spans(spans)),
                None => "figure".into(),
            },
            DocumentKind::Separator => "---".into(),
            DocumentKind::Breath => "~~~".into(),
            DocumentKind::RawBlock { content, format } => {
                format!("raw:{}:{}", format, content)
            }
            DocumentKind::Embedded(_) => "embedded".into(),
        };
        s.into_bytes()
    }
}

impl Domain for Document {
    type Language = DocumentKind;

    fn id() -> &'static str {
        "document"
    }

    fn local_name(kind: &DocumentKind) -> Cow<'static, str> {
        match kind {
            DocumentKind::Section { level: 1, .. } => "section".into(),
            DocumentKind::Section { level, .. } => format!("section/{}", level).into(),
            DocumentKind::Paragraph { .. } => "p".into(),
            DocumentKind::CodeBlock { language, .. } if language.is_empty() => "code".into(),
            DocumentKind::CodeBlock { language, .. } => format!("code/{}", language).into(),
            DocumentKind::Quote { .. } => "quote".into(),
            DocumentKind::Callout { kind, .. } => match kind {
                CalloutKind::Note => "callout/note".into(),
                CalloutKind::Tip => "callout/tip".into(),
                CalloutKind::Important => "callout/important".into(),
                CalloutKind::Warning => "callout/warning".into(),
                CalloutKind::Caution => "callout/caution".into(),
            },
            DocumentKind::List {
                style: ListStyle::Unordered,
                ..
            } => "list".into(),
            DocumentKind::List {
                style: ListStyle::Ordered,
                ..
            } => "olist".into(),
            DocumentKind::ListItem { .. } => "li".into(),
            DocumentKind::DefinitionList => "dl".into(),
            DocumentKind::Table { .. } => "table".into(),
            DocumentKind::Figure { .. } => "figure".into(),
            DocumentKind::Separator => "hr".into(),
            DocumentKind::Breath => "breath".into(),
            DocumentKind::RawBlock { .. } => "raw".into(),
            DocumentKind::Embedded(_) => "document".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Node<D> — uniform tree structure, domain-specific kind
// ---------------------------------------------------------------------------

/// A node in a gestalt tree. Tree structure is uniform.
/// Domain-specific data lives in `kind`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node<D: Domain> {
    pub meta: Vec<Meta>,
    pub children: Vec<Node<D>>,
    pub kind: D::Language,
}

impl<D: Domain> Node<D> {
    /// The full lens label: `"{domain_id}/{local_name}"`.
    pub fn lens_label(&self) -> String {
        format!("{}/{}", D::id(), D::local_name(&self.kind))
    }

    /// Content-addressed identity. Same content = same Oid.
    pub fn oid(&self) -> Oid {
        let encoded = self.kind.encode();
        Oid::hash(
            format!(
                "{}:{}",
                self.lens_label(),
                String::from_utf8_lossy(&encoded)
            )
            .as_bytes(),
        )
    }
}

impl<D: Domain> crate::dom::DOM for Gestalt<D> {
    fn uri(&self) -> String {
        format!("gestalt://{}:{}", D::id(), self.oid())
    }

    fn attributes(&self) -> &[Meta] {
        &self.head
    }

    fn content(&self) -> Vec<&dyn crate::dom::DOM> {
        self.body
            .iter()
            .map(|n| n as &dyn crate::dom::DOM)
            .collect()
    }

    fn oid(&self) -> Oid {
        Gestalt::oid(self)
    }
}

impl<D: Domain> crate::dom::DOM for Node<D> {
    fn uri(&self) -> String {
        format!("gestalt://{}:{}", self.lens_label(), Node::oid(self))
    }

    fn attributes(&self) -> &[Meta] {
        &self.meta
    }

    fn content(&self) -> Vec<&dyn crate::dom::DOM> {
        self.children
            .iter()
            .map(|n| n as &dyn crate::dom::DOM)
            .collect()
    }

    fn oid(&self) -> Oid {
        Node::oid(self)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::DOM;
    use crate::semantic::Meta;

    #[test]
    fn empty_gestalt_has_no_body() {
        let g = Gestalt::empty();
        assert!(g.body.is_empty());
        assert!(g.head.is_empty());
    }

    #[test]
    fn gestalt_from_nodes() {
        let node: Node<Document> = Node {
            meta: vec![],
            children: vec![],
            kind: DocumentKind::Separator,
        };
        let g = Gestalt::from_nodes(vec![node]);
        assert_eq!(g.body.len(), 1);
    }

    #[test]
    fn document_domain_id() {
        assert_eq!(Document::id(), "@gestalt/document");
    }

    #[test]
    fn document_local_name_section() {
        let kind = DocumentKind::Section { level: 1, title: vec![] };
        assert_eq!(Document::local_name(&kind), "section");
    }

    #[test]
    fn document_local_name_section_level() {
        let kind = DocumentKind::Section { level: 2, title: vec![] };
        assert_eq!(Document::local_name(&kind), "section/2");
    }

    #[test]
    fn document_local_name_paragraph() {
        let kind = DocumentKind::Paragraph { content: vec![] };
        assert_eq!(Document::local_name(&kind), "p");
    }

    #[test]
    fn document_local_name_code_block_with_lang() {
        let kind = DocumentKind::CodeBlock { language: "rust".into(), content: "".into() };
        assert_eq!(Document::local_name(&kind), "code/rust");
    }

    #[test]
    fn document_local_name_code_block_no_lang() {
        let kind = DocumentKind::CodeBlock { language: "".into(), content: "".into() };
        assert_eq!(Document::local_name(&kind), "code");
    }

    #[test]
    fn node_oid_deterministic() {
        let a: Node<Document> = Node { meta: vec![], children: vec![], kind: DocumentKind::Separator };
        let b: Node<Document> = Node { meta: vec![], children: vec![], kind: DocumentKind::Separator };
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn node_oid_differs_by_kind() {
        let a: Node<Document> = Node { meta: vec![], children: vec![], kind: DocumentKind::Separator };
        let b: Node<Document> = Node { meta: vec![], children: vec![], kind: DocumentKind::Breath };
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn gestalt_implements_dom() {
        let g = Gestalt::empty();
        let dom: &dyn DOM = &g;
        assert!(dom.uri().contains("document"));
        assert!(dom.content().is_empty());
        assert!(dom.attributes().is_empty());
    }

    #[test]
    fn node_implements_dom() {
        let node: Node<Document> = Node { meta: vec![], children: vec![], kind: DocumentKind::Separator };
        let dom: &dyn DOM = &node;
        assert!(dom.uri().contains("hr"));
        assert!(dom.content().is_empty());
    }

    #[test]
    fn node_dom_attributes_returns_meta() {
        let node: Node<Document> = Node {
            meta: vec![Meta::Id("hook".into())],
            children: vec![],
            kind: DocumentKind::Separator,
        };
        let dom: &dyn DOM = &node;
        assert_eq!(dom.attributes().len(), 1);
    }

    #[test]
    fn gestalt_two_same_nodes_same_oid() {
        let node: Node<Document> = Node { meta: vec![], children: vec![], kind: DocumentKind::Separator };
        let a = Gestalt::from_nodes(vec![node.clone()]);
        let b = Gestalt::from_nodes(vec![node]);
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn gestalt_different_nodes_different_oid() {
        let a = Gestalt::from_nodes(vec![Node::<Document> {
            meta: vec![], children: vec![], kind: DocumentKind::Separator
        }]);
        let b = Gestalt::from_nodes(vec![Node::<Document> {
            meta: vec![], children: vec![], kind: DocumentKind::Breath
        }]);
        assert_ne!(a.oid(), b.oid());
    }
}
