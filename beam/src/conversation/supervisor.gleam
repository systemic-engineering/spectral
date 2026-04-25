//// Conversation supervisor — static supervision tree.
////
//// RestForOne:
////   @compiler → garden (factory_supervisor)
////
//// @compiler compiles grammars → loads BEAM modules → returns traces.
//// Garden starts domain servers as dynamic children.
////
//// If @compiler crashes, garden and all domain servers restart (clean slate).
//// If a domain server crashes, the garden factory_supervisor restarts it.

import conversation/compiler
import conversation/garden
import gleam/erlang/process
import gleam/otp/actor
import gleam/otp/factory_supervisor
import gleam/otp/static_supervisor.{type Supervisor} as supervisor
import gleam/otp/supervision

/// Start the conversation supervision tree.
/// Returns the supervisor handle.
pub fn start(
  compiler_name: process.Name(compiler.Message),
  garden_name: process.Name(
    factory_supervisor.Message(String, String),
  ),
) -> actor.StartResult(Supervisor) {
  supervisor.new(supervisor.RestForOne)
  |> supervisor.restart_tolerance(intensity: 3, period: 60)
  |> supervisor.add(compiler_child(compiler_name))
  |> supervisor.add(garden_child(garden_name))
  |> supervisor.start
}

/// Create a child specification for embedding this supervisor
/// in a parent supervision tree (e.g. Reed's top-level supervisor).
pub fn supervised(
  compiler_name: process.Name(compiler.Message),
  garden_name: process.Name(
    factory_supervisor.Message(String, String),
  ),
) -> supervision.ChildSpecification(Supervisor) {
  supervision.supervisor(fn() { start(compiler_name, garden_name) })
}

fn compiler_child(
  name: process.Name(compiler.Message),
) -> supervision.ChildSpecification(process.Subject(compiler.Message)) {
  supervision.worker(run: fn() { compiler.start_named(name) })
}

fn garden_child(
  name: process.Name(
    factory_supervisor.Message(String, String),
  ),
) -> supervision.ChildSpecification(
  factory_supervisor.Supervisor(String, String),
) {
  garden.supervised(name)
}
