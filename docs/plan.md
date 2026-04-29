# Spectral Plan — Two Ticks

Consumer-driven. Mirror delivers. Spectral consumes. What breaks tells us
spectral-db's shape.

---

## Tick 1: Mirror Delivers ✅

Complete. Branch `glint/tick-1-mirror-delivers`.

- All six feature branches merged (339 tests)
- Cli struct: `Cli::open()`, `Cli::dispatch()`, `Cli::crystal_oid()`
- main.rs → thin shell
- `mirror crystal --oid` prints loaded crystal hash
- Meta-property test passes (same spec → same OID)
- 20 CLI e2e tests passing, 2 ignored contracts

---

## Tick 2: Spectral Consumes

### 2.0 MirrorOid<H: HashAlg = CoincidenceHash>

The foundation type. Before anything else.

```rust
pub struct MirrorOid<H: HashAlg = CoincidenceHash>(Oid<H>);
```

Default is coincidence. Always. You never write the parameter unless
you're crossing a boundary.

```rust
MirrorOid                    // CoincidenceHash. Home.
MirrorOid<SHA1>              // git. Visiting.
```

The Shard:

```rust
pub struct Shard<V, H: HashAlg = CoincidenceHash> {
    value: V,
    oid: MirrorOid<H>,
}
```

Foreign keys via GAT:

```rust
pub trait ForeignKey {
    type Target<F: HashAlg>: ContentAddressed<F>;
    fn foreign<F: HashAlg>(&self) -> Option<&Oid<F>>;
}
```

Home produces visitors. Visitors don't produce home.

Update the Store trait:

```rust
pub trait Store {
    type Value;
    type Hash: HashAlg = CoincidenceHash;
    type Shard: ContentAddressed<Self::Hash>;
    type Error;
    type Loss: Loss;

    fn insert(&mut self, value: Self::Value)
        -> Imperfect<Self::Shard, Self::Error, Self::Loss>;
    fn get(&self, oid: &MirrorOid<Self::Hash>)
        -> Imperfect<Self::Shard, Self::Error, Self::Loss>;
}
```

Where this lives: `mirror/src/store.rs` (update existing).

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

spectral-db has three storage layers (from Mara's research):
- Node store (git-backed, schema-validated)
- SpectralCoordStore (eigenvalue vectors)
- ManifoldStore (16×16 manifold states — already returns Imperfect)

SpectralDb becomes generic over Store:

```rust
pub struct SpectralDb<S: Store> {
    store: S,
}

impl<S: Store> SpectralDb<S> {
    pub fn tick(&mut self, signal: S::Value)
        -> Imperfect<S::Shard, S::Error, S::Loss>;
    pub fn tock(&mut self)
        -> Imperfect<(), S::Error, S::Loss>;
}
```

Three store impls, one for each layer. Linked by SpectralIndex.
The shape of tick/tock emerges from wiring the five optic commands
through the store.

What breaks here tells us what Store is missing.

Key findings from Mara's research:
- Loss has direction (mutation vs measurement) — may need two Loss types
- The three stores aren't independent — linked through SpectralIndex
- ManifoldStore is closest to Store shape already
- `.mirror` grammar via mirror's parser — `.conv` extension dropped, `schema.rs` parser removed
- Mnesia integration structurally complete, has placeholder stubs

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
- `spectral compile boot/00-prism.mirror` → prints OID (coincidence hash)
- `spectral init` → creates `.git/mirror/` + writes crystal
- `spectral repl` → `shard>` prompt, parses expressions, prints results
- spectral-db compiles with `Store` generic
- MirrorOid<CoincidenceHash> is the default throughout
- ForeignKey<SHA1> bridges to git

---

## What Tick 1 + 2 Tell Us

After both ticks, we know:
- What Store methods spectral-db actually calls (the interface settles)
- What MirrorLoss fields the CLI actually reads (the loss settles)
- What the REPL needs from the parser (the parse contract settles)
- Where the five optic commands break (the missing pieces reveal themselves)
- Whether the three-store architecture needs one Store or three (the shape emerges)
- Whether directed loss (mutation vs measurement) needs separate types

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
