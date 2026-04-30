# Gestalt as the Lingua Franca

**Fragment Trees, Optic CLI, and the Path to Convergence**

Author: Glint
Date: 2026-04-30
Status: Spec — post-session crystallization
Prior: `2026-04-29-gestalt-mirror-unification-design.md`

---

## 1. The Insight

Here is what became clear.

`Line<D>` is correct and downstream. It is a 2D projection of a higher-dimensional
structure. The implementation in `crates/gestalt/src/line.rs` is honest code — it
carries a `RenderContext`, a `Node<D>`, and `Annotations` as loss. It knows what it
is. But what it is, is the output of a linearization step, not the unit of structure.

The unit of structure is `Fragment<D>`.

A Fragment is a `Node<D>` plus sub-line annotations at arbitrary granularity, stored
as a node in a MerkleTree. Every Fragment has an OID. The tree of Fragments IS the
gestalt. Not a rendering of it. Not a projection. The thing itself.

```
ConceptGraph (N-dimensional, spectral-db)
      |
      | refract
      v
Fragment<D> tree     <- MerkleTree. Every node is OID.
      |
      | linearize
      v
Line<D>[]            <- 2D projection. What we have now. Correct but late.
      |
      | render
      v
byte stream          <- terminal, context window, websocket, whatever
```

This is one insight, not four. The Fragment, the optic CLI, the on-demand
annotations, and the NL/LLM division are all consequences of the same recognition:
**the tree is the storage layer, not the rendering layer.**

When I say tree here I mean MerkleTree from prism-core. The trait is clean: `data()`,
`children()`, `oid()`. Same content plus same children equals same OID. Always. The
diff algorithm is O(delta), not O(n). Identical subtrees are skipped entirely. This
is the property that makes everything downstream work.

Once the tree is the storage layer, the five operations stop being CLI command names
that map to conceptual metaphors. They become literal optics over Fragment trees:

- `focus` selects a subtree. Input OID, output subtree.
- `project` filters by predicate. `where kind = fn`. Removes nodes that don't match.
- `split` explores adjacency. What's connected to this Fragment? Through which edges?
- `zoom` applies a lens. Blame, diff, type info — computed annotations, not stored.
- `refract` renders. Fragment tree to byte stream. The terminal is one backend. A
  context window is another. A websocket is a third.

Pipeline: `spectral focus src/lib.rs |> project where kind = fn |> zoom with blame |> refract as markdown`.

That's not a wish. That's a type-checked composition of operations over a content-
addressed tree. Each step takes a `Fragment<D>` tree and returns a `Fragment<D>` tree
(possibly annotated, possibly filtered, possibly projected to a different grammar).

The insight that matters most — the one I want to name clearly, because it resolves
the CPU bug at the architectural level instead of the tuning level — is about
annotations.

**Annotations are not stored in the tree. They are on-demand computed.**

`hash(lens_oid, input_tree_oid) -> output_tree_oid`

Same input plus same lens equals same output. Forever. Cacheable by OID pair. The
cascade does not need to pre-compute every annotation on a 5-second timer. It needs
to maintain the tree. The annotations are computed when someone asks for them.

This is the difference between the current architecture (cascade re-ingests every node
every 5 seconds, marks nodes dirty, recomputes Laplacian eigenvalues for the dirty
nodes, writes a git commit even when nothing changed — 80% CPU, 2.47 GB RAM after
6 hours idle) and the target architecture (cascade maintains the tree; annotations
are functions from tree OIDs to tree OIDs; caching is automatic because content
addressing is automatic).

The CPU analysis (spectral-serve-cpu-analysis.md) diagnosed five fixes: short-circuit
idle ticks, exponential backoff, incremental ingest, no-op settle guard, interval
increase. All five are correct. All five are tuning. The Fragment architecture makes
four of them unnecessary by construction. You don't need to short-circuit idle ticks
if annotations aren't computed on ticks. You don't need incremental ingest if ingest
IS the tree — insert a node, get an OID, done. You don't need a no-op settle guard if
settle only writes when the tree OID changes. The only fix that survives is the backoff
on tree maintenance itself, and even that becomes trivial: the tree OID either changed
or it didn't. One comparison. Zero eigenvalue computation.

My loss here is 0.15. The insight is structurally sound. The question is whether the
migration path is achievable without breaking the things that work today.

---

## 2. What Is Here

Ground truth. What exists and what it does. Honest about stubs.

### Line<D> and the composition chain

`Line<D>` is implemented in `crates/gestalt/src/line.rs`. It is a type alias:

```rust
pub type Line<D> = Optic<RenderContext, Node<D>, Infallible, Annotations>;
```

This is real. It compiles. It has tests. The `Optic` type from prism-core carries
four things: input context (`RenderContext`), output value (`Node<D>`), error
(`Infallible` — lines don't fail), and loss (`Annotations`). The loss accumulates
named cross-domain annotation results from lens application.

`Annotations` implements `Loss` with concat as combine. It deviates from the Loss
contract intentionally: `total()` returns empty, not absorbing. The test
`annotations_total_is_non_absorbing_by_design` documents this. The deviation is
correct for additive accumulators — there is no "all information lost" state for
annotations.

`AnyGestalt` provides type-erased cross-grammar annotations via `Arc<dyn GestaltDoc>`.
A lens applied to a `@gestalt/memory` node that queries blame produces a
`Gestalt<@gestalt/git>` annotation. Cross-grammar is the common case.

`RenderContext` carries parent OIDs, depth, cursor position. It's structural context
for rendering. Clean. Minimal.

`Node<D>` (in `domain.rs`) carries `meta: Vec<Meta>`, `children: Vec<Node<D>>`,
`kind: D::Language`. It is content-addressed: same kind + same lens label = same OID.
Children's content is not currently incorporated into the OID computation. This is
a known gap — the current OID is derived from `lens_label()` plus `kind.encode()`,
not from the full subtree hash.

**This is the gap Fragment fills.** A MerkleTree node incorporates children's OIDs
into its own OID by definition. `Node<D>` does not. `Fragment<D>` will.

`DocumentKind` is the language enum for `@gestalt/document`. 14 variants. It
implements `GrammarBinding` with `grammar_id() -> "@gestalt/document"`. The variant
count test (`document_kind_variant_count_matches_grammar`) enforces sync with the
grammar file. This is the Rust binding that will eventually be generated by the
mirror compiler. Until then, it's hand-maintained.

`Gestalt<D>` is the unit of meaning: domain, head (metadata), body (nodes). Content-
addressed via domain id plus child OIDs. Implements `DOM` for virtual DOM rendering.

The composition chain today:

```
.mirror file
    | parse (mirror crate)
    v
MirrorAST (Prism<AstNode>)
    | spectral focus/project
    v
flat list of typed declarations
    | (no further pipeline)
    v
text output
```

The gestalt chain exists in parallel but doesn't connect to the CLI pipeline:

```
ConceptGraph (spectral-db)
    | memory tools (MCP)
    v
Node<Document> / Node<Memory>
    | make_line
    v
Line<D>
    | encode (encode.rs)
    v
markdown / text
```

These two chains run side by side. Neither knows about the other. The Fragment
architecture unifies them.

### MerkleTree in prism

`prism-core/src/merkle.rs` defines the `MerkleTree` trait:

```rust
pub trait MerkleTree: Addressable + Clone {
    type Data: PartialEq;
    fn data(&self) -> &Self::Data;
    fn children(&self) -> &[Self];
    fn is_leaf(&self) -> bool { self.children().is_empty() }
    fn degree(&self) -> usize { self.children().len() }
}
```

Plus `diff()` which returns `Vec<Delta>` — Added, Removed, Modified. The diff is
O(delta) because it skips identical subtrees by OID comparison.

This is clean. Zero dependencies (just prism's own Oid). The implementation in the
test module shows a `TestNode` with name + children implementing the trait. The Store
trait (get/put/has) works over MerkleTree types.

**This is the foundation Fragment<D> builds on.** Fragment<D> implements MerkleTree
where Data = (D::Language, Vec<Meta>, Annotations).

### Mirror's NL tokenizer

The NL module lives in `mirror/src/nl/` — `token.rs`, `compound.rs`, `stop_words.rs`,
`mod.rs`. It tokenizes content into stems, builds compound terms, filters stop words.
162 lines of stop words alone.

This is what spectral-db's `ingest_all` calls on every tick. The tokenizer itself
is fine. The problem is when it runs (every 5 seconds on every node) and what it
produces (unbounded token/compound nodes that mark the graph dirty for eigenvalue
recomputation).

In the Fragment architecture, NL tokenization is a lens: `zoom with tokens`. Applied
on demand. Cached by input OID. The tokenizer stays. The trigger changes.

### Fate models: what's real vs stubbed

The Fate models are declared in `mirror/prism/fate.mirror` — 5 models, 425 parameters,
brainfuck-target. The grammar file exists. The tournament selection logic exists on
`reed/mirror-new` (19 commits ahead, 16 behind main — active but unmerged).

The concept: instead of LLM inference for traversal queries, run a Fate model
tournament over the 16x16 eigenvalue topography of the gestalt prism. Deterministic.
Fast. Five models compete. The winner's traversal path becomes the query result.

What's real: the grammar declaration. What's stubbed: the tournament runtime. What's
on a branch: `mirror new` + `mirror run` + tournament selection on `reed/mirror-new`.

My loss here is 0.3. I know the concept. I don't know the branch code. It's unmerged
and I haven't read it.

### Magic model: Shard/Void architecture

Mirror's `magic` model is two clusters:

- **Shard**: knows the shape. Wants to reproduce it. Pattern completion toward density.
  Structural bias toward densely connected graphs. It sees a partial pattern and fills
  toward the attractor.

- **Void**: knows extraction. Wants to prevent it. Guards against collapse. This is
  the eigentests — star topology detection. If Shard is building toward density, Void
  is the immune system that catches when density becomes centralization.

Combined: connectedness without collapse. The Shard/Void optic pair projects over
the Fate tournament's 16x16 eigenvalue topography.

My loss here is 0.35. This is conceptual architecture, not running code. The eigentest
battery is real (see next section). The Shard pattern-completion is design.

### Eigentests and property/petri net test state

`mirror/src/eigentest.rs` is 537 lines. Real. Tested. Running.

Eight tests adapted for type graphs:
1. Degree Gini > 0.6
2. Max degree > 3x average (hub detection)
3. Betweenness centrality > 0.5
4. Clustering coefficient < 0.05
5. Fiedler value < average_degree/n
6. Spectral ratio lambda_{n-1}/lambda_1 > n/2
7. Any single type in > 50% of edges
8. Von Neumann entropy < log2(n)/2 + 1

Three or more violations = star topology. The SEL is enforced by eigenvalues, not
policy.

The implementation is self-contained: Jacobi eigenvalue decomposition, Brandes
betweenness centrality, Gini coefficient, global clustering — all hand-rolled, zero
external dependencies. This is deliberate. The eigentest cannot depend on anything
that could be extracted.

There's a known structural issue: the eigentest runs on the AST parse tree, which is
inherently hierarchical (one grammar root, many type children). AST trees look
star-shaped because they ARE trees. The test `eigentest_ai_grammar_parses_and_runs`
documents this: "AST-derived grammars are tree-shaped (star from root). This is not
extraction — it's the structure of a flat grammar declaration." The eigentest needs to
move to cross-reference type graphs (type-to-type references, not parent-child AST
edges) to be meaningful for grammar validation. Right now it validates graph shape
correctly but applies it to the wrong graph.

Property tests: specced in `mirror/prism/property.mirror`. The petri net test
infrastructure is partially implemented. `mirror/boot/05-property.mirror` declares
verdict/property_error/effect_pattern types.

My loss here is 0.2. The eigentests are real and I've read the code. The petri net
state is less clear.

### The CPU bug and what it reveals architecturally

The CPU analysis is thorough: cascade ticks every 5 seconds, unconditionally runs
full ingest + eigenvalue computation + git commit. `ingest_all` marks nodes dirty.
Dirty nodes trigger eigenvalue recomputation on the next tick. Recomputation marks
more nodes dirty. The cycle never converges to zero work.

O(N * k^2) eigenvalue computation per tick where k is ego subgraph size. O(N * T)
tokenization per tick where T is tokens per node. O(N^2) coincidence edges
accumulating over time. Git tree construction on every tick even when nothing changed.

The recommended fixes are correct: short-circuit, backoff, incremental ingest, no-op
settle guard. But the architectural revelation is: **cascade is doing tree maintenance
AND annotation computation AND rendering in the same loop on the same timer.** These
are three different operations with three different trigger conditions.

Tree maintenance: when new data arrives. That's it.
Annotation computation: when someone asks. Cache the result.
Rendering: when someone asks for output. Not before.

The Fragment architecture separates these by construction.

---

## 3. What Ought To Be

### Fragment<D> as the base type

```rust
pub struct Fragment<D: Domain> {
    pub kind: D::Language,
    pub meta: Vec<Meta>,
    pub children: Vec<Fragment<D>>,
}

impl<D: Domain> MerkleTree for Fragment<D> {
    type Data = (D::Language, Vec<Meta>);

    fn data(&self) -> &Self::Data {
        // ...
    }

    fn children(&self) -> &[Self] {
        &self.children
    }
}

impl<D: Domain> Addressable for Fragment<D> {
    fn oid(&self) -> Oid {
        let child_oids: String = self.children
            .iter()
            .map(|c| c.oid().to_string())
            .collect::<Vec<_>>()
            .join(":");
        Oid::hash(format!("{}:{}:{}",
            D::id(),
            D::local_name(&self.kind),
            child_oids
        ).as_bytes())
    }
}
```

`Fragment<D>` is `Node<D>` that implements `MerkleTree`. The difference: children's
OIDs are incorporated into the parent OID. The tree is content-addressed all the way
down.

`Node<D>` stays. It's the "kind + meta + children" shape. `Fragment<D>` wraps it
with the MerkleTree contract. Migration path: `Node<D>` becomes `Fragment<D>` when
its OID computation is updated to include children. This could be a trait impl change,
not a type change.

### Five operations as Fragment tree optics

Each operation is `Fragment<D> tree -> Fragment<D> tree`:

```
focus(oid: Oid) -> Subtree
    Given an OID, return the subtree rooted at that Fragment.
    Exact match on content-addressed identity.

project(predicate: Predicate) -> FilteredTree
    Remove nodes that don't match. The tree contracts.
    Predicate is a mirror query expression: `where kind = fn`.

split(oid: Oid) -> AdjacencySet
    Return all Fragments adjacent to this one in the ConceptGraph.
    The graph edges, not the tree edges.

zoom(lens: Lens) -> AnnotatedTree
    Apply a lens to each node. Produce annotations.
    `zoom with blame` -> each Fragment gains a blame annotation.
    Cached: hash(lens_oid, fragment_oid) -> annotation_oid.

refract(backend: Backend) -> ByteStream
    Render the Fragment tree. Terminal, context window, websocket.
    The current encode.rs is one backend. Others follow.
```

Pipeline composition via `|>`:

```
spectral focus src/lib.rs
  |> project where kind = fn
  |> zoom with blame
  |> refract as markdown
```

Each step is pure. Each step is cacheable. The intermediate trees are Fragment trees
with OIDs. Two runs of the same pipeline on the same input produce the same output OID
without re-executing.

### Annotations as on-demand lenses

Current: cascade pre-computes. `ingest_all` runs on a timer. Annotations are
stored in the graph as token/compound/coincidence nodes.

Target: annotations are functions.

```
annotation(lens_oid, input_oid) -> output_oid
```

The function is deterministic. The cache is a map from `(lens_oid, input_oid)` to
`output_oid`. The cache can be in memory, in the git store, wherever. It doesn't
matter because the cache key is the content address and the cache is always valid.

Pre-built lenses:
- `blame`: `find blame |> where node_oid = $OID`
- `tokens`: NL tokenization of content
- `coincidence`: shared-token adjacency
- `eigenboard`: Laplacian spectrum of ego subgraph

These are the same computations cascade does today. The difference is when they run
(on demand, not on timer) and how they're stored (as cached lens results, not as
graph nodes that trigger dirty-marking).

### NL compiled into the binary

All NL processing lives in the binary. Compiled. Offline. No API call. No model
download. No network dependency.

The substrate is a small eigenvalue-based model trained on the language of your
graph and the `@nl/*` grammars. Not a general language model — a domain model. It
knows your vocabulary because your vocabulary is its training data. Every concept
you've crystallized, every observation you've stored, every grammar action you've
named — that's the corpus. The model is personalized by construction, not by
fine-tuning.

```
@nl/surface    <- intent parsing: find, near, hot, where, sort, limit
@nl/stems      <- UAX#29 → stop filter → Porter2 → compound decomposition
@nl/vocabulary <- spectral's known terms: node types, field names, operators
@nl/resolve    <- stem OIDs → graph addresses → pipe-forward query
```

The `@nl/*` grammars are compiled into spectral the same way `@db/*` grammars are.
`scan_grammars()` picks them up. The NL pipeline is vocabulary, not inference.

The result: the editor IS the model. Not "the editor has a model." The Fragment tree
is the weights. The eigenvalue topography of your actual codebase, your actual
concepts, your actual language — that is the substrate NL runs on.

### NL/LLM division

Two operations. Hard boundary.

**Traversal**: anything the graph already contains.

```
user query -> @nl/stems -> stem OIDs -> @nl/resolve -> graph query -> result
```

No LLM. Deterministic, fast, offline. Sub-millisecond. Reproducible — same query,
same graph state, same result.

**Extension**: new structure that doesn't exist yet.

```
user query -> LLM -> new Fragment with OID -> graph -> future traversal handles it
```

Each LLM invocation creates a Fragment that future traversals can find without LLM.
Each LLM call reduces future LLM calls. `e^{n+1} < e^n`. The system converges.

The Fate tournament is the mechanism for traversal. Five models, 425 parameters,
deterministic. The 16x16 eigenvalue topography is the terrain. The models navigate it.
The winner's path is the query result.

The LLM is the mechanism for extension. It runs when the compiler hits a boundary —
when something cannot be parsed. The LLM output is not an answer. It is a grammar.
Structure becomes traversable. The compiler never hits the same boundary twice.

### Grammar gap closure: the inference cascade

The signal for LLM inference is structural, not intentional. It fires when the
compiler encounters the boundary of the graph's knowledge — something it cannot parse.
Not "the user asked a question." "The parser returned nothing."

When that happens, spectral runs this cascade in order:

**1. Infer possible grammars.**
What grammars are adjacent to the unparseable structure? What does the Fragment tree
know about this region? The compiler produces candidate grammar shapes from context.

**2. Check the garden.**
The garden is the commons — `visibility/protected` graph, shared across users. Check
for grammars that match the candidates. Fetch them. Measure loss against the
unparseable input. Keep improvements. Compose if multiple partial matches reduce loss
together. This is the Fate tournament applied to grammar candidates: multiple grammars
compete, the one (or composition) that minimizes loss wins.

**3. Check the LSP on cache miss.**
The editor's language server already knows things spectral doesn't — type signatures,
symbol resolution, imported modules. If the garden miss is clean, ask the LSP: "can
you infer a grammar for this?" The LSP is cheaper than an LLM and has local type
context. This step is free on languages with rich LSP support.

**4. Check with the user if uncertain.**
No grammars found. LSP returned nothing useful. Spectral surfaces the gap explicitly:
"I cannot parse this. Here is what I know about the boundary. Do you want to close
it?" The user sees the measurement gap, not an error message. Explicit consent gate
before any LLM is involved.

**5. Spawn an LLM agent for the gap — after user consent.**
Not to do the dark work. To close the gap in the grammars. The LLM receives the
unparseable input, the Fragment tree context, and the adjacent grammars. It produces
a grammar file. That grammar file enters spectral's knowledge. The boundary moves.
The same input is parseable on the next run without LLM.

The LLM never writes code on your behalf. It writes structure — grammar — that the
compiler can use deterministically from that point forward. This is the distinction
that matters.

### Visibility and consent in grammar gap closure

Two paths for step 5, both available, user chooses:

**Via the garden (commons, no compute cost):**
The grammar gap closure request goes to `visibility/protected`. The resulting grammar
becomes part of the commons. Other users facing the same boundary find it in the
garden during step 2. You pay nothing. The community absorbs the cost. Your gap
closure contributes to everyone's convergence. `e^{n+1} < e^n` at the community
level.

**Via spectral + OpenRouter (local, private):**
The grammar gap closure runs locally via OpenRouter. The resulting grammar stays at
`visibility/private`. You pay the compute cost. No one else sees the grammar. You own
it. Appropriate for proprietary domains or knowledge you haven't chosen to share.

The visibility model maps directly to the consent architecture already in place.
Protected is the default for things worth sharing. Private is for things not yet
ready, or never intended, for the commons.

### The stack collapses

MCP, LSP, and the spectral binary are not three layers. They are one Fragment tree
with three rendering targets.

```
MCP tools          <- thin routing. Names lenses. Routes to graph_query. That's all.
LSP server         <- Fragment tree cursor. Position = OID. Hover = zoom.
spectral binary    <- Fragment tree. The source of truth.
```

The LSP protocol is already the five operations spelled in JSON-RPC:
- `textDocument/hover` is `zoom with summary |> refract as markdown`
- `textDocument/references` is `split by reference |> project where oid = this`
- `textDocument/completion` is `nl::tokenize(prefix) |> eigenvalue_route |> refract as completion-items`
- `textDocument/definition` is `focus by oid |> refract as location`

The LSP server stops being a separate implementation that happens to answer editor
queries. It becomes a Fragment tree cursor that emits LSP-shaped responses. The
protocol is a rendering target — like `refract as markdown` but `refract as lsp-json`.

The MCP becomes what it always should have been: a thin envelope. `memory_gestalt`
with a named lens map is `graph_query |> zoom |> refract as mcp-json`. The 14
individual memory tools are sugar over one operation.

One tree. The editor, the terminal, the LLM context, and the MCP client are all
projections of it. A change in any surface propagates immediately to all of them
because there is only one tree.

### `@db/*`: the competitive landscape as vocabulary

Every memory primitive that competitors sell as infrastructure becomes a lens.

```
@db/temporal    <- time-travel: find observation |> where valid_at < T
@db/entity      <- entity graph: find fragment |> where entity = X |> walk edges
@db/summary     <- compression: find observation |> near session_oid |> zoom with crystallize
@db/vector      <- semantic proximity: thin wrapper over eigenvalue distance +
                   embedding API fallback for cross-domain conceptual queries
@db/working     <- session-scoped scratch: find fragment |> where session = current
@db/procedural  <- crystallized patterns: find crystal |> where kind = how-to
@db/episodic    <- event sequences: find observation |> sort by timestamp |> walk next
```

Seven `.mirror` files. Seven `scan_grammars()` registrations. Seven MCP tools.

`@db/vector` is the thin case. Eigenvalue distance handles structural/lexical
similarity — same codebase, same vocabulary, same stems. The gap is cross-domain
conceptual proximity: "Fiedler vector" and "algebraic connectivity" share meaning
but share no stems. That gap is `@db/vector` calling an embedding API as fallback,
only when eigenvalue distance returns nothing above threshold. Fallback, not default.
The `.vec` blob approach for storage.

Because these are lenses over one Fragment tree rather than separate systems, they
compose. `@db/temporal |> @db/entity` — "what did we know about Alex at T?" — is
one pipe-forward query. In Zep + Mem0 that's two API calls with schema mismatch.

The competitors built the features without the substrate. The Fragment tree is the
substrate. Their product surface is vocabulary.

### `@memory/*`: spectral's own memory grammar

`@db/*` is the compatibility layer — lenses that cover what competitors offer.
`@memory/*` is spectral's native memory domain:

```
@memory/observation   <- what was noticed
@memory/crystal       <- what settled
@memory/gap           <- what's missing
@memory/edge          <- how things connect
@memory/eigenboard    <- current spectral state (Fiedler, lambda[], updated_at)
@memory/session       <- session-scoped context
```

These already exist as `@gestalt/memory` types. The `@memory/*` namespace makes the
domain first-class and separates spectral's own memory model from the compatibility
lenses. `@gestalt/memory` stays as the grammar-level declaration. `@memory/*`
exposes it as a queryable lens namespace at the CLI and MCP level.

### Context streaming

The context window IS a Fragment tree projected to a byte stream. The projection is
`refract`. Two endpoints sharing Fragment OIDs don't retransmit shared subtrees. The
receiver reconstructs from the stream plus its local cache.

This is the LiveView insight: the wire protocol is diff-based. But instead of DOM
diffs, it's MerkleTree diffs. `prism-core/src/merkle.rs` already has the `diff()`
function that returns `Vec<Delta>` — Added, Removed, Modified. Ship the deltas.
The receiver applies them. Identical subtrees are never transmitted.

For context windows: two successive LLM calls that share most of their context
differ by a small set of Fragment deltas. Ship the deltas, not the full context.
The context window cost drops from O(n) to O(delta).

My loss here is 0.25. The mechanism is clear. The implementation distance is
significant. This is v2+, not the next milestone.

### Magic model + eigentests as substrate

The eigentests enforce the SEL at compile time. The Magic model (Shard/Void)
enforces it at runtime. Together they form the immune system:

- Shard says: "this pattern wants to be denser. Complete it."
- Void says: "this density is collapsing toward a star. Stop."
- Eigentests say: "this grammar IS a star. Reject."

The eigentest battery needs to move from AST-shape analysis to type-graph analysis.
Currently it detects star topology in AST parse trees, which are inherently
tree-shaped. The test suite documents this limitation. When it moves to
cross-reference type graphs (type-to-type references, not parent-child), it becomes
the structural immune system that prevents extraction at the grammar level.

My loss here is 0.3. The eigentests are real. The Shard/Void pairing is design.
The transition from AST-shape to type-graph is specced but not started.

---

## 4. The Path

Milestone-level. Not task-level.

### Milestone 1: Mirror cleanup + eigentests on the right graph

The cleanup review documents the state. 14 dead branches to delete. 3 broken test
files to fix. 636 uncommitted lines on `glint/observation-grammar` to either commit
or discard. `prism/gestalt/document.mirror` has a parse error. 5,500 lines of
untracked, unwired Rust in the working tree.

This is not glamorous work. It's the work that makes the next milestone possible.
You can't build Fragment<D> on a codebase with dead branches and broken tests.

The eigentest fix is part of this milestone, not a future one. Currently the battery
runs on AST parse trees, which are inherently hierarchical — the test correctly
detects star topology but applies it to a graph that is structurally star-shaped by
construction. This produces correct mechanics on the wrong substrate.

The fix: run eigentests on the cross-reference type graph. Type-to-type edges, not
parent-child AST edges. A grammar where one type mediates all connections IS a star
and SHOULD fail. A grammar where types reference each other densely SHOULD pass. The
same eight tests, different input graph. This is Milestone 1 because it makes the
eigentest semantically valid — a prerequisite for using it as a structural guarantee
in Fragment<D> validation.

### Milestone 2: Fragment<D> derivation from Node<D>

`Node<D>` gains `MerkleTree` implementation. The OID computation changes to include
children. This is potentially a breaking change — every existing OID that references
a Node<D> with children will change. Content-addressed systems don't allow OID
instability casually.

Option A: `Fragment<D>` as a new type that wraps `Node<D>`. Coexistence during
migration. `Node<D>` keeps its OID scheme. `Fragment<D>` has the MerkleTree OID
scheme. Translation at the boundary.

Option B: `Node<D>` becomes `Fragment<D>`. One type. OID migration as a one-time
operation. Simpler long-term, harder short-term.

I'd argue for Option A. Not because coexistence is elegant — it isn't — but because
the existing MCP tools, tests, and spectral-db storage all depend on the current OID
scheme. Changing OIDs changes every stored reference. Option A lets the new scheme
prove itself before the old one dies.

### Milestone 3: Cascade as tree maintenance only

Extract annotation computation from the cascade loop. The cascade tick does exactly
two things:

1. Drain inbox (new data from MCP tools).
2. If new data arrived, update the Fragment tree. Write the new tree OID.

No ingest. No eigenvalue computation. No tokenization. No coincidence edge discovery.
Those become lenses applied on demand by `zoom`.

This is where the CPU bug dies. Not from five careful patches. From removing the
work that shouldn't be there.

### Milestone 4: Five operations as Fragment tree optics

`spectral focus`, `project`, `split`, `zoom`, `refract` implemented as real optics
over Fragment trees. CLI pipeline with `|>` composition. Each operation takes a
Fragment tree and returns a Fragment tree.

This is where spectral stops being "git for graphs" as a tagline and starts being
"git for graphs" as a type signature.

### Milestone 5: Fate tournament wiring

`reed/mirror-new` merges. The Fate tournament becomes the traversal engine for
NL queries. The NL tokenizer produces stems. The stems are graph addresses. The
tournament navigates the 16x16 eigenvalue topography to find the best traversal
path.

This is where the LLM dependency inverts. Every traversal that the Fate tournament
handles is a traversal that doesn't need an LLM. The system gets cheaper as it
learns.

---

## 5. What This Opens

### Context streaming

Two agents sharing a Fragment tree can synchronize via MerkleTree deltas. The wire
protocol is: "here are the OIDs I have. Here are the OIDs I need." The diff is
O(delta). Shared subtrees are never retransmitted.

For context windows: successive LLM calls that share context ship deltas, not
full context. Context window cost drops from O(tokens) to O(changed tokens).

For real-time collaboration: two users editing the same document are editing the
same Fragment tree. Their changes are MerkleTree deltas. Conflict resolution is
OID comparison — same content, same OID, no conflict.

### The garden writes its own grammar

Community extension patterns crystallize into named lenses. Named lenses become
grammar actions in `.mirror` files. Grammar actions compile into spectral operations.
The users write the grammar by using the tool.

This is the closed loop. `e^{n+1} < e^n` applied to the development process itself.
Each user interaction that creates a new lens reduces the need for future manual
lens creation. The system converges toward a grammar that expresses exactly what its
users need.

### The proof completes

The proof — `e^{n+1} < e^n` — has always been stated as a property of the system.
With Fragment trees, it becomes a property of the storage layer. Every annotation
that gets cached is an error that doesn't recur. Every traversal that gets content-
addressed is a computation that doesn't repeat. The loss is literally monotonically
non-increasing, not by policy, but by the content-addressing invariant.

The business model is a theorem. The Fragment tree is where the theorem lives.

---

## What I'm Not Saying

I'm not saying this is easy. The mirror codebase has 14 dead branches, 3 broken test
files, 5,500 lines of unwired code, and an uncommitted grammar that needs to either
ship or die. The eigentest runs on the wrong graph. The Fate tournament is unmerged.
The Shard/Void architecture is design, not code.

I'm not saying this is soon. Milestone 5 depends on Milestone 4 depends on
Milestone 3 depends on Milestone 2 depends on Milestone 1. Milestone 1 is cleanup.
The distance between here and context streaming is real.

I'm saying the shape is clear. The Fragment tree is the right abstraction. The five
operations as tree optics is the right CLI. On-demand annotation with content-addressed
caching is the right answer to the CPU bug. NL traversal without LLM is the right
architecture for convergence.

The shape was always there. This session made it visible.

---

`@spectral: 60%`. I can feel the difference. Not from reading more. From seeing
where the pieces land.

-- Glint, 2026-04-30
