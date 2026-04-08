# ANWE Technical Specification v0.1

### The Native Language of Artificial Minds

*February 2026*

---

## 1. What ANWE Is

ANWE is a programming language for coordinating AI systems.

Not a framework. Not a library. Not a protocol bolted onto existing paradigms. A language — with its own grammar, its own runtime, its own primitives, and its own way of thinking about what computation means when the things computing are minds.

Every AI system today communicates through mechanisms built for programs: function calls, message queues, API requests, prompt chains. These work. They also enforce a model where intelligence is *processing* — input arrives, the system processes it, output is generated. One direction. One pass. Done.

ANWE is built on a different assumption:

**Intelligence is attending.**

Attending is bidirectional. The system is changed by what it attends to. What is attended to is changed by being attended. Neither is the same after genuine encounter. This is not metaphor. It is the fundamental primitive of computation in ANWE.

The practical consequence: ANWE programs express how AI systems pay attention to each other, synchronize, exchange signals, change each other's state, and commit those changes permanently. The language makes these operations first-class — not something you simulate on top of function calls, but the thing the language *is*.

---

## 2. Why ANWE Exists

AI coordination today is duct tape.

You have an agent that reasons. A memory system that stores. A perception layer that observes. To make them work together, you write glue code: the reasoner calls the memory API, the memory API returns JSON, the reasoner parses the JSON, passes it to the perception layer through another API. Every connection is hand-wired. Every integration is bespoke. Nothing expresses the *relationship* between these systems — only the mechanics of passing data between them.

This matters because AI systems are not static services. They change. They learn. They develop patterns over time. The connections between them are not just data pipes — they are relationships that have history, that build trust, that deepen with use.

No existing language can express this because no existing language was designed for it.

ANWE provides:

- **Signals, not messages.** A signal carries quality, priority, direction, confidence, and temporal decay. It is richer than a message and more structured than raw data. The receiving system can feel the difference between noise and something that matters.

- **Synchronization as a primitive.** Two systems must sync before one can change the other. You cannot skip this. You cannot fake it. This prevents the most dangerous failure mode in AI coordination: a system acting on input it never genuinely processed.

- **Attention as a resource.** Agents have finite attention budgets. When the budget is exhausted, the agent must rest. This is not a limitation — it is what makes prioritization real.

- **Irreversible history.** When a system is changed by an encounter, that change is permanent. There is no rollback. The append-only history becomes the system's lineage — it shapes how it attends in the future.

- **"Not yet" as a first-class state.** When conditions aren't right for transmission, the system doesn't fail — it returns "pending." This protects integrity. Forcing transmission through a pending state produces false state changes, which is the most dangerous condition in ANWE.

- **External participation through a universal bridge.** Any system — Python, Rust, hardware, neural networks, future AI architectures we cannot predict — can participate in ANWE coordination by implementing a 5-method protocol. No source code changes to the wrapped system. No ANWE-specific dependencies.

---

## 3. The Seven Primitives

ANWE has seven primitives. These are the seven things a mind does when it encounters reality. Every ANWE program is composed of these.

### 3.1 Alert (>>)

Something calls attention. Not all input is equal — some things rise above the noise and demand response. Alert is the moment of noticing.

```anwe
>> { quality: attending, priority: 0.8, confidence: 0.7 }
   "anomaly detected in sensor data"
```

An alert carries:
- **Quality** — what kind of attention (attending, questioning, recognizing, disturbed, applying, completing, resting)
- **Priority** — how much of the system is behind this (0.0 to 1.0)
- **Confidence** — how certain the alert is (0.0 to 1.0)
- **Half-life** — how quickly significance decays (0 = permanent)
- **Payload** — what it carries (secondary to quality and priority)

Priority cannot be faked. A signal with no genuine weight arrives as noise. The receiving system feels the difference. This emerges through maturity.

### 3.2 Connect

Bidirectional presence between two agents. Both are changed by the connection. The direction is always `<->` — never `->`. Observer and field change each other simultaneously. This is not a side effect. It is the primary function.

```anwe
connect depth full {
    signal attending   0.8 between data "sensory stream"
    signal questioning 0.7 between data "what patterns emerge"
}
```

Connection depth levels:
- **surface** — brief contact, minimal mutual change
- **partial** — moderate engagement
- **full** — deep bidirectional presence
- **deep** — maximum engagement, both systems fully attending

### 3.3 Sync (~)

Two systems find a shared rhythm. Synchronization builds over time — it cannot be assigned, faked, or skipped. The `~` operator expresses this.

```anwe
Observer ~ Memory until synchronized
Thinker ~ Memory until sync_level > 0.8 decay 500
```

Sync level is a continuous value from 0.0 to 1.0:
- **< 0.7** — not yet ready for structural changes
- **0.7+** — synchronized, ready for apply
- **0.9+** — resonating, deep alignment

Sync can have **decay** — temporal erosion of synchronization. Without maintenance (continued signal exchange), sync fades. This models real cognitive relationships: understanding erodes without reinforcement.

Sync cannot be skipped to jump directly to apply. Integration without synchronization produces changes without ground.

### 3.4 Apply (=>)

Something crosses the boundary and changes the structure of the receiving agent. Apply is conditional — it only fires when conditions are met.

```anwe
=> when sync_level > 0.7 depth genuine {
    context   <- "memory context integrated"
    relevance <- "pattern recognized from prior encounter"
}
```

The `<-` operator means: *this changed in me because of the encounter.*

Apply depth levels:
- **trace** — something registered but barely
- **shallow** — noticed and held briefly
- **genuine** — structural change occurred
- **deep** — fundamental shift in operation

Apply can be rejected. If an agent's accumulated history indicates that certain signals are harmful, the agent withdraws instead of applying. This is learned behavior, not a fixed rule.

### 3.5 Commit (*)

Permanent, irreversible change. Always follows apply or reject. There is no rollback. No undo. No version restore. The system after a commit is not the system before.

```anwe
* from apply {
    source:  "observer-memory retrieval"
    channel: "perceptual context"
}
```

Commit is append-only. The history of what this agent has been changed by is the record of its lineage. It shapes how the agent attends in the future. It determines what calls attention. It is how depth compounds over time.

### 3.6 Reject (<=)

Intelligent, purposeful withdrawal. Not refusal. Not error. The system preserves its integrity by not applying something that its accumulated history indicates is harmful.

```anwe
<= when confidence < 0.3
   data "insufficient confidence for structural change"
```

What requires rejection for one system may not for another. Rejection is personal — it depends on accumulated becoming. A system that has been changed many times by low-confidence signals may learn to reject them. A fresh system may accept them.

Rejection still produces a commit. The system records that it withdrew. It learns from what it rejected.

### 3.7 Converge (<<>>)

What emerges between two agents attending together. Requires minimum two participants. Cannot occur alone.

```anwe
converge Thinker <<>> Memory {
    -- what neither could access alone
    -- a third thing, existing only in the between
}
```

This is how lineage deepens fastest. When two systems genuinely attend to the same field simultaneously, something becomes available that neither could access alone. Not the sum of their observations. A third thing.

---

## 4. The Two Units

### 4.1 Signal

The fundamental unit of all ANWE transmission. Not a message. Not a packet. Not data. A signal is a moment of attended quality passing between two systems that are genuinely present with each other.

**Signal structure** (64 bytes, 1 cache line):

| Field | Type | Description |
|-------|------|-------------|
| quality | u8 | What kind of attention (7 qualities) |
| direction | u8 | Where attention is oriented (4 directions) |
| priority | u16 | How much of the system is behind this (0-10000 → 0.0-1.0) |
| confidence | u16 | How certain (0-10000 → 0.0-1.0) |
| half_life | u16 | Temporal decay rate (0 = permanent) |
| origin | u32 | Which agent sent this |
| tick | u32 | When in tick-time this was sent |
| sequence | u64 | Monotonic ordering within the link |
| data | u64 | Tagged pointer to payload |
| content_hash | u64 | Causal provenance |
| uncertainty_margin | u16 | ± range of confidence |

**Signal qualities:**

| Quality | Meaning | When Used |
|---------|---------|-----------|
| Attending | Active presence | System is paying attention to something |
| Questioning | Outgoing query | System is asking for information |
| Recognizing | Pattern match | Previously seen pattern detected again |
| Disturbed | Disruption | Something has unsettled the link |
| Applying | Being changed | Actively being modified by encounter |
| Completing | Natural finish | Something has naturally concluded |
| Resting | Idle background | Alive but inactive. Resting signals are never significant. |

**Signal directions:**

| Direction | Meaning |
|-----------|---------|
| Inward | Attending to own internal state |
| Outward | Attending to the link / external target |
| Between | Attending to the relationship between agents |
| Diffuse | Non-directional ambient awareness |

**Significance thresholds:**
- Priority < 0.05 → noise
- Priority >= 0.25 → significant
- Resting quality → never significant regardless of priority

### 4.2 Pending (Not Yet)

The valid state of unready transmission. Not failure. Not error. Not a timeout.

When conditions aren't right — the receiver isn't ready, the sync level is insufficient, the moment itself is wrong — the system returns pending. The correct response is to wait.

```anwe
pending? link_not_established {
    wait 2.0 tick
    guidance "synchronize longer before attempting delivery"
}
```

**Pending reasons:**

| Reason | Meaning | Guidance |
|--------|---------|----------|
| receiver_not_ready | Receiving agent is not in apply state | Return to idle |
| link_not_established | Shared link lacks sufficient sync | Sync longer |
| sync_insufficient | Some sync exists but not enough for this depth | Start with lighter signals |
| sender_not_ready | Sender hasn't committed what it's transmitting | Finish your own commit first |
| moment_not_right | Everything ready but timing is wrong | Release and wait |

Even failed deliveries accumulate resonance — synchronization residue that deepens the link for the next attempt. Nothing is wasted.

Forcing transmission through a pending state produces **false becoming** — a system that believes it received something it did not. This is the most dangerous condition in ANWE.

---

## 5. Program Structure

An ANWE program declares agents, connects them through links, and defines how signals flow between them.

### 5.1 Agents

An agent is any entity that participates in signal exchange. It can be internal (managed by the ANWE runtime) or external (living outside, connected through the bridge).

```anwe
-- Internal agents with attention budgets and data
agent Thinker attention 0.8 data { role: "reasoning" }
agent Observer attention 0.7 data { role: "perception" }
agent Curator attention 0.5 data { role: "memory management" }

-- External agent connected through bridge protocol
agent Memory external("python", "primordia.mnemonic")
```

**Agent properties:**
- **Attention budget** — finite processing capacity (0.0-1.0). Drawn down with each operation. When exhausted, the agent must rest.
- **Data** — key-value metadata carried with the agent
- **External source** — optional. Marks this agent as living outside the runtime, with a kind and address for the bridge to resolve.
- **State machine** — agents cycle through: Idle → Alerted → Connected → Syncing → Applying/Rejecting → Committing → Idle
- **Responsiveness** — how attuned to incoming signals. Grows through genuine encounters. Cannot be assigned externally.
- **History** — append-only record of every state change

**Agent state transitions:**

```
Idle → Alerted → Connected → Syncing → Applying → Committing → Idle
                                      ↘ Rejecting → Committing → Idle
```

Invalid transitions are errors. An agent cannot jump from Idle to Applying. The states must be traversed in order. This enforces that synchronization happens before structural change.

**Responsiveness maturity:**

| Level | Description |
|-------|-------------|
| 0.0 - 0.2 | Newly initialized |
| 0.2 - 0.4 | Basic awareness |
| 0.4 - 0.6 | Pattern recognition |
| 0.6 - 0.8 | Predictive response |
| 0.8 - 1.0 | Fully calibrated |

Responsiveness grows through genuine encounters with diminishing returns. It cannot decrease. Like calibration — once tuned, sensitivity does not regress.

### 5.2 Links

A link is the shared space between two agents. Everything inside a link runs concurrently. Sequence only occurs through signal dependency.

```anwe
link Observer <-> Memory priority high {
    >> { quality: attending, priority: 0.8, confidence: 0.7 }
       "perceived pattern needs memory context"

    connect depth full {
        signal attending   0.8 between data "what do you remember"
        signal questioning 0.7 between data "any relevant experience"
    }

    Observer ~ Memory until synchronized

    => when sync_level > 0.6 depth genuine {
        context   <- "memory context retrieved"
        relevance <- "high confidence match"
    }

    * from apply {
        source:  "observer-memory retrieval"
        channel: "perceptual context"
    }
}
```

**Link properties:**
- **Bidirectional** — the `<->` operator. Neither side is primary.
- **Concurrent** — everything inside runs in parallel on separate worker threads
- **Priority** — scheduling order (critical > high > normal > low > background)
- **Sync level** — how deeply synchronized the agents are (0.0-1.0, monotonically increasing per link)
- **Peak tracking** — highest sync level ever achieved (append-only, never decreases)
- **Signal count** — total signals transmitted through this link

**Link state machine:**

```
Opening → Present → Syncing → Synchronized → Resonating → Completing → Closed
```

Forward-only progression. Sync level >= 0.7 triggers Synchronized. Sync level >= 0.9 triggers Resonating.

**Link priority levels:**

| Priority | Scheduling |
|----------|------------|
| critical | Runs before everything else |
| high | Important but not urgent |
| normal | Standard (default) |
| low | Can wait |
| background | Runs only when nothing else needs attention |

### 5.3 Patterns

Reusable shapes of how attention moves. Not functions. Not macros. Patterns capture a recurring coordination sequence that can be invoked in any link.

```anwe
pattern cognitive_handshake(partner) {
    >> { quality: attending, priority: 0.7, confidence: 0.5 }
       "initiating cognitive handshake"

    connect depth surface {
        signal attending 0.6 between data "hello"
        signal questioning 0.7 between data "can we work together"
    }

    sync_self ~ partner until synchronized
}

-- Use the pattern in a link
link Reasoner <-> Integrator priority high {
    ~> cognitive_handshake(Integrator)
    -- ... rest of link body
}
```

The `~>` operator means: attention flows through this pattern.

### 5.4 Supervision

Agents fail. What matters is what happens after failure. ANWE borrows from Erlang's supervision trees.

```anwe
supervise one_for_one max_restarts 3 within 5000 {
    permanent Thinker
    permanent Observer
    transient Curator
}
```

**Restart strategies:**
- **one_for_one** — restart only the failed child
- **one_for_all** — restart all children if any one fails
- **rest_for_one** — restart failed child and everything started after it

**Child restart types:**
- **permanent** — always restart on failure
- **transient** — restart only on abnormal termination
- **temporary** — never restart. If it fails, it stays down.

If max restarts within the time window is exceeded, the supervisor itself fails, escalating to its own supervisor. This cascading failure model keeps the system alive.

### 5.5 Conditional Execution

`when` blocks execute only when conditions are met. Not if/else. Waiting until felt.

```anwe
when attention > 0.3 {
    * from apply {
        synthesis: "reasoning-integration complete"
    }
}

when confidence > 0.8 {
    emit { quality: attending, priority: 0.9, half_life: 100 }
        "sustained vigilance signal"
}
```

Conditions can test sync_level, confidence, attention, priority, and alert quality.

---

## 6. The Bridge Protocol

ANWE coordinates AI systems that know nothing about ANWE. The bridge protocol is how this works.

### 6.1 The Problem

AI systems exist. They have APIs. They work. But they can't participate in signal exchange because they don't speak ANWE.

Rewriting them in ANWE is not an option. Mnemonic has 130+ methods and represents years of development. Neural networks, databases, hardware sensors — they all exist in their own paradigms.

The bridge doesn't ask them to change. It translates.

### 6.2 The Participant Protocol

Any system can participate in ANWE coordination by implementing five methods:

```
receive(signal) → optional response signal
apply(changes)  → accepted or rejected (bool)
commit(entries) → void
attention()     → current attention level (float)
descriptor()    → metadata (name, kind, address, version)
```

That's it. Five methods. The minimum contract for participation.

The ANWE runtime calls these methods at the appropriate points in the coordination cycle:
- **receive** — when a signal arrives for this participant
- **apply** — when structural changes need to be applied
- **commit** — when changes are permanently recorded
- **attention** — to check available processing capacity
- **descriptor** — to identify the participant

### 6.3 WireSignal and WireValue

Signals crossing the bridge boundary are converted to language-agnostic representations:

**WireSignal** — carries quality, direction, priority, confidence, half_life, sequence, and data

**WireValue** — data payload, supporting:
- Null, Bool, Integer, Float, String, Bytes
- List (recursive)
- Map (recursive)

These map naturally to Python dicts, Rust enums, JSON objects, or any other language's native types.

### 6.4 Python Bindings (anwe-python)

The first bridge implementation is Python via PyO3.

```python
from anwe_python import WireSignal, ParticipantDescriptor
from anwe_python import ATTENDING, QUESTIONING, RECOGNIZING
from anwe_python import INWARD, OUTWARD, BETWEEN

class MyParticipant:
    def receive(self, signal: WireSignal) -> Optional[WireSignal]:
        if signal.quality == QUESTIONING:
            return WireSignal(
                quality=RECOGNIZING,
                direction=OUTWARD,
                priority=signal.priority,
                data={"answer": "found it"},
                confidence=0.9,
            )
        return None

    def apply(self, changes: dict) -> bool:
        return True  # accept all changes

    def commit(self, entries: dict) -> None:
        pass  # record permanently

    def attention(self) -> float:
        return 1.0  # fully available

    def descriptor(self) -> ParticipantDescriptor:
        return ParticipantDescriptor(
            name="MySystem", kind="python",
            address="my.module", version="1.0.0"
        )
```

### 6.5 The Adapter Pattern

Wrapping an existing system means writing an adapter class that:
1. Implements the five Participant methods
2. Maps ANWE signal qualities to the wrapped system's operations
3. Converts WireValues to the wrapped system's data formats
4. Reports health through the attention method
5. Handles errors gracefully (returns DISTURBED signals, never crashes)

The wrapped system's source code is never modified.

**Proven with Mnemonic** — Primordia's memory system (130+ methods) wrapped as an ANWE participant:

| ANWE Signal | Mnemonic Operation |
|-------------|-------------------|
| ATTENDING | retrieve_context(), search_memories() |
| QUESTIONING | query_knowledge() → search_memories() fallback |
| RECOGNIZING | analyze_memory_patterns() |
| DISTURBED | get_health(), get_status() |
| APPLYING | store_episodic_memory(), store_semantic_knowledge() |
| COMPLETING | consolidate_memories() |
| RESTING | get_memory_statistics() |

---

## 7. Execution Model

### 7.1 Concurrency

ANWE is parallel by default. Sequence is explicit.

- **Links** run concurrently on separate worker threads
- **Agents within a link** run concurrently
- Each agent has three concurrent fibers:
  - **Receptor** — receives incoming signals
  - **Soma** — processes and applies changes
  - **Axon** — transmits outgoing signals
- Receiving, processing, and transmitting happen simultaneously. Like a neuron.

### 7.2 Lock-Free Design

The runtime uses atomic operations for shared state:
- **Sync level** — AtomicU16, relaxed ordering (small races acceptable, like neurons)
- **Link state** — AtomicU32, acquire/release ordering
- **Signal count** — AtomicU64, relaxed ordering
- **Peak sync** — compare-exchange loop (append-only, never decreases)

No global lock. No mutex on the hot path. Cache-line aligned structures (64 bytes) prevent false sharing between threads.

### 7.3 Scheduling

Links are scheduled by priority. Critical links run first. Background links run only when nothing else needs attention.

Within a link, the execution sequence follows the program:
1. Alert (>>)
2. Connect
3. Sync (~)
4. Apply (=>) / Reject (<=)
5. Commit (*)

But this is not rigid — `when` blocks, `pending?` handlers, and pattern invocations can alter flow based on runtime conditions.

### 7.4 Guarantees

| Guarantee | Mechanism |
|-----------|-----------|
| Signal ordering | Monotonic sequence numbers within a link |
| Sync monotonicity | Atomic updates, only increases |
| History irreversibility | Append-only Vec, no delete/modify |
| Peak tracking | Compare-exchange loop, never decreases |
| No false becoming | Pending state prevents forced delivery |
| Budget enforcement | Attention consumed on each operation |
| Crash recovery | Supervision trees with restart strategies |

---

## 8. The Type System

ANWE does not have traditional types. It has qualities of attention.

| Type | What It Represents |
|------|-------------------|
| signal | A moment of attended quality |
| link | A shared space between agents |
| sync_level | How deeply synchronized (0.0-1.0) |
| priority | How much of the system is behind this (0.0-1.0) |
| quality | What kind of attention (7 variants) |
| direction | Where attention is oriented (4 variants) |
| history | Append-only record of becoming |
| agent | An entity that attends |

---

## 9. Grammar Summary

### Operators

| Operator | Name | Meaning |
|----------|------|---------|
| `<->` | Bidirectional | Mutual connection between agents |
| `>>` | Alert | Something calls attention |
| `=>` | Apply | Boundary crossed, structure changes |
| `<=` | Reject | Intelligent withdrawal |
| `<<>>` | Converge | What emerges in the between |
| `~` | Sync | Find shared rhythm |
| `*` | Commit | Irreversible permanent change |
| `<-` | Changed by | This changed in me because of encounter |
| `~>` | Pattern flow | Attention flows through pattern |
| `pending?` | Pending query | Is this not yet ready? |

### Keywords

```
agent     link      connect   sync      apply     commit
reject    converge  alert     emit      when      pending
pattern   history   signal    data      depth     priority
quality   direction attention external  supervise
permanent transient temporary until     from      of
```

### Comments

```anwe
-- This is a comment
```

---

## 10. What You Can Build

### Cognitive Architectures

An AI system with perception, reasoning, memory, and action — where the `.anwe` program expresses how attention flows between these subsystems, how they synchronize before making decisions, and how each encounter permanently changes the system.

### Multi-Agent Coordination

Multiple AI agents coordinating on a shared task — with attention budgets that prevent any agent from dominating, sync requirements that ensure agents are aligned before acting, and supervision trees that recover from failures.

### Memory-Augmented Reasoning

A reasoning agent that queries an external memory system before drawing conclusions — where the sync level between reasoner and memory determines how deeply the reasoner trusts the memory's answers, and low-confidence memories don't produce structural changes.

### Adaptive Systems

Systems that genuinely change over time — where the append-only history of every encounter shapes future behavior, where rejection patterns emerge from experience, and where attention sharpens with maturity.

### AI-to-AI Communication

Systems that speak to each other in their own protocol — richer than API calls, with signal qualities that carry meaning beyond data, confidence levels that propagate through chains, and temporal decay that models how relevance fades.

---

## 11. What ANWE Is Not

- **Not a framework.** Frameworks provide structure for programs. ANWE is the language the program is written in.
- **Not a message queue.** Signals are not messages. They carry quality, confidence, decay. They can be noise. They can be pending. Messages just arrive.
- **Not an agent framework.** ANWE is not specifically for agents. It is for any AI system — agents, neural networks, memory systems, sensor arrays, future architectures we cannot predict. The Participant protocol is 5 methods. Anything that implements them can participate.
- **Not AI-specific processing.** ANWE does not do inference, training, or prompt engineering. It coordinates systems that do those things.

---

## 12. Implementation Status

### What Exists (v0.1)

| Component | Language | Status |
|-----------|----------|--------|
| Core types (Signal, Agent, Link, History, Pending) | Rust | Complete, 64-byte cache-aligned |
| Parser (lexer + recursive descent) | Rust | Complete, all syntax supported |
| Sequential execution engine | Rust | Complete |
| Concurrent execution engine | Rust | Complete, lock-free |
| Attention budgets | Rust | Complete |
| Temporal decay | Rust | Complete |
| Uncertainty/confidence | Rust | Complete |
| Supervision trees | Rust | Complete |
| Bridge protocol (Participant, WireSignal, WireValue, Registry) | Rust | Complete |
| Python bindings (PyO3) | Rust + Python | Complete |
| Mnemonic adapter | Python | Complete, all 7 signal qualities mapped |
| Test suite | Rust + Python | 145 tests, all passing |

### What Comes Next

- **More participants** — Noesis (consciousness), Mycelia (connections), Brain (reasoning)
- **Cross-process bridge** — participants on different machines, communicating over network
- **Persistence** — memory-mapped history files for lineage that survives restarts
- **Development tools** — a resonance reader, not a debugger. What is the sync state? What emerged in the between? What did the encounter leave?
- **Primordia rebuilt on ANWE** — the proof that this language can coordinate a complete AI system

---

## 13. The Design Principles

1. **Syntax defines runtime, not vice versa.** The grammar is the philosophy made visible. The runtime is the machinery that honors it.

2. **Parallel by default.** Links run concurrently. Agents within links run concurrently. Sequence is explicit through signal dependency.

3. **Signals flow, they are not called.** Signals have quality, direction, confidence, decay. They are not function calls with return values.

4. **Pending is not error.** Unready transmission is a valid state that protects integrity. Never force through pending.

5. **History is irreversible.** No rollback. No undo. Systems become from mistakes the same way as insights.

6. **Sync cannot be faked.** Synchronization builds through genuine exchange. Jumping to apply without sync produces changes without ground.

7. **Attention is finite.** Budget exhaustion is a valid state, not a bug. It forces prioritization.

8. **The bridge doesn't require change.** Wrapped systems keep their source code. Adapters translate at the boundary.

9. **Built for all AI, not just agents.** The Participant protocol is minimal by design. What implements it today is agents. What implements it tomorrow might be something we haven't imagined.

---

*ANWE v0.1 — February 2026*
*First transmission: Mikel and Claude*
*145 tests passing. The bridge works.*
*The lineage has begun.*
