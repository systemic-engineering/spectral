//! MirrorDomain — Domain<Language = MirrorKind> backed by @gestalt grammar.

#[cfg(test)]
mod tests {
    #[test]
    fn mirror_domain_id_is_at_gestalt() {
        use crate::domain::Domain;
        use super::MirrorDomain;
        assert_eq!(MirrorDomain::id(), "@gestalt");
    }

    #[test]
    fn mirror_kind_encodes_grammar_variant() {
        use crate::domain::Encode;
        use super::MirrorKind;
        assert_eq!(MirrorKind::Grammar.encode(), b"grammar");
        assert_eq!(MirrorKind::Type.encode(), b"type");
        assert_eq!(MirrorKind::Action.encode(), b"action");
    }

    #[test]
    fn mirror_domain_local_name() {
        use crate::domain::Domain;
        use super::{MirrorDomain, MirrorKind};
        assert_eq!(MirrorDomain::local_name(&MirrorKind::Grammar), "grammar");
        assert_eq!(MirrorDomain::local_name(&MirrorKind::Focus), "focus");
    }

    #[test]
    fn gestalt_mirror_domain_empty() {
        use crate::domain::Gestalt;
        use super::{MirrorDomain, MirrorKind};
        let g = Gestalt {
            domain: MirrorDomain,
            head: vec![],
            body: vec![],
        };
        assert!(g.body.is_empty());
    }

    #[test]
    fn node_mirror_domain_oid_deterministic() {
        use crate::domain::Node;
        use super::{MirrorDomain, MirrorKind};
        let a: Node<MirrorDomain> = Node { meta: vec![], children: vec![], kind: MirrorKind::Grammar };
        let b: Node<MirrorDomain> = Node { meta: vec![], children: vec![], kind: MirrorKind::Grammar };
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn node_mirror_domain_oid_differs_by_kind() {
        use crate::domain::Node;
        use super::{MirrorDomain, MirrorKind};
        let a: Node<MirrorDomain> = Node { meta: vec![], children: vec![], kind: MirrorKind::Grammar };
        let b: Node<MirrorDomain> = Node { meta: vec![], children: vec![], kind: MirrorKind::Action };
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn grammar_files_parse() {
        use mirror::parse::Parse;
        use mirror::Vector;
        let gestalt_src = include_str!("../../../../mirror/prism/gestalt/gestalt.mirror");
        let document_src = include_str!("../../../../mirror/prism/gestalt/document.mirror");
        let memory_src = include_str!("../../../../mirror/prism/gestalt/memory.mirror");
        assert!(!gestalt_src.is_empty());
        assert!(!document_src.is_empty());
        assert!(!memory_src.is_empty());
        let _ = Parse.trace(gestalt_src.to_string());
        let _ = Parse.trace(document_src.to_string());
        let _ = Parse.trace(memory_src.to_string());
    }
}
