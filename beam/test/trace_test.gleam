import conversation/key
import conversation/oid
import conversation/trace
import gleam/option.{None, Some}
import gleeunit/should

pub fn new_trace_test() {
  let kp = key.generate()
  let actor_oid = key.oid(key.public_key(kp))
  let t = trace.new(actor_oid, kp, "hello", None)
  // Trace should have the value we put in
  should.equal(trace.value(t), "hello")
}

pub fn trace_verify_test() {
  let kp = key.generate()
  let pub_key = key.public_key(kp)
  let actor_oid = key.oid(pub_key)
  let t = trace.new(actor_oid, kp, "signed message", None)
  trace.verify(t, pub_key) |> should.be_true()
}

pub fn trace_verify_wrong_key_fails_test() {
  let kp1 = key.generate()
  let kp2 = key.generate()
  let actor_oid = key.oid(key.public_key(kp1))
  let t = trace.new(actor_oid, kp1, "message", None)
  // Verify with wrong key should fail
  trace.verify(t, key.public_key(kp2)) |> should.be_false()
}

pub fn trace_oid_deterministic_test() {
  let kp = key.generate()
  let actor_oid = key.oid(key.public_key(kp))
  let t = trace.new(actor_oid, kp, "data", None)
  let oid1 = trace.oid(t)
  let oid2 = trace.oid(t)
  oid.equals(oid1, oid2) |> should.be_true()
}

pub fn trace_with_parent_test() {
  let kp = key.generate()
  let actor_oid = key.oid(key.public_key(kp))
  let parent = oid.from_bytes(<<"parent":utf8>>)
  let t = trace.new(actor_oid, kp, "child", Some(parent))
  should.equal(trace.value(t), "child")
  trace.verify(t, key.public_key(kp)) |> should.be_true()
}

pub fn different_values_different_oids_test() {
  let kp = key.generate()
  let actor_oid = key.oid(key.public_key(kp))
  let t1 = trace.new(actor_oid, kp, "value1", None)
  let t2 = trace.new(actor_oid, kp, "value2", None)
  oid.equals(trace.oid(t1), trace.oid(t2)) |> should.be_false()
}
