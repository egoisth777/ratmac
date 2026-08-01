# Engine Constraints on Per-Subtask Machine Instances

**Date:** 2026-07-28
**Scope:** Ground-truth audit of everything in ratmac's current code and written rules that blocks or shapes giving each subtask — a subagent working one ticket, typically in its own git worktree — its own machine instance, composable into a graph with parallel fan-out and fan-in.
**Method:** Full read of all eighteen source files under `src/`, the project runbook, the working rules, the goal authority, and the open issue about running the cycle as a runbook. Every claim carries `path:line`. Nothing here proposes a design; each constraint is followed only by what lifting it would require.

**Classification used throughout.** Every constraint is tagged:

- **(a) enforced by code** — a refusal, a hard-coded path, or a data-flow fact a reviewer can hit by running the binary.
- **(b) written rule, no enforcement** — prose in `.arca/` that binds contributors but that nothing in `src/` checks.
- **(c) accident of implementation** — nobody chose it; it fell out of a partial build, a dead type, or a missing case.

---

## Verdict

**The singleton is one line of code. The obstacle is the guard vocabulary.**

Exactly one code path enforces one-Run-per-directory: `scheduler.rs:242` refuses `start` when `.arca/state.toml` exists. Everything else that reads like a singleton is either written rule with no enforcement, or accident. The plural `Runs` type that requirement R-021 points to is real, compiles, and is constructed **only inside a test** — no production path builds one.

The hard blocker is elsewhere and is not about identity at all. `sensitivity_receipts` and `completion_gate` each carry a **literal `ticket` string in the runbook file** (`machine.rs:47-52`, `machine.rs:76`). A phase that gates on a ticket names that ticket in TOML. Since the runbook is one file per project root (`scheduler.rs:203`) and is re-read on every `step` (`scheduler.rs:312`), a per-ticket gate today requires either editing the shared runbook per ticket or giving each ticket its own runbook — which is exactly the fork the open issue about running the cycle as a runbook records and leaves unresolved.

The second hard blocker is the graph itself. Transitions carry no condition of any kind (`graph.rs:50-59`: two booleans, both about routing *kind*, not about data), and `transition_for` returns **the first** matching edge (`graph.rs:131-136`). A phase therefore has exactly one ordinary successor. Fan-out is unrepresentable, not merely unimplemented.

The written rules are not neutral: the project already answered this question, in advance, naming this exact scenario. ADR-0003 (`.arca/goal/design.md:23`) was written because "wishwillow fans tickets out to Subagents in worktrees; if any agent may call `step`, the Machine leaks downward into every worker," and it decided that "Ticket→worktree parallelism stays inside a Phase; Exit Guards check the merged result" (`.arca/goal/design.md:27`). That is isolation-by-prohibition chosen deliberately, with the failure it avoids stated. It also carries its own escape clause in the same sentence: "the policy is a documented rule, not enforced code (revisit if violated in practice)."

Against that sit three passages that pre-authorize the opposite: ADR-0007 defers a run identity scheme rather than refusing one, ADR-0008 writes the N-Run migration down ("per-Run files move under a runs directory; the v1 flat layout is the one-active-Run projection of that"), and the frozen glossary already calls the State File and Transition Log **per-Run**. The engine drifted from that promise: R-021 claims "nothing in formats ... assumes a singleton", but `state.rs:16-24` fixes seven fields and refuses an eighth, so no run identifier fits today. **The code is stricter than the rule it implements.**

The good news for isolation: the engine already derives every path from a single `root`, resolves that root as plain `current_dir()`, and confines `files_exact`/`file_contains` guards inside it. Two worktrees today are already two fully independent engine instances — by accident, not by design, and with no way to relate them.

---

## Findings

### 1. Run identity and the instance model

**There are two Run models. The engine persists one and never uses the other.**

The persisted model is `RunState` (`model.rs:193-202`) — a flat struct of seven required fields, strictly parsed: `state.rs:16-24` lists them, `state.rs:70-83` rejects both a missing field and an unknown one. This is what `.arca/state.toml` holds and what every CLI command reads.

The in-memory model is `Run` (`model.rs:110-115`) with `RunArtifacts` (`model.rs:79-83`) and the plural `Runs` (`model.rs:168`). Its doc comment states the intent plainly:

> `model.rs:166` — "A plural collection of independent Runs; it has no singleton or identity policy."

**Who constructs `Runs`: nothing in `src/`.** The only construction anywhere is `test/qa/tests/t011_r021.rs:37`, which builds two `Run` values from two fixture files and asserts `runs.len() == 2`. The requirement it proves (R-021, "the data model allows N Runs") is satisfied by a type the engine never instantiates. **(c)** — the plural type is a compile-time promise with no runtime consumer.

`Run` itself is constructed once in production, at `scheduler.rs:298`, and immediately returned to a caller that discards it: `cli.rs:144-148` calls `scheduler.start()` and propagates only the error. `RunArtifacts` — the per-Run bundle of state, log, and lock paths, which is the closest thing in the codebase to a per-instance handle — is populated only by `with_artifact_root` (`model.rs:155-158`), reachable only from that same discarded return value, and read only by `test/qa/tests/t014_r024.rs:48`. **(c)**

**Which of the seven `RunState` fields are live:**

| Field | Written by | Status |
| :--- | :--- | :--- |
| `phase` | `scheduler.rs:251` (start), `scheduler.rs:383` (step), `blocked.rs:251` (hold) | Live — the only field routing depends on |
| `status` | `scheduler.rs:251` only, always `Planned` | **Dead in practice (c)** — see below |
| `goal_revision` | `scheduler.rs:399`, only on the freeze transition | Live but single-shot |
| `input_revision` | never | Dead |
| `output_revision` | never | Dead |
| `active_refs` | never | Dead |
| `blocker` | `scheduler.rs:824` only | Unreachable from the CLI |

`status` deserves its own note. `start` writes `Planned` (`scheduler.rs:251`). `step` copies the state and mutates only `phase` and possibly `goal_revision` (`scheduler.rs:382-400`) — `status` is never touched. `hold` mutates only `phase` (`blocked.rs:251`). The two writers that could set `Blocked` are `Run::block_for` (`model.rs:160-163`, reachable only through `evaluate_entry_prerequisites`, which no CLI command calls) and `Scheduler::record_missing_prerequisite` (`scheduler.rs:816-826`, called only from `test/qa/src/lib.rs` and `t022_r026.rs`). **A state file written by `rtm` therefore reads `status = "planned"` for the entire life of the Run.** `Passed`, `Failed`, and `Executing` exist in the enum (`model.rs:12-18`) and are never written by anything. **(c)**

`active_refs` is the one dead field the project has already noticed. `.arca/dict.md:62` and `.arca/issue/deferred/i-015-cycle-as-runbook/ubi-lang.md:12` both describe it as present in the format and in the fixtures and populated by nothing. That matters here because it is the obvious carrier for "which ticket is this instance working on" — the issue's own option (c) proposes exactly that.

`graph.rs:152-168` — `MachineState` is a newtype over a single `Phase`, documented as having "no lifecycle/status dimension". It is constructed nowhere in `src/`: `step` and `status` work directly on `RunState`. **(c)** — a third, wholly unused position type.

**What per-instance identity would require:** a run identifier does not exist anywhere in the on-disk format. `state.rs:16-24` fixes the field list at seven and `state.rs:77-83` refuses any eighth key, so adding one is a format change that invalidates every fixture under `test/fixtures/`.

---

### 2. Root resolution — worktrees already work, by accident

`src/bin/rtm.rs:5` is the whole of it:

```rust
let project_root = match env::current_dir() {
```

There is no upward search for a marker, no `--root` flag, no `git rev-parse`. A repository-wide grep for `ancestors()` and `.parent()` finds four hits, all local path manipulation (`pin.rs:153`, `scaffold.rs:70`, `scheduler.rs:111`, `state.rs:100`) and none of them a root walk. The string `"git"` appears in `src/` exactly once, at `completion.rs:223`, where `.git` is skipped by name while hashing a tree.

Three consequences, all unchosen:

1. **Every worktree is already a separate engine instance.** `Scheduler::open(root)` (`scheduler.rs:190-198`) derives the runbook, the state store, the lock, the log, and every guard target from that one `root`. Two worktrees of the same repository share no engine state whatsoever — separate `.arca/state.toml`, separate `.arca/rtm.lock`, separate `.arca/evidence.toml`. **(c)** — isolation between worktrees is total and unintended.
2. **`rtm` only works from the exact project root.** Run it from a subdirectory and `load_class` (`scheduler.rs:202-212`) reads `<subdir>/.arca/ratmac.toml`, fails, and refuses with "read .arca/ratmac.toml: No such file". **(c)**
3. **The bootstrap builds a separate Engine per worktree.** `tools/rtm.ps1:69` sets `$root` to the script's parent, `:71-76` refuses to run unless the current directory equals it, and `:84-85` looks for the binary under `$root/target/`. A worktree gets its own `target/`, its own compile, and its own Engine hash pinned into its own `.arca/evidence.toml` (`pin.rs:161-163`). **(a)** for the refusal, **(c)** for the per-worktree rebuild cost.

A fourth, smaller one: `abandon`'s confirmation phrase is derived from the project **directory name** (`abandon.rs:58-71`), so it is already per-worktree — `abandon ratmac-t-042` rather than `abandon ratmac`. **(c)**, and it happens to be the only place in the engine where an instance has a human-facing name.

---

### 3. Locking — what `.arca/rtm.lock` actually serializes

The lock is an exclusive-create of one file (`scheduler.rs:131-136`) with an RAII `Drop` that unlinks it (`scheduler.rs:173-177`). Contention spins: up to 4096 attempts with `thread::yield_now()` between them and no sleep (`scheduler.rs:138-171`), then refuses with "lock remained held after 4096 attempts".

**Scope: one directory, one lock.** The path is always `<root>/.arca/rtm.lock` (`scheduler.rs:238`, `scheduler.rs:310`, `scheduler.rs:800`). Nothing coordinates across roots. Two worktrees contend for nothing. **(a)** — and this is the correct behavior for per-subtask instances, already in place.

**What it covers, and does not:**

| Command | Takes the lock? |
| :--- | :--- |
| `start` | Yes — `scheduler.rs:239` |
| `step` | Yes — `scheduler.rs:311`, held through **all** guard evaluation |
| `status` | Yes — `scheduler.rs:839` (a read-only command that contends) |
| `hold` | **No** — `cli.rs:227-230` calls `plan_hold`/`apply_hold` directly; `blocked.rs` never mentions `InvocationLock` |
| `abandon` | **No** — `cli.rs:271-273`; and `abandon.rs:139-140, 221-230` *deletes* the lock file |

So the lock does not serialize all writers of Scheduler-owned files. A `hold` and a `step` can write `.arca/state.toml` and append `.arca/log.md` concurrently, and an `abandon` running against an in-flight `step` unlinks the lock the `step` is holding. **(c)** — a genuine gap, not a decision.

Because `step` holds the lock across the whole guard pass (`scheduler.rs:311` through `scheduler.rs:341`), and guards may spawn subprocesses (`scheduler.rs:676-690`, `.current_dir(root)`), **the lock's real effect is serializing guard command execution within one directory** — including a full test-suite invocation, if a runbook declares one. Any design that puts several instances under one root inherits that serialization.

A killed process leaks the lock: `Drop` never runs, and the schema's own text says the only cure is `rtm abandon` (`.arca/schema.md`, "Abandoning a Run": "A stale lock is retired through this same path; no bypass flag exists"). **(a)**

`scheduler.rs:110-129` additionally refuses to run at all if a lock file under the pre-rebrand command name sits beside the current one — checked before acquisition, after acquisition, and on every contention retry.

---

### 4. Routing — one successor, no conditions

`Transition` (`graph.rs:50-59`) carries `from`, `to`, and two booleans: `freezes_goal` and `blocked_route`. Both describe the *kind* of edge, not a predicate over anything. **There is no condition field, no guard reference, no priority, no label.** The parser confirms the closed set: `machine.rs:373-377` accepts exactly `from`, `to`, `freeze`, `blocked-route` and refuses any other key with code `RB103`. **(a)**

`transition_for` (`graph.rs:131-136`):

```rust
self.transitions
    .iter()
    .find(|transition| transition.from.as_str() == phase && !transition.blocked_route)
```

`find` — the **first** non-blocked edge leaving the phase. A second edge from the same phase is silently unreachable. `next_phase` (`graph.rs:147-149`) is `transition_for(...).map(to)`, and `step` uses exactly that (`scheduler.rs:350`). So the destination is a pure function of the current phase name. **(a)** — fan-out is not representable in the type, let alone in the router.

The doctor reinforces the shape rather than flagging the ambiguity: `doctor.rs:250-256` warns (`RB206`) on a duplicate edge, but nothing warns on two *different* destinations from one phase, which is the case that silently loses a route. **(c)**

**Blocked routes are the only second edge, and `step` never takes one.** `graph.rs:139-144` finds it, `blocked.rs:141-145` requires the current phase to declare one, and the comment at `graph.rs:54-57` states why: "step never takes it, so ordinary routing stays deterministic and never branches on any lifecycle field." Taking it requires a human typing `hold <ticket-id>` verbatim (`blocked.rs:84-97`). **(a)**

**Exactly one entry point is enforced twice.** `Scheduler::initial_phase` (`scheduler.rs:757-779`) collects phases with no inbound ordinary edge and refuses on zero ("no unique initial Phase") *and* on more than one ("multiple initial Phases"). `doctor.rs:288-311` repeats it as errors `RB202` and `RB203` — "several initial Phases; a Run starts in exactly one". **(a)** — a graph with parallel entry points cannot start.

Multiple *endings* are merely discouraged: `doctor.rs:340-351` emits warning `RB205` when more than one phase is terminal — "one ending is the ordinary shape, several usually mean a missing edge". **(b)** at the tooling level: it is advice, exit code 1, not a refusal.

**What fan-out would require:** a `Transition` that can carry a predicate; a router that returns a set rather than an `Option`; a `RunState.phase` that can hold more than one position (today a `String`, `model.rs:195`); and an initial-phase rule that admits several. Each of the four is enforced by code independently.

---

### 5. Guard kinds and per-instance targets

The vocabulary is closed at seven kinds with per-kind field lists (`machine.rs:59-80`), and the parser refuses a field a kind does not accept (`machine.rs:512-519`, code `RB107`). **(a)**

**Two kinds require a literal ticket, and this is the central obstacle.**

`machine.rs:47-52` defines `SensitivityReceipts { ticket: String }` and `CompletionGate { ticket: String }`; `machine.rs:76` gives both the single accepted field `ticket`; `machine.rs:537-542` parses it with `field.string("ticket")?` — **required, no default, no substitution, no interpolation**. The runbook spec states the same (`.arca/runbook-spec.md:62-63`). **(a)**

That string is then used as a **path relative to the root**:

- `receipt.rs:248-249` — `gate_sensitivity(root, ticket_relative)` does `root.join(ticket_relative)` and reads it as the ticket file.
- `receipt.rs:256-259` — the ticket *identifier* is the file stem of that path.
- `receipt.rs:269` / `receipt.rs:108-110` — receipts are then sought at `<root>/.arca/evidence/<stem>/`.
- `completion.rs:330-342` — identical shape, plus `/completion/`.

So a phase gating on ticket `t-042` must contain the literal string `.arca/ticket/archive/t-042.md` in `.arca/ratmac.toml`. There is exactly one runbook per root (`scheduler.rs:203`, `machine.rs:228`, both hard-coding `.arca/ratmac.toml`), and it is re-read on every single `step` (`scheduler.rs:312`) and every `status` (`scheduler.rs:844`) — so an edit takes effect immediately, but it is still an edit to a file the working rules classify as human-authored.

**A path-escape asymmetry worth recording.** `files_exact` and `file_contains` route through `guarded_target` (`scheduler.rs:904-939`), which rejects `..`, absolute paths, and drive prefixes, then canonicalizes and refuses a symlink that leaves the root. `sensitivity_receipts` and `completion_gate` **do not**: `scheduler.rs:483-486` passes `ticket` straight through to `root.join(...)`. A ticket path of `../sibling-worktree/.arca/ticket/archive/t-042.md` would be read. The evidence directory would still resolve inside the *current* root, since only the file stem survives (`receipt.rs:256-259`). **(c)** — an inconsistency, and incidentally the only existing seam through which one instance can read another's records.

**Hard-coded `.arca/*` paths — the R-016 debt, already named by the project.** `.arca/index.md:72` — "`contract.rs` | Intake/record contract gates; hard-codes `.arca/issue`, `.arca/residual`, `.arca/ticket` (R-016 debt)"; `.arca/index.md:74` says the same of `blocked.rs`; `.arca/index.md:102` — "R-016: `contract.rs`/`blocked.rs`/`goal.rs` bake in `.arca/*` paths."

The full inventory, all **(a)**:

| Path | Sites |
| :--- | :--- |
| `.arca/ratmac.toml` | `scheduler.rs:203`, `machine.rs:228`, `blocked.rs:128`, `cli.rs:329`, `cli.rs:404` |
| `.arca/state.toml` | `state.rs:56`, `blocked.rs:196`, `abandon.rs:108`, `cli.rs:405`, `model.rs:88` |
| `.arca/log.md` | `scheduler.rs:361`, `blocked.rs:197`, `abandon.rs:160`, `model.rs:89` |
| `.arca/rtm.lock` | `scheduler.rs:238`, `scheduler.rs:310`, `scheduler.rs:800`, `abandon.rs:110`, `model.rs:90` |
| `.arca/evidence.toml` | `pin.rs:161-163`, `abandon.rs:109` |
| `.arca/evidence/` | `receipt.rs:36`, `receipt.rs:108-110`, `completion.rs:342` |
| `.arca/goal/` | `goal.rs:16`, `goal.rs:23` |
| `.arca/goal/spec.md` | `contract.rs:81`, `contract.rs:187`, `contract.rs:215` |
| `.arca/issue/` | `contract.rs:399` |
| `.arca/residual/` | `contract.rs:221` |
| `.arca/ticket/` | `contract.rs:296`, `blocked.rs:110` |

Every one of these is `root.join(<literal>)`. The consequence for per-subtask instances is uniform: **an instance's identity is entirely its root directory.** Two instances under one root would collide on all twelve; two instances in two worktrees collide on none.

**The contract gates read the whole project, not one ticket.** `gate_intake` (`contract.rs:79-201`) reads issue bundles across intake, deferred, and archive as one namespace, with deferred live and archive historical; `gate_records` (`contract.rs:204-361`) walks every residual and every ticket, checks one-residual-per-requirement globally (`contract.rs:280-291`), one-owning-ticket-per-gap globally (`contract.rs:331-347`), and runs a cycle detector over the whole ticket dependency graph (`contract.rs:349-354`). Neither takes a ticket argument — `machine.rs:77` gives both kinds an empty accepted-field list. **(a)** — these gates are inherently whole-project, and a per-subtask instance in its own worktree would evaluate them against that worktree's partial copy of the records.

**`unproven_mechanization` couples every instance to the runbook's declared vocabulary.** `contract.rs:367-381` classifies PGE-001, PGE-002, and PGE-003 as *missing* whenever the runbook declares no gate of kind `intake_contract`, `record_contract`, or `sensitivity_receipts` respectively, and `contract.rs:266-271` then refuses any `satisfied` residual for those requirements. `declared_gate_kinds` (`contract.rs:384-395`) reads the single project runbook. So splitting one runbook into several per-ticket runbooks would change which requirements are provable, in every instance. **(a)** — and a sharp edge for any per-instance runbook scheme.

**Freshness is a whole-tree hash.** `completion.rs:195-241` digests every file under the receipt's declared `tree-roots`, skipping only `target` and `.git`. A receipt goes stale the moment anything under those roots changes (`completion.rs:498-504`). Under one root with parallel subtasks, any instance's edit invalidates every other instance's completion receipts. Under separate worktrees, it does not. **(a)** — the single strongest technical argument that per-subtask isolation wants separate directories.

**The goal revision is likewise whole-directory.** `goal.rs:22-39` hashes every file under `.arca/goal/` including relative names, and `scheduler.rs:329-340` re-checks it on every `step` once frozen. Two instances under one root share one goal revision and one freeze; two worktrees each freeze their own copy independently. **(a)**

**Guard evaluation order and failure aggregation** are per-phase and sequential: `scheduler.rs:466-501` walks the phase's guards in declaration order and collects every failure. There is no notion of a guard belonging to an instance rather than to a phase.

---

### 6. Written-rule constraints

Every item here is **(b)** — prose that binds contributors with nothing in `src/` checking it — except where noted. They divide into three groups: rules that forbid the design, rules that pre-authorize it, and one rule that already decided the question the opposite way.

**The forbidding rules.**

`.arca/steering.md:75`, under Non-goals: "Not multi-tenant: one repository, one Run at a time, local disk." The file's own header (`.arca/steering.md:4-6`) says it "is direction: what ratmac is for, the bets behind it, and the lines no goal, issue, or ticket may cross. When direction changes, this file changes **first**." So this is not a preference — crossing it is a pivot, and the pivot lands here before anything else moves.

The goal authority splits the same idea across three requirements, and the split matters:

- `.arca/goal/spec.md:27` — **R-021**: "The data model allows N Runs; nothing in formats or engine assumes a singleton."
- `.arca/goal/spec.md:28` — **R-022**: "v1 CLI allows at most one active Run per project; `rtm start` refuses while a Run is active."
- `.arca/goal/spec.md:29` — **R-023**: "`rtm step` and `rtm status` take no run-id in v1; they target the active Run."

R-021 permits the model; R-022 and R-023 restrict the command-line interface. Only R-022 has an enforcer (`scheduler.rs:242`) — **that one is (a)**. R-023 is enforced negatively by `cli.rs:136-140`, which refuses any extra argument to `start`, `status`, or `step`; the doc comment at `cli.rs:86-89` states the intent: "both operate on the active Run selected by `Scheduler::open`."

`.arca/goal/spec.md:30` — **R-024**: "Scheduler-owned files sit flat under `.arca/`, no folder: `ratmac.toml`, `state.toml`, `log.md`, `rtm.lock`." N concurrent Runs under one root need N of each. **(a)** in effect, via the twelve hard-coded paths in section 5.

`.arca/runbook-spec.md:9-10` — "A runbook is plain TOML data at `.arca/ratmac.toml`, **one per project**, human-reviewed and read-only at runtime (R-010, R-013)." Repeated in the frozen glossary at `.arca/goal/ubi-lang.md:9`. This is the sharpest sentence against a per-ticket or per-worktree runbook. Note the scope tension: `.arca/runbook-spec.md:3-7` declares itself "the single authority for the Machine Class **format**", and "one per project" is a deployment fact, not a format rule. The read-only half has no enforcer at all — `.arca/runbook-spec.md:77` records it as "prose-only - no writer of the runbook exists in `src/`, so there is nothing to constrain yet."

**The caller policy — the hardest doc-level obstacle.** `.arca/goal/spec.md:94` (**ORS-001**): "a Subagent never invokes any `rtm` command." `.arca/goal/spec.md:14` (**R-008**) says the same, `.arca/goal/spec.md:111` (**TWL-010**) restates it for worktree lifecycle, `.arca/schema.md` restates it under "Caller policy for `rtm`", and `.arca/goal/ubi-lang.md:19` bakes it into the definition of Subagent: "A worker agent in a ticket worktree. Reads state; never invokes `rtm`."

A subtask that owns its own machine instance must step that instance. Under ORS-001 it cannot. This is the one written rule the design cannot route around by re-scoping — and it is `(b)`: `.arca/goal/spec.md:94` states plainly that "The Engine gains no caller identity, authentication, or authorization state." The engine literally cannot tell who is calling. Compliance is voluntary today, and `.arca/goal/spec.md:96` (ORS-003) tests it only as a recorded role scenario, not as a runtime refusal.

**The rule that already decided this question, in advance, naming this exact scenario.** ADR-0003, `.arca/goal/design.md:23`:

> "wishwillow fans tickets out to Subagents in worktrees; if any agent may call `step`, the Machine leaks downward into every worker."

and its consequences, `.arca/goal/design.md:27`:

> "Subagents need zero Scheduler awareness — the Machine is invisible below the Main-Agent. Ticket→worktree parallelism stays inside a Phase; Exit Guards check the merged result. The CLI needs no caller authentication in v1; the policy is a documented rule, not enforced code (revisit if violated in practice)."

This is the decided alternative: **fan-out is modeled as work inside one Phase of one Run, with the guard checking the merged result** — isolation by prohibition, chosen deliberately, with the leak it prevents named. The last clause is also the written escape hatch: "revisit if violated in practice."

Three smaller rules foreclose the surrounding machinery:

- `.arca/goal/design.md:80` (ADR-0009) — "The Phase Prompt is the ONLY machine information an agent ever receives — never the flowchart, never other Phases." A fan-in barrier needs an instance to learn about sibling instances; the prompt may not carry it.
- `.arca/goal/design.md:88` (ADR-0010) — "No process management, no spawn flag." A parent instance may not launch children.
- `.arca/issue/deferred/i-015-cycle-as-runbook/design.md:126` — "No agent-spawning, no process management, no scheduling: the Run is still stepped by a caller."

**The pre-authorizing rules.** Three passages written the extension path down before it was needed.

`.arca/goal/design.md:59-61` (ADR-0007, "Model N Runs, allow 1 active"):

> "**Decision.** Data model: Runs are plural — nothing in formats or engine assumes a singleton. v1 CLI: at most ONE active Run per project ... **Consequences.** ... Lifting the limit is additive: allow `start` to create a second Run, grow an optional run-id argument; no breaking change. The Run identity scheme is deferred until the limit lifts (YAGNI). The on-disk layout must not preclude N Runs (settled in ADR-0008)."

`.arca/goal/design.md:74` (ADR-0008 consequences):

> "N-Run extension path: when ADR-0007's limit lifts, per-Run files move under a runs directory; the v1 flat layout is the one-active-Run projection of that — additive migration, deferred."

The identity scheme is *reserved*, not refused. But the code has drifted from the promise: R-021's "nothing in formats or engine assumes a singleton" is false of the format as built — `state.rs:16-24` fixes seven fields and `state.rs:77-83` refuses an eighth, so a run identifier cannot be added without a format change that invalidates every fixture. **The written rule and the code disagree, and the code is the stricter one.**

`.arca/goal/spec.md:20` (**R-014**) — "each Run owns its State File, Transition Log, and lockfile" — and `.arca/goal/ubi-lang.md:15-16`, which call the State File and Transition Log **"Per-Run"**. The design vocabulary is already per-instance; only the paths are not.

`.arca/runbook-spec.md:80` records existing precedent for a ticket-keyed layout inside the Scheduler's own world: "Run evidence (`.arca/evidence.toml`) is Scheduler-owned; agent-authored receipts live under `.arca/evidence/<ticket>/`."

**Routing is an open question, not a closed one.** `.arca/steering.md:98-103`:

> "**How much routing does a runbook get?** Each phase now has one automatic destination - `rtm step` takes the first transition declared out of that phase - plus at most one human-authorized alternate, the blocked route that `rtm hold` takes. A loop that exits on a condition cannot be written down. Either runbooks gain conditional and repeating transitions, or every process gets rewritten as a straight walk. Undecided."

The section's preamble (`.arca/steering.md:79-84`) says it "binds nothing - nothing here is chosen and no work is cut from it." So the single-successor router of section 4 is code-enforced but **not** doctrine: the fork is written down and open.

**What `i-015` already records as open.** The issue about running the cycle as a runbook is `status: "pending"` (`index.md:6`) — not integrated, not in the signed sprint set (`.arca/steering.md:118-119`). It names the ticket-field problem as the mechanical blocker, `spec.md:17` (**PCR-007**):

> "Both kinds require a literal `ticket` field, and the runbook is human-authored and read-only at runtime (R-010, R-013) with no interpolation in the format. As written, gating ticket t-058 means editing the runbook every loop turn. **This - not the log file - is the mechanical reason the cycle is not a runbook today.**"

Four options are recorded "for P1 to choose between rather than settled here" (`design.md:10`):

| Option | Text | Marked |
| :--- | :--- | :--- |
| (a) Coarse loop — one Phase gated by `record_contract` | `design.md:12-17` | **Rejected** — silently drops PGE-003 and PGE-005, "a regression dressed as a simplification" |
| (b) **Run per ticket** — "`rtm start` once per ticket; the runbook names that ticket. Keeps the gates, but makes the runbook a per-ticket artifact and multiplies Runs, and the cycle's own P1-P3 stages then sit outside any Run." | `design.md:18-20` | **No rejection marker** — three costs named, not scored against the multi-tenancy non-goal |
| (c) Bind the gate target from `active_refs` | `design.md:21-27` | **Recommended** — "Cheapest of the three that keep the gates" |
| (d) Field interpolation in the runbook format | `design.md:28-29` | **Rejected** — "makes the runbook a template language and reopens R-013" |

Option (b) is this design, written down by the project itself, with its costs enumerated and no verdict attached.

The constraint recorded against the recommended option (c) is worth quoting in full, `design.md:34-41`, because it is an argument the (b)-shaped design does not have to answer:

> "If the active ref is only ever *set* and never *derived*, then whatever writes `.arca/state.toml` chooses which ticket `sensitivity_receipts` and `completion_gate` grade. Point it at a ticket whose residuals are already `satisfied` and both gates pass while the real work stays unproven - and this needs no bad intent, a stale ref left over from the previous loop turn does it."

When the instance *is* the ticket, there is no ref to point at the wrong thing.

Two further items `i-015` leaves open bear directly here. `design.md:107-110` — "**Open: Run lifetime.** Whether the cycle Run is perpetual ... or per-sprint ... is undecided." And `design.md:66-70` — the transition log line is exactly `- Transition: <from> -> <to>` (confirmed at `scheduler.rs:410`), so "today's history cannot say *which* ticket advanced." A graph of instances would need that, and it does not exist.

**The acceptance bar any option must clear**, `spec.md:31`: "Every guarantee `PGE-003` and `PGE-005` already carry per ticket is still carried per ticket."

**One enforcement caveat that undercuts the whole rule layer.** `.arca/index.md:68`, about `state.rs`:

> "that centralizes the Engine, it does not defend the file: nothing stops an agent or a person editing `.arca/state.toml` directly ... **Invariant 1 is therefore a rule the Engine keeps, not a rule it enforces.**"

`.arca/steering.md:86-97` records the matching open question, "Catch it, or stop it?". So the sole-writer invariant — the property that makes today's shared Run safe at all — is itself **(b)**. The design pressure's framing is exactly right: today's isolation is by prohibition, and the prohibition has no mechanism behind it.

**Definitional gaps found while checking.** "Engine" is used throughout `.arca/steering.md` and `.arca/index.md` but is defined in neither `.arca/dict.md` nor `.arca/goal/ubi-lang.md`, whose line 3 states "Terms not listed here must not be used in docs, code, or CLI output" — the canonical term is *Scheduler* (`.arca/goal/ubi-lang.md:11`). "Ticket" likewise has no glossary entry in either file. `.arca/steering.md` has no section titled Horizon despite `.arca/steering.md:22` and `.arca/dict.md:84` referencing one as an authority. And `.arca/issue/deferred/i-015-cycle-as-runbook/test-plan.md:12` still carries verification row PCRV-006 for the PCR-006 requirement dropped at `spec.md:19-21` — whose content, "the contract and freeze guards read [roots] from the parsed class ... with no `.arca` literal in `src/`", is precisely the mechanism a per-instance design needs.

**No git-state guard kind exists.** `.arca/index.md:87-89`: "Two guards are not runbook kinds: goal drift against the frozen revision hash is implicit, and no git-state kind exists - only `command_exit` reaches tree state." A worktree-based design has no declarative way to guard on branch or worktree state, and `command_exit` drags in the pinning requirement (`scheduler.rs:706-755`, `doctor.rs:366-374`).

**Process cost.** `.arca/dict.md:71` puts the runbook in the program lane: "Changes to what the program does (`src/`, tests, the runbook). Must enter the loop: issue → goal → residual → ticket. No program commit without a ticket." So none of the code changes in section 5 can land directly. And `.arca/steering.md:152-153`: "On a pivot: steering -> `goal/` -> issue triage -> tickets. The frozen goal is never edited while tickets are open; a new issue is the only road back."

---

### 7. Evidence and state file placement under git

Verified with `git ls-files` and `.gitignore`.

**Tracked** (`git ls-files .arca`): `dict.md`, `index.md`, `schema.md`, `steering.md`, `runbook-spec.md`, `runbook-authoring.md`, the five `goal/` files, the issue/ticket/residual trees, `.arca/research/ratmac-feasibility/`, and two runtime-adjacent files:

- **`.arca/ratmac.toml` — tracked.** The runbook is version-controlled. A per-ticket runbook scheme therefore produces tracked churn, or an untracked file the reviewable-snapshot rule would flag.
- **`.arca/log.md` — tracked.** The append-only history is in git.

**Gitignored** (`.gitignore`): `target`, `debug`, `**/*.rs.bk`, `*.pdb`, `**/mutants.out*/`, and `.arca-private/` — the hidden-test tree.

**Neither tracked nor ignored:** `.arca/state.toml`, `.arca/rtm.lock`, `.arca/evidence.toml`, and the whole `.arca/evidence/` tree. None of them appears in `git ls-files`, and no `.gitignore` line covers them. **(c)** — they are invisible today only because none of them exists in this repository; the moment `rtm start` runs here, `git status` shows three untracked files with no rule saying what to do with them.

That collides directly with a written rule. `.arca/schema.md`, "Reviewable snapshot": "every file under the declared evidence roots (`src/`, `test/`, `.arca/`) must be tracked or staged; anything untracked or unstaged is either committed, staged, or declared as an explicit exception in the record." **(b)** — nothing enforces it, and the engine's own runtime output would violate it.

**Current disk state confirms the repository runs no Run:** `.arca/` contains no `state.toml`, no `rtm.lock`, no `evidence.toml`, and no `evidence/` directory, while holding 28 tickets and 55 residuals. The loop that produced all of that ran without the engine — which is what `.arca/schema.md` says under "Evidence receipts": "This repository's own loop runs no Run, so no gate consumes receipts here and none are written."

**The shipped runbook is a toy.** `.arca/ratmac.toml` declares three phases (`build`, `build-review`, `build-done`), two file guards on `artifacts/release.txt`, one exempt `rustc --version` probe, and two plain transitions. It declares **no** `intake_contract`, **no** `record_contract`, **no** `sensitivity_receipts`, **no** `completion_gate`, and **no** blocked route. Consequences: `contract.rs:367-381` would classify PGE-001, PGE-002, and PGE-003 missing; `blocked.rs:141-145` would refuse every `hold`; and the `ticket`-field problem has never actually been exercised in this repository. **(c)** — the gap between the specified loop and the runbook that exists is the reason the per-ticket-gate question is still open rather than settled by use.

**Per-worktree git cost.** `tools/rtm.ps1:84-89` builds and resolves the Engine under `$root/target/`, and `.gitignore` excludes `target`. Each worktree gets a fresh compile and pins its own Engine hash into its own untracked `.arca/evidence.toml` (`pin.rs:161-163`, `rtm.ps1:104`). A pin mismatch is a refusal, not a warning (`rtm.ps1:107-112`). **(a)**

---

## Summary of the three-way split

**(a) Enforced by code — these must be changed, not merely re-decided:**

1. `scheduler.rs:242` — `start` refuses while `.arca/state.toml` exists. *The* singleton.
2. `machine.rs:76`, `machine.rs:537-542` — `sensitivity_receipts` and `completion_gate` demand a literal `ticket` string in the runbook.
3. `graph.rs:50-59`, `graph.rs:131-136` — transitions carry no condition; the router takes the first edge. One successor per phase.
4. `scheduler.rs:757-779`, `doctor.rs:288-311` — exactly one initial phase, refused twice.
5. `state.rs:16-24`, `state.rs:77-83` — the state file has exactly seven fields; an eighth is a parse refusal. No run identifier can be added without a format change.
6. Twelve hard-coded `.arca/*` paths, all `root.join(<literal>)` — instance identity *is* root identity.
7. `contract.rs:79-201`, `contract.rs:204-361` — the contract gates are whole-project by construction and take no ticket.
8. `completion.rs:195-241`, `completion.rs:498-504` — receipt freshness is a whole-tree hash; concurrent edits under one root invalidate each other's receipts.
9. `goal.rs:22-39`, `scheduler.rs:329-340` — the goal revision is a whole-directory hash re-checked on every step.
10. `contract.rs:367-381` — a requirement whose gate kind the runbook does not declare is classified missing, so per-instance runbooks change what is provable.

**(b) Written rule with no enforcement — these can be re-decided by editing prose, but three of them are pivot-grade:**

1. `.arca/steering.md:75` — "Not multi-tenant: one repository, one Run at a time, local disk." A non-goal in the file that "changes **first**" on a pivot.
2. `.arca/goal/spec.md:94` (ORS-001) and `.arca/goal/spec.md:14` (R-008) — "a Subagent never invokes any `rtm` command." The engine holds no caller identity by explicit requirement, so this is unenforceable by construction, and a subtask that owns an instance must step it.
3. `.arca/goal/design.md:23,27` (ADR-0003) — the decided alternative: "Ticket→worktree parallelism stays inside a Phase; Exit Guards check the merged result", chosen because otherwise "the Machine leaks downward into every worker". Carries its own escape clause: "the policy is a documented rule, not enforced code (revisit if violated in practice)."
4. `.arca/runbook-spec.md:9` and `.arca/goal/ubi-lang.md:9` — the runbook is "one per project". The read-only half is recorded as having no enforcer at `.arca/runbook-spec.md:77`.
5. `.arca/goal/design.md:80` (ADR-0009) — a Phase Prompt carries "never the flowchart, never other Phases", so no instance can be told about a sibling.
6. `.arca/goal/design.md:88` (ADR-0010) and `.arca/issue/deferred/i-015-cycle-as-runbook/design.md:126` — no process management, no spawn flag, no scheduling.
7. The reviewable-snapshot rule in `.arca/schema.md` requiring everything under `.arca/` to be tracked or staged — violated by the engine's own runtime files.
8. `doctor.rs:340-351` (`RB205`) discouraging more than one terminal phase — a warning, not a refusal.
9. `.arca/index.md:68` — the sole-writer invariant is "a rule the Engine keeps, not a rule it enforces." Today's shared-Run safety rests on this.

**Written rules that pre-authorize the design rather than block it:**

1. `.arca/goal/spec.md:27` (R-021) — "The data model allows N Runs; nothing in formats or engine assumes a singleton." Now false of the format as built (`state.rs:16-24`).
2. `.arca/goal/design.md:61` (ADR-0007) — "Lifting the limit is additive ... The Run identity scheme is deferred until the limit lifts (YAGNI)."
3. `.arca/goal/design.md:74` (ADR-0008) — "when ADR-0007's limit lifts, per-Run files move under a runs directory; the v1 flat layout is the one-active-Run projection of that."
4. `.arca/goal/spec.md:20` (R-014) and `.arca/goal/ubi-lang.md:15-16` — the State File and Transition Log are already described as **per-Run**.
5. `.arca/steering.md:98-103` — routing is an explicitly open question, in a section that "binds nothing".
6. `.arca/issue/deferred/i-015-cycle-as-runbook/design.md:18-20` — "Run per ticket" is recorded as a live option with no rejection marker, while (a) and (d) carry one.

**(c) Accident of implementation — nobody chose these:**

1. `Runs` (`model.rs:168`) is constructed only in `t011_r021.rs:37`. R-021's "the data model allows N Runs" rests on a type with no production consumer.
2. `Run`, `RunArtifacts`, and `MachineState` are effectively dead — the per-instance handle that already exists is unused.
3. `RunState.status` never leaves `planned`; `Executing`, `Passed`, and `Failed` are written by nothing.
4. `input_revision`, `output_revision`, and `active_refs` are never written — the last is the natural carrier for per-instance ticket binding and is already noticed in `.arca/dict.md:62`.
5. `hold` and `abandon` take no lock, and `abandon` unlinks a lock a live `step` may hold.
6. `sensitivity_receipts` and `completion_gate` bypass the `guarded_target` path-escape check that `files_exact` and `file_contains` enforce.
7. Root resolution is bare `current_dir()` — worktree isolation is total and unintended, and `rtm` cannot run from a subdirectory.
8. `.arca/state.toml`, `.arca/rtm.lock`, `.arca/evidence.toml`, and `.arca/evidence/` are neither tracked nor ignored.
9. The shipped runbook exercises none of the ticket-bound or contract guard kinds, so the central obstacle has never been hit in practice.
10. `doctor.rs` warns on a duplicate edge but not on two different destinations from one phase — the case that silently discards a route.
