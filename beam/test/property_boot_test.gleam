import conversation/boot
import conversation/coincidence
import conversation/compiler
import conversation/domain
import conversation/loader
import conversation/supervisor as conv_sup
import gleam/erlang/process
import gleam/list
import gleam/otp/factory_supervisor
import gleam/string
import gleeunit/should

fn setup() -> #(
  process.Subject(compiler.Message),
  process.Name(factory_supervisor.Message(String, String)),
) {
  let compiler_name = process.new_name("compiler")
  let garden_name = process.new_name("garden")
  let assert Ok(_) = conv_sup.start(compiler_name, garden_name)
  let _ = coincidence.start_server()
  let subject = process.named_subject(compiler_name)
  #(subject, garden_name)
}

/// Inline grammar with requires — proves the full pipeline without garden files.
pub fn inline_grammar_with_requires_test() {
  let source =
    "grammar @inline_prop {
  type = a | b | c

  requires shannon_equivalence
}
"

  let #(subject, garden_name) = setup()
  let assert Ok(beams) = boot.boot(subject, garden_name, [source])
  let domains = boot.results(beams)

  should.be_true(domain.is_running("inline_prop"))
  should.be_true(loader.is_loaded("conv_inline_prop"))

  let assert Ok(requires) = loader.get_requires("conv_inline_prop")
  should.equal(requires, ["shannon_equivalence"])

  let _ = coincidence.stop_server()
}

/// Inline grammar with both requires and invariant.
pub fn inline_grammar_with_requires_and_invariant_test() {
  let source =
    "grammar @dual_prop {
  type = x | y | z

  requires shannon_equivalence
  invariant connected
}
"

  let #(subject, garden_name) = setup()
  let assert Ok(beams) = boot.boot(subject, garden_name, [source])
  let domains = boot.results(beams)

  should.be_true(domain.is_running("dual_prop"))

  let assert Ok(requires) = loader.get_requires("conv_dual_prop")
  let assert Ok(invariants) = loader.get_invariants("conv_dual_prop")
  should.equal(requires, ["shannon_equivalence"])
  should.equal(invariants, ["connected"])

  let _ = coincidence.stop_server()
}

/// Full pipeline: boot infrastructure domains, then @training with
/// requires/invariant. Properties enforced through @coincidence.
pub fn full_property_pipeline_test() {
  let conv = "/Users/alexwolf/dev/projects/conversation/conv"
  let garden =
    "/Users/alexwolf/dev/systemic.engineering/garden/public"
  let assert Ok(property_source) =
    boot.read_file(conv <> "/property.conv")
  let assert Ok(topology_source) =
    boot.read_file(conv <> "/topology.conv")
  let assert Ok(training_source) =
    boot.read_file(garden <> "/@training/training.conv")

  let #(subject, garden_name) = setup()

  // Boot infrastructure first, then application
  let assert Ok(_infra) =
    boot.boot(subject, garden_name, [property_source, topology_source])

  // @training has disconnected type groups — enforcement correctly rejects
  let result = boot.boot(subject, garden_name, [training_source])
  should.be_error(result)
  let assert Error(reason) = result
  should.be_true(string.contains(reason, "property enforcement"))
  should.be_true(string.contains(reason, "connected"))

  // Infrastructure domains compiled before the failure
  should.be_true(domain.is_running("property"))
  should.be_true(domain.is_running("topology"))

  let _ = coincidence.stop_server()
}

/// Compilation fails when a required property is unknown.
pub fn enforcement_unknown_requires_fails_test() {
  let source =
    "grammar @bad_req {
  type = a | b

  requires nonexistent_property
}
"
  let #(subject, garden_name) = setup()
  let result = boot.boot(subject, garden_name, [source])
  should.be_error(result)
  let _ = coincidence.stop_server()
}

/// Compilation succeeds when all required properties pass.
pub fn enforcement_valid_requires_passes_test() {
  let source =
    "grammar @good_req {
  type = a | b | c

  requires shannon_equivalence
}
"
  let #(subject, garden_name) = setup()
  let assert Ok(_domains) = boot.boot(subject, garden_name, [source])
  let _ = coincidence.stop_server()
}

/// Compilation fails when an invariant property is unknown.
pub fn enforcement_unknown_invariant_fails_test() {
  let source =
    "grammar @bad_inv {
  type = a | b

  invariant nonexistent_invariant
}
"
  let #(subject, garden_name) = setup()
  let result = boot.boot(subject, garden_name, [source])
  should.be_error(result)
  let _ = coincidence.stop_server()
}
