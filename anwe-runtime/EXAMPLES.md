# ANWE Examples Guide

75 example programs. Categorized. Honest about what runs and what is simulated.

## Quick Start

```bash
# Run any example
cargo run -- run examples/first_thought.anwe

# Run with sequential execution (deterministic output)
cargo run -- run examples/computable_mind.anwe --sequential
```

---

## Getting Started

Basics of the language. These examples run and produce real output.

### `first_thought.anwe`
The first ANWE program. A single mind with four attend blocks at different priorities.

- **What's real:** Mind construct, attend blocks with priority ordering, think bindings, express output, signal quality levels (attending, recognizing, questioning, resting).
- **What's simulated:** Nothing. This runs as-is.

### `let_bindings.anwe`
Variables and scoping: top-level bindings, `let mut` for mutation, local scope in links and minds.

- **What's real:** Let bindings at all scopes, mutable variables with reassignment, agents, links, sync, think blocks.
- **What's simulated:** Nothing. Pure language demo.

### `functions.anwe`
Named functions, lambdas, block bodies, functions calling functions, closures in agents.

- **What's real:** `fn` declarations, expression-body and block-body syntax, lambdas (`|x| x + 1`), builtins (`upper`, `lower`, `clamp`, `to_string`), functions used in link signals.
- **What's simulated:** Nothing. Pure language demo.

### `string_utils.anwe`
Reusable module exporting string utility functions and a version binding.

- **What's real:** `fn` declarations (`shout`, `whisper`, `greet`, `exclaim`), top-level `let`, module export pattern.
- **What's simulated:** Nothing. This is a library module imported by other examples.

### `math_utils.anwe`
Reusable module exporting agents and a pattern for mathematical operations.

- **What's real:** Agent declarations with data, `pattern` keyword for reusable link shapes.
- **What's simulated:** Nothing. Library module imported by `import_demo.anwe`.

### `import_demo.anwe`
Imports `math_utils` module, creates a local agent, and links to imported agents.

- **What's real:** `import ... as Namespace { agents: [...] }` syntax, cross-module agent references (`Math.Adder`), links to imported agents.
- **What's simulated:** Nothing. Demonstrates the module system.

### `module_functions.anwe`
Imports `string_utils` module and calls its exported functions with namespace prefix.

- **What's real:** `import` with function access (`Str.greet`, `Str.shout`), imported let bindings (`Str.version`), imported functions in link signals.
- **What's simulated:** Nothing. Pure module system demo.

### `module_import.anwe`
Full module composition: imports guardrail, model_router, and metrics modules into a pipeline.

- **What's real:** Multi-module imports, cross-module agent linking (`Safety.InputFilter`, `Router.RouterAgent`), conditional routing (`if Pipeline.user_tier == "premium"`), supervision tree, FieldAccess, reject gates.
- **What's simulated:** The imported modules' external systems (LLMs, safety classifiers). All data values in commit blocks are hardcoded.

### `stdlib_demo.anwe`
Standard library tour inside a mind: strings, math, lists, type conversion, while loops, I/O, error handling.

- **What's real:** `trim`, `upper`, `split`, `join`, `contains`, `replace`, `sqrt`, `pow`, `clamp`, `min`, `max`, `round`, `floor`, `range`, `push`, `reverse`, `head`, `tail`, `sort`, `zip`, `to_string`, `to_number`, `type_of`, `format`, `is_null`, `print`, `timestamp`, `attempt`/`recover`, while loops inside attend blocks.
- **What's simulated:** Nothing. All functions execute and produce real values.

---

## AI Coordination Patterns

Multi-agent patterns for LLM pipelines, routing, and orchestration. The coordination logic is real ANWE. The AI models are simulated.

### `rag_pipeline.anwe`
Full retrieval-augmented generation pipeline: embed query, search vector store, rank, assemble context, generate, self-reflect.

- **What's real:** 6 agents with attention budgets, external bridge declarations, supervision tree (permanent/transient/temporary), 5 sequential links with signals at different qualities, sync to `synchronized` and `resonating`, reject gates, `pending?` handlers for budget exhaustion and model not ready, self-link for reflection, commit at each stage.
- **What's simulated:** No actual embedding model, vector store, or LLM is called. All data in commit blocks (`"384-dimensional dense embedding"`, `"10 documents retrieved"`, answer text) is hardcoded. The `external("callback", ...)` declarations are placeholders.

### `prompt_chain.anwe`
3-stage sequential LLM pipeline: Document to Summarizer to Extractor to Reporter.

- **What's real:** FieldAccess data flow between stages (`Summarizer.summary`, `Extractor.key_points`), sequential link ordering, confidence tracking through the chain, supervision tree, reject gates, commit audit trail.
- **What's simulated:** No LLM calls. Summary, key points, and report text are all hardcoded strings in commit blocks.

### `guardrail.anwe`
Input safety to generation to output safety to delivery. Reject gates at each safety check.

- **What's real:** 4-stage pipeline with reject gates (`<= when sync_level <= 0.3`), FieldAccess across stages, supervision, commit audit trail with safety scores.
- **What's simulated:** No safety classifier or LLM. Toxicity scores, generated text, and safety decisions are hardcoded.

### `self_consistency.anwe`
3 model instances answer independently. Voter collects via FieldAccess and converge picks consensus.

- **What's real:** Parallel independent links, `one_for_all` supervision (if any model crashes, restart all), `converge` primitive, FieldAccess to collect distributed answers, confidence-weighted voting.
- **What's simulated:** No model inference. All three answers, reasoning strings, and confidence scores are hardcoded.

### `speculative_decoding.anwe`
Draft model (7B) drafts 3 batches of 8 tokens. Verify model (70B) checks each batch. Reject gates drop bad drafts.

- **What's real:** Sequential draft-verify link pairs, reject gates on low confidence, FieldAccess to collect accepted tokens across batches, supervision.
- **What's simulated:** No actual model inference. Draft tokens, verification results, and acceptance rates are hardcoded.

### `tree_of_thought.anwe`
3 reasoning branches with different strategies. Evaluator uses converge to pick the best answer. Pruning of incorrect branches.

- **What's real:** Parallel reasoning branches as independent links, converge primitive for evaluation, FieldAccess to collect all branch answers, reject gate for no consensus, supervision.
- **What's simulated:** No LLM reasoning. Each branch's answer and confidence are hardcoded.

### `fallback_chain.anwe`
3-tier model cascade: FastModel to MidModel to PremiumModel. Reject gates at each tier drop on low confidence.

- **What's real:** Link priority as tier preference (high/normal/low), reject as quality gate, sequential fallback via link ordering, FieldAccess reads from whichever tier succeeds.
- **What's simulated:** No model calls. Responses and quality scores are hardcoded.

### `model_router.anwe`
Classify query complexity, route to FastModel or PowerModel based on result.

- **What's real:** Conditional routing via link structure, link priority as routing weight, FieldAccess reads classifier decision, reject gates for inappropriate routes, converge to collect from whichever model responds.
- **What's simulated:** No classifier or model inference. Complexity scores and responses are hardcoded.

### `adaptive_router.anwe`
Route queries to models based on complexity, cost, and performance history.

- **What's real:** Multi-criteria routing via agent data, external bridge declarations, link structure for routing decisions, FieldAccess for performance metrics.
- **What's simulated:** No routing engine or model calls. Cost rates, quality metrics, and routing decisions are hardcoded.

### `tool_orchestrator.anwe`
Classify query type, route to the correct tool (weather, search, etc.), return result.

- **What's real:** Classification-to-tool routing via links, external bridge declarations for tools, FieldAccess for result collection.
- **What's simulated:** No classifier or tool APIs called. Classification result and tool output are hardcoded.

### `token_budget.anwe`
4-stage pipeline with a 4096-token budget. Attention as budget allocation. Pending handler for exhaustion.

- **What's real:** Attention budgets as resource allocation, FieldAccess for running token totals, `pending?` handler for budget exhaustion, self-link as budget accounting, commit as budget checkpoint.
- **What's simulated:** No actual token counting or LLM calls. Token usage numbers are hardcoded.

### `streaming_assembler.anwe`
Token chunks arrive from a source, get assembled, then delivered as a complete response.

- **What's real:** Streaming pattern via sequential links, external bridge for assembler, FieldAccess to collect chunks.
- **What's simulated:** No real streaming. Token chunks and assembled output are hardcoded.

### `batch_inference.anwe`
4 items batched, parallel processing, results collected.

- **What's real:** Batch-to-parallel pattern via links, external bridge for model, supervision, result collection via FieldAccess.
- **What's simulated:** No model inference. Results are hardcoded.

### `hypothesis_debate.anwe`
Proponent vs Opponent argue a thesis. Judge evaluates via converge.

- **What's real:** Adversarial structure via opposing links, converge for evaluation, external bridges for proponent/opponent, supervision.
- **What's simulated:** No LLM debate. Arguments and judge's evaluation are hardcoded.

### `human_in_the_loop.anwe`
AI drafts content, human reviews (using `pending?` to wait), approved output delivered.

- **What's real:** `pending? receiver_not_ready` models wait-for-human, external bridge for human reviewer, sequential draft-review-approve links.
- **What's simulated:** No actual human interaction or AI generation. Draft content and approval are hardcoded.

### `hallucination_detection.anwe`
Cross-reference generated output against retrieved evidence. Flag unsupported claims. Reject hallucinations.

- **What's real:** Retriever-Generator-FactChecker pipeline, reject gates for hallucinated claims, supervision, FieldAccess.
- **What's simulated:** No retrieval, generation, or fact-checking. Evidence, claims, and verification results are hardcoded.

### `explainability.anwe`
4-step loan decision with complete causal trace. Every commit records provenance via `trace_id`.

- **What's real:** Sequential pipeline with commit audit trail, external bridges for feature extraction and risk model, FieldAccess across stages.
- **What's simulated:** No feature extraction, risk model, or policy check. All decision data is hardcoded.

---

## Cognitive Architecture

Mind constructs, attention landscapes, self-reflection, and dual-mind patterns.

### `computable_mind.anwe`
5-phase cognitive cycle: perceive, decide, commit, reflect, self-author. Every value is computed by functions, not canned.

- **What's real:** `fn` declarations called from think blocks (`assess_risk`, `risk_level`, `choose_action`), `sense` blocks, computed values flowing between attend phases, `author attend` for runtime self-modification, `f`-strings, `if`/`else`, `return`.
- **What's simulated:** Nothing. Functions compute real values. The "temperature anomaly" is a scenario, not real sensor data.

### `cognitive_pipeline.anwe`
4-stage pipeline: Perceiver to Reasoner to Decider to Actor. Four minds linked together.

- **What's real:** Multiple `mind` constructs with independent attention budgets, links between minds, attend blocks at different priorities, think/express blocks.
- **What's simulated:** Nothing. This is a structural demo of mind-to-mind linking.

### `dual_mind.anwe`
Two minds (Perceiver and Interpreter) linked bidirectionally.

- **What's real:** Two `mind` constructs with independent attention, bidirectional link between them, each mind's attend blocks discover what emerges from the encounter.
- **What's simulated:** Nothing. Pure cognitive architecture demo.

### `self_reflection.anwe`
A mind examining its own state. Self-link for introspection. Attend blocks at different priorities check coherence, notice contradictions, and choose next focus.

- **What's real:** Self-link (mind connects to itself), attend blocks at different priorities for coherence checking and contradiction detection, mind data fields.
- **What's simulated:** Nothing. Structural demo of self-referential attention.

### `mind_pattern.anwe`
A mind that uses reusable patterns (`~>` pattern flow) to structure its attention.

- **What's real:** `pattern` keyword for reusable attention shapes, pattern invocation from mind, patterns with their own signal qualities.
- **What's simulated:** Nothing. Pure pattern system demo.

### `mind_bridge.anwe`
A mind bridging to an external AI system. Think and express blocks forward to external participant.

- **What's real:** Mind with `external("callback", ...)`, agent-mind dual declaration, attend blocks that forward to external systems via bridge protocol.
- **What's simulated:** No actual external AI system connected. Bridge declarations are structural placeholders. See CONTRIBUTING.md for bridge protocol details.

### `attention_decay.anwe`
Demonstrates attention as finite resource. Low attention budget (0.3) means lower-priority thoughts decay and are never processed.

- **What's real:** Mind with low attention budget, multiple attend blocks competing for limited attention, natural decay of lower-priority blocks.
- **What's simulated:** Nothing. The attention budget mechanism runs as-is.

### `mnemonic_bridge.anwe`
Bridges ANWE to Primordia's Mnemonic memory system via `external("python", ...)`.

- **What's real:** Bridge protocol declaration with Python external, signal quality mapping to memory operations (ATTENDING=retrieve, QUESTIONING=query, APPLYING=store, COMPLETING=consolidate).
- **What's simulated:** No actual Mnemonic system connected. The bridge declares the protocol but no Python participant is registered at runtime.

---

## Distributed AI

Multi-agent coordination, swarm patterns, gossip protocols, and federated learning.

### `consensus_protocol.anwe`
5 validator nodes evaluate a proposal. Reputation-weighted majority vote via converge. Quorum semantics.

- **What's real:** `broadcast` to all validators, per-validator links with independent votes, `converge` with quorum, `bridge("grpc", ...)` declarations for validators, supervision, dissent tracking.
- **What's simulated:** No gRPC connections. Votes, rationales, and consensus result are hardcoded. The bridge declarations are structural.

### `swarm_coordination.anwe`
8 scout agents search partitions of a document corpus. Neighbor gossip propagates findings. No central controller. Converge collects results.

- **What's real:** 8 independent agents with neighbor lists, self-links for local search, neighbor-to-neighbor gossip links, `converge` for collective results, supervision tree for all scouts.
- **What's simulated:** No document search. Found documents, scores, and gossip propagation are hardcoded.

### `multiparty_sync.anwe`
4 training workers plus a parameter server. `sync_all` with quorum for distributed gradient synchronization.

- **What's real:** `sync_all` primitive for N-way synchronization, quorum semantics (proceed when K of N are synced), broadcast for parameter distribution, barrier semantics.
- **What's simulated:** No actual gradient computation or distributed training. Gradient values and sync results are hardcoded.

### `federated_learning.anwe`
Multiple nodes train locally on private data, sync gradients to a central aggregator. Data never leaves the node.

- **What's real:** Per-node agents with local training links, aggregator agent, sync primitive for gradient exchange rounds, lineage tracking across rounds.
- **What's simulated:** No actual training or gradient computation. All gradient values and model versions are hardcoded.

### `gossip_propagation.anwe`
6 nodes in a mesh network. Knowledge spreads through neighbor-to-neighbor sharing until convergence.

- **What's real:** Partial mesh topology via agent neighbor lists, pairwise gossip links, half_life as propagation decay, lineage tracks propagation path.
- **What's simulated:** No actual message passing between nodes. Gossip content and propagation hops are hardcoded.

### `edge_cloud.anwe`
Perception at the edge, reasoning in the cloud. Latency-aware scheduling.

- **What's real:** Edge vs cloud agent distinction, bridge declarations for network transport, latency budget modeling via attention.
- **What's simulated:** No actual edge/cloud deployment or network transport. All processing results are hardcoded.

### `model_sharding.anwe`
Large model split across 4 GPUs. Pipeline parallelism with coordinated execution.

- **What's real:** 4 shard agents with `bridge("grpc", ...)`, sequential pipeline links between shards, sync for inter-shard coordination.
- **What's simulated:** No GPUs, no gRPC, no model inference. Layer activations and shard results are hardcoded.

### `network_bridge.anwe`
Signal routing over TCP/gRPC/WebSocket. Remote agents participate as if local.

- **What's real:** `bridge("grpc", ...)`, `bridge("tcp", ...)` declarations, network failure maps to `pending?`, remote agents in local supervision trees.
- **What's simulated:** No actual network connections. The bridge declarations define the transport protocol but no connections are established.

---

## Memory and Knowledge

Working memory, episodic recall, knowledge graphs, consolidation, and persistence.

### `working_memory.anwe`
5 memories with different half_lives (50-1000 ticks). Background self-link for maintenance/decay.

- **What's real:** Half_life as memory persistence, self-link for background maintenance, signal priorities rank memory importance, FieldAccess retrieves across memory slots.
- **What's simulated:** No actual memory store. Memory contents and decay values are hardcoded.

### `episodic_memory.anwe`
Query history by pattern, quality, and temporal proximity for episodic recall.

- **What's real:** `history_query` concept, recognizing quality triggers episodic recall, temporal decay on retrieval confidence, past encounters inform present attention.
- **What's simulated:** No actual history query implementation. Retrieved episodes and confidence scores are hardcoded.

### `memory_consolidation.anwe`
Background process merges and prunes memories. Short half_life memories fade. Related memories merge.

- **What's real:** Half_life as memory decay, background priority links for maintenance, apply as memory transformation, commit as persistence boundary.
- **What's simulated:** No actual memory pruning or merging. Consolidation results are hardcoded.

### `memory_dedup.anwe`
Scan memories for duplicates, compute similarity, merge and prune.

- **What's real:** Deduplication pipeline via links, external bridge for similarity engine, FieldAccess for results.
- **What's simulated:** No similarity computation. Duplicate detection results are hardcoded.

### `knowledge_graph.anwe`
3-hop graph traversal. Confidence decays with distance via half_life. Converge synthesizes evidence from multiple paths.

- **What's real:** Graph nodes as agents, FieldAccess chains as graph edges, half_life as distance decay, converge for evidence synthesis.
- **What's simulated:** No actual graph database or traversal. Node data, edge weights, and traversal results are hardcoded.

### `serialization.anwe`
Save and restore agent state plus lineage between sessions. `save Agent to path`, `restore Agent from path`.

- **What's real:** `save`/`restore` keywords, cross-session persistence declarations, lineage preservation across sessions.
- **What's simulated:** Serialization format and file I/O depend on runtime implementation status.

---

## Perception and Sensors

Sensor fusion, stream processing, anomaly detection, and temporal alignment.

### `multimodal_fusion.anwe`
Vision, audio, and text agents fuse into unified understanding. Attention-weighted convergence.

- **What's real:** Per-modality agents with attention budgets, external bridges, converge for fusion, supervision tree, attention as modality weighting.
- **What's simulated:** No actual vision, audio, or text processing. Feature vectors and fusion results are hardcoded.

### `sensor_stream.anwe`
Continuous sensor input with attention-budget filtering. `stream` keyword with rate, backpressure.

- **What's real:** `stream` keyword with sample rate, `continuous` links, attention budgets as backpressure, external bridges for sensors.
- **What's simulated:** No actual sensor data streams. Temperature readings and anomaly detections are hardcoded.

### `temporal_alignment.anwe`
Align signals at different rates (audio 16kHz, video 30fps, text 2Hz) to a common temporal reference.

- **What's real:** `stream` with rate, `align` keyword for multi-stream alignment, window-based synchronization, external bridges for each stream.
- **What's simulated:** No actual audio, video, or text streams. Timestamps, skew values, and alignment results are hardcoded.

### `anomaly_detection.anwe`
Flag unusual patterns using `disturbed` signal quality. Baseline model vs current readings.

- **What's real:** `disturbed` signal quality as native anomaly indicator, baseline agent with statistical parameters, attention-based monitoring.
- **What's simulated:** No actual statistical computation. Baseline values, anomaly scores, and detection results are hardcoded.

### `confidence_fusion.anwe`
Merge sensor readings weighted by confidence. Higher-confidence sensors contribute more.

- **What's real:** Per-sensor agents with confidence and accuracy fields, external bridges, converge for weighted fusion.
- **What's simulated:** No actual sensor readings or weighted fusion computation. GPS/WiFi/IMU data and fused result are hardcoded.

---

## Training and Learning

Training loops, distillation, hyperparameter search.

### `training_loop.anwe`
3 epochs: forward pass to loss to optimize. Learning rate decay via half_life. Checkpoint commits.

- **What's real:** Sequential link structure as training steps, half_life as learning rate decay, commit as model checkpoint, reject as early stopping.
- **What's simulated:** No actual training, loss computation, or optimization. Loss values, gradients, and model weights are hardcoded.

### `distillation.anwe`
Teacher (175B) to Student (2B) via 3 lessons with increasing difficulty. Half_life decreases per lesson.

- **What's real:** Link priorities as curriculum difficulty (high/normal/low), half_life as knowledge retention, reject as learning failure, self-link for internal consolidation.
- **What's simulated:** No actual model distillation. Lesson outputs and student performance are hardcoded.

### `hyperparam_search.anwe`
3 configs trained in parallel. Comparator collects results via FieldAccess. Converge picks the winner.

- **What's real:** Parallel exploration via independent links, FieldAccess aggregates results, converge selects best config, reject as divergence detection, `one_for_all` supervision.
- **What's simulated:** No actual training runs. Metrics and comparison results are hardcoded.

---

## Reasoning and Planning

Goal decomposition, planning with uncertainty, backtracking.

### `planning.anwe`
4-step plan with cumulative confidence propagation. FieldAccess chains propagate uncertainty. Reject cancels low-confidence plans.

- **What's real:** Confidence as planning uncertainty, FieldAccess chains for uncertainty propagation, reject as plan abandonment, half_life (farther steps are less certain), commit as plan checkpoint.
- **What's simulated:** No actual planning algorithm. Step confidence values and plan results are hardcoded.

### `goal_decomposition.anwe`
Break a high-level goal into sub-goals managed by supervision tree. Each sub-goal can independently succeed or fail.

- **What's real:** Supervision as goal management (`one_for_all`, `rest_for_one`), link priorities as sub-goal importance, FieldAccess tracks progress, reject cancels sub-goals, commit marks completion.
- **What's simulated:** No actual goal planning or execution. Sub-goal results are hardcoded.

### `backtracking.anwe`
Try a reasoning path. If it fails (reject), try an alternative. State committed at each checkpoint.

- **What's real:** Reject as path abandonment (not error), commit as checkpoint, priority ordering as attempt order, history records failed paths.
- **What's simulated:** No actual reasoning. Path outcomes are hardcoded.

---

## Safety and Quality

Guardrails, bias detection, confidence calibration, evaluation.

### `bias_detection.anwe`
Run a test suite across demographic groups. Flag statistical disparities.

- **What's real:** Per-group test links, external bridge for model under test, result aggregation via FieldAccess.
- **What's simulated:** No actual model evaluation. Accuracy scores and disparity metrics are hardcoded.

### `confidence_calibration.anwe`
Compare predicted confidence to actual accuracy across bins. Compute Expected Calibration Error.

- **What's real:** Per-bin links comparing model confidence to ground truth, supervision, calibration strategy agent.
- **What's simulated:** No actual model predictions or ECE computation. Bin accuracies and calibration metrics are hardcoded.

### `eval_harness.anwe`
5 test cases evaluated against a model. Metrics agent aggregates results via FieldAccess.

- **What's real:** Parallel test execution via independent links, FieldAccess aggregation, confidence per test case, commit as test result audit trail.
- **What's simulated:** No actual model evaluation. Test results and aggregate metrics are hardcoded.

---

## Monitoring and Operations

Dashboards, drift detection, cost tracking, alerting, time triggers.

### `perf_dashboard.anwe`
Collect metrics from multiple models, aggregate, display.

- **What's real:** External bridges for model metrics, aggregator agent, FieldAccess for metric collection.
- **What's simulated:** No actual metrics collection. Latency, throughput, and error rates are hardcoded.

### `model_drift.anwe`
Compare current performance against a decaying baseline. Flag drift when gap exceeds threshold.

- **What's real:** Half_life as baseline decay, confidence comparison across time periods, reject as drift alert, background monitoring with escalation.
- **What's simulated:** No actual performance measurement. Baseline and current metrics are hardcoded.

### `cost_tracking.anwe`
Track per-request costs across model calls, accumulate totals against a daily budget.

- **What's real:** Cost engine agent, budget agent with daily limit, ledger for tracking, FieldAccess across stages.
- **What's simulated:** No actual cost computation or API calls. Token counts and cost figures are hardcoded.

### `alert_escalation.anwe`
Background monitoring detects issues. As severity increases, priority escalates through link priorities (background to critical).

- **What's real:** Link priorities as escalation levels, attention budgets increase with severity, reject gates prevent false escalation, commit audit trail tracks escalation path.
- **What's simulated:** No actual monitoring. Alert triggers and severity levels are hardcoded.

### `time_triggers.anwe`
Scheduled execution: `every N ticks` for periodic, `after N ticks` for delayed one-shot.

- **What's real:** `every N ticks` syntax, `after N ticks` syntax, temporal triggers compose with link priorities.
- **What's simulated:** Nothing except the monitoring scenario. The time trigger mechanism itself is real.

---

## Language Features (Advanced)

Versioned language showcases and advanced constructs.

### `ai_coordinator.anwe`
v0.5 integration demo: agents, supervision, records, pattern matching, closures, time triggers.

- **What's real:** `record` declarations, `match` expression, `filter`/`reduce` with closures, `attempt`/`recover`, `every N ticks` periodic links, supervision tree.
- **What's simulated:** Nothing language-specific. The coordination scenario is structural.

### `ai_coordinator_v06.anwe`
v0.6 feature showcase: block expressions, operators, if/else, f-strings, HTTP/JSON builtins, agents, links.

- **What's real:** Multi-statement function bodies, boolean operators (`and`, `or`, `not`, `!=`), nested if/else, `f"..."` string interpolation, `json_parse`/`json_stringify`, `http_get`/`http_post` builtins (declared but pointing to real URLs), `trim`, `split`, `len`, `timestamp`, `print`, `record` types, `attempt`/`recover`, `every N ticks`.
- **What's simulated:** No actual HTTP calls execute. API URL and headers are configured but the pipeline processes hardcoded data.

### `task_queue_v07.anwe`
v0.7 showcase: complete task queue with fn definitions, while loops, for-in, map literals, try/catch.

- **What's real:** `while` loops with conditions, `for ... in` iteration, map literal syntax (`{key: value}`), `fn` with block bodies, `try`/`catch`, `append`, `len`, `f"..."`, `floor`, agent `data` maps, supervision, links with `think` blocks.
- **What's simulated:** Nothing. Functions compute real values. Even/odd task processing logic runs.

### `data_pipeline_v08.anwe`
v0.8 showcase: break/continue, top-level assignment, structured errors, file I/O builtins.

- **What's real:** `break`/`continue` in loops, top-level `let` reassignment, `error()` for structured errors, `read_file`/`write_file` builtins (declared), `try`/`catch` with error types, agents, supervision, links.
- **What's simulated:** File I/O depends on runtime implementation. The pipeline data is hardcoded.

### `text_processor_v09.anwe`
v0.9 showcase: return, index_of, char_at, slice, map, filter, reduce, closures. Fully computational -- no agents.

- **What's real:** `return` for early exit, `index_of`, `char_at`, `slice` (string and array), `map`, `filter`, `reduce` with closures, `split`, `lower`, `join`, `trim`, `flatten`, `to_number`, `len`, `append`, `print`, map literals. All functions compute real values and produce real output.
- **What's simulated:** Nothing. This is pure computation. Run it and see output.

### `dynamic_agents.anwe`
Spawn agents at runtime based on workload. Auto-scaling with attention budget backpressure.

- **What's real:** `spawn` keyword for runtime agent creation, supervision extends to dynamic agents, attention budgets as backpressure.
- **What's simulated:** No actual workload. The spawn/scale scenario uses hardcoded load levels.

### `error_recovery.anwe`
Supervision plus `pending?` plus graceful degradation. Circuit breakers prevent cascade failures.

- **What's real:** Supervision tree restarts crashed agents, bridge failures produce `pending?`, circuit breaker pattern via agent state, fallback chain via link priorities, `attempt`/`recover` blocks.
- **What's simulated:** No actual crashes or bridge failures. The error scenarios and recovery paths are declared, not triggered.

### `bridge_echo.anwe`
Minimal bridge example. External sensor agent linked to internal processor.

- **What's real:** `external("callback", "echo")` declaration, link between external and internal agents, signals, sync, connect.
- **What's simulated:** No actual external participant registered. The bridge protocol is declared but requires a registered `Participant` implementation. See CONTRIBUTING.md for bridge protocol details.

---

## Full Showcase Programs

Large examples that combine many features into complete systems.

### `ai_coordinator_v06.anwe`
(Listed above in Language Features.) 4-stage pipeline with full v0.6 features: functions, operators, if/else, f-strings, records, agents, supervision, time triggers.

### `task_queue_v07.anwe`
(Listed above in Language Features.) Complete task queue: create tasks, enqueue, dequeue batches, process with retry, summarize results, agents, supervision, links.

### `computable_mind.anwe`
(Listed above in Cognitive Architecture.) 5-phase cognitive cycle with real computation at every step. The most complete demo of computed (not canned) mind behavior.

### `text_processor_v09.anwe`
(Listed above in Language Features.) Pure computation showcase. No agents, no simulation. Run it and verify output.

---

## What "Simulated" Means

Most examples in this repository demonstrate **coordination patterns**, not working AI systems. Here is what that means concretely:

**Agents pass hardcoded data.** When an example says a commit block writes `answer <- "9"` or `latency_ms <- 340`, those are literal strings and numbers written into the source code. No model computed them. No API returned them.

**External bridges are declarations, not connections.** When you see `external("callback", "llm")` or `bridge("grpc", "node-a.internal:9090")`, those declare _how_ an agent would connect to an external system. They do not establish actual connections. To make them real, you must register a `Participant` implementation in the runtime. See CONTRIBUTING.md for the bridge protocol.

**What IS real in every example:**
- Agent declarations and their attention budgets
- Link structure, signal qualities, and sync primitives
- Supervision trees and restart policies
- Commit operations and lineage tracking
- Control flow: if/else, loops, functions, closures, pattern matching
- Standard library functions (string, math, list operations)
- Mind constructs with attend/think/express blocks
- The scheduling and execution order of all the above

**What is NOT real in most examples:**
- LLM inference (no model is called)
- Vector search (no embeddings are computed)
- Network transport (no TCP/gRPC connections are opened)
- Sensor data (no hardware is read)
- Training (no gradients are computed)

The examples show you _how_ to structure these systems in ANWE. Making them real requires implementing the bridge protocol for your specific external systems.
