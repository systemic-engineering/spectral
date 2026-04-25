# Grammar Codegen — spectral/mirror/*.mirror → Generated Packages

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Write `.mirror` grammars in `spectral/mirror/`, write `spectral.spec` declaring targets, generate Rust and Gleam packages. No hand-written bridge code.

**Architecture:** Grammars are source of truth. `.spec` file declares what @code/X grammar generates which package. `mirror craft spectral.spec` generates all targets. `generated/` directories are output, never edited. The grammar OID IS the version.

**Tech Stack:** mirror compiler (with @code pipeline from the mirror plan), Gleam, Rust

**Working directory:** `/Users/alexwolf/dev/projects/spectral/`
**Branch:** `reed/grammar-codegen`
**Depends on:** mirror `reed/emit-code` branch (must be merged first)
**Build:** `CARGO_TARGET_DIR=/tmp/spectral-codegen cargo check --workspace` + `cd beam/admin && gleam check`

---

## Prerequisites

The mirror compiler plan (`mirror/docs/superpowers/plans/2026-04-15-emit-code-pipeline.md`)
must be complete. Specifically: `mirror craft --target rust` and `mirror craft --target gleam`
must work for arbitrary `.mirror` source files.

---

### Task 1: Write spectral/mirror/ grammars

**Files:**
- Create: `mirror/prism.mirror`
- Create: `mirror/gestalt.mirror`
- Create: `mirror/witness.mirror`
- Create: `mirror/fate.mirror`
- Create: `mirror/admin.mirror`

- [ ] **Step 1: prism.mirror**

Transcribe prism-core's types into mirror grammar. See
`/Users/reed/identity/tasks/next/spectral-mirror-grammars-spec.md` Section "Grammar Sketches"
for the full content. The grammar declares: oid, luminosity, addressable, crystal, named,
lambda, imperfect, optic_kind, five operations.

- [ ] **Step 2: gestalt.mirror**

`in @prism`. Token, Theme, Node, Patch, Panel. Token = named(lambda(token_value)).
See the spec for full content.

- [ ] **Step 3: witness.mirror**

`in @prism`. Sign, Verify, Visibility, Seal, Attestation.

- [ ] **Step 4: fate.mirror**

`in @prism`. Five models, Features, Decision, ManifoldState, Strategy, FateOutput.

- [ ] **Step 5: admin.mirror**

`in @gestalt`. Routes, views, handlers for the admin panel.

- [ ] **Step 6: Verify all grammars compile**

```bash
for f in mirror/*.mirror; do
  mirror craft "$f" --check
done
```

Expected: all compile, no errors.

- [ ] **Step 7: Commit**

```bash
git checkout -b reed/grammar-codegen
git add mirror/
git commit -m "🟢 spectral/mirror/: prism, gestalt, witness, fate, admin grammars"
```

---

### Task 2: spectral.spec

**Files:**
- Create: `spectral.spec`

- [ ] **Step 1: Write the spec file**

```mirror
craft {
  source mirror("mirror/*.mirror") {
    @prism
    @gestalt
    @witness
    @fate
    @admin
  }

  target @gestalt => gestalt out @code/rust("crates/gestalt/src/generated/") {
    @gestalt
  }

  target @witness => witness_rs out @code/rust("crates/witness-rs/src/generated/") {
    @witness
  }

  target @prism => prism_gleam out @code/gleam("beam/prism_gleam/src/generated/") {
    @prism
  }

  target @gestalt => gestalt_gleam out @code/gleam("beam/gestalt/src/generated/") {
    @gestalt
  }

  target @fate => fate_gleam out @code/gleam("beam/fate/src/generated/") {
    @fate
  }

  target @admin => admin_gleam out @code/gleam("beam/admin/src/generated/") {
    @admin
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add spectral.spec
git commit -m "🟢 spectral.spec: target declarations for all grammars"
```

---

### Task 3: Generate Rust packages

**Files:**
- Generated: `crates/gestalt/src/generated/`
- Generated: `crates/witness-rs/src/generated/`

- [ ] **Step 1: Run generation**

```bash
mirror craft spectral.spec --target gestalt
mirror craft spectral.spec --target witness_rs
```

- [ ] **Step 2: Verify generated Rust compiles**

```bash
CARGO_TARGET_DIR=/tmp/spectral-codegen cargo check --workspace
```

- [ ] **Step 3: Wire generated code into crate lib.rs**

In `crates/gestalt/src/lib.rs`:
```rust
pub mod generated;
pub use generated::*;
```

In `crates/witness-rs/src/lib.rs`:
```rust
pub mod generated;
pub use generated::*;
```

- [ ] **Step 4: Verify existing tests still pass**

```bash
CARGO_TARGET_DIR=/tmp/spectral-codegen cargo test --workspace
```

- [ ] **Step 5: Commit**

```bash
git add crates/gestalt/src/generated/ crates/gestalt/src/lib.rs
git add crates/witness-rs/src/generated/ crates/witness-rs/src/lib.rs
git commit -m "🟢 generated Rust: gestalt + witness-rs from mirror grammars"
```

---

### Task 4: Generate Gleam packages

**Files:**
- Generated: `beam/prism_gleam/src/generated/`
- Generated: `beam/gestalt/src/generated/`
- Generated: `beam/fate/src/generated/`
- Generated: `beam/admin/src/generated/`

- [ ] **Step 1: Run generation**

```bash
mirror craft spectral.spec --target prism_gleam
mirror craft spectral.spec --target gestalt_gleam
mirror craft spectral.spec --target fate_gleam
mirror craft spectral.spec --target admin_gleam
```

- [ ] **Step 2: Verify generated Gleam compiles**

```bash
cd beam/admin && gleam check
cd beam/gen_prism && gleam check
```

- [ ] **Step 3: Wire generated code into Gleam packages**

Each `gleam.toml` adds the generated package as a local dependency.
Each main module imports from `generated/`.

- [ ] **Step 4: Commit**

```bash
git add beam/*/src/generated/
git commit -m "🟢 generated Gleam: prism, gestalt, fate, admin from mirror grammars"
```

---

### Task 5: Replace hand-written types with generated

**Files:**
- Modify: `crates/gestalt/src/token.rs` — use generated Token type
- Modify: `crates/gestalt/src/dom.rs` — use generated Node type
- Modify: `crates/witness-rs/src/lib.rs` — use generated types

- [ ] **Step 1: gestalt crate — replace hand-written with generated**

Remove hand-written type declarations that now exist in `generated/`.
Keep runtime code (materialize, diff, apply_patches) — it uses generated types.

- [ ] **Step 2: witness-rs crate — replace hand-written with generated**

Same pattern. Types from generated. Runtime stays hand-written.

- [ ] **Step 3: Verify all tests pass**

```bash
CARGO_TARGET_DIR=/tmp/spectral-codegen cargo test --workspace
```

- [ ] **Step 4: Commit**

```bash
git add crates/
git commit -m "♻️ replace hand-written types with generated — grammar is source of truth"
```

---

### Task 6: Verify OID consistency across languages

- [ ] **Step 1: Write cross-language OID test**

The OID of `Token` in generated Rust must be derivable from the same grammar source
as the OID of `Token` in generated Gleam. Write a test that:

1. Compiles `mirror/gestalt.mirror`
2. Gets the grammar OID
3. Verifies the generated Rust contains a comment with the grammar OID
4. Verifies the generated Gleam contains a comment with the same grammar OID

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "🟢 cross-language OID consistency — same grammar OID in Rust and Gleam output"
```

---

## Ship Criteria

- [ ] 5 grammars in `spectral/mirror/` — all compile
- [ ] `spectral.spec` declares all targets
- [ ] `mirror craft spectral.spec` generates Rust and Gleam packages
- [ ] Generated Rust compiles in workspace
- [ ] Generated Gleam compiles
- [ ] Hand-written types replaced with generated types
- [ ] Runtime code uses generated types
- [ ] Grammar OID consistent across Rust and Gleam output
- [ ] All existing tests pass

---

## What This Enables

- Tiago's fork button: edit a grammar, regenerate, tokens update across languages
- Design system admin: token types generated from `@gestalt` grammar
- gen_prism: process types generated from `@prism` grammar
- No bridge code between Rust and Gleam — same grammar generates both
- Content-addressed versioning — grammar OID IS the version

---

*Grammars are source of truth. Generated code is output.*
*The grammar IS the protocol. The OID IS the version.*
*Edit the grammar. Regenerate. The types align by construction.*

*2026-04-15. Reed.*
