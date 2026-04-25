# The Garden Surface

The protected source compiles to a public grammar. The grammar IS
the API surface. The API surface is all you need.

---

## @code/spectral

The public grammar that spectral-db exposes:

```mirror
grammar @code/spectral {
    in @code

    type signal(mutation)
    type node_state = ticking | settled(u64) | crystal
    type spectral_index(graph)
    type manifold_state([f64])

    action tick(signal) -> imperfect
    action tock() -> imperfect
    action get(oid) -> imperfect
    action focus(path) -> imperfect
    action eigenvalues(index) -> [f64]
    action holonomy(index) -> f64
    action converged(index) -> bool
}
```

The grammar is `public`. The implementation is `protected`. The
consumer writes `in @code/spectral` and gets the typed surface.
They know WHAT the database does. They don't see HOW.

---

## The Visibility Split

```
@code/spectral              public    the grammar, the API, the contract
spectral-db implementation  protected the store, the index, the LAPACK
```

Two layers. Same codebase. Different visibility. The grammar is the
interface. The implementation is behind the consent boundary.

### What the garden visitor sees

```
garden.systemic.engineering/spectral-db

  @code/spectral (public)
    types: signal, node_state, spectral_index, manifold_state
    actions: tick, tock, get, focus, eigenvalues, holonomy, converged
    holonomy: 0.000 (crystal — the grammar is settled)

  implementation (protected)
    holonomy: 0.089 (amber — still settling)
    agents: 3 active
    gutter: visible but source: 🔒
```

The grammar is crystal. Green. The implementation is amber. Still
settling. The visitor sees both states. Different visibility. Same
gutter.

The measurement is public. The source is protected. The observation
IS the demo. The consent boundary IS the paywall.

### What the consumer writes

```mirror
in @code/spectral

action my_pipeline(data) {
    let signal = transform(data)?;
    @code/spectral.tick(signal)?;
    @code/spectral.tock()?;
    @code/spectral.holonomy(index)
}
```

The consumer uses the grammar. The grammar is the contract. The
contract is public. The implementation delivers. The implementation
is protected.

---

## The Live Garden

```
garden.systemic.engineering

  ● mirror        crystal  holonomy: 0.000
  ● spectral      partial  holonomy: 0.034    ●●○
  ● terni         crystal  holonomy: 0.000
  ● fate          settling holonomy: 0.012    ●○
  ● spectral-db   partial  holonomy: 0.089    ●●●○○

  agents active: 3
  ├── mara     @spectral-db  refactoring store.rs
  ├── seam     @mirror       reviewing cli.rs
  └── taut     @fate         benchmarking derive

  last tick: 2.3s ago
  last crystal: mirror @ 14:23:07
```

The visitor watches the codebase settle. In real time. The agents
working. The eigenvalues shifting. The holonomy decreasing. The
garden growing.

Click on a crate → the gutter. Live. Green/amber/red.
Click on timeline → holonomy over time. The settling.
Click on shard> → the REPL. Query the live state.

```
shard> focus @code/spectral
  types: 4
  actions: 7
  holonomy: 0.000
  status: crystal

shard> focus spectral-db
  status: protected
  holonomy: 0.089
  agents: 3
  🔒 source requires `in @spectral`
```

---

## Auto-Deploy

Every time an agent crystallizes a crate, the garden updates.
The crystal IS the deployment. The OID changes. The garden
refreshes. No CI pipeline. No deploy script. The crystal
settling IS the deploy.

```
mirror crystallized    → garden updates mirror page
terni crystallized     → garden updates terni page
spectral-db settling   → garden shows amber, agents working
```

---

## The Grammar IS

```
the grammar IS the API surface
the grammar IS the documentation
the grammar IS the type contract
the grammar IS the demo
the grammar IS the product boundary
```

One grammar. Published. Content-addressed. The shard IS the docs.
The consumer gets the types. The visitor gets the gutter. The
implementation gets the consent boundary.

`type(public) grammar`. `type(protected) source`.

The observation IS the marketing. The consent boundary IS the
paywall. The gutter IS the demo.

---

*The garden doesn't describe itself. The garden IS itself.
The agents building the infrastructure are visible on the
infrastructure they're building. The visitor watches
autopoiesis in real time.*
