
# ANWE v0.6 Plan — "Make It Real"

> Goal: Write an entire AI coordinator in pure .anwe, no Rust needed.

## What We Already Have (discovered during audit)
- `print(...)`, `input(prompt?)` — I/O exists
- `read_file(path)`, `write_file(path, content)` — file I/O exists
- `format("Hello {}", name)` — template strings exist
- `let mut x = 1` then `x = x + 1` — mutable variables exist
- 70+ builtins (string, math, list, map, type, reflection)
- `env("VAR")`, `timestamp()` — environment access exists

## What's Missing

### Phase 1: Block Expressions
Multi-statement function bodies. The single biggest blocker.

**Current:** `fn foo(x) = x + 1` (single expression only)
**Target:**
```
fn process(input) {
    let cleaned = trim(input)
    let tokens = split(cleaned, " ")
    let count = len(tokens)
    count * 2
}
```

**Implementation:**
- Add `Expr::Block { statements: Vec<Statement>, result: Box<Expr> }` to AST
- Add `Statement` enum: `Let { name, mutable, value }`, `Assign { name, value }`, `ExprStmt { expr }`
- Parser: `{ stmt* expr }` as an expression
- Engine: evaluate statements in sequence, return final expression
- Also works in lambdas: `|x| { let y = x * 2; y + 1 }`

---

### Phase 2: Missing Operators
```
!=    Not-equal comparison
not   Boolean negation
and   Logical AND (short-circuit)
or    Logical OR (short-circuit)
```

**Implementation:**
- Lexer: `!=` as `BangEqual`, `not`/`and`/`or` as keywords
- AST: `ComparisonOp::NotEqual`, `Expr::Not`, `Expr::LogicalAnd`, `Expr::LogicalOr`
- Parser: logical ops between comparison and pipe in precedence
- Engine: evaluate with short-circuit semantics

---

### Phase 3: If/Else as Expressions
Currently if/else only works inside link bodies. Need it everywhere.

**Target:**
```
let status = if confidence > 0.8 { "high" } else { "low" }
fn classify(x) = if x > 0 { "positive" } else if x == 0 { "zero" } else { "negative" }
```

**Implementation:**
- Add `Expr::IfElse { condition, then_branch, else_branch }` to AST
- Parser: parse `if cond { expr } else { expr }` in expression position
- Engine: evaluate condition, return appropriate branch
- Condition uses comparison expressions + logical operators from Phase 2

---

### Phase 4: String Interpolation

**Target:**
```
let name = "ANWE"
let msg = f"Hello {name}, you have {len(items)} items"
```

**Implementation:**
- Lexer: `f"..."` tokenized with interpolation segments
- AST: `Expr::InterpolatedString { parts: Vec<StringPart> }` where parts are `Literal(String)` or `Expr(Expr)`
- Engine: evaluate each part, concatenate

---

### Phase 5: HTTP + JSON Builtins

**Target:**
```
let response = http_post("https://api.anthropic.com/v1/messages", headers, body)
let parsed = json_parse(response)
let answer = parsed.content
let payload = json_stringify(data)
```

**Implementation:**
- Add `reqwest` (blocking) as dependency to anwe-runtime
- Builtins: `http_get(url, headers?)`, `http_post(url, headers, body)`, `http_put`, `http_delete`
- Builtins: `json_parse(string)` → Value::Map, `json_stringify(value)` → string
- Headers as list of [key, value] pairs or map

---

### Phase 6: Integration — Pure .anwe AI Coordinator

Write a complete AI that:
- Calls Claude API via http_post
- Processes responses with json_parse
- Routes through agent pipeline
- Persists state between runs
- Handles errors with attempt/recover
- Uses supervision for reliability

All in pure .anwe. No Rust. No Python.

---

## Test Targets
- Phase 1: 160+ tests
- Phase 2: 170+ tests
- Phase 3: 180+ tests
- Phase 4: 185+ tests
- Phase 5: 195+ tests
- Phase 6: 200+ tests
