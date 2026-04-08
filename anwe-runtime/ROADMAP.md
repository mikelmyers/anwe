# ANWE Roadmap

Where things stand, what's next, and what's not happening.

---

## What's Done (v1.0)

Released 2026-02-28. This is real, tested, and working:

- **Full language implementation** -- lexer, recursive descent parser, AST, sequential and concurrent evaluators
- **Seven attention primitives** -- Alert, Connect, Sync, Apply, Commit, Reject, Converge
- **First-person cognition** -- mind blocks with attend, think, express, sense, author
- **General-purpose language features** -- functions, closures, lambdas, if/else, while, for-in, match, break/continue/return, let/mut bindings
- **Data types** -- strings, numbers, booleans, lists, maps, null, structured errors
- **70+ builtin functions** -- string ops, math, list ops, map ops, higher-order functions (map/filter/reduce/sort/find/any/all), type conversion, JSON, HTTP, file I/O
- **Standard library** -- `lib/std.anwe` with 25 utility functions written in pure ANWE
- **Module system** -- `import "module" as Alias {}` with file resolution and namespace prefixing
- **Agent system** -- spawn/retire, state persistence (save/restore), history queries, dynamic multi-agent coordination
- **Concurrent execution** -- lock-free MPMC signal channel (64-byte cache-aligned), concurrent mind execution
- **Bridge protocol** -- 5-method trait for external system participation (Rust, Python, anything)
- **Python bindings** -- PyO3-based bindings in `crates/anwe-python/`
- **REPL** -- multi-line input, `:load` command, persistent engine state
- **CLI** -- `anwe run`, `anwe parse`, `anwe repl`, `anwe version`
- **417 tests** across 4 crates
- **75+ example programs** covering RAG pipelines, model routing, federated learning, memory systems, reasoning patterns, safety guardrails, swarm coordination
- **Documentation** -- language reference, getting started tutorial, specification, contributing guide

---

## What's Next (v1.1 -- Near Term)

Realistic next steps. No dependencies on anything speculative.

- **VS Code syntax highlighting** -- TextMate grammar for `.anwe` files. Available in [`editors/vscode/`](editors/vscode/) — see install instructions there.
- **Better error messages** -- line/column info on parse and runtime errors. The parser tracks positions internally but doesn't surface them well in error output.
- **REPL improvements** -- command history (readline/rustyline), tab completion for builtins and loaded names, `:help` command.
- **Documentation site** -- static site (mdBook or similar) generated from the existing markdown docs.
- **More bridge implementations** -- HTTP/REST bridge (JSON over HTTP), stdin/stdout bridge (line-delimited JSON), WebSocket bridge. The protocol is defined; these are transport layers.
- **Test coverage for examples** -- automated runner that executes all 75+ examples and checks for crashes/panics.
- **Stdlib expansion** -- more utility functions in pure ANWE as real usage reveals gaps.

---

## Future (v1.2+)

Things that would make ANWE production-grade. Each is a real project.

- **Language Server Protocol (LSP)** -- diagnostics, go-to-definition, hover info, completions. Prerequisite: clean span tracking throughout the AST.
- **Package manager / module registry** -- `anwe install`, versioned dependencies, a registry for shared ANWE modules.
- **Debugger** -- step-through execution, breakpoints, agent state inspection. The sequential evaluator is the natural target.
- **Attention budget profiler** -- visualize how attention flows through agent systems, where budgets are spent, where synchronization stalls.
- **Real LLM bridges** -- OpenAI, Anthropic, and local model bridges so agents can actually call LLMs. This is the obvious next integration but requires careful API design for streaming, token limits, and error handling.
- **Distributed execution** -- agent systems spanning multiple machines. The signal channel abstraction could be backed by a network transport, but this is nontrivial.
- **Hot-reload** -- reload agent programs without restarting the runtime. Useful for development loops.
- **Persistent agent memory** -- durable state across runtime restarts (SQLite or similar), beyond the current save/restore JSON serialization.

---

## Not Planned

Being honest about scope:

- **Not a general-purpose language.** ANWE has general-purpose features because agents need them, but if you're writing a web server or a CLI tool, use Rust or Python. ANWE is for agent coordination.
- **Not a replacement for orchestration frameworks.** LangChain, CrewAI, AutoGen solve the "glue Python functions to LLM calls" problem. ANWE solves a different problem: giving agents a native language for attention and coordination. They're complementary.
- **Not optimizing for raw performance.** The runtime is written in Rust and is fast enough, but the goal is expressiveness for agent systems, not competing on benchmarks.
- **Not building a cloud platform.** ANWE is a language and runtime. It runs on your machine. There's no managed service planned.

---

*Last updated: 2026-02-28*
