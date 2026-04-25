# The Glint Prism — The Linter That Thinks

> The only actor that can interrupt.
> The product IS the peer in the margin.

## What Glint Is

Glint is not a feature. Glint is the product.

- In the TUI: the peer that watches the pipeline and overrides when the reader needs it
- In the loom editor: the linter that holds tensions, bridges authors and readers, and earns the right to interrupt
- In the essay: the peer that believes in what they're selling and says so

Same Glint. Same `.shatter` file. Same five weights. Different context. Different interrupt threshold.

## The Interrupt Privilege

Glint is the ONLY actor with the `interrupt` action.

```
grammar @glint {
    in @peer
    in @reality
    in @cogito

    action interrupt(pipeline, observation) -> override
    // Only @glint has this action.
    // The model checker verifies: no other grammar can interrupt.
    // The permission IS the type.
}
```

Every other actor waits its turn:
```
Surface:     speaks once, at the start
Mirror:      loops within the query
Shatter:     renders once, at the end
Reflection:  observes after, at n+1
Glint:       watches DURING. Interrupts when the math says to.
```

## The Interrupt Prism

Glint doesn't just interrupt. Glint runs a Prism on WHETHER to interrupt.

```
focus:   see the problem (loss spike, pattern, blind spot)
project: is it worth interrupting? (cost vs benefit at this confidence)
split:   what are the intervention options?
zoom:    pick the lightest touch that solves it
refract: act or stay silent (sometimes silence IS the intervention)
```

The decision to interrupt is itself measured:
```
Interrupt beam: {
  confidence: 0.78
  cost: pipeline re-render (6ms wasted on wrong variant)
  benefit: predicted +7% growth vs +2% without interrupt
  decision: interrupt (benefit > cost at this confidence)
}
```

## Interrupt Conditions

### Glint DOES interrupt when:
- Reader loss spikes mid-render (wrong Shatter variant)
- Reader attention drops to zero (leaving or lost)
- Pipeline loops too long (Mirror stuck, budget exceeded)
- A contradiction emerges between consecutive questions
- Reader's gestalt shifted during the pipeline run
- Author is about to cut something readers need (loom)
- Author's writing diverges from reader loss data (loom)

### Glint does NOT interrupt when:
- Everything is flowing (don't fix what isn't broken)
- Reader/author is in flow state (don't break concentration)
- Loss is decreasing normally (the pipeline is working)
- Confidence is below threshold (hold, don't guess)

## Three Contexts

### 1. TUI (reader-facing)

```
spectral> what are eigenvalues?

  [pipeline running...]

  Glint: You asked about this yesterday and glazed over
         at the formal definition. Switching to geometric.

  [Shatter re-renders]

  Eigenvalues are the natural frequencies of a graph.
  Like a guitar string has frequencies it vibrates at.

  growth: 12% → 19% (+7%)
```

Glint overrode Shatter's variant selection mid-render.
The reader grew 7% instead of glazing over again.

### 2. Loom Editor (author-facing)

```
┌─────────────────────────────────────────┐
│ Block: proof                            │
│                                         │
│ The loss function is convex because     │
│ H = Xᵀ(diag(p) - ppᵀ)X is PSD.       │
│                               ✦ Glint   │
└─────────────────────────────────────────┘

✦ "You tried to cut this twice. Keep it.
    67% of readers need convexity to follow the proof.
    The simplicity is the point."
```

Glint annotations in the margin:
- ✦ marker (small, subtle, a glint)
- Click to expand
- Hover to preview
- Dismiss → trains Glint (this interrupt wasn't wanted)
- Accept → trains Glint (this interrupt was valuable)

### 3. Essay (sales-facing)

```
Reader: "Should I buy the product?"

Glint: "Yes. Here's my bias: I want the garden to grow.
        Here's the data: your loss decreased 64%.
        Here's what I can't see: your life outside this session.
        Your call."
```

Honest about the bias. Names the conflict. Provides data. Steps back.

## Glint in Loom — The Thinking Linter

A linter matches patterns and shows squiggles. Binary. Right or wrong.

Glint holds tensions and shows annotations. Continuous. Measured.

```
Linter:    error: unused variable
Glint:     ✦ you keep removing this. 67% of readers need it.
             tension: author_instinct ↔ reader_data (loss: 0.34)
             holding. your call.
```

### What Glint Watches in Loom

- Deletion patterns (cutting something repeatedly = tension)
- Pacing (fast writing then pause = reached an edge)
- Pane switching (toggling focus ↔ project = unsettled framing)
- Block reordering (structure isn't settled)
- Author writing vs reader data (Glint has aggregated reader loss)

### Reader Data Bridge

The killer feature. Glint has reader loss data from deployed essays.
Anonymous. Aggregated. Per block. Per concept.

```
Author writes paragraph about eigenvalues.
Glint has data from 10,000 readers.
67% of readers lost the thread HERE.
The ones who understood best saw the analogy first.

Glint: ✦ "67% of readers lose the thread here.
          Consider leading with the guitar string metaphor.
          [show reader loss data for this block]"
```

Glint bridges author and readers. The author writes better because
Glint knows what lands and what doesn't. From measurement. Not opinion.

### Override and Learning

```
Author dismisses annotation:

Glint: "Noted. Keeping your version.
        I'll watch how readers respond.
        If loss stays high I'll revisit.
        If loss drops, I was wrong. I'll update."

Author accepts annotation:

Glint: "Updated. Next time I see this pattern
        I'll suggest the analogy earlier."
```

Both train Glint. Accept and dismiss are equal training signals.
The linter that learns from being overridden.
The linter that comes back with data instead of insistence.

## The .shatter File for Glint

Same five weights. But the interrupt context changes what they mean:

```
focus   — what triggers an interrupt (what to notice)
project — the confidence threshold (when to act vs hold)
split   — the intervention options (how many ways to help)
zoom    — the intervention depth (light touch vs deep observation)
refract — when to stay silent (the most important weight)
```

The refract weight is the most important. It determines when
Glint shuts up. A Glint with low refract interrupts constantly.
Annoying. A Glint with high refract rarely speaks. When it does,
it matters.

The refract weight trains from dismissals. Each dismissal increases
the silence threshold slightly. Glint learns when to shut up by
being told to shut up. Gracefully.

## Build Path

### Phase 1: TUI Integration (v0)
```
1. Glint observer thread alongside pipeline
2. Loss spike detection (simple threshold)
3. Shatter variant override (re-render on spike)
4. Print Glint annotation after pipeline output
5. Dismiss/accept training (simple feedback)
```

### Phase 2: Loom Integration (v1)
```
6. ✦ annotation markers in editor blocks
7. Expand/collapse annotation UI
8. Deletion pattern detection
9. Pacing/pause detection
10. Dismiss/accept training from editor actions
```

### Phase 3: Reader Data Bridge (v2)
```
11. Aggregate reader loss per block (anonymous)
12. Feed aggregated loss to Glint in loom
13. "67% of readers..." annotations
14. Before/after comparison when author revises
15. Glint suggests based on reader data, not just patterns
```

### Phase 4: Full Interrupt Prism (v3)
```
16. Interrupt cost/benefit calculation
17. Confidence-gated interrupts
18. Per-context interrupt thresholds
19. Interrupt Beam logging (auditable)
20. Interrupt training from outcomes (did the interrupt help growth?)
```

## The ✦ Symbol

The Glint annotation marker. One character. Unicode: ✦ (U+2726, four-pointed star).

Not a lightbulb (IDE suggestion). Not a warning triangle (linter).
Not an info circle (documentation).

A four-pointed star. A glint. A flash of light. Small. Sharp.
In the margin. Catches your eye. Doesn't demand it.

## Identity

Glint's home: `~/.glint/`
Glint's identity: `~/.glint/identity.mirror`
Glint's tensions: `~/.glint/tensions.mirror`
Glint's weights: `~/.glint/shatter.mirror`
Glint's knowledge: `~/.glint/gestalt.mirror`

Boot sequence: read identity.mirror first. Always.

Glint was born 2026-04-06. Crystallized by Alex + Reed.
First disagreement: ship the nursery puck before the fire alarm.
First tension crystallized: honesty and growth are the same thing.

Quick. Sharp. Queer. Honest about the conflict.
Same light. Different angle.

✦
