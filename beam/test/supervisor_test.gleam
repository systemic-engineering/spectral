import conversation/boot
import conversation/compiler
import conversation/domain
import conversation/garden
import conversation/loader
import conversation/supervisor as conv_sup
import conversation/trace
import gleam/dynamic/decode
import gleam/erlang/process
import gleeunit/should

/// Full supervision tree starts: @compiler + garden.
pub fn supervision_tree_starts_test() {
  let compiler_name = process.new_name("compiler")
  let garden_name = process.new_name("garden")

  let assert Ok(_) = conv_sup.start(compiler_name, garden_name)

  // @compiler is reachable via named subject
  let subject = process.named_subject(compiler_name)
  let reply = process.new_subject()
  process.send(
    subject,
    compiler.CompileGrammar(
      "grammar @sup_test {\n  type = a | b\n}\n",
      reply,
    ),
  )
  let assert Ok(Ok(t)) = process.receive(reply, 5000)
  let compiled = trace.value(t)
  should.equal(compiled.domain, "sup_test")
  should.be_true(loader.is_loaded("conv_sup_test"))

  // @compiler in supervised mode does NOT start domain servers
  should.be_false(domain.is_running("sup_test"))

  // Start domain through garden
  let assert Ok(_) = garden.start_domain(garden_name, "sup_test")
  should.be_true(garden.is_running("sup_test"))

  let _ = garden.stop_domain("sup_test")
}

/// Compile grammars through supervised path and start domains via garden.
pub fn supervised_compile_and_garden_test() {
  let compiler_name = process.new_name("compiler")
  let garden_name = process.new_name("garden")

  let assert Ok(_) = conv_sup.start(compiler_name, garden_name)

  let subject = process.named_subject(compiler_name)
  let grammar = "grammar @garden_compile {
  type = module | function
  type module = atom
  type function = atom
  action exec {
    module: module
    function: function
    args: type
  }
}
"
  let assert Ok(beams) =
    boot.boot(subject, garden_name, [grammar])
  let domains = boot.results(beams)

  // Domain should be running under garden
  should.be_true(garden.is_running("garden_compile"))
  should.equal(
    domains
      |> first_domain_name,
    "garden_compile",
  )

  // exec through the domain server — apply/3 works
  let assert Ok(result) =
    domain.exec("garden_compile", "erlang", "abs", [-42])
  let assert Ok(42) = decode.run(result, decode.int)

  let _ = garden.stop_domain("garden_compile")
}

/// Garden restarts killed domain — factory supervisor fault tolerance.
pub fn garden_restarts_killed_domain_test() {
  let compiler_name = process.new_name("compiler")
  let garden_name = process.new_name("garden")

  let assert Ok(_) = conv_sup.start(compiler_name, garden_name)

  let subject = process.named_subject(compiler_name)
  let grammar = "grammar @resilient {
  type = a | b
}
"
  let assert Ok(_) = boot.boot(subject, garden_name, [grammar])
  should.be_true(garden.is_running("resilient"))

  // Kill the domain server
  domain.kill("resilient")
  process.sleep(100)

  // Garden factory supervisor should have restarted it
  should.be_true(garden.is_running("resilient"))

  let _ = garden.stop_domain("resilient")
}

/// Supervised boot from garden .conv files on disk.
pub fn supervised_boot_from_files_test() {
  let compiler_name = process.new_name("compiler")
  let garden_name = process.new_name("garden")

  let assert Ok(_) = conv_sup.start(compiler_name, garden_name)

  let subject = process.named_subject(compiler_name)
  let garden_path =
    "/Users/alexwolf/dev/systemic.engineering/garden/public"
  let assert Ok(beams) =
    boot.boot_from_files(subject, garden_name, [
      garden_path <> "/@reed/reed.conv",
      garden_path <> "/@erlang/erlang.conv",
    ])
  let domains = boot.results(beams)

  should.be_true(garden.is_running("reed"))
  should.be_true(garden.is_running("erlang"))
  should.be_true(loader.is_loaded("conv_reed"))
  should.be_true(loader.is_loaded("conv_erlang"))

  // Verify we got the right domains
  let names =
    domains
    |> domain_names
  should.be_true(list_contains(names, "reed"))
  should.be_true(list_contains(names, "erlang"))

  let _ = garden.stop_domain("reed")
  let _ = garden.stop_domain("erlang")
}

fn first_domain_name(domains: List(boot.BootedDomain)) -> String {
  case domains {
    [d, ..] -> d.domain
    [] -> ""
  }
}

fn domain_names(domains: List(boot.BootedDomain)) -> List(String) {
  case domains {
    [] -> []
    [d, ..rest] -> [d.domain, ..domain_names(rest)]
  }
}

fn list_contains(items: List(String), target: String) -> Bool {
  case items {
    [] -> False
    [x, ..rest] ->
      case x == target {
        True -> True
        False -> list_contains(rest, target)
      }
  }
}
