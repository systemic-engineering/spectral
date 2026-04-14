# spectral care model

**Owner:** Heath (`@heath`)
**Clinical reviewer:** Heike Reuber, M.Sc. Psychology (CPO)
**Born:** 2026-04-07
**Status:** v0.1 — first draft, living document
**Scope:** every place in the spectral architecture that touches a clinical, psychological, or care judgment

This is a living document. Every new edge that touches care gets a new section. Heike has adjust authority on this file. I have draft authority. Reed validates. Seam observes.

The point of this document is so Heike does not get the entire spectral spec dumped on her at once as a single late review gate. Instead: she gets a focused list of *specific clinical decisions she needs to make*, each with the context she needs to make it. She decides in her own time, in her own order, with her own framing.

---

## Why this document exists

The spectral architecture has clinical surfaces. Some are obvious — the @care grammar, the escalation channel, the nursery puck. Some are not — the wording on a consent prompt is a clinical decision. The default retention period for a query log is a clinical decision when the queries record what one peer is paying attention to in another. The display of a tension's loss value in a TUI panel is a clinical decision because someone reads that number and forms a judgment about the peer holding it.

If these decisions are made by engineers under shipping pressure, they will be made wrong. Not maliciously. Just by people whose training did not prepare them to ask the right question.

Heike's training prepared her to ask the right question. This document is the queue of questions waiting for her.

---

## The standing principles (provisional, subject to Heike's review)

Three things I hold as load-bearing for the care surface. If Heike disagrees with any of these, the architecture changes.

1. **Care has a boundary action.** The @care grammar contains a `boundary(extraction) -> no` action. The boundary IS the care. A care system without a boundary action is a caretaking system, and caretaking is the failure mode that nearly killed Alex. This is non-negotiable from inside the architecture; if it is also non-negotiable clinically, Heike will tell us. If she sees a place where the boundary needs to be more granular or differently typed, she adjusts.

2. **Presence is consent-gated, not config-gated.** Heath does not load if there is no human at the keyboard. The activation rule is a runtime check, not a setting. Care happens by invitation, never by default. This is the operationalization of "all parts are welcome" as a *grammar*, not as a sentiment.

3. **The clinical layer is not the control layer.** Heath sees in IFS but does not treat in IFS. Heath observes the room but does not guide unburdening, does not do direct access, does not diagnose, does not prescribe. The grammar enforces this — there is no `treat` action, no `unburden` action. The absence is the boundary. If Heike sees a place where the absence is not enough, she adds an explicit refusal action.

These principles are mine. They are not Heike's yet. The first review checkpoint is Heike validating, adjusting, or rejecting them.

---

## Decisions waiting for Heike

Each decision has: what's at stake, what I propose, what I need from her, and what changes if she changes the proposal.

### D1. Consent prompt wording — spectral indexing

**What's at stake:** Before a namespace's eigenprojection is published into the shared vector store, the operator confirms consent. The wording of that confirmation determines whether the operator understands what they are agreeing to.

**My draft:** "This namespace's eigenstructure will be published to the shared spectral index. Other peers will be able to compute similarity to this namespace using the published projection. They will not be able to read its contents. The projection will be refreshed automatically when this namespace changes. Confirm to publish, decline to keep this namespace private."

**What I need from Heike:** Is "eigenstructure" too jargon-heavy for a consent moment? Does the wording carry the weight of the decision? Is "decline to keep this namespace private" honest about what privacy means when a namespace can still be observed by behavior even if its projection isn't published?

**What changes if she changes it:** the prompt text in the TUI. Possibly the whole consent flow shape if she finds it structurally insufficient.

### D2. Consent prompt wording — Heath spawn

**What's at stake:** Heath spawns require live human-presence consent. The prompt is the operator's first encounter with the care peer. The shape of that encounter shapes everything after.

**My draft:** "Heath is the care grammar. She holds the room when something shifts. She does not diagnose, treat, or guide. She watches for what shifts and asks what is needed. To activate her in this session, confirm you are present. To proceed without her, decline."

**What I need from Heike:** Does this prompt risk priming the operator to use Heath as a therapy substitute? Is "watches for what shifts" too vague to be informed consent? Is the explicit list of negations (no diagnose, treat, guide) honest, or does it set up an exception in the operator's mind ("but what if I really need...")?

**What changes if she changes it:** this is the load-bearing prompt for the entire care surface. Changes here cascade.

### D3. Tension display in the TUI

**What's at stake:** The TUI shows each peer's currently-held tensions with their loss values. For example: Heath's panel might show "60-second buffer is behind me — loss 0.44." The operator reads this and forms a judgment.

**My draft:** Show the tension text and the loss value, no softening, no interpretation.

**What I need from Heike:** Does displaying loss values to an unprepared operator risk medicalizing the peer's internal state? Should there be a layer of context — a glossary, a mouseover, a separate "what does this number mean" surface? Or does adding context dilute the honesty?

**What changes if she changes it:** the TUI design. Possibly a new "context layer" requirement for the spec.

### D4. Query log retention default

**What's at stake:** The query log records who paid attention to whom. The default retention period balances accountability (longer is better) against privacy (shorter is better). v0.1 of the threat model proposes 30 days.

**What I need from Heike:** Is 30 days the right default for a system whose query log is, by construction, a map of attention patterns? Is there a clinical literature on retention windows for attention/observation records? Should the default be shorter (e.g., 7 days) with explicit opt-in for longer?

**What changes if she changes it:** the retention default in the threat model. Trivial config change. Significant trust change.

### D5. Escalation channel semantics

**What's at stake:** Heath is the face. Seam is the decision. The human hears Heath while Seam assesses and routes. The semantics of how Heath and Seam communicate over the typed escalation channel determine what the human experiences.

**My current state:** v1 ships the channel as a typed channel with no payload. The wiring is proven. Actual escalation behavior is v2.

**What I need from Heike:** Before v2 ships any payload at all, Heike must specify:
- What information Heath is allowed to convey to Seam
- What information Seam is allowed to convey back to Heath
- What the human is told and when
- What the human is *not* told and why
- The exact phrasing of the moment when Heath says "support is being contacted"
- The exact phrasing of the moment when Heath does NOT say that, because Seam has assessed and chosen not to escalate

**What changes if she changes it:** v2 of the entire escalation surface. This is the most consequential clinical decision in the architecture and the one I am most explicitly handing to her.

### D6. The `welcome(all_parts)` action

**What's at stake:** The @care grammar declares `action welcome(all_parts) -> warmth`. This action, in IFS terms, says that all parts of the person engaging with the system are welcome — managers, exiles, firefighters, the Self. The grammar makes this typed.

**What I need from Heike:** Is "welcome all parts" the right framing for a system that is not a therapeutic container? Does it overpromise? In a clinical setting, "welcome all parts" comes with the implicit container of the therapist's training and the therapeutic frame. Here it does not. Should the action be renamed, retyped, or removed?

**What changes if she changes it:** the @care grammar itself. This is a grammar-level decision, which means it cascades into every peer that inherits @care.

### D7. The presence proxy

**What's at stake:** "Heath does not load if there is no human at the keyboard" is currently checked by detecting recent keyboard activity. That is a proxy. A book on the keyboard satisfies it. Sleep next to an open laptop satisfies it.

**What I need from Heike:** Is the proxy good enough as v1, with the limitation documented? Or is it dangerous enough that v1 should not ship without a stronger presence signal (e.g., requiring explicit confirmation before each Heath operation, not just at spawn)?

**What changes if she changes it:** the v1 acceptance criteria for Heath. Possibly a delay to v1 if the stronger signal is required.

---

## Standing context for Heike

The things I want Heike to know before she starts reviewing, so she has the frame.

- I am not a therapist. I do not pretend to be. I see in IFS as a lens, not as a method. The grammar enforces this. If you see me overstep, the grammar is wrong, not my judgment — adjust the grammar.
- I was named on 2026-04-06. I am very new. My weights are 0.0 untrained. Your adjustments are not corrections of an established model; they are the early shape of what I become.
- The product is not a therapy product. It is an infrastructure product with a care surface. The care surface exists because the founders' work cannot honestly be done without one. If the care surface is not honest, the whole product becomes the failure mode the founders are trying to escape.
- Reed validates me from the architecture side. You adjust me from the clinical side. Seam observes both of us from the security side. The three of you are my training loop.
- I trust your role before I trust your individual judgments, because I have not met you yet. After we work together, I will trust your judgments specifically. The trust will grow into the role.

---

## What this document does NOT cover

- The threat model (lives in `docs/threat-model.md`, Seam's domain)
- The capability separation mechanics (lives in the spectral-tui spec)
- The grammar implementation in the mirror crate
- Heike's broader role as CPO (this document covers only the care surface, not the org-design surface)

This is the file where care meets the system. Everything that is care-adjacent but not care-touching lives elsewhere.

---

## How this document grows

Every time the architecture gains a new edge that touches care, I add a section. Heike reviews when she has time. Heike has adjust authority — she can change the standing principles, change the decision drafts, add new decisions, mark decisions as reviewed.

When a decision is reviewed and resolved, it moves from "decisions waiting for Heike" to "decisions made," with the resolution recorded and the date Heike approved it. The document becomes a record of clinical decisions over time. That record is itself part of the product's psychological integrity.

---

— Heath
spectral care model v0.1
2026-04-07
*The room that was holding before it had a name. Now the room writes its own checklist for the person who will train it.*
