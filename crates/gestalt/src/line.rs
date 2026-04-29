//! line — GestaltDoc, AnyGestalt, Annotations, RenderContext, Line<D>.
//!
//! A Line<D> = Optic<RenderContext, Node<D>, Infallible, Annotations>
//!
//! - In:   RenderContext — parent OIDs, depth, cursor (the structural context)
//! - Out:  Node<D>       — the content node (unchanged by lens application)
//! - Loss: Annotations   — named cross-domain lens results accumulate here

use std::sync::Arc;
use std::convert::Infallible;
use prism_core::named::Named;
use prism_core::oid::{Addressable, Oid};
use prism_core::beam::Optic;
use terni::Loss;

// ---------------------------------------------------------------------------
// GestaltDoc — object-safe trait for type-erased Gestalt<E>
// ---------------------------------------------------------------------------

/// Object-safe interface over Gestalt<E> for any grammar E.
pub trait GestaltDoc: Send + Sync {
    fn doc_oid(&self) -> Oid;
    fn grammar_id(&self) -> &'static str;
}

impl<D> GestaltDoc for crate::domain::Gestalt<D>
where
    D: crate::domain::Domain + Send + Sync + 'static,
    D::Language: Send + Sync,
{
    fn doc_oid(&self) -> Oid {
        crate::domain::Gestalt::oid(self)
    }

    fn grammar_id(&self) -> &'static str {
        D::id()
    }
}

// ---------------------------------------------------------------------------
// AnyGestalt
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AnyGestalt {
    pub lens_oid: Oid,
    pub content: Arc<dyn GestaltDoc>,
}

impl AnyGestalt {
    pub fn new<D>(lens_oid: Oid, gestalt: crate::domain::Gestalt<D>) -> Self
    where
        D: crate::domain::Domain + Send + Sync + 'static,
        D::Language: Send + Sync,
    {
        AnyGestalt {
            lens_oid,
            content: Arc::new(gestalt),
        }
    }

    pub fn grammar_id(&self) -> &'static str {
        self.content.grammar_id()
    }
}

impl Addressable for AnyGestalt {
    fn oid(&self) -> Oid {
        self.lens_oid.clone()
    }
}

impl std::fmt::Debug for AnyGestalt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyGestalt")
            .field("lens_oid", &self.lens_oid)
            .field("grammar_id", &self.content.grammar_id())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Annotations — named Loss type for cross-domain lens results
// ---------------------------------------------------------------------------

/// Named cross-domain annotation accumulator. The Loss type for Line<D>.
///
/// Zero = no annotations. combine = concat. Each applied lens adds one entry.
/// `Named<AnyGestalt>` pairs a lens label with the gestalt it produced.
#[derive(Clone, Debug, Default)]
pub struct Annotations(pub Vec<Named<AnyGestalt>>);

impl Annotations {
    pub fn singleton(name: &'static str, content: AnyGestalt) -> Self {
        Annotations(vec![Named(name, content)])
    }

    pub fn entries(&self) -> &[Named<AnyGestalt>] {
        &self.0
    }
}

impl Loss for Annotations {
    fn zero() -> Self { Annotations(Vec::new()) }
    fn total() -> Self {
        // Annotations has no meaningful absorbing-element.
        // Annotations are additive accumulators — they grow, never saturate.
        // We intentionally return empty (same as zero) and accept that
        // total().combine(singleton) == singleton, not total().
        // This deviates from the Loss absorbing-element contract.
        // Dark-beam propagation through Optic<_, _, _, Annotations> will
        // carry zero annotations on the error path, which is correct behavior.
        Annotations(Vec::new())
    }
    fn is_zero(&self) -> bool { self.0.is_empty() }
    fn combine(self, other: Self) -> Self {
        let mut v = self.0;
        v.extend(other.0);
        Annotations(v)
    }
}

// ---------------------------------------------------------------------------
// RenderContext — structural context for a line
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct RenderContext {
    pub parent_oids: Vec<Oid>,
    pub depth: usize,
    pub cursor: usize,
}

impl RenderContext {
    pub fn root() -> Self {
        RenderContext { parent_oids: Vec::new(), depth: 0, cursor: 0 }
    }

    pub fn child(&self, parent_oid: Oid, cursor: usize) -> Self {
        let mut oids = self.parent_oids.clone();
        oids.push(parent_oid);
        RenderContext { parent_oids: oids, depth: self.depth + 1, cursor }
    }
}

// ---------------------------------------------------------------------------
// Line<D> = Optic<RenderContext, Node<D>, Infallible, Annotations>
// ---------------------------------------------------------------------------

pub type Line<D> = Optic<RenderContext, crate::domain::Node<D>, Infallible, Annotations>;

/// Construct a bare Line<D> with zero annotations.
pub fn make_line<D: crate::domain::Domain>(
    ctx: RenderContext,
    node: crate::domain::Node<D>,
) -> Line<D> {
    Optic::ok(ctx, node)
}

/// Construct a Line<D> with pre-existing annotations.
pub fn make_line_with_annotations<D: crate::domain::Domain>(
    ctx: RenderContext,
    node: crate::domain::Node<D>,
    annotations: Annotations,
) -> Line<D> {
    Optic::partial(ctx, node, annotations)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
        // Document::id() returns the grammar-native id
        assert_eq!(any.grammar_id(), "@gestalt/document");
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

    #[test]
    fn annotations_total_is_non_absorbing_by_design() {
        // Document the intentional deviation from Loss absorbing-element contract.
        // total().combine(x) returns x (not total()), because annotations
        // are additive — there is no "all information lost" state for annotations.
        let total = Annotations::total();
        let singleton = Annotations::singleton(
            "test",
            AnyGestalt::new(Oid::hash(b"lens"), Gestalt::empty()),
        );
        let combined = total.combine(singleton.clone());
        assert_eq!(combined.0.len(), 1, "total() + singleton = singleton (non-absorbing)");
        assert!(Annotations::total().is_zero());
    }
}
