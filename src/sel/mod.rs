//! Runtime layer. SEL (Spectral Eigen License). Requires --features sel.
//!
//! This module contains the mutable runtime: join, LLM client, tick/tock, shatter.

pub mod llm;
pub mod join;
pub mod fate_actor;
pub mod matrix;
pub mod surface;
pub mod shatter_model;
pub mod reflection;
pub mod weight_file;
pub mod training;
#[cfg(test)]
mod nl_integration_test;
