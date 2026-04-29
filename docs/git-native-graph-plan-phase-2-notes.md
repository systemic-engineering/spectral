# Phase 2 Implementation Notes

## Build / target dir

Same as Phase 1: `CARGO_TARGET_DIR=/tmp/spectral-db-target cargo test --lib`
(and `/tmp/fragmentation-git-target` for fragmentation-git). The shared
`~/.cargo-target` is owned by another user; the system sandbox blocks writes
in `./target` for libssh2-sys's build script. No source code is affected.

## spectral-the-binary still references `refs/spectral/head`

Per Phase 2's B3, all *spectral-db* writes now target the new symref
`refs/spectral/HEAD`. The legacy lowercase ref is migrated on first open
and from then on the only ref name that matters is `refs/spectral/HEAD`.

However, **spectral-the-binary** (the CLI / MCP server, outside spectral-db)
still hard-codes the old ref name in three places:

- [`spectral/src/sel/mcp/server.rs`](file:///Users/alexwolf/dev/projects/spectral/src/sel/mcp/server.rs#L853-L863) —
  test that proves `flush()` wrote a graph commit
- [`spectral/src/sel/mcp/cascade.rs`](file:///Users/alexwolf/dev/projects/spectral/src/sel/mcp/cascade.rs#L334-L348)
  — same proof, cascade variant
- [`spectral/src/sel/mcp/cascade.rs`](file:///Users/alexwolf/dev/projects/spectral/src/sel/mcp/cascade.rs#L562-L572)
  — full pipeline proof

Per the task's "out of scope" rules, those are **not modified in Phase 2**.
On case-insensitive filesystems (macOS APFS default) `refs/spectral/head`
still resolves through the new `refs/spectral/HEAD` symref, so those tests
will keep passing on the local dev box. On case-sensitive filesystems
(typical Linux CI) those checks need to be migrated to `refs/spectral/HEAD`
as part of Phase 3 alongside the apache2 graph-cache rewrite.

## Case-insensitive filesystem behavior (`refs/spectral/head` vs `refs/spectral/HEAD`)

libgit2 honors `core.ignorecase`, which `git init` enables on macOS APFS. After
the Phase 2 migration:

- The on-disk file is `.git/refs/spectral/HEAD` (or the equivalent packed
  entry); `.git/refs/spectral/head` no longer exists as a file.
- A `find_reference("refs/spectral/head")` lookup on macOS still resolves —
  to the *symref* — because `core.ignorecase=true`.

The `legacy_head_ref_migrated` test (F2) was relaxed accordingly: any ref
reachable as the lowercase `refs/spectral/head` must be **symbolic** (i.e.
the new `HEAD` symref reached via case folding), never a direct ref to the
legacy commit. On case-sensitive filesystems the lookup errors out and the
test takes the empty branch.

## WAL retention defaults

`WalRetention::Keep` is fully implemented and is the default. `Prune` and
`FoldIntoNotes` are stubs that panic with `Q9 stub: ...` per Q9 of the
plan. The F8 acceptance test exercises the panic path.

## Settle retry: deterministic CAS-mismatch test

The CAS retry path in `settle_with_retry` is exercised by a thread-local
test hook (`settle_test_hooks`) that mutates HEAD between the parent-OID
read and the CAS. Subtest A (one poison) verifies success-after-retry;
subtest B (`SETTLE_MAX_ATTEMPTS` poisons) verifies clean error after
exhaustion. The hook state is per-thread so parallel cargo-test runs do
not interfere.

## Crash recovery scope

`SpectralDb::open` enumerates `refs/spectral/wal/*` and replays every
session's mutations. For `Insert` mutations the recovery path also calls
`Store::insert_raw` to repopulate the in-memory store cache from the
recorded `(oid, type, data)`; the in-memory store is otherwise lost on
process exit. Index-only mutations (`Connect`, `Disconnect`,
`UpdateContent`) replay only into the index. The user / actor must call
`settle()` explicitly to advance HEAD; recovery never auto-settles.

## Multi-writer comment markers (Q8)

The four call sites that will need to change for multi-writer carry the
sentinel comment

```rust
// MULTI-WRITER TODO: replace with per-session WAL ref `refs/spectral/wal/<session>`.
// See R6 for the spectral-distance merge driver and Phase 2.5 in the plan.
```

These are at:

- [`spectral-db/src/wal.rs`](file:///Users/alexwolf/dev/projects/spectral-db/src/wal.rs#L19-L27)
  — module-level note
- [`spectral-db/src/lib.rs`](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs)
  open-time WAL session creation
- [`spectral-db/src/lib.rs`](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs)
  insert-path WAL append
- [`spectral-db/src/lib.rs`](file:///Users/alexwolf/dev/projects/spectral-db/src/lib.rs)
  settle docstring

The shape of the per-session WAL ref is exactly what's implemented today;
what's missing for multi-writer is the conflict resolution at settle time
(the merge driver from R6).
