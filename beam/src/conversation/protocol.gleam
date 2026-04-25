//// Protocol types — the contract between conversation AST and BEAM runtime.
////
//// A conversation specifies desired BEAM state. The runtime converges toward it.
//// These types define what the conversation says. The runtime decides how to get there.

/// What a conversation specifies as desired BEAM state.
pub type Spec {
  /// Multi-arm dispatch. Subject = what to evaluate. First match wins.
  Case(subject: String, arms: List(Arm))
  /// Multi-arm dispatch. ALL matching arms fire. Produces deltas from each.
  Branch(arms: List(Arm))
  /// Guard clause. If predicate holds, apply the inner spec.
  When(op: Op, path: String, literal: String, then: Spec)
  /// Desired process state. The runtime converges toward this.
  DesiredState(process: String, state: String)
  /// No change. Current state is acceptable.
  Pass
}

/// One arm in a case dispatch.
pub type Arm {
  Arm(pattern: Pattern, body: Spec)
}

/// What an arm matches against.
pub type Pattern {
  /// Comparison: `> 0.1`, `== "active"`.
  Cmp(op: Op, value: String)
  /// Wildcard: `_`. Matches anything.
  Wildcard
}

/// Comparison operator — shared by When guards and Cmp patterns.
pub type Op {
  Gt
  Lt
  Gte
  Lte
  Eq
  Ne
}
