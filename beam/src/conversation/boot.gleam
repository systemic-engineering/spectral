//// Boot — compile grammars through a supervised @compiler.
////
//// One boot path: supervised. The caller provides the compiler subject
//// and garden name from the supervision tree. This module orchestrates
//// compilation + domain startup through them.

import conversation/compiler
import conversation/garden
import conversation/loader
import conversation/trace
import gleam/erlang/process.{type Subject}
import gleam/list
import gleam/otp/factory_supervisor
import prism_beam

/// Read a file from disk.
@external(erlang, "file_ffi", "read_file")
pub fn read_file(path: String) -> Result(String, String)

/// Result of booting a domain.
pub type BootedDomain {
  BootedDomain(
    domain: String,
    module: String,
    lenses: List(String),
    extends: List(String),
  )
}

/// Compile grammars through a supervised @compiler and start domain
/// servers through the garden factory supervisor.
/// Each compiled domain is wrapped in a Beam — the compilation trace.
pub fn boot(
  compiler_subject: Subject(compiler.Message),
  garden_name: process.Name(
    factory_supervisor.Message(String, String),
  ),
  grammars: List(String),
) -> Result(List(prism_beam.Beam(BootedDomain)), String) {
  compile_loop(compiler_subject, garden_name, grammars, [])
}

/// Read .conv files from disk, then boot.
pub fn boot_from_files(
  compiler_subject: Subject(compiler.Message),
  garden_name: process.Name(
    factory_supervisor.Message(String, String),
  ),
  paths: List(String),
) -> Result(List(prism_beam.Beam(BootedDomain)), String) {
  case read_all_files(paths, []) {
    Ok(sources) -> boot(compiler_subject, garden_name, sources)
    Error(e) -> Error(e)
  }
}

/// Extract all BootedDomains from a list of Beams.
pub fn results(beams: List(prism_beam.Beam(BootedDomain))) -> List(BootedDomain) {
  list.map(beams, fn(b) { b.result })
}

/// Check if a booted domain is alive.
pub fn is_alive(booted: BootedDomain) -> Bool {
  garden.is_running(booted.domain)
  && loader.is_loaded(booted.module)
}

/// Check if all lens imports are satisfied.
pub fn imports_resolved(domains: List(BootedDomain)) -> Bool {
  let booted_names = list.map(domains, fn(d) { d.domain })
  list.all(domains, fn(d) {
    list.all(d.lenses, fn(lens) { list.contains(booted_names, lens) })
  })
}

/// Check if all extends parents are satisfied.
pub fn extends_resolved(domains: List(BootedDomain)) -> Bool {
  let booted_names = list.map(domains, fn(d) { d.domain })
  list.all(domains, fn(d) {
    list.all(d.extends, fn(parent) { list.contains(booted_names, parent) })
  })
}

// --- Internal ---

fn compile_loop(
  compiler_subject: Subject(compiler.Message),
  garden_name: process.Name(
    factory_supervisor.Message(String, String),
  ),
  remaining: List(String),
  acc: List(prism_beam.Beam(BootedDomain)),
) -> Result(List(prism_beam.Beam(BootedDomain)), String) {
  case remaining {
    [] -> Ok(list.reverse(acc))
    [source, ..rest] -> {
      case compile_one(compiler_subject, garden_name, source) {
        Ok(beam) ->
          compile_loop(compiler_subject, garden_name, rest, [
            beam,
            ..acc
          ])
        Error(e) -> Error(e)
      }
    }
  }
}

fn compile_one(
  compiler_subject: Subject(compiler.Message),
  garden_name: process.Name(
    factory_supervisor.Message(String, String),
  ),
  source: String,
) -> Result(prism_beam.Beam(BootedDomain), String) {
  let reply = process.new_subject()
  process.send(compiler_subject, compiler.CompileGrammar(source, reply))
  case process.receive(reply, 10_000) {
    Error(_) -> Error("timeout compiling grammar")
    Ok(Error(e)) -> Error(e)
    Ok(Ok(t)) -> {
      let compiled = trace.value(t)
      let beam_module = compiled.module

      // Start domain server through garden factory supervisor
      case garden.is_running(compiled.domain) {
        True -> Nil
        False -> {
          let _ = garden.start_domain(garden_name, compiled.domain)
          Nil
        }
      }

      let lenses = case loader.get_lenses(beam_module) {
        Ok(l) -> l
        Error(_) -> []
      }
      let extends = case loader.get_extends(beam_module) {
        Ok(e) -> e
        Error(_) -> []
      }
      Ok(prism_beam.new(BootedDomain(
        domain: compiled.domain,
        module: beam_module,
        lenses: lenses,
        extends: extends,
      )))
    }
  }
}

fn read_all_files(
  paths: List(String),
  acc: List(String),
) -> Result(List(String), String) {
  case paths {
    [] -> Ok(list.reverse(acc))
    [path, ..rest] -> {
      case read_file(path) {
        Ok(contents) -> read_all_files(rest, [contents, ..acc])
        Error(e) -> Error("reading " <> path <> ": " <> e)
      }
    }
  }
}
