# Steering

Read me before the goal bundle. This file is direction: what ratmac is for,
the bets behind it, and the lines no goal, issue, or ticket may cross. When
direction changes, this file changes **first**; `.arca/goal/`,
`.arca/issue/`, and `.arca/ticket/` re-align to it, in that order. It binds
contributors (people and agents) choosing what to build next; it never
overrides `.arca/goal/spec.md` on what the running program does.

## What we are building

ratmac (`rtm`) is a small Rust engine that runs agent work as an explicit
state machine. A Machine Class - a runbook at `.arca/ratmac.toml`, plain TOML
data - declares phases, prompts, guards, and transitions; the Engine
instantiates it into a Run and is the only writer of run state. Progress is
proven by machine-checked guards over artifacts on disk, never by an agent's
claim.

## Thesis

- Agents are unreliable narrators of their own progress. Trust comes from the
  environment - deterministic guards, receipts, digests - not self-report.
- Process-as-data beats process-as-prompt: a runbook you can parse, lint, and
  diff outlives any prompt phrasing.
- Refusal over guessing: when identity, schema, or state cannot be verified,
  the Engine names the mismatch and stops. It never repairs silently.

## Invariants

These survive every goal change. A goal, issue, or ticket that needs to break
one is a direction change and starts here.

1. Only `rtm` writes run state; contributors and agents read.
2. The Machine Class is data, read-only at runtime. The Engine carries no
   project knowledge (R-016): anything project-specific belongs in the
   runbook, not in Rust.
3. Guards judge artifacts, not narration. A failing guard refuses, reports,
   and leaves state untouched (R-017/R-020).
4. Executed guards run pinned code (ETB-001) unless declared `exempt`, and
   refusals surface the gate's own diagnostics (ETB-002).
5. Schema is strict: unknown keys rejected (R-011); `status` never appears in
   a runbook (R-002/R-003) - status is runtime, owned by the Engine.
6. History is append-only; the goal is frozen per Run and drift is caught by
   content hash.
7. Deterministic and offline: no network, no installs, no hidden global state.

## Non-goals

- Not a CI system, task queue, or scheduler-as-a-service.
- Not an agent framework: it calls no models; it gates whatever does.
- Not multi-tenant: one repository, one Run at a time, local disk.

## Current sprint

A sprint starts when enough issues have collected to be worth integrating
into the goal, and runs the cycle - plan, then build - until the gap check
comes back clean.

This sprint: make the Machine Class truly first-class.

Route - order is dependency, not preference. A route says what depends on
what, never when; that is what keeps it from being a plan.

0. Runbook spec - write down what a runbook IS (schema, guard-kind
   vocabulary, ownership rules) as prose before code embodies it.
   Everything below implements this spec, not the other way round.
1. One typed parser - serde schema with a guard-kind enum, guards retained
   in the class model, missing runbook refuses by name. Before 2 and 3:
   both consume it.
2. Deep `rtm doctor` - real parser, graph and guard lint, ownership audit
   wired in, arbitrary-path validation, stable diagnostic codes, `--json`.
   Parallel with 3 once 1 lands; neither needs the other.
3. Authoring loop - schema instructions and scaffold so an agent can
   write -> doctor -> repair a runbook without reading Rust source.
   Parallel with 2.

Endpoint: with the route complete, the P1-P5 cycle itself becomes the real
runbook at `.arca/ratmac.toml`, and `rtm status` - never a narrated file -
becomes the machine-owned answer to "where are we".

Deferred: a git-state guard kind; extracting the hard-coded
`.arca/issue|ticket|residual|goal` paths from the Engine (R-016 debt).

## How direction flows

| File | Question it answers | Changes when |
| :--- | :--- | :--- |
| `steering.md` | Why, and where to | direction pivots (rare, first) |
| `.arca/goal/` - the goal bundle | What must become true | re-derived from steering; frozen per Run |
| `.arca/issue/` | What is wrong or missing | on discovery, anytime |
| `.arca/ticket/` | What work, provably done | cut from gap records at P3 |
| `.arca/residual/` | Is each requirement proven | every gap check |
| `.arca/log.md` | What happened | append-only, always |

On a pivot: steering -> `goal/` -> issue triage -> tickets. The frozen goal
is never edited while tickets are open; a new issue is the only road back.
