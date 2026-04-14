//// Garden — factory supervisor for domain servers.
////
//// The package manager for the language. When you install a grammar,
//// the garden starts its domain server as a dynamic child. Domain
//// servers that crash are restarted by the factory supervisor (transient).
////
//// The garden is embedded in a static supervisor alongside @compiler:
////   conversation_supervisor (RestForOne)
////   ├── @compiler
////   └── garden (factory_supervisor)
////
//// @compiler crash → garden + all domain servers restart (clean slate).
//// Domain server crash → factory supervisor restarts that one domain.

import gleam/dynamic.{type Dynamic}
import gleam/erlang/process.{type Pid}
import gleam/otp/actor
import gleam/otp/factory_supervisor
import gleam/otp/supervision

/// Start the garden factory supervisor directly.
/// Use this for standalone testing; prefer `supervised` for production.
pub fn start(
  name: process.Name(factory_supervisor.Message(String, String)),
) -> actor.StartResult(factory_supervisor.Supervisor(String, String)) {
  factory_supervisor.start(builder(name))
}

/// Create a child specification for embedding the garden in a static supervisor.
/// The name is used to register the factory supervisor so other processes
/// can start/stop domain servers through it.
pub fn supervised(
  name: process.Name(factory_supervisor.Message(String, String)),
) -> supervision.ChildSpecification(
  factory_supervisor.Supervisor(String, String),
) {
  factory_supervisor.supervised(builder(name))
}

/// Start a domain server under the garden factory supervisor.
pub fn start_domain(
  name: process.Name(factory_supervisor.Message(String, String)),
  domain: String,
) -> actor.StartResult(String) {
  let sup = factory_supervisor.get_by_name(name)
  factory_supervisor.start_child(sup, domain)
}

/// Stop a running domain server.
/// Delegates directly to domain_server — the factory supervisor
/// tracks the process lifecycle via the linked pid.
pub fn stop_domain(domain: String) -> Result(Nil, Dynamic) {
  do_stop(domain)
}

/// Check if a domain server is running.
pub fn is_running(domain: String) -> Bool {
  do_is_running(domain)
}

fn builder(
  name: process.Name(factory_supervisor.Message(String, String)),
) -> factory_supervisor.Builder(String, String) {
  factory_supervisor.worker_child(start_domain_server)
  |> factory_supervisor.named(name)
  |> factory_supervisor.restart_tolerance(intensity: 5, period: 60)
}

/// The template function for the factory supervisor.
/// Takes a domain name, starts a domain_server linked to the caller.
fn start_domain_server(domain: String) -> actor.StartResult(String) {
  case do_start_link(domain) {
    Ok(pid) -> Ok(actor.Started(pid: pid, data: domain))
    Error(_) ->
      Error(actor.InitFailed("failed to start domain server: " <> domain))
  }
}

@external(erlang, "domain_server", "start_link")
fn do_start_link(domain: String) -> Result(Pid, Dynamic)

@external(erlang, "domain_server", "stop")
fn do_stop(domain: String) -> Result(Nil, Dynamic)

@external(erlang, "domain_server", "is_running")
fn do_is_running(domain: String) -> Bool
