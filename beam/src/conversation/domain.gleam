//// Domain server — GenServer for compiled grammar modules.
////
//// After @conversation compiles a grammar and loads the module,
//// this starts a gen_server registered as the domain atom.
//// When the compiled module's action functions call
//// gen_server:call(Domain, {Action, Args}), this server receives them.
////
//// Identity follows the cairn pattern: sha512(domain) → Ed25519 keypair.

import gleam/dynamic.{type Dynamic}

/// Start a domain GenServer registered as the domain atom (unsupervised).
/// Returns Ok(Nil) on success, Error with reason on failure.
@external(erlang, "domain_server", "start")
pub fn start(domain: String) -> Result(Nil, Dynamic)

/// Start the domain supervisor. Call once at boot.
@external(erlang, "conversation_sup", "start_link")
pub fn start_supervisor() -> Result(Dynamic, Dynamic)

/// Start a supervised domain server. Restarts on crash.
@external(erlang, "conversation_sup", "start_domain")
pub fn start_supervised(domain: String) -> Result(Dynamic, Dynamic)

/// Stop a running domain server.
@external(erlang, "domain_server", "stop")
pub fn stop(domain: String) -> Result(Nil, Dynamic)

/// Check if a domain server is running.
@external(erlang, "domain_server", "is_running")
pub fn is_running(domain: String) -> Bool

/// Kill a domain server process (for testing supervisor restart).
@external(erlang, "domain_server", "kill")
pub fn kill(domain: String) -> Nil

/// Call an action on a domain server directly.
/// Args is any Gleam value — passed through to Erlang as-is.
@external(erlang, "domain_server", "call_action")
pub fn call_action(
  domain: String,
  action: String,
  args: a,
) -> Result(Dynamic, String)

/// exec — the native primitive. Calls Module:Function(Args) on the BEAM.
/// Module and Function are strings, converted to atoms by the server.
/// This is what @erlang's domain server does: apply/3.
@external(erlang, "domain_server", "exec")
pub fn exec(
  domain: String,
  module: String,
  function: String,
  args: List(a),
) -> Result(Dynamic, String)
