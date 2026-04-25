import conversation/coincidence
import conversation/domain
import gleeunit/should

/// Server starts, reports running, stops cleanly.
pub fn coincidence_server_starts_and_stops_test() {
  let assert Ok(_) = coincidence.start_server()
  should.be_true(coincidence.is_running())
  let assert Ok(_) = coincidence.stop_server()
  should.be_false(coincidence.is_running())
}

/// Double-start returns error (already registered).
pub fn coincidence_server_double_start_test() {
  let assert Ok(_) = coincidence.start_server()
  let assert Error(_) = coincidence.start_server()
  let assert Ok(_) = coincidence.stop_server()
}

/// Named action: shannon_equivalence via domain.call_action.
pub fn shannon_equivalence_via_server_test() {
  let assert Ok(_) = coincidence.start_server()
  let source = "grammar @test {\n  type = a | b | c\n}\n"
  let assert Ok(_) =
    domain.call_action("coincidence", "shannon_equivalence", [source])
  let assert Ok(_) = coincidence.stop_server()
}

/// Named action: connected via domain.call_action.
pub fn connected_via_server_test() {
  let assert Ok(_) = coincidence.start_server()
  let source = "grammar @test {\n  type = a | b\n}\n"
  let assert Ok(_) = domain.call_action("coincidence", "connected", [source])
  let assert Ok(_) = coincidence.stop_server()
}

/// Named action: bipartite via domain.call_action.
pub fn bipartite_via_server_test() {
  let assert Ok(_) = coincidence.start_server()
  let source = "grammar @test {\n  type = a | b\n}\n"
  let assert Ok(_) = domain.call_action("coincidence", "bipartite", [source])
  let assert Ok(_) = coincidence.stop_server()
}

/// Named action: exhaustive via domain.call_action.
pub fn exhaustive_via_server_test() {
  let assert Ok(_) = coincidence.start_server()
  let source = "grammar @test {\n  type = a | b\n}\n"
  let assert Ok(_) = domain.call_action("coincidence", "exhaustive", [source])
  let assert Ok(_) = coincidence.stop_server()
}

/// Generic check action dispatches by property name.
pub fn check_action_dispatches_test() {
  let assert Ok(_) = coincidence.start_server()
  let source = "grammar @test {\n  type = a | b | c\n}\n"
  let assert Ok(_) =
    domain.call_action("coincidence", "check", [
      "shannon_equivalence", source,
    ])
  let assert Ok(_) = coincidence.stop_server()
}

/// Fallback: unknown action returns domain echo tuple.
pub fn unknown_action_fallback_test() {
  let assert Ok(_) = coincidence.start_server()
  let assert Ok(_) =
    domain.call_action("coincidence", "whatever", ["arg1"])
  let assert Ok(_) = coincidence.stop_server()
}
