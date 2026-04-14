import conversation/nif
import gleeunit/should

pub fn parse_conv_returns_oid_test() {
  let source = "grammar @test {\n  type = a | b\n}\n"
  let assert Ok(oid) = nif.parse_conv(source)
  should.not_equal(oid, "")
}

pub fn parse_conv_error_test() {
  let assert Error(msg) = nif.parse_conv("@@@invalid")
  should.not_equal(msg, "")
}

pub fn parse_conv_empty_grammar_test() {
  let source = "grammar @empty {\n}\n"
  let assert Ok(oid) = nif.parse_conv(source)
  should.not_equal(oid, "")
}

pub fn parse_conv_deterministic_test() {
  let source = "grammar @test {\n  type = a | b\n}\n"
  let assert Ok(oid1) = nif.parse_conv(source)
  let assert Ok(oid2) = nif.parse_conv(source)
  should.equal(oid1, oid2)
}
