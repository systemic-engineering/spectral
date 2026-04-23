//! Gestalt — unified document model and design system for spectral.
//!
//! ## Layers
//!
//! ### Document model (from standalone gestalt)
//! - `semantic` — shared meaning vocabulary: roles, marks, meta, callout kinds
//! - `document` — inline content (Span) and structural enumerations
//! - `domain` — Domain trait, Gestalt<D>, Node<D> — the vocabulary-agnostic content tree
//! - `encode` — markdown ↔ Gestalt<Document> parsing and rendering
//!
//! ### Virtual DOM + design system (from spectral)
//! - `dom` — DOM trait + virtual DOM Node + diffing
//! - `panel` — Panel trait for rendering state into DOM subtrees
//! - `token` — design tokens as named lambdas over TokenValue
//!
//! ### New block types
//! - `form` — interactive form fields (TextField, DateField, CurrencyField, CheckboxField, SignatureField)
//! - `spectral` — eigenvalue visualization blocks (EigenvalueProfile, LossHeatmap, MixingFader, TournamentBracket, CouplingGraph)

pub mod document;
pub mod dom;
pub mod domain;
pub mod encode;
pub mod form;
pub mod panel;
pub mod semantic;
pub mod spectral;
pub mod token;
