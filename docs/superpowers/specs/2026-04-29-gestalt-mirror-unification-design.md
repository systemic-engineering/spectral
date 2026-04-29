# Gestalt-Mirror Unification Design

**Date:** 2026-04-29
**Branch:** `mara/git-persistence-fixes`
**Status:** Design — pending implementation plan

---

## Thesis

`@gestalt` is a `.mirror` grammar. Sub-grammars (`@gestalt/document`,
`@gestalt/markdown`, `@gestalt/memory`, ...) declare the vocabularies that
the current Rust enums approximate. `DocumentKind` is `@gestalt/document`
materialized as static types before mirror could express it. The shape was
always there.

`MirrorAST` + `Gestalt` together become the rendering engine. A lens IS a
mirror grammar optic. A gestalt document IS a rendered grammar traversal.

---

## Grammar Hierarchy

```
@gestalt                     ← meta-grammar: domain, node, gestalt, annotation, lens
  @gestalt/document          ← DocumentKind declared here (replaces the Rust enum)
  @gestalt/markdown          ← encode.rs as grammar actions
  @gestalt/memory            ← spectral memory domain (observation, gap, crystal, ...)
  @gestalt/surface           ← NL translation grammar (find, where, sort, ...)
  @gestalt/git               ← git optics (blame, diff, branch, commit, ...)
```

Grammar files live in `mirror/prism/gestalt/` (parallel to `mirror/prism/ai.mirror`).
`@gestalt` inherits from `@prism` — the five operations are the vocabulary.

```
grammar @gestalt < @prism {
  type = domain | node | gestalt | line | annotation | lens

  type domain   = { grammar: ref }
  type gestalt  = { grammar: ref, head: meta[], body: node[] }
  type node     = { kind: ref, meta: meta[], children: node[] }
  type line     = { context: render-context, content: node, annotations: annotation[] }
  type annotation = { name: ref, content: gestalt }
  type lens     = { name: ref, query: string }

  action render(g: gestalt) -> output
  action linearize(g: gestalt) -> line[]
}
```

### `@gestalt/document`

Replaces `DocumentKind`. Same vocabulary, declared natively:

```
grammar @gestalt/document < @gestalt {
  type = section | paragraph | code-block | quote | callout
       | list | list-item | table | figure | separator | breath | raw

  type section    = { level: int, title: spans }
  type paragraph  = { content: spans }
  type code-block = { language: string, content: string }
  type quote      = { attribution: spans? }
  type callout    = { kind: callout-kind, title: string }
  type list       = { style: list-style, start: int }
  type table      = { columns: column-align[] }
  type figure     = { caption: spans? }
  type raw        = { content: string, format: string }

  action render(node: section) -> html { ... }
  action render(node: paragraph) -> html { ... }
}
```

`DocumentKind` in Rust becomes generated output (or is retired in favor of
operating on `MirrorKind` directly once the pipeline is in place).

### `@gestalt/memory`

The spectral memory domain — what MCP tools operate on:

```
grammar @gestalt/memory < @gestalt {
  type = observation | gap | crystal | edge | eigenboard

  type observation = { content: string, source: ref? }
  type gap         = { description: string, fiedler: float }
  type crystal     = { content: string, settled_at: timestamp }
  type edge        = { from: ref, to: ref, weight: float }
  type eigenboard  = { fiedler: float, lambda: float[], updated_at: timestamp }

  action blame(node: ref)  -> gestalt<@gestalt/git>
  action diff(from: ref, to: ref) -> gestalt<@gestalt/git>
}
```

---

## `Line<D>`: Node and Beam Unified

A line in a gestalt document is both a node (content) and a beam (pipeline
carrier). This is not a compromise — it is the correct type.

```rust
// Line<D> = Optic<RenderContext, Node<D>, Infallible, Annotations>
//
// In:   RenderContext — parent OIDs, cursor position, depth (the "source side")
// Out:  Node<D>       — the content node (unchanged by lens application)
// Loss: Annotations   — named lens results accumulate here
type Line<D> = Optic<RenderContext, Node<D>, Infallible, Annotations>;
```

**Why both?** `Beam::In` is rendering context (what came before, structurally).
`Beam::Out` is the content node. `Beam::Loss` is where annotations live —
named, content-addressed, cross-domain. The content never changes. The
annotations grow as lenses are applied.

A bare line (no lenses) has zero annotation loss. A fully-annotated line
carries a `Named<Gestalt<E>>` per applied lens. `Loss::combine` merges them.

---

## `Annotations`: Named Loss

```rust
pub struct Annotations(Vec<Named<AnyGestalt>>);

pub struct AnyGestalt {
    pub lens_oid: Oid,             // Named<optic_oid> — identity of the lens
    pub content: Box<dyn GestaltDoc>, // type-erased Gestalt<E> — any grammar
}

impl Loss for Annotations {
    fn combine(self, other: Self) -> Self {
        Annotations([self.0, other.0].concat())
    }
    fn is_zero(&self) -> bool { self.0.is_empty() }
    fn as_f64(&self) -> f64 { self.0.len() as f64 }
}
```

`Named<P>` from prism carries the label and content-addresses the whole
annotation: `hash("named:{name}:{content_oid}")`. No stringly-typed maps.

The annotation content IS a `Gestalt<E>` where `E` is ANY grammar — not
necessarily the same as the outer document's grammar. A lens applied to a
`@gestalt/memory` line that queries blame produces a `Gestalt<@gestalt/git>`
annotation. Cross-grammar is the common case, not the exception.

---

## Lens Syntax and Dispatch

An agent calls `memory_gestalt` with named lenses:

```json
{
  "lenses": {
    "blame": "find blame |> where node_oid = 'abc123' |> sort by date",
    "summary": "find observation |> near 'abc123' |> limit 3"
  }
}
```

Each key-value pair parses as `Named("blame", QueryOp("find blame |> ..."))`.
The lens name becomes the annotation key. The query is a pipe-forward mirror
expression evaluated against the line's `RenderContext`.

`Named<QueryOp>` is content-addressed: `hash("named:blame:{query_oid}")`.
Two callers writing identical lenses produce identical OIDs before execution.
The result is cacheable.

**Predefined optics** (`memory_blame`, `memory_diff`, etc.) are
`impl Operation<Line<@gestalt/memory>>` with fixed queries. They are the same
mechanism — named optics with static query strings. Registering them as
standalone MCP tools is sugar over `memory_gestalt` with a fixed lens set.

---

## `memory_gestalt` MCP Tool

Replaces and subsumes the 14 current memory tools. Each existing tool is a
named lens composition:

```
memory_blame     ↔  blame: find blame |> where node_oid = OID
memory_diff      ↔  diff: find diff |> between FROM and TO
memory_thread    ↔  thread: find note |> where topic = TOPIC |> sort by date
memory_status    ↔  status: find eigenboard |> limit 1
```

The tool definition:

```json
{
  "name": "memory_gestalt",
  "description": "Traversal<ConceptGraph, Line<@gestalt/memory>> — query the memory graph and annotate results with named lenses. Each lens is a pipe-forward mirror query. Returns a gestalt document.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query":  { "type": "string", "description": "Source query: 'find observation |> where fiedler > 0.04'" },
      "lenses": { "type": "object", "description": "Named lens map: { blame: 'find blame |> ...', summary: '...' }" }
    },
    "required": ["query"]
  }
}
```

**Execution pipeline:**

```
query → ConceptGraph traversal → Vec<Node<@gestalt/memory>>
                                        ↓
                               for each node: Line<@memory>
                                        ↓
                               for each lens: Named<QueryOp>.apply(line)
                                    → Imperfect::partial(node, Annotations::singleton(name, result))
                                        ↓
                               Gestalt<@gestalt/memory> with Annotations loss
                                        ↓
                               rendered as gestalt document (git-diff-like format)
```

**Response format** — git-diff-like, content as primary column, annotations
as named columns per lens that matched:

```
node/observation  abc123  "eigenvalue convergence in tick-B"
  @blame          commit 8b52916  reed  2026-04-28  "tick-b-phase2: update cascade tests"
  @summary        near: crystal 99f3aa  "Settlement via Laplacian diffusion"

node/gap          def456  "NL → graph query translation missing"
  @blame          commit c81eebc  reed  2026-04-27  "merge: observation-pipeline"
```

Lines with no matching annotations are clean (no annotation columns). Loss is
reported in the document head: how many nodes had no match per lens.

---

## The Rendering Engine

`MirrorAST` + `Gestalt` together are the rendering engine. Rendering is
`refract` — the fifth operation, "scatter and reconverge":

```
grammar @gestalt < @prism {
  action refract(line: line[]) -> output   ← renders the gestalt document
}
```

Current rendering backends (`encode.rs` for markdown, `dom.rs` for virtual
DOM) become grammar actions in `@gestalt/markdown` and `@gestalt/dom`:

```
grammar @gestalt/markdown < @gestalt {
  action refract(node: section)    -> string { "# " + node.title }
  action refract(node: paragraph)  -> string { node.content }
  action refract(node: code-block) -> string { "```" + node.language + "\n" + node.content + "\n```" }
}
```

`scan_grammars()` in spectral picks these up. The MCP server gets a
`refract` tool for each grammar that declares one.

---

## `DocumentKind` Migration Path

1. Write `mirror/prism/gestalt/document.mirror` — declare `@gestalt/document`
   with the same vocabulary as `DocumentKind`
2. Add `MirrorKind` as `Domain::Language` for the `MirrorDomain` impl
3. `Gestalt<Document>` and `Gestalt<MirrorDomain>` coexist during migration
4. `encode.rs` actions migrate to `@gestalt/markdown` grammar actions
5. Once the grammar pipeline is verified, `DocumentKind` Rust enum is retired

The migration is not a flag-day. Each sub-grammar migrates independently.
`@gestalt/memory` ships first (new, no Rust enum to retire). `@gestalt/document`
migrates when the render pipeline is verified correct.

---

## Dependency Structure

```
prism-core          ← zero deps. Beam, Named, Optic, Traversal, Loss.
mirror              ← depends on prism. MirrorAST, GrammarRef, Parse.
gestalt             ← depends on prism-core. Gestalt<D>, Node<D>, Line<D>, Annotations.
                      Domain trait, encode, dom, panel. Currently NO mirror dep.
spectral            ← depends on mirror + gestalt + spectral-db.
                      MCP server. scan_grammars(). memory_gestalt.
```

`@gestalt/*.mirror` files live in `mirror/prism/gestalt/` — mirror hosts the
grammars because mirror IS the grammar compiler. Gestalt the Rust crate
operates on the compiled output.

**The bridge question:** gestalt's `Domain` trait currently requires a static
`type Language`. Making it runtime-dynamic (grammar resolved by `GrammarRef`)
is a future step. For now: `MirrorKind` (a Rust enum mirroring `MirrorAST`
variants without child nodes) is `Domain::Language` for any mirror-backed
domain. The grammars drive vocabulary; the Rust enum is the generated binding.

---

## What This Is Not

- Not a rewrite. Each piece builds on what exists.
- Not a flag-day migration. Grammar files and Rust types coexist during transition.
- Not a dependency cycle. Mirror hosts grammars; gestalt operates on compiled output.

---

## Open Questions

1. **`Domain` type parameter vs runtime `GrammarRef`**: keeping `Gestalt<D>` with
   compile-time `D` vs moving to `Gestalt { grammar: GrammarRef, ... }`. The
   compile-time version preserves type safety at the cost of flexibility. The
   runtime version enables fully dynamic grammar composition. Decision deferred
   to implementation — both paths are navigable from the current design.

2. **`refract` execution model**: grammar actions in `.mirror` are currently stubs
   (the `mirror ai` pipeline). Until the Fate runtime ships, rendering actions
   are Rust implementations that mirror the grammar declarations. The grammar
   IS the spec; the Rust IS the implementation. They should match.

3. **`AnyGestalt` type erasure**: `Box<dyn GestaltDoc>` loses compile-time
   grammar type. An enum of known grammars avoids this at the cost of
   extensibility. Decision: start with type erasure, add enum if perf matters.
