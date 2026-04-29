# Mirror NL Interface — Mapping to Spectral Needs

> Mirror is gcc. Spectral is make. The NL interface spectral needs is already
> most of the way built — in the wrong repo.

**Date:** 2026-04-29
**Branch:** `mara/git-persistence-fixes`
**Cross-ref:** `docs/memory-layer-analysis.md` — Gap 2 (NL → graph query)

---

## What Mirror Actually Is

Mirror is not primarily an NL system. It is a self-hosting compiler for a
typed domain grammar language. But embedded in it are exactly the primitives
spectral needs for its natural language interface. They are:

### 1. NL Tokenizer (`src/nl/`)

A full offline pipeline: UAX #29 word boundary segmentation → stop word filter
→ Porter2 stemming → compound decomposition → `Prism<Token>` tree.

Key properties:
- **Content-addressed tokens.** `Token::content_oid()` = `sha("token:{stem}")`.
  Two documents that share the stem `eigenvalu` produce identical OIDs at that
  position. Shared OIDs = shared meaning (in the structural/lexical sense).
- **Compound-aware.** `approx_lambda_2` decomposes to `[approx, lambda, 2]`.
  Each part shares its OID with standalone occurrences. "lambda" inside a
  compound shares an OID with "lambda" as a standalone word.
- **`shared_oid_count(a, b)`** — counts content addresses in common between
  two token trees. This is BM25 after stemming, without an external library,
  at zero network cost.
- **Zero deps.** `rust-stemmers` + `unicode-segmentation`. Fully offline.
  Deterministic. No embedding model.

### 2. Grammar Language (`.mirror`)

Typed domain declarations:
```
grammar @surface {
  type = intent | transform | terminal
  type intent = find | near | hot
  type transform = where | walk | sort | limit
  type terminal = count | loss

  action translate(text: string) <= Query
}
```

The grammar compiler (`mirror compile`) produces a content-addressed
`MirrorFragment` (a `Fractal<MirrorData, CoincidenceHash<5>>`). The grammar
defines the vocabulary. The action defines the dispatch.

### 3. Grammar-as-MCP Auto-Registration

Spectral's `scan_grammars()` already reads `.mirror` files in the
project and auto-registers every `action-def` as an MCP tool. A `.mirror` file
in the spectral project root IS a new MCP tool surface — no code change
required. `@surface__translate` becomes an MCP tool the moment `surface.mirror`
exists.

### 4. Eigentest (Grammar Validation)

`src/eigentest.rs` — 8-test Laplacian battery on grammar type graphs. Detects
star topology (one type mediates all connections = fragile grammar). The
Surface grammar's type graph will be checked against this automatically.

### 5. Five Fate Models (Stubbed)

`mirror ai <model> <file>` — reads a `.mirror` form, extracts spectral features
from its type graph, runs `FateRuntime::select`. Pipeline:

```
mirror ai abyss form.mirror | mirror ai explorer - | mirror ai fate -
```

The chain is the program. Currently echoes model names (stub). When
implemented: each invocation selects the next model via bit-for-bit
deterministic inference (425 parameters, no network). The Surface model (language
→ query) maps to the `explorer` fate model in the 5-model stack.

### 6. Query Mode (`cmd_query`)

Currently: catches unknown commands, runs them through `ASTPrism.focus().project()`,
returns a parsed AST. This is the placeholder for "form-as-operation query mode"
(README: TBD). It already routes any unrecognized input into the parse pipeline
— the plumbing for "this string is a query, not a command" is there.

---

## Mapping Mirror to Spectral's Gaps

### Gap 2: Natural Language → Graph Query

The analysis in `memory-layer-analysis.md` proposed a 3-week Surface model
(an LLM call translating NL into pipe-forward queries). Mirror changes the
estimate.

**What mirror provides:**

| Step | Mirror component | Status |
|---|---|---|
| NL text → stems | `nl::tokenize()` | **Shipped** |
| Stems → OID overlap | `shared_oid_count()` | **Shipped** |
| Query vocabulary declaration | `.mirror` grammar language | **Shipped** |
| Grammar → MCP tool | `scan_grammars()` auto-registration | **Shipped** |
| Grammar validation | `eigentest` battery | **Shipped** |
| NL → compiled query (execution) | `@surface__translate` action | **Missing** |

**The missing piece is one function.** Everything else is already there. The
`translate` action needs to:

1. Call `nl::tokenize(text)` to get the stem tree
2. Match stems against spectral's known vocabulary: node types (`observation`,
   `gap`, `crystal`), pipe-forward keywords (`find`, `where`, `sort`, `limit`,
   `count`), field names (`fiedler`, `updated_at`, `node_type`)
3. Construct a pipe-forward query string
4. Return it (the MCP caller then passes it to `graph_query`)

This is not an LLM call. It's a match table on content-addressed stems. The
OID for the stem `find` is stable and deterministic — if the input contains it,
the match fires.

**Revised effort: ~1 week, not 3.**

The scope: write `surface.mirror` (grammar declaration) + implement the `translate`
Rust function + wire into the MCP dispatch. The Fate model machinery (when it
ships) will provide a more sophisticated version — but the structural version
works for spectral's technical vocabulary now.

**What it won't cover:**
- Semantic queries across synonym boundaries ("car" ≠ "automobile"). This still
  requires embeddings (Gap 1 work) or the Fate inference engine.
- Multi-hop reasoning ("find everything related to what Alex was working on last
  week"). This requires temporal edge support (Gap 3) + graph walk, not just
  keyword matching.

For spectral's primary use case — technical/code vocabulary, queries by node
type, field conditions, structural relationships — the mirror NL tokenizer covers
the common case without embeddings.

### Gap 1: Semantic Retrieval

Mirror's tokenizer covers the **structural/lexical** similarity half:

- "eigenvalue" and "eigenvalu" → same OID (stemming handles inflection)
- "approx_lambda_2" and "lambda" → shared OID in compound tree (compound
  decomposition handles concatenated terms)
- "SpectralIndex" and "spectral" + "index" → shared OIDs (CamelCase decomposition)

This is BM25-quality retrieval for free, offline, deterministic. For spectral's
technical vocabulary it handles ~80% of the relevant query surface.

The **semantic** half (conceptually related but lexically distant terms —
"Fiedler vector" and "algebraic connectivity", "settlement" and "crystallization")
still requires embeddings. The `.vec` blob approach from the analysis still
applies for that case.

**But the order of operations changes.** Start with mirror's tokenizer as the
retrieval layer (zero cost, offline, ships today). Add embeddings later for the
semantic edge cases. Don't invert the order by starting with embeddings.

### Gaps 3 and 4

Mirror doesn't directly address temporal edges or entity extraction — these are
spectral-db concerns. But the grammar system is relevant:

- Temporal edges could be declared as a grammar (`grammar @temporal { type =
  edge; action stamp(oid: string, valid_until: timestamp) }`) and
  auto-registered as an MCP tool.
- Entity extraction from prose could be piped through mirror's NL tokenizer
  first (free stems) before the optional LLM extraction step — reducing the
  cost of the LLM call by giving it pre-stemmed, de-stopped input.

---

## The Integration Architecture

```
User query (NL)
      │
      ▼
mirror nl::tokenize()            ← already exists in mirror
[UAX#29 → stop filter → stem → compound tree]
      │
      ▼
@surface__translate              ← to be built: surface.mirror + translate fn
[stem OIDs → pipe-forward query string]
      │
      ▼
spectral graph_query             ← already exists
[pipe-forward → ConceptGraph traversal → ShannonLoss result]
      │
      ▼
Results + OIDs
[agent uses OIDs for memory_recall, memory_store, etc.]
```

This pipeline requires no LLM call for the translation step. It uses
content-addressed token overlap as the matching mechanism. The only external
call in the whole pipeline is the optional embedding for semantic gap coverage.

The pipeline is also **observable**: every step produces a loss metric.
`nl::tokenize` reports which tokens survived filtering. `@surface__translate`
can report which query terms it matched and which it couldn't map.
`graph_query` reports ShannonLoss on every result. The system knows what it
doesn't know at every step.

---

## What Needs to Be Built

### 1. `surface.mirror` in spectral project root

```
in @actor

grammar @surface {
  type = intent | transform | terminal | field | value

  type intent = find | near | hot
  type transform = where | walk | sort | limit
  type terminal = count | loss
  type field = node_type | fiedler | updated_at | content | kind

  action translate(content)
}
```

When spectral boots the MCP server, `scan_grammars()` picks this up and
registers `surface__translate` as an MCP tool automatically.

### 2. `fn translate(text: &str) -> String` in spectral

Takes NL text, calls `mirror::nl::tokenize()`, maps stems to spectral vocabulary,
returns a pipe-forward query string. Spectral already depends on mirror
(`mirror = { path = "../mirror" }` in Cargo.toml). The NL module is available.

The initial implementation is a match table: known stem OIDs → query tokens.
Future: Fate model inference for unknowns.

### 3. Wire to MCP dispatch

Add `MemoryMsg::Translate { text: String }` + dispatch through `translate()`,
pass result to `graph_query`. The agent can then call `surface__translate` with
natural language and get back graph results.

---

## Mirror Dependency Check

Spectral's `Cargo.toml` already has mirror as a dependency (used for
`Parse.trace()` in `scan_grammars`). The `mirror::nl` module is available
today. No new dependency needed.

```toml
# spectral/Cargo.toml (existing)
mirror = { path = "../mirror" }
```

`mirror::nl::tokenize` and `mirror::nl::shared_oid_count` are pub exports.
They can be called directly from spectral's MCP server code.

---

## Revised 6-Week Bet (from `memory-layer-analysis.md`)

| Week | Original plan | Revised plan |
|---|---|---|
| 1 | Auto-recall on session start | Auto-recall on session start *(unchanged)* |
| 2–3 | Semantic entry points (embeddings) | **Mirror NL tokenizer + `surface.mirror`** — structural retrieval first, ~1 week |
| 4 | Temporal edges | Temporal edges *(unchanged)* |
| 5–6 | Surface model (NL → query, LLM) | **Embeddings for semantic gap** — layer on top of structural retrieval |

The revised plan ships a working NL interface in week 2 (not week 5–6) by using
what mirror already built. The LLM-based semantic layer becomes an enhancement,
not a prerequisite.

---

## What This Changes About the Position

After the 6-week bet, spectral has:

- **NL → graph query** via mirror tokenizer (structural, zero-cost, offline)
- **NL → graph query** via optional embedding (semantic, for edge cases)
- **Temporal edges** for Zep-style time-travel queries
- **Auto-recall** for cross-session continuity

No other system in the landscape has the structural retrieval layer — the
content-addressed OID overlap is unique to the mirror/spectral stack. The
semantic layer (embeddings) is table stakes, now covered. But the structural
layer is a moat: deterministic, offline, zero-cost, reproducible across
sessions, reproducible across machines.

The combination: vector similarity finds the neighborhood; Laplacian diffusion
expands structurally; mirror tokenizer matches technical vocabulary without
embeddings; git history provides the timeline. No single competitor has more
than two of these. Spectral has all four.

---

## References

- `src/nl/mod.rs` — mirror's NL tokenizer (UAX#29 + stemmer + compound tree)
- `src/nl/token.rs` — `Token`, `content_oid()`, stem-to-OID mapping
- `src/eigentest.rs` — grammar validation battery (star topology detection)
- `src/cli.rs:1583` — `cmd_query()` — the TBD query mode placeholder
- `src/cli.rs:542` — `cmd_ai()` — the fate model stub
- `mirror/prism/ai.mirror` — example grammar declaring AI action vocabulary
- `docs/specs/compiler-surface-plan.md` — 6-phase compiler surface plan
- `docs/memory-layer-analysis.md` — Gap 2 original estimate (3 weeks, LLM)
- `src/sel/mcp/tools.rs` — spectral's `scan_grammars()` bridge
- `Cargo.toml` — spectral already depends on mirror
