# dict.md — Glossary

Plain-word definitions for terms used in work with Billy. Consult this file before coining a new term; reuse an entry if one fits. When a response must introduce a term not listed here, add a short entry. When a term is replaced, delete every mention of the old term outside log.md — no "retired" markers anywhere; the replacement is recorded once, in log.md. That deletion rule covers live documents only: where it meets the archive rule that a completed record keeps its bytes, preservation wins, so archived issue bundles, archived tickets, archived gap records, and log.md keep the old wording exactly, and an audit over live surfaces enumerates those historical carriers instead of skipping an unbounded set (schema.md `SVC-010`).

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
- **Scaffold**: The fixed structure (prompts, states, gates) wrapped around model calls.
- **Runbook**: The TOML file defining states, per-state prompts, and transitions for an agent run. Its schema also carries the validity rules — what a runbook may or may not contain (e.g. no lifecycle status as a graph dimension).
- **State**: Where a Run sits in its machine graph — one node of a runbook's machine. The only dimension of machine position; `status` is separate runtime lifecycle.
- **Run**: One live instance of a runbook's machine, created by `rtm start`: the whole running thing, with its own id, its own Run Record, and its own work. Not the file, and not the position in the graph.
- **Run Record**: The one file the Engine writes for one Run, `.ratmac/runs/<run-id>/run.toml`, holding that Run's current `state`, `status`, and run-scoped fields. Only the Scheduler writes it.
- **Artifact**: A file or output produced by real work (test log, build output, git commit).
- **Artifact guard / artifact-verified exit**: A State can be left only when an artifact proves the work happened — read from process outputs (exit codes, compiler, git) the agent cannot write.
- **Sensitivity receipt**: Proof that a test can actually fail (seen red before green). Blocks fake or always-passing tests.
- **Advisory vs enforced**: Advisory = the agent is asked to follow rules (~70% compliance). Enforced = a mechanism blocks violations (hooks ~100%).
- **Hook / PreToolUse**: A program the harness runs before each tool call; it can deny the call. The "wall" that stops an agent from editing scheduler-owned files.
- **Guard gaming**: Satisfying the letter of a guard without doing the work.
- **Fabrication**: An agent claiming work is done when it is not.
- **Honest stall**: A run that stops and hands back to the human instead of faking progress. A feature, not a bug.
- **Statewright**: Rust competitor product (per-state tool allowlists, MCP-first, patented method). Not Stateright.
- **MCP (Model Context Protocol)**: Protocol for exposing tools/resources to agents over a connection; heavier than CLI calls.
- **SDD / Spec Kit**: Spec-driven development; markdown stage workflows with no enforcement.
- **Codemod**: Mechanical, rule-based code rewriting tool — for syntax-level migrations, unlike semantic ones.
- **Beachhead**: The first narrow market to win before expanding.
- **rtm doctor**: The ratmac checker command: parses a runbook and reports schema, graph, and guard problems before any run.
- **Guard lint**: A doctor check that every exit guard reads an agent-uncontrolled source (exit code, git state) — flags fake-able guards.
- **Property-based testing**: Testing by generating many random inputs and asserting a property always holds, instead of hand-picked cases.
- **Lean**: A theorem prover: you write mathematical statements and machine-checked proofs. Heavy; proves only properties you explicitly state.
- **Active refs** (`active_refs`): One of the seven Run Record fields (R-025) — the Scheduler-written list of what a Run is currently working on, ticket and requirement ids. In the format and in the fixtures since the start; nothing populates it yet.
- **Per-ticket gate**: An exit guard whose verdict is about one named ticket — `sensitivity_receipts` (its planned tests each have a red-before-green receipt) and `completion_gate` (its declared checks each have a green, fresh receipt). Both need a ticket id, which a read-only runbook cannot supply per loop turn.
- **Verdict format**: The strict live `verdict.toml` record has exactly three non-empty string fields: current `state`, one legal transition `input`, and the external reviewer's `rationale`. The Engine stores no reviewer identity and adds no value; atomic archival preserves the original bytes.
- **Input list**: The closed transition-input values a branching State accepts, declared in the runbook and mapped one-to-one to its ordinary outgoing edges. A verdict input outside the list is refused, not routed; the list is also the reviewer's menu for that State.

## Shop process (.arca)

- **Sprint**: Starts when enough issues have collected to be worth integrating into the goal; runs the cycle — plan (P1–P3) then build (P4–P5) — until the gap check comes back clean. Issue-triggered, not time-boxed (unlike scrum's calendar sprint). Billy's term, 2026-07-27.
- **Plan-Build Runbook**: The formal name of the Machine Class for the P1–P5 working cycle and the first real runbook intended to run on the RatMac engine. The workflow owns its rules; RatMac only executes the declared Machine Class.
- **Route**: The ordered dependency list inside a sprint: what depends on what, one why per edge. Never dates or task breakdowns — that would be a plan. Lives in steering.md, Current sprint.
- **Stage derivation**: Answering "which stage are we in" by reading the tree with a fixed lookup rule (open tickets → building; unresolved gap records → P3; stale records → P2; a `pending` issue directly in the intake work area → P1; else idle), never by storing it in a file. A bundle in the Deferred issue buffer does not force P1; selecting it visibly moves that same bundle to intake with status `pending`. Stored status is narration, and narration drifts. Lives in index.md's front door.
- **Deferred issue**: One unresolved, exact five-file issue bundle whose `spec.md` gives at least one ask the disposition `deferred`, even if sibling asks were accepted or marked duplicate. It keeps its issue id and is neither completed history nor a reason to mint a replacement issue.
- **Deferred issue buffer**: `.arca/issue/deferred/`, the live waiting location for a Deferred issue. The bundle stays whole there with status `deferred`; it does not force P1, and selecting it moves that same bundle and issue id to the intake work area with status `pending`.
- **Landing**: One committed change plus its log.md line — one commit = one landing = one log line. The smallest provable step. Inside a ticket, the red commit (tests exist, fail) and the green commit (all tests pass) are its two required landings. Lives in schema.md, "Units and git".
- **Program lane**: Changes to what the program does (`src/`, tests, the runbook). Must enter the loop: issue → goal → residual → ticket. No program commit without a ticket. Lives in schema.md, "Units and git".
- **Shop lane**: Changes to `.arca` docs (steering, schema, index, dict, tpl, vis). Lands directly — steering first on pivots — one log line per landing; issue creation is the stated exception. Lives in schema.md, "Units and git".
- **Ticket worktree**: The linked Git worktree every build turn runs in — a ticket branch named after its ticket, holding that turn's landings; when the turn ends green, the ticket branch merges into `main` and the worktree and branch are removed. The opposite of a Trial worktree (goal/ubi-lang.md), whose branch never merges into `main`. Lives in schema.md, "Units and git".
- **Wish**: An idea parked with zero commitment; lives in wishlist.md. Cheap to write, allowed to rot — capture must cost less than forgetting.
- **Wishlist**: The unordered pool of wishes; the capture side of planning. Write-cheap by design — opposite discipline from steering, which is write-expensive because everything on it must be chosen.
- **Advisor**: The reviewing voice of a session: it watches how the work itself goes, and files what it finds as evidence — Wishes on `main` and trial-log observations. It authors notes only; it invokes no lifecycle verb and no `rtm` command, and it never waits to be asked for an observation. Lives in schema.md, "The wishlist" and "Trial worktrees".
- **Promotion**: The human act of choosing a wish onto steering's route. Never automated — this is where human judgment enters the pipeline.
- **Demotion**: Explicitly returning a route item to wish status. Legal and healthy; keeps steering from accumulating unchosen "zombie" items that make the map lie.
- **Hardening gradient**: The pipeline wish → route item → issue → goal → gap → ticket → landing. Each step hardens intent: maybe → chosen → specified → decided → measured → committed → done.
- **Delta**: The change an issue states — authored intent, system-independent. The same issue reads identically against any codebase.
- **Gap (residual)**: Derived measurement, goal minus current HEAD per requirement — system-relative, recomputed each planning pass, never archived as truth. Strictly contains the delta plus accommodation and contradiction-removal.
- **Accommodation**: Work the current system needs so a delta can land without breaking invariants (seams, refactors, migrations, test infra). Visible only in the gap, never stated by the issue — why tickets are cut from gaps, not issues.
- **Contradiction**: Existing behavior the target state forbids. Issues are written additively, so only the goal-vs-current diff surfaces the "stop doing Y" work.
- **Drift**: The gap changing while issues sit still, as the system moves toward or away from the goal. Why gaps must be re-measured every planning pass.
- **Requirement ID**: `XXX-NNN` — three letters naming the issue that authored the requirement, three digits numbering it (`RBS-001` runbook spec, `TRP` typed runbook parser, `DRD` deep rtm doctor, `AAL` agent authoring loop, `PCR` retained for the Plan-Build Runbook from its earlier P-Cycle Runbook name). One prefix per issue, chosen from the issue title; the expansion is defined in that issue's `ubi-lang.md`, so no abbreviation reaches a response undefined. Requirements keep their id forever — through the goal, the residual that measures them, and the ticket that lands them.
- **Ideal shape**: The destination — the properties the finished system has, authored in steering.md as prose with no requirement IDs, no dates, no ordering, no measurement. Distinct from the three neighbours it is easy to confuse it with: Horizon *orders* what comes next, the goal bundle *specifies* what must become true this Run, the gap check *measures* distance. Its one mechanical use: every issue folded in at P1 names the property it advances, so a batch of issues can no longer become the goal by arriving. Billy's term, 2026-07-28.
- **Deliberate-damage check**: Briefly breaking the code on purpose to watch a named test fail — the mutation evidence every gap record cites. Runs only from a Checkpoint, never lands, and its kills are written into the owning gap record only after the observed failure. Lives in schema.md, "Deliberate damage and discard safety".
- **Discard command**: Any command that throws away uncommitted changes — `git checkout -- <path>`, `git restore`, `git clean`, `git reset --hard`, dropping a stash. Never run while the tree holds unsaved completed work: look first, then save or park. Restoring saved bytes from a Checkpoint is restoration, not a discard.
- **Checkpoint (safety commit)**: The ephemeral commit made after a turn's tests are green and before any deliberate damage — subject exactly `t-<id>: checkpoint - not a landing`. Unpublished, unmerged, not a Landing, no log line; every damage undo restores from it (`git restore --source=<checkpoint> --staged --worktree -- <paths>`), and `git commit --amend` folds it into the green landing before the merge.
- **Park**: Setting unsaved wanted work aside without landing it: `git stash push -m "t-<id>: <what>"`, dropped only after its content lands or is explicitly declared obsolete. The alternative to a save when a discard must run first.
- **Build target**: One thing the build produces from source - a command a person can run, or a library other code links. Each target has a name and one output file.
- **Output collision**: Two build targets writing the same output file. Whichever finishes last survives, so what you run depends on build order rather than on what you asked for.
- **Pause point**: A seam compiled into a test build only, letting a test stop the engine at a named moment mid-operation, look at the tree, and let it continue. The shipped command never carries one.

