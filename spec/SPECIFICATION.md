# ANWE Specification

This document describes ANWE as an implementation-oriented programming language project.

## 1. Scope

ANWE is an experimental language with two main layers:

- A general-purpose expression language
- A coordination layer built around agents, links, signals, synchronization, and commit/reject semantics

This specification is intentionally grounded in the repository implementation rather than a purely aspirational design.

## 2. Language Model

ANWE source files use the `.anwe` extension. Programs may contain:

- top-level bindings and functions
- records and imports
- agent declarations
- link blocks
- patterns
- supervision declarations
- mind/attend forms where supported by the runtime and parser

The language is dynamically typed.

Runtime value categories described in the codebase and reference docs include:

- number
- string
- bool
- list
- map
- null
- function
- error
- agent reference

## 3. Core General-Purpose Features

ANWE includes standard programming constructs:

- `let` and `let mut`
- assignment
- arithmetic and comparison operators
- `if` expressions
- `while` loops
- `for ... in` loops
- `match`
- functions and closures
- string interpolation
- records
- module imports
- file I/O
- structured errors with `try/catch`

Example:

```anwe
fn summarize(items) {
    let total = reduce(items, |acc, x| acc + x, 0);
    {
        count: len(items),
        total: total,
        average: total / len(items)
    }
}
```

## 4. Coordination Features

The coordination layer introduces language-level constructs that do not map to ordinary function calls.

### 4.1 Agents

Agents are named runtime participants.

```anwe
agent Sensor
agent Analyzer
agent Memory external("python", "participant.module")
```

Agents may carry metadata or be backed by external participants depending on the implementation path used.

### 4.2 Links

Links express bidirectional coordination between agents.

```anwe
link Sensor <-> Analyzer {
    >> { quality: attending, priority: 0.8 }
       "new sample"
}
```

### 4.3 Signal-Oriented Operators

The operator vocabulary currently documented and implemented across the repository includes:

- `>>` alert / signal emission
- `~` synchronization
- `=>` apply
- `<=` reject
- `*` commit
- `<->` bidirectional link declaration
- `<<>>` convergence in some examples and grammar drafts
- `~>` pattern invocation

### 4.4 Connect, Apply, Reject, Commit

Typical coordination flow:

```anwe
link Sensor <-> Analyzer {
    >> { quality: attending, priority: 0.8 }
       "reading available"

    connect depth full {
        signal attending 0.7 between
    }

    Sensor ~ Analyzer until synchronized

    => when sync_level > 0.6 {
        accepted <- true
    }

    * from apply {
        stage: "sensor-analysis"
    }
}
```

Semantically:

- `connect` describes signal exchange
- `~` represents synchronization
- `=>` applies structural or state changes
- `<=` exits or rejects under a condition
- `*` records a committed outcome

## 5. Execution Model

The CLI exposes two execution modes:

- concurrent mode by default
- sequential mode with `--sequential`

The implementation currently lives in the Rust crates under [`../anwe-runtime/crates/`](../anwe-runtime/crates).

The project also includes:

- a lock-free signal channel implementation
- a scheduler
- parser and lexer infrastructure
- a bridge registry for external participants

## 6. CLI Surface

The CLI in [`../anwe-runtime/src/main.rs`](../anwe-runtime/src/main.rs) supports:

- `run`
- `parse`
- `repl`
- `version`
- `bench`
- `hello`

Bridge wiring is exposed through:

```bash
anwe run --bridge Name=cmd:path file.anwe
```

## 7. Standard Library and Builtins

The language runtime and docs expose builtins for:

- strings: `split`, `join`, `trim`, `upper`, `lower`, `contains`, `replace`
- lists: `map`, `filter`, `reduce`, `find`, `sort`, `flatten`
- maps: `keys`, `values`, `map_get`, `map_set`, `map_merge`
- math: `sqrt`, `pow`, `min`, `max`, `clamp`
- conversion: `to_string`, `to_number`, `to_bool`
- files: `file_read`, `file_write`, `file_append`
- errors: `error`, `is_error`, `error_kind`, `error_message`

The repository also includes `lib/std.anwe`.

## 8. Grammar Status

The formal grammar draft is in [`grammar.ebnf`](grammar.ebnf).

That file is useful as a design artifact, but it should be treated as a draft until it is fully reconciled with the parser implementation. In the current repository, the parser and runtime are the source of truth when docs and grammar diverge.

## 9. Status and Honesty Notes

ANWE is already a real language project in the sense that this repository contains:

- source files in a distinct language
- a parser
- an evaluator/runtime
- a CLI
- examples
- bridge code

ANWE is not yet a finished or production-ready language. Some repository text still blends:

- implemented behavior
- design goals
- philosophy

This specification is meant to keep those separate and anchor future work in executable behavior.
