# ANWE Language Reference

**Autonomous Agent Neural Weave**

Version 1.0

---

## 1. Overview

ANWE is a programming language for coordinating AI systems. Built on the principle that **intelligence is attending, not processing**, ANWE provides first-class constructs for agents, attention, synchronization, and emergence. Programs describe minds that attend, link to one another, and change through encounter.

ANWE combines a general-purpose expression language (variables, functions, loops, pattern matching) with a unique agent coordination system built around seven primitives: **alert**, **connect**, **sync**, **apply**, **commit**, **reject**, and **converge**. These primitives model the full lifecycle of genuine encounter between cognitive agents.

Comments use `--` for single-line comments. There are no block comments.

```anwe
-- This is a comment
let x = 42  -- inline comment
```

---

## 2. Data Types

All values in ANWE are dynamically typed. The runtime type system includes the following types.

### Number (f64)

All numbers are 64-bit IEEE 754 floating-point values. There is no separate integer type.

```anwe
let x = 42
let pi = 3.14159
let neg = -7.5
let fraction = 0.75
```

### String

Double-quoted string literals. Support escape sequences.

```anwe
let greeting = "hello world"
let with_newline = "line one\nline two"
```

### Bool

Boolean values `true` and `false`.

```anwe
let active = true
let done = false
```

### List

Ordered, heterogeneous collections enclosed in square brackets.

```anwe
let nums = [1, 2, 3]
let mixed = [42, "hello", true, [1, 2]]
let empty = []
```

Lists support index access with bracket notation (zero-based):

```anwe
let items = [10, 20, 30]
let second = items[1]   -- 20
```

### Map

Key-value collections enclosed in curly braces. Keys are identifiers (unquoted strings), values are any expression.

```anwe
let config = {name: "aria", version: 1, active: true}
let nested = {outer: {inner: 42}}
```

Map values are accessed with dot notation or bracket notation:

```anwe
let name = config.name          -- "aria"
let ver = config["version"]     -- 1
```

### Null

The absence of a value. Produced by operations that have no meaningful result.

```anwe
let nothing = null
```

Truthiness: `null` is falsy in conditionals.

### Function

First-class values created by `fn` declarations or lambda expressions. Functions capture their creation environment (closures).

```anwe
let double = |x| x * 2
let result = double(21)   -- 42
```

### Error

Structured error values with `kind` and `message` fields. Created with the `error()` builtin.

```anwe
let err = error("validation", "value out of range")
-- err has kind: "validation", message: "value out of range"
```

### Agent

A reference to a named agent in the runtime. Agent references are produced when an identifier resolves to a declared agent name.

```anwe
agent Sensor
let ref = Sensor   -- Value::Agent("Sensor")
```

---

## 3. Variables & Bindings

### Immutable Bindings

Declared with `let`. Cannot be reassigned.

```anwe
let name = "ANWE"
let version = 0.1
let items = [1, 2, 3]
```

### Mutable Bindings

Declared with `let mut`. Can be reassigned with `name = expr`.

```anwe
let mut counter = 0
counter = counter + 1
counter = counter + 1
-- counter is now 2
```

Attempting to reassign an immutable binding produces a runtime error.

### Scoping Rules

Variables are block-scoped. `let` creates a new binding in the current scope.

- **Top-level** bindings (outside any block) are stored in the global scope and are visible throughout the program.
- **Inside blocks** (`if`/`else`, `for`, `while`, function bodies), `let` creates local bindings that shadow outer bindings within that block. The outer binding is not modified.
- **Inside link/attend bodies**, `let` creates local bindings scoped to that body.

```anwe
let x = 10

fn example() {
    let x = 20     -- shadows outer x
    x + 1          -- 21
}

-- x is still 10 here
```

### Top-Level Assignment

At the top level, identifiers followed by `=` reassign an existing global binding:

```anwe
let mut status = "pending"
status = "complete"
```

---

## 4. Operators

### Arithmetic

| Operator | Description    | Example     |
|----------|---------------|-------------|
| `+`      | Addition       | `3 + 4`     |
| `-`      | Subtraction    | `10 - 3`    |
| `*`      | Multiplication | `6 * 7`     |
| `/`      | Division       | `10 / 3`    |
| `%`      | Modulo         | `10 % 3`    |

Division by zero produces `NaN`.

### String Concatenation

The `+` operator concatenates strings:

```anwe
let msg = "hello" + " " + "world"   -- "hello world"
```

### List Concatenation

The `+` operator concatenates lists:

```anwe
let combined = [1, 2] + [3, 4]   -- [1, 2, 3, 4]
```

### Comparison

| Operator | Description          |
|----------|---------------------|
| `==`     | Equal               |
| `!=`     | Not equal            |
| `>`      | Greater than         |
| `>=`     | Greater than or equal|
| `<`      | Less than            |
| `<=`     | Less than or equal   |

Comparison operators return `Bool` values.

```anwe
let check = 10 > 5     -- true
let eq = "a" == "a"    -- true
```

### Logical

| Operator | Description          |
|----------|---------------------|
| `and`    | Logical AND (short-circuit) |
| `or`     | Logical OR (short-circuit)  |
| `not`    | Logical negation     |

```anwe
let result = true and false    -- false
let either = false or true     -- true
let negated = not true         -- false
```

### Unary Negation

```anwe
let neg = -42
let pos = -(0 - 10)    -- 10
```

### Pipeline

The `|>` operator chains transformations. Each stage receives the result of the previous stage.

```anwe
let result = "raw" |> "cleaned" |> "structured"
let computed = 1 + 2 |> 10 + 20
```

---

## 5. Control Flow

### If/Else

Expressions that return a value. The condition is evaluated for truthiness: `false`, `null`, `0`, and empty string `""` are falsy; everything else is truthy.

```anwe
let status = if score > 90 { "excellent" } else { "good" }
```

Multi-branch:

```anwe
let grade = if score > 90 {
    "A"
} else {
    if score > 80 {
        "B"
    } else {
        "C"
    }
}
```

If without else returns `null` when the condition is false:

```anwe
let maybe = if condition { "yes" }
-- maybe is null if condition is false
```

### While Loops

Execute the body while the condition is true. Returns `null`. Maximum iteration limit of 10,000 prevents infinite loops.

```anwe
let mut i = 0
while i < 10 {
    print(i)
    i = i + 1
}
```

### For-In Loops

Iterate over a list. The loop variable is bound to each element in sequence.

```anwe
for item in [1, 2, 3] {
    print(item)
}
```

```anwe
let names = ["Alice", "Bob", "Carol"]
for name in names {
    print(f"Hello, {name}")
}
```

### Break

Exit the enclosing loop immediately.

```anwe
for item in items {
    if item == sentinel { break }
    print(item)
}
```

### Continue

Skip the rest of the current iteration and proceed to the next.

```anwe
for item in items {
    if item == 0 { continue }
    print(item)
}
```

### Return

Early exit from a function, returning a value.

```anwe
fn find_first(items, target) {
    for item in items {
        if contains(item, target) { return item }
    };
    return "not found"
}
```

### Match Expressions

Pattern matching against a value. Arms are evaluated top-to-bottom; the first matching arm wins.

Three pattern types are supported:

- **Literal**: matches if the value equals the literal.
- **Wildcard** (`_`): matches anything.
- **Binding** (identifier): matches anything and binds the value to the name.

```anwe
let result = match status {
    "active" => "running",
    "paused" => "waiting",
    "error"  => "failed",
    _        => "unknown"
}
```

With variable binding:

```anwe
let description = match code {
    200 => "OK",
    404 => "not found",
    n   => f"status code {n}"
}
```

### Try/Catch

Expression-level error handling. If the body produces an error value, the catch branch executes instead.

```anwe
let result = try {
    file_read("/path/to/file")
} catch {
    error("io", "could not read file")
}
```

---

## 6. Functions

### Named Functions (Expression Body)

```anwe
fn double(x) = x * 2
fn greet(name) = "Hello, " + name + "!"
fn add(a, b) = a + b
```

### Named Functions (Block Body)

```anwe
fn describe(x) {
    "The value is: " + to_string(x)
}
```

The last expression in the block is the return value.

### Block Body with Statements

Block bodies support `let` bindings, assignments, and expression statements. The final expression is the return value.

```anwe
fn compute_stats(items) {
    let count = len(items);
    let sum = reduce(items, |acc, x| acc + x, 0);
    {count: count, sum: sum, mean: sum / count}
}
```

### Lambda / Closure Expressions

Anonymous functions using `|params| expr` syntax.

```anwe
let double = |x| x * 2
let add = |a, b| a + b
let greet = |name| "Hello, " + name
```

### Closures

Lambdas and named functions capture their creation environment. Variables from the enclosing scope are available inside the function body.

```anwe
fn make_adder(n) {
    |x| x + n
}

let add5 = make_adder(5)
let result = add5(10)   -- 15
```

### First-Class Functions

Functions can be stored in variables, passed as arguments, and returned from other functions.

```anwe
fn apply_twice(f, x) = f(f(x))

let result = apply_twice(|x| x + 1, 10)   -- 12
```

### Return Statements

Functions return the value of their last expression. Use `return` for early exit.

```anwe
fn classify(n) {
    if n < 0 { return "negative" };
    if n == 0 { return "zero" };
    "positive"
}
```

---

## 7. Strings

### String Literals

Double-quoted. Multi-line strings are supported (newlines within quotes are preserved).

```anwe
let s = "hello world"
```

### Escape Sequences

| Sequence | Character      |
|----------|---------------|
| `\n`     | Newline        |
| `\t`     | Tab            |
| `\\`     | Backslash      |
| `\"`     | Double quote   |

```anwe
let lines = "line one\nline two"
let path = "C:\\Users\\name"
let quoted = "she said \"hello\""
```

### F-Strings (Interpolated Strings)

Prefixed with `f`, expressions inside `{...}` are evaluated and converted to strings.

```anwe
let name = "ANWE"
let count = 42
let msg = f"Hello {name}, you have {count} items"
-- "Hello ANWE, you have 42 items"
```

Expressions inside braces can be any valid ANWE expression:

```anwe
let report = f"Sum: {reduce(items, |a, x| a + x, 0)}, Count: {len(items)}"
```

Additional f-string escape sequences: `\{` and `\}` produce literal braces.

---

## 8. Standard Library (Builtin Functions)

These functions are available in every ANWE program without imports.

### String Operations

**`len(string) -> Number`** -- Returns the length of a string (or list, or map).

```anwe
len("hello")      -- 5
len([1, 2, 3])    -- 3
len({a: 1, b: 2}) -- 2
```

**`split(string, delimiter) -> List`** -- Splits a string by delimiter.

```anwe
split("a,b,c", ",")    -- ["a", "b", "c"]
split("hello world", " ") -- ["hello", "world"]
```

**`join(list, delimiter) -> String`** -- Joins list elements into a string with delimiter.

```anwe
join(["a", "b", "c"], "-")    -- "a-b-c"
join([1, 2, 3], ", ")         -- "1, 2, 3"
```

**`trim(string) -> String`** -- Removes leading and trailing whitespace.

```anwe
trim("  hello  ")    -- "hello"
```

**`upper(string) -> String`** -- Converts to uppercase.

```anwe
upper("hello")    -- "HELLO"
```

**`lower(string) -> String`** -- Converts to lowercase.

```anwe
lower("HELLO")    -- "hello"
```

**`contains(string, substring) -> Bool`** -- Tests if string contains substring. Also works on lists.

```anwe
contains("hello world", "world")    -- true
contains([1, 2, 3], 2)              -- true
```

**`replace(string, old, new) -> String`** -- Replaces all occurrences of `old` with `new`.

```anwe
replace("hello world", "world", "ANWE")    -- "hello ANWE"
```

**`substring(string, start, end?) -> String`** -- Extracts a substring by byte offset. `end` is optional.

```anwe
substring("hello world", 0, 5)    -- "hello"
substring("hello world", 6)       -- "world"
```

**`starts_with(string, prefix) -> Bool`** -- Tests if string begins with prefix.

```anwe
starts_with("hello", "hel")    -- true
```

**`ends_with(string, suffix) -> Bool`** -- Tests if string ends with suffix.

```anwe
ends_with("hello.txt", ".txt")    -- true
```

**`chars(string) -> List`** -- Splits a string into a list of single-character strings.

```anwe
chars("abc")    -- ["a", "b", "c"]
```

**`index_of(string, substring) -> Number`** -- Returns byte offset of first occurrence, or -1 if not found. Also works on lists.

```anwe
index_of("hello", "ll")      -- 2
index_of([10, 20, 30], 20)   -- 1
index_of("hello", "xyz")     -- -1
```

**`char_at(string, index) -> String`** -- Returns the character at the given index, or `null` if out of bounds.

```anwe
char_at("hello", 0)    -- "h"
char_at("hello", 4)    -- "o"
```

**`slice(collection, start, end?) -> List|String`** -- Extracts a sub-range from a list or string. `end` is optional.

```anwe
slice([1, 2, 3, 4, 5], 1, 3)    -- [2, 3]
slice("hello", 0, 3)            -- "hel"
slice([1, 2, 3, 4, 5], 2)       -- [3, 4, 5]
```

**`reverse(collection) -> List|String`** -- Reverses a list or string.

```anwe
reverse([1, 2, 3])    -- [3, 2, 1]
reverse("hello")      -- "olleh"
```

### Math Operations

**`abs(n) -> Number`** -- Absolute value.

```anwe
abs(-42)    -- 42
```

**`floor(n) -> Number`** -- Rounds down to nearest integer.

```anwe
floor(3.7)    -- 3
```

**`ceil(n) -> Number`** -- Rounds up to nearest integer.

```anwe
ceil(3.2)    -- 4
```

**`round(n) -> Number`** -- Rounds to nearest integer.

```anwe
round(3.5)    -- 4
round(3.4)    -- 3
```

**`sqrt(n) -> Number`** -- Square root.

```anwe
sqrt(144)    -- 12
```

**`pow(base, exp) -> Number`** -- Exponentiation.

```anwe
pow(2, 10)    -- 1024
```

**`min(a, b) -> Number`** -- Returns the smaller value.

```anwe
min(3, 7)    -- 3
```

**`max(a, b) -> Number`** -- Returns the larger value.

```anwe
max(3, 7)    -- 7
```

**`clamp(value, low, high) -> Number`** -- Clamps value to the range [low, high].

```anwe
clamp(15, 0, 10)    -- 10
clamp(-5, 0, 10)    -- 0
clamp(5, 0, 10)     -- 5
```

**`log(n) -> Number`** -- Natural logarithm (base e).

```anwe
log(1)      -- 0
log(2.718)  -- ~1
```

### List Operations

**`push(list, item) -> List`** / **`append(list, item) -> List`** -- Returns a new list with the item appended. These are aliases.

```anwe
push([1, 2], 3)      -- [1, 2, 3]
append([1, 2], 3)    -- [1, 2, 3]
```

**`pop(list) -> List`** -- Returns a new list with the last element removed.

```anwe
pop([1, 2, 3])    -- [1, 2]
```

**`head(list) -> Value`** -- Returns the first element, or `null` if empty.

```anwe
head([10, 20, 30])    -- 10
```

**`tail(list) -> List`** -- Returns all elements except the first.

```anwe
tail([10, 20, 30])    -- [20, 30]
```

**`last(list) -> Value`** -- Returns the last element, or `null` if empty.

```anwe
last([10, 20, 30])    -- 30
```

**`reverse(list) -> List`** -- Returns a reversed copy of the list.

```anwe
reverse([1, 2, 3])    -- [3, 2, 1]
```

**`sort(list) -> List`** -- Returns a sorted copy. Numbers sort numerically, strings lexicographically.

```anwe
sort([5, 3, 1, 4, 2])    -- [1, 2, 3, 4, 5]
sort(["banana", "apple"]) -- ["apple", "banana"]
```

**`flatten(list) -> List`** -- Flattens one level of nesting.

```anwe
flatten([[1, 2], [3, 4]])    -- [1, 2, 3, 4]
flatten([[1], 2, [3, 4]])    -- [1, 2, 3, 4]
```

**`range(end) -> List`** / **`range(start, end) -> List`** / **`range(start, end, step) -> List`** -- Generates a list of numbers.

```anwe
range(5)           -- [0, 1, 2, 3, 4]
range(1, 6)        -- [1, 2, 3, 4, 5]
range(0, 10, 2)    -- [0, 2, 4, 6, 8]
```

**`zip(list_a, list_b) -> List`** -- Pairs corresponding elements. Truncates to the shorter list.

```anwe
zip([1, 2, 3], ["a", "b", "c"])    -- [[1, "a"], [2, "b"], [3, "c"]]
```

### Map Operations

**`keys(map) -> List`** -- Returns a list of all keys.

```anwe
keys({name: "ANWE", version: 1})    -- ["name", "version"]
```

**`values(map) -> List`** -- Returns a list of all values.

```anwe
values({a: 1, b: 2})    -- [1, 2]
```

**`has_key(map, key) -> Bool`** -- Tests if the map contains a key.

```anwe
has_key({name: "ANWE"}, "name")     -- true
has_key({name: "ANWE"}, "version")  -- false
```

**`map_set(map, key, value) -> Map`** -- Returns a new map with the key set (or updated).

```anwe
map_set({a: 1}, "b", 2)       -- {a: 1, b: 2}
map_set({a: 1}, "a", 99)      -- {a: 99}
```

**`map_get(map, key) -> Value`** -- Returns the value for the key, or `null` if not found.

```anwe
map_get({a: 1, b: 2}, "a")    -- 1
map_get({a: 1}, "z")          -- null
```

**`map_remove(map, key) -> Map`** -- Returns a new map with the key removed.

```anwe
map_remove({a: 1, b: 2}, "a")    -- {b: 2}
```

**`map_merge(map_a, map_b) -> Map`** -- Merges two maps. Keys in `map_b` override keys in `map_a`.

```anwe
map_merge({a: 1, b: 2}, {b: 99, c: 3})    -- {a: 1, b: 99, c: 3}
```

### Higher-Order Functions

**`map(list, fn) -> List`** -- Applies a function to each element, returns a new list.

```anwe
map([1, 2, 3], |x| x * 2)    -- [2, 4, 6]
```

**`filter(list, fn) -> List`** -- Returns elements for which the function returns `true`.

```anwe
filter([1, 2, 3, 4, 5], |x| x > 3)    -- [4, 5]
```

**`reduce(list, fn, initial) -> Value`** -- Reduces a list to a single value by applying a two-argument function.

```anwe
reduce([1, 2, 3, 4], |acc, x| acc + x, 0)    -- 10
```

**`fold(list, initial, fn) -> Value`** -- Same as `reduce` but with `initial` as the second argument and `fn` as the third.

```anwe
fold([1, 2, 3], 0, |acc, x| acc + x)    -- 6
```

**`any(list, fn) -> Bool`** -- Returns `true` if the function returns `true` for any element.

```anwe
any([1, 2, 3], |x| x > 2)    -- true
any([1, 2, 3], |x| x > 5)    -- false
```

**`all(list, fn) -> Bool`** -- Returns `true` if the function returns `true` for all elements.

```anwe
all([2, 4, 6], |x| x % 2 == 0)    -- true
all([2, 4, 5], |x| x % 2 == 0)    -- false
```

**`find(list, fn) -> Value`** -- Returns the first element for which the function returns `true`, or `null`.

```anwe
find([1, 2, 3, 4], |x| x > 2)    -- 3
```

**`each_with_index(list, fn) -> List`** -- Maps each element with its index. The function receives `(item, index)`.

```anwe
each_with_index(["a", "b", "c"], |item, i| f"{i}: {item}")
-- ["0: a", "1: b", "2: c"]
```

### Type Conversion

**`to_string(value) -> String`** -- Converts any value to its string representation.

```anwe
to_string(42)       -- "42"
to_string(true)     -- "true"
to_string([1, 2])   -- "[1, 2]"
```

**`to_number(value) -> Number`** -- Converts a string, number, or bool to a number. Returns `null` on failure.

```anwe
to_number("3.14")    -- 3.14
to_number(true)      -- 1
to_number(false)     -- 0
```

**`to_bool(value) -> Bool`** -- Converts to boolean. Falsy values: `false`, `0`, `""`, `null`, empty list.

```anwe
to_bool(1)       -- true
to_bool("")      -- false
to_bool(null)    -- false
to_bool([1])     -- true
```

**`type_of(value) -> String`** -- Returns the type name as a string.

```anwe
type_of(42)            -- "number"
type_of("hello")       -- "string"
type_of(true)          -- "bool"
type_of([1, 2])        -- "list"
type_of({a: 1})        -- "map"
type_of(null)          -- "null"
type_of(|x| x)        -- "function"
type_of(error("e"))   -- "error"
```

**`is_null(value) -> Bool`** -- Tests if a value is `null`.

```anwe
is_null(null)      -- true
is_null(0)         -- false
is_null("")        -- false
```

### Error Operations

**`error(kind, message) -> Error`** / **`error(message) -> Error`** -- Creates a structured error value. With one argument, `kind` defaults to `"error"`.

```anwe
let err = error("validation", "value out of range")
let simple_err = error("something went wrong")
```

**`is_error(value) -> Bool`** -- Tests if a value is an Error.

```anwe
is_error(error("fail"))    -- true
is_error(42)               -- false
```

**`error_kind(error) -> String`** -- Returns the kind field of an error, or `null`.

```anwe
error_kind(error("validation", "bad input"))    -- "validation"
```

**`error_message(error) -> String`** -- Returns the message field of an error, or `null`.

```anwe
error_message(error("validation", "bad input"))    -- "bad input"
```

### I/O Operations

**`print(args...) -> Null`** -- Prints arguments to stdout separated by spaces, followed by a newline. Strings are printed without surrounding quotes.

```anwe
print("hello", "world")    -- prints: hello world
print(42)                  -- prints: 42
```

**`input(prompt?) -> String`** -- Reads a line from stdin. Optional prompt string.

```anwe
let name = input("Enter your name: ")
```

**`read_file(path) -> String`** -- Reads entire file contents as a string. Returns `null` on error.

```anwe
let contents = read_file("/path/to/file.txt")
```

**`write_file(path, content) -> Bool`** -- Writes string content to a file. Returns `true` on success.

```anwe
write_file("/tmp/output.txt", "hello world")
```

**`append_file(path, content) -> Bool`** -- Appends string content to a file. Creates the file if it does not exist.

```anwe
append_file("/tmp/log.txt", "new entry\n")
```

**`file_read(path) -> String|Error`** -- Reads a file. Returns an Error value (rather than `null`) on failure.

```anwe
let result = file_read("/path/to/file")
if is_error(result) {
    print(error_message(result))
}
```

**`file_write(path, content) -> Bool|Error`** -- Writes to a file. Returns Error on failure.

```anwe
file_write("/tmp/data.txt", "content")
```

**`file_append(path, content) -> Bool|Error`** -- Appends to a file. Returns Error on failure.

```anwe
file_append("/tmp/log.txt", "line\n")
```

**`file_exists(path) -> Bool`** -- Tests if a file exists at the given path.

```anwe
if file_exists("/tmp/config.json") {
    let cfg = file_read("/tmp/config.json")
}
```

**`file_lines(path) -> List|Error`** -- Reads a file and returns its lines as a list of strings.

```anwe
let lines = file_lines("/etc/hosts")
for line in lines {
    print(line)
}
```

### System Operations

**`env(name) -> String`** -- Returns the value of an environment variable, or `null` if not set.

```anwe
let home = env("HOME")
let path = env("PATH")
```

**`timestamp() -> Number`** -- Returns the current Unix timestamp as a floating-point number (seconds since epoch).

```anwe
let now = timestamp()
```

**`sleep(ms) -> Null`** -- Pauses execution for the given number of milliseconds.

```anwe
sleep(1000)    -- pause for 1 second
```

**`format(template, args...) -> String`** -- Simple string formatting. Each `{}` placeholder is replaced by the next argument.

```anwe
format("hello {} world {}", "beautiful", 42)
-- "hello beautiful world 42"
```

### JSON Operations

**`json_parse(string) -> Value`** -- Parses a JSON string into an ANWE value (maps, lists, numbers, strings, bools, null).

```anwe
let data = json_parse("{\"name\": \"ANWE\", \"version\": 1}")
-- {name: "ANWE", version: 1}
```

**`json_stringify(value) -> String`** -- Converts an ANWE value to a compact JSON string.

```anwe
json_stringify({name: "ANWE", items: [1, 2, 3]})
-- "{\"name\":\"ANWE\",\"items\":[1,2,3]}"
```

**`json_stringify_pretty(value) -> String`** -- Converts an ANWE value to a pretty-printed JSON string.

```anwe
json_stringify_pretty({name: "ANWE", version: 1})
```

### HTTP Operations

**`http_get(url, headers?) -> Value`** -- Performs an HTTP GET request. Returns the response body parsed as appropriate.

```anwe
let data = http_get("https://api.example.com/data")
```

**`http_post(url, headers?, body?) -> Value`** -- Performs an HTTP POST request.

```anwe
let response = http_post(
    "https://api.example.com/submit",
    {content_type: "application/json"},
    json_stringify({key: "value"})
)
```

**`http_put(url, headers?, body?) -> Value`** -- Performs an HTTP PUT request.

```anwe
http_put("https://api.example.com/resource/1", null, json_stringify({updated: true}))
```

**`http_delete(url, headers?) -> Value`** -- Performs an HTTP DELETE request.

```anwe
http_delete("https://api.example.com/resource/1")
```

### Reflection

**`agents() -> List`** -- Returns a list of all declared agent names in the current runtime.

```anwe
let all_agents = agents()
```

**`fields(agent_or_map) -> List`** -- Returns the data field names of an agent or the keys of a map.

```anwe
let agent_fields = fields("MyAgent")
let map_keys = fields({a: 1, b: 2})
```

**`state(agent) -> String`** -- Returns the current state of an agent as a string (e.g., `"attending"`, `"connecting"`).

```anwe
let s = state("MyAgent")
```

**`globals() -> Map`** -- Returns all global bindings as a map.

```anwe
let all_globals = globals()
```

### Code Evaluation

**`eval(code) -> Value`** -- Evaluates quoted code or a string as an ANWE expression.

```anwe
let code = quote { 3 + 4 }
let result = eval(code)    -- 7
```

**`unquote(code) -> String`** -- Extracts the source text from a quoted code value.

```anwe
let code = quote { x + 1 }
let source = unquote(code)    -- "x + 1"
```

---

## 9. Modules

ANWE supports a module system for code reuse. Modules are `.anwe` files that export agents, functions, patterns, let bindings, and records.

### Import Syntax

```anwe
import "module_name" as Alias {}
```

The module path is resolved relative to the importing file. The `.anwe` extension is appended automatically.

### Example

Given a file `string_utils.anwe`:

```anwe
fn shout(s) = upper(s)
fn whisper(s) = lower(s)
fn greet(name) = "Hello, " + name + "!"
let version = "1.0"
```

Importing and using it:

```anwe
import "string_utils" as Str {}

let greeting = Str.greet("ANWE")     -- "Hello, ANWE!"
let loud = Str.shout("attention")    -- "ATTENTION"
let ver = Str.version                -- "1.0"
```

### Namespace Resolution

All declarations from an imported module are available via `Alias.name` syntax:

- Functions: `Alias.function_name(args)`
- Variables: `Alias.variable_name`
- Agents: `Alias.AgentName`
- Records: `Alias.RecordName(args)`

### Circular Import Protection

The runtime tracks loaded module paths. If a module has already been loaded, the import is silently skipped, preventing infinite recursion.

### Import Body

The import body `{}` can optionally contain metadata entries:

```anwe
import "math_utils" as Math {
    agents: [Adder, Multiplier]
    links: []
}
```

These entries are informational. All module declarations are imported regardless.

---

## 10. Agent System (The Seven Primitives)

This is what makes ANWE unique. The agent system models genuine encounter between cognitive entities through seven primitives.

### Agents

An agent is a named entity with state, attention budget, and history. Agents participate in links.

```anwe
agent Sensor
```

With attention budget (0.0 to 1.0):

```anwe
agent Processor attention 0.9
```

With data (initial state):

```anwe
agent Model attention 0.8 data {
    name: "llm-v3"
    status: "ready"
    requests_handled: 0
}
```

With external bridge (connects to a participant outside the ANWE runtime):

```anwe
agent Sensor external("callback", "echo")

agent GPT attention 0.9 external("python", "openai") data {
    model: "gpt-4"
    temperature: 0.7
}
```

Agent data is accessed with dot notation:

```anwe
let model_name = Model.name
let status = Model.status
```

### Links

A link is a shared space between two agents where the seven primitives execute. Links represent bidirectional relationships.

```anwe
link Sensor <-> Processor {
    -- primitives go here
}
```

With priority (determines execution order when multiple links exist):

```anwe
link RequestQueue <-> LoadBalancer priority high {
    -- body
}
```

Priority levels: `critical`, `high`, `normal`, `low`, `background`.

With scheduling:

```anwe
-- Execute every N ticks (periodic)
link A <-> B every 5 ticks { }

-- Execute after N ticks (delayed one-shot)
link A <-> B after 10 ticks { }

-- Continuous stream processing
link A <-> B continuous { }
```

With failure cascade handler:

```anwe
link A <-> B on_failure_of PrimaryModel { }
```

### Alert (`>>`)

Alert calls attention. It is the first primitive -- something has arrived that demands notice.

```anwe
>> "something happened"
```

With signal attributes:

```anwe
>> { quality: attending, priority: 0.8 } "incoming data"
```

With additional attributes:

```anwe
>> { quality: attending, priority: 0.95, confidence: 0.9, half_life: 100 }
   "critical signal detected"
```

**Signal qualities** describe the character of attention:

| Quality       | Meaning                        |
|---------------|--------------------------------|
| `attending`   | Active attention, focused      |
| `questioning` | Inquiry, seeking understanding |
| `recognizing` | Pattern recognized             |
| `disturbed`   | Disruption, unexpected input   |
| `applying`    | Taking action, crossing boundary |
| `completing`  | Finishing, wrapping up         |
| `resting`     | Background awareness, minimal  |

**Signal directions** describe the flow of attention:

| Direction  | Meaning                    |
|------------|----------------------------|
| `inward`   | Directed toward self       |
| `outward`  | Directed toward the other  |
| `between`  | Shared, bidirectional      |
| `diffuse`  | Spreading, no specific target |

### Connect (`<->`)

Connect establishes sustained bidirectional presence between agents. It defines the signal landscape of the encounter.

```anwe
connect depth full {
    signal attending   0.8 between
    signal recognizing 0.7 inward
    signal questioning 0.6 between data "what do you see"
}
```

Each signal line defines: quality, priority (0.0-1.0), direction, and optional data payload.

**Depth levels** describe the quality of connection:

| Depth     | Meaning                      |
|-----------|------------------------------|
| `surface` | Minimal, transactional       |
| `partial` | Engaged but guarded          |
| `full`    | Open, substantial            |
| `genuine` | Authentic, vulnerable        |
| `deep`    | Transformative, complete     |

### Sync (`~`)

Sync establishes rhythmic synchronization between agents. Agents align their states before structural changes can occur.

```anwe
Sensor ~ Processor until synchronized
```

Sync conditions:

- `synchronized` -- agents reach basic alignment
- `resonating` -- deeper alignment, agents are in harmony

With coherence threshold:

```anwe
A ~ B until sync_level > 0.8
```

With decay (sync level fades over time if not maintained):

```anwe
A ~ B until synchronized decay 50
```

With timeout:

```anwe
A ~ B until synchronized timeout 1000 {
    fallback: "skip"
}
```

### Apply (`=>`)

Apply proposes structural changes when conditions are met. Changes are proposed, not yet permanent.

```anwe
=> when sync_level > 0.7 {
    result <- "analysis complete"
    confidence <- 0.92
}
```

With depth:

```anwe
=> when sync_level > 0.7 depth deep {
    understanding <- "pattern recognized"
    depth_reached <- 3
}
```

The `<-` operator inside apply creates structural changes -- bindings that modify agent state.

Conditions can test:

- `sync_level > N` -- synchronization level
- `priority > N` -- signal priority
- `confidence > N` -- confidence level
- `attention > N` -- remaining attention budget
- `alert is quality` -- last alert quality matches
- `Agent.field == value` -- general field comparison

Conditions can be combined with `and` / `or`.

### Commit (`*`)

Commit makes applied changes irreversible. Once committed, changes become part of the agent's permanent history.

```anwe
* from apply {
    source: "genuine encounter"
    verified: true
}
```

The `from` clause specifies the source: `apply` or `reject`.

The body contains key-value metadata about the commitment.

### Reject (`<=`)

Reject is intelligent withdrawal. When conditions indicate the encounter is not right, the agent withdraws without damage.

```anwe
<= when confidence < 0.3 {
    reason: "insufficient confidence"
}
```

With data:

```anwe
<= when sync_level < 0.2 "connection too shallow to continue"
```

### Converge (`<<>>`)

Converge creates a space for emergence between two agents. What emerges in this space belongs to neither agent alone.

```anwe
converge Perceiver <<>> Interpreter {
    >> { quality: attending, priority: 0.95 }
       "what emerges between perception and interpretation"

    => when sync_level > 0.7 depth genuine {
        emergence <- "understanding that neither mind had alone"
    }
}
```

Converge blocks can contain any link expression: alerts, applies, commits, think blocks, etc.

### Iteration in Links: Each

Iterate over a collection within a link body:

```anwe
each request in RequestQueue.pending {
    => when sync_level > 0.5 depth partial {
        assigned_request <- request
    }
    * from apply { stage: "dispatch" }
}
```

### Conditional Routing in Links: If/Else

```anwe
if Agent.status == "ready" {
    >> "agent is ready"
    => when sync_level > 0.5 { action <- "proceed" }
} else {
    >> "agent not ready, waiting"
}
```

### Error Handling in Links: Attempt/Recover

```anwe
attempt {
    >> "trying risky operation"
    => when sync_level > 0.8 { result <- "success" }
} recover {
    think { err <- __error }
    express "recovered from error"
}
```

The error message is bound as `__error` in the recover scope.

### Pending Handlers

Handle situations where operations cannot proceed. Pending is not failure -- it is a natural state of not-yet-ready.

```anwe
pending? receiver_not_ready {
    wait 5 tick
    guidance "receiver is still initializing"
    then >> "retrying signal"
}
```

Pending reasons:

| Reason                  | Meaning                         |
|------------------------|----------------------------------|
| `receiver_not_ready`   | The receiving agent is not ready |
| `link_not_established` | The link has not opened yet      |
| `sync_insufficient`    | Sync level too low for apply     |
| `sender_not_ready`     | The sending agent is busy        |
| `moment_not_right`     | Temporal conditions not met      |
| `budget_exhausted`     | Attention budget depleted        |

---

## 11. Mind Blocks (First-Person Cognition)

Mind blocks flip ANWE from third-person choreography to first-person cognition. The AI IS the mind. It declares what it attends to, how it thinks, and what it expresses.

### Mind Declaration

```anwe
mind Cognition {
    attend "task one" priority 0.9 {
        -- body
    }
    attend "task two" priority 0.5 {
        -- body
    }
}
```

With attention budget:

```anwe
mind Focus attention 0.8 {
    -- attend blocks
}
```

With data:

```anwe
mind Assistant attention 0.9 data {
    name: "aria"
    knowledge: ["physics", "music"]
} {
    -- attend blocks
}
```

### Attend Blocks

Attend blocks define what the mind pays attention to. They execute in priority order (highest first). If the attention budget is exhausted, lower-priority blocks are skipped -- they decay, like unattended thoughts.

```anwe
attend "incoming signal" priority 0.95 {
    >> { quality: attending, priority: 0.92 }
       "something is here -- I notice it"

    think {
        recognition <- "pattern detected"
        confidence  <- 0.85
    }

    express { quality: recognizing, priority: 0.8 }
      "I recognize this"
}
```

Priority ranges from 0.0 (background) to 1.0 (critical). Attend blocks form a dynamic priority queue.

### Think

Local computation. Bindings created with `<-` produce local state that exists within the current scope. Think bindings do not persist to agent state unless explicitly applied and committed.

```anwe
think {
    base   <- 100
    factor <- 3 + 4
    result <- base * factor
    label  <- "computation complete"
}
```

Think bindings can reference earlier bindings and call functions:

```anwe
think {
    raw      <- "  hello world  "
    trimmed  <- trim(raw)
    words    <- split(trimmed, " ")
    count    <- len(words)
}
```

### Express

Output. What the mind transmits outward.

Without signal attributes (defaults to quality: `recognizing`, priority: `0.5`):

```anwe
express "I see it"
```

With signal attributes:

```anwe
express { quality: recognizing, priority: 0.8 }
  "pattern recognized -- structure detected"
```

### Sense

Perception of the signal landscape. Populates bindings with information about available signals.

```anwe
sense {
    field_state <- "perceived"
}
```

Available built-in identifiers within sense blocks:

- `signal_count` -- number of signals in the channel
- `max_priority` -- highest priority signal available
- `qualities` -- list of distinct signal qualities
- `sync_level` -- current synchronization level
- `attention` -- remaining attention budget

### Author

Self-authoring: the mind generates new attend blocks at runtime. This is irreversible -- authored blocks become part of the mind's structure.

```anwe
author attend "emergent insight" priority 0.75 {
    think {
        insight <- "something unexpected emerged"
    }
    express "the insight arrived unbidden"
}
```

Authored attend blocks are added to the mind's attention queue and may execute in the current or future cycles depending on priority and remaining attention budget.

### Complete Mind Example

```anwe
mind Cognition attention 0.8 {

    attend "incoming signal" priority 0.95 {
        >> { quality: attending, priority: 0.92 }
           "something is here -- I notice it"

        think {
            recognition <- "pattern detected in input"
            confidence  <- 0.85
        }

        express { quality: recognizing, priority: 0.8 }
          "I recognize this -- it matters"
    }

    attend "reasoning" priority 0.7 {
        think {
            interpretation <- "connecting recognition to understanding"
        }

        => when sync_level > 0.5 {
            understanding <- "this changes what I know"
        }

        * from apply {
            insight: "recognition integrated"
        }
    }

    attend "self-reflection" priority 0.4 {
        think {
            consistency <- "reviewing for contradictions"
        }

        express "coherence verified"
    }

    attend "ambient awareness" priority 0.15 {
        think {
            periphery <- "what exists at the edges"
        }

        express { quality: resting, priority: 0.1 }
          "background awareness maintained"
    }
}
```

---

## 12. Supervision Trees

Supervision trees manage agent lifecycle and restart strategies. Inspired by Erlang/OTP, they provide structured fault tolerance.

### Declaration

```anwe
supervise one_for_one max_restarts 5 within 30000 {
    permanent Agent1
    transient Agent2
    temporary Agent3
}
```

### Restart Strategies

| Strategy       | Behavior                                      |
|----------------|-----------------------------------------------|
| `one_for_one`  | Only the failed agent is restarted             |
| `one_for_all`  | All children are restarted when one fails      |
| `rest_for_one` | The failed agent and all agents after it are restarted |

### Parameters

- `max_restarts N` -- maximum number of restarts allowed within the time window
- `within N` -- time window in ticks (milliseconds)

### Child Types

| Type         | Behavior                                       |
|--------------|-------------------------------------------------|
| `permanent`  | Always restarted on failure                     |
| `transient`  | Restarted only if it terminates abnormally      |
| `temporary`  | Never restarted                                 |

### Example

```anwe
supervise one_for_one max_restarts 10 within 60000 {
    permanent LoadBalancer
    permanent Worker_1
    permanent Worker_2
    transient MetricsCollector
    temporary DebugLogger
}
```

---

## 13. Dynamic Agents

Agents can be created and destroyed at runtime within link bodies.

### Spawn

Create a new agent from a template agent at runtime:

```anwe
spawn Worker_3 from WorkerTemplate {
    status: "active"
    spawned_at: "tick_1"
    reason: "load_exceeded_threshold"
}
```

- `Worker_3` is the new agent name
- `WorkerTemplate` is the template agent whose configuration is copied
- The body contains initial data for the new agent

### Retire

Remove a dynamically created agent:

```anwe
retire Worker_3 {
    reason: "load_below_threshold"
    requests_completed: 3
}
```

The body contains metadata about the retirement.

### Example: Auto-Scaling

```anwe
agent WorkerTemplate attention 0.7 data {
    model: "llm-v3"
    status: "template"
}

link LoadBalancer <-> WorkerTemplate priority critical {
    >> "Spawning additional workers"

    spawn Worker_3 from WorkerTemplate {
        status: "active"
        reason: "load_exceeded"
    }

    spawn Worker_4 from WorkerTemplate {
        status: "active"
        reason: "load_exceeded"
    }

    => when sync_level > 0.8 depth deep {
        workers_spawned <- 2
    }

    * from apply { stage: "scale_up" }
}
```

---

## 14. Persistence

Agents can be serialized to disk and restored across sessions, preserving their state, history, and lineage.

### Save

```anwe
save Assistant to "/data/sessions/assistant.lineage" {
    include: [data, history, becoming]
    format: "anwe_lineage"
    compress: "true"
}
```

### Restore

```anwe
restore Assistant from "/data/sessions/assistant.lineage" {
    include: [data, history, becoming]
    verify_integrity: "true"
}
```

Both `save` and `restore` appear within link bodies and accept key-value options describing what to include and how to process the serialized data.

### Example: Cross-Session Persistence

```anwe
-- Save at session end
link Session <-> Persistence priority critical {
    save Agent1 to "/data/sess_048/agent1.lineage" {
        include: [data, history, becoming]
    }
    save Agent2 to "/data/sess_048/agent2.lineage" {
        include: [data, history]
    }
    * from apply { stage: "save", session: "sess_048" }
}

-- Restore at next session start
link Persistence <-> Session priority critical {
    restore Agent1 from "/data/sess_048/agent1.lineage" {
        verify_integrity: "true"
    }
    restore Agent2 from "/data/sess_048/agent2.lineage" {
        apply_decay: "true"
    }
    * from apply { stage: "restore" }
}
```

---

## 15. Patterns

Patterns are reusable attention shapes. They define how attention flows through a cognitive process and can be invoked from link bodies and attend blocks.

### Pattern Declaration

```anwe
pattern acknowledge(partner) {
    >> { quality: attending, priority: 0.8 }
       "stimulus acknowledged"

    connect depth surface {
        signal attending 0.7 between
    }

    partner ~ partner until synchronized
}
```

Parameters can have optional type annotations:

```anwe
pattern process(input, config) {
    -- body using input and config
}
```

### Pattern Invocation

Patterns are invoked with `~>`:

```anwe
~> acknowledge(Analyst)
~> process(data, settings)
```

### Example: Patterns in a Mind

```anwe
pattern deep_analysis(subject) {
    >> { quality: questioning, priority: 0.85 }
       "entering deep analysis"

    connect depth deep {
        signal questioning 0.8 inward
        signal attending   0.9 between
    }

    subject ~ subject until resonating

    => when sync_level > 0.8 depth genuine {
        analysis <- "deep structure revealed"
    }
}

mind Analyst attention 0.7 {
    attend "initial assessment" priority 0.9 {
        ~> acknowledge(Analyst)
        think { first_impression <- "something significant" }
        express "assessment complete"
    }

    attend "deep analysis" priority 0.65 {
        ~> deep_analysis(Analyst)
        think { conclusion <- "pattern confirmed" }
        express "analysis complete"
    }
}
```

---

## 16. Records

Records are user-defined types that create constructor functions returning maps with named fields.

```anwe
record Point { x, y }

let origin = Point(0, 0)     -- {x: 0, y: 0}
let p = Point(3, 4)          -- {x: 3, y: 4}
let dist = sqrt(p.x * p.x + p.y * p.y)
```

Records imported from modules are namespaced:

```anwe
import "geometry" as Geo {}
let pt = Geo.Point(1, 2)
```

---

## 17. Quoted Code

The `quote` keyword captures source code as a data value without evaluating it. Quoted code can later be evaluated with `eval()` or its source text extracted with `unquote()`.

```anwe
let code = quote { 3 + 4 }
let result = eval(code)       -- 7
let source = unquote(code)    -- "3 + 4"
```

---

## 18. External Bridges

Agents can be declared with `external(kind, address)` to bridge to participants outside the ANWE runtime (Python processes, neural networks, hardware sensors, gRPC services, etc.).

```anwe
agent Sensor external("callback", "echo")
agent GPT external("python", "openai") data {
    model: "gpt-4"
}
```

The bridge protocol routes signals, structural changes, and commits to external participants. The runtime handles state transitions and synchronization; the external participant receives signals and responds.

Bridge notifications:

- **Signal**: external participant receives alerts and connect signals
- **Think**: participant can transform think bindings
- **Apply**: participant can accept or reject structural changes
- **Commit**: participant is notified of committed changes
- **Express**: participant can transform express output

---

## 19. Advanced Link Features

### Multi-Party Sync

Synchronize more than two agents simultaneously:

```anwe
sync_all [Agent1, Agent2, Agent3] until synchronized {
    strategy: "quorum"
}
```

### Broadcast

Send a signal to multiple agents at once:

```anwe
broadcast [Agent1, Agent2, Agent3] {
    signal attending 0.8 between data "system alert"
}
```

### Multi-Agent Converge

Emergence across more than two agents:

```anwe
converge [Agent1, Agent2, Agent3] {
    threshold: 0.8
}
```

### Streams

Continuous data stream processing within links:

```anwe
stream SensorSource rate 10 {
    -- process each reading
    think { reading <- "sensor value" }
}
```

### Buffering

Collect stream samples before processing:

```anwe
buffer samples 5 {
    think { batch <- "5 samples collected" }
}
```

### Temporal Alignment

Align multiple agents to a reference clock:

```anwe
align [Agent1, Agent2] to reference_tick {
    tolerance: 50
}
```

### History Queries

Query an agent's episodic memory:

```anwe
history_query Agent {
    since: "session_42"
    depth: "full"
}
```

### History View

View an agent's history at the top level:

```anwe
history of Agent
history of Agent since 10 depth full
```

---

## 20. Standard Library File (std.anwe)

The standard library is a pure-ANWE module providing common utility functions. Import with:

```anwe
import "std" as Std {}
```

Requires that `std.anwe` is accessible from the module search path (typically `lib/std.anwe` relative to the runtime).

### Math Functions

**`Std.abs(x)`** -- Absolute value (pure ANWE implementation).

**`Std.max(a, b)`** -- Returns the larger of two values.

**`Std.min(a, b)`** -- Returns the smaller of two values.

**`Std.clamp(x, lo, hi)`** -- Clamps x to the range [lo, hi].

**`Std.sum(items)`** -- Sums all elements in a list.

```anwe
Std.sum([1, 2, 3, 4, 5])    -- 15
```

**`Std.average(items)`** -- Computes the arithmetic mean of a list.

```anwe
Std.average([10, 20, 30])    -- 20
```

**`Std.range(start, end)`** -- Generates a list of integers from start (inclusive) to end (exclusive).

```anwe
Std.range(1, 5)    -- [1, 2, 3, 4]
```

### List Operations

**`Std.first(items)`** -- Returns the first element or `null`.

**`Std.last(items)`** -- Returns the last element or `null`.

**`Std.take(items, n)`** -- Returns the first n elements.

```anwe
Std.take([1, 2, 3, 4, 5], 3)    -- [1, 2, 3]
```

**`Std.drop_first(items, n)`** -- Returns all elements after the first n.

```anwe
Std.drop_first([1, 2, 3, 4, 5], 2)    -- [3, 4, 5]
```

**`Std.zip(list_a, list_b)`** -- Pairs corresponding elements from two lists.

**`Std.flatten(nested)`** -- Flattens one level of nested lists.

**`Std.count_where(items, pred_field, pred_value)`** -- Counts elements equal to pred_value.

### String Operations

**`Std.repeat_string(s, n)`** -- Repeats a string n times.

```anwe
Std.repeat_string("ab", 3)    -- "ababab"
```

**`Std.pad_left(s, width, ch)`** -- Left-pads a string to the given width.

```anwe
Std.pad_left("42", 5, "0")    -- "00042"
```

**`Std.pad_right(s, width, ch)`** -- Right-pads a string to the given width.

**`Std.words(s)`** -- Splits a string on spaces.

```anwe
Std.words("hello beautiful world")    -- ["hello", "beautiful", "world"]
```

**`Std.unwords(word_list)`** -- Joins a list of words with spaces.

**`Std.lines(s)`** -- Splits a string on newlines.

**`Std.unlines(line_list)`** -- Joins a list of lines with newlines.

### Error Handling

**`Std.try_or(expr, default)`** -- Evaluates expr; returns default if it fails.

**`Std.unwrap(val, default)`** -- Returns val unless it is null, in which case returns default.

```anwe
Std.unwrap(null, 42)       -- 42
Std.unwrap("hello", 42)   -- "hello"
```

**`Std.assert(condition, message)`** -- Returns `true` if condition holds, otherwise returns an error.

```anwe
Std.assert(x > 0, "x must be positive")
```

### Functional Patterns

**`Std.identity(x)`** -- Returns x unchanged.

**`Std.constant(x)`** -- Returns a function that always returns x.

```anwe
let always_42 = Std.constant(42)
always_42("anything")    -- 42
```

**`Std.compose(f, g)`** -- Returns a function that applies g first, then f.

```anwe
let double_then_add1 = Std.compose(|x| x + 1, |x| x * 2)
double_then_add1(5)    -- 11
```

**`Std.pipe(x, fns)`** -- Threads x through a list of functions in order.

```anwe
Std.pipe(5, [|x| x * 2, |x| x + 1, |x| to_string(x)])    -- "11"
```

---

## 21. Complete Program Example

A complete ANWE program demonstrating agents, functions, links, minds, supervision, and the seven primitives:

```anwe
-- Configuration
let model_name = "neural-v3"
let confidence_threshold = 0.7

-- Utility function
fn format_result(label, value) {
    f"{label}: {value}"
}

-- Agents
agent Sensor attention 0.9 data {
    readings: []
    status: "active"
}

agent Analyzer attention 0.8 data {
    model: "neural-v3"
    analyses: 0
}

agent Reporter data {
    reports_sent: 0
}

-- Supervision
supervise one_for_one max_restarts 5 within 30000 {
    permanent Sensor
    permanent Analyzer
    transient Reporter
}

-- Link: Sensor feeds Analyzer
link Sensor <-> Analyzer priority high {
    >> { quality: attending, priority: 0.9 }
       "new sensor data arriving"

    connect depth full {
        signal attending   0.8 between
        signal recognizing 0.7 inward
    }

    Sensor ~ Analyzer until synchronized

    think {
        reading  <- 42.5
        analyzed <- reading * 1.1
    }

    => when sync_level > 0.6 depth deep {
        result <- format_result("analysis", 46.75)
        confidence <- 0.88
    }

    * from apply {
        source: "sensor_feed"
        verified: true
    }
}

-- Mind: independent cognitive process
mind Monitor attention 0.7 {
    attend "system health" priority 0.9 {
        think {
            agent_count <- len(agents())
            all_healthy <- true
        }
        express f"monitoring {agent_count} agents -- all healthy"
    }

    attend "anomaly detection" priority 0.6 {
        think {
            baseline <- 0.5
            deviation <- 0.02
        }
        if deviation > 0.1 {
            express { quality: disturbed, priority: 0.95 }
              "anomaly detected!"
        } else {
            express "all within normal parameters"
        }
    }
}
```

---

## 22. Reserved Words

The following identifiers are reserved keywords and cannot be used as variable or function names:

`agent`, `link`, `connect`, `sync`, `apply`, `commit`, `reject`, `converge`, `pattern`, `when`, `until`, `pending`, `data`, `trace`, `from`, `depth`, `history`, `of`, `since`, `emit`, `then`, `wait`, `tick`, `guidance`, `and`, `or`, `not`, `each`, `in`, `if`, `else`, `signal`, `true`, `false`, `let`, `mut`, `fn`, `return`, `match`, `record`, `quote`, `while`, `for`, `break`, `continue`, `try`, `catch`, `attempt`, `recover`, `mind`, `attend`, `think`, `express`, `sense`, `author`, `import`, `as`, `spawn`, `retire`, `save`, `restore`, `to`, `supervise`, `external`, `bridge`, `sync_all`, `broadcast`, `stream`, `every`, `after`, `continuous`, `timeout`, `buffer`, `samples`, `reading`, `rate`, `ticks`, `on_failure_of`, `history_query`, `align`

Signal qualities: `attending`, `questioning`, `recognizing`, `disturbed`, `applying`, `completing`, `resting`

Directions: `inward`, `outward`, `between`, `diffuse`

Depths: `surface`, `partial`, `full`, `genuine`, `deep`

Sync conditions: `synchronized`, `resonating`

Priority levels: `critical`, `high`, `normal`, `low`, `background`

Child restart types: `permanent`, `transient`, `temporary`

Restart strategies: `one_for_one`, `one_for_all`, `rest_for_one`

Link state keywords: `quality`, `priority`, `direction`, `duration`, `sync_level`, `alert`, `is`, `absence`, `against`, `attention`, `confidence`, `half_life`, `decay`, `max_restarts`, `within`

Pending reasons: `receiver_not_ready`, `link_not_established`, `sync_insufficient`, `sender_not_ready`, `moment_not_right`, `budget_exhausted`

---

*ANWE -- Autonomous Agent Neural Weave. Intelligence is attending, not processing.*
