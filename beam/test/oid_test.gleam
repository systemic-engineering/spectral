import conversation/oid
import gleeunit/should

pub fn from_bytes_deterministic_test() {
  // Same input always produces same oid
  let a = oid.from_bytes(<<"hello":utf8>>)
  let b = oid.from_bytes(<<"hello":utf8>>)
  oid.equals(a, b) |> should.be_true()
}

pub fn different_input_different_oid_test() {
  let a = oid.from_bytes(<<"hello":utf8>>)
  let b = oid.from_bytes(<<"world":utf8>>)
  oid.equals(a, b) |> should.be_false()
}

pub fn to_string_roundtrip_test() {
  let a = oid.from_bytes(<<"test":utf8>>)
  let s = oid.to_string(a)
  let b = oid.from_string(s)
  oid.equals(a, b) |> should.be_true()
}

pub fn to_string_is_hex_test() {
  let a = oid.from_bytes(<<"abc":utf8>>)
  let s = oid.to_string(a)
  // SHA-512 produces 128 hex characters
  should.equal(128, string_length(s))
}

fn string_length(s: String) -> Int {
  do_string_length(s)
}

@external(erlang, "string", "length")
fn do_string_length(s: String) -> Int
