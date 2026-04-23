//! MCP subsystem — actors for the Spectral MCP server.
//!
//! Each actor owns its resource. No shared mutexes. All access goes through messages.

pub mod memory;
