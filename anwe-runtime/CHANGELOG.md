# ANWE Changelog

All notable changes to the ANWE language and runtime.

## [1.0.0] — 2026-02-28

### Added
- Complete documentation for public release
- README.md — project overview and quick start
- LANGUAGE_REFERENCE.md — complete language reference
- GETTING_STARTED.md — tutorial from zero to real programs
- CONTRIBUTING.md — contribution guidelines
- LICENSE — MIT license
- This changelog

### Changed
- Version bumped to 1.0.0 for public release
- Updated Cargo.toml with repository metadata

---

## [0.9.0] — 2026-02-28

### Added
- **Return statements**: `return expr` for early function exit
- **New builtins**: `index_of` (strings and lists), `char_at`, `slice` (strings and lists)
- **Standard library**: `lib/std.anwe` — 25 utility functions written in pure ANWE
  - Math: abs, max, min, clamp, sum, average, range
  - List: first, last, take, drop_first, zip, flatten, count_where
  - String: repeat_string, pad_left, pad_right, words, unwords, lines, unlines
  - Error: try_or, unwrap, assert
  - Functional: identity, constant, compose, pipe
- **CLI polish**: `anwe version` command, "Autonomous Agent Neural Weave" branding
- 16 new tests (417 total)
- Integration example: `text_processor_v09.anwe`

### Fixed
- Return value propagation through for/while loops — Return now correctly bubbles up through loops instead of being consumed like Break

---

## [0.8.0] — 2026-02-27

### Added
- **Break/Continue**: `break` exits loops, `continue` skips to next iteration
- **Top-level assignment**: `let mut` bindings with reassignment via `name = expr`
- **Structured errors**: `Value::Error { kind, message }` with `error()`, `is_error()`, `error_kind()`, `error_message()` builtins
- **Try/catch expressions**: `try { expr } catch { fallback }` for expression-level error handling
- **File I/O**: `file_read`, `file_write`, `file_append`, `file_exists`, `file_lines` with structured error returns
- **Module imports**: `import "module" as Alias {}` with file resolution, parsing, and namespace prefixing
- **REPL upgrades**: Multi-line input, `:load` command, persistent engine state
- 100+ new tests (401 total)
- Integration example: `data_pipeline_v08.anwe`

---

## [0.7.0] — 2026-02-26

### Added
- **While loops**: `while condition { body }` as expressions
- **For-in loops**: `for item in collection { body }` as expressions
- **Map literals**: `{key: value}` syntax with field access via `map.key`
- **Try/catch**: Expression-level error handling
- **Sleep builtin**: `sleep(ms)` for timed operations
- **Lambda expressions**: `|params| body` syntax with closure capture
- **Block expressions**: `{ let x = 1; let y = 2; x + y }` with sequential statements
- Map operations: `keys`, `values`, `has_key`, `map_set`, `map_get`, `map_remove`, `map_merge`
- Higher-order functions: `map`, `filter`, `reduce`, `fold`, `any`, `all`, `find`, `each_with_index`
- Pattern matching: `match expr { pattern => body }`
- F-string interpolation: `f"hello {name}"`
- 100+ new tests (301 total)

---

## [0.6.0] — 2026-02-25

### Added
- **Pure ANWE programs**: Programs written entirely in ANWE without agent declarations
- **Let bindings**: `let name = expr` for variable binding
- **Functions**: `fn name(params) { body }` and `fn name(params) = expr`
- **HTTP builtins**: `http_get`, `http_post`, `http_put`, `http_delete`
- **JSON builtins**: `json_parse`, `json_stringify`, `json_stringify_pretty`
- **Reflection**: `agents()`, `fields()`, `state()`, `globals()`
- String operations: `len`, `split`, `join`, `trim`, `upper`, `lower`, `contains`, `replace`, `substring`, `starts_with`, `ends_with`, `chars`
- Math operations: `abs`, `floor`, `ceil`, `round`, `sqrt`, `pow`, `min`, `max`, `clamp`, `log`
- List operations: `push`/`append`, `pop`, `head`, `tail`, `last`, `reverse`, `sort`, `flatten`, `range`, `zip`
- Type conversion: `to_string`, `to_number`, `to_bool`, `type_of`, `is_null`
- I/O: `print`, `input`, `read_file`, `write_file`, `append_file`, `env`, `timestamp`, `format`
- AI coordinator example: `ai_coordinator_v06.anwe`

---

## [0.5.0] — 2026-02-25

### Added
- **Dynamic agents**: `spawn` and `retire` for runtime agent creation/destruction
- **Multi-agent sync**: `sync_all`, `broadcast`, `multi_converge`
- **State persistence**: `save` / `restore` with JSON serialization
- **History queries**: `history_query` for episodic memory
- **Map type**: `Value::Map` with stdlib functions
- **Concurrent mind execution**: All mind/attend/think/express/sense/author primitives work in concurrent engine
- **String interpolation**: `{name}` and `{Agent.field}` in strings
- **Each/IfElse in concurrent mode**: Iteration and conditionals in concurrent evaluator

---

## [0.1.0] — 2026-02-23

### Added
- Initial release — ANWE v0.1 "The First Transmission"
- Complete lexer and recursive descent parser
- Seven primitives: Alert (>>), Connect (<->), Sync (~), Apply (=>), Commit (*), Reject (<=), Converge (<<>>)
- First-person cognition: mind blocks with attend, think, express, sense, author
- Agent state machine with 7 states
- Attention system with finite budgets and temporal decay
- Lock-free MPMC signal channel (64-byte cache-aligned)
- Append-only irreversible history
- Pattern system with parameter substitution
- Bridge protocol for external participation (5-method trait)
- Python bindings via PyO3
- 68 example programs
- 145 passing tests
- CLI: `anwe run`, `anwe parse`, `anwe repl`, `anwe bench`, `anwe hello`
