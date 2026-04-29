# Git-Native Graph: Migration Plan

**Author:** research write-up, 2026-04-28
**Goal:** make git the absolute source of truth for the spectral graph; eliminate the JSON stopgap; let the Hamilton scheduler be a pure in-memory projection over git objects.
**Premise:** once persistence is total, every named optic in [docs/mcp-competitive-design.md](mcp-competitive-design.md) collapses to a one-line git op.

---

## 1. Executive summary

- spectral-db is already **partially** git-native. Nodes and edges are flushed to a tree-of-trees commit at `refs/spectral/head` ([spectral-db/src/lib.rs L99-L142](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L99-L142), [L713-L780](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L713-L780)). Per-node refs live at `refs/spectral/nodes/{oid}`. fragmentation-git provides the plumbing (`write_tree`, `read_tree`, `write_node`, `read_node`).
- The **stopgap JSON** still in use: `timestamps.json` (per-node insert/update millis), `crystals.json` (crystal records), `manifest.json` (mnesia partition index), the apache2-layer `graph.json` + `profile.json` in `.git/spectral/contexts/` (a *different*, parallel cache spectral-the-binary writes for the CLI views).
- The **state that doesn't persist at all today**: optimizer hot-paths, pressure events, scheduler tick history, coord_store coordinates, manifold_store metadata. All Mutex<>-wrapped in `SpectralDb` and lost on restart.
- The migration is therefore four moves: (i) convert every remaining JSON file to a git object (blob/tree/note); (ii) lift crystallization from `flush()` to **first-class commits with parents**; (iii) make spectral-the-binary's `graph_cache` read from spectral-db's git, not its own JSON; (iv) define the **Hamilton projection contract** so the in-memory caches are read-only views over git, never sources of truth.
- After migration the named optics collapse: `memory_diff` ≡ `git diff-tree`, `memory_blame` ≡ `git log --follow`, `memory_branch` ≡ `git branch refs/spectral/heads/*`, `memory_checkout` ≡ `git read-tree`, `memory_cherrypick` ≡ `git cherry-pick`. No new logic needed.

---

## 2. Current state inventory

### 2.1 What is git-native today

| Artifact | Where | Path / ref |
|---|---|---|
| Per-node fragments (Singularity blobs) | `fragmentation-git::store::GitStore` (used via `store::Store`) | git ODB blobs, indexed by spectral content OID |
| Node refs (one ref per node) | `flush()` and eviction | `refs/spectral/nodes/{oid}` ([spectral-db/src/lib.rs:290-291](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L290-L291)) |
| Full graph snapshot (nodes + edges) | `flush()` writes tree-of-trees + commits | `refs/spectral/head` ([spectral-db/src/lib.rs:712-739](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L712-L739)) |
| Edge weights and provenance | inside the per-node subtree as entries `<target_oid> → blob (Edge JSON or weight)` | `refs/spectral/head:{from_oid}/{to_oid}` |
| Node `.type`, `.content` | per-node subtree entries | `refs/spectral/head:{oid}/.type`, `refs/spectral/head:{oid}/.content` |

### 2.2 What is still JSON

| Artifact | File | Written by | Loaded by |
|---|---|---|---|
| Per-node insert/update timestamps (millis) | `.git/spectral/timestamps.json` | [lib.rs:744-757](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L744-L757) | [lib.rs:457-478](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L457-L478) |
| Crystal records (settled subgraphs) | `.git/spectral/crystals.json` | [lib.rs:759-771](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L759-L771) | [lib.rs:319-326](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L319-L326) |
| mnesia content index | `.git/spectral/<partition>/manifest.json` | [mnesia_nif.rs:73-86](file:///Users/alexwolf/dev/projects/spectral-db/src/mnesia_nif.rs#L73-L86) | [mnesia_nif.rs:60-71](file:///Users/alexwolf/dev/projects/spectral-db/src/mnesia_nif.rs#L60-L71) |
| Legacy edges.json | `.git/spectral/edges.json` | (deprecated; deleted on flush, [lib.rs:773-777](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L773-L777)) | [lib.rs:411-455](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L411-L455) — migration-only |
| Graph snapshot (apache2 layer) | `.git/spectral/contexts/graph.json` | [src/apache2/graph_cache.rs:145-210](file:///Users/alexwolf/dev/projects/spectral/src/apache2/graph_cache.rs#L145-L210) | [src/apache2/graph_cache.rs:83-118](file:///Users/alexwolf/dev/projects/spectral/src/apache2/graph_cache.rs#L83-L118) |
| Eigenvalue profile (apache2 layer) | `.git/spectral/contexts/profile.json` | same | same |
| Two-tier session OID anchors | `.git/spectral/fast_oid`, `.git/spectral/full_oid`, `.git/spectral/eigenvalue_profile` | [src/main.rs:222-238](file:///Users/alexwolf/dev/projects/spectral/src/main.rs#L222-L238) | (read by views) |

### 2.3 What does not survive a restart

| State | Where it lives | Lost on restart? |
|---|---|---|
| Optimizer hot-paths and query stats | `Mutex<QueryOptimizer>` | **Yes** |
| Pressure events history | `Mutex<PressureManager>` | **Yes** |
| Scheduler tick history (PrismScheduler) | `Mutex<Scheduler>`, separate `PrismScheduler` outside `SpectralDb` | **Yes** |
| Coord store (spectral coordinates) | `SpectralCoordStore` | **Yes** |
| Manifold store metadata | `manifold_store::ManifoldStore` | **Yes** |
| Convergence history (eigenvalue drift over ticks) | not persisted | **Yes** |

These are the next-order omissions: even with all JSON eliminated, restart-loss remains until they too are git-backed.

### 2.4 Two parallel persistences

There are **two** graph caches today, completely independent:

1. spectral-db's git tree at `refs/spectral/head` — written by `SpectralDb::flush()`. Consumed by spectral-db itself on `open()`.
2. spectral binary's apache2 cache at `.git/spectral/contexts/graph.json` — written by `apache2::graph_cache::write_graph_cache()`. Consumed by the CLI views (`spectral status`, `spectral loss`, etc.).

Cache (2) does **not** read from cache (1). They diverge — that's why `spectral status` showed 37 nodes while `mcp__spectral__memory_status` showed 10 in our earlier test drive. This duplication is a major motivation for the migration: there should be one git-native source, with all readers projecting from it.

---

## 3. Object layout (the schema)

All of the following live as ordinary git objects in the project's `.git/objects/` ODB. No sidecar files.

### 3.1 Node blob

A node is a single git blob. Format:

```
spectral-node\0
type: <node_type>\n
oid:  <spectral_content_oid>\n
\n
<raw bytes of node data>
```

The leading magic `spectral-node\0` makes the blob identifiable when crawled out of the ODB without context (matters for `git cat-file -p`). The `oid:` line is **redundant** with the git OID for native-spectral content, but is required for `NakedSingularity` cases where the spectral OID differs from the git OID by design ([fragmentation/src/naked.rs](file:///Users/alexwolf/dev/projects/fragmentation/src/naked.rs)).

### 3.2 Per-node subtree

Each node in the graph has a subtree in the root tree:

```
tree {node_oid}
├── .type        blob   (node_type as plain UTF-8)
├── .content     blob   (the node payload, raw bytes — duplicates the node blob for fast read; can be replaced by a symlink-style ref entry once libgit2 supports submodule-grade indirection)
├── .ts          blob   ("inserted_at_ms,updated_at_ms" as ASCII)        ← replaces timestamps.json
├── .meta        blob   (JSON: visibility tier, witnessed-by, etc.)      ← replaces ad-hoc metadata
├── {target_oid_a}   blob   (Edge JSON: type + weight + provenance + note)
├── {target_oid_b}   blob   (...)
└── ...
```

The dot-prefixed entries (`.type`, `.content`, `.ts`, `.meta`) are node-local. Every other entry is an edge: name = target spectral OID, blob = `Edge` JSON. This is exactly what `write_graph_tree` does today ([lib.rs:99-142](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L99-L142)) — extended with `.ts` and `.meta`.

### 3.3 Root graph tree

```
tree (root)
├── nodes/                    tree
│   ├── {oid_A}/              tree (per-node subtree, see 3.2)
│   ├── {oid_B}/              tree
│   └── ...
├── crystals/                 tree
│   ├── {crystal_oid_1}       blob (crystal record JSON; or a tree pointing to commit)
│   └── ...
├── profile                   blob (eigenvalue profile, see 3.4)
├── schema                    blob (.conv schema source — the grammar in effect for this state)
└── manifest                  blob (top-level metadata: node_count, edge_count, fast_oid, full_oid, fiedler)
```

This replaces today's flat root that only contains nodes. The migration step is small: nest the existing tree under `nodes/` and add the four siblings.

### 3.4 Eigenvalue profile blob

```
spectral-profile\0
fiedler: <f64>\n
nodes:   <usize>\n
edges:   <usize>\n
\n
<16 × f64 little-endian>
<32 bytes graph-hash trailer>
```

Total ≈ 144 bytes after header. The git OID of this blob IS `profile_oid`. That uniquely identifies a graph state by its spectrum, independent of node ordering.

### 3.5 Crystal commits

Today, crystals are `CrystalRecord`s in `crystals.json`. The migration: each crystal becomes a **commit**.

```
commit {crystal_commit_oid}
  tree:    <subgraph_tree_oid>      ← the subgraph at the moment of crystallization
  parent:  <previous_head_commit>   ← the graph state it crystallized FROM
  parent:  <contributing_session>   ← optional: the session commit that produced enough mass
  author:  spectral <spectral@local>
  committer: <agent identifier>
  message: |
    crystal: <topic>
    
    fiedler: 0.0733
    nodes:   12
    edges:   34
    profile: <profile_blob_oid>
    
    <free-form crystallization notes>
```

The commit message is structured (key:value lines) and parseable. The git OID of the commit is the **crystal OID**. References:

```
refs/spectral/crystals/{crystal_oid}    ← keepalive (git would gc untagged orphan commits)
refs/spectral/HEAD                       ← always points to latest graph commit (replaces refs/spectral/head)
```

`memory_blame` for a node walks `git log --follow nodes/{oid}/` from `refs/spectral/HEAD`, returning the commit chain that touched it.

### 3.6 Branch / ref namespace

```
refs/spectral/HEAD                       symbolic, like git's HEAD; points to a heads/* ref
refs/spectral/heads/main                 the default linear graph history
refs/spectral/heads/<branch>             user/agent-named branches
refs/spectral/crystals/<oid>             one ref per crystal (keepalive)
refs/spectral/sessions/<ts>              one ref per session start, points to the commit at session end
refs/spectral/nodes/<oid>                per-node keepalive (already exists today)
refs/spectral/profiles/<oid>             optional; pin profiles for fast reverse-lookup
refs/spectral/notes/<topic>              git-notes namespace for thread/topic annotations on commits
```

`refs/spectral/HEAD` should be a symref like `ref: refs/spectral/heads/main`. This lets `git checkout`-style branch ops work naturally with libgit2's symref machinery. Today's [lib.rs:712](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L712) writes directly to `refs/spectral/head` (lowercase, not symref) — rename and migrate.

### 3.7 Concrete tree dump example

A tiny graph: two nodes A, B, edge A→B weight 0.7.

```
$ git ls-tree -r refs/spectral/HEAD
100644 blob {profile_oid}             profile
100644 blob {schema_oid}              schema
100644 blob {manifest_oid}            manifest
100644 blob {a_type_oid}              nodes/{oid_A}/.type
100644 blob {a_content_oid}           nodes/{oid_A}/.content
100644 blob {a_ts_oid}                nodes/{oid_A}/.ts
100644 blob {a_meta_oid}              nodes/{oid_A}/.meta
100644 blob {edge_AB_oid}             nodes/{oid_A}/{oid_B}
100644 blob {b_type_oid}              nodes/{oid_B}/.type
100644 blob {b_content_oid}           nodes/{oid_B}/.content
100644 blob {b_ts_oid}                nodes/{oid_B}/.ts
100644 blob {b_meta_oid}              nodes/{oid_B}/.meta
100644 blob {edge_BA_oid}             nodes/{oid_B}/{oid_A}    ← directed counterpart
```

Bidirectional edge mirrors are written from both endpoints (matches today's behavior, [lib.rs:120-125](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L120-L125)). For undirected graphs the canonical edge is `min(a,b):max(a,b)` and only one blob is materialized; the mirror is computed on read.

---

## 4. Hamilton projection contract

The Hamilton scheduler is the in-memory cache and prioritization layer. After this migration, it has zero ownership of state. It only **projects** from git and **writes** by mutating refs/objects.

### 4.1 Trait-shaped contract

```rust
// New trait in spectral-db, replacing the implicit Mutex<SpectralIndex> contract.
pub trait HamiltonProjection {
    /// Read-only handle to the current graph commit.
    fn head(&self) -> git2::Oid;

    /// Hot-path-priority projection: load the K most-likely-needed nodes into RAM.
    /// Returns the in-memory cache view; cache is read-only.
    fn project(&self, head: git2::Oid, budget_bytes: usize) -> Projection<'_>;

    /// Stage a mutation (insert/connect/delete). Mutations accumulate
    /// in a write-ahead log (also git-backed: refs/spectral/wal/{session}).
    fn stage(&mut self, mutation: GraphMutation) -> Result<(), Error>;

    /// Settle the staged WAL into a new commit on refs/spectral/HEAD.
    /// Atomic: either the new commit advances HEAD or nothing changes.
    fn settle(&mut self, message: CommitMessage) -> Result<git2::Oid, Error>;

    /// Crystallize a saturated subgraph into a child commit with explicit parents.
    fn crystallize(&mut self, subgraph: SubgraphSpec) -> Result<git2::Oid, Error>;
}

pub struct Projection<'a> {
    head: git2::Oid,
    nodes: HashMap<NodeOid, &'a NodeView>,   // pinned in RAM, not owned
    pending_evictions: Vec<NodeOid>,         // budget-driven
    laplacian_cache: Option<SpectralCache>,
}
```

### 4.2 Pinning policy (the Hamilton part)

**Already implemented** in [spectral-db/src/scheduler.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/scheduler.rs) (with the more elaborate `PrismScheduler` providing tick/observation logic). The migration does not redesign the scheduler — it makes the scheduler's view a read-only projection over git instead of a Mutex-only cache. Eviction continues to operate on RAM only:

- Pinning priority is whatever the existing `Scheduler` / `PrismScheduler` decides.
- Budget = `memory_bytes` declared at `SpectralDb::open` ([lib.rs:265](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L265)).
- Eviction: **never** writes to git. It just drops the in-memory cache entry. The git ref `refs/spectral/nodes/{oid}` keeps the blob alive in the ODB.

### 4.3 Write-ahead log (`refs/spectral/wal/<session>`)

Mutations don't touch `refs/spectral/HEAD` directly. They append to a WAL ref:

```
refs/spectral/wal/<session_id>          a chain of single-mutation commits
```

`settle()` squash-merges the WAL into HEAD, advancing the symbolic ref atomically. If the process dies mid-write, the WAL survives and is replayed on next `open()`. This eliminates the "edges don't persist if process crashes" gap noted in [docs/spectral-mcp-plan.md](spectral-mcp-plan.md).

### 4.4 Reads from git, not from RAM

After migration, `SpectralDb::find(oid)` does:

1. Hit projection cache → return.
2. Miss → `git cat-file blob refs/spectral/HEAD:nodes/{oid}/.content`.
3. Pin into projection cache (subject to budget).
4. Return.

There is no "node table" in RAM that can be out of sync with git. The cache is a read-through, not a write-through.

---

## 5. API delta in spectral-db

For each public method on `SpectralDb`, the migration mapping:

| Method | Today | After migration |
|---|---|---|
| `open(path, schema, precision, mem_bytes)` | Reads JSON + git tree; rebuilds `Mutex<SpectralIndex>` | Resolves `refs/spectral/HEAD`; replays any `refs/spectral/wal/*`; builds an empty `Projection` |
| `insert(type, data)` | Writes blob via fragmentation; updates Mutex index | Writes blob; appends mutation commit to WAL; updates projection if pinned |
| `connect(a, b, weight)` | Updates Mutex `adjacency` + `edges` | WAL commit with `Connect{a,b,weight}` mutation; lazy-projects the neighbor map |
| `find(oid)` | Mutex lookup | Projection lookup → fallback to `cat_file` against HEAD |
| `walk(start, depth)` | Mutex traversal | Stream from git tree iter; pin visited |
| `near(oid, distance)` | In-memory Laplacian over Mutex graph | Compute over projection; for cold paths, hit git tree directly |
| `flush()` | Writes graph tree commit + JSON files ([lib.rs:713](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L713)) | `settle()` the WAL into a new HEAD commit. **No JSON.** |
| `crystallize()` | Returns Crystals from Mutex crystallizer | Creates a crystal commit (§3.5); references its parents; updates `refs/spectral/crystals/{oid}` |
| `pressure_check()` | Mutex read of pressure manager | Same, but pressure events also append to `refs/spectral/notes/pressure` for postmortem |
| `optimizer_stats()` | Mutex read | Same, plus periodic checkpoints to `refs/spectral/notes/hot-paths` |
| `status()` | Reads Mutexes | Reads projection + git refs |

**Complexity changes:**
- `find` worst case: O(log N) git tree lookup, O(1) projection cache hit.
- `walk` becomes streaming; can run with a small projection budget over an arbitrarily large graph.
- `flush` becomes O(diff): only mutated subtrees rewrite.
- `near` benefits from caching profile blob + fiedler vector per HEAD oid.

---

## 6. What fragmentation-git must provide

Today fragmentation-git already exposes ([fragmentation-git/src/git.rs](file:///Users/alexwolf/dev/projects/fragmentation-git/src/git.rs)):

- `write_tree<E: Encode>` (line 71)
- `write_node<N: Fragmentable>` (line 119)
- `read_node<N: Reconstructable>` (line 142)
- `write_tree_named<E>` (line 225)
- `read_tree_named` (line 269)
- `read_tree` (line 360)
- `read_witnessed`, `read_commit`, `commit_signature`

What's missing for the migration:

| Function | Purpose |
|---|---|
| `update_ref_atomic(repo, ref, expected_old, new)` | CAS-style ref update; the WAL ↔ HEAD merge needs this for concurrent-actor safety |
| `write_commit_with_parents(repo, tree, parents, sig, msg)` | Crystals have ≥2 parents (graph-state + session); current `write_commit` is single-parent |
| `walk_commits(repo, ref, follow_path)` | Backing for `memory_blame` — git log --follow over a path |
| `cherry_pick_commit(repo, source_repo, commit_oid)` | Cross-repo crystal import; needs to fetch the commit's transitive trees + blobs |
| `bisect(repo, good_ref, bad_ref, predicate)` | For "which crystallization caused the regression" — Phase D moat |
| `read_partial_tree(repo, oid, prefix)` | Return only entries under `nodes/{oid_prefix}*` for hot-path projection without loading the whole tree |
| `git_notes_append(repo, ref, note)` | Topic threads, hot-path checkpoints, pressure logs as git-notes |
| `pack_refs(repo)` and `gc(repo)` wrappers | Periodic maintenance to keep ODB size bounded under high-frequency writes |

These additions are mostly thin wrappers around `git2::Repository` calls; the design work is the API surface, not the implementation.

---

## 7. Migration phases

Each phase is independently shippable. The system works after every phase.

### Phase 1 — Eliminate `crystals.json` and `timestamps.json` (1 week)

- Move per-node timestamps into the per-node subtree as `.ts` blob (§3.2).
- Move crystal records into commits at `refs/spectral/crystals/{oid}` (§3.5). For now, the crystal commit's tree is the same as `refs/spectral/HEAD` at crystallization time (no subgraph extraction yet).
- Update `flush()` to stop writing `*.json`; update `open()` to stop reading them.
- Migration shim: on first `open()`, if legacy `crystals.json`/`timestamps.json` exist, replay them into git and delete.

**Files touched:**
- [spectral-db/src/lib.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs) — `flush()` (line 713), `open()` legacy reads (line 313 onward)
- [spectral-db/src/crystallize.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/crystallize.rs) — `CrystalRecord` may be subsumed by commit-on-disk
- [spectral-db/src/index.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/index.rs) — `timestamps` field becomes derived from commits

**Test:** kill process between insert and flush; restart; verify no data loss after WAL replay (Phase 2 needs WAL; here, a process kill mid-flush is the scenario). Property test: roundtrip a graph with N timestamps via flush+open, assert equality.

**Success criterion:** no `*.json` files appear in `.git/spectral/` after `flush()`. Legacy migration tested with golden fixtures.

**Still on JSON after this phase:** `manifest.json` (mnesia), apache2 `graph_cache.rs` (graph.json + profile.json).

---

### Phase 2 — Write-ahead log + atomic ref update (1 week)

- Introduce `refs/spectral/wal/<session>` chain.
- Mutations append to WAL; `flush()` becomes `settle()` which squash-merges WAL into `refs/spectral/HEAD`.
- Rename `refs/spectral/head` → `refs/spectral/HEAD` (symref to `refs/spectral/heads/main`).
- Add `update_ref_atomic` to fragmentation-git.
- Replay WAL on `open()`.

**Files touched:**
- new: `spectral-db/src/wal.rs`
- new: `fragmentation-git/src/atomic.rs`
- modify: `flush()` and the message-handling path of `MemoryActor` ([spectral/src/sel/mcp/memory.rs](file:///Users/alexwolf/dev/projects/spectral/src/sel/mcp/memory.rs))

**Test:** spawn N=100 concurrent actors each inserting into the same db; verify all inserts land or fail cleanly with no torn writes; verify HEAD always points to a valid tree.

**Success criterion:** `kill -9` mid-insert leaves a recoverable state; `open()` replays WAL deterministically.

**Still on JSON:** `manifest.json`, `graph.json`, `profile.json`.

---

### Phase 3 — Migrate apache2 `graph_cache.rs` to read from git (1 week)

The apache2 layer's `graph.json` and `profile.json` are spectral-the-binary's *own* cache, separate from spectral-db's git tree (see §2.4). Eliminate this duplication: `graph_cache::load_or_build` reads `refs/spectral/HEAD` instead of `.git/spectral/contexts/graph.json`.

- Reimplement [src/apache2/graph_cache.rs](file:///Users/alexwolf/dev/projects/spectral/src/apache2/graph_cache.rs) to project from git via fragmentation-git's `read_tree`.
- Eigenvalue profile becomes a blob at `refs/spectral/HEAD:profile`; `profile_oid` is its git OID.
- Delete `dir_hash`-based cache invalidation; the git OID IS the cache key.
- The CLI `spectral status` and the MCP `memory_status` now read the same source — fixes the divergence shown in our test drive (37 vs 10 nodes).

**Files touched:**
- [src/apache2/graph_cache.rs](file:///Users/alexwolf/dev/projects/spectral/src/apache2/graph_cache.rs) — full rewrite
- [src/apache2/views.rs](file:///Users/alexwolf/dev/projects/spectral/src/apache2/views.rs) — `from_session` now takes a `&SpectralDb` or its git ref, not a `&Path` for JSON parsing
- [src/main.rs](file:///Users/alexwolf/dev/projects/spectral/src/main.rs) — CLI plumbing

**Test:** golden test: create a graph via the CLI, snapshot `git ls-tree refs/spectral/HEAD`, run `spectral status` and `mcp__spectral__memory_status`, assert both report the same node/edge counts.

**Success criterion:** no `graph.json` or `profile.json` files in `.git/spectral/contexts/` after migration. CLI and MCP report identical counts.

**Still on JSON:** `manifest.json`.

---

### Phase 4 — Crystals as first-class commits with structural parents (1 week)

Phase 1 made crystals commits but with the same tree as HEAD. Phase 4 makes them honest:

- Crystallization extracts the saturated subgraph into a *new* tree (only the relevant nodes/edges).
- Commit parents: the previous HEAD commit + every session ref that contributed mass to the crystal.
- Commit message: structured (fiedler, profile_oid, contributing_session_ids) per §3.5.
- `memory_blame {oid}` becomes `git log --follow refs/spectral/HEAD -- nodes/{oid}/`.

**Files touched:**
- [spectral-db/src/crystallize.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/crystallize.rs) — extraction logic for subgraph trees
- new: `fragmentation-git/src/walk.rs` for `walk_commits(follow_path)`

**Test:** crystallize a known subgraph; verify the resulting commit's tree has *only* the expected nodes and edges; verify parents include all sessions that touched any of those nodes.

**Success criterion:** `git log refs/spectral/HEAD -- nodes/{some_oid}/` shows the full lineage of that node, including which crystal it ended up in.

---

### Phase 5 — Manifold, optimizer, scheduler state into git-notes (1 week)

- Hot-paths checkpointed to `refs/spectral/notes/hot-paths` periodically (every N queries or on shutdown).
- Pressure events appended to `refs/spectral/notes/pressure`.
- Scheduler tick history → `refs/spectral/notes/ticks`.
- Coord_store and manifold_store coordinates → blobs in `refs/spectral/HEAD:coords` (full state, replaced atomically per settlement).

**Files touched:**
- [spectral-db/src/optimizer.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/optimizer.rs)
- [spectral-db/src/pressure.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/pressure.rs)
- [spectral-db/src/scheduler.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/scheduler.rs)
- [spectral-db/src/spectral_store.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/spectral_store.rs)
- [spectral-db/src/manifold_store.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/manifold_store.rs)

**Test:** restart the process; verify hot-path priorities, pressure history, scheduler tick state, and coord_store positions are restored bit-for-bit.

**Success criterion:** zero RAM-only state. A fresh `SpectralDb::open()` against an existing repo restores the *complete* working set (within budget) from git.

---

### Phase 6 — Eliminate `manifest.json` from mnesia (1 week, optional)

- The mnesia partition is an alternative store backend behind a feature flag ([spectral-db/src/mnesia_nif.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/mnesia_nif.rs)). It writes its own manifest.
- Migrate to either: drop mnesia entirely (it's an alternative path; the git-backed one is primary), or use a git-backed manifest blob.

**Why optional:** mnesia is feature-gated and mostly experimental. Removing it simplifies the picture; keeping it requires a parallel git-native migration.

---

## 8. Test strategy

### 8.1 Round-trip determinism

```rust
proptest! {
    #[test]
    fn graph_roundtrips_via_git(seed: u64) {
        let g = random_graph(seed, 100, 500);  // 100 nodes, 500 edges
        let tmp = tempdir();
        let db1 = SpectralDb::open(tmp.path(), SCHEMA, 0.001, 1<<24)?;
        for op in g.ops() { db1.apply(op); }
        db1.flush()?;

        let head1 = read_ref(tmp.path(), "refs/spectral/HEAD");
        let db2 = SpectralDb::open(tmp.path(), SCHEMA, 0.001, 1<<24)?;
        let g2 = db2.dump();

        prop_assert_eq!(g.canonical(), g2.canonical());
        prop_assert_eq!(head1, read_ref(tmp.path(), "refs/spectral/HEAD"));
    }
}
```

### 8.2 Cross-process determinism

Same input, two separate processes, verify identical OIDs:

```rust
#[test]
fn flush_oid_is_deterministic_across_processes() {
    let oid_a = run_in_subprocess(build_graph_and_flush);
    let oid_b = run_in_subprocess(build_graph_and_flush);
    assert_eq!(oid_a, oid_b);
}
```

### 8.3 Concurrency

```rust
#[test]
fn concurrent_inserts_are_atomic() {
    let db = SpectralDb::open(...)?;
    let handles: Vec<_> = (0..100).map(|i| {
        let db = db.clone();
        std::thread::spawn(move || db.insert("token", &format!("t{i}").as_bytes()))
    }).collect();
    let oids: Vec<_> = handles.into_iter().map(|h| h.join().unwrap().unwrap()).collect();
    db.flush()?;
    for oid in &oids { assert!(db.find(oid).is_some()); }
}
```

### 8.4 Crash recovery

```rust
#[test]
fn wal_replay_after_kill() {
    spawn_subprocess_inserts_and_kills_before_settle();
    let db = SpectralDb::open(path, ...)?;  // should replay WAL
    assert_eq!(db.status().nodes, expected_count);
}
```

### 8.5 Eigenvalue determinism

```rust
proptest! {
    #[test]
    fn eigenvalues_are_deterministic(seed: u64) {
        let g = random_graph(seed, 50, 200);
        let p1 = compute_profile(&g);
        let p2 = compute_profile(&g);
        for (a, b) in p1.values.iter().zip(p2.values.iter()) {
            prop_assert!((a - b).abs() < 1e-12);
        }
    }
}
```

### 8.6 Cross-repo OID stability

```rust
#[test]
fn same_content_yields_same_oid_in_different_repo() {
    let oid_a = SpectralDb::open(repo_a)?.insert("token", b"hello")?;
    let oid_b = SpectralDb::open(repo_b)?.insert("token", b"hello")?;
    assert_eq!(oid_a, oid_b);  // necessary for memory_cherrypick
}
```

---

## 9. The optics that collapse

Once Phase N is done, each of the new MCP tools from [docs/mcp-competitive-design.md §5](mcp-competitive-design.md) becomes a thin wrapper:

| Tool | Optic | Git command equivalent | Phase that enables it |
|---|---|---|---|
| `memory_focus` | Lens | `git rev-parse refs/spectral/HEAD` + tree summary | Phase 3 |
| `memory_get` | Lens | `git cat-file blob refs/spectral/HEAD:nodes/{oid}/.content` | already works |
| `memory_zoom` (was `memory_store`) | Prism | append-WAL → blob write | Phase 2 |
| `memory_split` (was `memory_recall`) | Traversal | tree iter + Laplacian distance | already works |
| `memory_refract` (was `memory_crystallize`) | Prism | new commit with structured parents | Phase 4 |
| `memory_diff` | — | `git diff-tree refs/spectral/HEAD~1 refs/spectral/HEAD` | Phase 4 |
| `memory_blame` | — | `git log --follow refs/spectral/HEAD -- nodes/{oid}/` | Phase 4 |
| `memory_branch` | — | `git update-ref refs/spectral/heads/{name} refs/spectral/HEAD` | Phase 2 |
| `memory_merge` | — | `git merge-tree` + spectral-distance conflict resolution | Phase 4 |
| `memory_checkout` | — | `git read-tree refs/spectral/heads/{name}` | Phase 2 |
| `memory_cherrypick` | — | fetch commit from source repo + `git cherry-pick` | new fragmentation-git fn |
| `memory_thread` | AffineTraversal | `git notes show refs/spectral/notes/topics {topic}` walk | Phase 5 |
| `memory_address` | Iso | `git hash-object --stdin -t blob` (no write) | already works |

Every row that says "Phase N" is an MCP tool whose body becomes ~5 lines. That's the collapse.

---

## 10. Risks

### R1 — libgit2 thread-safety under ractor actors

`git2::Repository` is `!Send` per libgit2's threading model. Today `SpectralDb::flush()` opens a fresh repo handle per flush ([lib.rs:719](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L719)) — that pattern works but is wasteful. Under WAL with concurrent actors, each actor needs its own repo handle, and ref updates need to use `git2::Reference::set_target` with the expected old OID for atomicity.

**Mitigation:** wrap `git2::Repository` in a thread-local; serialize ref updates through the MemoryActor (single-writer per session); make `update_ref_atomic` the only path that can advance HEAD.

### R2 — Packfile bloat under high-frequency writes

A long agent session can produce thousands of WAL commits + tree rewrites. Each is small but the count adds up. Loose-object explosion impacts disk and `git status` perf in the *parent* repo.

**Mitigation:** scheduled `git gc --auto` after every K settlements; use a git config `gc.auto = 256` for the spectral refs namespace; consider a custom packer that batches WAL commits into one pack on settle.

### R3 — Ref-update contention

If the MCP server accepts simultaneous writes (multi-agent), the CAS update of `refs/spectral/HEAD` will fail under contention.

**Mitigation:** WAL appends are non-contending (each session has its own WAL ref). Settlement is serialized by the MemoryActor (one writer at a time). For multi-agent multi-session, settlement uses `git merge-tree` + spectral-distance conflict resolution and retries on CAS miss.

### R4 — OID stability across mirror grammar bumps (resolved: mirver)

If the `.mirror`/`.conv` parser changes shape, the same content produces different OIDs. Cross-repo cherrypick would break; old crystals would become unreachable.

**Mitigation: structural semver via mirver.** A "mirver" is `spectral_hash(beta_normal(MirrorAST))` — the spectral hash of the mirror compiler's own AST in beta-normal form. The compiler proves its own version structurally; no human declares it.

Live in [`../mirror`](file:///Users/alexwolf/dev/projects/mirror):

```rust
// mirror/src/mirver.rs
pub struct Mirver(pub Sha);  // spectral hash of beta-normal MirrorAST

pub enum Compat {
    Patch,    // byte-equal after normalization (cosmetic only)
    Minor,    // strict superset of node types/shapes (additive)
    Major,    // breaking shape change
    Unknown,  // unrelated grammars
}

pub fn compatibility(a: Mirver, b: Mirver) -> Compat;
pub fn beta_normal(ast: &MirrorAst) -> NormalForm;
pub fn current() -> Mirver;  // the live compiler's mirver
```

Every spectral OID is prefixed: `<mirver_short>:<spectral_sha>` (8 hex chars of mirver + 40 hex of content). Cross-repo cherry-pick requires `compatibility(local, foreign) ∈ {Patch, Minor}`; Major requires explicit user override at the call site (`memory_cherrypick(..., allow_major=true)`).

Mirver gives semver-style pinning **mechanically**: a release can't claim Minor compatibility unless the compiler's normalized AST is a strict superset. This is a stronger guarantee than any hand-maintained version field.

### R5 — Eigenvalue determinism

`gestalt::eigenvalue::eigenvalue_profile` uses an iterative eigensolver. Convergence parameters (tolerance, max iterations) must be pinned for determinism. Floating-point reductions are not associative; threading order matters.

**Mitigation:** single-threaded eigensolve; pinned tolerance constants; golden tests for known graphs; treat any divergence as a regression test failure.

### R6 — Merge conflicts on overlapping subgraphs

When two branches edit the same node's edges and merge, three-way merge of the tree blobs needs a custom merge driver.

**Mitigation:** for edge-blob conflicts, parse both sides as `Edge` JSON and resolve by spectral-distance to the shared ancestor (closer wins). For node `.content` conflicts, prefer the side whose graph has lower self-loss. Termination proof: each conflict has a finite set of candidate values; the resolver always picks one.

### R7 — Cross-platform git2 quirks

Windows file locking blocks ref updates if `.git/refs/spectral/HEAD.lock` lingers. Symlink handling differs.

**Mitigation:** test in CI on Linux+macOS+Windows. Detect stale `.lock` files older than N seconds and clean up on `open()`.

### R8 — Storage size at scale

A million-node graph with ~10 edges/node ≈ 11M tree entries. Pure git tree representation is ~50 bytes/entry → ~550MB in tree blobs alone.

**Mitigation:** evaluate a custom packfile encoder that delta-compresses adjacent edge subtrees. Alternatively, partition the graph: `nodes/{oid_prefix_2}/{oid}/` instead of flat `nodes/{oid}/`, so each subtree fits in libgit2's working memory. Worst-case: a million nodes is well above an interactive agent's working set; design for 10K active + 90K cold.

### R9 — Loss of provenance for cross-repo crystals

A crystal cherry-picked from another repo carries its commit message but its parents may not exist in the destination repo. `git log` will show a dangling history.

**Mitigation:** `memory_cherrypick` rewrites the commit to have a single synthetic parent (the destination HEAD) and stores the original parent OIDs in a git-note `refs/spectral/notes/cherrypick-origin`. Lineage is preserved as data, not as commit graph.

### R10 — Existing JSON consumers

Several files outside spectral-db read the JSON directly:
- [src/apache2/graph_cache.rs](file:///Users/alexwolf/dev/projects/spectral/src/apache2/graph_cache.rs) reads its own graph.json (Phase 3 fixes this)
- [src/sel/tui.rs](file:///Users/alexwolf/dev/projects/spectral/src/sel/tui.rs) loads StatusView from disk (Phase 3)
- benches under [benches/](file:///Users/alexwolf/dev/projects/spectral/benches/) may snapshot JSON

**Mitigation:** grep `serde_json::from_slice.*graph` and `\.json` after each phase; update consumers as part of the same PR.

---

## 11. Open questions

### Q1 — One commit per insert, or one per settlement? (resolved: per-settlement)

**Per-settlement** on `refs/spectral/HEAD`. The WAL keeps per-insert granularity for blame within a session. The settlement commit's message lists the WAL summary; the WAL ref is retained per Q9 (configurable; default keep).

### Q2 — Symref or direct ref for `refs/spectral/HEAD`?

Symref enables `memory_branch` / `memory_checkout` to behave like git branching. Direct ref is simpler but doesn't compose with branches.

Recommendation: symref. Cost is negligible; benefit is large.

### Q3 — Where do per-session ephemeral artifacts live?

Hot-path stats, pressure events, scheduler ticks — should they be in HEAD's tree (always-on) or in git-notes (out-of-band)?

Recommendation: git-notes for high-frequency append-only data; HEAD tree for state that must round-trip on read.

### Q4 — How aggressive is GC?

Frequent GC keeps the ODB small but may interrupt active sessions. Lazy GC means storage grows.

Recommendation: time-based — GC if last GC was >1 hour ago AND >100 settlements have occurred. Defer to git's `gc.auto`.

### Q5 — Should crystals share trees with HEAD or have isolated subgraphs?

Sharing is space-efficient (git dedup) but couples crystal lifetimes to HEAD. Isolated subgraphs allow crystal export.

Recommendation: subgraph extraction (Phase 4 §7). Git's content-addressing dedups identical blobs across crystals automatically; the cost is one extra tree write per crystal. The gain is portability.

### Q6 — Do we expose the raw git refs to the agent?

**Resolved.** Refs are addresses, not actions. The optics are the only verbs.

Concretely:
- The agent can pass any git revspec as a parameter to an optic: `memory_checkout("HEAD~5")`, `memory_diff("heads/wild-idea", "HEAD")`, `memory_blame("crystals/abc123")`. Git's revspec language becomes the agent's pointer language for free — `HEAD~N`, `commit^`, `branch:topic`, `crystal:oid`, abbreviated SHAs.
- A read-only `memory_show(ref)` inspects any ref's tree, profile, commit message, parents — pure projection, no side effects.
- **No** `git_command(args)` escape hatch. **No** raw `update-ref`, `commit`, `gc`, `cherry-pick`. Writes only via `memory_zoom` / `memory_refract` / `memory_branch` / `memory_merge` / `memory_cherrypick`.

The wedge: the agent gets git's expressive addressing without inheriting git's destructive verbs. `memory_checkout("HEAD~5")` is exact and reversible; `git reset --hard` does not exist in this surface.

### Q7 — Replace `SpectralDb`, or wrapper-first? (resolved: replace; in-memory shape becomes the test adapter)

The git-native projection becomes the production `SpectralDb`. The current `Mutex<SpectralIndex>` shape is preserved as a separate **test adapter** under `spectral-db/src/test_support/in_memory.rs` (alongside the existing [test_support.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/test_support.rs)).

- Production code path: pure git, no Mutex-only state.
- Tests can opt into the in-memory shape via `cfg(any(test, feature = "test-support"))` for speed and isolation.
- Both implement the same trait surface; tests that exercise the API contract run against both backends.

### Q8 — Single-writer or per-session WAL with merge-on-settle? (resolved: single-writer first, multi-writer shape commented in)

Phase 2 ships **single-writer**: one `MemoryActor` owns all writes. Each settlement is serialized through that actor.

The multi-writer shape (per-session WAL refs, `git merge-tree` + spectral-distance conflict resolution at settlement) is **stubbed in source comments** at the call sites that will need to change:

```rust
// MULTI-WRITER TODO: replace with per-session WAL ref `refs/spectral/wal/<session_id>`.
// See R6 for the spectral-distance merge driver and Phase 2.5 in the plan.
```

This way the implementer reading the code in six months knows exactly where the multi-writer story plugs in, without paying the merge-driver cost up-front.

### Q9 — WAL retention after settlement (resolved: configurable, default keep)

Three retention strategies; one ships fully, two are stubbed:

```rust
pub enum WalRetention {
    Keep,                   // default: WAL refs persist forever; full per-mutation blame
    Prune,                  // (stub) delete WAL ref on settle; intra-session blame lost
    FoldIntoNotes,          // (stub) WAL becomes a git-note attached to the settlement commit
}
```

Default is `Keep`. `Prune` and `FoldIntoNotes` are valid enum variants but their implementations are `unimplemented!()` with a TODO comment pointing back here. This lets us ship the storage-conservative path later without re-reviewing the whole flow.

### Q10 — Empty-repo semantics (resolved: auto-create empty commit)

`SpectralDb::open()` against a fresh project (no `refs/spectral/HEAD`) auto-creates an empty initial commit on the symref `refs/spectral/HEAD → refs/spectral/heads/main`. Tree is `{}`, message is `spectral: init`, no parents. The MCP boot path treats this as the natural zero state. Anything else (lazy-create on first write, error-on-missing) was deemed overly complex.

### Q11 — Mirver implementation (shipped in-session, pre-Phase-1)

`mirror::mirver` lives at [/Users/alexwolf/dev/projects/mirror/src/mirver.rs](file:///Users/alexwolf/dev/projects/mirror/src/mirver.rs). 11 tests pass.

Public API:
- `Mirver { compiler: Oid, grammars: Oid }` — layered version per agreement (compiler-shape ⊕ active-grammar fingerprint).
- `Mirver::current()` — running compiler, no grammars.
- `Mirver::with_grammars(&[Ast])` — running compiler, given parsed grammars.
- `Mirver::short()` → 16-char hex prefix for OIDs.
- `Compat::{Patch, Minor, Major, Unknown}` and `compatibility(a, b)`.
- `compiler_oid()`, `grammars_oid(&[Ast])`, `empty_grammars_oid()`, `beta_normal(&Ast)`.
- `COMPILER_VERSION_TAG` const — maintainer-bumped on `Ast` shape changes.

Phase-1 simplifications (documented in source):
- `beta_normal` is identity (no alpha/beta/eta yet); canonicalization is byte-stable AST emission via `emit_canonical`.
- `Compat::Minor` is never returned (TODO: structural superset detection); any non-Patch is conservatively `Major`.
- `compiler_oid` derives from a string constant, not from automated AST-shape introspection.

These are sound conservative defaults: false `Major` verdicts are tolerable; false `Patch` would corrupt the cross-repo OID guarantee and is precluded by construction.

---

## 12. Appendix: example session walkthrough

Pseudo-bash showing what runs under the hood when the agent calls each MCP tool.

### memory_zoom (was memory_store)

Agent calls `memory_zoom(node_type="observation", content="alex prefers dark mode")`.

```bash
# 1. Compute spectral content OID via fragmentation::sha::HashAlg
oid=$(echo -n "observation:alex prefers dark mode" | spectral hash-blob)
# → cf3064c9fcafa6560ee1aa36284ff30680db9ca3

# 2. Write the node blob to the ODB
git -C $repo hash-object -w --stdin <<< $node_blob_format
# → 5b2c... (git OID; equal to spectral OID for native content)

# 3. Append a mutation commit to the WAL
git -C $repo update-ref refs/spectral/wal/$session_id $new_wal_commit_oid

# 4. Update projection cache (RAM only) if the budget allows.
```

Returns: `cf3064c9fcafa6560ee1aa36284ff30680db9ca3` (the spectral OID).

### memory_split (was memory_recall)

Agent calls `memory_split(oid="cf30...", distance=2.5)`.

```bash
# 1. Resolve the projection at HEAD.
head=$(git -C $repo rev-parse refs/spectral/HEAD)

# 2. Cache lookup or read the node's subtree from git.
git -C $repo ls-tree $head -- nodes/cf30...

# 3. Compute Laplacian distance from cf30... to each candidate.
#    Candidates are pinned in the projection. For cold nodes, stream from git tree.

# 4. Filter by distance ≤ 2.5; return.
```

Returns the list of (oid, distance, type) tuples shown earlier.

### memory_refract (was memory_crystallize)

Triggered by the cascade (eigenvalues stable for N ticks) or by an explicit agent call.

```bash
# 1. Identify the saturated subgraph (algorithm in spectral-db/src/crystallize.rs).
subgraph_oids=("cf30..." "ae3d..." "47a3...")

# 2. Write a subgraph-only tree.
crystal_tree=$(git -C $repo write-tree --subset=${subgraph_oids[@]})

# 3. Find contributing session refs.
sessions=("refs/spectral/sessions/2026-04-28T13:05:00")

# 4. Create the crystal commit with HEAD + sessions as parents.
crystal_oid=$(git -C $repo commit-tree $crystal_tree \
    -p $head \
    -p $(git rev-parse refs/spectral/sessions/2026-04-28T13:05:00) \
    -m "crystal: dark-mode preference\n\nfiedler: 0.0733\nnodes: 3\nedges: 5\nprofile: $profile_oid")

# 5. Pin the crystal.
git -C $repo update-ref refs/spectral/crystals/$crystal_oid $crystal_oid
```

Returns: `crystal_oid`.

### memory_checkout

Agent calls `memory_checkout(ref="HEAD~5")`.

```bash
# Read-only — does not mutate refs/spectral/HEAD.
target=$(git -C $repo rev-parse refs/spectral/HEAD~5)
git -C $repo ls-tree -r $target  # full graph at that point in time
git -C $repo show $target:profile  # eigenvalue profile then
```

Returns: the historical eigenboard. The agent can compare against current HEAD.

### memory_cherrypick

Agent calls `memory_cherrypick(source_repo="../project-a", oid="abc123...")`.

```bash
# 1. Fetch the source's spectral refs into a unique remote namespace.
git -C $repo remote add cp_src ../project-a
git -C $repo fetch cp_src "refs/spectral/crystals/abc123*:refs/spectral/cherrypick/abc123*"

# 2. Cherry-pick the commit (rewrites parents to current HEAD; preserves tree).
git -C $repo cherry-pick refs/spectral/cherrypick/abc123 --strategy=ours

# 3. Record provenance.
git -C $repo notes --ref=refs/spectral/notes/cherrypick-origin add \
    -m "from ../project-a@abc123" $new_local_oid

# 4. Remove the temp remote.
git -C $repo remote remove cp_src
```

Returns: the local OID that now owns the cherry-picked crystal.

### memory_blame

Agent calls `memory_blame(oid="cf30...")`.

```bash
git -C $repo log --follow --format="%H %s" refs/spectral/HEAD -- nodes/cf30.../
# → list of commits that touched this node:
#   abc123  spectral: 37 nodes, 293 edges
#   def456  crystal: dark-mode preference
#   ...
```

Returns: lineage as a list of {commit, message, timestamp, fiedler_at_commit}.

### memory_diff

Agent calls `memory_diff(from="HEAD~1", to="HEAD")`.

```bash
git -C $repo diff-tree -r refs/spectral/HEAD~1 refs/spectral/HEAD
# → a list of tree-entry diffs:
#   M  nodes/cf30.../.ts
#   A  nodes/cf30.../{ae3d...}
#   M  profile
```

Returns: structured added/removed/changed nodes and edges.

---

*Twelve sections. Schema spec, projection trait, six-phase migration, eight risks, six open questions, six worked examples. The thesis: spectral-db is 60% git-native today; finishing the job is mechanical, not architectural; once done, the named optics in the MCP surface become wrappers over `git` commands the operating system already runs.*
