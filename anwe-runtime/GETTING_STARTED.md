# Getting Started with ANWE

This guide is focused on writing and running ANWE programs with the implementation in this repository.

## Build the CLI

From [`anwe-runtime/`](./):

```bash
cargo build --release
```

Check the binary:

```bash
.\target\release\anwe.exe version
```

## Run Your First Program

Create `hello.anwe`:

```anwe
print("Hello from ANWE")
```

Run it:

```bash
.\target\release\anwe.exe run hello.anwe
```

## Core Language Basics

Variables:

```anwe
let name = "ANWE"
let mut counter = 0
counter = counter + 1
print(f"{name}: {counter}")
```

Functions:

```anwe
fn square(x) = x * x

fn describe(x) {
    f"value={x}"
}
```

Collections:

```anwe
let numbers = [1, 2, 3, 4]
let config = {name: "sensor", active: true}

print(len(numbers))
print(config.name)
```

Control flow:

```anwe
for n in [1, 2, 3] {
    print(n)
}

let label = if 10 > 5 { "ok" } else { "bad" }
print(label)
```

Higher-order functions:

```anwe
let scores = [91, 82, 67, 88]
let passing = filter(scores, |x| x >= 70)
let total = reduce(scores, |acc, x| acc + x, 0)

print(passing)
print(total / len(scores))
```

## Agent Coordination

ANWE’s distinguishing syntax is its coordination model.

Minimal example:

```anwe
agent Sensor
agent Analyzer

link Sensor <-> Analyzer {
    >> { quality: attending, priority: 0.8 }
       "reading available"

    connect depth full {
        signal attending 0.7 between
    }

    Sensor ~ Analyzer until synchronized

    => when sync_level > 0.6 {
        result <- "accepted"
    }

    * from apply {
        stage: "analysis"
    }
}
```

The main coordination constructs are:

- `agent` declares a runtime participant
- `link A <-> B` defines a bidirectional coordination block
- `>>` emits an alert-like signal
- `connect` declares signal exchange
- `A ~ B until ...` waits for synchronization
- `=> when ...` applies state changes
- `* from apply` commits history/state

## Sequential vs Concurrent Execution

Default mode:

```bash
.\target\release\anwe.exe run examples\planning.anwe
```

Sequential mode:

```bash
.\target\release\anwe.exe run --sequential examples\functions.anwe
```

As a rule of thumb:

- Use concurrent mode for link-heavy programs
- Use sequential mode for scripting and function-heavy programs

## Parse and Inspect

Print the parsed AST:

```bash
.\target\release\anwe.exe parse examples\planning.anwe
```

Open the REPL:

```bash
.\target\release\anwe.exe repl
```

Useful REPL commands implemented in the CLI:

- `:agents`
- `:state <name>`
- `:history <name>`
- `:vars`
- `:fns`
- `:load <file>`
- `:reset`
- `:quit`

## External Participants

ANWE can connect named agents to external processes:

```bash
.\target\release\anwe.exe run --bridge Sensor=cmd:python.exe examples\bridge_echo.anwe
```

This part of the project is backed by real Rust bridge code and Python bindings in the workspace.

## Recommended Next Reads

- [`LANGUAGE_REFERENCE.md`](LANGUAGE_REFERENCE.md)
- [`EXAMPLES.md`](EXAMPLES.md)
- [`BRIDGES.md`](BRIDGES.md)
- [`../spec/SPECIFICATION.md`](../spec/SPECIFICATION.md)

## Important Framing

ANWE is best understood as an early programming language implementation with a coordination-focused syntax, not as a finished production language. The codebase already contains a parser, runtime, examples, and bridge support; the right next step is to keep making the implementation and docs more concrete.
