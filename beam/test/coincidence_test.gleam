import conversation/coincidence
import gleeunit/should

// -- shannon_equivalence --

pub fn shannon_equivalence_passes_test() {
  let source = "grammar @test {\n  type = a | b | c\n}\n"
  let assert Ok(reason) = coincidence.check_shannon_equivalence(source)
  should.not_equal(reason, "")
}

pub fn shannon_equivalence_via_check_property_test() {
  let source = "grammar @test {\n  type = a | b\n}\n"
  let assert Ok(reason) = coincidence.check_property(source, "shannon_equivalence")
  should.not_equal(reason, "")
}

// -- connected --

pub fn connected_trivially_passes_test() {
  // Single type, no references -- trivially connected.
  let source = "grammar @test {\n  type = a | b\n}\n"
  let assert Ok(reason) = coincidence.check_connected(source)
  should.not_equal(reason, "")
}

// -- bipartite --

pub fn bipartite_trivially_passes_test() {
  let source = "grammar @test {\n  type = a | b\n}\n"
  let assert Ok(reason) = coincidence.check_bipartite(source)
  should.not_equal(reason, "")
}

// -- exhaustive --

pub fn exhaustive_passes_test() {
  let source = "grammar @test {\n  type = a | b\n}\n"
  let assert Ok(reason) = coincidence.check_exhaustive(source)
  should.not_equal(reason, "")
}

// -- error cases --

pub fn check_property_unknown_property_test() {
  let source = "grammar @test {\n  type = a | b\n}\n"
  let assert Error(reason) = coincidence.check_property(source, "nonexistent")
  should.equal(reason, "unknown property: nonexistent")
}

pub fn check_property_invalid_source_test() {
  let assert Error(_reason) = coincidence.check_shannon_equivalence("@@@invalid")
}

pub fn check_property_no_grammar_test() {
  // Source with no grammar block
  let assert Error(reason) = coincidence.check_shannon_equivalence("in @something")
  should.equal(reason, "no grammar block")
}
