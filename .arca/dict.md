# dict.md — Glossary

Plain-word definitions for terms used in work with Billy. Consult this file before coining a new term; reuse an entry if one fits. When a response must introduce a term not listed here, add a short entry. When a term is replaced, delete every mention of the old term outside log.md — no "retired" markers anywhere; the replacement is recorded once, in log.md.

## State machines

- **FSM (finite state machine)**: A system that is always in exactly one of a fixed set of states and moves between them only via defined transitions.
- **Runner**: The engine that executes an FSM — holds current state, receives events, applies transitions.
- **Deterministic**: Same state + same input always gives the same next state. No randomness, no ambiguity.
- **Transition**: A rule "from state A, on event E, go to state B."
- **Guard**: A condition on a transition; the transition fires only if the condition holds.
- **Entry/exit action**: Code that runs when a state is entered or left.
- **Hierarchical state machine (HSM)**: States can contain sub-states; a child inherits its parent's transitions.
- **Statechart (Harel statechart)**: FSM extended with hierarchy, parallel regions, and history. The academic superset of HSM.
- **Parallel states**: Two independent regions active at the same time.
- **History state**: Remembers which sub-state was active when the parent was left, to resume there.
- **Compile-time FSM**: States/transitions are code (types, macros); topology is frozen when the program builds. Cannot load a new graph from a file.
- **Data-driven (runtime-defined) FSM**: Topology loaded from a file (TOML/JSON/XML) at run time. What ratmac needs.
- **Typestate pattern**: Encoding state in the type system so invalid operations fail to compile.
- **SCXML**: W3C XML format that describes a statechart as a document, so it can be validated, diffed, and interpreted.
- **XState**: Popular JavaScript statechart library; the reference model for "machine as data."
- **Model checking**: Exhaustively exploring every state of a machine to prove properties (no deadlock, all states reachable). A verifier, not a runner.
- **Reachability / dead-end detection**: Static graph checks that every state can be reached and can reach an end state.

## Rust ecosystem

- **Crate**: A Rust package.
- **Proc-macro (procedural macro)**: Code that generates code at compile time; how most Rust FSM DSLs are built.
- **DSL (domain-specific language)**: A mini-language for one problem area.
- **serde**: The standard Rust serialization/deserialization framework.
- **petgraph**: The main Rust graph library (reachability, cycles, traversal).
- **statig / smlang / rust-fsm**: The mature compile-time FSM crates.
- **rustate**: XState-like crate; machines built at run time with a builder.
- **dd_statechart / scxml / statechart (crates)**: Data-driven statechart crates; small adoption.
- **Stateright**: Rust model checker for distributed systems. Not the same as Statewright.
- **no_std**: Rust without the standard library, for embedded targets.
- **Bus factor**: How many maintainers can vanish before a project dies. Low bus factor = risky dependency.

## ratmac / agent orchestration

- **Harness**: The agent product hosting the model (Claude Code, etc.).
- **Scaffold**: The fixed structure (prompts, phases, gates) wrapped around model calls.
- **Runbook**: The TOML file defining phases, per-phase prompts, and transitions for an agent run. Its schema also carries the validity rules — what a runbook may or may not contain (e.g. no lifecycle status as a graph dimension).
- **Phase**: One state in a runbook.
- **Artifact**: A file or output produced by real work (test log, build output, git commit).
- **Artifact guard / artifact-verified exit**: A phase can be left only when an artifact proves the work happened — read from process outputs (exit codes, compiler, git) the agent cannot write.
- **Sensitivity receipt**: Proof that a test can actually fail (seen red before green). Blocks fake or always-passing tests.
- **Advisory vs enforced**: Advisory = the agent is asked to follow rules (~70% compliance). Enforced = a mechanism blocks violations (hooks ~100%).
- **Hook / PreToolUse**: A program the harness runs before each tool call; it can deny the call. The "wall" that stops an agent from editing scheduler-owned files.
- **Guard gaming**: Satisfying the letter of a guard without doing the work.
- **Fabrication**: An agent claiming work is done when it is not.
- **Honest stall**: A run that stops and hands back to the human instead of faking progress. A feature, not a bug.
- **Statewright**: Rust competitor product (per-state tool allowlists, MCP-first, patented method). Not Stateright.
- **MCP (Model Context Protocol)**: Protocol for exposing tools/resources to agents over a connection; heavier than CLI calls.
- **SDD / Spec Kit**: Spec-driven development; markdown phase workflows with no enforcement.
- **Codemod**: Mechanical, rule-based code rewriting tool — for syntax-level migrations, unlike semantic ones.
- **Beachhead**: The first narrow market to win before expanding.
- **rtm doctor**: The ratmac checker command: parses a runbook and reports schema, graph, and guard problems before any run.
- **Guard lint**: A doctor check that every exit guard reads an agent-uncontrolled source (exit code, git state) — flags fake-able guards.
- **Property-based testing**: Testing by generating many random inputs and asserting a property always holds, instead of hand-picked cases.
- **Lean**: A theorem prover: you write mathematical statements and machine-checked proofs. Heavy; proves only properties you explicitly state.

## Shop process (.arca)

- **Sprint**: Starts when enough issues have collected to be worth integrating into the goal; runs the cycle — plan (P1–P3) then build (P4–P5) — until the gap check comes back clean. Issue-triggered, not time-boxed (unlike scrum's calendar sprint). Billy's term, 2026-07-27.
- **Route**: The ordered dependency list inside a sprint: what depends on what, one why per edge. Never dates or task breakdowns — that would be a plan. Lives in steering.md, Current sprint.
- **Stage derivation**: Answering "which stage are we in" by reading the tree with a fixed lookup rule (open tickets → building; unresolved gap records → P3; stale records → P2; pending issues → P1; else idle), never by storing it in a file. Stored status is narration, and narration drifts. Lives in index.md's front door.
- **Landing**: One committed change plus its log.md line — one commit = one landing = one log line. The smallest provable step. Inside a ticket, the red commit (tests exist, fail) and the green commit (all tests pass) are its two required landings. Lives in schema.md, "Units and git".
- **Program lane**: Changes to what the program does (`src/`, tests, the runbook). Must enter the loop: issue → goal → residual → ticket. No program commit without a ticket. Lives in schema.md, "Units and git".
- **Shop lane**: Changes to `.arca` docs (steering, schema, index, dict, tpl, vis). Lands directly — steering first on pivots — one log line per landing; issue creation is the stated exception. Lives in schema.md, "Units and git".
