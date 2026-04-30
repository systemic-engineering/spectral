//! Fragment<D> — a Node<D> that implements MerkleTree.
//!
//! The difference: children's OIDs are incorporated into the parent OID.
//! Same content + same children = same OID. Always.
//! Node<D> stays for backward compatibility. Fragment<D> is the new
//! content-addressed tree type that makes diff, store, and cache work.

use crate::domain::{Domain, Document, DocumentKind, Encode, Node};
use crate::semantic::Meta;
use prism_core::merkle::MerkleTree;
use prism_core::oid::{Addressable, Oid};

/// The payload of a Fragment node: kind + metadata.
/// Stored as a named struct so MerkleTree::data() can return a reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentData<D: Domain> {
    pub kind: D::Language,
    pub meta: Vec<Meta>,
}

/// A content-addressed tree node. Like Node<D>, but the OID incorporates
/// children's OIDs — making the tree a proper MerkleTree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment<D: Domain> {
    pub data: FragmentData<D>,
    pub children: Vec<Fragment<D>>,
}

impl<D: Domain> Fragment<D> {
    /// The node's kind (convenience accessor).
    pub fn kind(&self) -> &D::Language {
        &self.data.kind
    }

    /// The node's metadata (convenience accessor).
    pub fn meta(&self) -> &[Meta] {
        &self.data.meta
    }
}

impl<D: Domain> Addressable for Fragment<D> {
    fn oid(&self) -> Oid {
        let child_oids: String = self
            .children
            .iter()
            .map(|c| c.oid().to_string())
            .collect::<Vec<_>>()
            .join(":");
        let encoded = self.data.kind.encode();
        Oid::hash(
            format!(
                "fragment:{}:{}:{}",
                D::id(),
                String::from_utf8_lossy(&encoded),
                child_oids,
            )
            .as_bytes(),
        )
    }
}

impl<D: Domain> MerkleTree for Fragment<D> {
    type Data = FragmentData<D>;

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn children(&self) -> &[Self] {
        &self.children
    }
}

impl<D: Domain> From<Node<D>> for Fragment<D> {
    fn from(node: Node<D>) -> Self {
        Fragment {
            data: FragmentData {
                kind: node.kind,
                meta: node.meta,
            },
            children: node.children.into_iter().map(Fragment::from).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_core::merkle::diff;

    fn leaf(kind: DocumentKind) -> Fragment<Document> {
        Fragment {
            data: FragmentData { kind, meta: vec![] },
            children: vec![],
        }
    }

    fn branch(kind: DocumentKind, children: Vec<Fragment<Document>>) -> Fragment<Document> {
        Fragment {
            data: FragmentData { kind, meta: vec![] },
            children,
        }
    }

    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    #[test]
    fn fragment_leaf_has_no_children() {
        let f = leaf(DocumentKind::Separator);
        assert!(f.is_leaf());
        assert_eq!(f.degree(), 0);
    }

    #[test]
    fn fragment_with_children_is_not_leaf() {
        let parent = branch(DocumentKind::Separator, vec![leaf(DocumentKind::Breath)]);
        assert!(!parent.is_leaf());
        assert_eq!(parent.degree(), 1);
    }

    // -----------------------------------------------------------------------
    // Content addressing — the core invariant
    // -----------------------------------------------------------------------

    #[test]
    fn same_content_same_oid() {
        let a = leaf(DocumentKind::Separator);
        let b = leaf(DocumentKind::Separator);
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn different_kind_different_oid() {
        let a = leaf(DocumentKind::Separator);
        let b = leaf(DocumentKind::Breath);
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn different_children_different_oid() {
        let a = branch(DocumentKind::Separator, vec![leaf(DocumentKind::Separator)]);
        let b = branch(DocumentKind::Separator, vec![leaf(DocumentKind::Breath)]);
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn children_affect_parent_oid() {
        // This is THE test that distinguishes Fragment from Node.
        // Node<D>.oid() does NOT incorporate children. Fragment<D>.oid() MUST.
        let f_leaf = leaf(DocumentKind::Separator);
        let parent = branch(DocumentKind::Separator, vec![leaf(DocumentKind::Breath)]);
        // Same kind, but parent has a child — OIDs MUST differ.
        assert_ne!(f_leaf.oid(), parent.oid());
    }

    // -----------------------------------------------------------------------
    // MerkleTree trait — data() and children()
    // -----------------------------------------------------------------------

    #[test]
    fn data_returns_kind_and_meta() {
        let f: Fragment<Document> = Fragment {
            data: FragmentData {
                kind: DocumentKind::Separator,
                meta: vec![Meta::Id("test".into())],
            },
            children: vec![],
        };
        let d = f.data();
        assert_eq!(d.kind, DocumentKind::Separator);
        assert_eq!(d.meta.len(), 1);
    }

    #[test]
    fn children_accessor_returns_children() {
        let child = leaf(DocumentKind::Breath);
        let parent = branch(DocumentKind::Separator, vec![child.clone()]);
        assert_eq!(parent.children(), &[child]);
    }

    // -----------------------------------------------------------------------
    // MerkleTree diff — identical subtrees are skipped
    // -----------------------------------------------------------------------

    #[test]
    fn diff_identical_fragments_is_empty() {
        let a = leaf(DocumentKind::Separator);
        let b = a.clone();
        let deltas = diff(&a, &b);
        assert!(deltas.is_empty());
    }

    #[test]
    fn diff_detects_added_child() {
        let a = leaf(DocumentKind::Separator);
        let b = branch(DocumentKind::Separator, vec![leaf(DocumentKind::Breath)]);
        let deltas = diff(&a, &b);
        assert!(!deltas.is_empty(), "should detect structural difference");
    }

    #[test]
    fn diff_deep_shared_subtree_skipped() {
        let shared = branch(DocumentKind::Breath, vec![leaf(DocumentKind::Separator)]);
        let a = branch(DocumentKind::Separator, vec![shared.clone()]);
        let b = branch(DocumentKind::Separator, vec![shared]);
        let deltas = diff(&a, &b);
        assert!(deltas.is_empty(), "identical subtrees must be skipped");
    }

    // -----------------------------------------------------------------------
    // Conversion from Node<D> — lossless round-trip for structure
    // -----------------------------------------------------------------------

    #[test]
    fn from_node_preserves_structure() {
        let n: Node<Document> = Node {
            kind: DocumentKind::Separator,
            meta: vec![Meta::Id("hook".into())],
            children: vec![Node {
                kind: DocumentKind::Breath,
                meta: vec![],
                children: vec![],
            }],
        };
        let frag = Fragment::from(n);
        assert_eq!(frag.data.kind, DocumentKind::Separator);
        assert_eq!(frag.data.meta.len(), 1);
        assert_eq!(frag.children.len(), 1);
        assert_eq!(frag.children[0].data.kind, DocumentKind::Breath);
    }

    #[test]
    fn from_node_oid_differs_from_node_oid_when_children_present() {
        // A Node with children has an OID that ignores children.
        // A Fragment from the same Node incorporates children.
        // They MUST differ (proving Fragment actually incorporates children).
        let n: Node<Document> = Node {
            kind: DocumentKind::Separator,
            meta: vec![],
            children: vec![Node {
                kind: DocumentKind::Breath,
                meta: vec![],
                children: vec![],
            }],
        };
        let frag = Fragment::from(n.clone());
        assert_ne!(
            n.oid(),
            frag.oid(),
            "Fragment OID must differ from Node OID when children present"
        );
    }

    #[test]
    fn from_node_preserves_kind_on_leaf() {
        // Fragment uses a different hash format than Node (includes children
        // component even when empty). This test documents that the kind is
        // preserved correctly during conversion even as OIDs diverge.
        let n: Node<Document> = Node {
            kind: DocumentKind::Separator,
            meta: vec![],
            children: vec![],
        };
        let frag = Fragment::from(n.clone());
        assert_eq!(frag.data.kind, n.kind);
    }
}
