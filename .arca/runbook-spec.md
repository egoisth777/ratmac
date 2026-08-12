# Runbook specification

This file defines what a runbook **is**. It is the single authority for the
Machine Class format: the parser implements it, `rtm doctor` checks against it,
and the authoring instructions link to it. Nothing else in this repository
defines a runbook key, a guard kind, or a diagnostic code — where another
document needs one, it cites this one (RBS-004).

A runbook is plain TOML data at `.ratmac/ratmac.toml` in the invoking checkout,
one per project, human-reviewed and read-only at runtime (R-010, R-013). It
declares its named workflow roots, States, transitions, and optional child
classes; `status` is runtime lifecycle the Scheduler owns and may not appear
anywhere in the file (R-002, R-003). Parsing is strict — an unknown key is a
hard error, never an ignored one (R-011).

## Top level

| Key | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `roots` | table | no | Named repository-relative workflow roots. Each key is a non-empty role name and each value is a non-empty relative path. |
| `states` | table of tables | yes | One entry per State, keyed by State name. A name may not be empty. |
| `transitions` | array of tables | no | The directed edges between States. Absent means a single-State machine with no edges. |
| `classes` | table of tables | no | FDC-009: one entry per declared child Machine Class, keyed by class name. See "Classes and spawns". |

Any other top-level key is `RB103`, except a pre-cutover `phases` table, which is `RB111` and takes precedence. A top-level `status` key is `RB104`.

## Roots

`[roots]` maps an authored role name to a repository-relative directory. A
path may not be absolute or escape the repository. Each declared directory
must exist when a lifecycle command or project doctor loads the runbook, and
may not equal, contain, or sit beneath the resolved Engine root. The parser
reports malformed declarations as `RB601`; an undeclared role named by a guard
is `RB602`; missing declared paths are `RB603`; Engine-root overlap is
`RB604`.

## States

Each `[states.<name>]` table declares one State.

| Field | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `prompt` | string | yes | Short prose stating the State's intent. The Scheduler renders the State Prompt as this prose plus generated Exit Guards and, for a branching State, its legal input values (R-028, FDC-001). It is the only machine information an agent receives (R-029). |
| `inputs` | array of strings | branching State only | The closed, non-empty, unique, ordered set of exact transition-input values. Required when the State has more than one ordinary outgoing edge; forbidden for a straight line or terminal. |
| `guards` | array of tables | no | The State's Exit Guards, evaluated in declaration order at `rtm step`. Absent or empty means the State may be left unconditionally. Guards are readiness checks, never route selectors. |
| `spawns` | array of tables | no | FDC-009: the child Runs this State may create, one entry per child. Only top-level States accept it. See "Classes and spawns". |

Comments in the file carry authoring intent and never reach an agent (R-012).

## Transitions

Each `[[transitions]]` table declares one directed edge.

| Field | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `from` | string | yes | Source State name. It must name a declared State, or the parse refuses (`RB108`). |
| `to` | string | yes | Destination State name, likewise declared. |
| `input` | string | branching ordinary edge only | The exact value selecting this edge. Every value in the source State's `inputs` list labels exactly one ordinary edge, and every ordinary edge from that State carries one listed value. Forbidden on straight-line and blocked routes. |
| `freeze` | string | no | The only accepted value is `"goal"`, marking the intake-completion boundary at which the goal revision is frozen (ETB-003). Any other value is `RB109`. |
| `blocked-route` | boolean | no | `true` marks a human-authorized escape (PGE-006). `rtm step` never takes a blocked route, a blocked route carries no `input`, and it does not participate in input coverage or initial-State reachability. |

The **initial State** is the one State with no inbound ordinary transition.
Zero such States is `RB202`; more than one is `RB203`.

A **terminal State** is one with no ordinary outgoing transition (blocked
routes do not count). Terminality is structural: nothing declares it.
Entering a terminal State completes the Run — `rtm start` beginning there and
`rtm step` arriving there write the Engine-owned status `passed` (FDC-002),
and a passed Run refuses further motion. Lifecycle status is Engine runtime
data and never a runbook key (R-002/R-003).

## Classes and spawns

FDC-009: one runbook can declare a composed machine. The declarations are
data; creating, joining, and superseding child Runs are Engine verbs governed
by FDC-007/FDC-011/FDC-012.

Each `[classes.<name>]` table declares one child Machine Class inline. A class
body is a whole machine under the same rules as the top level - a `states`
table and `transitions` array validated identically (the shared codes apply,
locations prefixed `class "<name>"`) - plus one extra key:

| Field | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `states` | table of tables | yes | The child machine's States, same rules as top level; its table path is `[classes.<name>.states]`. |
| `transitions` | array of tables | no | The child machine's edges, same rules as top level. |
| `bindings` | table of tables | no | One entry per binding name the class declares: `[classes.<c>.bindings.<name>]` with the single optional boolean field `required` (default `false`). |

A malformed `classes` shape is `RB501`; a malformed binding entry is `RB502`.
A class body accepts no `classes` key and its States accept no `spawns` -
the format itself is one level deep, the shape FDC-012 caps.

Each `[[states.<name>.spawns]]` table declares one child this State may spawn:

| Field | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `class` | string | yes | A class declared in this runbook's `classes` table. An undeclared class is `RB504`. |
| `name` | string | yes | The child instance's name, unique within the State. |
| `bind` | array of strings | no | The binding names the spawner supplies. They must cover the class's required set exactly and name nothing the class does not declare (`RB505`). |

A malformed spawn entry is `RB503`. Static validation proves the spawn table
names a declared class and its binding names equal the child class's required
set; binding values are supplied at spawn time, never in the runbook.

## Guard kinds

An Exit Guard is a predicate over artifacts on disk, never over an agent's
claim. `kind` is required on every guard and selects the row below; the closed
list is the whole vocabulary, and a `kind` outside it is `RB106`. A field that
is not listed for the selected kind is `RB107`; a missing required field is
`RB105`. `status` inside a guard table is `RB104`.

| Kind | Judges | Required fields | Optional fields |
| :--- | :--- | :--- | :--- |
| `files_exact` | The listing of a directory equals the declared entry set; with no entry set, only that the path exists. | `path` | `root` (declared role), `entries` (array of strings; `files` is an accepted alias and must agree when both appear) |
| `file_contains` | A substring is present in a file. | `path`, `contains` | `root` (declared role) |
| `command_exit` | A spawned program's exit code equals `expected`. The child's stderr is captured and rendered as a bounded diagnostic on refusal, or its stdout when stderr is silent - labelled `diagnostic (stdout)` so the reader knows which channel spoke (ETB-002). Unless `exempt`, the program must resolve to a pinnable regular file whose hash is recorded in Run evidence (ETB-001). | `program`, `expected` | `args` (array of strings), `exempt` (boolean; marks a toolchain probe that reads no project state) |
| `sensitivity_receipts` | Every planned test the addressed ticket declares has a sensitivity receipt under `.ratmac/evidence/` (PGE-003). | exactly one of `ticket`, `ticket-binding` | `root` (declared role) |
| `completion_gate` | Every check the addressed ticket declares has a green, fresh completion receipt (PGE-005). | exactly one of `ticket`, `ticket-binding` | `root` (declared role) |
| `intake_contract` | The fixed `goal` and `issue` roles provide the goal authority and intake/deferred/archive issue namespace. The guard parses each ask's exact `accepted\|rejected\|duplicate\|deferred` disposition from `spec.md`, never from status alone; enforces the deferred and archived bundle rules, accepted requirement IDs, and live links (PGE-001). | none | none |
| `join` | FDC-009/FDC-011: the composition join. Satisfied only when the spawn ledger's live children carry Engine-written terminal `passed` facts - at least `min` of them. Until a ledger records children, a join guard honestly refuses. `require` accepts only `"all_passed"`; any other value is `RB506`, as is `min` below 1. | `require` | `min` (integer >= 1; default 1) |
| `record_contract` | The fixed `goal`, `residual`, and `ticket` roles provide the records for one residual per frozen requirement, evidence behind every `satisfied`, one owning ticket per gap, acyclic ticket dependencies, and complete ticket sections (PGE-002). | none | none |

A per-item guard names the one item it judges in exactly one of two ways
(PCR-007). `ticket = "t-047"` writes the item in the runbook. `ticket-binding
= "item"` names a binding instead: the caller supplies its value when the
child Run is spawned, the value is recorded once in the parent's append-only
spawn ledger, and the Engine reads it back at dispatch. Declaring both forms,
neither, or an empty one is `RB112`. A bound address that no ledger entry
supplies refuses at dispatch, naming the guard kind, the binding, and the Run.

For `files_exact`, `file_contains`, `sensitivity_receipts`, and
`completion_gate`, an optional `root = "<role>"` resolves `path` or the
resolved item address beneath that declared root. Without `root`, the address remains relative to
the Run workspace. Contract guards have no path fields: `intake_contract`
requires `goal` and `issue`; `record_contract` requires `goal`, `residual`,
and `ticket`.

Guard evaluation never compiles or fetches project source (ETB-001), and a
failing guard refuses, reports, and leaves State and Status untouched (R-017).

## Ownership

Who may write what. A rule with a named enforcer is mechanically checked; a
rule marked prose-only is a convention this file states and no code enforces.
The Machine Class is tracked in each checkout; history, the root lock, the
per-Run locks, mutable state, and Run evidence live under the resolved Engine
root, which one repository shares across its worktrees.

| Rule | Enforcer |
| :--- | :--- |
| The project Machine Class stays at `.ratmac/ratmac.toml` in the invoking checkout. It is human-reviewed and read-only during Run lifecycle commands; scaffolding may create a runbook only at a caller-selected path that does not exist. | prose-only for human review and runtime immutability; `scaffold::write_scaffold` is the real create-only scaffold writer |
| A State Prompt or guard contract never directs an agent to write `.ratmac/runs/<id>/run.toml`, `.ratmac/runs/<id>/evidence.toml`, `.ratmac/mint.toml`, `.ratmac/log.md`, `.ratmac/locks/root.lock`, or `.ratmac/locks/runs/<id>.lock` (PGE-004). | `ownership::audit_ownership` |
| The Run Record for each Run is `.ratmac/runs/<id>/run.toml`; its position field is `state`; it is Engine-owned and has no project-level alias. | `state::StateStore` |
| Project history stays at `.ratmac/log.md` and is Engine-owned. | `scheduler::Scheduler`, `blocked::apply_hold`, and `abandon::apply_abandon` are the lifecycle write paths |
| The short root lock stays at `.ratmac/locks/root.lock`; it protects minting and shared roster, ledger, or ticket mutation, never guard evaluation. A read-modify-write of a ticket shared across Runs needs the root lock because separate Run locks cannot serialize that shared file. A holder proves ownership with a kernel claim on an open handle whose identity still matches the path, so a dead holder's lock frees itself and no process removes a lock it does not hold. An unheld lock pathname is transient residue that ordinary acquisition may reclaim; refusal byte-identity guarantees cover durable Run and shared artifacts, not lock residue. | `lock::RootLock`; `blocked::apply_hold`; `abandon::apply_abandon` removes a lock only while holding that claim |
| Each Run's motion lock stays at `.ratmac/locks/runs/<id>.lock`; it serializes motion on that addressed Run and is never a substitute for the root lock. Ownership rests on the same kernel claim and handle/path identity check; the token written inside the file is diagnostic text for refusals, never the basis of any decision. | `lock::RunLock`; `abandon::apply_abandon` removes a lock only while holding that claim |
| Run evidence is Scheduler-owned at `.ratmac/runs/<id>/evidence.toml`. | `pin::Evidence::write` resolves the file through `pin::evidence_path` |
| Agent-authored test receipts live under `.ratmac/evidence/<run-id>/`, separate from Run evidence. | `receipt::run_evidence_dir` |
| A guard whose verdict rests on content the agent under test can write proves less than one that does not; declaring such a guard is allowed but reported as `RB302`. | `doctor::lint_guards` |
| A runbook is reviewed by a human before it becomes the project's Machine Class. | prose-only |

## Diagnostics

`rtm doctor` reports one finding per defect: a stable code, a severity, a
location, and a message. Same defect, same code, across runs and releases. The
exit code is `0` with no findings, `1` when every finding is a warning, and `2`
when any finding is an error.

| Code | Severity | Defect |
| :--- | :--- | :--- |
| `RB101` | error | The runbook is absent or unreadable at the requested path. |
| `RB102` | error | The file is not valid TOML. |
| `RB103` | error | An unknown key appears where the schema declares a closed set. |
| `RB104` | error | `status` appears in the runbook; status is runtime, not schema. |
| `RB105` | error | A required field is missing. |
| `RB106` | error | A guard declares a kind outside the vocabulary. |
| `RB107` | error | A guard carries a field the selected kind does not accept. |
| `RB108` | error | A transition endpoint names a State the runbook does not declare. |
| `RB109` | error | `freeze` carries a value other than `"goal"`. |
| `RB110` | error | A key carries a value of the wrong type. |
| `RB111` | error | The runbook declares a pre-cutover `phases` table instead of `states`; the loader refuses before any further parse or run work, naming the runbook file and the repair (rename the table to `states`), and this refusal takes precedence over the generic unknown-key `RB103`. |
| `RB112` | error | A per-item guard (`sensitivity_receipts`, `completion_gate`) declares both address forms, neither, or an empty one: exactly one of `ticket` and `ticket-binding` names the item it judges. |
| `RB601` | error | The `roots` table is malformed: it is not a table, has an empty role, a non-string or empty path, an absolute path, or a path that lexically escapes the repository. |
| `RB602` | error | A guard names a root role the runbook does not declare, including a fixed contract role. |
| `RB603` | error | A declared root role names a path that does not exist or cannot be read when the runbook loads. |
| `RB604` | error | A declared root equals, contains, or sits beneath the resolved Engine root. |
| `RB201` | error | The runbook declares no States. |
| `RB202` | error | No initial State: every State has an inbound ordinary transition. |
| `RB203` | error | Several initial States: more than one State has no inbound ordinary transition. |
| `RB204` | error | A State is unreachable from the initial State. |
| `RB205` | warning | The machine has more than one terminal State. One ending is the ordinary shape; several usually mean a missing edge. |
| `RB206` | warning | Two transitions declare the same edge. |
| `RB207` | warning | A transition leaves and enters the same State. |
| `RB208` | error | A State's `inputs` value is not a non-empty array of unique, non-empty strings. |
| `RB209` | error | A State has several ordinary outgoing edges but declares no closed `inputs` list. |
| `RB210` | error | At least one declared legal input has no ordinary outgoing edge. |
| `RB211` | error | More than one ordinary outgoing edge carries the same transition input. |
| `RB212` | error | An ordinary transition input is foreign to its State's list, labelled and unlabelled ordinary branches are mixed, or a terminal/straight-line State declares an input contract. |
| `RB213` | error | A blocked route declares `input`. |
| `RB214` | error | A State on a cycle over ordinary edges carries no receipt- or contract-class guard, so nothing statically proves the cycle terminates. |
| `RB301` | error | A `command_exit` guard is neither `exempt` nor resolvable to a pinnable regular file. |
| `RB302` | warning | A guard's verdict rests on agent-writable content. |
| `RB401` | error | A prompt or guard contract directs an agent to write a Scheduler-owned artifact. |
| `RB501` | error | The `classes` table is malformed: not a table, empty, an empty class name, or a class body that is not a table. |
| `RB502` | error | A class's `bindings` declaration is malformed. |
| `RB503` | error | A State's `spawns` declaration is malformed: not an array of tables, missing or empty `class`/`name`, a duplicate spawn name, or a `bind` value that is not an array of unique non-empty strings. |
| `RB504` | error | A spawn names a class the runbook does not declare. |
| `RB505` | error | A spawn's binding names do not cover the class's required set exactly, or name a binding the class does not declare. |
| `RB506` | error | A `join` guard carries a `require` value outside the closed vocabulary or a `min` below 1. |

## Back-references

This file formalizes decided behavior; it changes none of it. Each row names
the decision and the statement above that preserves it.

| Requirement | Preserved by |
| :--- | :--- |
| `R-002` | Top level: `status` is runtime lifecycle and appears nowhere in the file; `RB104`. |
| `R-003` | Top level: a runbook declares `states` and `transitions` only. |
| `R-011` | Top level and Guard kinds: an unknown key is `RB103`, never ignored. |
| `R-028` | States: `prompt` is required, and the State Prompt is that prose plus the generated guard list. |
| `ETB-001` | Guard kinds: a non-`exempt` `command_exit` must resolve to a pinnable regular file; `RB301`. |
| `ETB-002` | Guard kinds: a failing `command_exit` refusal carries the bounded diagnostic from stderr, or from stdout as the declared fallback channel when stderr is silent. |
| `ETB-003` | Transitions: `freeze = "goal"` marks the one recognised freeze boundary; `RB109`. |
| `PGE-003` | Guard kinds: `sensitivity_receipts` reads the ticket's planned-test receipts. |
| `PGE-005` | Guard kinds: `completion_gate` reads the ticket's completion receipts. |
| `PGE-006` | Transitions: `blocked-route = true` is the human-authorized escape `rtm step` never takes. |
| `FDC-001` | States and Transitions: a branch declares closed `inputs`, ordinary edges carry unique covering `input` values, straight lines remain unlabelled, and blocked routes remain outside selection; `RB208`–`RB213`. |
| `FDC-008` | Guard kinds and Transitions: every State on an ordinary-edge cycle carries a receipt- (`sensitivity_receipts`, `completion_gate`) or contract-class (`intake_contract`, `record_contract`) guarded out-edge, checked by kind membership alone; `RB214`. |
| `FDC-009` | Classes and spawns, Guard kinds: one runbook declares a composed machine - inline class bodies, per-State spawn tables, and the `join` guard kind; `RB501`–`RB506`. |
| `FDC-012` | Classes and spawns: a class body accepts no `classes` and its States no `spawns` - the format itself is one level deep. |
| `SVC-003` | Top level, States, Classes and spawns, and Transitions: the top-level spelling is `states`; `[states.<name>]`, `[[states.<name>.spawns]]`, and `[classes.<name>.states]` are the State/class/spawn table paths; `from` and `to` name declared States. |
| `SVC-005` | Top level and Diagnostics: `RB111` refuses a pre-cutover `phases` table before further parse or run work, naming the runbook file and the repair (rename the table to `states`) rather than falling back to `RB103`. |
| `SVC-006` | Diagnostics: `RB111` is the one new code for the pre-cutover `phases` table; every other code keeps its exact identity while State wording changes. |
