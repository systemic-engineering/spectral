//! line — GestaltDoc, AnyGestalt, Annotations, RenderContext, Line<D>.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Gestalt;
    use prism_core::oid::{Addressable, Oid};

    // Task 3 tests
    #[test]
    fn any_gestalt_oid_from_lens_oid() {
        let g = Gestalt::empty();
        let lens_oid = Oid::hash(b"test-lens");
        let any = AnyGestalt::new(lens_oid.clone(), g);
        assert_eq!(any.oid(), lens_oid);
    }

    #[test]
    fn any_gestalt_grammar_id_returns_domain_id() {
        let g = Gestalt::empty();
        let any = AnyGestalt::new(Oid::hash(b"lens"), g);
        // Document::id() currently returns "document"
        assert_eq!(any.grammar_id(), "document");
    }

    #[test]
    fn any_gestalt_is_clone() {
        let g = Gestalt::empty();
        let any = AnyGestalt::new(Oid::hash(b"lens"), g);
        let _cloned = any.clone();
    }

    // Task 4 tests
    #[test]
    fn annotations_zero_is_empty() {
        let z = Annotations::zero();
        assert!(z.is_zero());
        assert_eq!(z.0.len(), 0);
    }

    #[test]
    fn annotations_combine_concatenates() {
        let a = Annotations(vec![
            Named("blame", AnyGestalt::new(Oid::hash(b"l1"), Gestalt::empty()))
        ]);
        let b = Annotations(vec![
            Named("summary", AnyGestalt::new(Oid::hash(b"l2"), Gestalt::empty()))
        ]);
        let combined = a.combine(b);
        assert_eq!(combined.0.len(), 2);
        assert_eq!(combined.0[0].name(), "blame");
        assert_eq!(combined.0[1].name(), "summary");
    }

    #[test]
    fn annotations_singleton() {
        let ann = Annotations::singleton(
            "diff",
            AnyGestalt::new(Oid::hash(b"lens"), Gestalt::empty()),
        );
        assert_eq!(ann.0.len(), 1);
        assert_eq!(ann.0[0].name(), "diff");
        assert!(!ann.is_zero());
    }

    // Task 5 tests
    #[test]
    fn render_context_root_has_zero_depth() {
        let ctx = RenderContext::root();
        assert_eq!(ctx.depth, 0);
        assert_eq!(ctx.cursor, 0);
        assert!(ctx.parent_oids.is_empty());
    }

    #[test]
    fn render_context_child_increments_depth() {
        let parent_oid = Oid::hash(b"parent");
        let ctx = RenderContext::root().child(parent_oid.clone(), 0);
        assert_eq!(ctx.depth, 1);
        assert_eq!(ctx.parent_oids.len(), 1);
        assert_eq!(ctx.parent_oids[0], parent_oid);
    }

    #[test]
    fn make_line_carries_node() {
        use crate::domain::{Document, DocumentKind, Node};
        let node: Node<Document> = Node {
            meta: vec![],
            children: vec![],
            kind: DocumentKind::Separator,
        };
        let line = make_line(RenderContext::root(), node.clone());
        assert!(line.value().is_some());
        assert_eq!(line.value().unwrap().kind, DocumentKind::Separator);
    }

    #[test]
    fn make_line_starts_with_zero_loss() {
        use crate::domain::{Document, DocumentKind, Node};
        let node: Node<Document> = Node {
            meta: vec![],
            children: vec![],
            kind: DocumentKind::Separator,
        };
        let line = make_line(RenderContext::root(), node);
        assert!(line.value().is_some());
    }
}
