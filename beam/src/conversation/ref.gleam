//// Ref — the @ operator in Gleam.
////
//// Content-addressed references with scope typing.

import conversation/oid.{type Oid}

/// A scoped content address. The phantom type parameter carries scope information.
pub opaque type ScopedOid(scope) {
  ScopedOid(oid: Oid)
}

/// A reference to a value — either inline or by content address.
pub type Ref(a) {
  At(ScopedOid(a))
  Inline(a)
}

/// A non-empty list. Guarantees at least one element.
pub type NonEmpty(a) {
  NonEmpty(first: a, rest: List(a))
}

/// Construct a NonEmpty list.
pub fn non_empty(first: a, rest: List(a)) -> NonEmpty(a) {
  NonEmpty(first: first, rest: rest)
}

/// Construct a NonEmpty from a regular list. Fails if empty.
pub fn from_list(items: List(a)) -> Result(NonEmpty(a), Nil) {
  case items {
    [] -> Error(Nil)
    [first, ..rest] -> Ok(NonEmpty(first: first, rest: rest))
  }
}

/// Convert a NonEmpty back to a regular list.
pub fn to_list(ne: NonEmpty(a)) -> List(a) {
  [ne.first, ..ne.rest]
}

/// Create a ScopedOid from an Oid.
pub fn scope(oid: Oid) -> ScopedOid(scope) {
  ScopedOid(oid: oid)
}

/// Extract the Oid from a ScopedOid.
pub fn oid(scoped: ScopedOid(scope)) -> Oid {
  scoped.oid
}

/// Resolve a Ref. Inline values are returned directly.
/// At references resolve through the provided hash function.
pub fn resolve(r: Ref(a), _hash_fn: fn(a) -> Oid) -> a {
  case r {
    Inline(value) -> value
    At(_scoped) -> panic as "At resolution requires a lookup function — not yet implemented"
  }
}
