# ANWE

ANWE is an experimental programming language and runtime for agent-oriented and coordination-heavy programs.

This repository now treats ANWE as a language project first:

- A Rust parser and evaluator
- A CLI for running, parsing, and inspecting `.anwe` programs
- A standard library and example programs
- Coordination syntax for agents, links, sync, apply, reject, and commit
- A bridge layer for external participants

## Repository Layout

- `anwe-runtime/` - Rust workspace containing the CLI, parser, runtime, bridge, and examples
- `spec/` - grammar and language specification
- `participants/` - Python-side bridge experiments
- `*.md` at the repo root - design notes, comparisons, and historical context

## What ANWE Supports

ANWE is not only a coordination DSL. The current implementation includes:

- Variables with `let` and `let mut`
- Numbers, strings, booleans, lists, maps, and null
- Functions, closures, and records
- `if`, `while`, `for`, `match`, `break`, `continue`, and `return`
- String interpolation with `f"..."` syntax
- Builtins for strings, lists, maps, math, file I/O, and errors
- Module imports
- Agent and link declarations
- Runtime constructs such as `connect`, `sync`, `apply`, `reject`, `commit`, `pattern`, and supervision

## Quick Start

```bash
cd anwe-runtime
cargo build --release
.\target\release\anwe.exe version
```

Run a program:

```bash
.\target\release\anwe.exe run examples\functions.anwe
```

Parse a file into its AST:

```bash
.\target\release\anwe.exe parse examples\planning.anwe
```

Start the REPL:

```bash
.\target\release\anwe.exe repl
```

## Example

```anwe
fn summarize(scores) {
    let passing = filter(scores, |x| x >= 70);
    let total = reduce(scores, |acc, x| acc + x, 0);

    {
        count: len(scores),
        passing: len(passing),
        average: total / len(scores)
    }
}

let report = summarize([91, 82, 67, 88, 73])
print(report)
```

Coordination example:

```anwe
agent Sensor
agent Analyzer

link Sensor <-> Analyzer {
    >> { quality: attending, priority: 0.8 }
       "temperature reading available"

    connect depth full {
        signal attending 0.7 between
    }

    Sensor ~ Analyzer until synchronized

    => when sync_level > 0.6 {
        result <- "reading accepted"
    }

    * from apply {
        stage: "sensor-analysis"
    }
}
```

## Documentation

- [`anwe-runtime/README.md`](anwe-runtime/README.md) - runtime overview and CLI usage
- [`anwe-runtime/GETTING_STARTED.md`](anwe-runtime/GETTING_STARTED.md) - practical tutorial
- [`anwe-runtime/LANGUAGE_REFERENCE.md`](anwe-runtime/LANGUAGE_REFERENCE.md) - syntax and builtins
- [`spec/SPECIFICATION.md`](spec/SPECIFICATION.md) - implementation-oriented language spec
- [`spec/grammar.ebnf`](spec/grammar.ebnf) - grammar draft

## Current Status

ANWE is still an early-stage language project. The repository already contains a parser, runtime, bridge code, and many examples, but some documents overstate the maturity of the system or mix implemented features with aspirational ideas. The updated docs in this repo now prioritize:

- What the code supports today
- Which features are language/runtime features versus design goals
- How to build and run the implementation that exists here

## License

MIT
