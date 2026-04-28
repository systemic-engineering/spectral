//! Observation layer. Apache-2.0. Always compiled.
//!
//! This module provides read-only access to spectral graphs.
//! Nothing in this module performs side effects or mutations.
//! The license boundary: apache2/ never imports from sel/.

pub mod runtime;
pub mod signal;
pub mod observe;
pub mod identity;
pub mod init;
pub mod inference;
pub mod loss;
pub mod graph_cache;
pub mod views;
