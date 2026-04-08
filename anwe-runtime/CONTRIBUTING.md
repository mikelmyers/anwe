# Contributing to ANWE

Thank you for your interest in contributing to ANWE — the Native Language of Artificial Minds.

## Before You Start

Read these documents in order:

1. **[ANWE.md](../ANWE.md)** — The philosophy. Understand *why* before you write *what*.
2. **[CONSTRAINTS.md](../CONSTRAINTS.md)** — The non-negotiable rules. These protect the language's soul.
3. **[LANGUAGE_REFERENCE.md](LANGUAGE_REFERENCE.md)** — The complete language reference.

ANWE is not a conventional language. Contributions that understand its philosophy will land faster than those that treat it as "just another programming language."

## How to Contribute

### Reporting Issues

Open an issue on GitHub with:
- What you expected to happen
- What actually happened
- A minimal `.anwe` file that reproduces the problem
- Your OS and Rust version (`rustc --version`)

### Submitting Changes

1. Fork the repository
2. Create a feature branch from `main`
3. Write your changes
4. Add tests — every new feature needs tests
5. Run the full test suite: `cargo test --workspace`
6. Submit a pull request with a clear description

### What We're Looking For

**Good first contributions:**
- Bug fixes with tests
- New builtin functions (add to `eval_builtin` in both `engine.rs` and `concurrent.rs`)
- New example `.anwe` programs demonstrating language features
- Documentation improvements
- Standard library functions in `lib/std.anwe`

**Larger contributions (discuss first):**
- New language syntax or keywords
- Changes to the seven primitives
- Runtime architecture changes
- New crates or major dependencies

### Code Style

- Follow existing patterns in the codebase
- Match the indentation and formatting of surrounding code
- Add builtin functions to both `engine.rs` and `concurrent.rs`
- Tests go in the `#[cfg(test)] mod tests` block at the end of the relevant file
- Name tests descriptively: `v09_return_from_function`, not `test1`

### The Seven Primitives Are Sacred

The seven primitives (Alert, Connect, Sync, Apply, Commit, Reject, Converge) are the foundation of ANWE. They are not methods. They are not functions. They are the seven things a mind does when it encounters reality. Changes to their semantics require exceptional justification.

### Tests

All tests must pass before a PR will be reviewed:

```bash
cargo test --workspace
```

Current test count: 417 tests across 4 crates.

## Project Structure

```
anwe-runtime/
├── crates/
│   ├── anwe-core/       # Core types (Pulse, Signal, Agent, History)
│   ├── anwe-parser/     # Lexer, AST, recursive descent parser
│   ├── anwe-runtime/    # Execution engine (sequential + concurrent)
│   ├── anwe-bridge/     # External participation protocol
│   └── anwe-python/     # Python bindings (PyO3)
├── src/main.rs          # CLI entrypoint
├── examples/            # Example .anwe programs
├── lib/std.anwe         # Standard library (pure ANWE)
└── Cargo.toml           # Workspace configuration
```

## License

By contributing to ANWE, you agree that your contributions will be licensed under the MIT License.

## Questions?

Open an issue or start a discussion. We're building something that has never existed before — a language for minds. Your perspective matters.
