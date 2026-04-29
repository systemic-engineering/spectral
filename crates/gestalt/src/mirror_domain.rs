//! MirrorDomain — Domain<Language = MirrorKind> backed by @gestalt grammar.
//!
//! MirrorKind mirrors the MirrorAST variants (without child nodes).
//! MirrorDomain is Domain<Language = MirrorKind>.
//!
//! This makes Gestalt<MirrorDomain> the native representation of mirror grammar
//! trees inside gestalt. scan_grammars() produces Gestalt<MirrorDomain>.

use crate::domain::{Domain, Encode};
use std::borrow::Cow;

/// The vocabulary of the @gestalt meta-grammar — mirrors MirrorAST variant names.
/// No child nodes. Children live in Node<MirrorDomain>.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MirrorKind {
    Grammar,
    Type,
    Action,
    Property,
    Focus,
    Project,
    Split,
    Zoom,
    Refract,
    Import,
    Export,
    Abstract,
    Module,
}

impl Encode for MirrorKind {
    fn encode(&self) -> Vec<u8> {
        match self {
            MirrorKind::Grammar  => b"grammar".to_vec(),
            MirrorKind::Type     => b"type".to_vec(),
            MirrorKind::Action   => b"action".to_vec(),
            MirrorKind::Property => b"property".to_vec(),
            MirrorKind::Focus    => b"focus".to_vec(),
            MirrorKind::Project  => b"project".to_vec(),
            MirrorKind::Split    => b"split".to_vec(),
            MirrorKind::Zoom     => b"zoom".to_vec(),
            MirrorKind::Refract  => b"refract".to_vec(),
            MirrorKind::Import   => b"import".to_vec(),
            MirrorKind::Export   => b"export".to_vec(),
            MirrorKind::Abstract => b"abstract".to_vec(),
            MirrorKind::Module   => b"module".to_vec(),
        }
    }
}

/// The @gestalt meta-grammar domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirrorDomain;

impl Domain for MirrorDomain {
    type Language = MirrorKind;

    fn id() -> &'static str {
        "@gestalt"
    }

    fn local_name(kind: &MirrorKind) -> Cow<'static, str> {
        match kind {
            MirrorKind::Grammar  => "grammar".into(),
            MirrorKind::Type     => "type".into(),
            MirrorKind::Action   => "action".into(),
            MirrorKind::Property => "property".into(),
            MirrorKind::Focus    => "focus".into(),
            MirrorKind::Project  => "project".into(),
            MirrorKind::Split    => "split".into(),
            MirrorKind::Zoom     => "zoom".into(),
            MirrorKind::Refract  => "refract".into(),
            MirrorKind::Import   => "import".into(),
            MirrorKind::Export   => "export".into(),
            MirrorKind::Abstract => "abstract".into(),
            MirrorKind::Module   => "module".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Domain, Encode, Gestalt, Node};

    #[test]
    fn mirror_domain_id_is_at_gestalt() {
        assert_eq!(MirrorDomain::id(), "@gestalt");
    }

    #[test]
    fn mirror_kind_encodes_grammar_variant() {
        assert_eq!(MirrorKind::Grammar.encode(), b"grammar");
        assert_eq!(MirrorKind::Type.encode(), b"type");
        assert_eq!(MirrorKind::Action.encode(), b"action");
    }

    #[test]
    fn mirror_domain_local_name() {
        assert_eq!(MirrorDomain::local_name(&MirrorKind::Grammar), "grammar");
        assert_eq!(MirrorDomain::local_name(&MirrorKind::Focus), "focus");
    }

    #[test]
    fn gestalt_mirror_domain_empty() {
        let g = Gestalt {
            domain: MirrorDomain,
            head: vec![],
            body: vec![],
        };
        assert!(g.body.is_empty());
    }

    #[test]
    fn node_mirror_domain_oid_deterministic() {
        let a: Node<MirrorDomain> = Node { meta: vec![], children: vec![], kind: MirrorKind::Grammar };
        let b: Node<MirrorDomain> = Node { meta: vec![], children: vec![], kind: MirrorKind::Grammar };
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn node_mirror_domain_oid_differs_by_kind() {
        let a: Node<MirrorDomain> = Node { meta: vec![], children: vec![], kind: MirrorKind::Grammar };
        let b: Node<MirrorDomain> = Node { meta: vec![], children: vec![], kind: MirrorKind::Action };
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn grammar_files_parse() {
        use mirror::parse::Parse;
        use mirror::Vector;
        // include_str! paths are relative to this source file:
        // crates/gestalt/src/mirror_domain.rs → ../../../../mirror/prism/gestalt/
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
