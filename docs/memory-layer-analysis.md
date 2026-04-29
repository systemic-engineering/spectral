# Agentic Memory Layer Analysis

> Spectral memory against the landscape. Where it leads, where it's missing, how to close the gaps.

**Date:** 2026-04-29
**Method:** Implementation audit + web research + existing spectral/systemic.engineering research synthesis. All findings stored as spectral memory nodes via MCP tools.

---

## What Spectral Has (The Structural Distinctives)

Spectral is not another graph database with an MCP wrapper. It is a git-native memory substrate where the concept graph IS the git tree. Every memory state is a commit. Every query is a projection over git objects. Every crystal is a commit with structural parents.

What makes it distinct from every other system in this analysis:

1. **Git-native persistence.** The graph lives at `refs/spectral/HEAD` as a tree-of-trees. Per-node subtrees carry `.type`, `.content`, `.ts`, `.meta` entries; edges are named entries pointing to target OIDs. No sidecar database. No external service. `git` is the only dependency.

2. **Content-addressed nodes.** OID = `sha(kind:name:value)`. Identical content produces identical OIDs across repos. Dedup is O(1) at write time without an LLM call. Cross-repo crystal portability is free.

3. **Eigenvalue graph fingerprint.** 16-dimensional Laplacian decomposition. The Fiedler vector encodes graph bisection structure. k-NN in spectral space is a direct proxy for structural community membership — more principled than Leiden's greedy modularity optimization. Deterministic, reproducible, no embedding model.

4. **ShannonLoss.** Every query reports bits of information filtered out. The system quantifies its own ignorance. No other surveyed system exposes an information-theoretic self-knowledge metric.

5. **Settlement-to-crystal lifecycle.** Observe (focus) / decide (project, split, zoom) / settle (refract) as distinct graph operations with optic-typed failure modes (Lens = total, Prism = zero-or-one, Traversal = zero-or-many). Crystals are epistemic primitives — settled subgraphs that the system has learned — not cache entries.

6. **Six git-native optic tools.** `memory_diff` (git diff-tree), `memory_blame` (git log --follow), `memory_branch` (git update-ref), `memory_checkout` (symref repoint), `memory_thread` (git-notes walk), `memory_cherrypick` (replay commit onto HEAD). Plus 8 built-in tools. 14 tools total.

7. **Grammar-driven tool extension.** `.mirror` files auto-register as MCP tools. The agent can extend its own tool surface by writing grammar. Self-evolving capability that survives across sessions.

8. **Zero cost.** No embedding API calls. No external database. No network dependency. Fully offline, fully local, fully private.

Spectral OID: `313f5dc7da8557e999da455a075f5db72392de21`

---

## The Landscape

### Memory Tiers (Cognitive Architecture Frame)

The CoALA framework (Sumers et al., 2023) provides the canonical taxonomy for agent memory, drawn from cognitive science and the SOAR architecture. Five tiers:

| Tier | Definition | Industry mapping | Spectral mapping |
|---|---|---|---|
| **Sensory** | Raw input stream, minimal filtering | Token stream, raw messages, file events | `gestalt_detect` — scans raw files |
| **Working** | Short-term scratchpad for current task | Conversation context, in-context memory (Letta core, LangChain buffer) | Hamilton projection — the in-memory SpectralDb cache, budget-bounded |
| **Episodic** | Records of past experiences | Conversation logs, event stores (Zep episodes, Letta recall) | Git commit history at `refs/spectral/HEAD` — every settlement IS an episode |
| **Semantic** | Factual knowledge, entity relationships | Knowledge graphs, extracted facts (Mem0 facts, Zep entities, Cognee KG) | Concept graph nodes and edges, structurally queryable via pipe-forward queries |
| **Procedural** | How to do things | Code, pipelines, learned skills (Letta skills) | Crystals + grammar-driven tool generation — settled patterns the system has learned |

How existing systems cover these tiers:
- **Letta:** Working + partial episodic + partial procedural. No semantic graph.
- **Mem0:** Semantic + partial episodic. No working memory management. No procedural.
- **Zep:** Episodic + semantic + temporal. No working memory. No procedural.
- **GraphRAG:** Semantic at community level only. No episodic, working, or procedural.
- **Spectral:** Episodic (git history), semantic (graph), procedural (crystals + grammar). Working memory is the Hamilton projection. Sensory is gestalt. The most complete coverage of any single system — but with gaps in temporal edge validity and semantic/vector retrieval.

Spectral OID: `7560adc3a50b241b42c77ce0d54807e0085999b9`

### MemGPT / Letta

**Architecture:** OS-metaphor tiered memory. The context window is RAM; external storage is disk. The LLM pages data between tiers via function calls.

- **Main context (RAM):** System prompt + core memory blocks (always-present, writable) + chat summary + recent history (FIFO).
- **External (disk):** Recall storage (full conversation history, searchable) + archival storage (overflow knowledge).
- **Memory management:** LLM-directed. Memory pressure at ~70% triggers warnings; at 100% triggers eviction + recursive summarization.
- **Storage:** Postgres.
- **Retrieval:** LLM-directed function calls + vector search on archival.

**Letta V1 (Oct 2025)** deprecated heartbeats and send_message-as-tool, acknowledging frontier models handle reasoning natively. Added memory blocks, stateful agents, skill learning, AI Memory SDK, Conversations API, and Context Repositories (git-based memory for coding agents, Feb 2026).

**Where spectral is ahead:** Git-native persistence (Letta uses Postgres), eigenvalue structure, settlement/crystal lifecycle, zero embedding cost, deterministic retrieval, full version history without snapshots.

**Where spectral is behind:** Letta's white-box memory makes the exact prompt visible to developers. Letta's self-managing memory means the agent actively decides what to page in/out. Spectral has no equivalent context-window-management layer — the agent must know OIDs or use structured queries.

**Filesystem surprise:** Letta's own benchmarks showed GPT-4o mini with plain filesystem tools scored 74% on LoCoMo, beating Mem0's graph variant at 68.5%. Agent familiarity with the retrieval mechanism matters more than sophistication.

Spectral OID: `f7aac1b25fd41496362238de52ea14b0b1d0bc90`

### Mem0

**Architecture:** Managed/self-hosted memory layer. Two-phase pipeline: (1) Extraction — LLM identifies salient facts, (2) Update — compare against existing via vector similarity, consolidate or create. Graph variant (Mem0g) adds directed labeled graph.

- **Storage:** Postgres + Qdrant (or other vector DB). Hybrid graph + vector + key-value.
- **Retrieval:** BM25 + vector + entity search.
- **Key features:** Intelligent filtering, dynamic forgetting (low-relevance entries decay), memory consolidation, multiple memory types via unified API.
- **Results:** 66.9% on LOCOMO (+26% vs OpenAI native memory), 91% lower p95 latency, 90% token savings.
- **Integrations:** 21 frameworks, 19 vector store backends.

**Where spectral is ahead:** Git-native persistence with full version history, eigenvalue structure, content-addressed dedup without LLM calls, zero embedding cost, deterministic retrieval, settlement/crystal lifecycle.

**Where spectral is behind:** Mem0's semantic retrieval is the baseline expectation — agents can search by meaning, not just by structure. Mem0's dynamic forgetting (confidence decay over time) is missing from spectral. Mem0's entity extraction pipeline automatically structures unstructured input. Mem0's integration ecosystem (21 frameworks) dwarfs spectral's current MCP-only surface.

Spectral OID: `3de4dc1def133dc034adef6398a30122f83f4fe7`

### Zep (Temporal Knowledge Graph)

**Architecture:** The most architecturally interesting competitor. Powered by Graphiti — a dynamic, temporally-aware knowledge graph engine.

- **Three subgraphs:** Episode (raw events + timestamps), semantic entity (entities + facts in 1024D embedding space), community (groups via community detection).
- **Storage:** Neo4j + Postgres + Redis.
- **Retrieval:** Hybrid (BM25 + vector + entity) with predefined Cypher templates (NOT LLM-generated Cypher, to prevent hallucinated graph mutations).
- **Key differentiator:** Bi-temporal edges. Every edge carries `event_time` (when true in the world) and `ingestion_time` (when learned). Superseded facts get `valid_until` stamped, not deleted. Enables answering "what did the system believe at time T?"
- **Results:** 94.8% on DMR (vs MemGPT 93.4%), up to 18.5% improvement on LongMemEval, sub-200ms retrieval.

**Where spectral is ahead:** Git-native persistence (Zep needs Neo4j + Postgres + Redis — three services), eigenvalue structure, content-addressed nodes, zero embedding cost, crystal lifecycle, full offline operation.

**Where spectral is behind:** Zep's temporal edges are first-class — spectral has git-level version history but not edge-level temporal validity. Zep's predefined Cypher templates are a smart safety measure against hallucinated mutations. Zep's community detection feeds directly into retrieval. Spectral has eigenvalue clustering but does not yet use it for community-based retrieval.

Spectral OID: `7e740b33f95605dca6a581a9efa1df03a09edd32`

### Graph+Vector Hybrids (Cognee, GraphRAG, LlamaIndex)

**Cognee:** Open-source knowledge engine. Pipeline: unstructured text -> LLM entity extraction -> knowledge graph + vector embeddings. MCP server available. Degrades on informal/specialized text.

**Microsoft GraphRAG:** Solves global synthesis by running hierarchical Leiden community detection, pre-computing LLM summaries at each level. Community summaries for global queries, vector + graph traversal for local queries. Substantially outperforms naive vector RAG on comprehensiveness. Expensive indexing.

**LlamaIndex:** Improved long/short-term memory. Small-to-big / auto-merging retrieval (index small chunks, return parent for synthesis). Knowledge graph integrations available but not primary.

**A-MEM (NeurIPS 2025):** Zettelkasten-inspired. Creates interconnected knowledge networks through dynamic indexing. Each memory note has structured attributes (context, keywords, tags). New memories trigger updates to existing ones. Outperforms baselines across 6 foundation models; doubles performance on multi-hop reasoning.

**Common thread:** All use LLM calls for entity/relationship extraction (expensive). All use vector embeddings for retrieval (model-dependent). All improve over flat vector search for multi-hop queries. None have git-native persistence, eigenvalue structure, content-addressing, or settlement lifecycle.

**Where spectral is ahead:** Git-native durability, content-addressing, eigenvalue structure, zero external dependencies, crystal lifecycle.

**Where spectral is behind:** All provide semantic/vector retrieval as baseline. GraphRAG's community summaries enable global synthesis queries spectral cannot answer. A-MEM's dynamic memory evolution (new memories updating old ones) is more sophisticated than spectral's append-and-ingest model. Cognee's MCP server is already shipping entity extraction.

Spectral OID: `507a03764fd9ae63eba59783bc3989bbf39ba148`

---

## Comparative Matrix

| Capability | spectral | MemGPT/Letta | Mem0 | Zep | Cognee/GraphRAG |
|---|---|---|---|---|---|
| Git-native durability | **Yes** | No | No | No | No |
| Structural graph | **Yes** | No | Partial (Mem0g) | **Yes** | **Yes** |
| Eigenvalue profile | **Yes** | No | No | No | No |
| Version history | **Yes** (full git) | No | No | Partial (temporal edges) | No |
| Semantic retrieval | No | **Yes** | **Yes** | **Yes** | **Yes** |
| Natural language query | No | **Yes** | **Yes** | **Yes** | Partial |
| Temporal edges | No | Partial (FIFO) | No | **Yes** (bi-temporal) | No |
| Entity extraction | No | **Yes** | **Yes** | **Yes** | **Yes** |
| Auto-summarization | No | **Yes** | Partial | Partial | **Yes** (GraphRAG) |
| Cross-session continuity | **Yes** (git) | **Yes** | **Yes** | **Yes** | **Yes** |
| ShannonLoss metric | **Yes** | No | No | No | No |
| Settlement/crystal lifecycle | **Yes** | No | No | No | No |
| Content-addressed nodes | **Yes** | No | No | No | No |
| Memory branching/merging | **Yes** | No | No | No | No |
| Time-travel (blame/diff) | **Yes** | No | No | Partial | No |
| Offline/local-only | **Yes** | No | No | No | Partial (Cognee) |
| Zero embedding cost | **Yes** | No | No | No | No |
| Grammar-driven tools | **Yes** | No | No | No | No |
| Dynamic forgetting | No | Partial (eviction) | **Yes** | Partial | No |
| Infrastructure requirements | git | Postgres | Postgres + vector DB | Neo4j + Postgres + Redis | Neo4j/Memgraph + vector DB |

**Reading this table:** Spectral's column has the most unique checkmarks (8 capabilities no other system has). But it also has the most "No" entries in the semantic/NL/extraction row — the capabilities every other system treats as table stakes. The gap is not in storage or persistence (spectral wins decisively) but in the retrieval and ingestion layers that sit above storage.

---

## Gaps and How to Close Them

### Gap 1: Semantic Retrieval

**What's missing:** Every competitor provides embedding-based semantic similarity search. Spectral has only structural (Laplacian distance) and exact-match (content OID) retrieval. For queries like "find memories about dark mode preferences" there is no natural-language-to-graph-structure bridge.

**Concrete proposal:** Add an optional `.vec` blob in each per-node subtree containing an embedding vector from a lightweight local model (e.g., all-MiniLM-L6-v2 via candle or ort — no API call). At query time, vector similarity finds entry-point nodes; Laplacian diffusion expands structurally from there. The embedding is a disposable cache; the git OID is source of truth. Re-embed on model change. This is the hybrid approach that GraphRAG, HippoRAG, and Zep all converge on. Spectral adds the git-native layer underneath.

**Effort:** 2 weeks. **Impact:** Closes the single largest capability gap.

### Gap 2: Natural Language -> Graph Query

**What's missing:** All competitors accept free-form text queries. Spectral requires pipe-forward structured queries (`find observation |> where field op value`). The Surface model (language -> query translation) described in CLAUDE.md is not yet implemented.

**Concrete proposal:** Implement the Surface model as an LLM call (or small fine-tuned model) that translates natural language into the pipe-forward query language. The query language is the compilation target. Alternatively, once semantic entry points exist (Gap 1), a hybrid retrieval path can bypass the query language entirely for simple lookups: vector search -> spectral walk -> return.

**Effort:** 3 weeks. **Impact:** Makes spectral usable by agents trained on Mem0/Zep-shaped APIs.

### Gap 3: Temporal Edges

**What's missing:** Zep's bi-temporal edges (event_time + ingestion_time, valid_from + valid_until) enable "what did the system believe at time T?" Spectral has git-level version history (which provides commit-level temporality) but not edge-level temporal validity within a single graph state.

**Concrete proposal:** Extend the edge blob format in per-node subtrees to include `valid_from` and optional `valid_until` timestamps. When a fact is superseded, create a new edge blob and stamp `valid_until` on the old one. The git history already captures when each state existed; this adds intra-commit temporal resolution. A `memory_temporal_query(time, oid)` tool returns the graph state as believed at a specific time.

**Effort:** 1 week. **Impact:** High — temporal reasoning is where Zep wins its benchmarks.

### Gap 4: Entity/Concept Extraction Pipeline

**What's missing:** Mem0, Zep, Cognee, and GraphRAG all automatically extract entities and relationships from unstructured text. Spectral's `gestalt_detect` does structural code/file analysis but not semantic entity extraction from prose content. There is no automatic "Alex prefers dark mode" -> `entity(Alex)` + `preference(dark_mode)` pipeline.

**Concrete proposal:** Add an optional LLM-powered extraction step in the `spectral_index` pipeline, between gestalt import and edge detection. Extract entities and relationships from prose content, create typed nodes (entity, relationship), connect to source observation nodes. Gate behind a feature flag (`spectral index --extract-entities`) so the zero-cost path remains default.

**Effort:** 2 weeks. **Impact:** Medium — needed for rich graph construction, but many use cases work without it.

Spectral OID (gaps 1-4): `5a94aa12fd80e78695a88f4d94383be090f69327`

---

## The Bridging Path

Spectral OID: `6b87cc41ec172d6ef8b62c504483614382882489`

The strategic question is not "how do we add everything the competitors have" but "what is the minimum addition that makes spectral's structural advantages legible to agents and users while preserving what no one else has?"

### The 6-Week Bet

**Week 1: Auto-recall on session start.** When the MCP server boots, run a standing `graph_query` (`find observation |> sort by updated_at desc |> limit 20`) and expose results via the `memory://current` resource. The agent gets relevant prior context without knowing OIDs. Combine with the eigenboard's hot paths for a "what was I working on?" ambient surface. This is the cheapest change with the biggest UX impact. Also: fix `memory_recall` default distance to 2.5 (matching actual graph scale, as documented in `mcp-competitive-design.md`).

**Weeks 2-3: Semantic entry points.** Add optional embedding vectors as `.vec` blobs. Use a local model — no API dependency. Vector similarity finds entry-point nodes; Laplacian diffusion expands structurally. The hybrid retrieval path: `vector_search(query) -> top_k_nodes -> spectral_walk(distance)`. This gives spectral the semantic retrieval that every competitor has, layered on top of the structural substrate that no competitor has.

**Week 4: Temporal edge metadata.** Extend the edge blob format with `valid_from` / `valid_until`. Low implementation cost, high value for Zep-style temporal reasoning. Combined with git history, this gives spectral the strongest temporal story in the landscape — version-level history (git) plus edge-level temporal validity.

**Weeks 5-6: Surface model (NL -> query).** An LLM call that translates natural language into pipe-forward queries. With semantic entry points already working, this becomes a convenience layer rather than a critical path — agents can use vector search directly. But it makes the query language accessible and teaches the agent the spectral vocabulary.

### What This Achieves

After 6 weeks, spectral has:
- Everything it has today (git-native, eigenvalue, ShannonLoss, crystals, content-addressing, offline, zero-cost)
- Semantic retrieval via hybrid vector+structural path
- Natural language query via Surface model
- Temporal edge validity for time-travel queries
- Auto-recall for cross-session continuity

What remains for later:
- Entity extraction pipeline (week 7-8)
- Community summaries a la GraphRAG (week 9-10)
- Personalized PageRank retrieval a la HippoRAG (week 11-12)
- Dynamic forgetting / confidence decay (week 13)

### The Positioning After the Bet

Spectral becomes the only system that combines:
- Semantic retrieval (table stakes, now covered)
- Structural retrieval (eigenvalue, unique)
- Temporal retrieval (edges + git history, strongest in class)
- Full version control of memory state (unique)
- Zero infrastructure cost (unique)

The moat is not one feature. The moat is the combination: every memory operation collapses to a git operation. `memory_diff` = `git diff-tree`. `memory_blame` = `git log --follow`. `memory_branch` = `git update-ref`. No competitor can replicate this without re-architecting around git — and by then, spectral has the semantic layer too.

---

## References

- `docs/git-native-graph-plan.md` — Phases 1-6, the migration that just landed
- `docs/insights/ai-memory-knowledge-graph-research.md` — Prior landscape research (2026-04-23)
- `docs/insights/sigmap-structural-indexing.md` — SigMap validation of structural retrieval
- `docs/mcp-competitive-design.md` — MCP competitive positioning and optic-aligned tool design
- Zep temporal KG: arXiv 2501.13956
- GraphRAG: arXiv 2404.16130
- Mem0: arXiv 2504.19413
- A-MEM: arXiv 2502.12110
- HippoRAG: arXiv 2405.14831
- CoALA (Cognitive Architectures for Language Agents): arXiv 2309.02427
- Memory in the Age of AI Agents (survey): arXiv 2512.13564
- Graph-based Agent Memory taxonomy: arXiv 2602.05665
- Mem0 State of AI Agent Memory 2026: mem0.ai/blog/state-of-ai-agent-memory-2026
- 2026 AI Agent Memory Wars: chauyan.dev/en/blog/ai-agent-memory-wars-three-schools-en
