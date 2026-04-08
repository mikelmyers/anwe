# ANWE Bridge Protocol

The bridge protocol lets external systems — Python scripts, Rust services, hardware controllers, neural networks — participate in ANWE coordination as full agents.

## What's Working Today

| Feature | Status |
|---------|--------|
| Participant trait (5 core + 2 mind methods) | Working, tested |
| Wire protocol (WireSignal, WireValue) | Working, used at all bridge boundaries |
| CallbackParticipant (Rust inline) | Working, with echo/silent presets |
| MindCallbackParticipant (think/express) | Working, with reflective preset |
| Engine integration (both modes) | Working — alert, connect, sync, apply, commit all call bridge |
| Mind integration | Working — think and express notify participants |
| StdioParticipant (spawn external process) | Implemented, minimal JSON parsing |
| Integration tests | 7+ comprehensive tests, all passing |

## The Trait

Any external system participates by implementing 5 methods (+ 2 optional):

```rust
trait Participant {
    fn receive(&mut self, signal: &WireSignal) -> Option<WireSignal>;
    fn apply(&mut self, changes: &[(String, WireValue)]) -> bool;
    fn commit(&mut self, entries: &[(String, WireValue)]);
    fn attention(&self) -> f32;       // defaults to 1.0
    fn descriptor(&self) -> &ParticipantDescriptor;

    // Optional — participate in mind blocks
    fn think(&mut self, bindings: &[(String, WireValue)]) -> Option<Vec<(String, WireValue)>>;
    fn express(&mut self, signal: &WireSignal, content: &WireValue) -> Option<WireValue>;
}
```

When a link primitive fires, the engine calls the appropriate method:
- `alert` / `connect` / `sync` → `receive()`
- `apply` → `apply()` — returning `false` triggers the reject path
- `commit` → `commit()`
- `think` → `think()` — returned bindings are merged into the mind
- `express` → `express()` — returned value replaces the expression

## Quick Start: Callback Participant

The simplest way to bridge — write handlers inline in Rust:

```rust
use anwe_bridge::{CallbackParticipant, ParticipantDescriptor, ParticipantRegistry};

let mut registry = ParticipantRegistry::new();
registry.register("Sensor", Box::new(
    CallbackParticipant::echo("Sensor")  // echoes signals, accepts all changes
));

let mut engine = Engine::new().with_participants(registry);
engine.execute(&program)?;
```

In your .anwe program, declare the agent as external:

```anwe
agent Sensor external("callback", "echo")
agent Processor

link Sensor <-> Processor {
    >> { quality: attending, priority: 0.8 } "temperature reading: 42.1"
    Sensor ~ Processor until synchronized
    => when sync_level > 0.7 { "analysis complete" }
    * from Processor
}
```

## Stdio Bridge (Any Language)

The stdio bridge spawns an external process and communicates via JSON over stdin/stdout:

```bash
anwe run --bridge Sensor=cmd:python3\ sensor.py examples/my_program.anwe
```

The external process receives JSON messages and responds with JSON:

```json
{"type": "receive", "signal": {"quality": "attending", "priority": 0.8, ...}}
{"type": "apply", "changes": [["temperature", 42.1]]}
{"type": "commit", "entries": [["status", "confirmed"]]}
```

**Current limitation:** The stdio JSON parsing is minimal. It works for simple values but complex nested structures may need improvement.

## Example Programs

- [`examples/bridge_echo.anwe`](examples/bridge_echo.anwe) — Minimal bridge with echo participant
- [`examples/mind_bridge.anwe`](examples/mind_bridge.anwe) — Mind that bridges to external AI
- [`examples/mnemonic_bridge.anwe`](examples/mnemonic_bridge.anwe) — Python memory system as participant

## What's Not Built Yet

- **Network transport** (TCP, gRPC, WebSocket) — the architecture supports it but no transport implementations exist
- **Language-specific SDKs** — no Python/JS/Go packages for building participants
- **Discovery / service registry** — agents must be registered programmatically
- **Authentication / encryption** — stdio bridge has no security layer

These are on the [roadmap](ROADMAP.md).

## Source Code

- Trait and wire types: [`crates/anwe-bridge/src/`](crates/anwe-bridge/src/)
- Callback participants: [`crates/anwe-bridge/src/participant.rs`](crates/anwe-bridge/src/participant.rs)
- Stdio bridge: [`crates/anwe-bridge/src/stdio.rs`](crates/anwe-bridge/src/stdio.rs)
- Engine integration: search for `bridge_notify` in [`crates/anwe-runtime/src/engine.rs`](crates/anwe-runtime/src/engine.rs)
