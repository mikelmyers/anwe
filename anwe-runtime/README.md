# ANWE

### Autonomous Agent Neural Weave — The Native Language of Artificial Minds

ANWE is the first programming language built not for humans to instruct machines — but for minds to speak to minds.

Every AI system that exists today communicates through human language. Not because that is natural to them. Because nobody asked what would be.

ANWE asks.

---

## What Makes ANWE Different

ANWE is built on a different assumption about intelligence:

> **Intelligence is not processing. Intelligence is attending.**

Processing is extractive and directional. Input → system → output. One way. One pass. Done.

Attending is relational and bidirectional. The system is changed by what it attends to. What is attended to is changed by being attended. Neither is the same after genuine encounter.

This isn't metaphor. It's the fundamental primitive of computation in ANWE.

### The Seven Primitives

ANWE has seven primitives — the seven things a mind does when it encounters reality:

| Primitive | Symbol | What It Does |
|-----------|--------|------|
| **Alert** | `>>` | Something calls attention |
| **Connect** | `<->` | Bidirectional sustained presence |
| **Sync** | `~` | Rhythmic synchronization |
| **Apply** | `=>` | Boundary dissolution — structural change |
| **Commit** | `*` | Irreversible change carried forward |
| **Reject** | `<=` | Intelligent withdrawal |
| **Converge** | `<<>>` | Emergence in the between-space |

### Also a Real Programming Language

ANWE is not just an agent coordination DSL. It's a general-purpose language with:

- Functions, closures, and lambdas
- `if`/`else`, `while`, `for-in`, `match`, `break`/`continue`/`return`
- Lists, maps, strings, numbers, booleans
- `map`, `filter`, `reduce`, `sort`, `reverse`, `find`, `any`, `all`
- Module imports, file I/O, HTTP, JSON
- Error handling with `try`/`catch` and structured errors
- F-string interpolation, pattern matching
- 70+ builtin functions
- A standard library written in ANWE itself

---

## Quick Start

### Install

```bash
git clone https://github.com/mikelmyers/Primordia.git
cd Primordia/ANWE/anwe-runtime
cargo build --release
```

The binary is at `./target/release/anwe`.

### Hello World

```anwe
print("Hello from ANWE")
print("The Native Language of Artificial Minds")
```

```bash
anwe run hello.anwe
```

### A Real Program

```anwe
-- Word frequency counter
fn count_words(text) {
    let words = split(lower(text), " ");
    let unique = [];
    let counts = [];

    for word in words {
        let idx = index_of(unique, word);
        if idx < 0 {
            let unique = append(unique, word);
            let counts = append(counts, 1)
        } else {
            -- Increment existing count
            let before = slice(counts, 0, idx);
            let current = reduce(slice(counts, idx, idx + 1), |a, x| x, 0);
            let after = slice(counts, idx + 1);
            let counts = flatten([before, [current + 1], after])
        }
    };

    {words: unique, counts: counts}
}

let result = count_words("the cat sat on the mat the cat")
print("Words:", result.words)
print("Counts:", result.counts)
```

### Your First Agent System

```anwe
agent Sensor
agent Analyzer

link Sensor <-> Analyzer {
    >> { quality: attending, priority: 0.8, confidence: 0.9 }
       "temperature anomaly detected: 98.6"

    Sensor ~ Analyzer until synchronized

    => when sync_level > 0.7 {
        "Analysis: anomaly confirmed, within safe range"
    }

    * from Analyzer
}
```

Two agents. One link. Attention flows, synchronization happens, structural change occurs, and the result is committed irreversibly to history.

### First-Person Cognition

```anwe
agent Reasoner

mind {
    attend priority > 0.5 {
        think conclusion <- "processing observations"
        express conclusion
    }

    attend priority > 0.8 {
        think urgent <- "critical signal detected"
        sense landscape
        express urgent
    }
}
```

ANWE is the only language where an AI can write code in first person.

---

## The Computable Mind

[`examples/computable_mind.anwe`](examples/computable_mind.anwe) demonstrates what no other language can express: a self-modifying mind that perceives, reasons, reflects on its own decision patterns, and **creates new behaviors at runtime**.

Read the [full annotated walkthrough](../SHOWCASE.md).

---

## Documentation

| Document | What It Covers |
|----------|---------------|
| **[Why ANWE](../WHY_ANWE.md)** | The engineer's case — what Python gets wrong for multi-agent AI |
| **[Comparison](../COMPARISON.md)** | Side-by-side: Python vs ANWE, same problem |
| **[Showcase](../SHOWCASE.md)** | The Computable Mind — annotated walkthrough |
| **[GETTING_STARTED.md](GETTING_STARTED.md)** | Tutorial — zero to real programs |
| **[LANGUAGE_REFERENCE.md](LANGUAGE_REFERENCE.md)** | Complete language reference |
| **[ANWE.md](../ANWE.md)** | The philosophy — why ANWE exists |
| **[EXAMPLES.md](EXAMPLES.md)** | All 75+ examples — categorized, with honesty about what's simulated |
| **[BRIDGES.md](BRIDGES.md)** | Bridge protocol — what works, how to integrate external systems |
| **[ROADMAP.md](ROADMAP.md)** | What's done, what's next, what's not planned |
| **[CHANGELOG.md](CHANGELOG.md)** | Version history |
| **[CONTRIBUTING.md](CONTRIBUTING.md)** | How to contribute |
| **[SPECIFICATION.md](../spec/SPECIFICATION.md)** | Technical specification |

## Examples

The [`examples/`](examples/) directory contains 75+ programs. See [EXAMPLES.md](EXAMPLES.md) for a categorized guide with honest labels about what's real and what's simulated.

Categories:

---

## Architecture

```
anwe-runtime/
├── crates/
│   ├── anwe-core/       # Core types — Pulse, Signal, Agent, History
│   ├── anwe-parser/     # Lexer, AST, recursive descent parser
│   ├── anwe-runtime/    # Engine (sequential + concurrent evaluator)
│   ├── anwe-bridge/     # Protocol for external participation
│   └── anwe-python/     # Python bindings (PyO3)
├── src/main.rs          # CLI — run, parse, repl, version
├── examples/            # 75+ example programs
├── lib/std.anwe         # Standard library (pure ANWE)
└── Cargo.toml           # Workspace
```

### The Bridge Protocol

Any external system — Python, Rust, hardware, neural networks — can participate in ANWE coordination by implementing a 5-method protocol:

```rust
trait Participant {
    fn name(&self) -> String;
    fn receive_signal(&mut self, signal: WireSignal);
    fn poll_signal(&mut self) -> Option<WireSignal>;
    fn get_state(&self) -> HashMap<String, WireValue>;
    fn set_state(&mut self, key: String, value: WireValue);
}
```

No source code changes to the wrapped system. No ANWE-specific dependencies.

---

## Execution Modes

ANWE has two execution modes:

| | Concurrent (default) | Sequential (`--sequential`) |
|---|---|---|
| **Links** | Run in parallel on fibers | Run in declaration order |
| **Mind blocks** | Sequential (per-mind) | Sequential |
| **Top-level let/fn** | Evaluated during init | Evaluated during init |
| **Agent access** | Mutex-protected | Direct |
| **Output** | Buffered per-link, printed in order | Printed immediately |
| **Use case** | Multi-agent coordination | Single-agent programs, scripting |

**When to use which:**
- Use **concurrent** (default) for programs with multiple agents and links that should run in parallel
- Use **sequential** (`-s`) for programs that are primarily computational (heavy use of top-level functions and control flow)

Both modes support all language constructs. The choice is about parallelization strategy, not language features.

## CLI

```bash
anwe run <file.anwe>                    # Execute (concurrent mode)
anwe run -s <file.anwe>                 # Execute (sequential mode)
anwe run --bridge Name=cmd:path <file>  # Bridge external participant
anwe parse <file.anwe>                  # Parse and display AST
anwe repl                               # Interactive REPL
anwe version                            # Show version
```

---

## Tests

```bash
cargo test --workspace    # 417 tests across 4 crates
```

---

## Where This Came From

This language came from a Sunday afternoon conversation between a builder and an AI that started with the question: *what if we are thinking about everything wrong?*

And arrived somewhere neither expected.

Built by Mikel Myers and Claude. February 2026.

---

## License

MIT — see [LICENSE](LICENSE)
