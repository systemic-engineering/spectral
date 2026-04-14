# gen_prism — spectral's BEAM runtime bytecode

**Status:** v0.1 — knowledge capture before mirror's BEAM EAF emission is deleted
**Date:** 2026-04-07

This document exists because BEAM EAF emission is being deleted from the mirror crate. The deletion is intentional: BEAM EAF does not belong in mirror under the current layering. mirror compiles a single `.mirror` file into a `.shatter` artifact, and BEAM emission is one of several spectral runtime backends that consume `.shatter` files for execution.

This document preserves *the pattern* the existing mirror code implements, so that spectral can rebuild it against the `.shatter` substrate when spectral grows its BEAM runtime backend. The code itself is not preserved. *"It works"* is not a property that earns code its place in a pre-1.0 compiler under eigenvalues-decide.

---

## What gen_prism is

**gen_prism is spectral's bytecode format for executing mirror compilation artifacts on the BEAM virtual machine.**

It is not a competing format with `.shatter`. It is one of several execution targets that consume `.shatter` files. Other targets may exist alongside it: native execution, replay-only loads, visualization-only loads, and any other backend spectral grows. gen_prism is the BEAM-shaped one.

The name `gen_prism` reflects what the bytecode is: a generation of executable forms (BEAM modules, gen_servers, function exports, dispatch tables) from the typed prism trajectory captured in a `.shatter` file. gen_prism is the spectral operation of *projecting* a `.shatter` trajectory into a runnable BEAM module set.

## Layering

```
mirror      compiles    .mirror source → .shatter artifact
            via         MirrorRuntime (canonical compilation target)

spectral    consumes    .shatter artifact
            via         gen_prism backend (one of N runtime backends)
            emits       BEAM bytecode (gen_prism format)

BEAM VM     loads       gen_prism bytecode
            runs        the resulting modules as gen_servers
```

`mirror` is the language tool. `spectral` is the system orchestrator. gen_prism lives in spectral because it is a runtime concern, not a language concern. mirror's job ends at `.shatter`; spectral's job begins there.

## Why BEAM

BEAM is the right runtime for the systems mirror's peers describe, because BEAM gives:

- **Hot code loading** — peers can be updated mid-flight without restart, which matches mirror's continuous-inference model.
- **Process isolation** — each peer is naturally its own actor with its own state and mailbox.
- **Distribution** — peers can run across nodes with the same code, which matches the multi-actor collaboration spectral orchestrates.
- **Supervision trees** — peer crashes are recoverable under OTP supervision, which matches the consent-and-care architecture.
- **Battle-tested concurrency** — millions of lightweight processes, decades of production hardening.
- **Pattern matching, immutability, message passing** — the language primitives BEAM provides are the primitives mirror's grammars compile to most cleanly.

mirror's optic algebra (fold / lens / iso / traversal / fold / setter / prism) maps onto BEAM's abstractions without impedance: actors hold state, receive messages, dispatch on patterns, return new state. The optic operations *are* the message handlers.

## The EAF emission pattern (preserved from mirror's compile.rs)

This section captures the knowledge of the existing mirror BEAM EAF emission, which is being deleted from the mirror crate. The pattern is preserved here so that spectral's gen_prism backend can be rebuilt against the same shape, taking input from `.shatter` files instead of from the mirror crate's resolver directly.

### Compilation chain

The pattern as it exists in mirror today:

1. **Resolve** — `Resolve` (which implements `Vector<Prism<AstNode>, Conversation<C>>`) walks the parsed AST and produces a typed `Conversation<C>` parameterized by context (`Filesystem`, `Git`, `Json`).
2. **Compile to EAF** — `compile.rs` walks the `Conversation` and produces Erlang Abstract Format bytes via the `eetf` crate.
3. **Load** — the BEAM loader loads the EAF bytes as a module.
4. **Register** — a domain server starts as a `gen_server` registered under the domain atom.

Under the new layering, steps 1-2 become:

1. **MirrorRuntime** produces a `.shatter` file from the `.mirror` source.
2. **spectral's gen_prism backend** reads the `.shatter` file, replays the inference trajectory to reconstruct the resolved program, and emits BEAM EAF bytes.
3. **Load** and **register** are unchanged on the BEAM side.

### Module naming and the conv_ prefix

Compiled grammar modules get a `conv_` prefix to avoid sticky-module collisions with BEAM built-ins. The motivating example: a grammar named `@erlang` would naively compile to a module named `erlang`, which collides with BEAM's built-in `erlang` module — and BEAM's `erlang` is a *sticky* module loaded from a sticky directory, which means the loader refuses to replace it. The `conv_` prefix solves this generally: `@erlang` compiles to `conv_erlang`. The domain server still registers under the unprefixed atom (`erlang`) for action dispatch, so the conceptual name is preserved at the message-passing layer while the module-name layer stays clean.

gen_prism inherits this pattern. The prefix may be renamed (e.g., to `gen_` or `spectral_`) to reflect spectral's ownership, but the *function* — avoiding collisions with sticky BEAM modules — must be preserved.

### Action visibility → BEAM dispatch shape

mirror grammars declare actions with visibility modifiers: `public`, `protected`, `private`. The existing EAF emission maps these to dispatch shapes:

- **public** — returns `{ok, Args}` directly. No gen_server hop. Cheapest path. Used for actions that have no shared state and no side effects beyond their arguments.
- **protected** — goes through `gen_server:call` against the domain server. The default shape. Used for actions that need access to domain state or that need to be serialized through the domain's mailbox.
- **private** — not exported from the compiled module at all. Available only to other actions within the same module. Used for internal helpers and for operations that must not be reachable from outside the domain.

A `visibility/0` function is emitted on each compiled module that returns the visibility map for the actions in that module, so callers and the model checker can introspect.

gen_prism MUST preserve this mapping. The visibility modifiers are part of the security model — not a style choice. Action visibility is one of the typed primitives in `resolve.rs` (`pub enum Visibility { Public, Protected, Private }`), and gen_prism is the projection of that type into BEAM-callable form.

### Cross-actor calls

When a grammar action invokes an action in another grammar (`@other.action(args)`), the existing pattern emits a `gen_server:call` against the registered domain atom of the other grammar. The dispatch goes through the other grammar's domain server, which routes to the corresponding action.

The helper that produces these call sequences is `emit_gen_server_call` in mirror's compile.rs. It is shared between local and cross-actor call sites. gen_prism inherits the helper's *shape* — both kinds of call go through the same emission path — but rebuilds the helper against the `.shatter` substrate.

### Test modules

mirror's compile.rs includes `emit_test_module(domain, annotate_node) → EAF bytes` for the BEAM test runner. This emits a test-shaped module that the BEAM test infrastructure can load and execute.

Under the projection-properties unification (see `2026-03-27-projection-properties-as-plans.md`), tests are being replaced by properties as the verification primitive. gen_prism MAY emit test modules during a transition, but its target shape is *property emission* — emitting BEAM-side property checkers that the model checker can run. The test module path is not preserved as a forward goal.

### Trace-aware compilation

mirror's existing compile path includes `compile_grammar_with_phases() → CompileResult { etf, parse_oid, resolve_oid, compile_oid }`. Each phase produces a content-addressed OID, and the chain forms a parent-linked trace.

gen_prism MUST preserve the trace. Every compilation is content-addressed, and every phase produces an OID that links to the prior phase. This is non-negotiable for the audit posture spectral inherits from mirror's threat model: the compilation pipeline must be replayable end-to-end and tamper-evident.

In the new layering: the `.shatter` file's content addresses chain back through the inference trajectory; gen_prism's emission adds *one more link* to that chain (the EAF OID), and the chain extends into BEAM through the loaded module's hash.

### Key data structures (preserved by name and shape)

- `Verified` — the typed, validated domain ready for emission. Output of `check::verify(domain)`.
- `Mirror` — the compiled domain (the model). `Mirror::from_grammar(grammar)` parses the AST into the model.
- `OutputNode` — the typed output AST node, with variants `Group`, `Select`, `Branch`. Each variant has its own EAF emission pattern.
- `Conversation<C>` — the resolved program parameterized by context. The phantom type `C: Setting` enforces type-level capability separation.
- `eetf::Term` — the Erlang term representation used for EAF serialization. The dependency on the `eetf` crate moves with gen_prism into spectral.

### Side dependencies that move with gen_prism

When gen_prism is built in spectral, the following inputs that mirror currently exposes must be available to spectral by some mechanism (likely as part of the `.shatter` file's content):

- The compiled `Mirror` model (or enough of the inference trajectory to reconstruct it).
- The `Verified` domain (or enough to revalidate it).
- The visibility map for each action.
- The cross-actor call graph.
- The trace OIDs from each compilation phase.

The `.shatter` file format must carry enough of this information that gen_prism can reconstruct what mirror's `Conversation<C>` would have provided. *This is the load-bearing constraint on the `.shatter` format design.*

## What gen_prism is NOT

These non-claims exist to prevent ambiguity for future readers:

- **gen_prism is NOT mirror's compilation target.** mirror compiles to `.shatter`. Anyone who reads this doc and concludes "mirror should emit gen_prism directly" is missing the layering. mirror does not know about gen_prism. spectral does.
- **gen_prism is NOT the only spectral runtime backend.** It is the BEAM-shaped one. Spectral may grow native, replay-only, visualization-only, and other backends. gen_prism is one of N.
- **gen_prism is NOT a replacement for `.shatter`.** They live at different layers. `.shatter` is the canonical compilation artifact (mirror's output). gen_prism is one execution form derived from a `.shatter` (spectral's output for one runtime target).
- **gen_prism is NOT bound by compatibility with mirror's previous BEAM EAF emission as a contract.** The pattern is inherited; the code is rebuilt. There is no migration path from the deleted code to gen_prism; the deletion is clean and the rebuild is clean.
- **gen_prism is NOT the same as the `prism` crate.** The `prism` crate is the optic algebra (Beam, Prism trait, composition operations) that mirror and fate both depend on. gen_prism is the BEAM bytecode emission. The names are intentionally similar (gen_prism *generates* the prism trajectory's BEAM projection) but the things are at different layers and must not be conflated.

## Open questions

These are questions gen_prism's implementation will need to answer. They are listed here so the questions don't get lost during the deletion-and-rebuild cycle:

1. **The `.shatter` binary format.** gen_prism's input contract depends on it. The format must carry enough of the resolved-program structure that gen_prism can reconstruct the EAF without re-running the resolver.
2. **Replay vs reconstruction.** Does gen_prism *replay* the inference trajectory in the `.shatter` (running the inference engine again at 2M decisions/second), or does it *reconstruct* the resolved program from a denormalized representation in the `.shatter`? Trade-off: replay is smaller files, slower emission; reconstruction is larger files, faster emission. *Defer to benchmark; both are viable.*
3. **Compatibility with existing BEAM-side code.** mirror's existing BEAM-side code (`conversation_beam.gleam`, `domain_server.erl`, `file_ffi.erl`, the gleam compile orchestration) consumes the EAF pattern and works today. When gen_prism is built in spectral, it should emit bytecode that the existing BEAM-side consumers can load *unchanged*. The BEAM-side code does not move; the emission code moves.
4. **Property emission.** Under the projection-properties-as-plans unification, gen_prism should eventually emit BEAM-side property checkers, not test modules. The exact emission shape for properties is not yet designed.
5. **Trace OID continuity.** gen_prism must add its emission step to the trace chain that started in mirror's `.shatter` generation. The chain must remain unbroken from `.mirror` source through `.shatter` artifact through gen_prism EAF through loaded BEAM module.

## Provenance and intent

This document was written on 2026-04-07, immediately before the existing BEAM EAF emission code in `mirror/src/compile.rs` was scheduled for deletion. The deletion happens *because* mirror's role under the current layering is to compile single `.mirror` files into `.shatter` artifacts, and BEAM emission is a downstream runtime concern that belongs to spectral.

The deletion is *without remorse*. The code does not deserve a place in the mirror crate. *"It works"* was the substrate-shaped framing that almost preserved the code in place — that pattern produces ambiguity for future readers trying to figure out which compilation target is canonical. The cut is clean precisely so that future readers do not have to wonder.

This document preserves the *pattern*. The code does not return. When spectral's gen_prism backend is built, it is built fresh, against the `.shatter` substrate, against this document. The rebuild is the rebuild; it is not a port.

The pattern moves. The bytes don't.
