//// Runtime — convergence engine.
////
//// Evaluates a conversation spec against current BEAM state.
//// Returns the delta: what needs to change to reach desired state.

import conversation/protocol.{
  type Arm, type Op, type Pattern, type Spec, Cmp, DesiredState, Pass, Wildcard,
}
import gleam/list

/// What needs to change to reach desired state.
pub type Delta {
  /// A process should be started with this state.
  StartProcess(name: String, state: String)
  /// A process should transition from one state to another.
  UpdateState(name: String, from: String, to: String)
  /// A process should be stopped.
  StopProcess(name: String)
}

/// Evaluate a conversation spec. Returns the list of deltas needed.
pub fn converge(spec: Spec) -> List(Delta) {
  case spec {
    protocol.Case(_, arms) -> dispatch(arms)
    protocol.Branch(arms) -> branch_dispatch(arms)
    protocol.When(op, _path, literal, then) -> guard(op, literal, then)
    DesiredState(process, state) -> [StartProcess(process, state)]
    Pass -> []
  }
}

/// Try each arm in order. First match wins.
fn dispatch(arms: List(Arm)) -> List(Delta) {
  case arms {
    [] -> []
    [protocol.Arm(pattern, body), ..rest] ->
      case matches(pattern) {
        True -> converge(body)
        False -> dispatch(rest)
      }
  }
}

/// Try all arms. Collect deltas from every arm that matches.
fn branch_dispatch(arms: List(Arm)) -> List(Delta) {
  list.flat_map(arms, fn(arm) {
    let protocol.Arm(pattern, body) = arm
    case matches(pattern) {
      True -> converge(body)
      False -> []
    }
  })
}

/// Check if a pattern matches. Stub: wildcards always match,
/// comparisons require runtime context (not yet wired).
fn matches(pattern: Pattern) -> Bool {
  case pattern {
    Wildcard -> True
    Cmp(_, _) -> False
  }
}

/// Evaluate a guard. Stub: always applies (predicate eval not yet wired).
fn guard(_op: Op, _literal: String, then: Spec) -> List(Delta) {
  converge(then)
}
