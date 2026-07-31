# 05 — Invocation and Join Semantics

Wave-2 research, topic 2 of 3. Date: 2026-07-29. Scope: how one machine
instance spawns child instances and joins on them, replacing today's
single-scalar position. Builds on `03-formalisms.md` (recursive state
machines, workflow nets, statecharts) and `02-orchestration-prior-art.md`
(supervision trees, Temporal `ParentClosePolicy`, LangGraph Send, AWS
Parallel/Map); it does not repeat their surveys.

**Tier assumptions.** Per the established ladder — tier 0 conditional edges,
tier 1 terminating loops, tier 2 parameter binding, tier 3 fan-out/join —
this document designs tiers 2–3 assuming tiers 0–1 exist. Every dependency
on tier 0 or tier 1 is flagged inline as **[tier-0 dep]** / **[tier-1 dep]**.
The headline result: the fan-out/join design below needs tier 2 but does
**not** need tier 0 or tier 1 for its core mechanism, because a join is a
*gate on the unique successor*, not a branch — and the project's own cycle
self-hosts without tier 1 if the outer sprint loop remains "one Run per
sprint" (§7).

**Erratum & supersession note** (2026-07-29, ruling `AR-10` of the
doctrine-convergence issue, `i-016`): the §7 example's escape edge is
corrected in place, `blocked_route` → `blocked-route`, the canonical
spelling the parser, the specification, and the working rules share; and
the runbook-format extension §1 proposes — the class and spawn tables —
is hereby declared explicitly as superseding the format-spec restriction
(`RBS-004`) that `08-adversarial-review.md` cites.

**Supersession — child run and ledger residency (2026-07-29, ruling
`AR-01`/`AR-08` of the doctrine-convergence issue, `i-016`):** throughout
§1, §3, §4, and §7 below, a child run root is written as a path-shaped id
nested under the spawn name — `.arca/runs/tickets/t-042/` — and the spawn
ledger sits beside it as a top-level file in that same namespace —
`.arca/runs/tickets/ledger.toml` — with a child addressed as
`rtm step --run tickets/t-042`. That scheme is superseded. The adopted
canonical layout (the review's scheme, `08-adversarial-review.md` §2
AR-01, adopted as `08` §4 item 6; batch human sign-off, recorded in
design.md) makes every child an ordinary flat top-level run —
`.arca/runs/<child-run-id>/`, addressed as `--run <child-run-id>` like any
other run — and nests the spawn ledger under the *parent's own* run
directory instead of the registry: `.arca/runs/<parent-run-id>/spawn/
<spawn-name>/ledger.toml` (also settling the ledger-placement tension,
`AR-08`, `08` §4 item 14). A ledger entry now carries the child's run id
plus its binding map, rather than owning the child's directory. Settled
position: [04-run-identity.md](04-run-identity.md) §2.1's proposed layout;
design.md's "Adopted defaults" items 6 and 14 (their mechanics now bind
via the run-residency issue, `i-017`, and the machine-composition issue,
`i-018`, per design.md's own split pointer). The child-instance examples
below are otherwise unchanged as written and should be read through this
correction.

---

## 0. Today's baseline (facts, with citations)

- **Position is one scalar.** `MachineState { phase: Phase }` — no plural,
  no stack, no child set (src/graph.rs:152–156).
- **Transitions carry no condition.** `Transition { from, to, freezes_goal,
  blocked_route }` — four fields, nothing else (src/graph.rs:50–59).
  `transition_for` returns the *first* non-blocked edge leaving a phase
  (src/graph.rs:131–136), so ordinary routing is a function of the runbook
  and the current phase alone.
- **PGE-006 as written in code:** "`step` never takes it, so ordinary
  routing stays deterministic and never branches on any lifecycle field"
  (src/graph.rs:55–57).
- **Guards are a closed vocabulary of 7 kinds** (src/machine.rs:59–68), and
  the two ticket-scoped kinds bind one literal name: `SensitivityReceipts
  { ticket: String }` and `CompletionGate { ticket: String }`
  (src/machine.rs:47–52); `accepted_fields` maps both to `&["ticket"]`
  (src/machine.rs:76); the parser reads the literal with
  `field.string("ticket")?` (src/machine.rs:538, 541). A gate therefore
  binds ONE hard-coded ticket and cannot count N branches.
- **A guard may not mention `status`** — the parser rejects any guard table
  containing a `status` key (src/machine.rs:480–482); `RB104` in
  `.arca/runbook-spec.md:96`.
- **One Run per project root.** `rtm start` refuses if `.arca/state.toml`
  exists: "cannot start: an active Run already exists for this project"
  (src/scheduler.rs:242–247). Run artifacts are computed from a root:
  `state.toml`, `log.md`, `rtm.lock` under `<root>/.arca`
  (src/model.rs:86–93).
- **RunState is seven required fields:** `phase`, `status`, `goal_revision`,
  `input_revision`, `output_revision`, `active_refs`, `blocker`
  (src/model.rs:194–202). `active_refs` is initialized empty at start
  (src/scheduler.rs:255) and is the natural home for "open spawn" markers
  (§4).
- **Guards already read Scheduler-owned files inside `step`.** The goal-drift
  check loads `.arca/evidence.toml` and refuses on mismatch
  (src/scheduler.rs:327–341). This is the precedent §6 leans on: consulting
  an engine-written on-disk fact during gating is already doctrine.
- **A relational guard already exists.** `record_contract` quantifies over
  *all* residuals/tickets — "one owning ticket per gap, acyclic ticket
  dependencies" — without naming any single ticket
  (`.arca/runbook-spec.md:65`). The join guard of §3 is the same move aimed
  at child runs.
- **The runbook is plain data.** ADR-0004: TOML, unknown keys are hard
  errors (`.arca/goal/design.md:29–35`). ADR-0007: `ratmac.toml` is a pure
  template, read-only at runtime; `rtm start` instantiates a Run; a Run owns
  its State File, Transition Log, lockfile (`.arca/goal/design.md:41–43`).
  No interpolation, no template language — hard invariant honored throughout
  this proposal.
- **Status vocabulary:** `planned | executing | blocked | passed | failed`
  (src/model.rs:12–18). `blocked` is not terminal; `passed`/`failed` are
  terminal only *at a graph-terminal phase* (a phase with no ordinary
  outgoing transition) — §3 makes this precise.

---

## 1. The invoke edge

### Today

There is no invoke construct. The whole runbook is one anonymous machine
class: top-level `[phases.X]` + `[[transitions]]`
(`.arca/ratmac.toml`, whole file). Nothing can reference another class
because no second class can be declared.

### Proposed

Two additions to the runbook format, both plain data:

**(a) Named child classes** under a new top-level `[classes.<name>]` table.
The existing top-level `[phases]`/`[[transitions]]` remain the implicit
root class, so every current runbook parses unchanged (ADR-0004's
unknown-key rule means old engines refuse new runbooks loudly — correct
behavior).

**(b) A spawn declaration on a phase** — a `spawn` table, at most one per
phase (v-next restriction, §5):

```toml
[phases.p45-build.spawn]
name      = "tickets"     # ledger name; must be unique in the class
class     = "ticket"      # must resolve to [classes.ticket]  (RB501)
bind      = ["ticket"]    # binding NAMES the caller must supply at spawn time
workspace = "worktree"    # closed vocab: "worktree" | "shared"
```

Exact field semantics:

| field | type | meaning |
|---|---|---|
| `name` | string | Key of the spawn ledger under `.arca/runs/<name>/`. |
| `class` | string | Child machine class; must be declared in this runbook. |
| `bind` | array of strings | The binding *names* (never values) the spawner must supply. Must equal the child class's required binding set (RB505). |
| `workspace` | string | `"worktree"`: engine creates one git worktree per child (reusing the trial-worktree machinery of the TWL series); `"shared"`: child agents work in the project tree. |

**How a binding gets INTO the child without the runbook becoming a
program:** the runbook declares only the binding's *schema* (name, shape,
requiredness — §2). The *value* is chosen by the caller at spawn time and
recorded by the engine in the child's State File:

```
rtm spawn --spawn tickets --bind ticket=t-042
```

The engine, on `rtm spawn`:

1. verifies the parent Run's current phase declares spawn `tickets`
   (otherwise refuse — spawning is only legal while the parent sits in the
   spawning phase, which is also what freezes the expected set, §3);
2. validates each `--bind` name and value against the child class's binding
   schema (§2);
3. creates the child run root `.arca/runs/tickets/t-042/` with its own
   `state.toml`, `log.md`, `rtm.lock` (the `RunArtifacts::for_root` shape of
   src/model.rs:86–93, generalized: today it hard-codes
   `root.join(".arca")` at src/model.rs:87 and needs a second constructor
   for nested run directories);
4. appends an entry to the engine-owned ledger
   `.arca/runs/tickets/ledger.toml` (§3);
5. if `workspace = "worktree"`, creates the child's worktree and records its
   path in the ledger entry.

The runbook never contains a value, a loop, or a substitution site. The
spawn table is exactly as declarative as a transition: it names a class and
a schema, and the doctor can check every referenced name statically (§5).
The child never "receives" anything at runtime either — it *reads* its own
State File, where the engine wrote the binding once, at instantiation.

**Child stepping.** A child is addressed by run path:
`rtm step --run tickets/t-042` (or the engine resolves the enclosing run
root from the child worktree's cwd). Each child has its own `rtm.lock`, so
children step concurrently without touching the parent's lock
(`InvocationLock` is per run root — src/scheduler.rs:794–800).

---

## 2. Binding semantics

### Today

The only parameterization is the literal `ticket: String` field inside two
guard kinds (src/machine.rs:47–52, 76, 538, 541). Changing the ticket means
editing the runbook — i.e., the machine class is monomorphic: one class per
ticket value.

### What a binding is, formally

A binding is a named, write-once, engine-recorded value pair `name → value`
attached to a Run at instantiation, with:

- **declaration in data:** the class declares the binding's schema, never a
  value;
- **valuation at spawn:** the caller (human at `rtm start`, or the spawning
  agent at `rtm spawn`) supplies the value on the command line;
- **storage in run state:** the engine records the map in the child's State
  File; it is immutable for the Run's life (write-once, like
  `goal_revision`);
- **consumption by reference:** guards name the binding; the *engine*
  computes any derived path or argument. There is no substitution syntax
  anywhere in TOML.

### Proposed schema declaration (per class)

```toml
[classes.ticket.bindings.ticket]
required = true
shape    = "token"    # closed vocab: "token" (path-safe [a-z0-9-]+) — only value in v-next
doc      = "the ticket this instance drives"   # comment-grade prose, never rendered to agents
```

### Proposed guard reference

The two ticket-scoped kinds gain one accepted field, `binding`, mutually
exclusive with `ticket`:

```toml
# today (still legal — literal form is kept):
[[phases.t5-complete.guards]]
kind   = "completion_gate"
ticket = "t-042"

# proposed (parametric form):
[[classes.ticket.phases.t5-complete.guards]]
kind    = "completion_gate"
binding = "ticket"
```

Engine changes, precisely:

- `accepted_fields`: `"sensitivity_receipts" | "completion_gate" =>
  &["ticket", "binding"]` (src/machine.rs:76), with parse-time enforcement
  of *exactly one* of the two (new code `RB111`, extending the RB1xx family
  of `.arca/runbook-spec.md:93–102`).
- `GuardKind` variants change from `ticket: String` to a two-armed
  reference, e.g. `SensitivityReceipts { ticket: TicketRef }` with
  `enum TicketRef { Literal(String), Binding(String) }`.
- At evaluation, `Binding("ticket")` resolves through the Run's State File
  bindings map; the *resolved value* then flows into the existing path
  computation (`receipt::ticket_evidence_dir` — the agent-authored receipts
  stay under `.arca/evidence/<value>/` exactly as today,
  `.arca/runbook-spec.md:80`). No path string ever appears in the runbook;
  guards declare paths relative to a binding-derived root that the ENGINE
  computes in code. This is the answer to "no interpolation": substitution
  happens in Rust, on validated tokens, never in data.
- The `shape = "token"` check exists precisely so a binding value can be
  safely embedded in a path by the engine (no separators, no traversal).

### Proposed RunState change

`RunState` (src/model.rs:194–202) gains an eighth required field:

```toml
# child .arca/runs/tickets/t-042/state.toml (Scheduler-owned, engine-written)
phase           = "t4-tests"
status          = "planned"
goal_revision   = ""
input_revision  = ""
output_revision = ""
active_refs     = []
blocker         = ""

[bindings]
ticket = "t-042"
```

Write-once at instantiation; the parser refuses a `step` against a State
File whose bindings don't satisfy the class's binding schema (missing
required, unknown name, shape violation) — same refuse-by-name posture as
"State File phase {state_phase:?} is undeclared in ratmac.toml"
(src/scheduler.rs:322–326).

---

## 3. The relational join

### Today

`completion_gate` binds one literal ticket (src/machine.rs:50–52) and the
scheduler evaluates it against that single name
(src/scheduler.rs:530–544). Counting N children is inexpressible.

### Proposed guard kind: `join`

Vocabulary grows from 7 to 8 (src/machine.rs:59–68):

```toml
[[phases.p45-build.guards]]
kind    = "join"
spawn   = "tickets"       # must name a spawn declared on this same phase (RB504)
require = "all_passed"    # closed vocab; only value in v-next
min     = 1               # optional; default 1; least count of satisfied children
```

`accepted_fields("join") = &["spawn", "require", "min"]`.

### The spawn ledger (what the engine must store)

`.arca/runs/<spawn>/ledger.toml`, engine-owned, append/annotate-only:

```toml
# .arca/runs/tickets/ledger.toml  (Scheduler-owned; agents never write it — PGE-004 posture)
[[children]]
id        = "t-042"                  # derived from binding values; unique per ledger
class     = "ticket"
bind      = { ticket = "t-042" }
spawned_at = "<git rev at spawn>"
workspace = ".rtm-worktrees/tickets/t-042"   # present when workspace = "worktree"
abandoned = false                    # flips only via human-confirmed `rtm abandon` (§4)
```

Why a ledger and not "glob `.arca/runs/tickets/*/`": the ledger *fixes the
expected set*. A deleted child directory must make the join refuse loudly
("ledger names t-042; no run found"), not silently shrink N. The expected
set is closed by construction: `rtm spawn` is only legal while the parent
occupies the spawning phase (§1 step 1), and the parent cannot leave that
phase until the join passes — so by the time the join releases the
transition, the ledger it judged is final.

### Evaluation (engine, in-process, inside the pinned gate boundary — same
posture as PGE-003/PGE-005, src/scheduler.rs:504–544)

A `join { spawn = S, require = all_passed, min = m }` passes iff:

1. `.arca/runs/S/ledger.toml` exists and parses;
2. for every entry with `abandoned = false`, the child's
   `.arca/runs/S/<id>/state.toml` exists, its `phase` is graph-terminal in
   the child class (no ordinary outgoing transition — the same computation
   `transition_for` performs, src/graph.rs:131–136, inverted), and its
   `status = "passed"`;
3. the count of entries satisfying (2) is ≥ `m`.

On refusal, the failure names every non-satisfying child —
`"join", observed: "t-042 at t4-tests/executing; t-043 failed", expected:
"every spawned child terminal and passed"` — mirroring the
aggregate-defect rendering of `evaluate_sensitivity_receipts`
(src/scheduler.rs:510–524).

### Does this satisfy "guards judge artifacts, not narration"?

Yes, with one transitive step worth stating. A child's State File is
written only by the Scheduler (`.arca/schema.md:236` — the Scheduler stays
the sole writer of state), and the child's `status = "passed"` at a
terminal phase was itself only reachable because the *child's* guards —
receipts, contracts, files — passed. So the parent's join judges an
engine-written, machine-verified summary whose truth is grounded in the
child's artifact guards. It is exactly as artifact-grounded as the
goal-drift check that already reads `.arca/evidence.toml` inside `step`
(src/scheduler.rs:327–341). It is *more* trustworthy than `files_exact`
over agent-writable content, which `RB302` merely warns about
(`.arca/runbook-spec.md:111`): a child State File is not agent-writable at
all (`.arca/schema.md:301`).

**Coverage caveat.** `join` proves "everything spawned finished well," not
"everything that needed spawning was spawned." Coverage (one child per cut
ticket) is a separate relation; v-next delegates it to the existing
relational contract (`record_contract` already checks one owning ticket per
gap, `.arca/runbook-spec.md:65`) plus the phase prompt, and §"Open
questions" proposes `covers` as a future join field.

---

## 4. Parent–child protocol

### Does the invoking phase block?

**No new wait machinery: waiting is refusal.** The parent's `rtm step`
simply returns `StepOutcome::Refused` (the existing refusal path,
src/scheduler.rs:344–347) while the join guard fails. The parent Run
"parks" in the spawn/join phase. This reuses the single most load-bearing
property of the engine — a step either passes all guards or refuses by
name — and adds zero scheduler state.

### Can the parent step elsewhere while children run?

Not in v-next, and deliberately so. Parent position stays a single scalar
(src/graph.rs:152–156 unchanged); the plurality lives entirely in the set
of child State Files on disk. This is the statechart AND-state flattened
into the filesystem: concurrency without a concurrent position. A parent
that could wander off mid-spawn would need either a position *set* or
history states — both rejected by 03's analysis as the expensive road. The
parent *agent* is free to do human-mediated work meanwhile; the parent
*machine* is not, and `active_refs` (src/model.rs:200, initialized empty at
src/scheduler.rs:255) records `["runs/tickets"]` while a spawn ledger is
open so `rtm status` can say what the Run is waiting on.

### Child terminal states, from the parent's view

| parent view | child on-disk fact |
|---|---|
| **succeeded** | `status = "passed"` at a graph-terminal phase of the child class |
| **failed** | `status = "failed"` at any phase (a failed Run does not proceed) |
| **abandoned** | ledger entry `abandoned = true` — written only by human-confirmed `rtm abandon --run tickets/t-042 --reason "..."` |
| **running** | anything else (`planned`/`executing`/`blocked`, non-terminal phase) |

`abandoned` is the child-level mirror of the PGE-006 blocked route: the
engine never abandons anything on its own; a human explicitly removes a
child from the expected set, and the ledger records that removal durably.
`join` skips abandoned entries but still requires `min` satisfied children,
so a join can never pass vacuously by abandoning everything (with the
default `min = 1`).

### Failure propagation — recommendation

Applying 02's supervision-tree options:

- **one-for-one restart (automatic):** rejected. Automatic restart is a
  routing decision taken on a lifecycle observation — precisely what
  PGE-006 exists to forbid — and it makes replay nondeterministic (the same
  filesystem would mean "restart happened or not depending on wall clock").
- **abandon-all:** rejected as a default. It destroys the work of passing
  siblings and is the one option that can never be undone from artifacts.
- **escalate (report-and-hold):** **recommended for v-next.** The join
  refusal already names each failed child; the parent holds; a human either
  (a) runs the human-confirmed `rtm respawn --run tickets/t-042`, which
  marks the old child's ledger generation superseded and instantiates a
  fresh child run for the same bindings (one-for-one restart, *manually
  confirmed*), (b) abandons the child, or (c) takes the parent's declared
  blocked route. Rationale: this is exactly the division of labor the
  engine already commits to — automation never routes on failure, humans
  confirm escapes (src/graph.rs:55–57, PGE-006) — extended one level down
  the tree. It is Erlang's "let it crash, supervisor decides" with the
  human as supervisor of last resort, which matches the project's steering
  posture rather than Temporal's policy-in-code.

**Correction (2026-07-30, propagating the verb-authorization adoption —
`AR-11` of the doctrine-convergence issue, `i-016`, `08` §4 item 15,
recorded in design.md):** two invocations in this section drop their
confirmation. The terminal-states table's
`rtm abandon --run tickets/t-042 --reason "..."` and the escalate
bullet's `rtm respawn --run tickets/t-042` are written as if the run-id
flag or a `--reason` were the human act; they are not, and `--reason` is
not a confirmation. The confirmation is a typed `--confirm` phrase naming
the run id — the same shape as the working rules' abandon form in
`.arca/schema.md` (`rtm abandon --confirm "abandon <...>"`) — per
`FDC-007` in the machine-composition issue's (`i-018-machine-composition`)
spec.md: `respawn` and abandon-with-run-id require confirmation phrases
naming the run id, while `spawn` is ordinary motion with no phrase. The
original lines stay as written and should be read through this
correction.

---

## 5. Structure discipline (doctor checks)

### Today

The doctor's shape checks are the RB2xx family — unique initial phase
(`RB202`/`RB203`, enforced at runtime too, src/scheduler.rs:760–778),
reachability (`RB204`), terminal-count warning (`RB205`)
(`.arca/runbook-spec.md:103–109`). All are per-edge/per-phase table scans —
polynomial, per 03's requirement.

### Proposed shape restriction

**The spawn-join region is a single phase.** The `join` guard for spawn `S`
must be declared on the *same phase* that declares `S`. Spawn-at-entry,
join-at-exit: the phase itself is the single-entry/single-exit region, so
the well-structured (workflow-net) property 03 demands is preserved
trivially — the region collapses to one node and the parent graph's
existing RB2xx analysis is untouched. No cross-phase join, no overlapping
regions, no join naming a foreign spawn. This is the smallest restriction
that keeps every static check a table lookup.

Additionally, **the class-reference graph must be a DAG**: `main → ticket`
is fine; `ticket → main` or any cycle is refused. Recursive spawning is
expressible in a later tier if ever wanted (03 covers recursive state
machines and their costs); v-next forbids it so depth is bounded by the
class count and doctor checks stay polynomial: one DFS over
`O(classes + spawn declarations)`.

### Proposed diagnostics (new RB5xx family, extending `.arca/runbook-spec.md:93–112`)

| code | severity | finding |
|---|---|---|
| `RB501` | error | A spawn names a class the runbook does not declare. |
| `RB502` | error | The class-reference graph has a cycle. |
| `RB503` | error | A phase declares a spawn but no `join` guard on that phase names it. |
| `RB504` | error | A `join` guard names a spawn not declared on its own phase. |
| `RB505` | error | A spawn's `bind` set differs from the child class's required binding set. |
| `RB506` | error | A guard's `binding` field names a binding its own class does not declare. |
| `RB507` | error | A spawned class does not have exactly one terminal phase (child "passed" would be ambiguous). |
| `RB508` | error | A phase declares more than one spawn (v-next restriction). |
| `RB509` | warning | A class declares bindings no guard consumes. |

Each is a closed-world membership or set-equality check over declared
names — no path enumeration, no exponential blowup.

---

## 6. Determinism: resolving the PGE-006 tension

**The tension, stated honestly.** PGE-006's code comment says ordinary
routing "never branches on any lifecycle field" (src/graph.rs:55–57). A
join guard reads child `status` — a lifecycle field of *some* Run. Is the
invariant broken?

**Resolution: three distinctions.**

1. **Selection vs. gating.** PGE-006 governs *which edge* is taken.
   `transition_for` still returns the first (unique) ordinary edge with no
   inputs beyond the runbook and the phase name (src/graph.rs:131–136) —
   the join changes nothing there. Guards never select; they *permit or
   refuse* the already-determined successor. A refused step is not a
   branch; it is the same step, later. This is already how every guard
   works today (src/scheduler.rs:344–347).
2. **Own vs. foreign lifecycle.** The rule the code actually enforces is
   about a Run's *own* lifecycle: a guard table may not mention `status`
   (src/machine.rs:480–482, `RB104`), and routing never reads the stepping
   Run's status/blocker. A *child's* State File, from the parent's frame,
   is an on-disk fact produced by another writer — category-identical to
   `.arca/evidence.toml`, which `step` already consults for the goal-drift
   refusal (src/scheduler.rs:327–341).
3. **Volatile vs. stable-once-true.** The join reads child status only in
   conjunction with graph-terminality. A terminal `passed` cannot revert by
   any automated action; only human-confirmed `rtm respawn` (§4) replaces
   a child, and that act is itself durably recorded in the ledger. So the
   fact the join consumes is monotone between human interventions — the
   same stability class as a receipt.

**Proposed invariant restatement** (successor to PGE-006's comment; the
blocked-route sentence is unchanged):

> Ordinary routing is a function of the runbook and the current phase
> alone: each phase has at most one ordinary successor, and no lifecycle
> field of any Run ever selects among edges. Guards gate that unique
> successor; they may consult any on-disk fact that is stable once
> satisfied — including a child Run's terminal record, which only the
> Scheduler writes and only a human-confirmed act can supersede — and may
> never consult their own Run's `status`, `blocker`, or claim. Given a
> filesystem snapshot, `step` has exactly one outcome.

**Correction (2026-07-30, propagating the verdict-routed selection ruling —
`AR-03` of the doctrine-convergence issue, `i-016`, recorded in design.md's
"Individual human rulings," "Edge selection is verdict-only"):** the
restatement above is superseded and must not be copied into source as
PGE-006's replacement comment. It restates the linear invariant — "each
phase has at most one ordinary successor," guards only gating that unique
successor — which the ruling replaces: a branching phase declares a closed
verdict input list, every ordinary outgoing edge carries exactly one
unique value from it, and the verdict-guarded edge whose value matches the
live verdict is the edge taken (`07-conceptual-model.md` §5 "Typing" and
§9 "Resolved (2026-07-29)"). The stable-once-satisfied clause and the
own-`status`/`blocker` ban survive; the single-successor clause does not.
Settled position: `FDC-001` in the doctrine-convergence issue's spec.md;
design.md's "Individual human rulings," "Edge selection is verdict-only
(`AR-03`)."

"Determinism given the filesystem" is thereby preserved verbatim: replaying
`step` against the same tree of State Files yields the same
refusal-or-transition, every time.

---

## 7. The self-hosting test: P1–P5 through this proposal

The project's own cycle (`.arca/schema.md`, cycle diagram): P1 Fold in
issues → P2 Find the gaps → P3 Cut tickets → P4 Write this ticket's tests →
P5 Code + all tests + fix + review, with `P5 →|next ticket| P4` as the
serialized inner loop and `P2 →|nothing missing| IDLE` as the exit branch.

Mapping: the serialized P5→P4 inner loop becomes N children of class
`ticket` (one per cut ticket); the join closes the sprint. The outer
"next sprint" loop stays what it is today — a fresh `rtm start`
(ADR-0007: one Run per instantiation) — so **no [tier-1 dep] is needed for
self-hosting**. The `P2 →|nothing missing| IDLE` branch is the one true
**[tier-0 dep]**: with only this document's machinery, a gap-free P2 ends
via the human-confirmed blocked route, not an automated branch.

Full proposed runbook, end to end:

```toml
# Project Runbook — sprint cycle with per-ticket fan-out (proposed format).
# Phase prompts are agent-facing; they never ask an agent to write a
# Scheduler-owned file (PGE-004).

# ---------- root class: the sprint ----------

[phases.p1-fold]
prompt = "Fold accepted issues into the goal corpus; record intake evidence."

[[phases.p1-fold.guards]]
kind = "intake_contract"

[phases.p2-gaps]
prompt = "Compare goal against reality; record every gap as a residual."

[[phases.p2-gaps.guards]]
kind = "record_contract"

[phases.p3-cut]
prompt = "Cut tickets: every gap gets exactly one owning ticket."

[[phases.p3-cut.guards]]
kind = "record_contract"

[phases.p45-build]
prompt = """For every open ticket, spawn one child:
rtm spawn --spawn tickets --bind ticket=<id>
then drive each child through its phases in its worktree. This phase
completes when every spawned child has passed."""

[phases.p45-build.spawn]
name      = "tickets"
class     = "ticket"
bind      = ["ticket"]
workspace = "worktree"

[[phases.p45-build.guards]]
kind    = "join"
spawn   = "tickets"
require = "all_passed"
min     = 1

[phases.p-close]
prompt = "Close the sprint: reconcile records, archive, write the sprint note."

[[phases.p-close.guards]]
kind = "record_contract"

[[transitions]]
from = "p1-fold"
to   = "p2-gaps"

[[transitions]]
from   = "p2-gaps"
to     = "p3-cut"
freeze = "goal"          # ETB-003: intake-completion boundary

[[transitions]]
from = "p3-cut"
to   = "p45-build"

[[transitions]]
from = "p45-build"
to   = "p-close"

# PGE-006: human-confirmed escape if the sprint wedges mid-build.
[[transitions]]
from          = "p45-build"
to            = "p2-gaps"
blocked-route = true

# ---------- child class: one ticket ----------

[classes.ticket.bindings.ticket]
required = true
shape    = "token"
doc      = "the ticket this instance drives"

[classes.ticket.phases.t4-tests]
prompt = "Write this ticket's tests first; record one sensitivity receipt per planned test."

[[classes.ticket.phases.t4-tests.guards]]
kind    = "sensitivity_receipts"
binding = "ticket"

[classes.ticket.phases.t5-complete]
prompt = "Code until all tests pass; fix and review; record completion receipts."

[[classes.ticket.phases.t5-complete.guards]]
kind    = "completion_gate"
binding = "ticket"

[classes.ticket.phases.t-done]
prompt = "Terminal. Nothing to do."

[[classes.ticket.transitions]]
from = "t4-tests"
to   = "t5-complete"

[[classes.ticket.transitions]]
from = "t5-complete"
to   = "t-done"
```

Walkthrough: `rtm start` → root Run at `p1-fold`. Steps proceed to
`p45-build` under today's semantics unchanged. There the Main-Agent runs
`rtm spawn` once per cut ticket (say t-042, t-043, t-044); the engine
writes three child roots under `.arca/runs/tickets/` plus the ledger, and
three worktrees. Each child agent steps its own Run: `t4-tests` gated by
`sensitivity_receipts` resolved through `bindings.ticket`, `t5-complete`
gated by `completion_gate`, then `t-done` (terminal, `passed`). Any parent
`rtm step` before that point refuses with the per-child roster. When all
three read terminal-passed, the parent's join releases the unique successor
and the Run closes at `p-close`. Next sprint = next `rtm start`.

**Verdict: the cycle is expressible.** Residual gaps, flagged honestly:
(1) the `P2 → IDLE` branch needs tier 0 (today: blocked route or Run end);
(2) join coverage — "one child per cut ticket" — rests on the prompt plus
`record_contract`, not on the join itself (§3 caveat); (3) the P5→P4
"re-test after fix" micro-loop inside a ticket is expressed here as one
`t5-complete` phase whose completion gate demands green receipts, which is
faithful to how PGE-005 already collapses that loop into a gate
(src/scheduler.rs:526–544) — no [tier-1 dep].

---

## Tier dependency summary

| construct | needs tier 0 | needs tier 1 | needs tier 2 | notes |
|---|---|---|---|---|
| spawn table (§1) | no | no | yes | bindings are tier 2 |
| binding schema/reference (§2) | no | no | — | *is* tier 2 |
| join guard (§3) | no | no | yes | gate, not branch |
| escalate/respawn (§4) | no | no | no | human-confirmed verbs |
| P2→IDLE branch (§7) | **yes** | no | no | only tier-0 dep found |
| outer sprint loop (§7) | no | avoided | no | new Run per sprint instead |

---

## Open questions for the human

1. **Join coverage.** Should `join` grow a `covers` field tying the ledger
   to a source relation (e.g. "every open ticket has a child"), or is
   prompt + `record_contract` discipline enough for v-next? **Resolved
   (2026-07-29):** no `covers` field for now; coverage stays with the
   record contract plus the prompt (`08` §4 item 14, `AR-08`; batch human
   sign-off, recorded in design.md).
2. **Ledger placement.** `.arca/runs/<spawn>/ledger.toml` as proposed, or
   should ledger entries live inside the parent's State File? (Separate
   file keeps `state.toml`'s seven-field schema nearly intact but adds a
   second Scheduler-owned artifact class.) **Resolved (2026-07-29):**
   neither as asked — the ledger nests under the parent run's own
   directory, `.arca/runs/<parent-run-id>/spawn/<spawn-name>/ledger.toml`
   (`08` §4 item 14, `AR-08`; batch human sign-off, recorded in design.md;
   see the residency supersession note above).
3. **Child log/lock granularity.** One `log.md` per child (proposed) means
   the transition history of a sprint is sharded across N+1 files. Is a
   consolidated read view (`rtm log --all`) wanted, or is sharded fine?
   **Resolved (2026-07-29):** sharded is fine; no consolidated read view
   yet (`08` §4 item 16; batch human sign-off, recorded in design.md).
4. **`rtm respawn` generations.** Should a superseded child's directory be
   archived (like trial worktrees) or overwritten? Archival preserves the
   failure evidence the retrospective wants. **Resolved (2026-07-29):**
   archived in spirit — respawn mints a new id and the ledger entry
   records the superseded one, preserving the failure evidence (`08` §4
   item 11, `AR-09`; batch human sign-off, recorded in design.md).
5. **Worktree lifecycle for children.** Reuse the trial-worktree
   create/adopt/retire verbs wholesale, or is child-worktree lifecycle
   different enough (long-lived, agent-driven) to need its own TWL-style
   decision series? **Resolved (2026-07-29):** reuse the trial-worktree
   verbs wholesale (`08` §4 item 18; batch human sign-off, recorded in
   design.md).
6. **Literal `ticket =` deprecation.** Keep the literal form indefinitely
   (both forms legal, `RB111` only forbids both-at-once), or schedule a
   doctor warning steering runbooks toward `binding =`? **Resolved
   (2026-07-29):** keep the literal form; no deprecation warning scheduled
   (`08` §4 item 18; batch human sign-off, recorded in design.md).
7. **`min` semantics.** Is `min` (least satisfied children) wanted at all
   in v-next, or is the implicit "ledger non-empty and all passed"
   sufficient? Quorum joins (`require = "quorum"`) are deliberately left
   out; say the word if a real machine needs one. **Resolved (2026-07-29):**
   keep `min`, default 1; quorum stays out (`08` §4 item 18; batch human
   sign-off, recorded in design.md).

---

## Supersession note — 2026-07-30 atomic cut

Billy split the pending doctrine-convergence execution bundle into three current requirement homes:
input-routed transitions (`FDC-001`) remain in
[i-016-fsm-doctrine-convergence](../../issue/i-016-fsm-doctrine-convergence/index.md), Run completion
(`FDC-002`) moved to [i-020-run-completion](../../issue/i-020-run-completion/index.md), and input
delivery and durability (`FDC-003`) moved to
[i-019-input-delivery-durability](../../issue/i-019-input-delivery-durability/index.md). Earlier
references to one “verdict-routed execution core” are historical. The judge-authored verdict record
and the transition input it carries are now named separately; witnessed judgment remains deferred,
and judge independence remains in the machine-composition issue.
