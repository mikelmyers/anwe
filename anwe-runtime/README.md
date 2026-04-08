# ANWE Runtime

ANWE is an experimental language for agent coordination, stateful workflows, and general scripting. This directory contains the executable implementation: parser, runtime, CLI, examples, and bridge support.

## What This Project Is

Inside `anwe-runtime/` you have:

- `anwe-core` - runtime data structures such as signals, agents, links, and history
- `anwe-parser` - lexer, AST, and parser for `.anwe` source files
- `anwe-runtime` - sequential and concurrent evaluators
- `anwe-bridge` - participant bridge layer for external systems
- `anwe-python` - Python bindings built with PyO3
- `examples/` - sample programs
- `lib/std.anwe` - standard library module
- `src/main.rs` - CLI entry point

## Language Summary

ANWE combines ordinary language features with coordination primitives.

General-purpose features:

- `let`, `let mut`, assignment
- Functions and closures
- Lists, maps, strings, numbers, booleans, null
- `if`, `while`, `for`, `match`, `return`, `break`, `continue`
- Records and modules
- Errors and `try/catch`
- File I/O and common collection/string builtins

Coordination features:

- `agent`
- `link A <-> B`
- `>>` alerts
- `connect`
- `~` synchronization
- `=>` apply
- `<=` reject
- `*` commit
- `pattern`
- `supervise`
- external participants through `external(...)` and bridge wiring

## Build

```bash
cargo build --release
```

Windows:

```bash
.\target\release\anwe.exe version
```

Unix-like systems:

```bash
./target/release/anwe version
```

## CLI

The CLI implemented in [`src/main.rs`](src/main.rs) currently supports:

```bash
anwe run <file.anwe>
anwe run --sequential <file.anwe>
anwe run --bridge Name=cmd:path <file.anwe>
anwe parse <file.anwe>
anwe repl
anwe version
anwe bench
anwe hello
```

## Example: General Scripting

```anwe
fn average(items) {
    let total = reduce(items, |acc, x| acc + x, 0);
    total / len(items)
}

let scores = [91, 82, 67, 88, 73]
print(average(scores))
```

## Example: Agent Coordination

```anwe
agent Sensor
agent Analyzer

link Sensor <-> Analyzer priority high {
    >> { quality: attending, priority: 0.8 }
       "new sample available"

    connect depth full {
        signal attending 0.7 between
    }

    Sensor ~ Analyzer until synchronized

    => when sync_level > 0.6 {
        outcome <- "analysis complete"
    }

    * from apply {
        stage: "sample-processing"
    }
}
```

## Execution Modes

`anwe run` defaults to the concurrent engine. `anwe run --sequential` uses the sequential engine.

Use concurrent mode when the program is mostly link-based coordination. Use sequential mode when the program is mostly computation, expressions, and top-level scripting.

## External Participants

ANWE can route specific agents to external participants:

```bash
anwe run --bridge Sensor=cmd:participant examples/bridge_echo.anwe
```

The bridge layer lives in `anwe-bridge` and `anwe-python`. Replace `participant` with the command spec expected by your bridge adapter.

## Documentation

- [`GETTING_STARTED.md`](GETTING_STARTED.md)
- [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md)
- [`LANGUAGE_REFERENCE.md`](LANGUAGE_REFERENCE.md)
- [`EXAMPLES.md`](EXAMPLES.md)
- [`BRIDGES.md`](BRIDGES.md)
- [`ROADMAP.md`](ROADMAP.md)

## Verified Surface

The current implementation has a passing workspace test suite with 417 tests. For repo-facing claims about what is supported today, use [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) as the canonical summary.

## Notes on Maturity

This repository contains real language/runtime code, but it is still early-stage software. A few docs in the repo describe ANWE in more ambitious terms than the code currently justifies. The direction of this rewrite is to document ANWE as:

- a real parser and interpreter project
- a coordination-oriented language experiment
- an implementation with both working features and unfinished areas

That framing is much closer to the code in this workspace.
