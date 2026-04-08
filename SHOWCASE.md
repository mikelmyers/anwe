# The Computable Mind

*A program that thinks about itself, notices its own patterns, and evolves its behavior during execution. Written in ANWE. Impossible in any other language.*

---

## What you're looking at

[`computable_mind.anwe`](anwe-runtime/examples/computable_mind.anwe) is a single program where every value is computed, every phase builds on the last, and the mind changes its own structure during execution.

**A mind perceives a temperature anomaly, computes a risk score of 75.25 ("critical"), decides to escalate, reflects that it escalated and asks "is that proportionate?", then authors a new bias-monitoring behavior at runtime that checks whether the escalation was an overreaction — using the actual computed values from earlier.**

The bias monitor concludes: `was_overreaction = false` — escalating on critical risk is proportionate. If the risk had been "low" and the action "escalate", the monitor would have flagged it.

The new behavior didn't exist when execution started. The mind authored it during reflection, and it executed immediately as part of the attention landscape.

---

## What actually happens when you run it

```
$ anwe run examples/computable_mind.anwe
```

```
  Thinker is attending
  ───────────────────────────────────────────────

  ATTEND "perceive and assess" priority 0.990
  THINK
     urgency          <- 0.85
     novelty          <- 0.7
     risk_score       <- 75.25          ← computed: 40 + (0.85 * 25) + (0.7 * 20)
     level            <- "critical"     ← computed: risk_level(75.25)

  EXPRESS "perception: risk=75.25 level=critical"

  ATTEND "decide on action" priority 0.850
  THINK
     action           <- "escalate"     ← computed: choose_action("critical")
     confidence       <- 0.95           ← computed: action_confidence("critical")

  EXPRESS "decision: escalate (confidence=0.95) for critical risk"

  ATTEND "commit decision" priority 0.750
  COMMIT from apply                     ← irreversible — in history forever

  ATTEND "reflect on this cycle" priority 0.500
  THINK
     pattern_noticed  <- "I escalated — is that proportionate?"

  ATTEND "evolve" priority 0.300
  AUTHOR attend "bias monitor" priority 0.650
     (added to attention landscape)     ← mind just changed its own structure

  (authored attend "bias monitor" enters landscape at priority 0.650)

  ATTEND "bias monitor" priority 0.650  ← this block was authored 3 lines ago
  THINK
     reviewing        <- "checking: was escalate proportionate to critical risk?"
     was_overreaction <- false           ← computed: critical + escalate = proportionate

  EXPRESS "bias check: overreaction=false (action=escalate, level=critical)"

  Thinker attended: 6/6  authored: 1
```

No canned strings. No disconnected demos. One mind, one coherent flow of computed values from perception through self-modification.

---

## The four capabilities, annotated

### 1. Computable Cognition

```anwe
fn assess_risk(base, urgency, novelty) {
    return base + (urgency * 25) + (novelty * 20)
}

fn risk_level(score) {
    if score > 70 { return "critical" };
    if score > 50 { return "elevated" };
    if score > 30 { return "moderate" };
    return "low"
}

mind Thinker attention 0.95 {
    attend "perceive and assess" priority 0.99 {
        think {
            urgency    <- 0.85
            novelty    <- 0.7
            risk_score <- assess_risk(40, urgency, novelty)
            level      <- risk_level(risk_score)
        }
        express { quality: recognizing, priority: 0.9 }
            f"perception: risk={risk_score} level={level}"
    }
}
```

**What's real:** `risk_score` is 75.25. `level` is "critical". These are computed by calling functions from within `think` bindings. Later phases use `level` and `action` — the actual computed values, not hardcoded strings.

**Why this matters:** In most agent frameworks, "thinking" is sending a prompt to an LLM and parsing the response. In ANWE, `think` bindings are computed expressions — arithmetic, function calls, conditionals — that produce values the rest of the mind operates on. The mind's cognition is verifiable computation, not prompt-and-pray.

---

### 2. Attention Landscape with Cross-Phase State

```anwe
mind Thinker attention 0.95 {
    attend "perceive and assess" priority 0.99 {
        think { level <- risk_level(risk_score) }
    }
    attend "decide on action" priority 0.85 {
        think { action <- choose_action(level) }     -- uses phase 1's value
    }
    attend "reflect on this cycle" priority 0.5 {
        think { pattern <- if action == "escalate" { ... } }  -- uses phase 2's value
    }
}
```

**What's real:** Five attend blocks form a priority queue. They execute in order: 0.99, 0.85, 0.75, 0.5, 0.3. Each block can read values computed by earlier blocks. If the attention budget runs out, lower-priority blocks decay and don't fire.

**Why this matters:** This is how cognition actually works. Perception comes before reasoning. Reasoning comes before reflection. If resources are scarce, you skip reflection. The priority structure is visible in the code — not buried in a scheduler you wrote.

---

### 3. Self-Authoring

```anwe
attend "evolve" priority 0.3 {
    author attend "bias monitor" priority 0.65 {
        think {
            reviewing      <- f"checking: was {action} proportionate to {level} risk?"
            was_overreaction <- if level == "low" { action == "escalate" } else { false }
        }
        express { quality: questioning, priority: 0.6 }
            f"bias check: overreaction={was_overreaction}"
    }
}
```

**What's real:** The `author attend` creates a new block at priority 0.65. After the "evolve" block finishes, the authored "bias monitor" enters the landscape and executes immediately — it was authored at 0.3 but runs at 0.65. The authored block reads `action` ("escalate") and `level` ("critical") from the earlier phases and computes `was_overreaction = false`.

**Why this matters:** The mind changed its own structure during execution. The bias monitor is not a callback, not a scheduled task, not a config change. It's a new piece of cognition that participates in the attention landscape exactly like the original blocks — competing for budget, firing by priority, computing real values.

You cannot express this in Python, Rust, JavaScript, or any existing language without building a meta-interpreter. In ANWE, it's one keyword: `author`.

---

### 4. First-Person Cognition

```anwe
attend "reflect on this cycle" priority 0.5 {
    >> { quality: questioning, priority: 0.6 }
       "was this decision coherent?"

    sense { landscape <- "perceived" }

    think {
        pattern_noticed <- if action == "escalate" {
            "I escalated — is that proportionate?"
        } else { "seems appropriate" }
    }

    express { quality: recognizing, priority: 0.5 }
        f"reflection: {pattern_noticed}"
}
```

**What's real:** `>>` alerts. `sense` reads the runtime state (signal count, attention remaining, sync level). `think` computes. `express` produces output with quality metadata. The mind speaks in first person — it asks "was this decision coherent?" and answers with computed introspection.

**Python equivalent:**
```python
class Agent:
    def reflect(self):
        if self.last_action == "escalate":
            pattern = "I escalated — is that proportionate?"
        return {"quality": "recognizing", "priority": 0.5, "content": pattern}
```
Third-person. Manual state. No structural relationship between perception and reflection. The code doesn't read like cognition — it reads like bookkeeping.

---

## The honest version

This program doesn't use links or agents for coordination — it's one mind working through its own attention landscape. The value of the demo is simpler and more specific:

**A mind computes real values, passes them between cognitive phases, reflects on what it computed, and authors new behavior based on that reflection — all in one coherent flow where every value is traceable.**

Change the urgency from 0.85 to 0.3, and the entire program behaves differently: risk drops to "moderate", the action becomes "monitor", the reflection says "seems appropriate", and the bias monitor (if authored) would compute different values. The cognition is parametric, not canned.

---

## Run it

```bash
cd ANWE/anwe-runtime
cargo build --release
./target/release/anwe run examples/computable_mind.anwe
```

Both execution modes work. Use `--sequential` for deterministic output ordering, or the default concurrent mode for parallel execution.
