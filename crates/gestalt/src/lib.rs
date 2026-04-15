//! Gestalt — design system types for spectral.
//!
//! Three modules:
//! - `token` — design tokens as named lambdas over TokenValue
//! - `dom` — virtual DOM with content-addressed diffing
//! - `panel` — trait for rendering state into DOM subtrees

// TODO: modules not yet created
// pub mod dom;
// pub mod panel;
// pub mod token;

#[cfg(test)]
mod tests {
    #[test]
    fn token_is_named_lambda() {
        // RED: Token type doesn't exist yet
        let _t: super::token::Token = todo!();
    }

    #[test]
    fn dom_node_is_content_addressed() {
        // RED: DOM module doesn't exist yet
        let _n: super::dom::Node = todo!();
    }

    #[test]
    fn panel_renders_to_dom() {
        // RED: Panel trait doesn't exist yet
        todo!("Panel trait not implemented");
    }
}
