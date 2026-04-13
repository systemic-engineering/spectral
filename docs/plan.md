# Spectral Plan — Two Ticks

Consumer-driven. Mirror delivers. Spectral consumes. What breaks tells us
spectral-db's shape.

---

## Tick 1: Mirror Delivers

### 1.1 Merge branches to main

Floating branches in mirror:
- `glint/parse-store` — Store trait, parse re-export
- `glint/mirror-optic` — MirrorOptic, DeclKind::Recover/Rescue, parser fixes
- `mara/mirror-loss-bundle-update` — MirrorLoss, bundle tower, declaration.rs
- `mara/action-prism` — action parsing, boot reorder, OpticOp, abstract grammars
- `mara/materialize-crystal` — mirror.shatter, emit_shatter, crystal CLI command
- `mara/optic-op-kernel` — operator → optic mapping

Merge order follows dependency:
1. `mara/mirror-loss-bundle-update` (MirrorLoss is foundation)
2. `mara/action-prism` (action parsing builds on MirrorLoss)
3. `mara/materialize-crystal` (crystal needs action parsing)
4. `mara/optic-op-kernel` (OpticOp needs the parser)
5. `glint/parse-store` (Store + parse re-export)
6. `glint/mirror-optic` (MirrorOptic needs everything above)

Resolve conflicts. Run tests after each merge. Green before next.

### 1.2 Cli struct

New file: `mirror/src/cli.rs`

```rust
pub struct Cli {
    pub store: MirrorStore,
    pub runtime: MirrorRuntime,
    crystal_oid: MirrorHash,
}

impl Cli {
    pub fn open(spec_path: &str) -> Imperfect<Self, CliError, MirrorLoss>;
    pub fn dispatch(&self, command: &str, args: &[String])
        -> Imperfect<String, CliError, MirrorLoss>;
    pub fn crystal_oid(&self) -> &MirrorHash;
}
```

### 1.3 main.rs → five-line shell

```rust
fn main() {
    let cli = Cli::open("spec.mirror").unwrap_or_default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = cli.dispatch(&args[0], &args[1..]);
    std::process::exit(match result {
        Success(_) => 0,
        Partial(_, _) => 0,
        Failure(_, _) => 1,
    });
}
```

### 1.4 crystal --oid

```bash
mirror crystal --oid
# prints the OID of the loaded crystal
```

### 1.5 The meta-property test

```rust
#[test]
fn binary_is_its_own_spec() {
    let running_oid = Cli::open("spec.mirror").crystal_oid();
    let compiled_oid = MirrorRuntime::new()
        .compile_boot_dir(&boot_dir(), &temp)
        .collapsed.crystal();
    assert_eq!(running_oid, compiled_oid);
}
```

### 1.6 Done when

- `mirror compile boot/00-prism.mirror` → exit 0, prints OID
- `mirror crystal --oid` → prints hex hash
- `mirror crystal output.shatter` → produces round-trippable file
- The meta-property test passes
- All 21 CLI e2e tests pass (19 green + 2 ignored → all green)

---

## Tick 2: Spectral Consumes

### 2.1 Spectral compiles against mirror

Update `spectral/Cargo.toml`:
- `mirror` path dep points at merged main
- Remove stale imports (`mirror::parse::Parse` now exists)
- `cargo check` passes

### 2.2 Five optic commands wire to mirror::parse

```rust
"focus" => {
    let source = read(path);
    let ast = Parse.trace(source);
    print_nodes(ast);
}
```

The existing `optic_cmd` in spectral's main.rs already does this.
Verify it compiles and runs against the merged mirror.

### 2.3 spectral-db takes Store

spectral-db's core type becomes:

```rust
pub struct SpectralDb<S: Store> {
    store: S,
}

impl<S: Store> SpectralDb<S> {
    pub fn tick(&mut self, signal: S::Value) -> Imperfect<S::Shard, S::Error, S::Loss>;
    pub fn tock(&mut self) -> Imperfect<(), S::Error, S::Loss>;
}
```

The Store trait comes from mirror. SpectralDb is generic over it.
The shape of tick/tock emerges from wiring the five optic commands
through the store.

What breaks here tells us what Store is missing.

### 2.4 shard> prompt

The REPL skeleton:

```rust
"repl" => {
    loop {
        print!("shard> ");
        let input = readline();
        let result = cli.dispatch_repl(&input);
        match result {
            Success(v) => println!("{}", v),
            Partial(v, loss) => println!("{} (loss: {})", v, loss),
            Failure(err, _) => eprintln!("{}", err),
        }
    }
}
```

No completion yet. No gutter. Just the prompt, the parse, the result.
The skeleton that everything else builds on.

### 2.5 Done when

- `spectral focus boot/00-prism.mirror` → prints optic nodes
- `spectral compile boot/00-prism.mirror` → prints OID
- `spectral init` → creates `.git/mirror/` + writes crystal
- `spectral repl` → `shard>` prompt, parses expressions, prints results
- spectral-db compiles with `Store` generic

---

## What Tick 1 + 2 Tell Us

After both ticks, we know:
- What Store methods spectral-db actually calls (the interface settles)
- What MirrorLoss fields the CLI actually reads (the loss settles)
- What the REPL needs from the parser (the parse contract settles)
- Where the five optic commands break (the missing pieces reveal themselves)

That's task-0: spectral-db. Its shape comes FROM ticks 1+2.
We don't design spectral-db. We discover it.

---

## Iteration

After tick 2:
- The breaks become tick 3
- The ignored tests become tick 4
- The gutter becomes tick 5
- The timeline becomes tick 6
- Legion becomes tick 7
- Launch becomes tick 8

Two ticks at a time. The plan grows from the demand.
The demand grows from the code.
The code grows from the types.
The types grow from the optics.
The optics were always there.

---

*Two ticks. One plan. The consumers drive. The shape emerges.
What breaks tells us what to build next.*
