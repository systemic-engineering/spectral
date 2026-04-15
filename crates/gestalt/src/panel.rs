//! Panel — renders state into a DOM subtree.
//!
//! A panel is the unit of composition for the UI. Each panel owns its state
//! type and produces a Node tree. Panels compose by nesting their DOM outputs.

use crate::dom::Node;

/// A panel renders state into a DOM subtree.
pub trait Panel {
    /// The state this panel renders.
    type State;

    /// Render the current state into a DOM tree.
    fn render(&self, state: &Self::State) -> Node;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct CounterPanel;

    impl Panel for CounterPanel {
        type State = u32;

        fn render(&self, state: &u32) -> Node {
            Node::element(
                "div",
                vec![("class".into(), "counter".into())],
                vec![Node::text(format!("Count: {}", state))],
            )
        }
    }

    #[test]
    fn panel_renders_state() {
        let panel = CounterPanel;
        let node = panel.render(&42);
        assert_eq!(node.tag, "div");
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].text, Some("Count: 42".into()));
    }

    #[test]
    fn panel_different_state_different_output() {
        use prism_core::oid::Addressable;

        let panel = CounterPanel;
        let a = panel.render(&1);
        let b = panel.render(&2);
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn panel_same_state_same_output() {
        use prism_core::oid::Addressable;

        let panel = CounterPanel;
        let a = panel.render(&5);
        let b = panel.render(&5);
        assert_eq!(a.oid(), b.oid());
    }

    struct ListPanel;

    impl Panel for ListPanel {
        type State = Vec<String>;

        fn render(&self, state: &Vec<String>) -> Node {
            let items: Vec<Node> = state
                .iter()
                .map(|item| {
                    Node::element("li", vec![], vec![Node::text(item.clone())])
                })
                .collect();
            Node::element("ul", vec![], items)
        }
    }

    #[test]
    fn list_panel_renders_items() {
        let panel = ListPanel;
        let items = vec!["one".into(), "two".into(), "three".into()];
        let node = panel.render(&items);
        assert_eq!(node.tag, "ul");
        assert_eq!(node.children.len(), 3);
    }

    #[test]
    fn list_panel_empty_state() {
        let panel = ListPanel;
        let node = panel.render(&vec![]);
        assert_eq!(node.tag, "ul");
        assert!(node.children.is_empty());
    }
}
