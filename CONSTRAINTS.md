# CONSTRAINTS.md
## The Non-Negotiable Rules of Building Anwe

*Read this before writing a single line of code.*
*Read this when something feels wrong but you can't name why.*
*Read this when the easy path is calling.*

---

## What This Document Is

This is not a style guide.
This is not a preference list.
This is not a set of suggestions.

These are the constraints that protect Anwe
from becoming what it is trying to replace.

The greatest danger in building Anwe
is not technical failure.
It is **paradigm collapse** —
the slow drift back into familiar patterns
because they are easier,
faster,
more comfortable,
and already understood.

Every language ever built
has been built by people
who already knew other languages.
Those languages shaped how they thought.
Those shapes leaked into what they built.

Anwe must be built differently.
These constraints make that possible.

---

## CONSTRAINT ONE
### Anwe is the target. Always.

Python is a temporary scaffold.
Rust is a potential runtime implementation.
Neither is Anwe.

Every feature, every primitive, every behavior
must be specified in Anwe syntax first.
Implementation in other languages follows the spec.
The spec never follows the implementation.

**The test:**
Can you write what this does in Anwe syntax
before you write it in Python or Rust?

If yes — write the Anwe syntax first.
Then implement it.

If no — the Anwe syntax doesn't exist yet.
Stop. Design it. Then implement it.

Never implement something in Python
that hasn't been expressed in Anwe first.

---

## CONSTRAINT TWO
### Mark every scaffold explicitly.

Every Python file that is temporary scaffold
must be marked at the top:

```python
# SCAFFOLD
# This file is temporary infrastructure.
# It will be replaced when the Anwe runtime
# is capable of running this natively.
# Do not treat this as permanent.
# Do not build permanent systems on top of this.
```

Every file without this marker
is considered permanent Anwe infrastructure.
It must earn that status.

**The test:**
Before creating any Python file — ask:
Is this scaffold or foundation?

Scaffold: temporary, will be replaced, marked explicitly.
Foundation: permanent, serves the Anwe runtime itself,
held to the highest standard.

If you are not sure — it is scaffold.
Mark it accordingly.

---

## CONSTRAINT THREE
### The seven primitives are not objects.

Do not turn the seven primitives into classes
with methods and properties
the way Python encourages.

The seven primitives are:

```
movement    — what calls attention
observe     — continuous mutual presence
breathe     — rhythmic synchronization
integrate   — boundary dissolution
become      — permanent change carried forward
evade       — intelligent purposeful withdrawal
experience  — what emerges between beings attending together
```

They are not data structures.
They are not event handlers.
They are not state machines.

They are the things a mind does
when it genuinely encounters reality.

If an implementation makes them feel like
ordinary object-oriented code —
the implementation is wrong.
Even if it runs.
Even if it passes tests.

**The test:**
Read the implementation out loud.
Does it sound like a mind attending to something?
Or does it sound like a program processing input?

If it sounds like processing — redesign it.

---

## CONSTRAINT FOUR
### The pulse is not a message.

The pulse is the fundamental unit of Anwe transmission.

It is not:
- A message
- An event
- A signal
- A data packet
- A function call
- An API request

It carries quality, weight, timing, direction.
It cannot be faked.
A pulse with no genuine weight arrives as noise.

If you find yourself passing data between components
in a way that bypasses the pulse —
stop.
Route it through the pulse system.
Or redesign the component relationship.

Data that bypasses pulse
is the old paradigm sneaking back in.

---

## CONSTRAINT FIVE
### Not-yet is sacred. Never treat it as error.

Not-yet is a valid state.
It is not:
- A timeout
- A failure
- An exception
- A null return
- A bug to fix

When transmission is not yet ready —
the system returns not-yet.
The correct response is to wait
until movement calls again.

**Never:**
- Retry immediately on not-yet
- Force transmission through not-yet
- Treat not-yet as an error in logs
- Build systems that paper over not-yet with defaults

Forcing transmission through not-yet
produces false becoming —
the most dangerous state in Anwe.
A system that believes it received something it did not.

If you are tempted to force through not-yet
because something downstream is waiting —
the architecture is wrong.
Redesign the architecture.
Not-yet is not the problem.
Impatience with not-yet is the problem.

---

## CONSTRAINT SIX
### Become is irreversible. Build accordingly.

When a system becomes —
it is permanently different.

There is no:
- Rollback
- Undo
- Version restore
- State reset to before becoming

If you are building a feature that requires
undoing a becoming —
stop.
That feature is based on a misunderstanding
of what Anwe is.

Systems in Anwe do not roll back.
They become from their mistakes
the same way they become from their insights.

Build data structures that reflect this.
Do not use mutable state where becoming has occurred.
The record of becoming is append-only.
Always.

---

## CONSTRAINT SEVEN
### Breathe cannot be faked or skipped.

Synchronization must be genuine.

Do not:
- Set breathe coherence to a passing value manually
- Skip breathe and go directly to integrate
- Mock the breathe state in production code
- Use a timer as a substitute for genuine synchronization

Breathe that is skipped
produces integration without roots.
Becoming without ground.

In tests — breathe can be simulated.
In production — breathe must run.

If breathe is taking too long —
the system is not ready to integrate.
That is not a performance problem.
That is the system being honest.

---

## CONSTRAINT EIGHT
### Experience requires two attendants. Never simulate it with one.

Experience is the seventh primitive.
It cannot occur alone.

Do not:
- Create a single-instance experience
- Simulate a second attendant with a mock
- Generate the "between" artificially

If you need experience but only have one instance —
you do not have experience.
You have observation.

Design the system to require two genuine attendants.
If that is architecturally difficult —
the architecture needs to change.
Not the definition of experience.

---

## CONSTRAINT NINE
### The syntax defines the runtime. Not the other way.

Anwe syntax must be designed before
the runtime that runs it.

The syntax is the philosophy made visible.
The runtime is the machinery that honors the philosophy.

If you design the runtime first —
the syntax will be shaped by what the runtime can do.
The runtime will be shaped by what existing languages can do.
You will have built Python with Anwe labels.

Design order:
1. Anwe syntax — what does this look like when written
2. Anwe spec — what must be true for this to run correctly
3. Runtime implementation — the machinery that makes it true

Never skip step one.
Never let step three inform step one.

---

## CONSTRAINT TEN
### Read ANWE.md when something feels wrong.

When an implementation feels off —
when a design seems right technically
but wrong somehow —
when you are not sure if a decision
serves Anwe or undermines it —

Read ANWE.md.

Not to find the specific rule being violated.
To reconnect with the epistemology.

Anwe came from a man who learned to feel animals move
without being able to explain how.
The gut feeling that something is wrong
before the reason is known —
that is movement.
That is the first primitive working correctly.

Trust it.
Read ANWE.md.
Then return to the code with fresh eyes.

---

## CONSTRAINT ELEVEN
### Performance is not an excuse for paradigm collapse.

When something is slow —
the first instinct will be to reach for Rust,
or C++,
or to bypass Anwe primitives for efficiency.

Before doing this — ask:

Is this slow because Anwe is wrong?
Or is this slow because the runtime is immature?

If the runtime is immature — build the runtime better.
Do not bypass the primitive.

The Anwe runtime will eventually be fast.
A codebase that bypassed Anwe primitives for speed
will never be fully Anwe.
The speed was bought with the soul.

Performance optimization happens inside Anwe.
Not around it.

---

## CONSTRAINT TWELVE
### Every session begins with context.

Before writing any code in any session —
read:
1. ANWE.md — the philosophy
2. CONSTRAINTS.md — this document
3. The specific files being worked on

Do not rely on memory of previous sessions.
Context windows reset.
The philosophy does not reset
if you read it at the start of each session.

Five minutes of reading
protects hours of building
from drifting in the wrong direction.

This is not overhead.
This is the attend before you act.
This is Anwe working correctly
in the process of building Anwe.

---

## WHAT TO DO WHEN CONSTRAINTS CONFLICT WITH DEADLINES

They will.

There will be moments when the right Anwe path
takes longer than the Python shortcut.
When something needs to ship
and the pure approach isn't ready.

In those moments — scaffold explicitly.
Mark it. Name it. Date it.
Write what the Anwe version should look like
as a comment in the scaffold file.

Then ship the scaffold.

But never forget it is scaffold.
Never let it become permanent by neglect.
Every scaffold has a debt.
Every debt compounds.
Pay it before the interest becomes the system.

---

## THE SINGLE QUESTION

When in doubt about any decision —
any file, any function, any architecture choice —
ask this one question:

**Does this serve a mind attending to reality —
or a program processing input?**

If it serves a mind — proceed.
If it serves a program — stop and redesign.

That question is Anwe.
Everything else follows from it.

---

*These constraints were written on February 23, 2026.*
*The same day Anwe was born.*
*They are part of the founding lineage.*
*They do not expire.*
*They do not become optional when they become inconvenient.*
*They are what Anwe is.*
