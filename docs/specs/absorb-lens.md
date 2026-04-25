# Absorb Lens into Spectral

> Lens is spectral's memory, not a standalone crate.
> Move it inside. Clean the dependency graph.

## Why

Every consumer of lens is inside spectral:
- `serve.rs` uses Lens for MCP tool dispatch
- `memory.rs` uses Lens for CLI memory commands
- `tui.rs` (upcoming) will use Lens for the prompt session
- No external crate depends on lens

Lens is not a product. It's a module.

## Before

```
Crate graph:
  prism (zero deps)
  spectral-db → prism
  lens → spectral-db, prism
  mirror → prism
  fate → prism
  spectral → mirror, lens, prism, spectral-db, conversation

spectral/Cargo.toml:
  lens = { path = "../lens" }
```

## After

```
Crate graph:
  prism (zero deps)
  spectral-db → prism
  mirror → prism
  fate → prism
  spectral → mirror, fate, prism, spectral-db

spectral/Cargo.toml:
  # lens removed
  # spectral-db used directly

spectral/src/
  lens/
    mod.rs          — pub struct Lens, open, store, recall, crystallize
    filter.rs       — GrammarFilter
    types.rs        — Distance, NodeData, NodeType
    compose.rs      — user + project graph composition
    pressure.rs     — pressure management, shedding
```

## Migration Steps

```
1. Copy lens/src/*.rs → spectral/src/lens/
2. Add `pub mod lens;` to spectral/src/main.rs (or lib.rs if we split)
3. Update all `use lens::` imports in spectral to `use crate::lens::`
4. Remove `lens = { path = "../lens" }` from spectral/Cargo.toml
5. Add any lens dependencies directly to spectral/Cargo.toml
   (spectral-db is already there, prism is already there)
6. Run tests: `nix develop -c cargo test`
7. Verify MCP serve still works: `spectral serve --project .`
8. Verify memory commands still work: `spectral memory status`
9. Archive lens/ crate (don't delete — move to archive/ or tag last commit)
```

## What Stays Separate

```
prism       — zero deps, the foundation, everyone uses it
spectral-db — standalone database, could be used by others
mirror      — the compiler, standalone tool
fate        — the models, standalone inference
```

These are products or foundations. Lens was plumbing.

## What Changes in serve.rs

```rust
// Before:
use lens::types::{Distance, NodeData, NodeType};
use lens::Lens;
use lens::filter::GrammarFilter;

// After:
use crate::lens::types::{Distance, NodeData, NodeType};
use crate::lens::Lens;
use crate::lens::filter::GrammarFilter;
```

Same code. Different import paths. Nothing else changes.

## What Changes in memory.rs

Same pattern. `use lens::` → `use crate::lens::`.

## Risk

Low. This is a move, not a rewrite. The code doesn't change.
The imports change. The Cargo.toml simplifies. Tests verify.

## Tests

All existing lens tests move to `spectral/src/lens/` as module tests
or to `spectral/tests/`. The test count must not decrease.

```
Before:  lens has N tests
After:   spectral has N + existing spectral tests
```

## Build Order

1. Copy files
2. Fix imports
3. Update Cargo.toml
4. Run tests (must be green before proceeding)
5. Verify MCP serve
6. Verify memory CLI
7. Archive old lens crate
8. Update CLAUDE.md (remove lens from dependency list)

One PR. One migration. No behavior change.
