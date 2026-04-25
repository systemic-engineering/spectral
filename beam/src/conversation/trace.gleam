//// Trace — witnessed record.
////
//// A signed, content-addressed record of an actor's action.

import conversation/key.{type Key, type KeyPair}
import conversation/oid.{type Oid}
import conversation/ref.{type ScopedOid}
import gleam/option.{type Option}

/// A witnessed record. Signed by the actor that produced it.
pub type Trace(value) {
  Trace(
    oid: Oid,
    actor: ScopedOid(Key),
    parent: Option(Oid),
    value: value,
    signature: BitArray,
    timestamp: Int,
  )
}

/// Create a new trace. Signs the value with the actor's keypair.
pub fn new(
  actor_oid: ScopedOid(Key),
  kp: KeyPair,
  value: value,
  parent: Option(Oid),
) -> Trace(value) {
  let timestamp = do_system_time_ms()
  let payload = do_term_to_binary(#(value, parent, timestamp))
  let signature = key.sign(kp, payload)
  let trace_oid =
    oid.from_bytes(do_term_to_binary(#(value, parent, timestamp, signature)))
  Trace(
    oid: trace_oid,
    actor: actor_oid,
    parent: parent,
    value: value,
    signature: signature,
    timestamp: timestamp,
  )
}

/// Verify a trace's signature against a public key.
pub fn verify(t: Trace(value), k: Key) -> Bool {
  let payload = do_term_to_binary(#(t.value, t.parent, t.timestamp))
  key.verify(k, payload, t.signature)
}

/// Get the value from a trace.
pub fn value(t: Trace(value)) -> value {
  t.value
}

/// Get the content address of a trace.
pub fn oid(t: Trace(value)) -> Oid {
  t.oid
}

@external(erlang, "crypto_ffi", "system_time_ms")
fn do_system_time_ms() -> Int

@external(erlang, "crypto_ffi", "term_to_binary")
fn do_term_to_binary(term: a) -> BitArray
