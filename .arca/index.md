# ratmac

ratmac (`rtm`) is a Rust engine that runs agent work as an explicit state
machine. Its shared runtime root is the primary checkout's `.ratmac/`; the Machine Class
(`.ratmac/ratmac.toml`, plain TOML) is read from the invoking checkout and declares States,
prompts, guards, and transitions; the Engine instantiates it into a Run and is the only writer
of run state. Progress is proven by machine-checked guards over artifacts on disk - never by an
agent's claim. Deterministic and offline: no network, no installs, no hidden global state.

Consult this file first, every time: the map below orients you; the routes
table locates every file; then read the file itself. The map is orientation
only, **never evidence** - a residual may not cite it.

**Where are we?** `rtm status --run <id>` answers it. The **Plan-Build
Runbook** (`.ratmac/ratmac.toml`) is this project's Machine Class, so the
sprint's stage *is* the addressed Run's State: `intake`, `gap-check`,
`cut-tickets`, `ticket-turns`, `close`, `rest`. A sprint has one entry and one
end, so one Run answers for it from start to rest. With no argument, `rtm
status` prints the roster.

**No-live-Run fallback.** Only while the roster is empty - the window between
sprints - derive the stage from the tree, in order: open tickets in
`.arca/ticket/` -> P4/P5 (building); `missing|partial` residuals without
tickets -> P3; goal frozen but residuals stale -> P2; a `pending` issue bundle
directly under `.arca/issue/` -> P1; none of the above -> Idle. Bundles in
`.arca/issue/deferred/` are live waiting work but do not force P1. Selecting
one visibly moves that same complete bundle to the intake work area and
changes its status to `pending`. This derivation is the no-live-Run fallback
and never competes with a live Run: while one exists, its Run Record is the
only answer.

## Map - how ratmac hangs together

Stamped cache - describes the tree through the expiry-aware turn-close ticket
(`t-111`), surveyed 2026-09-18. The full doctor executable fingerprint first
landed in the full-doctor-fingerprint ticket (`t-070`, `3e6ee6c`). The accepted
carrier remains `.arca/issue/archive/i-023-doctor-full-fingerprint/`; `DFP-001`
widens only the argument-free human doctor's rendered Engine SHA-256 from a
16-character prefix to the complete 64-character lowercase digest. That
landing's public proof is in `test/qa/tests/t045_bootstrap_doctor.rs`, and
`test-hidden/t-058/` through `test-hidden/t-070/` held its hidden lanes. The
lane-recovery cycle and expiry-aware turn-close ticket added no `src/`
changes. Every Architecture, Binary, Modules, and Tests row below describes
this landed tree. Refresh at each cycle close (gap check green).

The state-vocabulary cutover (`SVC-001`-`SVC-010`, goal `ADR-0012`) has landed:
every row below reads State for the machine position, `states` for the runbook
table, and `run.toml` for the Run Record.

### Architecture

```mermaid
flowchart LR
    CLI["rtm CLI<br/>cli.rs: mint or address a Run"] --> SCH["scheduler.rs<br/>open/open_run/start/step/status"]
    RB[".ratmac/ratmac.toml<br/>runbook - plain TOML data"] --> MC["machine.rs<br/>the one reader<br/>typed guards + input contracts"]
    MC --> SCH
    SCH <--> ST["state.rs + model.rs<br/>.ratmac/runs/&lt;run-id&gt;/run.toml<br/>strict seven-field Run Record"]
    SCH --> PIN["pin.rs<br/>Run evidence + hash-only runbook pin"]
    SCH --> VER["verdict.rs<br/>live input -> immutable Run-local archive"]
    SCH --> LED["ledger.rs<br/>spawn ledger - append/annotate only"]
    SCH --> G{{guard dispatch}}
    G --> PIN
    G --> REC["receipt.rs<br/>sensitivity_receipts"]
    G --> COM["completion.rs<br/>completion_gate"]
    G --> CON["contract.rs<br/>intake / addressed-record contracts"]
    G --> GOL["goal.rs<br/>goal freeze + drift"]
    CLI --> HLD["blocked.rs<br/>addressed confirmed hold"]
    HLD --> SCH
    CLI --> ABN["abandon.rs<br/>confirmed Run retirement"]
    ABN --> ST
    GOL --- GB[".arca/goal/<br/>frozen goal bundle"]
    CLI --> DOC["doctor.rs<br/>RB* findings as data"]
    CLI --> SCA["scaffold.rs<br/>one clean runbook to start from"]
    DOC --> MC
    DOC --> OWN["ownership.rs<br/>PGE-004 lint"]
```

### Binary

`rtm` - hand-rolled CLI (`src/cli.rs`, no clap): `start`, `status`, `step`,
`hold`, `abandon`, `spawn`, `respawn`, `doctor`, `scaffold`, `skill`. `start`
takes no Run id and mints the next roster member; `status` and `step` require a
canonical exact `--run <id>`, `hold` binds through the same `open_run`
preflight, and retiring a live Run requires its roster id. `spawn` creates a
declared child from a parent's spawning State as ordinary checked motion;
`respawn` and live-run `abandon` demand `--confirm` phrases naming the run id
(FDC-007). Only leftover-lock retirement may be unaddressed; missing or unknown
addresses report the resolved Engine root's `.ratmac/runs/` roster. `doctor` is
read-only and deep: its argument-free human report identifies the exact running
Engine with the complete 64-character lowercase SHA-256, its build channel and
source commit, the local stable-edition resolution or refusal, off-pin live
Runs, and the resolved Engine root before the parse, graph, guard-lint, and
ownership findings over `MachineClass`. `--json` emits an object containing
`engine_root` and the `findings` array, with exit `0`/`1`/`2` for clean,
warnings, and errors. `rtm doctor <path>` diagnoses any runbook file - an
unreadable path is finding `RB101`, not a usage error. `rtm scaffold <path>`
writes the one runbook that starts clean. `rtm skill <path>` atomically writes
the identity-stamped operator-skill folder at a new path and never overwrites.

### Modules (src/)

| Module | Role |
| :--- | :--- |
| `cli.rs` | Hand-rolled parsing of ten verbs and exit codes. It dispatches the addressed lifecycle commands plus `doctor`, `scaffold`, and `skill`; argument-free human `doctor` renders the executable's full SHA-256, channel/source provenance, stable-edition resolution, and resolved Engine root, while JSON carries `engine_root` and `findings`. It contains no graph or guard policy. |
| `graph.rs` | `State`, `Transition`, `MachineGraph` - graph position without lifecycle. `transition_for_input` selects the unique ordinary edge whose optional `input` exactly matches; `None` selects an unlabelled straight edge. `has_ordinary_outgoing` is the one structural terminal predicate (blocked routes excluded). Declaration order and guards never select, and blocked routes remain hold-only. |
| `machine.rs` | `MachineClass::from_toml` - the whole runbook schema boundary and its only reader, hand-rolled over `toml::Value`. It retains the top-level named `roots`, typed `GuardKind`, closed State `inputs`, Transition `input`, inline `classes`, per-State `spawns`, and the `join` guard; malformed declarations are rejected by their stable `RB*` codes. |
| `roots.rs` | Parses and validates the Machine Class's optional `[roots]` role-to-repository-relative-path declarations, rejecting malformed, missing, escaping, or Engine-overlapping roots before lifecycle use. |
| `scheduler.rs` | Project/Run binding and ordinary execution. `open` has no Run; `open_run` binds one canonical live roster member. `open`/`open_run` and `start` refuse flat residue, while pinned reads reject runbook drift. `start` mints an uncapped never-reused id and writes `passed` when the initial State is terminal; `step` refuses a passed Run by name, evaluates guards before verdict routing/consumption, and writes `passed` beside a terminal successor in one replacement; `status` reloads and reports read-only. `resolve_state_scope` reads a child Run through its own class's view for step and status alike (FDC-010/FDC-011); `spawn` mints a declared child as an ordinary flat Run and appends its ledger entry, refusing any parent that is itself a recorded child (FDC-012); `respawn` supersedes by confirmed phrase; the `join` guard reads the ledger's live children's terminal facts. |
| `ledger.rs` | The Scheduler-owned per-run spawn ledger (FDC-011): append at spawn, successor entries at respawn, abandoned-mark flips at retirement - never rewritten; strict read refuses malformed entries by name. |
| `verdict.rs` | Strict Run-local transition-input delivery. A branch validates exact `state`/`input`/`rationale`; a straight State requires an absent live slot. Valid bytes rename to monotonic `verdicts/NNNNNN.toml` evidence before Run Record advance; refusals consume nothing. |
| `model.rs` | `Run`, `RunArtifacts`, plural `Runs`, and serde `RunState`/`Status`; persisted state belongs to `.ratmac/runs/<run-id>/run.toml`. |
| `state.rs` | Strictly parses and atomically replaces the addressed `.ratmac/runs/<run-id>/run.toml`. The write path is crate-private, centralizing Engine writes without filesystem-enforcing ownership, and it renders the report behind `rtm status`. |
| `root.rs` | Resolves the invoking checkout and the shared Engine runtime root: a linked worktree uses the primary checkout's `.ratmac/`, while runbook bytes remain in the invoking checkout; no usable Git falls back locally. It owns the one displayed path spelling. |
| `pin.rs` | Run evidence: Engine path and SHA-256 plus build channel and source-commit provenance, gate-artifact pins, goal baseline/freeze, and the hash-only SHA-256 pin of canonical `.ratmac/ratmac.toml`. A path, digest, channel, or provenance mismatch refuses; non-exempt command guards run pinned code. |
| `channel.rs` | Offline Engine-channel resolution. `stable` is the newest edition recorded in `.arca/editions.md` only when its tag still names the recorded commit; `nightly` is the current `HEAD`. It also reports live Runs whose pinned Engine provenance is off stable. |
| `receipt.rs` | Sensitivity receipts; digests re-derived, self-verifying. |
| `completion.rs` | Completion gate: green + fresh via tree digest. |
| `contract.rs` | Intake/record contract gates. They resolve required `goal`, `issue`, `residual`, and `ticket` roles, plus optional `authority`, from the runbook's validated `[roots]` table before reading records; intake spans intake, deferred, and archive as one issue-id namespace, and the record gate receives the addressed Run id for frozen-goal evidence. |
| `goal.rs` | Goal freeze and drift check (content hash of `.arca/goal/`). |
| `blocked.rs` | Plans and applies an always-addressed human-confirmed hold: ticket/blocker checks, `open_run` residue/pin preflight, declared blocked route, then all-or-none named-Run state, history, and ticket updates. A passed Run refuses the hold before any route lookup (FDC-002). |
| `abandon.rs` | Human-confirmed retirement. A live Run requires `--run`; class/pin/residue checks are intentionally bypassed so broken Runs remain retireable. One terminal event naming the addressed Run durably precedes retirement of that Run's state/evidence plus any leftover lock, all-or-none; its directory remains to reserve the id. A ledger-recorded child's confirmed retirement also flips its entry's abandoned mark. |
| `ownership.rs` | PGE-004 ownership lint over prompts and guard contracts; the doctor's fourth pass. |
| `doctor.rs` | Findings as data: stable code, severity, location, and message. Diagnosis goes through `machine.rs`, owns graph, guard-lint, and cycle-termination passes, pairs findings with the exact resolved Engine root, renders JSON as `{ engine_root, findings }`, and maps clean/warning/error to exit `0`/`1`/`2`. |
| `scaffold.rs` | AAL-002: the smallest doctor-clean runbook, written at a path that does not exist yet. One file, no options, never overwrites. |
| `skill.rs` | AOP-003/AOP-004: atomically writes one non-overwriting `ratmac-operator` skill folder whose `SKILL.md` carries the writing Engine's identity stamp and whose references teach invariant operating rules. |

### Runbook shape (`.ratmac/ratmac.toml`)

Defined once, in [runbook-spec.md](runbook-spec.md): top level, State and
transition fields, the closed guard-kind vocabulary with each kind's required
and optional fields, the ownership rules, and the `RB*` diagnostic codes. This
map deliberately keeps no copy - a second copy would be a second schema.

Goal drift and per-Run runbook-pin verification are implicit Engine checks,
not runbook guard kinds; verdict validation is input routing, not guard
dispatch. No dedicated git-state guard exists; only `command_exit` can invoke
an external program to inspect repository state.

### Tests

`test/qa/` is the public integration-test crate, with ticket suites through
`t111_turn_close_expiry`. The established FDC coverage remains in
`t059_run_residency` through `t069_child_reviewer`; later coverage includes
named workflow roots and resolved-root reporting (`t076`, `t078`), edition
guards, audit, channels, and stable bootstrap (`t094`, `t095`, `t101`, `t102`),
the self-describing CLI and operator skill (`t103`, `t104`), and the cycle-close
contracts (`t105` through `t111`), including root-once expiry-aware verification
and evidence-guarded final-only retry (`t111`). DFP-001 remains proven by
`t045_bootstrap_doctor::doctor_reports_complete_engine_fingerprint_and_is_write_free`
plus the inherited `t045` and `t057` suites. Wording surfaces are asserted
against `.arca/schema.md` and `AGENTS.md`. `.ratmac/lanes.toml` declares every
landed hidden crate from `test-hidden/t-058/` through `test-hidden/t-111/`;
`t-078` and `t-079` retain their dated `edition-001` expiry markers. Opt-in
release lane: `RATMAC_RELEASE_ACCEPTANCE=1`.

### Known limitations / deferred debt (steering.md)

- Findings carry a location (`state "build" guard 0`), never a line or span:
  an agent repairs by name, not by cursor position.
- The zero-project-knowledge requirement (`R-016`) has narrower debt in these
  modules: `contract.rs` still fixes this project's workflow role names and
  record layout beneath them, though their directories resolve through the
  runbook's `[roots]`; `blocked.rs` ranges over every declared root and
  `goal.rs` hashes an already-resolved path. None hard-codes an `.arca`
  workflow root.

## Read next

| You want | Read |
| :--- | :--- |
| Where we are heading; the lines no work may cross | [steering.md](steering.md) |
| How to contribute: loop, tickets, evidence - **binding** | [schema.md](schema.md) |
| What happened lately | [log.md](log.md) tail |
| Unchosen ideas parked with zero commitment | [Wishlist](wishlist.md) |

## Where things live

All agent routing and documentation must use these paths.

| Path | What lives there |
| :--- | :--- |
| `.arca/steering.md` | Direction and guardrails: thesis, invariants, non-goals; first re-aligned on a pivot. |
| `.arca/schema.md` | The working rules - binding for every contributor. |
| `.arca/runbook-spec.md` | What a runbook **is** - the one definition of the Machine Class format, guard kinds, ownership, and `RB*` diagnostics. |
| `.arca/runbook-authoring.md` | How to write one - scaffold, edit, `rtm doctor --json`, repair by code. Procedure only; every schema fact is a link into the specification. |
| `.arca/dict.md` | Glossary - plain-word definitions; consult before coining a term, add an entry when introducing one. |
| `.arca/wishlist.md` | Unordered wishes with zero commitment; the Advisor appends its own observations here (schema.md, "The wishlist"), and only a human promotes one into planning. |
| `.arca/goal/` | The goal bundle now in force (`spec.md` > `design.md` > `test-list.md`, plus `ubi-lang.md`, `index.md`). Frozen per Run. |
| `.arca/issue/<issue-id>/` | Intake work area for a newly created or explicitly selected issue; one exact five-file bundle (shape: schema.md, "The issue folder"). |
| `.arca/issue/deferred/<issue-id>/` | Deferred issue buffer: the live waiting location for that same five-file bundle when any `spec.md` ask is `deferred`; `index.md` status mirrors it as `deferred`. |
| `.arca/issue/archive/<issue-id>/` | Completed issue history: the same five-file shape, no `deferred` ask, and `index.md` status `rejected` or `integrated`; an integrated bundle has at least one accepted-or-duplicate ask, and duplicate-only integration adds no new goal row. |
| `.arca/residual/` | Gap records, one per requirement - proven yet? |
| `.arca/ticket/` | Small self-contained work units, cut from gap records. |
| `.ratmac/` | Shared Engine runtime root at the primary checkout: Git-ignored `runs/`, `mint.toml`, `locks/`, and `log.md`; the invoking checkout's Machine Class `ratmac.toml` and receipts under `evidence/<run-id>/` stay tracked. |
| `.ratmac/runs/<run-id>/run.toml` | Per-Run Run Record - written ONLY by `rtm`; everyone else reads. |
| `.arca/editions.md` | The committed record of what each edition marks, one row per `edition-NNN`; the edition audit compares it against the tag database. |
| `.arca/log.md` | Human-only append-only history; every human landing leaves a line. `rtm` logs transitions in `.ratmac/log.md`. |
| `.arca/tpl/` | Blank forms; a form filled in at its proper path is the real thing. |
| `.arca/vis/` | Shared pictures and graphs. |
| `test-hidden/` | Hidden test code, out of git, listed by its owning ticket. |
| `test/` | The runnable suite plus `test/test-list.md`. |
| `src/` | The Engine - mapped above. |

## Issue movement and reviewable history

The three issue locations form one issue-id namespace, with issue numbers
unique across all three. P1 works only `pending` bundles in the intake work
area. A deferred issue stays whole in the live buffer; selecting it moves that
same bundle and issue id to intake, sets its status to `pending`, and carries
the required live-link rewrites with it. Waiting in the buffer alone never
puts the tree in P1.

A completed bundle with no deferred ask may move whole into archive when its
status is `rejected`, or when it is `integrated` with at least one
accepted-or-duplicate ask; duplicate-only integration adds no new goal row.
The move preserves identity, shape, and content except required relative-link
rewrites. If a parsed archived `spec.md` contains any `deferred`
disposition, the correction restores that exact complete bundle to
`deferred/`, changes `index.md` status to `deferred`, and retargets the live
inbound and outbound links without minting a replacement or second carrier.
Links inside already archived records are frozen provenance, including inbound
links to the restored issue, and stay byte-for-byte unchanged.

Acceptance and merge-gate evidence is reviewable only when every file under
its declared roots is tracked or staged, or is enumerated as an explicit
exception. The evidence stores a manifest of path, tracking state, and SHA-256
beside the claim. The binding archive, restoration, and snapshot rules are in
[schema.md](schema.md#evidence-and-archive-rules).

## Bootstrap

    pwsh -File tools/rtm.ps1   # resolve (or build) and pin-check the Engine
    rtm doctor                 # orient: engine identity, runbook, run state

Details, caller policy, and everything binding: [schema.md](schema.md).
