/// gen_prism runtime — OTP process wrapper for prism operations.
///
/// Wraps PrismCallbacks in a gen_server-like process.
/// Each message dispatches to the corresponding callback.

import gleam/erlang/process.{type Subject}
import gleam/otp/actor
import gen_prism.{
  type PrismCallbacks, type PrismMsg, Focus, Project, Refract, Split, Zoom,
}

/// State held by the gen_prism process.
pub type State(input, focused, projected, crystal, error) {
  State(
    callbacks: PrismCallbacks(input, focused, projected, crystal, error),
  )
}

/// Handle a prism message by dispatching to the appropriate callback.
pub fn handle_message(
  msg: PrismMsg(input, focused, projected, crystal, error),
  state: State(input, focused, projected, crystal, error),
) -> actor.Next(PrismMsg(input, focused, projected, crystal, error), State(input, focused, projected, crystal, error)) {
  case msg {
    Focus(input, reply) -> {
      let result = state.callbacks.focus(input)
      reply(result)
      actor.continue(state)
    }
    Project(focused, reply) -> {
      let result = state.callbacks.project(focused)
      reply(result)
      actor.continue(state)
    }
    Split(projected, reply) -> {
      let result = state.callbacks.split(projected)
      reply(result)
      actor.continue(state)
    }
    Zoom(projected, reply) -> {
      let result = state.callbacks.zoom(projected)
      reply(result)
      actor.continue(state)
    }
    Refract(projected, reply) -> {
      let result = state.callbacks.refract(projected)
      reply(result)
      actor.continue(state)
    }
  }
}

/// Start a gen_prism process with the given callbacks.
///
/// Returns a Subject that can receive PrismMsg messages.
pub fn start(
  callbacks: PrismCallbacks(input, focused, projected, crystal, error),
) -> Result(Subject(PrismMsg(input, focused, projected, crystal, error)), actor.StartError) {
  actor.start(State(callbacks: callbacks), handle_message)
}
