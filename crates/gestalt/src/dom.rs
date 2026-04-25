//! DOM — vocabulary-agnostic content tree + virtual DOM diffing.
//!
//! Two layers in one module:
//!
//! 1. `DOM` trait — the vocabulary-agnostic interface. Any content tree
//!    implements this: gestalt documents, forms, spectral visualizations.
//!    Four methods: uri(), attributes(), content(), oid().
//!
//! 2. `Node` — the virtual DOM. A renderable, diffable tree element.
//!    Implements `DOM`. Content-addressed via Oid. Diffing produces `Vec<Patch>`.

use crate::semantic::Meta;
use prism_core::oid::{Addressable, Oid};

// ---------------------------------------------------------------------------
// DOM trait — the vocabulary-agnostic content tree interface
// ---------------------------------------------------------------------------

/// The tree trait. Every node is queryable and content-addressable.
///
/// Vocabulary-agnostic: gestalt documents, forms, spectral visualizations —
/// all implement DOM. The trait operates on four axes:
/// - Identity: what this node IS in its vocabulary (uri)
/// - Metadata: semantic annotations (attributes)
/// - Structure: child nodes (content)
/// - Content address: same content = same Oid (oid)
pub trait DOM {
    /// Structural address. Identifies what this node IS in its vocabulary.
    fn uri(&self) -> String;

    /// Metadata on this node.
    fn attributes(&self) -> &[Meta];

    /// Child DOM nodes. Containers return children, leaves return empty.
    fn content(&self) -> Vec<&dyn DOM>;

    /// Content-addressed identity. Same content = same Oid.
    fn oid(&self) -> Oid;
}

/// A DOM node. Content-addressed.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub tag: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Node>,
    pub text: Option<String>,
}

impl Node {
    /// Create a new element node.
    pub fn element(tag: impl Into<String>, attributes: Vec<(String, String)>, children: Vec<Node>) -> Self {
        Node {
            tag: tag.into(),
            attributes,
            children,
            text: None,
        }
    }

    /// Create a text node.
    pub fn text(content: impl Into<String>) -> Self {
        Node {
            tag: String::new(),
            attributes: Vec::new(),
            children: Vec::new(),
            text: Some(content.into()),
        }
    }

    /// Content-addressed identity. Same structure = same Oid.
    /// This is a direct method to avoid ambiguity with Addressable::oid() and DOM::oid().
    pub fn node_oid(&self) -> Oid {
        compute_node_oid(self)
    }
}

fn compute_node_oid(node: &Node) -> Oid {
    let mut content = format!("node:{}", node.tag);
    for (k, v) in &node.attributes {
        content.push_str(&format!(":{}={}", k, v));
    }
    for child in &node.children {
        content.push_str(&format!(":{}", compute_node_oid(child)));
    }
    if let Some(ref t) = node.text {
        content.push_str(&format!(":text={}", t));
    }
    Oid::hash(content.as_bytes())
}

impl Addressable for Node {
    fn oid(&self) -> Oid {
        compute_node_oid(self)
    }
}

/// A patch describing a change between two trees.
#[derive(Clone, Debug, PartialEq)]
pub enum Patch {
    /// Replace the node at this index.
    Replace(usize, Node),
    /// Update attributes at this index.
    UpdateAttrs(usize, Vec<(String, String)>),
    /// Insert a child at parent_index, child_position.
    InsertChild(usize, usize, Node),
    /// Remove a child at parent_index, child_position.
    RemoveChild(usize, usize),
    /// Update text content at this index.
    UpdateText(usize, String),
}

/// Diff two trees. Returns the patches needed to transform `old` into `new`.
///
/// Uses Oid comparison for fast path: same Oid = identical subtree = skip.
pub fn diff(old: &Node, new: &Node) -> Vec<Patch> {
    if compute_node_oid(old) == compute_node_oid(new) {
        return Vec::new();
    }
    diff_at(old, new, 0)
}

fn diff_at(old: &Node, new: &Node, index: usize) -> Vec<Patch> {
    if compute_node_oid(old) == compute_node_oid(new) {
        return Vec::new();
    }

    // Different tags = full replace
    if old.tag != new.tag {
        return vec![Patch::Replace(index, new.clone())];
    }

    let mut patches = Vec::new();

    // Check attributes
    if old.attributes != new.attributes {
        patches.push(Patch::UpdateAttrs(index, new.attributes.clone()));
    }

    // Check text
    if old.text != new.text {
        if let Some(ref t) = new.text {
            patches.push(Patch::UpdateText(index, t.clone()));
        }
    }

    // Diff children
    let old_len = old.children.len();
    let new_len = new.children.len();
    let min_len = old_len.min(new_len);

    // Compare common children
    for i in 0..min_len {
        let child_patches = diff_at(&old.children[i], &new.children[i], i);
        patches.extend(child_patches);
    }

    // Handle added children
    for i in min_len..new_len {
        patches.push(Patch::InsertChild(index, i, new.children[i].clone()));
    }

    // Handle removed children (in reverse to preserve indices)
    for i in (min_len..old_len).rev() {
        patches.push(Patch::RemoveChild(index, i));
    }

    patches
}

// ---------------------------------------------------------------------------
// DOM impl for Node
// ---------------------------------------------------------------------------

impl DOM for Node {
    fn uri(&self) -> String {
        if self.tag.is_empty() {
            "#text".to_string()
        } else {
            format!("node:{}", self.tag)
        }
    }

    fn attributes(&self) -> &[Meta] {
        // Render nodes carry no semantic Meta — attributes are in `self.attributes`.
        // DOM trait attributes() returns semantic metadata. Render nodes have none.
        &[]
    }

    fn content(&self) -> Vec<&dyn DOM> {
        self.children.iter().map(|c| c as &dyn DOM).collect()
    }

    fn oid(&self) -> Oid {
        compute_node_oid(self)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_creation() {
        let node = Node::element("div", vec![], vec![]);
        assert_eq!(node.tag, "div");
        assert!(node.children.is_empty());
        assert!(node.text.is_none());
    }

    #[test]
    fn text_node_creation() {
        let node = Node::text("hello");
        assert_eq!(node.text, Some("hello".into()));
        assert!(node.tag.is_empty());
    }

    #[test]
    fn node_is_content_addressed() {
        let a = Node::element("div", vec![], vec![]);
        let b = Node::element("div", vec![], vec![]);
        assert_eq!(a.node_oid(), b.node_oid());
    }

    #[test]
    fn different_tag_different_oid() {
        let a = Node::element("div", vec![], vec![]);
        let b = Node::element("span", vec![], vec![]);
        assert_ne!(a.node_oid(), b.node_oid());
    }

    #[test]
    fn different_attrs_different_oid() {
        let a = Node::element("div", vec![("class".into(), "a".into())], vec![]);
        let b = Node::element("div", vec![("class".into(), "b".into())], vec![]);
        assert_ne!(a.node_oid(), b.node_oid());
    }

    #[test]
    fn different_children_different_oid() {
        let a = Node::element("div", vec![], vec![Node::text("a")]);
        let b = Node::element("div", vec![], vec![Node::text("b")]);
        assert_ne!(a.node_oid(), b.node_oid());
    }

    #[test]
    fn diff_identical_trees_empty() {
        let tree = Node::element("div", vec![], vec![Node::text("hello")]);
        assert!(diff(&tree, &tree).is_empty());
    }

    #[test]
    fn diff_different_tag_replaces() {
        let old = Node::element("div", vec![], vec![]);
        let new = Node::element("span", vec![], vec![]);
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::Replace(0, n) if n.tag == "span"));
    }

    #[test]
    fn diff_updated_attrs() {
        let old = Node::element("div", vec![("class".into(), "old".into())], vec![]);
        let new = Node::element("div", vec![("class".into(), "new".into())], vec![]);
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::UpdateAttrs(0, _)));
    }

    #[test]
    fn diff_updated_text() {
        let old = Node::text("hello");
        let new = Node::text("world");
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::UpdateText(0, ref t) if t == "world"));
    }

    #[test]
    fn diff_insert_child() {
        let old = Node::element("div", vec![], vec![]);
        let child = Node::text("new child");
        let new = Node::element("div", vec![], vec![child.clone()]);
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::InsertChild(0, 0, _)));
    }

    #[test]
    fn diff_remove_child() {
        let child = Node::text("old child");
        let old = Node::element("div", vec![], vec![child]);
        let new = Node::element("div", vec![], vec![]);
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::RemoveChild(0, 0)));
    }

    #[test]
    fn diff_replace_child() {
        let old = Node::element("div", vec![], vec![Node::text("a")]);
        let new = Node::element("div", vec![], vec![Node::text("b")]);
        let patches = diff(&old, &new);
        assert!(!patches.is_empty());
        // The child text changed, so we get an UpdateText patch
        assert!(patches.iter().any(|p| matches!(p, Patch::UpdateText(_, _))));
    }

    #[test]
    fn node_implements_dom_uri() {
        let node = Node::element("div", vec![], vec![]);
        let dom: &dyn DOM = &node;
        assert_eq!(dom.uri(), "node:div");
    }

    #[test]
    fn text_node_implements_dom_uri() {
        let node = Node::text("hello");
        let dom: &dyn DOM = &node;
        assert_eq!(dom.uri(), "#text");
    }

    #[test]
    fn node_dom_content_returns_children() {
        let parent = Node::element("div", vec![], vec![Node::text("a"), Node::text("b")]);
        let dom: &dyn DOM = &parent;
        assert_eq!(dom.content().len(), 2);
    }

    #[test]
    fn node_dom_attributes_empty_for_render_node() {
        let node = Node::element("div", vec![("class".into(), "foo".into())], vec![]);
        let dom: &dyn DOM = &node;
        // Render nodes have no semantic Meta
        assert!(dom.attributes().is_empty());
    }

    #[test]
    fn node_dom_oid_consistent_with_addressable() {
        let node = Node::element("div", vec![], vec![]);
        let dom: &dyn DOM = &node;
        assert_eq!(dom.oid(), node.node_oid());
    }

    #[test]
    fn diff_multiple_children() {
        let old = Node::element("ul", vec![], vec![
            Node::element("li", vec![], vec![Node::text("a")]),
            Node::element("li", vec![], vec![Node::text("b")]),
        ]);
        let new = Node::element("ul", vec![], vec![
            Node::element("li", vec![], vec![Node::text("a")]),
            Node::element("li", vec![], vec![Node::text("c")]),
            Node::element("li", vec![], vec![Node::text("d")]),
        ]);
        let patches = diff(&old, &new);
        // Second child changed text, third child added
        assert!(patches.len() >= 2);
    }
}
