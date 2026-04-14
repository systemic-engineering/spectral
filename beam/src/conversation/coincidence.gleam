//// Coincidence — NIF bridge to measurement functions.
////
//// Exposes property-checking NIFs from the conversation crate.
//// Each function takes a grammar source string, compiles it,
//// and checks a specific property.
////
//// Also provides start/stop/is_running for the @coincidence domain server,
//// which routes action calls to these NIFs.

import gleam/dynamic.{type Dynamic}

/// Check a built-in property by name against a grammar source.
/// Returns the pass/fail reason string.
@external(erlang, "conversation_nif", "check_property")
pub fn check_property(
  source: String,
  property: String,
) -> Result(String, String)

/// Check shannon equivalence (content address uniqueness).
/// Every derivation of the grammar must produce a unique content OID.
@external(erlang, "conversation_nif", "check_shannon_equivalence")
pub fn check_shannon_equivalence(source: String) -> Result(String, String)

/// Check type graph connectivity (spectral).
/// The type reference graph must be a single connected component.
@external(erlang, "conversation_nif", "check_connected")
pub fn check_connected(source: String) -> Result(String, String)

/// Check type graph bipartiteness (spectral).
/// The type reference graph must have no odd-length cycles.
@external(erlang, "conversation_nif", "check_bipartite")
pub fn check_bipartite(source: String) -> Result(String, String)

/// Check exhaustiveness — every declared type has at least one variant.
@external(erlang, "conversation_nif", "check_exhaustive")
pub fn check_exhaustive(source: String) -> Result(String, String)

// ── Domain server ──────────────────────────────────────────────────────────

/// Start the @coincidence domain server (unsupervised).
@external(erlang, "coincidence_server", "start")
pub fn start_server() -> Result(Nil, Dynamic)

/// Stop the @coincidence domain server.
@external(erlang, "coincidence_server", "stop")
pub fn stop_server() -> Result(Nil, Dynamic)

/// Check if the @coincidence domain server is running.
@external(erlang, "coincidence_server", "is_running")
pub fn is_running() -> Bool
