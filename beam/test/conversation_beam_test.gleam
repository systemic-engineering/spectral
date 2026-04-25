import conversation/protocol.{
  Arm, Branch, Case, Cmp, DesiredState, Eq, Gt, Gte, Lt, Lte, Ne, Pass, When,
  Wildcard,
}
import conversation/runtime.{StartProcess, converge}
import gleeunit
import gleeunit/should

pub fn main() {
  gleeunit.main()
}

// -- Protocol types construct --

pub fn pass_spec_test() {
  let spec = Pass
  converge(spec) |> should.equal([])
}

pub fn desired_state_spec_test() {
  let spec = DesiredState("health_monitor", "critical")
  converge(spec) |> should.equal([StartProcess("health_monitor", "critical")])
}

pub fn wildcard_arm_matches_test() {
  let spec = Case("x", [Arm(Wildcard, DesiredState("p", "s"))])
  converge(spec) |> should.equal([StartProcess("p", "s")])
}

pub fn cmp_arm_falls_through_to_wildcard_test() {
  // Cmp patterns don't match yet (no runtime context), so wildcard wins
  let spec =
    Case("x", [
      Arm(Cmp(Gt, "0.1"), DesiredState("a", "high")),
      Arm(Wildcard, DesiredState("a", "low")),
    ])
  converge(spec) |> should.equal([StartProcess("a", "low")])
}

pub fn when_guard_applies_test() {
  let spec = When(Gt, "error.rate", "0.1", DesiredState("monitor", "alert"))
  converge(spec) |> should.equal([StartProcess("monitor", "alert")])
}

pub fn empty_case_produces_no_deltas_test() {
  let spec = Case("x", [])
  converge(spec) |> should.equal([])
}

// -- Branch — all matching arms fire --

pub fn branch_empty_test() {
  let spec = Branch([])
  converge(spec) |> should.equal([])
}

pub fn branch_single_wildcard_fires_test() {
  let spec = Branch([Arm(Wildcard, DesiredState("p", "s"))])
  converge(spec) |> should.equal([StartProcess("p", "s")])
}

pub fn branch_all_wildcards_fire_test() {
  // Unlike Case (first wins), Branch fires ALL matching arms
  let spec =
    Branch([
      Arm(Wildcard, DesiredState("a", "x")),
      Arm(Wildcard, DesiredState("b", "y")),
    ])
  converge(spec) |> should.equal([StartProcess("a", "x"), StartProcess("b", "y")])
}

pub fn branch_cmp_no_match_skipped_test() {
  // Cmp stubs to False — arm skipped, no deltas
  let spec = Branch([Arm(Cmp(Gt, "0.1"), DesiredState("a", "high"))])
  converge(spec) |> should.equal([])
}

pub fn branch_collects_matching_skips_nonmatching_test() {
  // Cmp falls through, wildcard fires — only wildcard produces a delta
  let spec =
    Branch([
      Arm(Cmp(Gt, "0.1"), DesiredState("a", "high")),
      Arm(Wildcard, DesiredState("b", "low")),
    ])
  converge(spec) |> should.equal([StartProcess("b", "low")])
}

// -- All Op variants construct --

pub fn all_ops_construct_test() {
  // Verify all six Op variants can be used in Cmp patterns
  let _gt = Cmp(Gt, "1")
  let _lt = Cmp(Lt, "2")
  let _gte = Cmp(Gte, "3")
  let _lte = Cmp(Lte, "4")
  let _eq = Cmp(Eq, "5")
  let _ne = Cmp(Ne, "6")
  should.be_true(True)
}
