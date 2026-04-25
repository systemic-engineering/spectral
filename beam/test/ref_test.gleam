import conversation/oid
import conversation/ref.{At, Inline, NonEmpty}
import gleeunit/should

pub fn non_empty_construction_test() {
  let ne = ref.non_empty(1, [2, 3])
  ref.to_list(ne) |> should.equal([1, 2, 3])
}

pub fn non_empty_single_test() {
  let ne = ref.non_empty("a", [])
  ref.to_list(ne) |> should.equal(["a"])
}

pub fn from_list_ok_test() {
  let assert Ok(ne) = ref.from_list([1, 2, 3])
  ref.to_list(ne) |> should.equal([1, 2, 3])
}

pub fn from_list_empty_errors_test() {
  ref.from_list([]) |> should.be_error()
}

pub fn scope_creates_scoped_oid_test() {
  let o = oid.from_bytes(<<"test":utf8>>)
  let scoped = ref.scope(o)
  let retrieved = ref.oid(scoped)
  oid.equals(o, retrieved) |> should.be_true()
}

pub fn resolve_inline_test() {
  let r = Inline(42)
  let resolved = ref.resolve(r, fn(x) { oid.from_bytes(<<x:int>>) })
  // Inline values resolve to themselves
  should.equal(resolved, 42)
}

pub fn ref_at_construction_test() {
  let o = oid.from_bytes(<<"actor":utf8>>)
  let scoped = ref.scope(o)
  let r = At(scoped)
  // At wraps a ScopedOid — verify we can pattern match it
  case r {
    At(_) -> should.be_true(True)
    Inline(_) -> should.be_true(False)
  }
}
