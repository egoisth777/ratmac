# Runbook specification

This file defines what a runbook **is**. It is the single authority for the
Machine Class format: the parser implements it, `rtm doctor` checks against it,
and the authoring instructions link to it. Nothing else in this repository
defines a runbook key, a guard kind, or a diagnostic code — where another
document needs one, it cites this one (RBS-004).

A runbook is plain TOML data at `.arca/ratmac.toml`, one per project,
human-reviewed and read-only at runtime (R-010, R-013). It declares Phases and
transitions and nothing else: `status` is runtime lifecycle the Scheduler owns
and may not appear anywhere in the file (R-002, R-003). Parsing is strict —
an unknown key is a hard error, never an ignored one (R-011).

## Top level

| Key | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `phases` | table of tables | yes | One entry per Phase, keyed by Phase name. A name may not be empty. |
| `transitions` | array of tables | no | The directed edges between Phases. Absent means a single-Phase machine with no edges. |
| `classes` | table of tables | no | FDC-009: one entry per declared child Machine Class, keyed by class name. See "Classes and spawns". |

Any other top-level key is `RB103`. A top-level `status` key is `RB104`.

## Phases

Each `[phases.<name>]` table declares one Phase.

| Field | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `prompt` | string | yes | Short prose stating the Phase's intent. The Scheduler renders the Phase Prompt as this prose plus generated Exit Guards and, for a branching Phase, its legal input values (R-028, FDC-001). It is the only machine information an agent receives (R-029). |
| `inputs` | array of strings | branching Phase only | The closed, non-empty, unique, ordered set of exact transition-input values. Required when the Phase has more than one ordinary outgoing edge; forbidden for a straight line or terminal. |
| `guards` | array of tables | no | The Phase's Exit Guards, evaluated in declaration order at `rtm step`. Absent or empty means the Phase may be left unconditionally. Guards are readiness checks, never route selectors. |
| `spawns` | array of tables | no | FDC-009: the child Runs this Phase may create, one entry per child. Only top-level Phases accept it. See "Classes and spawns". |

Comments in the file carry authoring intent and never reach an agent (R-012).

## Transitions

Each `[[transitions]]` table declares one directed edge.

| Field | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `from` | string | yes | Source Phase name. It must name a declared Phase, or the parse refuses (`RB108`). |
| `to` | string | yes | Destination Phase name, likewise declared. |
| `input` | string | branching ordinary edge only | The exact value selecting this edge. Every value in the source Phase's `inputs` list labels exactly one ordinary edge, and every ordinary edge from that Phase carries one listed value. Forbidden on straight-line and blocked routes. |
| `freeze` | string | no | The only accepted value is `"goal"`, marking the intake-completion boundary at which the goal revision is frozen (ETB-003). Any other value is `RB109`. |
| `blocked-route` | boolean | no | `true` marks a human-authorized escape (PGE-006). `rtm step` never takes a blocked route, a blocked route carries no `input`, and it does not participate in input coverage or initial-Phase reachability. |

The **initial Phase** is the one Phase with no inbound ordinary transition.
Zero such Phases is `RB202`; more than one is `RB203`.

A **terminal Phase** is one with no ordinary outgoing transition (blocked
routes do not count). Terminality is structural: nothing declares it.
Entering a terminal Phase completes the Run — `rtm start` beginning there and
`rtm step` arriving there write the Engine-owned status `passed` (FDC-002),
and a passed Run refuses further motion. Lifecycle status is Engine runtime
data and never a runbook key (R-002/R-003).

## Classes and spawns

FDC-009: one runbook can declare a composed machine. The declarations are
data; creating, joining, and superseding child Runs are Engine verbs governed
by FDC-007/FDC-011/FDC-012.

Each `[classes.<name>]` table declares one child Machine Class inline. A class
body is a whole machine under the same rules as the top level - a `phases`
table and `transitions` array validated identically (the shared codes apply,
locations prefixed `class "<name>"`) - plus one extra key:

| Field | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `phases` | table of tables | yes | The child machine's Phases, same rules as top level. |
| `transitions` | array of tables | no | The child machine's edges, same rules as top level. |
| `bindings` | table of tables | no | One entry per binding name the class declares: `[classes.<c>.bindings.<name>]` with the single optional boolean field `required` (default `false`). |

A malformed `classes` shape is `RB501`; a malformed binding entry is `RB502`.
A class body accepts no `classes` key and its Phases accept no `spawns` -
the format itself is one level deep, the shape FDC-012 caps.

Each `[[phases.<name>.spawns]]` table declares one child this Phase may spawn:

| Field | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `class` | string | yes | A class declared in this runbook's `classes` table. An undeclared class is `RB504`. |
| `name` | string | yes | The child instance's name, unique within the Phase. |
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
| `files_exact` | The listing of a directory equals the declared entry set; with no entry set, only that the path exists. | `path` | `entries` (array of strings; `files` is an accepted alias and must agree when both appear) |
| `file_contains` | A substring is present in a file. | `path`, `contains` | none |
| `command_exit` | A spawned program's exit code equals `expected`. The child's stderr is captured and rendered as a bounded diagnostic on refusal (ETB-002). Unless `exempt`, the program must resolve to a pinnable regular file whose hash is recorded in Run evidence (ETB-001). | `program`, `expected` | `args` (array of strings), `exempt` (boolean; marks a toolchain probe that reads no project state) |
| `sensitivity_receipts` | Every planned test the named ticket declares has a sensitivity receipt under `.arca/evidence/` (PGE-003). | `ticket` | none |
| `completion_gate` | Every check the named ticket declares has a green, fresh completion receipt (PGE-005). | `ticket` | none |
| `intake_contract` | `.arca/issue/<issue-id>/` (intake), `.arca/issue/deferred/<issue-id>/`, and `.arca/issue/archive/<issue-id>/` form one issue-id namespace of exact five-file bundles. The guard parses each ask's exact `accepted\|rejected\|duplicate\|deferred` disposition from `spec.md`, never from status alone; enforces at the intake-completion boundary that the whole bundle is under `deferred/` with status `deferred` if and only if at least one ask remains deferred, while archived bundles are `integrated\|rejected` and contain no deferred ask; requires every accepted requirement ID verbatim in the goal; permits a duplicate ask to reuse the goal's existing representation without adding its proposed ID as a new row; requires every integrated bundle to have no deferred ask and at least one accepted-or-duplicate disposition; and resolves links from live intake and deferred bundles in both directions (PGE-001). | none | none |
| `join` | FDC-009/FDC-011: the composition join. Satisfied only when the spawn ledger's live children carry Engine-written terminal `passed` facts - at least `min` of them. Until a ledger records children, a join guard honestly refuses. `require` accepts only `"all_passed"`; any other value is `RB506`, as is `min` below 1. | `require` | `min` (integer >= 1; default 1) |
| `record_contract` | One residual per frozen requirement, evidence behind every `satisfied`, one owning ticket per gap, acyclic ticket dependencies, complete ticket sections (PGE-002). | none | none |

Guard evaluation never compiles or fetches project source (ETB-001), and a
failing guard refuses, reports, and leaves Phase and Status untouched (R-017).

## Ownership

Who may write what. A rule with a named enforcer is mechanically checked; a
rule marked prose-only is a convention this file states and no code enforces.
The Machine Class, history, and invocation lock are project-level; mutable
state and Run evidence are scoped to one named Run.

| Rule | Enforcer |
| :--- | :--- |
| The project Machine Class stays at `.arca/ratmac.toml`. It is human-reviewed and read-only during Run lifecycle commands; scaffolding may create a runbook only at a caller-selected path that does not exist. | prose-only for human review and runtime immutability; `scaffold::write_scaffold` is the real create-only scaffold writer |
| A Phase Prompt or guard contract never directs an agent to write `.arca/runs/<id>/state.toml`, `.arca/runs/<id>/evidence.toml`, `.arca/log.md`, or `.arca/rtm.lock` (PGE-004). | `ownership::audit_ownership` |
| Each Run's State File is `.arca/runs/<id>/state.toml`; it is Engine-owned and has no project-level alias. | `state::StateStore` |
| Project history stays at `.arca/log.md` and is Engine-owned. | `scheduler::Scheduler`, `blocked::apply_hold`, and `abandon::apply_abandon` are the lifecycle write paths |
| The transient invocation lock stays at `.arca/rtm.lock` and is Engine-owned. | `scheduler::InvocationLock`; `abandon::apply_abandon` retires a leftover lock through the confirmed retirement path |
| Run evidence is Scheduler-owned at `.arca/runs/<id>/evidence.toml`. | `pin::Evidence::write` resolves the file through `pin::evidence_path` |
| Agent-authored test receipts live under `.arca/evidence/<ticket>/`, separate from Run evidence. | `receipt::ticket_evidence_dir` |
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
| `RB108` | error | A transition endpoint names a Phase the runbook does not declare. |
| `RB109` | error | `freeze` carries a value other than `"goal"`. |
| `RB110` | error | A key carries a value of the wrong type. |
| `RB201` | error | The runbook declares no Phases. |
| `RB202` | error | No initial Phase: every Phase has an inbound ordinary transition. |
| `RB203` | error | Several initial Phases: more than one Phase has no inbound ordinary transition. |
| `RB204` | error | A Phase is unreachable from the initial Phase. |
| `RB205` | warning | The machine has more than one terminal Phase. One ending is the ordinary shape; several usually mean a missing edge. |
| `RB206` | warning | Two transitions declare the same edge. |
| `RB207` | warning | A transition leaves and enters the same Phase. |
| `RB208` | error | A Phase's `inputs` value is not a non-empty array of unique, non-empty strings. |
| `RB209` | error | A Phase has several ordinary outgoing edges but declares no closed `inputs` list. |
| `RB210` | error | At least one declared legal input has no ordinary outgoing edge. |
| `RB211` | error | More than one ordinary outgoing edge carries the same transition input. |
| `RB212` | error | An ordinary transition input is foreign to its Phase's list, labelled and unlabelled ordinary branches are mixed, or a terminal/straight-line Phase declares an input contract. |
| `RB213` | error | A blocked route declares `input`. |
| `RB301` | error | A `command_exit` guard is neither `exempt` nor resolvable to a pinnable regular file. |
| `RB302` | warning | A guard's verdict rests on agent-writable content. |
| `RB401` | error | A prompt or guard contract directs an agent to write a Scheduler-owned artifact. |
| `RB501` | error | The `classes` table is malformed: not a table, empty, an empty class name, or a class body that is not a table. |
| `RB502` | error | A class's `bindings` declaration is malformed. |
| `RB503` | error | A Phase's `spawns` declaration is malformed: not an array of tables, missing or empty `class`/`name`, a duplicate spawn name, or a `bind` value that is not an array of unique non-empty strings. |
| `RB504` | error | A spawn names a class the runbook does not declare. |
| `RB505` | error | A spawn's binding names do not cover the class's required set exactly, or name a binding the class does not declare. |
| `RB506` | error | A `join` guard carries a `require` value outside the closed vocabulary or a `min` below 1. |

## Back-references

This file formalizes decided behavior; it changes none of it. Each row names
the decision and the statement above that preserves it.

| Requirement | Preserved by |
| :--- | :--- |
| `R-002` | Top level: `status` is runtime lifecycle and appears nowhere in the file; `RB104`. |
| `R-003` | Top level: a runbook declares `phases` and `transitions` only. |
| `R-011` | Top level and Guard kinds: an unknown key is `RB103`, never ignored. |
| `R-028` | Phases: `prompt` is required, and the Phase Prompt is that prose plus the generated guard list. |
| `ETB-001` | Guard kinds: a non-`exempt` `command_exit` must resolve to a pinnable regular file; `RB301`. |
| `ETB-002` | Guard kinds: a failing `command_exit` refusal carries the bounded stderr diagnostic. |
| `ETB-003` | Transitions: `freeze = "goal"` marks the one recognised freeze boundary; `RB109`. |
| `PGE-003` | Guard kinds: `sensitivity_receipts` reads the ticket's planned-test receipts. |
| `PGE-005` | Guard kinds: `completion_gate` reads the ticket's completion receipts. |
| `PGE-006` | Transitions: `blocked-route = true` is the human-authorized escape `rtm step` never takes. |
| `FDC-001` | Phases and Transitions: a branch declares closed `inputs`, ordinary edges carry unique covering `input` values, straight lines remain unlabelled, and blocked routes remain outside selection; `RB208`–`RB213`. |
| `FDC-009` | Classes and spawns, Guard kinds: one runbook declares a composed machine - inline class bodies, per-Phase spawn tables, and the `join` guard kind; `RB501`–`RB506`. |
| `FDC-012` | Classes and spawns: a class body accepts no `classes` and its Phases no `spawns` - the format itself is one level deep. |
