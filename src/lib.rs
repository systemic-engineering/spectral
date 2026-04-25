//! spectral — git for graphs.
//!
//! Observation layer (apache2/) is always available.
//! Runtime layer (sel/) requires --features sel.

pub mod apache2;

#[cfg(feature = "sel")]
pub mod sel;
