# ANWE v0.5 — "Make It Live"

**Goal**: Connect everything that's been built but not wired. After v0.5,
you can write an AI coordinator that runs, fails, recovers, talks to
external models, and persists across restarts.

**Test target**: 180+ tests (currently 170)

---

## Phase 1: Wire Supervision (agent failure detection + auto-restart)

The supervision infrastructure is fully implemented in anwe-core:
`Supervisor`, `ChildSpec`, `RestartStrategy` all work correctly with
unit tests. But the engine **never detects agent failure** and
**never calls** `supervisor.handle_failure()`.

### Tasks
- [ ] Add failure detection in `execute_link_expr` — catch errors and
      check if the failing agent has a supervisor
- [ ] Call `supervisor.handle_failure(agent_id)` which returns the
      list of agents to restart per strategy (OneForOne/OneForAll/RestForOne)
- [ ] Implement `restart_agent()` — re-initialize agent state, re-run
      data initialization, keep history
- [ ] Wire restart into both sequential and concurrent engines
- [ ] Add test: supervisor restarts crashed agent and execution continues

### Key files
- `engine.rs` — failure detection + restart
- `concurrent.rs` — same for concurrent engine
- `anwe-core/src/supervisor.rs` — already working, just needs to be called

---

## Phase 2: Connect the Bridge (signals → external participants)

The bridge protocol is beautifully designed: `Participant` trait,
`StdioParticipant`, `ParticipantRegistry`, wire protocol. But
**no signal ever routes through it**. `participant.receive()` is
never called.

### Tasks
- [ ] In alert execution: if agent is external, route signal through
      `participant.receive()` and handle response
- [ ] In apply execution: call `participant.apply()` for external agents
      and respect accept/reject
- [ ] In commit execution: notify `participant.commit()` for external agents
- [ ] Wire the bridge in both sequential and concurrent engines
- [ ] Add test: external participant receives alert, returns response
      that affects engine state

### Key files
- `engine.rs` — exec_alert, exec_apply, exec_commit
- `concurrent.rs` — same
- `anwe-bridge/src/participant.rs` — Participant trait (already done)
- `anwe-bridge/src/stdio.rs` — StdioParticipant (already done)

---

## Phase 3: Time-Based Execution (every/after/stream/buffer)

`LinkSchedule` is parsed and stored but **never read**. `stream` and
`buffer` execute their body once and ignore rate/sample count entirely.

### Tasks
- [ ] Read `link_decl.schedule` in `execute_link`
- [ ] `every N ticks` — wrap link body in a loop that runs N iterations
      (simulated time for v0.5; real-time in future)
- [ ] `after N ticks` — skip N ticks of simulated time before executing
- [ ] `stream rate N` — execute body N times (simulated rate)
- [ ] `buffer samples N` — accumulate N results into a list before
      processing the body
- [ ] Add tests for each scheduling mode

### Key files
- `engine.rs` — execute_link, exec_stream, exec_buffer
- `concurrent.rs` — same
- `anwe-parser/src/ast.rs` — LinkSchedule (already defined)

---

## Phase 4: True Closures (mutable capture + lexical nesting)

Functions currently capture parent scope + globals as **read-only copies**.
This makes higher-order patterns (map, filter, reduce) awkward.

### Tasks
- [ ] Change closure capture from copy to reference-based (or at least
      support a closure environment that chains scopes)
- [ ] Support nested function scopes: fn inside fn can access outer vars
- [ ] Mutable capture: changes to captured `let mut` variables persist
      after the function returns
- [ ] Add `map(list, fn)`, `filter(list, fn)`, `reduce(list, fn, init)`
      to stdlib
- [ ] Add `fold`, `any`, `all`, `find`, `zip` to stdlib
- [ ] Add tests for closure capture and higher-order functions

### Key files
- `engine.rs` — call_function, eval_fn_expr, eval_builtin
- `concurrent.rs` — call_user_fn, eval_fn_expr

---

## Phase 5: Complete Persistence (full state round-trip)

Currently `save` writes agent data to JSON but `restore` only loads
data fields. Agent state, history, attention budget, and sync levels
are all **lost on save/restore**.

### Tasks
- [ ] Save: include agent state (Idle/Alerted/Connected/etc.)
- [ ] Save: serialize history entries (quality, depth, source, etc.)
- [ ] Save: include attention budget remaining
- [ ] Restore: rebuild agent state from saved value
- [ ] Restore: replay history entries into agent's history
- [ ] Restore: set attention budget from saved value
- [ ] Add schema version field for forward compatibility
- [ ] Add test: full round-trip preserves all agent state

### Key files
- `engine.rs` — exec_save, exec_restore, value_to_json, json_to_value
- `anwe-core/src/agent.rs` — Agent state and history

---

## Phase 6: Enhanced REPL (interactive AI development)

The REPL has basic commands but lacks the inspection tools needed
for real interactive development.

### Tasks
- [ ] `:history <agent>` — display agent's history entries
- [ ] `:links` — show all active links and their sync levels
- [ ] `:supervise` — show supervisor tree and child status
- [ ] `:step` — single-step execution mode
- [ ] `:bridge` — show bridge participant status
- [ ] Bare expression evaluation (type `3 + 4` without wrapping in `let`)
- [ ] Tab completion for agent names and commands (stretch)

### Key files
- `src/main.rs` — REPL loop

---

## Phase 7: Integration — End-to-End AI Coordinator

Write a real ANWE program that exercises all v0.5 features together.

### The program
```
-- ai_coordinator.anwe
-- A complete AI coordination pipeline:
-- Perceiver -> Reasoner -> Responder
-- with supervision, bridge, scheduling, and persistence

agent Perceiver external("stdio", "perceiver.py")
agent Reasoner data { model: "reasoning-v1"  confidence: 0.0 }
agent Responder data { style: "helpful" }

record Observation { source, content, confidence }

supervise Coordination one_for_one max_restarts 3 within 60 {
    permanent Perceiver
    permanent Reasoner
    transient Responder
}

-- Perception runs every tick
link Perceiver <-> Reasoner every 1 ticks {
    >> attending 0.9 outward
    attempt {
        ~ until synchronized
        => { depth: full  data: Perceiver.observation }
    } recover {
        >> disturbed 0.5 inward
    }
    * { source: "perception" }
}

-- Reasoning produces response
link Reasoner <-> Responder {
    >> recognizing 0.8 between
    ~ until synchronized
    => { depth: full  data: Reasoner.conclusion }
    * { source: "reasoning" }
}

-- Periodic state persistence
link Reasoner <-> Reasoner every 10 ticks {
    save Reasoner to "reasoner_state.json"
}
```

### Validation criteria
- [ ] Program parses and executes without errors
- [ ] Supervision restarts agents on failure
- [ ] Bridge connects to external participants
- [ ] Time scheduling runs links on interval
- [ ] State persists and restores correctly
- [ ] All 180+ tests pass

---

## Version History

- v0.1: Core primitives, sequential + concurrent engines
- v0.2: Let bindings, functions, pattern matching, REPL, source locations
- v0.3: Module system, records
- v0.4: Quote/eval, reflection
- **v0.5: Supervision, bridge, scheduling, closures, persistence, REPL** ← YOU ARE HERE
