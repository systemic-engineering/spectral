import conversation/compiler
import conversation/key
import conversation/oid
import conversation/trace
import gleam/erlang/process
import gleam/option
import gleeunit/should

pub fn compile_grammar_returns_trace_test() {
  let assert Ok(started) = compiler.start()
  let subject = started.data
  let reply = process.new_subject()
  process.send(
    subject,
    compiler.CompileGrammar("grammar @test_compile {\n  type = a | b\n}\n", reply),
  )
  let assert Ok(Ok(t)) = process.receive(reply, 5000)
  case trace.value(t) {
    compiler.CompiledDomain(domain: "test_compile", ..) -> should.be_true(True)
    _ -> should.be_true(False)
  }
  process.send(subject, compiler.Shutdown)
}

pub fn compile_grammar_loads_module_test() {
  let assert Ok(started) = compiler.start()
  let subject = started.data
  let reply = process.new_subject()
  process.send(
    subject,
    compiler.CompileGrammar("grammar @test_loaded {\n  type = x | y\n}\n", reply),
  )
  let assert Ok(Ok(t)) = process.receive(reply, 5000)
  let compiled = trace.value(t)
  // Module name should be conv_test_loaded (conv_ prefix)
  should.equal(compiled.module, "conv_test_loaded")
  process.send(subject, compiler.Shutdown)
}

pub fn trace_is_verifiable_test() {
  let assert Ok(started) = compiler.start()
  let subject = started.data
  let reply = process.new_subject()
  process.send(
    subject,
    compiler.CompileGrammar("grammar @test_verify {\n  type = p | q\n}\n", reply),
  )
  let assert Ok(Ok(t)) = process.receive(reply, 5000)
  trace.verify(t, compiler.public_key()) |> should.be_true()
  process.send(subject, compiler.Shutdown)
}

pub fn compile_grammar_error_test() {
  let assert Ok(started) = compiler.start()
  let subject = started.data
  let reply = process.new_subject()
  // Source with no grammar block should error
  process.send(
    subject,
    compiler.CompileGrammar("template $t {\n  slug\n}\n", reply),
  )
  let assert Ok(Error(_msg)) = process.receive(reply, 5000)
  should.be_true(True)
  process.send(subject, compiler.Shutdown)
}

pub fn trace_has_parent_chain_test() {
  let assert Ok(started) = compiler.start()
  let subject = started.data
  let reply = process.new_subject()
  process.send(
    subject,
    compiler.CompileGrammar(
      "grammar @test_chain {\n  type = x | y\n}\n",
      reply,
    ),
  )
  let assert Ok(Ok(t)) = process.receive(reply, 5000)
  // The final trace should have a parent (compile phase trace OID)
  case t.parent {
    option.Some(_parent_oid) -> should.be_true(True)
    option.None -> should.be_true(False)
  }
  process.send(subject, compiler.Shutdown)
}

pub fn trace_source_oid_deterministic_test() {
  let assert Ok(started) = compiler.start()
  let subject = started.data
  let source = "grammar @test_det {\n  type = a | b\n}\n"

  let reply1 = process.new_subject()
  process.send(subject, compiler.CompileGrammar(source, reply1))
  let assert Ok(Ok(t1)) = process.receive(reply1, 5000)

  let reply2 = process.new_subject()
  process.send(subject, compiler.CompileGrammar(source, reply2))
  let assert Ok(Ok(t2)) = process.receive(reply2, 5000)

  // Same source → same source_oid in compiled domain
  let v1 = trace.value(t1)
  let v2 = trace.value(t2)
  should.equal(v1.source_oid, v2.source_oid)
  // Source OID matches direct computation
  should.equal(v1.source_oid, oid.from_bytes(<<source:utf8>>))

  process.send(subject, compiler.Shutdown)
}

pub fn hierarchical_compiler_trace_verifiable_test() {
  let root = key.generate()
  let root_pub = key.public_key(root)
  let assert Ok(started) = compiler.start_with_root(root_pub)
  let subject = started.data
  let reply = process.new_subject()
  process.send(
    subject,
    compiler.CompileGrammar(
      "grammar @test_hier {\n  type = a | b\n}\n",
      reply,
    ),
  )
  let assert Ok(Ok(t)) = process.receive(reply, 5000)
  // Trace should verify against the hierarchically derived public key
  let derived_pub = compiler.public_key_from(root_pub)
  trace.verify(t, derived_pub) |> should.be_true()
  // Should NOT verify against the flat-derived key
  trace.verify(t, compiler.public_key()) |> should.be_false()
  process.send(subject, compiler.Shutdown)
}
