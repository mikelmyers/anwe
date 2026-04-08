# ANWE Feature Matrix

This file classifies ANWE features using the current implementation as the source of truth.

Classification:

- `Stable` - parser + runtime + tests are present
- `Experimental` - implemented and tested, but still a moving target in language design
- `Draft` - documented or partially modeled, but not yet the cleanest source of truth

## Stable

These features are clearly implemented and exercised by the current test suite.

### Core Language

- Variables: `let`, `let mut`, reassignment
- Expressions: arithmetic, comparison, logical operators, grouping
- Control flow: `if`, `while`, `for ... in`, `break`, `continue`, `return`
- Functions: named functions, expression bodies, block bodies, closures
- Data types: number, string, bool, list, map, null, error
- Pattern matching with `match`
- Records
- String interpolation with `f"..."` syntax
- Builtins for strings, lists, maps, math, JSON, and reflection
- Error handling with `try/catch`
- File I/O: read, write, append, exists, lines
- Module imports
- REPL expression evaluation

Evidence:

- [`crates/anwe-runtime/src/engine.rs`](crates/anwe-runtime/src/engine.rs)
- [`crates/anwe-parser/src/parser.rs`](crates/anwe-parser/src/parser.rs)

### Coordination Runtime

- `agent`
- `link`
- `>>` alert
- `connect`
- `~` sync
- `=>` apply
- `<=` reject
- `*` commit
- `converge`
- `pattern`
- `pending?`
- link priorities
- time-based scheduling (`every`, `after`)
- supervision trees
- concurrent execution mode
- sequential execution mode

Evidence:

- [`crates/anwe-runtime/src/concurrent.rs`](crates/anwe-runtime/src/concurrent.rs)
- [`crates/anwe-runtime/src/engine.rs`](crates/anwe-runtime/src/engine.rs)
- [`crates/anwe-runtime/src/channel.rs`](crates/anwe-runtime/src/channel.rs)
- [`crates/anwe-runtime/src/scheduler.rs`](crates/anwe-runtime/src/scheduler.rs)

### Mind / First-Person Features

- `mind`
- `attend`
- `think`
- `express`
- `sense`
- `author`

Evidence:

- Parser tests such as `parse_mind_basic` in [`crates/anwe-parser/src/parser.rs`](crates/anwe-parser/src/parser.rs)
- Runtime tests such as `engine_mind_basic` in [`crates/anwe-runtime/src/engine.rs`](crates/anwe-runtime/src/engine.rs)

### Bridge and Persistence

- external agent declarations
- callback/participant bridge integration
- bridge-driven signal, apply, reject, commit notifications
- save/restore persistence

Evidence:

- `bridge_echo_participant_receives_signals` in [`crates/anwe-runtime/src/engine.rs`](crates/anwe-runtime/src/engine.rs)
- participant and registry tests in [`crates/anwe-bridge/src/participant.rs`](crates/anwe-bridge/src/participant.rs)
- persistence tests in [`crates/anwe-runtime/src/engine.rs`](crates/anwe-runtime/src/engine.rs)

## Experimental

These features appear to be implemented and tested, but they still feel like an evolving language surface and should be treated carefully in public docs.

- `spawn` / `retire`
- `sync_all`
- `broadcast`
- multi-agent converge forms
- stream/buffer constructs
- history query blocks
- align blocks
- quoting/unquoting code
- some reflection-style builtins and AI-oriented orchestration patterns

Why experimental:

- they expand the language surface significantly
- they are more likely to drift against the grammar/spec
- they are less essential than the core language + core coordination model

## Draft

These areas should not be treated as fully normative yet.

- [`../spec/grammar.ebnf`](../spec/grammar.ebnf) as a complete formal grammar
- older prose docs that describe ANWE in philosophical rather than implementation terms
- any feature described in docs without a matching parser/runtime/test path

## Current Build Note

On this machine, the workspace builds and tests successfully with:

- Rust stable via `rustup`
- `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1`

That environment detail matters because the installed Python is newer than the PyO3 version pinned in this repository.

## Recommendation

For public repo messaging, ANWE should present:

- the `Stable` section as the official language surface
- the `Experimental` section as available but evolving
- the `Draft` section as non-normative
