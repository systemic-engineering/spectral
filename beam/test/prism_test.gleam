import conversation/prism
import gleeunit/should

/// Identity matrix = traverse all. Preview always matches.
pub fn identity_prism_preview_test() {
  let p =
    prism.new([
      [1.0, 0.0, 0.0],
      [0.0, 1.0, 0.0],
      [0.0, 0.0, 1.0],
    ])

  let source = [1.0, 2.0, 3.0]
  let assert Ok(focus) = prism.preview(p, source)
  should.equal(focus, [1.0, 2.0, 3.0])
}

/// Single-variant selection: e1 from R3.
pub fn basis_selector_preview_test() {
  let p =
    prism.new([
      [1.0, 0.0, 0.0],
      [0.0, 0.0, 0.0],
      [0.0, 0.0, 0.0],
    ])

  let source = [5.0, 7.0, 9.0]
  let assert Ok(focus) = prism.preview(p, source)
  should.equal(focus, [5.0, 0.0, 0.0])
}

/// Zero in projected subspace -> Error.
pub fn preview_no_match_test() {
  let p =
    prism.new([
      [1.0, 0.0, 0.0],
      [0.0, 0.0, 0.0],
      [0.0, 0.0, 0.0],
    ])

  let source = [0.0, 3.0, 4.0]
  should.be_error(prism.preview(p, source))
}

/// Review embeds focus into full space via P^T.
pub fn review_embeds_test() {
  let p =
    prism.new([
      [1.0, 0.0, 0.0],
      [0.0, 1.0, 0.0],
      [0.0, 0.0, 0.0],
    ])

  let focus = [3.0, 4.0, 0.0]
  let result = prism.review(p, focus)
  should.equal(result, [3.0, 4.0, 0.0])
}

/// Identity transform in modify = no change.
pub fn modify_identity_transform_test() {
  let p =
    prism.new([
      [1.0, 0.0, 0.0],
      [0.0, 0.0, 0.0],
      [0.0, 0.0, 0.0],
    ])

  let identity = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
  ]

  let source = [5.0, 7.0, 9.0]
  let result = prism.modify(p, source, identity)
  should.equal(result, [5.0, 7.0, 9.0])
}

/// Transform scales the matched subspace, complement unchanged.
pub fn modify_scales_subspace_test() {
  let p =
    prism.new([
      [1.0, 0.0, 0.0],
      [0.0, 0.0, 0.0],
      [0.0, 0.0, 0.0],
    ])

  let scale2 = [
    [2.0, 0.0, 0.0],
    [0.0, 2.0, 0.0],
    [0.0, 0.0, 2.0],
  ]

  let source = [5.0, 7.0, 9.0]
  let result = prism.modify(p, source, scale2)
  should.equal(result, [10.0, 7.0, 9.0])
}

/// Composing two projections selects intersection of subspaces.
pub fn compose_intersection_test() {
  let p1 =
    prism.new([
      [1.0, 0.0, 0.0],
      [0.0, 1.0, 0.0],
      [0.0, 0.0, 0.0],
    ])

  let p2 =
    prism.new([
      [0.0, 0.0, 0.0],
      [0.0, 1.0, 0.0],
      [0.0, 0.0, 1.0],
    ])

  let composed = prism.compose(p1, p2)

  let source = [1.0, 2.0, 3.0]
  let assert Ok(focus) = prism.preview(composed, source)
  should.equal(focus, [0.0, 2.0, 0.0])
}

/// Opaque type correctly reports dimension.
pub fn dimension_accessor_test() {
  let p =
    prism.new([
      [1.0, 0.0, 0.0],
      [0.0, 1.0, 0.0],
      [0.0, 0.0, 1.0],
    ])

  should.equal(prism.dimension(p), 3)
}
