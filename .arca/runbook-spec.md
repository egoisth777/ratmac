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

Any other top-level key is `RB103`. A top-level `status` key is `RB104`.

## Phases

Each `[phases.<name>]` table declares one Phase.

| Field | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `prompt` | string | yes | Short prose stating the Phase's intent. The Scheduler renders the Phase Prompt as this prose plus a generated list of the Phase's Exit Guards (R-028). It is the only machine information an agent receives (R-029). |
| `guards` | array of tables | no | The Phase's Exit Guards, evaluated in declaration order at `rtm step`. Absent or empty means the Phase may be left unconditionally. |

Comments in the file carry authoring intent and never reach an agent (R-012).

## Transitions

Each `[[transitions]]` table declares one directed edge.

| Field | Type | Required | Meaning |
| :--- | :--- | :--- | :--- |
| `from` | string | yes | Source Phase name. It must name a declared Phase, or the parse refuses (`RB108`). |
| `to` | string | yes | Destination Phase name, likewise declared. |
| `freeze` | string | no | The only accepted value is `"goal"`, marking the intake-completion boundary at which the goal revision is frozen (ETB-003). Any other value is `RB109`. |
| `blocked-route` | boolean | no | `true` marks a human-authorized escape (PGE-006). `rtm step` never takes a blocked route, and a blocked route never makes its destination reachable for initial-Phase selection. |

The **initial Phase** is the one Phase with no inbound ordinary transition.
Zero such Phases is `RB202`; more than one is `RB203`.

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
| `intake_contract` | Issue-folder shape, statuses, accepted requirement IDs present in the goal, and forward/reverse links (PGE-001). | none | none |
| `record_contract` | One residual per frozen requirement, evidence behind every `satisfied`, one owning ticket per gap, acyclic ticket dependencies, complete ticket sections (PGE-002). | none | none |

Guard evaluation never compiles or fetches project source (ETB-001), and a
failing guard refuses, reports, and leaves Phase and Status untouched (R-017).

## Ownership

Who may write what. A rule with a named enforcer is mechanically checked; a
rule marked prose-only is a convention this file states and no code enforces.

| Rule | Enforcer |
| :--- | :--- |
| `.arca/ratmac.toml` is human-authored and read-only at runtime; no `rtm` command writes it. | prose-only - no writer of the runbook exists in `src/`, so there is nothing to constrain yet |
| A Phase Prompt or guard contract never directs an agent to write `.arca/state.toml`, `.arca/log.md`, or `.arca/rtm.lock` (PGE-004). | `ownership::audit_ownership` |
| The Scheduler is the sole writer of `.arca/state.toml` and `.arca/log.md` (R-009). | `state::StateStore` |
| Run evidence (`.arca/evidence.toml`) is Scheduler-owned; agent-authored receipts live under `.arca/evidence/<ticket>/`. | `pin::evidence_path` for the Scheduler-owned file, `receipt::ticket_evidence_dir` for the agent-writable tree |
| A guard whose verdict rests on content the agent under test can write proves less than one that does not; declaring such a guard is allowed but reported. | prose-only until `rtm doctor` reports it as `RB302` |
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
| `RB301` | error | A `command_exit` guard is neither `exempt` nor resolvable to a pinnable regular file. |
| `RB302` | warning | A guard's verdict rests on agent-writable content. |
| `RB401` | error | A prompt or guard contract directs an agent to write a Scheduler-owned artifact. |

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
