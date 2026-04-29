# Phase 4 — Notes

**Date:** 2026-04-28
**Status:** Shipped (subagent wrote the code; timed out at the reporting stage; verified post-hoc in-session).

## What landed

### spectral-db

- New: [src/subgraph.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/subgraph.rs) (229 lines)
  - `SubgraphSpec` struct + `SubgraphSpec::from_crystal(crystal)` — node-set extraction.
  - `write_subgraph_tree(repo, store, index, spec, profile)` — writes a tree containing only the spec's nodes (each with `.type`/`.content`/`.ts`/`.meta`) and edges to **other nodes in the subgraph**. Edges leaving the subgraph are dropped — true subgraph extraction.
- New: [src/profile.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/profile.rs) (166 lines)
  - `compute_profile(index)` — eigenvalues, fiedler, graph-hash trailer.
  - `encode_profile_blob(index)` / `encode_profile_bytes(values, fiedler, nodes, edges, trailer)` — emits the `spectral-profile\0` blob format per plan §3.4.
  - `decode_eigenvalues(bytes)` — round-trip helper.
- Modified: [src/lib.rs](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs) (+1841/-140)
  - `write_graph_tree` now emits a `profile` blob at the root of the graph tree.
  - Crystal emit at `refs/spectral/crystals/<hash>` now uses `subgraph::write_subgraph_tree` and `fragmentation_git::commit::write_commit_with_parents` with `[HEAD, session_refs...]` parents.
  - Crystal commit message extended with structured key:value lines (`fiedler:`, `nodes:`, `edges:`, `profile:`, `sessions:`); round-trip via `format_crystal_commit_message` / `parse_crystal_commit_message_with_meta` preserved.
  - Session refs `refs/spectral/sessions/<session_id>` advance with each settle.
  - Parent dedup: skip `session_ref` if it equals HEAD (git rejects duplicates).

### fragmentation-git

- New: [src/walk.rs](file:///Users/alexwolf/dev/projects/fragmentation-git/src/walk.rs) (195 lines)
  - `walk_commits_following(repo, start_ref, path_prefix)` — backing for `memory_blame`. Returns commits whose tree changes under `path_prefix`. libgit2 Revwalk + tree-diff.
- Modified: [src/commit.rs](file:///Users/alexwolf/dev/projects/fragmentation-git/src/commit.rs) — added `write_commit_with_parents(repo, tree, parents, sig, message)` supporting 0..N parents.
- Modified: [src/git.rs](file:///Users/alexwolf/dev/projects/fragmentation-git/src/git.rs), [src/lib.rs](file:///Users/alexwolf/dev/projects/fragmentation-git/src/lib.rs), [src/namespaced.rs](file:///Users/alexwolf/dev/projects/fragmentation-git/src/namespaced.rs) — minor adjustments for new module wiring.

### spectral

No code changes required this phase. [src/apache2/graph_cache.rs](file:///Users/alexwolf/dev/projects/spectral/src/apache2/graph_cache.rs) was made forward-compatible in Phase 3 — it now picks up the `profile` blob automatically (previously fell back to recomputing eigenvalues; now reads the blob spectral-db wrote).

## Test results

| Crate | Tests passing | Net new since Phase 3 |
|---|---|---|
| spectral-db (lib) | **352 / 352** | +14 |
| fragmentation-git (lib) | **26 / 26** | +3 |
| spectral (lib) | **84 / 84** | (Phase 3 unchanged) |

## Acceptance coverage

- ✅ A. Subgraph extraction (`subgraph::write_subgraph_tree`, `SubgraphSpec::from_crystal`)
- ✅ B. Multi-parent commit (`fragmentation_git::commit::write_commit_with_parents`)
- ✅ C. Contributing session refs (`refs/spectral/sessions/<id>` advance per settle; dedup vs HEAD)
- ✅ D. Profile blob in tree (encoded + emitted; Phase 3 reader decodes automatically)
- ✅ E. Structured crystal commit message (keys: fiedler, nodes, edges, profile, sessions)
- ✅ G. `walk_commits_following` (the `memory_blame` plumbing)

## Phase 4b — also shipped (verified post-hoc)

- ✅ F. `nodes/` wrapper on tree shape. Verified in-session that the Phase 4 subagent shipped both halves before timing out:
  - spectral-db's `write_graph_tree` ([lib.rs L376-L390](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L376-L390)) wraps per-node entries under `nodes/` and emits `profile` as a sibling.
  - spectral-db's `restore_edges_from_git_tree` ([lib.rs L416-L433](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs#L416-L433)) reads either `nodes/` or the legacy flat layout.
  - spectral's [src/apache2/graph_cache.rs L105-L123](file:///Users/alexwolf/dev/projects/spectral/src/apache2/graph_cache.rs#L105-L123) does the same on the consumer side, plus a new `graph_cache_reads_nodes_wrapper` test (11 tests in graph_cache; was 10 in Phase 3).
  - All Phase 1-3 trees still load via the fallback path (`legacy_head_ref_is_resolved` continues to pass).

## Deviations / footnotes

1. **Subagent timed out at the reporting stage.** Code landed cleanly; only the `final report` message was lost to timeout. Verified by reading the new files, running tests, and confirming the binary still works end-to-end.
2. **Graph-hash trailer.** `compute_profile` emits a non-zero 32-byte trailer derived from the index. If the existing `convergence::GraphHash` plumbing diverges from this in future work, reconcile to one canonical source.
3. **Phase 3 `breakdown` zeros remain.** Phase 4 did not address the gestalt `Breakdown` round-trip (md/code/config counts after restoring from git). That remains a Phase 5 concern alongside optimizer/scheduler/coord_store persistence.
4. **CLI/MCP uniformity integration test (E5 from Phase 3 plan)** still deferred. Architectural point already proven by `spectral status` ↔ `spectral status --json` agreement; full actor-harness test belongs in Phase 5.

## Live verification

```
$ cd /Users/alexwolf/dev/projects/spectral
$ spectral status
┌─ spectral ──────────────────────────────┐
│ nodes: 27  edges: 26  crystals: 0      │
│ loss: 0.000 bits  tension: 0.0000      │
│ growth: 0%  cached: 1                  │
│ hot paths: 0  queries: 0               │
└─────────────────────────────────────────┘
```

CLI reads the same `refs/spectral/HEAD` tree spectral-db's settle now writes with the new profile blob and (when crystals exist) multi-parent crystal commits.
