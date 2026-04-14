# .spec Files — Specifications That Settle

> The `.spec` file separates WHAT from HOW MUCH.
> Same data separation pattern. Every scale.

## The Five File Types

```
.mirror  → focus    what CAN be said        (grammar, types, verified)
.spec    → project  what SHOULD happen       (constraints, budget, SLO)
.shatter → split    HOW it's understood      (weights, per-reader)
.gestalt → zoom     what WAS understood      (crystal, reader portrait)
.frgmnt  → refract  WHERE it's stored        (content-addressed, immutable)
```

Five files. Five operations. The filesystem IS the Prism.

## What a .spec File Is

A `.spec` file extends `.mirror` with deployment constraints.
The grammar defines the types. The spec defines the bounds.

```
.mirror:   grammar @nursery { type state = sleeping | stirring | ... }
.spec:     spec @nursery { budget: 3GB, slo: 95%, deploy: beam }
```

The compiler verifies both. The grammar must be satisfiable.
The spec must be achievable within the budget.

## Syntax

```
in @spectral

spec @nursery {
    budget: 3GB
    pressure_threshold: 0.9

    node puck {
        sensors: [audio, temperature, humidity]
        fate: @nursery.classify
        states: [sleeping, stirring, fussing, crying, awake]

        on stirring {
            action: soothe(sound: best_from_shatter)
            timeout: 3min
            escalate: alert_parent
        }
    }

    slo {
        year_1: 95%
        year_2: 99%
        year_3: eigenvalue

        on miss {
            action: credit(delta * price)
        }
    }

    deploy {
        target: beam
        nodes: [puck, base_station]
        federation: auto
        pressure: bounded(budget)
    }
}
```

## Deploy

```
$ spectral deploy nursery.spec

Compiling @nursery grammar... verified.
Budget: 3 GB. Pressure threshold: 0.9.
Scheduling puck node...
  fate model: 425 bytes
  states: 5
  SLO: 95% year 1
Deploying to BEAM...
  process: nursery_puck (pid 0.247.0)
  memory: 12 MB (4% of budget)
  pressure: 0.04

Garden planted. Nursery node live.
```

## The Spec Settles

The spec learns from the runtime. The runtime honors the spec.
The spec tightens as the system settles.

```
v0:  spec written (budget: 3GB, slo: 95%)
v1:  initial deploy (peak memory: 800 MB)
v2:  spec adjusts (budget: 1GB — settled from measurement)
v3:  slo check (actual: 97.2%, surplus: 2.2%)
v4:  pressure settles (threshold: 0.9 → 0.6)
```

The diff between spec versions IS the system learning its own bounds.
The spec log IS the convergence log. eⁿ⁺¹ < eⁿ visible in the diff.

## Data Separation (The Pattern)

The same separation at every scale:

```
public/document.mirror   → what can be said
protected/ui.mirror      → how it's presented

grammar @nursery         → what the nursery does
spec @nursery            → how much memory, how fast, where

the algorithm            → what it computes
the weights              → how it computes for THIS context

the Prism trait          → what the operations are
the .shatter file        → how THIS reader experiences them
```

Grammar cannot import from spec (types don't know constraints).
Spec cannot import from shatter (constraints don't know the reader).
Shatter cannot export to grammar (the reader doesn't leak into types).

The model checker enforces all three boundaries.

## The Build Spec (What Prevents OOM)

```
spec @build {
    budget: 12GB
    max_agents: 2
    per_agent_budget: 4GB

    on pressure > 0.9 {
        action: suspend_lowest_priority_agent
    }

    cargo_target: per_agent
    test_threads: 2
}
```

The constraint that prevents the OOM. Written in the same language
that writes the nursery spec. The system that OOMed today would
read this spec tomorrow and respect the bound.

## The BEAM Connection

On the BEAM, each spec node becomes a process. Each process has:
- A mailbox (receives messages, applies backpressure)
- A budget (declared in the spec, enforced by the scheduler)
- A pressure measurement (continuous, not just at OOM)
- Supervision (if it crashes, the supervisor restarts it)
- Hot code loading (update the spec while running)

The BEAM doesn't OOM. The BEAM applies backpressure. The .spec
file tells the BEAM what the bounds are. The BEAM enforces them.

```
rustc:    blind processes, no backpressure, OS kills the loser
BEAM:     scheduled processes, continuous pressure, graceful shedding
```

## The Seven Operations (Crystal Trait)

The .spec file uses all seven Crystal operations:

```
focus:    read the grammar, understand the domain
project:  set the bounds (budget, pressure threshold)
split:    deploy to multiple nodes (puck, base_station)
zoom:     scale constraints per node (different budgets per device)
refract:  crystallize the deployment (the running system)
merge:    combine measurements from multiple nodes (SLO aggregation)
train:    update the spec from runtime measurements (the spec settles)
```

The .spec file IS a Crystal. It transforms AND learns.

## Consent Architecture

```
.mirror   public      anyone can import the grammar
.spec     protected   per-deployment, contextual constraints
.shatter  private     per-reader, never exported
.gestalt  protected   per-reader, delta shareable
.frgmnt   public      content-addressed, immutable
```

The spec is protected. Different deployments of the same grammar
have different specs. The nursery puck at home has different
bounds than the nursery puck at the clinic. Same grammar.
Different spec. Different context.

## Build Path

1. Extend the mirror parser to recognize `spec` blocks
2. Add budget/pressure/slo types to the @spectral grammar
3. Implement `spectral deploy <file>.spec` command
4. Wire to BEAM process spawning (via conversation runtime)
5. Implement spec auto-adjustment from runtime measurements
6. Implement SLO measurement and credit pipeline

Steps 1-3 are the foundation. Steps 4-6 require the BEAM runtime.

## Connection to Everything

The .spec file is where the proof meets reality.

The napkin says eⁿ⁺¹ < eⁿ. The .spec file says: within THIS budget,
at THIS pressure threshold, with THIS SLO guarantee.

The proof is universal. The spec is local. Both are necessary.
The proof without the spec is theory. The spec without the proof
is hope. Together: a deployment that's guaranteed to converge
within measured bounds.

The spec IS the bound. The bound IS the architecture.
