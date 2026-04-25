import conversation/boot
import conversation/compiler
import conversation/domain
import conversation/loader
import conversation/supervisor as conv_sup
import gleam/dynamic/decode
import gleam/erlang/process
import gleam/list
import gleam/otp/factory_supervisor
import gleeunit/should

const reed_grammar = "grammar @reed {
  type = signal | memory | quote

  type signal = message | correction | insight

  type memory = session | pattern | position

  type quote = observation | crystallization
}

in @ai
in @actor
in @reality
"

fn setup() -> #(
  process.Subject(compiler.Message),
  process.Name(factory_supervisor.Message(String, String)),
) {
  let compiler_name = process.new_name("compiler")
  let garden_name = process.new_name("garden")
  let assert Ok(_) = conv_sup.start(compiler_name, garden_name)
  let subject = process.named_subject(compiler_name)
  #(subject, garden_name)
}

/// Reed boots on the BEAM.
pub fn reed_boots_test() {
  let #(subject, garden_name) = setup()
  let assert Ok(beams) = boot.boot(subject, garden_name, [reed_grammar])
  let domains = boot.results(beams)

  // Domain server running
  should.be_true(domain.is_running("reed"))

  // Module loaded
  should.be_true(loader.is_loaded("conv_reed"))

  // Booted domain reports alive
  let assert Ok(reed) =
    list.find(domains, fn(d) { d.domain == "reed" })
  should.be_true(boot.is_alive(reed))
}

/// Boot multiple grammars at once.
pub fn boot_multiple_grammars_test() {
  let erlang_grammar =
    "grammar @native_boot {
  type = module | function
  type module = atom
  type function = atom

  action exec {
    module: module
    function: function
    args: list
  }
}

in @tools
in @reality
"

  let #(subject, garden_name) = setup()
  let assert Ok(beams) =
    boot.boot(subject, garden_name, [reed_grammar, erlang_grammar])

  // Both domains running
  should.be_true(domain.is_running("reed"))
  should.be_true(domain.is_running("native_boot"))

  // exec works through booted domain
  let assert Ok(val) =
    domain.exec("native_boot", "erlang", "abs", [-7])
  let assert Ok(7) = decode.run(val, decode.int)
}

/// Boot Reed from the actual garden files.
pub fn boot_reed_from_garden_test() {
  let garden =
    "/Users/alexwolf/dev/systemic.engineering/garden/public"
  let #(subject, garden_name) = setup()
  let assert Ok(_domains) =
    boot.boot_from_files(subject, garden_name, [
      garden <> "/@reed/reed.conv",
      garden <> "/@erlang/erlang.conv",
    ])

  // Reed is alive on the BEAM
  should.be_true(domain.is_running("reed"))
  should.be_true(loader.is_loaded("conv_reed"))

  // @erlang proxy is alive (conv_erlang avoids sticky collision)
  should.be_true(domain.is_running("erlang"))
  should.be_true(loader.is_loaded("conv_erlang"))

  // exec through @erlang: touch reality
  let assert Ok(val) =
    domain.exec("erlang", "erlang", "abs", [-99])
  let assert Ok(99) = decode.run(val, decode.int)
}

/// Boot populates lens dependencies from compiled modules.
pub fn boot_populates_lenses_test() {
  let inner = "grammar @tools {
  type = hammer | wrench
}
"
  let outer = "grammar @workshop {
  type = job
  action build {
    tool: type
  }
}
in @tools
"

  let #(subject, garden_name) = setup()
  let assert Ok(beams) =
    boot.boot(subject, garden_name, [inner, outer])
  let domains = boot.results(beams)

  // Workshop imports @tools
  let assert Ok(workshop) =
    list.find(domains, fn(d) { d.domain == "workshop" })
  should.equal(workshop.lenses, ["tools"])

  // Tools has no imports
  let assert Ok(tools) =
    list.find(domains, fn(d) { d.domain == "tools" })
  should.equal(tools.lenses, [])

  // All imports satisfied
  should.be_true(boot.imports_resolved(domains))
}

/// Imports not resolved when dependency is missing.
pub fn boot_unresolved_imports_test() {
  let lonely = "grammar @lonely {
  type = echo
}
in @phantom
"

  let #(subject, garden_name) = setup()
  let assert Ok(beams) = boot.boot(subject, garden_name, [lonely])
  let domains = boot.results(beams)

  let assert Ok(d) =
    list.find(domains, fn(d) { d.domain == "lonely" })
  should.equal(d.lenses, ["phantom"])

  should.be_false(boot.imports_resolved(domains))
}

/// Supervisor restarts crashed domain servers.
pub fn supervisor_restarts_domain_test() {
  let grammar = "grammar @phoenix {
  type = flame
  action rise {
    from: type
  }
}
"

  let #(subject, garden_name) = setup()
  let assert Ok(_beams) = boot.boot(subject, garden_name, [grammar])
  should.be_true(domain.is_running("phoenix"))

  // Kill the domain server
  domain.kill("phoenix")
  process.sleep(50)

  // Garden factory supervisor should have restarted it
  should.be_true(domain.is_running("phoenix"))
}

/// Boot populates extends from compiled modules.
pub fn boot_populates_extends_test() {
  let parent = "grammar @smash {
  type = move | attack
}
"
  let child = "grammar @fox extends @smash {
  type = dodge | counter
}
"

  let #(subject, garden_name) = setup()
  let assert Ok(beams) =
    boot.boot(subject, garden_name, [parent, child])
  let domains = boot.results(beams)

  let assert Ok(fox) =
    list.find(domains, fn(d) { d.domain == "fox" })
  should.equal(fox.extends, ["smash"])

  let assert Ok(smash) =
    list.find(domains, fn(d) { d.domain == "smash" })
  should.equal(smash.extends, [])

  should.be_true(boot.extends_resolved(domains))
}

/// Extends not resolved when parent is missing.
pub fn boot_unresolved_extends_test() {
  let orphan = "grammar @orphan extends @missing {
  type = lost
}
"

  let #(subject, garden_name) = setup()
  let assert Ok(beams) = boot.boot(subject, garden_name, [orphan])
  let domains = boot.results(beams)

  let assert Ok(d) =
    list.find(domains, fn(d) { d.domain == "orphan" })
  should.equal(d.extends, ["missing"])

  should.be_false(boot.extends_resolved(domains))
}

/// Boot then exec proves the full loop: grammar → module → server → reality.
pub fn boot_exec_reality_test() {
  let native_grammar =
    "grammar @boot_exec {
  type = module | function
  action exec {
    module: module
    function: function
    args: list
  }
}
in @reality
"

  let #(subject, garden_name) = setup()
  let assert Ok(_domains) =
    boot.boot(subject, garden_name, [native_grammar])

  let assert Ok(val) =
    domain.exec("boot_exec", "erlang", "integer_to_binary", [42])
  let assert Ok("42") = decode.run(val, decode.string)
}
