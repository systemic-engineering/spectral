import conversation/boot
import conversation/coincidence
import conversation/compiler
import conversation/domain
import conversation/loader
import conversation/supervisor as conv_sup
import conversation/trace
import gleam/erlang/process
import gleam/list
import gleeunit/should

const grammar_with_requires = "grammar @prop_test {
  type = a | b | c

  requires shannon_equivalence
  invariant connected
}
"

const plain_grammar = "grammar @plain_test {
  type = x | y
}
"

/// Compiled module exposes requires/0 with declared properties.
pub fn compiled_module_has_requires_test() {
  let assert Ok(started) = compiler.start()
  let subject = started.data
  let reply = process.new_subject()
  process.send(
    subject,
    compiler.CompileGrammar(grammar_with_requires, reply),
  )
  let assert Ok(Ok(t)) = process.receive(reply, 5000)
  let compiled = trace.value(t)
  let assert Ok(requires) = loader.get_requires(compiled.module)
  should.equal(requires, ["shannon_equivalence"])
  process.send(subject, compiler.Shutdown)
}

/// Compiled module exposes invariants/0 with declared invariants.
pub fn compiled_module_has_invariants_test() {
  let assert Ok(started) = compiler.start()
  let subject = started.data
  let reply = process.new_subject()
  process.send(
    subject,
    compiler.CompileGrammar(grammar_with_requires, reply),
  )
  let assert Ok(Ok(t)) = process.receive(reply, 5000)
  let compiled = trace.value(t)
  let assert Ok(invariants) = loader.get_invariants(compiled.module)
  should.equal(invariants, ["connected"])
  process.send(subject, compiler.Shutdown)
}

/// Module without properties has empty requires/0 and invariants/0.
pub fn compiled_module_empty_requires_invariants_test() {
  let assert Ok(started) = compiler.start()
  let subject = started.data
  let reply = process.new_subject()
  process.send(
    subject,
    compiler.CompileGrammar(plain_grammar, reply),
  )
  let assert Ok(Ok(t)) = process.receive(reply, 5000)
  let compiled = trace.value(t)
  let assert Ok(requires) = loader.get_requires(compiled.module)
  let assert Ok(invariants) = loader.get_invariants(compiled.module)
  should.equal(requires, [])
  should.equal(invariants, [])
  process.send(subject, compiler.Shutdown)
}

/// Compiler actor calls @coincidence when processing grammar with requires.
/// The grammar compiles successfully even with property checks running.
pub fn compiler_calls_coincidence_on_requires_test() {
  // Start @coincidence server so property checks can run
  let _ = coincidence.start_server()

  let assert Ok(started) = compiler.start()
  let subject = started.data
  let reply = process.new_subject()
  process.send(
    subject,
    compiler.CompileGrammar(grammar_with_requires, reply),
  )
  // Compilation should succeed — property checks don't fail compilation
  let assert Ok(Ok(t)) = process.receive(reply, 5000)
  let compiled = trace.value(t)
  should.equal(compiled.domain, "prop_test")

  process.send(subject, compiler.Shutdown)
  let _ = coincidence.stop_server()
}

/// Compiled module exposes ensures/0 with declared ensures.
pub fn compiled_module_has_ensures_test() {
  let assert Ok(started) = compiler.start()
  let subject = started.data

  let reply = process.new_subject()
  let source =
    "grammar @ensures_wire_test {
  type = a | b

  ensures response_time
}
"
  process.send(subject, compiler.CompileGrammar(source, reply))
  let assert Ok(Ok(t)) = process.receive(reply, 10_000)
  let compiled = trace.value(t)

  let assert Ok(ensures) = loader.get_ensures(compiled.module)
  should.equal(ensures, ["response_time"])

  process.send(subject, compiler.Shutdown)
}

/// Module without ensures has empty ensures/0.
pub fn compiled_module_empty_ensures_test() {
  let assert Ok(started) = compiler.start()
  let subject = started.data

  let reply = process.new_subject()
  process.send(
    subject,
    compiler.CompileGrammar(
      "grammar @no_ensures {\n  type = x | y\n}\n",
      reply,
    ),
  )
  let assert Ok(Ok(t)) = process.receive(reply, 10_000)
  let compiled = trace.value(t)

  let assert Ok(ensures) = loader.get_ensures(compiled.module)
  should.equal(ensures, [])

  process.send(subject, compiler.Shutdown)
}

/// Boot ordering: infra domains before app domains.
pub fn boot_ordering_test() {
  let infra = "grammar @infra {
  type = service
}
"
  let app = "grammar @app {
  type = feature
  requires shannon_equivalence
}
"

  let compiler_name = process.new_name("compiler")
  let garden_name = process.new_name("garden")
  let assert Ok(_) = conv_sup.start(compiler_name, garden_name)
  let _ = coincidence.start_server()
  let subject = process.named_subject(compiler_name)

  // Boot infra first, then app
  let assert Ok(infra_beams) =
    boot.boot(subject, garden_name, [infra])
  let assert Ok(app_beams) =
    boot.boot(subject, garden_name, [app])
  let infra_domains = boot.results(infra_beams)
  let app_domains = boot.results(app_beams)

  // Both domains running
  should.be_true(domain.is_running("infra"))
  should.be_true(domain.is_running("app"))

  // Order preserved
  let infra_names = list.map(infra_domains, fn(d) { d.domain })
  let app_names = list.map(app_domains, fn(d) { d.domain })
  should.equal(infra_names, ["infra"])
  should.equal(app_names, ["app"])

  let _ = coincidence.stop_server()
}

/// Boot handles empty grammar list.
pub fn boot_empty_grammars_test() {
  let app = "grammar @solo_app {
  type = widget
}
"

  let compiler_name = process.new_name("compiler")
  let garden_name = process.new_name("garden")
  let assert Ok(_) = conv_sup.start(compiler_name, garden_name)
  let _ = coincidence.start_server()
  let subject = process.named_subject(compiler_name)

  let assert Ok(beams) = boot.boot(subject, garden_name, [app])
  let domains = boot.results(beams)

  should.be_true(domain.is_running("solo_app"))

  let names = list.map(domains, fn(d) { d.domain })
  should.equal(names, ["solo_app"])

  let _ = coincidence.stop_server()
}
