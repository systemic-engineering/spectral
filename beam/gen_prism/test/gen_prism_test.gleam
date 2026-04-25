import gen_prism.{PrismCallbacks}
import gleeunit
import gleeunit/should

pub fn main() {
  gleeunit.main()
}

/// Verify PrismCallbacks can be constructed with simple functions.
pub fn callbacks_construction_test() {
  let callbacks =
    PrismCallbacks(
      focus: fn(input: Int) -> Result(Int, String) { Ok(input) },
      project: fn(focused: Int) -> Result(Int, String) { Ok(focused + 1) },
      split: fn(projected: Int) -> List(Int) { [projected] },
      zoom: fn(projected: Int) -> Result(Int, String) { Ok(projected * 2) },
      refract: fn(projected: Int) -> Result(String, String) { Ok("done") },
    )
  // Verify focus callback works
  callbacks.focus(42)
  |> should.be_ok
  |> should.equal(42)
}

/// Verify project callback transforms the value.
pub fn project_callback_test() {
  let callbacks =
    PrismCallbacks(
      focus: fn(input: Int) -> Result(Int, String) { Ok(input) },
      project: fn(focused: Int) -> Result(Int, String) { Ok(focused + 10) },
      split: fn(projected: Int) -> List(Int) { [projected] },
      zoom: fn(projected: Int) -> Result(Int, String) { Ok(projected) },
      refract: fn(projected: Int) -> Result(String, String) { Ok("ok") },
    )
  callbacks.project(5)
  |> should.be_ok
  |> should.equal(15)
}

/// Verify split produces a list.
pub fn split_callback_test() {
  let callbacks =
    PrismCallbacks(
      focus: fn(input: Int) -> Result(Int, String) { Ok(input) },
      project: fn(focused: Int) -> Result(Int, String) { Ok(focused) },
      split: fn(projected: Int) -> List(Int) { [projected, projected + 1] },
      zoom: fn(projected: Int) -> Result(Int, String) { Ok(projected) },
      refract: fn(projected: Int) -> Result(String, String) { Ok("ok") },
    )
  callbacks.split(3)
  |> should.equal([3, 4])
}

/// Verify zoom transforms within the projected space.
pub fn zoom_callback_test() {
  let callbacks =
    PrismCallbacks(
      focus: fn(input: Int) -> Result(Int, String) { Ok(input) },
      project: fn(focused: Int) -> Result(Int, String) { Ok(focused) },
      split: fn(projected: Int) -> List(Int) { [projected] },
      zoom: fn(projected: Int) -> Result(Int, String) { Ok(projected * 3) },
      refract: fn(projected: Int) -> Result(String, String) { Ok("ok") },
    )
  callbacks.zoom(7)
  |> should.be_ok
  |> should.equal(21)
}

/// Verify refract produces the final crystal.
pub fn refract_callback_test() {
  let callbacks =
    PrismCallbacks(
      focus: fn(input: Int) -> Result(Int, String) { Ok(input) },
      project: fn(focused: Int) -> Result(Int, String) { Ok(focused) },
      split: fn(projected: Int) -> List(Int) { [projected] },
      zoom: fn(projected: Int) -> Result(Int, String) { Ok(projected) },
      refract: fn(projected: Int) -> Result(String, String) {
        Ok("crystal:" <> int_to_string(projected))
      },
    )
  callbacks.refract(42)
  |> should.be_ok
  |> should.equal("crystal:42")
}

@external(erlang, "erlang", "integer_to_list")
fn int_to_string(n: Int) -> String
