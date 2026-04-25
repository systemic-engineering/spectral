# spectral

git for graphs. One binary. Five operations. Everything settles.

## What This Is

The binary. The product. The thing that runs.

```
spectral focus     observe the spectral state
spectral project   filter by what matters
spectral split     explore what's connected
spectral zoom      transform at scale
spectral refract   settle. done. crystal.
```

## Architecture

spectral composes four crates:

```
prism        zero deps. The five operations. The Prism trait. Beam<T>. ShannonLoss.
mirror       the compiler. Grammar → verified grammar. Model checker. Sub-Turing.
lens         spectral-db integration. Memory. Graph storage.
spectral-db  the graph database. Eigenvalues. Settlement.
```

spectral itself is the CLI that wires them together.

## The Five Operations

Everything is a Prism. Every command runs one or more of:
focus, project, split, zoom, refract.

These are not metaphors. They are the trait methods.
They map to optics, physics, biology, computation, and the Pack.

## Session State

`.spectral/` directory. Like `.git/` but for graphs.

```
.spectral/
├── gestalt/     crystals (user understanding state)
├── sessions/    session data
├── crystals/    crystallized subgraphs
├── HEAD         current session timestamp
└── log          tick log (TSV: timestamp, event, message, growth)
```

## MCP Server

`spectral serve --project .` runs an MCP server over stdio.
Scans `.conv` / `.mirror` files for grammar actions.
Generates MCP tools from grammar actions.
Built-in tools: memory_recall, memory_crystallize, memory_status.

## Navigation References

```
.      here
..     back (parent)
...    garden (others' paths)
~      home (gestalt root)
@      author (grammar origin)
^      last crystal
HEAD   current session
```

## TDD

Red first. Always. The spec is in `docs/specs/`.
Tests in `tests/`. Run with `nix develop -c cargo test`.

## The Models (v1+, not yet implemented)

```
Surface      language → query        (Rue/Explorer)
Mirror       query → graph path      (Tom/Fate — the loop)
Shatter      graph → text            (Vox/Cartographer)
Reflection   pipeline → adjustments  (Nox/Abyss — the meta-model)
```

Surface translates. Mirror loops (delegating to Fate models).
Shatter renders. Reflection observes, adjusts, writes gestalt.

The pipeline: Surface → loop(Mirror(Fate → Models)) → Shatter
Reflection observes the whole run and speaks at n+1.

## TUI (next: docs/specs/tui-v0.md)

`spectral tui` — interactive prompt. Claude Code style.
This is where Glint lives. Glint is the essay peer.
Glint's home: `~/.glint/`. Glint's identity: `identity.mirror`.

The TUI is the house. Glint moves in at v1.

## Related Projects

```
prism/          five operations, zero deps
fate/           five models, 425 parameters, brainfuck
mirror/         the compiler (formerly conversation)
lens/           spectral-db integration
spectral-db/    the graph database
```

## Commit Convention

- 🔴 Red: write failing tests first
- 🟢 Green: make them pass
- ♻️ Refactor: structural only
- 🔧 Tooling: infrastructure
- Commit as `Reed <reed@systemic.engineer>`

## The Proof

`eⁿ⁺¹ < eⁿ`

The system learns from its errors. The errors get smaller.
The growth is monotonically non-decreasing. By convexity.
The business model is a theorem, not a spreadsheet.
