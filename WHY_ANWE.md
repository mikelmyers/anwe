# Why ANWE

*The engineer's version. No philosophy. Just what it does and why it matters.*

---

## What sucks about building multi-agent AI in Python

You already know this. You've lived it.

**1. Coordination is duct tape.**
You use `asyncio.gather()` and hope everything comes back. When it doesn't, you write retry logic. Then you write retry logic for the retry logic. Then you add a supervisor. Then you realize the supervisor needs supervision. LangChain, CrewAI, AutoGen — they give you abstractions over individual agents, but the coordination between them is still your problem.

**2. Nothing carries confidence.**
Your RAG pipeline returns documents. How confident is each result? You don't know unless you built a wrapper class and disciplined your entire team to use it everywhere. Most teams don't. So you have pipelines producing answers with no uncertainty tracking, and you discover the problem in production when hallucinations cost you customers.

**3. Failure modes are afterthoughts.**
"What happens when the LLM times out?" Exception. "What happens when the vector store is down?" Exception. "What happens when the safety check takes too long?" Exception. Every failure mode is the same: try/except, retry, log, pray. There's no structural difference between "the model is loading" and "the model is broken."

**4. There is no memory of what happened.**
Your agent made a decision 47 requests ago. Why? Which documents did it see? What confidence did the safety check return? What was the sync state between your retriever and your generator? Unless you built a custom audit log — and you probably didn't — it's gone. In production, when a customer asks "why did your AI do that?", you open your logs and hope the answer is there.

**5. Agents don't know about each other.**
In Python, agents are functions that call other functions. Agent A doesn't know if Agent B is ready, overloaded, or dead. They share no rhythm. They have no structural relationship. Coordination is imperative: call A, wait, call B, wait, merge results, handle errors. This is orchestration by brute force.

---

## What ANWE does instead

ANWE is a programming language built for multi-agent AI systems. Not a framework on top of Python. A language.

Here's what you get:

### Supervision is native

```anwe
supervise one_for_one max_restarts 3 within 60000 {
    permanent Generator
    transient Embedder
    temporary Cache
}
```

If the embedder crashes, it restarts. If it crashes 3 times in 60 seconds, the supervisor escalates. The generator never restarts — it's permanent. The cache is disposable. This is Erlang-style supervision built into the language, not bolted on.

**Python equivalent: ~50 lines** of supervisor class, restart tracking, and time-window management.

### Confidence is native

Every signal in ANWE carries confidence:

```anwe
>> { quality: attending, priority: 0.9, confidence: 0.85 }
   "temperature anomaly detected in sensor cluster seven"
```

Every sync tracks sync_level. Every apply carries the confidence at which it occurred. You cannot produce a result without stating how confident you are. This isn't a wrapper class someone forgot to use — it's the language.

**Python equivalent:** A `UncertainValue` dataclass that nobody on your team remembers to wrap their results in.

### "Not ready" is not an error

```anwe
pending? receiver_not_ready {
    wait 2.0 tick
    guidance "generator loading — maintain context coherence"
}
```

When the LLM isn't ready, that's a valid state with guidance, not an exception. The system knows to wait and why. This is a structural distinction between "temporarily unavailable" and "broken."

**Python equivalent:** `try: ... except TimeoutError: time.sleep(2); retry()`

### History is append-only and irreversible

Every `*` commit in ANWE is permanent:

```anwe
* from apply {
    stage: "safety_review"
    approved: "true"
    confidence: "0.96"
}
```

You cannot delete history. You cannot modify it. You can query it. When a customer asks "why did your AI approve this content?", the answer is in the agent's history — every decision, every confidence score, every rejection.

**Python equivalent:** An audit log that requires discipline to maintain and that nobody deletes only because nobody has a reason to yet.

### Coordination is structural, not procedural

```anwe
agent SafetyReviewer attention 0.9
agent QualityReviewer attention 0.7

link SafetyReviewer <-> QualityReviewer priority high {
    SafetyReviewer ~ QualityReviewer until synchronized
    => when sync_level > 0.7 { status <- "aligned" }
    * from apply { stage: "review_sync" }
}
```

Agents are linked. They sync bidirectionally. Apply happens when sync reaches threshold. This isn't imperative "call A then call B" — it's a declaration of how agents relate to each other.

**Python equivalent:** `asyncio.gather()` with null-checking, exception handling, manual state merging, and no guarantee that the agents ever agreed on anything.

### Safety is a primitive, not a check

```anwe
<= when confidence < 0.3 data "content fails safety review"
```

Reject (`<=`) is one of ANWE's seven primitives. It's not an if-statement in your coordinator. It's a fundamental operation — intelligent withdrawal that is recorded in history, that carries the reason, that the system structurally understands as "this agent refused."

**Python equivalent:** `if not safety_result.approved: return {"error": "safety veto"}` — a conditional buried in business logic.

### Attention budgets enforce resource limits

```anwe
agent Generator attention 1.0    -- gets full resources
agent Cache     attention 0.3    -- background priority
```

Every agent has a finite attention budget. When it's exhausted, the agent cannot process more signals until budget recovers. Context windows, rate limits, compute allocation — these are attention budgets, and ANWE models them natively.

**Python equivalent:** Manual token counters, rate limit trackers, and semaphores that you wire together yourself.

---

## What you get that no other language provides

These aren't features you can add with a library. They require being built into the language:

**1. Seven cognitive primitives (>>, <->, ~, =>, *, <=, <<>>).**
Alert, Connect, Sync, Apply, Commit, Reject, Converge. These aren't function calls — they're the fundamental operations of multi-agent coordination. Every other language treats coordination as library code. ANWE treats it as syntax.

**2. Agents as first-class entities with state machines.**
Every agent has 7 states (Idle, Alerted, Connected, Syncing, Applying, Rejecting, Committing), an attention budget, and an irreversible history. This is not an `AgentState` enum you built — it's the runtime.

**3. First-person cognition.**
```anwe
mind Reviewer {
    attend "analyze content" priority 0.9 {
        think { assessment <- "evaluating safety constraints" }
        sense { landscape <- "perceived" }
        express { quality: recognizing, priority: 0.85 }
            "content passes safety review"
    }
}
```
ANWE is the only language where an AI can write code in first person — `think`, `sense`, `express`, `attend`. This isn't syntactic sugar. It's a paradigm that makes AI systems self-describing.

**4. Temporal decay.**
Signals and values have half-lives. A cached embedding from 5 minutes ago is less relevant than one from 5 seconds ago. ANWE tracks this automatically. In Python you build a TTL cache and hope the expiry windows are right.

**5. Uncertainty arithmetic.**
When you add two uncertain values, the uncertainty propagates (quadrature for addition, relative for multiplication). This is native. No library. No wrapper.

**6. Self-authoring minds.**
```anwe
author attend "emergent insight" priority 0.75 {
    think { insight <- "something unexpected emerged" }
    express "the insight arrived unbidden"
}
```
A mind can create new attention blocks at runtime. The system evolves its own behavior during execution. No other language has this.

---

## The comparison in numbers

| | Python | ANWE |
|---|---|---|
| RAG pipeline | 344 lines | 100 lines |
| Multi-agent review | 160 lines | 100 lines |
| Guardrail pipeline | ~200 lines | 80 lines |
| Infrastructure overhead | 40-60% of code | 0% |
| Confidence tracking | Manual everywhere | Native everywhere |
| Supervision | Build it yourself | 4 lines |
| Audit trail | Build it yourself | Automatic |

See [COMPARISON.md](COMPARISON.md) for the full side-by-side code.

---

## Who this is for

- **AI engineers** building multi-agent systems who are tired of writing coordination infrastructure
- **ML teams** who need confidence tracking, audit trails, and supervision in production
- **Researchers** exploring multi-agent coordination, attention mechanisms, and self-modifying systems
- **Anyone** who has written `asyncio.gather()` inside a `try/except` inside a `while retry_count < 3` and thought "there has to be a better way"

There is. It's called ANWE.

---

## Get started

```bash
git clone https://github.com/mikelmyers/Primordia.git
cd Primordia/ANWE/anwe-runtime
cargo build --release
./target/release/anwe run examples/rag_pipeline.anwe
```

Read the [Language Reference](anwe-runtime/LANGUAGE_REFERENCE.md) or the [Getting Started tutorial](anwe-runtime/GETTING_STARTED.md).
