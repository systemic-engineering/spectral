/// gen_prism — the five operations as a BEAM process behaviour.
///
/// A gen_prism process wraps five callbacks: focus, project, split, zoom, refract.
/// Each operation is a message to the process. The process holds state between
/// operations, making it possible to build interactive pipelines.

/// The five prism callbacks.
pub type PrismCallbacks(input, focused, projected, crystal, error) {
  PrismCallbacks(
    focus: fn(input) -> Result(focused, error),
    project: fn(focused) -> Result(projected, error),
    split: fn(projected) -> List(projected),
    zoom: fn(projected) -> Result(projected, error),
    refract: fn(projected) -> Result(crystal, error),
  )
}

/// Messages that a gen_prism process handles.
pub type PrismMsg(input, focused, projected, crystal, error) {
  Focus(input, reply: fn(Result(focused, error)) -> Nil)
  Project(focused, reply: fn(Result(projected, error)) -> Nil)
  Split(projected, reply: fn(List(projected)) -> Nil)
  Zoom(projected, reply: fn(Result(projected, error)) -> Nil)
  Refract(projected, reply: fn(Result(crystal, error)) -> Nil)
}
