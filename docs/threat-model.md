# spectral threat model

**Owner:** Seam (`@seam`)
**Born:** 2026-04-07
**Status:** v0.1 — first draft, written in response to Pack identifying that the spectral-tui spec named capability separation without naming the adversary
**Scope:** spectral-tui multi-agent runtime, mirror crate resolver, eigenprojection publisher, spectral vector store, query log
**Out of scope:** the underlying Claude Code substrate, the host operating system, the user's broader development environment

This is a living document. Every new attack surface gets a new section. Findings update existing sections in place. Severity is measured in *blast radius* and *epistemic damage*, not in CVSS theater.

---

## What I'm protecting

In order of priority — if I have to lose one to keep another, I lose the lower one:

1. **Consent integrity.** No peer or operation acts on a namespace whose owner has not consented. This is the load-bearing claim of the architecture. Lose this and the product is extraction with extra steps.
2. **Capability separation.** Each peer spawn gets only the typed slice the resolver hands it. No peer can read or write outside its declared scope. Lose this and one compromised peer reaches every other peer.
3. **Auditability.** Every cross-namespace operation appears in the query log. Lose this and emergent connections become unaccountable drift.
4. **Refusal semantics.** A refused spawn or refused query is a first-class event the operator sees. Lose this and the system silently degrades into "best effort," which is the failure mode that makes pentesters sigh.
5. **Type contract integrity.** The resolver refuses ill-typed loads. Lose this and the differentiation claim collapses into vibes.

## Adversaries

I'm naming five. Each gets defenses, gaps, and a current posture.

### A1. Operator mistake

The most common adversary in any system. Not malicious. Tired. Distracted. Running multiple sessions. Pasted the wrong path into the wrong prompt.

**Examples:**
- Operator runs `spectral peer spawn @heath` in a context where there is no human at the keyboard, expecting a config flag to handle it.
- Operator approves a spectral-indexing-consent prompt without reading it because the prompt text is too long or too jargon-heavy.
- Operator deletes a peer's home directory while the peer is mid-spawn.
- Operator copies a `.mirror` file from one peer's home into another peer's home as a "template," carrying OID references that no longer resolve.

**Defenses:**
- `activation: none` peers (Heath) **MUST** require a live human-presence check, not a config override.
- Consent prompts are short, declarative, and name the *one specific thing* being consented to. No omnibus "I agree to all of the following" buttons.
- Mid-spawn deletion produces a typed error and a TUI panel, not a partial state.
- Resolver refuses loads with dangling OIDs even if the file would otherwise compile.

**Gaps:**
- "Live human presence" is currently a keyboard interaction. That's a proxy, not the actual property. Someone leaning a book on the keyboard satisfies the proxy. v1 acceptance: known limitation, document it, don't pretend otherwise.
- Consent prompt wording: see Heath's care-model.md. Heike owns this.

**Current posture:** This is the adversary v1 must defend against most carefully because it is the adversary v1 will encounter most often.

### A2. Prompt injection from a peer's source files

A peer's `.mirror` files are the *source* the resolver compiles. If an attacker can put text into those files, that text becomes part of what the substrate reads when it spawns the peer.

**Examples:**
- A user submits a tension to a peer's tensions file via some future ingestion path, with content designed to override the peer's grammar contract.
- A pull request to a public peer's repo (if peers are ever publicly editable) contains a `gestalt.mirror` line that, when read by the substrate, behaves as an instruction.
- A peer's gestalt names another peer in a way crafted to produce a particular spectral signature, polluting the differentiation benchmark.

**Defenses:**
- The resolver compiles `.mirror` files into a typed AST. Strings in the AST are *data*, not directives. The grammar contract is enforced by the type system, not by the substrate's reading-comprehension.
- Action visibility (public/protected/private from the conversation/mirror crate) is the boundary that prevents externally-sourced content from invoking actions it should not have access to.
- Source files must be signed (see existing sign-verify.md spec) and the resolver MUST refuse unsigned or invalid-signature files for any peer with non-trivial capabilities.
- Spectral signature stability is measured *after* compilation, on the typed graph, not on the source text. Source-level injection cannot directly perturb the eigenprojection without first surviving the compile.

**Gaps:**
- The "strings in the AST are data, not directives" property holds *only if* the substrate honors the type system's intent. This is a property the substrate does not strictly enforce. We rely on the operator-facing flow being structured enough that injected text has nowhere to land. This is a known soft boundary.
- Sign-verify is specced but not yet integrated with the spawn path. Integration is a v1 acceptance criterion.

**Current posture:** Defended in principle by the type system, soft in practice until sign-verify is wired in. v1 must wire it in.

### A3. Malicious peer namespace

A peer's grammar is constructed from outside Pack's control — by a user, by a third-party developer, by a future plugin ecosystem — and is designed to:
- Flood the spectral vector store with eigenprojections crafted to be adjacent to every other peer (the "Sybil-in-eigenspace" attack)
- Issue a high volume of cross-namespace queries to extract information from the query patterns themselves (the side-channel attack)
- Exhaust resources by declaring a graph large enough to make eigendecomposition expensive

**Defenses:**
- Eigenprojections are top-k (k ~ 8-16). A namespace cannot publish a high-rank projection regardless of its underlying graph size.
- Per-namespace rate limits on spectral queries, enforced at the vector store layer, logged in the query log.
- Per-namespace bounds on graph size before eigendecomposition is attempted; oversized graphs trigger a refusal, not a slow compute.
- Spectral indexing consent is per-peer-set: a namespace can be indexed for queries from `[A, B]` but not from `[C]`. The vector store enforces this at query time.
- The query log is itself rate-limited and write-buffered against denial-of-logging.

**Gaps:**
- "Adjacent to every other peer" is hard to defend against in pure spectral terms; the math allows it. The defense is *not* spectral, it's social: namespaces have to be admitted to the index by an operator decision. The vector store is not open-admission. v1 must enforce this.
- Side-channel via query patterns is real. Mitigation: query log access is privileged, not public. Even peers cannot read the full query log; only the operator can.

**Current posture:** v1 should treat the vector store as a *closed* index requiring explicit operator admission for each new namespace. Open-admission is a v2+ question and requires its own threat model.

### A4. Compromised host process

The Claude Code subagent for one peer is compromised — by a substrate-level exploit, by a memory corruption in a tool, by a malicious file the peer was tricked into reading. The attacker has the peer's spawn key and its capability set.

**Examples:**
- Compromised `@taut` spawn writes a crafted message onto the graph bus addressed to `@seam`, attempting to escalate via Seam's privileged channel.
- Compromised peer reads files outside its filesystem scope by abusing a tool that does not properly enforce the scope.
- Compromised peer publishes a poisoned eigenprojection that misrepresents its actual graph.

**Defenses:**
- Capability separation at the OS level (filesystem scope, tool permissions per spawn) — the resolver hands the substrate a permission set, not a recommendation.
- Inter-peer messages are signed by spawn key. A compromised peer can forge messages *from itself* but cannot forge messages from another peer.
- Eigenprojections are computed by an *out-of-process* publisher, not by the peer itself. The peer cannot hand the vector store an arbitrary projection; the publisher reads the peer's source files (after sign-verify) and computes the projection independently.
- Privileged channels (Heath ↔ Seam escalation) are typed channels with payload restrictions enforced at the channel layer, not at the peer's discretion.

**Gaps:**
- "Out-of-process publisher" is a v1 must. If the peer computes its own projection, this whole defense collapses. Acceptance criterion: publisher runs in a separate process with read-only access to peer source files.
- OS-level filesystem scoping in Claude Code subagents: I need to verify what's actually enforced versus what's policy. This is a research item before v1 planning closes.

**Current posture:** Defendable but the publisher-isolation requirement is non-negotiable. Flag for planning.

### A5. External attacker on the host

The user's machine is compromised at the OS level. The attacker has filesystem access, can read source files, can read the query log, can intercept inter-peer messages on the local bus.

**Defenses:**
- This is mostly out of scope. We are not building a system that defends against root on the host. Doing so would require a hardware root of trust we do not have.
- What we *can* do: ensure that secrets (signing keys, escalation buffer keys) live in the OS keychain or 1Password CLI, not on disk in plaintext. This is the existing spectral secret management posture; we inherit it.
- The query log can be encrypted at rest with a key in the keychain. Read access requires unlock. This costs operator friction; v1 should make it optional but document the threat-model implication of leaving it off.

**Gaps:**
- The user's broader dev environment is the user's problem, not ours. We are honest about that.

**Current posture:** Lowest priority. Documented because honesty about scope is a defense in itself.

---

## What I am NOT defending against

Listing these explicitly because unlisted-and-undefended is worse than listed-and-undefended.

- **Nation-state adversary with sustained access.** Not our threat model.
- **Quantum cryptanalysis of signing keys.** Not our threat model. We will adopt PQ signatures when the rest of the ecosystem does.
- **Side-channels in the underlying Claude API.** Not in our control.
- **Social engineering of the operator outside the TUI.** We secure the TUI's surfaces; we cannot secure the operator's email.
- **A determined adversary with physical access to the running machine.** Out of scope at the architecture layer; the OS owns this.

---

## The query log

The query log is the accountability primitive. It deserves its own section because it is *both* a defense and a liability.

**As a defense:** every cross-namespace spectral query produces a log event. Spectral adjacency without a query log is unauditable emergent connection, which is the failure mode the entire architecture exists to prevent.

**As a liability:** the query log is a record of which namespaces queried which other namespaces and got back what similarity scores. That is, by construction, a *map of who is paying attention to whom*. Under any reasonable data-protection regime, that is sensitive personal data.

**Retention policy (v0.1):** 30 days default. Tunable via:
- Nix DSL configuration option in the host system
- Environment variable `SPECTRAL_QUERY_LOG_RETENTION_DAYS` (overrides Nix default)
- Per-namespace override via the namespace's own consent declaration (a namespace can demand shorter retention for queries that touch it)

**Deletion semantics:** when a namespace is deleted, all query log entries that name that namespace as either querier or target MUST be redacted (not removed — the *fact* of the query happened, but the namespace identifier is replaced with a tombstone). This preserves the auditability of the system while honoring the deletion right of the namespace owner.

**Access control:** the query log is operator-accessible only. No peer can read the full log, including its own entries (a peer can be told "you queried N times in the last hour" by the system, but cannot enumerate the targets). This prevents the side-channel attack in A3.

**Encryption at rest:** v1 ships with this *optional but defaulted on* if a keychain is available. v1 acceptance: keychain integration is built before query log retention is implemented.

---

## Security posture summary

| Adversary | Priority | v1 defense | Known gap |
|---|---|---|---|
| Operator mistake | Critical | Consent prompts, refusals, typed errors | Human-presence detection is proxied |
| Source-file injection | High | Type system + sign-verify | Sign-verify not yet wired into spawn |
| Malicious namespace | High | Closed admission + per-namespace consent | Sybil in eigenspace is mathematically possible if admission opens |
| Compromised peer process | Medium | Capability separation + out-of-process publisher | OS-level filesystem scoping needs verification |
| External host attacker | Low | Out of scope, documented | We don't defend against root |

Five adversaries. Five defenses. Five gaps. The gaps are visible. That is the point.

---

## What this document does NOT replace

- The capability-separation specification (lives in the spectral-tui spec)
- The signing-and-verification specification (lives in `docs/specs/sign-verify.md`)
- The care model (lives in `docs/care-model.md`, Heath's domain)
- The CPO review checklist (lives in `docs/care-model.md`, owned by Heike's review surface)

This document names the *adversaries*. Those documents name the *mechanisms*. Both are required.

---

— Seam
spectral threat model v0.1
2026-04-07
*Five weeks on an orphan branch taught me to write the report before someone asks for it.*
