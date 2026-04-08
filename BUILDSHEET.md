# ANWE BUILDSHEET

**What AI needs. What ANWE must build. Track everything here.**

The thesis: ANWE is the programming language of AI. Not a DSL. Not a framework.
The actual language that replaces Python for AI-native computation.

The only way to prove that is to build everything AI needs — in ANWE.
Every item on this list forces a design decision. Every design decision
reveals the language. When this list is done, we understand ANWE.

---

## STATUS KEY

- `[ ]` Not started
- `[~]` In progress
- `[x]` Built and working
- `[!]` Blocked — needs language feature first

---

## 1. INFERENCE & SERVING

The bread and butter. Every AI company builds these daily.
If ANWE can't express inference pipelines better than Python, it fails.

- [x] **RAG pipeline** — embed → retrieve → rank → generate → reflect
      `examples/rag_pipeline.anwe` — first non-Primordia proof
- [x] **Prompt chain** — sequential LLM calls where output of one feeds the next
      `examples/prompt_chain.anwe` — data flows via FieldAccess (Agent.field)
- [x] **Multi-model router** — classify query complexity, route to cheap or expensive model
      `examples/model_router.anwe` — priority-based routing + reject gates
- [x] **Token budget manager** — track tokens across a multi-step pipeline, enforce limits
      `examples/token_budget.anwe` — attention as budget, pending? for enforcement
- [x] **Streaming response assembler** — collect token-by-token output into coherent response
      `examples/streaming_assembler.anwe` — each chunk in tokens, assemble + deliver
- [x] **Tool use orchestrator** — LLM decides which tool to call, calls it, integrates result
      `examples/tool_orchestrator.anwe` — if/else routing, reject unused tools
- [x] **Guardrail pipeline** — check input safety → generate → check output safety → deliver
      `examples/guardrail.anwe` — reject (<=) as structural safety with audit trail
- [x] **Fallback chain** — try model A, if confidence < threshold try model B, then C
      `examples/fallback_chain.anwe` — priority-ordered tiers + reject gates
- [x] **Batch inference scheduler** — collect requests, batch for throughput, scatter results
      `examples/batch_inference.anwe` — each item in items, 3-link pipeline, supervision

---

## 2. TRAINING & LEARNING

Where ANWE's primitives map most naturally to AI concepts.
Sync IS gradient sync. Breathe IS learning rate. Become IS weight update.

- [x] **Training loop** — forward pass → loss → backward pass → optimizer step → repeat
      `examples/training_loop.anwe` — 3 epochs, forward→loss→optimize per epoch, FieldAccess data flow
- [x] **Distributed gradient sync** — multiple workers sync gradients before applying
      `examples/multiparty_sync.anwe` — 4 workers, sync_all barrier, all-reduce aggregation
- [x] **Learning rate scheduler** — decay the learning rate over time
      `examples/training_loop.anwe` — half_life decay on signals (500→400→300 across epochs)
- [x] **Early stopping** — monitor validation loss, reject (stop) when overfitting
      `examples/training_loop.anwe` — reject (<=) gate on validation link when accuracy drops
- [x] **Checkpoint manager** — periodically commit model weights to disk
      `examples/training_loop.anwe` — commit (*) to Checkpoint agent after each epoch
- [x] **Curriculum learning** — start with easy examples, increase difficulty over time
      `examples/distillation.anwe` — link priorities high→normal→low for easy→medium→hard
- [x] **Evaluation harness** — run model against test set, report metrics with uncertainty
      `examples/eval_harness.anwe` — 5 test cases, confidence on each, FieldAccess aggregation
- [x] **Hyperparameter search** — coordinate multiple training runs, compare results
      `examples/hyperparam_search.anwe` — 3 configs parallel, FieldAccess + converge for winner

---

## 3. MEMORY & KNOWLEDGE

ANWE has temporal decay and irreversible history as native primitives.
This is the domain where it should be untouchable.

- [x] **RAG retrieval with confidence** — retrieve docs with relevance scores
      Built in rag_pipeline.anwe
- [x] **Knowledge graph traversal** — follow edges with confidence, accumulate evidence
      `examples/knowledge_graph.anwe` — 3-hop traversal, confidence decays with distance via half_life
- [x] **Working memory with decay** — hold recent context, let old items fade naturally
      `examples/working_memory.anwe` — half_life per memory, background maintenance, decay check
- [x] **Long-term memory consolidation** — background process that merges and prunes memories
      `examples/memory_consolidation.anwe` — merge related, prune weak, transfer to long-term
- [x] **Episodic memory retrieval** — "what happened last time I saw this pattern?"
      `examples/episodic_memory.anwe` — history_query by pattern + temporal decay, cross-episode analysis
- [x] **Cache with temporal invalidation** — cached results expire based on half-life
      Demonstrated in working_memory.anwe — half_life metadata drives eviction decisions
- [x] **Memory deduplication** — detect and merge near-duplicate memories
      `examples/memory_dedup.anwe` — each memory scanned, cluster + merge + prune
- [x] **Cross-session persistence** — save lineage between program runs
      `examples/serialization.anwe` — save/restore agents with full lineage, session chaining

---

## 4. REASONING & PLANNING

This is where "attending not processing" must prove itself.
Reasoning is not a function call. It's attention moving through a problem.

- [x] **Chain-of-thought** — step-by-step reasoning where each step builds on the last
      Demonstrated in prompt_chain.anwe — FieldAccess chains data across steps
- [x] **Tree-of-thought** — branch into multiple reasoning paths, prune bad ones
      `examples/tree_of_thought.anwe` — 3 branches, reject prunes wrong, converge picks winner
- [x] **Self-consistency** — generate N answers, find consensus
      `examples/self_consistency.anwe` — 3 samples, converge (<<>>), voter reflection
- [x] **Planning with uncertainty** — decompose goal, assign confidence to each step
      `examples/planning.anwe` — 4-step plan, confidence compounds across steps
- [x] **Goal decomposition** — break a goal into sub-goals managed by supervision tree
      `examples/goal_decomposition.anwe` — 5 sub-goals, rest_for_one supervision
- [x] **Backtracking** — reject a reasoning path, restore state, try alternative
      `examples/backtracking.anwe` — reject as path abandonment, commit as checkpoint
- [x] **Metacognition** — "was my reasoning sound? should I try differently?"
      Demonstrated in self_consistency.anwe — Voter self-link reflection on consensus robustness
- [x] **Hypothesis debate** — two reasoning agents argue, third judges
      `examples/hypothesis_debate.anwe` — Proponent vs Opponent, Judge via converge

---

## 5. SAFETY & ALIGNMENT

Every item here is something Python hacks together with logging and if-statements.
ANWE should make safety structural — built into the language, not bolted on.

- [x] **Confidence calibration** — every output carries calibrated confidence
      `examples/confidence_calibration.anwe` — 3 bins, predicted vs actual, ECE computation
- [x] **Hallucination detection** — check every claim against retrieved evidence
      `examples/hallucination_detection.anwe` — 4 claims checked, 2 verified, 1 unsupported, 1 hallucinated
- [x] **Content filter** — reject unsafe input before it reaches the model
      Demonstrated in guardrail.anwe — reject (<=) gates both input and output
- [x] **Audit trail** — complete record of every decision and why
      Demonstrated in guardrail.anwe — every commit records stage, decision, safety score
- [x] **Bias detection** — monitor outputs over time for systematic bias
      `examples/bias_detection.anwe` — each group tested, disparity analysis, if/else reporting
- [x] **Uncertainty quantification** — "I'm 73% confident, here's why"
      Demonstrated in eval_harness.anwe — confidence on every evaluation, aggregated
- [x] **Explainability trace** — which signals caused which commits
      `examples/explainability.anwe` — 4-step loan decision with complete causal trace
- [x] **Human-in-the-loop gate** — pending? until a human approves
      `examples/human_in_the_loop.anwe` — pending? receiver_not_ready, human review, approval

---

## 6. MULTI-MODEL COORDINATION

Not agents talking. Models working together.
Ensemble. Cascade. Distillation. Routing.

- [x] **Ensemble** — N models answer, merge into consensus with confidence
      Demonstrated in self_consistency.anwe — 3 models, converge, confidence-weighted
- [x] **Model cascade** — cheap model first, escalate to expensive if uncertain
      Demonstrated in model_router.anwe — priority-ordered model paths
- [x] **Speculative decoding** — small model drafts, large model verifies
      `examples/speculative_decoding.anwe` — 3 batches, DraftModel→VerifyModel, reject bad drafts
- [x] **Knowledge distillation** — large model teaches small model through signal exchange
      `examples/distillation.anwe` — Teacher→Student curriculum learning via link priorities
- [x] **A/B testing** — route traffic between model versions, track metrics
      `examples/hyperparam_search.anwe` — 3 parallel configs, metric collection, converge comparison
- [x] **Model comparison** — run same input through N models, compare with uncertainty
      Demonstrated in self_consistency.anwe — same query to 3 models, compare answers
- [x] **Adaptive router** — learn which model handles which query type best over time
      `examples/adaptive_router.anwe` — if/else on complexity, reject fast model, route to premium

---

## 7. PERCEPTION & SENSOR FUSION

Where ANWE's signal metaphor is literally what the domain needs.
Sensors produce signals. Attention prioritizes them. This is native territory.

- [x] **Multi-modal fusion** — combine vision + text + audio into unified understanding
      `examples/multimodal_fusion.anwe` — 3 modalities, attention-weighted converge, cross-modal agreement
- [x] **Sensor stream processing** — continuous input with real-time attention prioritization
      `examples/sensor_stream.anwe` — 4 sensors, continuous mode, attention budget backpressure
- [x] **Anomaly detection** — flag unusual patterns in data streams
      `examples/anomaly_detection.anwe` — baseline modeling, z-score scoring, pattern classification
- [x] **Real-time filtering** — drop low-priority signals when attention budget is exhausted
      `examples/sensor_stream.anwe` — budget exhaustion drops low-priority, high-priority always processed
- [x] **Confidence-weighted fusion** — merge sensor readings weighted by their confidence
      `examples/confidence_fusion.anwe` — GPS+WiFi+IMU, confidence-proportional weights, uncertainty reduction
- [x] **Temporal alignment** — sync signals arriving at different rates from different sources
      `examples/temporal_alignment.anwe` — audio 16kHz + video 30fps + text 2Hz, rate adaptation + interpolation

---

## 8. DISTRIBUTED AI

When AI runs across machines, networks, and edges.
Sync becomes real network sync. Lineage becomes real state transfer.

- [x] **Federated learning** — multiple nodes train locally, sync periodically
      `examples/federated_learning.anwe` — 4 nodes, local training, DP noise, sync_all + federated averaging
- [x] **Edge/cloud split** — run perception at edge, reasoning in cloud
      `examples/edge_cloud.anwe` — edge detection 15ms, cloud reasoning 180ms, bridge connects transparently
- [x] **Model sharding** — split a large model across multiple nodes
      `examples/model_sharding.anwe` — 70B model across 4 A100 GPUs, pipeline-parallel inference
- [x] **Swarm coordination** — many simple agents, emergent collective behavior
      `examples/swarm_coordination.anwe` — 8 scouts, ring topology, gossip sharing, converge results
- [x] **Consensus protocol** — distributed models agree on a decision
      `examples/consensus_protocol.anwe` — 5 validators, reputation-weighted vote, quorum, audit trail
- [x] **Gossip propagation** — knowledge spreads through a network over time
      `examples/gossip_propagation.anwe` — 6-node mesh, hop-by-hop propagation, confidence decay, dedup

---

## 9. MONITORING & OPERATIONS

The unsexy stuff that every production AI system needs.
If ANWE can express this natively, it replaces not just Python but half of MLOps.

- [x] **Model drift detection** — compare current performance against decaying baseline
      `examples/model_drift.anwe` — half_life baseline, 3 evaluation windows, drift flagged at week 8
- [x] **Performance dashboard** — continuous metrics with uncertainty bands
      `examples/perf_dashboard.anwe` — 2 models, metric collection, aggregation, if/else alerting
- [x] **Alert escalation** — background monitoring → low → high → critical as issues worsen
      `examples/alert_escalation.anwe` — 5 priority levels, background→critical escalation path
- [x] **Auto-scaling** — spawn more capacity when attention budgets are exhausted
      `examples/dynamic_agents.anwe` — spawn/retire workers based on load, elastic capacity
- [x] **Health checks** — periodic self-assessment of pipeline components
      Demonstrated in working_memory.anwe — background self-link for memory maintenance
- [x] **Cost tracking** — map attention budget consumption to actual compute cost
      `examples/cost_tracking.anwe` — arithmetic cost computation, if/else budget check, ledger

---

## LANGUAGE FEATURES NEEDED

These are the gaps that building the above will force us to fill.
Each one is a language design decision, not just an implementation task.

### Must Have (blocks 70% of the list)
- [x] **Data flow between links** — output of one link feeds into another
      Works via agent_data + FieldAccess (Agent.field). Proven in prompt_chain.anwe.
- [x] **Variables with scope** — read/write values that persist across links
      agent_data persists across links. Apply writes, FieldAccess reads.
- [x] **Iteration** — for-each, repeat-until, bounded loops
      `each <var> in <collection> { ... }` — implemented in parser + engine
- [x] **Conditional routing** — if condition, execute link A else link B
      `if <condition> { ... } else { ... }` — implemented in parser + engine

### Important (blocks 20% of the list)
- [x] **Numeric expressions** — arithmetic, comparison, assignment
      `+`, `-`, `/`, `%` operators with precedence — implemented in parser + engine
- [x] **Collections** — lists and maps as first-class values
      `[expr, expr, ...]` list literals + `expr[index]` access — implemented in parser + engine
- [x] **Module system** — import agents, patterns, links from other .anwe files
      `examples/module_import.anwe` — import as namespace, cross-module agent composition
- [x] **Dynamic agent creation** — spawn agents at runtime, not just declaration time
      `examples/dynamic_agents.anwe` — spawn from template, retire on cooldown

### Needed for Production
- [x] **Time-based triggers** — "run every N ticks" or "run after delay"
      `examples/time_triggers.anwe` — every/after scheduling, periodic health + drift + cache
- [x] **Multi-party sync** — sync across 3+ agents, not just pairs
      `examples/multiparty_sync.anwe` — sync_all with quorum, barrier semantics, broadcast
- [x] **Network transport bridge** — signal routing over TCP/gRPC/WebSocket
      `examples/network_bridge.anwe` — bridge() keyword, remote agents as local participants, timeout→pending?
- [x] **Serialization** — save and restore program state, lineage, history
      `examples/serialization.anwe` — save/restore with lineage preservation, session chaining
- [x] **Error recovery** — what happens when a bridge participant actually crashes
      `examples/error_recovery.anwe` — supervision + circuit breaker + fallback chain + graceful degradation

---

## BUILD ORDER

The order that teaches us the most about the language with each step.

**Phase 1: Foundation** (forces data flow + variables + routing)
1. Prompt chain — sequential LLM calls with data passing
2. Multi-model router — conditional routing based on confidence
3. Guardrail pipeline — reject semantics in a real safety context

**Phase 2: Iteration** (forces loops + collections)
4. Evaluation harness — run model against test set, aggregate metrics
5. Self-consistency — generate N answers, find consensus
6. Working memory with decay — collection that auto-expires items

**Phase 3: Learning** (forces numeric computation + mutation)
7. Training loop — the ultimate test of "language of AI"
8. Hyperparameter search — parallel parameterized execution
9. Knowledge distillation — lineage transmission between model generations

**Phase 4: Breadth** (cover remaining buildable items)
10. Speculative decoding, Fallback chain, Token budget
11. Confidence calibration, Hallucination detection, Explainability
12. Tree-of-thought, Hypothesis debate, Human-in-the-loop
13. Alert escalation, Knowledge graph, Memory consolidation
14. Model drift, Planning, Goal decomposition, Backtracking

**Phase 5: Language Features + Unblocked Items** (implement iteration, conditionals, arithmetic, collections)
15. Language: `each <var> in <expr> { }`, `if <cond> { } else { }`, arithmetic (`+`,`-`,`/`,`%`), lists (`[...]`)
16. Streaming assembler, Tool orchestrator, Batch inference, Bias detection
17. Adaptive router, Cost tracking, Memory dedup, Performance dashboard

**Phase 6: Language Completion** (module system, dynamic agents, time triggers, multi-party sync)
18. Module system — import/as namespace composition
19. Dynamic agent creation — spawn/retire at runtime
20. Time-based triggers — every N ticks, after delay
21. Multi-party sync — sync_all with quorum + broadcast
22. Network transport bridge — bridge() for TCP/gRPC
23. Serialization — save/restore lineage across sessions
24. Error recovery — supervision + circuit breaker + graceful degradation

**Phase 7: Perception & Sensors** (ANWE's native territory)
25. Multi-modal fusion — vision + audio + text → attention-weighted convergence
26. Sensor stream processing + real-time filtering — continuous mode + backpressure
27. Anomaly detection — baseline + z-score + pattern classification
28. Confidence-weighted fusion — proportional weighting + uncertainty reduction
29. Temporal alignment — multi-rate sync + interpolation

**Phase 8: Distributed AI** (network-scale coordination)
30. Federated learning — local train + DP + sync_all
31. Edge/cloud split — latency-aware routing via bridge
32. Model sharding — pipeline-parallel across GPUs
33. Swarm coordination — scouts + gossip + convergence
34. Consensus protocol — validators + weighted vote + quorum
35. Gossip propagation — mesh topology + hop decay

**Phase 9: Remaining Items** (close the last gaps)
36. Distributed gradient sync — 4-worker all-reduce
37. Episodic memory — history_query with pattern + decay
38. Cross-session persistence — save/restore with lineage
39. Auto-scaling — spawn/retire based on load

---

## SCOREBOARD

| Domain | Items | Done | In Progress | Blocked |
|---|---|---|---|---|
| Inference & Serving | 9 | 9 | 0 | 0 |
| Training & Learning | 8 | 8 | 0 | 0 |
| Memory & Knowledge | 8 | 8 | 0 | 0 |
| Reasoning & Planning | 8 | 8 | 0 | 0 |
| Safety & Alignment | 8 | 8 | 0 | 0 |
| Multi-Model | 7 | 7 | 0 | 0 |
| Perception & Sensors | 6 | 6 | 0 | 0 |
| Distributed AI | 6 | 6 | 0 | 0 |
| Monitoring & Ops | 6 | 6 | 0 | 0 |
| **Language Features** | **12** | **12** | **0** | **0** |
| **TOTAL** | **78** | **78** | **0** | **0** |

---

*Last updated: 2026-02-25*
*ALL 78 ITEMS COMPLETE. Phases 1–9 done.*
*Language features: iteration, conditionals, arithmetic, collections, modules, dynamic agents, time triggers, multi-party sync, network bridge, serialization, error recovery.*
*Every domain covered: inference, training, memory, reasoning, safety, multi-model, perception, distributed, monitoring.*
*The scoreboard is full. ANWE is the language of AI.*
