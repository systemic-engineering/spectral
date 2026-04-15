//! Gestalt — design system types for spectral.
//!
//! Three modules:
//! - `token` — design tokens as named lambdas over TokenValue
//! - `dom` — virtual DOM with content-addressed diffing
//! - `panel` — trait for rendering state into DOM subtrees

pub mod dom;
pub mod panel;
pub mod token;
