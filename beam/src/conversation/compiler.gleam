//// Compiler — @compiler actor.
////
//// The @compiler receives .conv source, compiles the grammar block via
//// the Rust NIF, loads the compiled module onto the BEAM, and returns a
//// witnessed Trace(CompiledDomain).
////
//// Identity is deterministic: sha512("compiler") → Ed25519 keypair.
////
//// Two start modes:
//// - start()       — imperative path. Starts domain supervisor, manages
////                    domain server lifecycle on compile. Backwards compatible.
//// - start_named() — supervised path. Pure compilation only. The garden
////                    factory supervisor handles domain server lifecycle.

import conversation/coincidence
import conversation/domain
import conversation/grammar
import conversation/key
import conversation/loader
import conversation/nif
import conversation/oid
import conversation/ref.{type ScopedOid}
import conversation/trace.{type Trace}
import gleam/erlang/process.{type Subject}
import gleam/list
import gleam/option.{None}
import gleam/otp/actor

/// A compiled domain grammar.
pub type CompiledDomain {
  CompiledDomain(domain: String, source_oid: oid.Oid, module: String)
}

/// Compilation phase for traced chain.
pub type Phase {
  ParsePhase(phase_oid: oid.Oid)
  ResolvePhase(phase_oid: oid.Oid)
  CompilePhase(phase_oid: oid.Oid)
}

/// Messages the @compiler actor accepts.
pub type Message {
  CompileGrammar(
    source: String,
    reply: Subject(Result(Trace(CompiledDomain), String)),
  )
  Shutdown
}

type State {
  State(
    kp: key.KeyPair,
    actor_oid: ScopedOid(key.Key),
    manage_domains: Bool,
  )
}

/// The @compiler actor's deterministic public key (flat derivation).
pub fn public_key() -> key.Key {
  key.from_seed(domain_seed(<<"compiler":utf8>>))
  |> key.public_key
}

/// The @compiler actor's public key derived from a root key (hierarchical).
pub fn public_key_from(root: key.Key) -> key.Key {
  key.derive_child(root, "compiler")
  |> key.public_key
}

/// Start the @compiler actor (imperative path, flat derivation).
/// Starts the domain supervisor and manages domain server lifecycle
/// on each compile. Use this for backwards compatibility with the
/// existing boot path.
pub fn start() -> actor.StartResult(Subject(Message)) {
  let _ = domain.start_supervisor()
  let kp = key.from_seed(domain_seed(<<"compiler":utf8>>))
  do_start(kp, True)
}

/// Start the @compiler actor with hierarchical key derivation.
/// Derives the compiler's identity from the root key.
pub fn start_with_root(root: key.Key) -> actor.StartResult(Subject(Message)) {
  let _ = domain.start_supervisor()
  let kp = key.derive_child(root, "compiler")
  do_start(kp, True)
}

/// Start the @compiler actor with a registered name (supervised path, flat).
/// Does NOT start a domain supervisor or manage domain servers.
/// Pure compilation: grammar → NIF → ETF → BEAM module → trace.
/// The garden factory supervisor handles domain server lifecycle.
pub fn start_named(
  name: process.Name(Message),
) -> actor.StartResult(Subject(Message)) {
  let kp = key.from_seed(domain_seed(<<"compiler":utf8>>))
  do_start_named(kp, name)
}

/// Start named with hierarchical key derivation.
pub fn start_named_with_root(
  name: process.Name(Message),
  root: key.Key,
) -> actor.StartResult(Subject(Message)) {
  let kp = key.derive_child(root, "compiler")
  do_start_named(kp, name)
}

fn do_start(
  kp: key.KeyPair,
  manage_domains: Bool,
) -> actor.StartResult(Subject(Message)) {
  let actor_oid = key.oid(key.public_key(kp))
  let state =
    State(kp: kp, actor_oid: actor_oid, manage_domains: manage_domains)
  actor.new(state)
  |> actor.on_message(handle_message)
  |> actor.start
}

fn do_start_named(
  kp: key.KeyPair,
  name: process.Name(Message),
) -> actor.StartResult(Subject(Message)) {
  let actor_oid = key.oid(key.public_key(kp))
  let state = State(kp: kp, actor_oid: actor_oid, manage_domains: False)
  actor.new(state)
  |> actor.on_message(handle_message)
  |> actor.named(name)
  |> actor.start
}

fn handle_message(state: State, msg: Message) -> actor.Next(State, Message) {
  case msg {
    CompileGrammar(source, reply) -> {
      let source_oid = oid.from_bytes(<<source:utf8>>)
      case nif.compile_grammar_traced(source) {
        Ok(#(etf, parse_oid_str, resolve_oid_str, compile_oid_str)) -> {
          let domain_name = case grammar.from_source(source) {
            Ok(g) -> grammar.domain(g)
            Error(_) -> "unknown"
          }
          case loader.load_etf_module(etf) {
            Ok(module) -> {
              // Only manage domain servers in imperative mode.
              // In supervised mode, the garden handles this.
              let domain_was_started = case state.manage_domains {
                True ->
                  case domain.is_running(domain_name) {
                    False -> {
                      let _ = domain.start_supervised(domain_name)
                      True
                    }
                    True -> False
                  }
                False -> False
              }

              // Build traced compilation chain: parse → resolve → compile → swap
              let parse_trace =
                trace.new(
                  state.actor_oid,
                  state.kp,
                  ParsePhase(oid.from_string(parse_oid_str)),
                  None,
                )
              let resolve_trace =
                trace.new(
                  state.actor_oid,
                  state.kp,
                  ResolvePhase(oid.from_string(resolve_oid_str)),
                  option.Some(trace.oid(parse_trace)),
                )
              let compile_trace =
                trace.new(
                  state.actor_oid,
                  state.kp,
                  CompilePhase(oid.from_string(compile_oid_str)),
                  option.Some(trace.oid(resolve_trace)),
                )

              // Enforce property declarations through @coincidence.
              // On failure: stop the domain server (if we started it) and
              // purge the loaded module so no half-loaded state remains.
              let enforcement_result = case check_requires(module, source) {
                Error(reason) -> Error("property enforcement: " <> reason)
                Ok(Nil) ->
                  case check_invariants(module, source) {
                    Error(reason) -> Error("property enforcement: " <> reason)
                    Ok(Nil) -> Ok(Nil)
                  }
              }

              case enforcement_result {
                Error(reason) -> {
                  // Clean up: stop domain server if we started it, purge module.
                  case domain_was_started {
                    True -> {
                      let _ = domain.stop(domain_name)
                      Nil
                    }
                    False -> Nil
                  }
                  let _ = loader.purge_module(module)
                  process.send(reply, Error(reason))
                }
                Ok(Nil) -> {
                  let compiled =
                    CompiledDomain(
                      domain: domain_name,
                      source_oid: source_oid,
                      module: module,
                    )
                  let t =
                    trace.new(
                      state.actor_oid,
                      state.kp,
                      compiled,
                      option.Some(trace.oid(compile_trace)),
                    )
                  process.send(reply, Ok(t))
                }
              }
            }
            Error(e) -> process.send(reply, Error(e))
          }
        }
        Error(e) -> process.send(reply, Error(e))
      }
      actor.continue(state)
    }
    Shutdown -> actor.stop()
  }
}

/// SHA-512 hash, first 32 bytes — Ed25519 seed for the cairn pattern.
fn domain_seed(name: BitArray) -> BitArray {
  let assert <<seed:bytes-size(32), _rest:bytes>> = do_sha512(name)
  seed
}

@external(erlang, "crypto_ffi", "sha512")
fn do_sha512(data: BitArray) -> BitArray

/// Check required properties through @coincidence.
/// Returns Error if any required property fails.
fn check_requires(
  beam_module: String,
  source: String,
) -> Result(Nil, String) {
  case loader.get_requires(beam_module) {
    Ok(requires) -> {
      let failures =
        list.filter_map(requires, fn(name) {
          case coincidence.check_property(source, name) {
            Ok(_) -> Error(Nil)
            Error(reason) ->
              Ok("required property '" <> name <> "' failed: " <> reason)
          }
        })
      case failures {
        [] -> Ok(Nil)
        [first, ..] -> Error(first)
      }
    }
    Error(_) -> Ok(Nil)
  }
}

/// Check invariant properties through @coincidence.
/// Returns Error if any invariant property fails.
fn check_invariants(
  beam_module: String,
  source: String,
) -> Result(Nil, String) {
  case loader.get_invariants(beam_module) {
    Ok(invariants) -> {
      let failures =
        list.filter_map(invariants, fn(name) {
          case coincidence.check_property(source, name) {
            Ok(_) -> Error(Nil)
            Error(reason) ->
              Ok("invariant '" <> name <> "' failed: " <> reason)
          }
        })
      case failures {
        [] -> Ok(Nil)
        [first, ..] -> Error(first)
      }
    }
    Error(_) -> Ok(Nil)
  }
}
