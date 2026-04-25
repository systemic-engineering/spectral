//// Prism — root optics primitive. A projection matrix.
////
//// A Prism selects which variant you're working with.
//// preview = project + check nonzero.
//// review  = embed via transpose.
//// modify  = complement + transform.
//// compose = matmul.
////
//// The routing IS the computation. Fortran handles the math.
////
//// Build the NIF before running tests:
////   just build-prism-nif

/// An opaque projection matrix. Constructed through `new`.
pub opaque type Prism {
  Prism(dimension: Int, projection: List(List(Float)))
}

/// Construct a Prism from a projection matrix (list of rows).
pub fn new(projection: List(List(Float))) -> Prism {
  let dimension = list_length(projection)
  Prism(dimension: dimension, projection: projection)
}

/// The dimension of this prism's space.
pub fn dimension(p: Prism) -> Int {
  p.dimension
}

/// Project source into the prism's subspace.
/// Returns Ok(focus) if the projection is nonzero, Error(Nil) otherwise.
pub fn preview(p: Prism, source: List(Float)) -> Result(List(Float), Nil) {
  case nif_preview(p.dimension, p.projection, source) {
    Ok(focus) -> Ok(focus)
    Error(_) -> Error(Nil)
  }
}

/// Embed a focus value back into the full space via P^T.
pub fn review(p: Prism, focus: List(Float)) -> List(Float) {
  nif_review(p.dimension, p.projection, focus)
}

/// Transform the matched part, leave the complement unchanged.
/// result = (I - P) * source + transform * (P * source)
pub fn modify(
  p: Prism,
  source: List(Float),
  transform: List(List(Float)),
) -> List(Float) {
  nif_modify(p.dimension, p.projection, source, transform)
}

/// Compose two prisms. The result selects the intersection of subspaces.
/// composed = p2 * p1
pub fn compose(p1: Prism, p2: Prism) -> Prism {
  let composed = nif_compose(p1.dimension, p1.projection, p2.projection)
  Prism(dimension: p1.dimension, projection: composed)
}

// --- FFI to Fortran via NIF ---

@external(erlang, "conversation_prism_nif", "prism_preview")
fn nif_preview(
  n: Int,
  projection: List(List(Float)),
  source: List(Float),
) -> Result(List(Float), Nil)

@external(erlang, "conversation_prism_nif", "prism_review")
fn nif_review(
  n: Int,
  projection: List(List(Float)),
  focus: List(Float),
) -> List(Float)

@external(erlang, "conversation_prism_nif", "prism_modify")
fn nif_modify(
  n: Int,
  projection: List(List(Float)),
  source: List(Float),
  transform: List(List(Float)),
) -> List(Float)

@external(erlang, "conversation_prism_nif", "prism_compose")
fn nif_compose(
  n: Int,
  p1: List(List(Float)),
  p2: List(List(Float)),
) -> List(List(Float))

// --- Helpers ---

fn list_length(l: List(a)) -> Int {
  do_list_length(l, 0)
}

fn do_list_length(l: List(a), acc: Int) -> Int {
  case l {
    [] -> acc
    [_, ..rest] -> do_list_length(rest, acc + 1)
  }
}
