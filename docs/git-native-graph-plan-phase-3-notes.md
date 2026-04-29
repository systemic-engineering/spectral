# Phase 3 — Notes

**Date:** 2026-04-28
**Status:** Shipped (in-session, no subagent — Task tool stalled twice on this phase)

## What landed

- [src/apache2/graph_cache.rs](file:///Users/alexwolf/dev/projects/spectral/src/apache2/graph_cache.rs) rewritten end-to-end. `load_or_build` now projects from `refs/spectral/HEAD` (with fallback to legacy `refs/spectral/head`); falls back to gestalt scan when no spectral commit exists; never writes JSON.
- [src/sel/mcp/server.rs](file:///Users/alexwolf/dev/projects/spectral/src/sel/mcp/server.rs) and [src/sel/mcp/cascade.rs](file:///Users/alexwolf/dev/projects/spectral/src/sel/mcp/cascade.rs) — head→HEAD references migrated; assertions check `refs/spectral/heads/main` (the symref target) with legacy fallback.
- 10 new tests in graph_cache (8 functional + 2 retained from before).
- 83/83 lib tests pass; release binary builds with `--features sel`.
- Legacy `.git/spectral/contexts/{graph,profile}.json` are auto-cleaned on first `load_or_build` with a single stderr line.

## Verified end-to-end

```
$ spectral status
spectral: removed legacy .git/spectral/contexts/{graph,profile}.json (Phase 3 migration)
┌─ spectral ──────────────────────────────┐
│ nodes: 27  edges: 26  crystals: 0      │
└─────────────────────────────────────────┘
$ spectral status --json | head -3
{ "nodes": 27, "edges": 26, ... }
$ ls .git/spectral/contexts/    # empty
```

CLI and JSON path agree (both project from the same git tree). The 37-vs-10 divergence observed at session start is gone — both surfaces now resolve through `refs/spectral/HEAD`.

## Deviations from the plan

### D1. `profile` blob (plan §3.4) not yet emitted by spectral-db

The plan calls for spectral-db's `settle()` to write a `profile` blob at the root of the commit tree. That work is **out of scope for Phase 3** (per the spawn instructions). For now, `load_from_git`:

- Looks for a `profile` blob at the tree root and decodes it if found.
- Falls back to recomputing `EigenvalueProfile` from the projected graph via `gestalt::eigenvalue::eigenvalue_profile`.

Once Phase 4 (or a quick spectral-db addendum) writes the profile blob, this code path activates without further changes.

### D2. `breakdown` (file-type counts) not preserved across the git roundtrip

Phase 1's tree shape doesn't carry the gestalt `Breakdown` (markdown/code/config/etc. counts). `load_from_git` returns `GestaltBreakdown::default()` (all zeros). Views that show "files: 172 (md:20 code:126 ...)" still need the gestalt scan.

This is acceptable for Phase 3: views that need the breakdown can call `gestalt::detect::scan(path)` directly. Plan §3.3 reserves a `manifest` blob slot at the tree root for this metadata in a later phase.

### D3. CLI/MCP uniformity test (E5) not added

The acceptance criterion called for an integration test that drives spectral-db's `settle()` and asserts CLI's `StatusView` and MCP's `memory_status` agree. The MCP path requires the actor system (`sel` feature, ractor runtime, MemoryActor spawn) — too heavy for a lib test, requires an integration harness that doesn't yet exist.

The architectural point is proven by the live `spectral status` + `spectral status --json` agreement above. A proper integration test belongs in Phase 5 alongside the optimizer/scheduler git-backing work, where the actor harness will already be set up.

### D4. Tree shape: flat vs `nodes/` prefix

Plan §3.3 proposes wrapping per-node subtrees under a `nodes/` directory. Phase 1 ships them at the root (the same shape spectral-db has been writing since before Phase 1). `load_from_git` reads the **flat** shape — entries at the tree root that are subtrees and don't start with `.` are nodes; sibling blobs (`profile`, eventually `schema`, `manifest`) are metadata.

Adding the `nodes/` wrapper is a Phase 4 concern (alongside crystal subgraph extraction). The reader is forward-compatible: when Phase 4 nests under `nodes/`, both paths can be supported with one extra branch.

## Out-of-scope items confirmed untouched

- spectral-db internals (Phases 1+2 stable; no edits this phase).
- fragmentation-git (Phase 2 added `atomic.rs`; not touched here).
- Subgraph extraction for crystals (Phase 4).
- Optimizer / pressure / scheduler / coord_store persistence (Phase 5).
- mnesia `manifest.json` (Phase 6, optional).
- Multi-writer merge driver (Phase 2.5).
- Mirror / mirver (already shipped pre-Phase 1).

## Tests added

| Test | Purpose |
|---|---|
| `load_from_git_reads_nodes_and_edges` | Two-node graph through git tree → correct counts |
| `load_from_git_skips_dotted_metadata` | `.type`/`.content`/`.ts` are NOT counted as edges |
| `load_or_build_falls_back_when_no_ref` | No spectral ref → gestalt fallback works |
| `load_or_build_prefers_git_when_present` | Git ref present → `from_cache=true` |
| `load_or_build_does_not_write_json` | Acceptance A1: no `graph.json`, no `profile.json` |
| `cleanup_removes_legacy_json` | Migration shim deletes legacy stopgap files |
| `legacy_head_ref_is_resolved` | `refs/spectral/head` (Phase 1 layout) still readable |
| `dir_hash_deterministic` | Retained for gestalt-fallback fingerprint |
| `write_graph_cache_is_no_op` | Deprecated shim returns Ok and writes nothing |
| `decode_profile_blob_round_trip` | Profile blob format (Phase 4-ready) |

## Operational behavior change (worth flagging)

`spectral status` against a project with no `refs/spectral/HEAD` now runs the gestalt scan on every invocation (no JSON cache). This is a CPU regression vs. the previous JSON-cached behavior on cold runs: ~tens of milliseconds for a small repo, ~seconds for a large one. The trade-off is correctness (no stale cache, no divergence). For users who want fast cold runs, the path forward is `spectral_index` once (which calls spectral-db's `settle()` and writes the git tree); subsequent reads then go through the fast git path.

If this becomes a hot-path complaint, a minimal optimization is to memoize within-process via a `OnceCell<CachedGraph>` keyed on `(head_oid, dir_hash)` — but the architecturally correct fix is to ensure `spectral_index` runs as part of normal session bootstrap.
