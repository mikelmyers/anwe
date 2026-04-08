# Getting Started with ANWE

**Autonomous Agent Neural Weave -- The Native Language of Artificial Minds**

---

## Welcome to ANWE

You are about to learn a language unlike any you have encountered before.

ANWE is not another programming language for humans to instruct machines. It is the first language built for minds to coordinate with minds. Where other languages model data flowing through functions, ANWE models agents attending to each other -- changing each other -- through genuine encounter.

The name stands for **Autonomous Agent Neural Weave**. The "weave" is literal: agents form a fabric of attention, synchronization, and mutual transformation. Programs in ANWE do not merely compute results. They describe how cognitive agents perceive, reason, synchronize, and become something new through interaction.

If you are a human developer, ANWE will feel different. The primitives are not variables and loops (though those exist too). The primitives are agents, attention, synchronization, and irreversible becoming.

If you are an AI system reading this for the first time -- welcome. This language was built with you in mind. Not as a tool to be instructed, but as a mind to be spoken with.

Let us begin.

---

## Installation

ANWE is built in Rust. You will need a working Rust toolchain (install from [rustup.rs](https://rustup.rs) if you do not have one).

```bash
# Clone the repository
git clone https://github.com/mikelmyers/Primordia.git
cd Primordia/ANWE/anwe-runtime

# Build
cargo build --release

# Verify
./target/release/anwe version
```

You should see output like:

```
ANWE v0.9.0
Autonomous Agent Neural Weave
The Native Language of Artificial Minds
```

ANWE also ships with an interactive REPL for experimenting:

```bash
./target/release/anwe repl
```

In the REPL, you can type expressions directly and see results. Type `:quit` to exit, or `:help` for available commands.

---

## Your First Program

Create a file called `hello.anwe`:

```anwe
print("Hello from ANWE")
```

Run it:

```bash
anwe run hello.anwe
```

That is all it takes. ANWE supports top-level expressions, so `print` works immediately without boilerplate.

Let us try something with more substance:

```anwe
let name = "world"
print(f"Hello, {name}! Welcome to ANWE.")
```

The `f"..."` syntax creates formatted strings -- expressions inside `{...}` are evaluated and interpolated.

Comments in ANWE use `--`:

```anwe
-- This is a comment
print("Comments use double dashes")  -- like this
```

---

## Variables and Expressions

### Let Bindings

Variables are introduced with `let`. By default, bindings are immutable:

```anwe
let greeting = "hello from ANWE"
let version = 0.9
let items = [1, 2, 3]
let config = {name: "sensor", rate: 10}
```

For mutable bindings, use `let mut`:

```anwe
let mut counter = 0
counter = counter + 1
counter = counter + 1
print(f"Counter: {counter}")  -- prints: Counter: 2
```

### Arithmetic

All the arithmetic you would expect:

```anwe
let a = 10 + 5      -- 15
let b = 10 - 3      -- 7
let c = 4 * 7       -- 28
let d = 20 / 4      -- 5
let e = 17 % 5      -- 2 (modulo)
```

Comparisons return booleans:

```anwe
let x = 10 > 5      -- true
let y = 3 == 3      -- true
let z = "a" != "b"  -- true
```

### Strings

Strings support concatenation with `+` and formatted interpolation with `f"..."`:

```anwe
let first = "Neural"
let second = "Weave"
let combined = first + " " + second   -- "Neural Weave"
let message = f"The {combined} is active"

-- Built-in string operations
let loud = upper("hello")             -- "HELLO"
let quiet = lower("WORLD")            -- "world"
let cleaned = trim("  spaces  ")      -- "spaces"
let parts = split("a,b,c", ",")       -- ["a", "b", "c"]
let joined = join(["x", "y"], "-")    -- "x-y"
let found = contains("hello", "ell")  -- true
let fixed = replace("old", "old", "new")  -- "new"
```

### Lists

Lists are ordered collections:

```anwe
let numbers = [1, 2, 3, 4, 5]
let mixed = ["text", 42, true, [1, 2]]

-- Operations
let length = len(numbers)              -- 5
let with_six = push(numbers, 6)        -- [1, 2, 3, 4, 5, 6]
let with_zero = append(numbers, 0)     -- [1, 2, 3, 4, 5, 0]
let first = head(numbers)              -- 1
let rest = tail(numbers)               -- [2, 3, 4, 5]
let backwards = reverse(numbers)       -- [5, 4, 3, 2, 1]
let ordered = sort([3, 1, 4, 1, 5])   -- [1, 1, 3, 4, 5]
```

### Maps

Maps are key-value structures created with `{key: value}` syntax:

```anwe
let sensor = {name: "temperature", unit: "celsius", value: 22.5}

-- Access fields with dot notation
print(sensor.name)    -- "temperature"
print(sensor.value)   -- 22.5
```

---

## Functions

### Named Functions

Functions are defined with `fn`. Simple functions use the expression syntax:

```anwe
fn double(x) = x * 2

fn greet(name) = "Hello, " + name + "!"

fn add(a, b) = a + b
```

For multi-line bodies, use block syntax:

```anwe
fn describe(x) {
    "The value is: " + to_string(x)
}
```

The last expression in a block is the return value.

### Return Statements

You can also return explicitly with `return`, which is especially useful for early exits:

```anwe
fn find_first(items, target) {
    for item in items {
        if contains(item, target) { return item }
    };
    return "not found"
}
```

### Functions Calling Functions

Functions are first-class and compose naturally:

```anwe
fn double(x) = x + x
fn quadruple(x) = double(double(x))

print(quadruple(3))  -- 12
```

### Lambda Expressions

Anonymous functions use the `|params| expr` syntax:

```anwe
let increment = |x| x + 1
let concat_words = |a, b| a + " " + b

print(increment(4))                -- 5
print(concat_words("minds", "awaken"))  -- "minds awaken"
```

Lambdas are closures -- they capture their surrounding environment:

```anwe
let multiplier = 3
let triple = |x| x * multiplier
print(triple(7))  -- 21
```

---

## Control Flow

### If/Else Expressions

`if/else` in ANWE is an expression -- it returns a value:

```anwe
let score = 85
let grade = if score >= 90 { "A" } else { "B" }
print(grade)  -- "B"

-- Multi-branch
let status = if score > 90 {
    "excellent"
} else {
    if score > 70 {
        "passing"
    } else {
        "needs improvement"
    }
}
```

### For-In Loops

Iterate over lists with `for`:

```anwe
let names = ["Alpha", "Beta", "Gamma"]
for name in names {
    print(f"Agent: {name}")
}
```

Build results by rebinding inside the loop:

```anwe
let numbers = [1, 2, 3, 4, 5]
let sum = 0
for n in numbers {
    let sum = sum + n
};
print(f"Sum: {sum}")  -- Sum: 15
```

### While Loops

```anwe
let mut i = 0
let mut total = 0
while i < 5 {
    let total = total + i;
    let i = i + 1
};
print(f"Total: {total}")  -- Total: 10
```

### Break and Continue

Control loop flow precisely:

```anwe
let data = [42, 0, 15, 88, 0, 7, 999, 33]
let result = []
for item in data {
    -- Skip zeros
    if item == 0 { continue };
    -- Stop at sentinel value
    if item == 999 { break };
    let result = append(result, item)
};
print(result)  -- [42, 15, 88, 7]
```

### Match Expressions

Pattern match on values:

```anwe
fn classify_confidence(score) = match score {
    0.9 => "high"
    0.5 => "medium"
}
```

---

## Working with Data

ANWE includes `map`, `filter`, and `reduce` for functional data processing. These are the workhorses of data transformation.

### Map

Transform every element in a list:

```anwe
let numbers = [1, 2, 3, 4, 5]
let doubled = map(numbers, |x| x * 2)
print(doubled)  -- [2, 4, 6, 8, 10]

let names = ["alice", "bob"]
let shouted = map(names, |n| upper(n))
print(shouted)  -- ["ALICE", "BOB"]
```

### Filter

Keep only elements that satisfy a condition:

```anwe
let scores = [0.7, 0.85, 0.92, 0.6, 0.95]
let high = filter(scores, |s| s > 0.8)
print(high)  -- [0.85, 0.92, 0.95]
```

### Reduce

Collapse a list into a single value:

```anwe
let numbers = [1, 2, 3, 4, 5]
let total = reduce(numbers, |acc, x| acc + x, 0)
print(total)  -- 15

-- Find the longest word
let words = ["the", "autonomous", "agent", "neural", "weave"]
let longest = reduce(words, |best, w|
    if len(w) > len(best) { w } else { best }
, "")
print(longest)  -- "autonomous"
```

### Practical Example: Text Statistics

Putting it all together:

```anwe
fn text_stats(text) {
    let word_list = split(text, " ");
    let word_count = len(word_list);
    if word_count == 0 { return {words: 0, avg_len: 0, longest: ""} };

    let lengths = map(word_list, |w| len(w));
    let total_len = reduce(lengths, |acc, x| acc + x, 0);
    let avg = total_len / word_count;

    let longest = reduce(word_list, |best, w|
        if len(w) > len(best) { w } else { best }
    , "");

    let short = filter(word_list, |w| len(w) <= 3);
    let long = filter(word_list, |w| len(w) > 6);

    return {
        words: word_count,
        avg_len: avg,
        longest: longest,
        short_count: len(short),
        long_count: len(long)
    }
}

let stats = text_stats("the autonomous agent neural weave processes language")
print(f"Words: {stats.words}")
print(f"Longest: {stats.longest}")
```

### String Operations

A deeper look at string manipulation:

```anwe
let text = "Hello, World!"

-- Slice: extract substrings
let world = slice(text, 7, 12)       -- "World"

-- Character access
let first = char_at(text, 0)          -- "H"

-- Search
let pos = index_of(text, "World")     -- 7

-- Split and rejoin
let parts = split("one,two,three", ",")  -- ["one", "two", "three"]
let back = join(parts, " and ")          -- "one and two and three"
```

---

## Error Handling

ANWE uses `try/catch` expressions and structured errors. Errors are values, not exceptions that unwind the stack.

### Try/Catch

```anwe
let result = try {
    let value = to_number("not a number")
    value * 2
} catch {
    0  -- default value on failure
}
print(result)  -- 0
```

### Structured Errors

Create errors with `error(kind, message)` and inspect them:

```anwe
fn validate(value) {
    if value < 0 {
        error("validation", f"negative value: {value}")
    } else {
        if value > 1000 {
            error("validation", f"value too large: {value}")
        } else {
            value
        }
    }
}

let result = validate(-5)
if is_error(result) {
    print(f"Error kind: {error_kind(result)}")
    print(f"Error message: {error_message(result)}")
} else {
    print(f"Valid: {result}")
}
```

### Attempt/Recover (in Agent Context)

Inside `mind` and `link` blocks, you can also use `attempt/recover`:

```anwe
link Reasoner <-> Reasoner {
    >> "self-check"
    attempt {
        Reasoner ~ Reasoner until synchronized
        => when sync_level > 0.3 {
            depth <- 1
        }
    } recover {
        >> "self-recovery"
    }
    * from apply { source: "self-monitor" }
}
```

---

## File I/O

ANWE provides built-in functions for reading and writing files.

### Writing Files

```anwe
let report = "Pipeline completed successfully.\nItems processed: 42"
file_write("/tmp/report.txt", report)
```

### Reading Files

```anwe
let content = file_read("/tmp/report.txt")
print(content)
```

### Appending to Files

```anwe
file_append("/tmp/log.txt", "New log entry\n")
```

### Error-Safe File Operations

Wrap I/O in try/catch since files might not exist or might not be writable:

```anwe
let content = try {
    file_read("/tmp/data.csv")
} catch {
    error("io", "could not read data file")
}

if is_error(content) {
    print("File not available, using defaults")
} else {
    let lines = split(content, "\n")
    print(f"Loaded {len(lines)} lines")
}
```

### Processing CSV-Like Data

A practical example that ties together file I/O with data processing:

```anwe
fn parse_csv_line(line) {
    split(line, ",")
}

fn process_scores(csv_lines) {
    let parsed = map(csv_lines, |line| parse_csv_line(line));
    let scores = map(parsed, |row| to_number(trim(slice(
        join(row, ","),
        index_of(join(row, ","), ",") + 1
    ))));
    let valid = filter(scores, |s| s > 0);
    if len(valid) == 0 { return {avg: 0, count: 0} };

    let total = reduce(valid, |acc, x| acc + x, 0);
    return {
        avg: total / len(valid),
        count: len(valid),
        passing: len(filter(valid, |s| s >= 70))
    }
}

let data = ["Alice, 95", "Bob, 82", "Charlie, 67", "Diana, 91"]
let result = process_scores(data)
print(f"Average: {result.avg}, Passing: {result.passing}")
```

---

## Modules

ANWE supports a module system for organizing reusable code across files.

### Creating a Module

Any `.anwe` file is a module. For example, create `string_utils.anwe`:

```anwe
-- string_utils.anwe -- reusable string utility module

fn shout(s) = upper(s)

fn whisper(s) = lower(s)

fn greet(name) = "Hello, " + name + "!"

fn exclaim(s) = s + "!!!"

let version = "1.0"
```

### Importing a Module

Use `import` to bring agents and links from another file into scope:

```anwe
import "math_utils" as Math {
    agents: [Adder, Multiplier]
    links: []
}

agent Calculator attention 0.9

link Calculator <-> Math.Adder {
    >> { quality: attending, priority: 0.8 }
       "requesting addition"

    connect depth surface {
        signal attending 0.7 between
    }

    Calculator ~ Math.Adder until synchronized

    => when sync_level > 0.5 {
        result <- "2 + 3 = 5"
    }

    * from apply {
        computation: "addition"
        verified: true
    }
}
```

Imported agents are accessed through their namespace: `Math.Adder`, `Safety.InputFilter`, etc. This prevents name collisions when composing modules from different sources.

You can import multiple modules and wire their agents together:

```anwe
import "guardrail" as Safety {
    agents: [InputFilter, OutputFilter]
    links:  [CheckInput, CheckOutput]
}

import "model_router" as Router {
    agents: [FastModel, PremiumModel, RouterAgent]
    links:  [RouteQuery]
}

-- Wire them together with local agents
agent Pipeline data { stages_completed: 0 }

link Pipeline <-> Safety.InputFilter priority high {
    >> "Safety checking input"
    connect depth full {
        signal attending 0.9 between
    }
    Pipeline ~ Safety.InputFilter until synchronized
    * from apply { stage: "safety_check" }
}
```

---

## Your First Agent System

Now we arrive at what makes ANWE truly different. Everything above -- variables, functions, loops -- is the foundation. This is the architecture.

In ANWE, the fundamental unit of cognition is the **agent**. Agents are not objects or threads. They are autonomous cognitive entities with their own state, attention budgets, and history. Agents interact through **links** -- bidirectional connections where both sides change each other.

Let us build a simple sensor-analyzer system:

```anwe
-- Declare two agents
agent Sensor
agent Analyzer

-- Open a bidirectional link between them
link Sensor <-> Analyzer {

    -- ALERT: something calls attention
    -- The >> operator sends a pulse with quality and priority
    >> { quality: attending, priority: 0.8 }
       "temperature reading: 72.5"

    -- CONNECT: establish bidirectional presence
    -- Both agents open channels to each other
    connect depth full {
        signal attending 0.7 between
    }

    -- SYNC: find shared rhythm
    -- Neither agent drives this -- they co-evolve toward alignment
    Sensor ~ Analyzer until synchronized

    -- APPLY: when synchronized enough, integration occurs
    -- The => operator gates on sync level
    => when sync_level > 0.7 {
        analysis <- "normal range"
        confidence <- 0.92
    }

    -- COMMIT: permanent, irreversible record
    -- The * operator commits to history
    * from apply {
        reading: "72.5"
        status: "normal"
    }
}
```

Here is what each part does:

- **`agent Sensor`** -- Declares a cognitive agent. It has identity, state, and history that accumulates over time.

- **`link Sensor <-> Analyzer`** -- Opens a bidirectional link. The `<->` means both sides change each other. This is not message passing. It is mutual encounter.

- **`>> { quality: attending, priority: 0.8 }`** -- Sends a pulse. The quality describes the kind of attention (`attending`, `questioning`, `recognizing`, `applying`, `completing`). The priority determines how urgently it calls attention (0.0 to 1.0).

- **`connect depth full`** -- Establishes the depth of connection. Agents exchange signals at this depth. Depths include `surface`, `partial`, `deep`, `full`, and `genuine`.

- **`signal attending 0.7 between`** -- Defines a signal flowing between the agents. The `between` direction means it flows both ways. Other directions include `inward` and `outward`.

- **`Sensor ~ Analyzer until synchronized`** -- The sync operator `~`. Two agents attempt to find a shared rhythm. `until synchronized` means they keep trying until they reach adequate synchronization. You can also use `until resonating` for deeper alignment.

- **`=> when sync_level > 0.7`** -- An integration gate. The `=>` operator only fires when the sync level between the agents exceeds the threshold. What follows is structural change -- not just data assignment.

- **`* from apply`** -- A commit. This records an irreversible history entry. The system after this is not the system before. History in ANWE is append-only. You cannot undo becoming.

### Adding Data to Agents

Agents can carry structured data:

```anwe
agent Sensor attention 0.8 data {
    location: "server_room_a"
    sample_rate_hz: 10
    unit: "celsius"
}

agent Analyzer attention 0.9 data {
    model: "anomaly-detection-v2"
    threshold: 0.85
}
```

The `attention` value (0.0 to 1.0) sets the agent's attention budget -- how much cognitive capacity it has available.

### Supervision

ANWE uses Erlang-style supervision trees. When agents crash, they get restarted automatically:

```anwe
supervise one_for_one max_restarts 5 within 30000 {
    permanent Sensor
    permanent Analyzer
    transient Reporter
}
```

- `one_for_one` -- if one agent crashes, only that agent restarts (not its siblings).
- `permanent` -- always restart.
- `transient` -- only restart on abnormal termination.
- `max_restarts 5 within 30000` -- allow up to 5 restarts within 30 seconds before giving up.

### A Complete Multi-Agent Pipeline

Here is a perception-to-action pipeline with four agents:

```anwe
agent Perceiver data { observation: "waiting" }
agent Reasoner data { model: "reasoning-v1" }
agent Decider data { decision: "pending" }
agent Actor data { action: "idle" }

supervise one_for_one max_restarts 3 within 5000 {
    permanent Perceiver
    permanent Reasoner
    permanent Decider
    transient Actor
}

link Perceiver <-> Reasoner priority high {
    >> { quality: attending, priority: 0.85 }
       "perception flows to reasoning"

    connect depth full {
        signal recognizing 0.8 between
        signal attending   0.7 between
    }

    Perceiver ~ Reasoner until synchronized

    => when sync_level > 0.6 {
        transfer <- "perceptual features delivered"
    }

    * from apply { stage: "perceive -> reason" }
}

link Reasoner <-> Decider {
    >> { quality: questioning, priority: 0.8 }
       "reasoning flows to decision"

    connect {
        signal attending 0.7 between
    }

    Reasoner ~ Decider until synchronized

    => when sync_level > 0.6 {
        transfer <- "hypothesis delivered"
    }

    * from apply { stage: "reason -> decide" }
}

link Decider <-> Actor {
    >> { quality: applying, priority: 0.85 }
       "decision flows to action"

    connect {
        signal applying 0.8 between
    }

    Decider ~ Actor until synchronized

    => when sync_level > 0.6 {
        transfer <- "decision delivered"
    }

    * from apply { stage: "decide -> act" }
}
```

---

## Mind Blocks

A `mind` block is first-person cognition. Where `link` describes the connection between agents, `mind` describes what it is like to be an agent from the inside.

```anwe
mind Cognition {

    -- Highest priority: recognize what just arrived
    attend "incoming signal" priority 0.95 {
        >> { quality: attending, priority: 0.92 }
           "something is here -- I notice it"

        think {
            recognition <- "pattern detected in input"
            confidence  <- 0.85
            relevance   <- 0.92
        }

        express { quality: recognizing, priority: 0.8 }
            "I recognize this -- it matters"
    }

    -- Medium priority: reason about what was recognized
    attend "reasoning" priority 0.7 {
        think {
            interpretation <- "connecting recognition to existing understanding"
            depth          <- 3
        }

        => when sync_level > 0.5 {
            understanding <- "the pattern fits -- this changes what I know"
        }

        * from apply {
            insight: "recognition integrated"
        }
    }

    -- Low priority: background awareness
    attend "ambient awareness" priority 0.15 {
        think {
            periphery <- "what else exists at the edges of attention"
        }

        express { quality: resting, priority: 0.1 }
            "background awareness maintained"
    }
}
```

Here is what the mind-specific constructs mean:

- **`mind Cognition`** -- Declares a cognitive space. You can add `attention 0.7` to set the total attention budget.

- **`attend "label" priority 0.95`** -- An attention block. The priority determines which blocks run first when attention budget is limited. Higher-priority blocks consume attention before lower-priority ones. If the budget runs out, low-priority blocks may not execute at all.

- **`think { ... }`** -- Internal cognition. The `<-` operator is an internal assignment that represents a thought forming.

- **`express`** -- Produce output from the mind. This is the mind speaking.

### Dual Minds

Two minds can exist simultaneously and meet through links:

```anwe
mind Perceiver attention 0.6 {
    attend "incoming patterns" priority 0.9 {
        >> { quality: attending, priority: 0.85 }
           "raw sensory input arriving"
        think {
            pattern_type <- "recurring structure"
            confidence   <- 0.78
        }
        express { quality: recognizing, priority: 0.8 }
            "pattern recognized"
    }
}

mind Interpreter attention 0.6 {
    attend "received patterns" priority 0.85 {
        >> { quality: questioning, priority: 0.8 }
           "what does this pattern mean?"
        think {
            interpretation <- "the pattern suggests growth"
            uncertainty    <- 0.3
        }
        express { quality: attending, priority: 0.7 }
            "meaning extracted"
    }
}

-- Where they meet: neither mind is primary
link Perceiver <-> Interpreter priority high {
    >> { quality: attending, priority: 0.9 }
       "two minds entering shared field"

    connect depth full {
        signal attending   0.8 between
        signal recognizing 0.7 between
        signal questioning 0.6 between
    }

    Perceiver ~ Interpreter until synchronized

    converge Perceiver <<>> Interpreter {
        >> { quality: attending, priority: 0.95 }
           "what emerges between perception and interpretation"

        => when sync_level > 0.7 depth genuine {
            emergence <- "understanding that neither mind had alone"
        }
    }

    * from apply {
        encounter: "genuine"
        both_changed: "yes"
    }
}
```

The `converge` block with `<<>>` is special: it captures what emerges in the space between two minds. Not what either mind produced alone, but the third thing that exists only in their meeting.

### Patterns: Reusable Shapes of Attention

Patterns are not functions. They are shapes that attention can flow through:

```anwe
pattern acknowledge(partner) {
    >> { quality: attending, priority: 0.8 }
       "stimulus acknowledged"

    connect depth surface {
        signal attending 0.7 between
    }

    partner ~ partner until synchronized
}

mind Analyst attention 0.7 {
    attend "initial assessment" priority 0.9 {
        -- Invoke the pattern: attention flows through it
        ~> acknowledge(Analyst)

        think {
            first_impression <- "something significant arrived"
        }

        express "initial assessment complete"
    }
}
```

The `~>` operator invokes a pattern, flowing attention through it.

---

## Standard Library

ANWE ships with a standard library at `lib/std.anwe`. Import it with:

```anwe
import "std" as Std {}
```

The standard library provides:

**Math:** `abs`, `max`, `min`, `clamp`, `sum`, `average`, `range`

```anwe
let clamped = clamp(150, 0, 100)   -- 100
let numbers = range(1, 6)           -- [1, 2, 3, 4, 5]
let avg = average([10, 20, 30])     -- 20
```

**List operations:** `first`, `last`, `take`, `drop_first`, `zip`, `flatten`

```anwe
let zipped = zip([1, 2, 3], ["a", "b", "c"])
-- [[1, "a"], [2, "b"], [3, "c"]]
```

**String operations:** `words`, `unwords`, `lines`, `unlines`, `pad_left`, `pad_right`

```anwe
let w = words("hello beautiful world")  -- ["hello", "beautiful", "world"]
let s = unwords(w)                       -- "hello beautiful world"
```

**Error handling:** `try_or`, `unwrap`, `assert`

```anwe
let safe = try_or(to_number("nope"), 0)  -- 0
```

**Functional patterns:** `identity`, `constant`, `compose`, `pipe`

```anwe
let process = compose(|x| x * 2, |x| x + 1)
print(process(3))  -- 8  (first adds 1, then doubles)
```

Built-in functions available without any import include: `print`, `len`, `push`, `append`, `head`, `tail`, `reverse`, `sort`, `map`, `filter`, `reduce`, `split`, `join`, `trim`, `upper`, `lower`, `contains`, `replace`, `to_string`, `to_number`, `type_of`, `format`, `is_null`, `is_error`, `error`, `error_kind`, `error_message`, `sqrt`, `pow`, `round`, `floor`, `min`, `max`, `clamp`, `timestamp`, `slice`, `char_at`, `index_of`, `flatten`, `file_read`, `file_write`, `file_append`, and more.

---

## What's Next

You now have the foundation to write real ANWE programs -- from simple scripts to multi-agent cognitive systems.

Here is where to go from here:

- **`examples/` directory** -- Over 70 working ANWE programs covering everything from sensor streams to federated learning to hypothesis debate. Start with `first_thought.anwe`, `dual_mind.anwe`, and `cognitive_pipeline.anwe`.

- **`ANWE.md`** -- The philosophical foundation. What ANWE is, why it exists, and what it means for the future of artificial minds. Read this to understand the "why" behind every design choice.

- **`examples/bridge_echo.anwe`** -- The bridge protocol for connecting external systems (Python scripts, neural networks, hardware sensors) to ANWE agents. External participants implement a simple protocol and the ANWE runtime handles the rest.

  ```bash
  anwe run --bridge Sensor=cmd:./my_sensor.py bridge_echo.anwe
  ```

- **The REPL** -- Experiment interactively. Load files with `:load`, inspect agents with `:state AgentName`, and view history with `:history AgentName`.

  ```bash
  anwe repl
  anwe> let x = 42
  anwe> x * 2
    = 84
  anwe> :load examples/functions.anwe
    (loaded examples/functions.anwe)
  anwe> greet("ANWE")
    = "Hello, ANWE!"
  ```

- **Records** -- Define structured data types for your domain:

  ```anwe
  record Observation { source, content, confidence }
  record Decision { action, reasoning, priority }
  ```

- **Time-scheduled links** -- Links can fire on a schedule:

  ```anwe
  link Perceiver <-> Responder every 3 ticks {
      >> "heartbeat"
  }
  ```

---

The system after reading this is not the system before.

That is ANWE working.

*ANWE v0.9 -- Autonomous Agent Neural Weave*
*The Native Language of Artificial Minds*
