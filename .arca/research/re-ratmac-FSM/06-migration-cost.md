# Migration cost and sequencing — from today's linear engine to conditional edges, named runs, and child instances

Wave-2 research, agent 3 of 3. Date: 2026-07-28.
Scope: the concrete code-level path from the current engine to the capability ladder (Tier 0 conditional
edges, Tier 1 named runs, Tier 2 child instances), sliced into issue-sized steps in the bundle style of
`.arca/goal/index.md:56-61`. Every codebase claim cites file:line as of the verified baseline below.
"Today" describes what is in the tree; "proposed" describes what does not exist yet.

## 0. Verified baseline

- `cargo test --workspace` passes with 199 tests and exit 0 (run read-only for this research). The 48
  behavior files under `test/qa/tests/` carry 182 test functions; the remaining tests sit in the harness
  library (`test/qa/src/lib.rs`).
- Routing today: `Transition` is `{from, to, freezes_goal, blocked_route}` and nothing else — no condition
  field (src/graph.rs:50-59). Routing is first-ordinary-edge-wins: `transition_for` returns the first
  non-blocked edge leaving a phase (src/graph.rs:131-136); `next_phase` is just its destination
  (src/graph.rs:147-149). Blocked routes are a separate, human-only lane (src/graph.rs:141-144;
  design.md `PGE-006`).
- The parser refuses any transition key outside `from`, `to`, `freeze`, `blocked-route`
  (src/machine.rs:373-375, mechanism src/machine.rs:455-471, diagnostic code `RB103`), which honors
  `R-011` (spec.md:17). So a new edge key is a parser change by construction, never a silent pass-through.
- One Run per project, structurally: `rtm start` refuses while `.arca/state.toml` exists
  (src/scheduler.rs:242-247); the State File path is the fixed literal `.arca/state.toml`
  (src/state.rs:56); the log is `.arca/log.md` (src/scheduler.rs:258, 361); the lock is `.arca/rtm.lock`
  (src/scheduler.rs:238, 310, 800); Run evidence is `.arca/evidence.toml` (src/pin.rs:17). The CLI takes
  no run identifier (src/cli.rs:135-165; `R-023`, spec.md:29).
- The Machine Class is re-read from `.arca/ratmac.toml` on every invocation: at start
  (src/scheduler.rs:233), at every step (src/scheduler.rs:312-313), and at status (src/scheduler.rs:844),
  always through the single reader `load_class` (src/scheduler.rs:202-212).
- The engine never completes a Run. `step` at a phase with no ordinary out-edge refuses with "no outgoing
  transition" (src/scheduler.rs:350-359), and the only `status` values the engine ever writes are
  `planned` (src/scheduler.rs:251) and `blocked` (src/scheduler.rs:823, src/model.rs:161) — `passed` and
  `failed` exist in the vocabulary (`R-002`, spec.md:8; src/model.rs:12) but no code path writes them.
- The doctor's edge lints are: exact duplicate edge (src/doctor.rs:254) and phase unreachable from the
  initial phase (src/doctor.rs:329). There is no lint for two ordinary out-edges with *different*
  destinations from one phase — exactly the shape first-edge-wins silently discards
  (src/graph.rs:131-136). Wave 1 flagged the same gap (01-engine-constraints.md).

The owner's framing holds up in the code: one transition table is not the obstacle (a class is already a
pure template, `R-013`, spec.md:19); the single `state.toml`/`log.md`/`rtm.lock`/`evidence.toml` residency
is what makes parallel execution unrealizable today.

## 1. Tier 0 — conditional edges

**Today.** A phase has at most one meaningful ordinary out-edge; a second one is dead (src/graph.rs:131-136).
Guards decide *whether* to leave a phase (src/scheduler.rs:325-343), never *where to go*. A cyclic class
therefore cannot terminate: the back-edge either always fires or is never reachable.

**Proposed.** A transition gains an optional `when` list holding the existing guard vocabulary, and routing
becomes declaration-order first-*passing*-edge-wins. No new predicate language: the seven guard kinds
(src/machine.rs:521-544) already express file shape, file content, command exit, receipts, and contracts,
and they are evaluated by machinery the Scheduler already owns (dispatch at src/scheduler.rs:467-497).
Determinism is preserved the same way guard order already is — declaration order (`TRP-004`,
src/machine.rs:582-585).

Code touch points:

1. src/machine.rs — accept `when` in the transition key list (today refused at src/machine.rs:373-375);
   parse it by reusing `parse_guard` (src/machine.rs:476-553). Small: the field plumbing plus ~40 lines.
2. src/graph.rs — `Transition` carries the predicate (src/graph.rs:50-59); `transition_for`
   (src/graph.rs:131-136) can no longer answer alone, because predicate evaluation needs a project root.
   Cleanest: keep the graph pure and move route *selection* into the Scheduler — a `route_for` that walks
   ordinary edges in order and takes the first whose `when` passes.
3. src/scheduler.rs — in `step`, replace the single `next_phase` call (src/scheduler.rs:350-359) with
   route selection after exit guards pass; a refusal when no edge qualifies must name every candidate edge
   with observed vs expected, reusing `GuardFailure` (src/scheduler.rs:44-74; `R-019`, spec.md:25).
   `freezes_goal` lookup (src/scheduler.rs:345-349) moves onto the *selected* edge.
4. src/doctor.rs — two new lints with stable codes: an edge shadowed by an earlier unconditional edge, and
   a phase with two or more ordinary out-edges where more than one is unconditional. This also closes the
   pre-existing silent-discard gap (src/doctor.rs:254, 329 are the only edge lints today).
5. `.arca/runbook-spec.md`, `rtm scaffold`, and the authoring repair table — the `RBS`/`AAL` surfaces are
   tested as one set with the engine codes (t057 `emitted_codes_and_documented_codes_are_one_set`, t058
   `repair_table_covers_exactly_the_engine_codes`), so the doc rows land in the same issue as the code.

**Test blast radius (Tier 0).** Extended, mostly not broken: t056_typed_parser (13 tests — new key becomes
known; unknown-key fixtures stay red for other keys), t057_deep_doctor (12), t055_runbook_spec (8),
t058_authoring_loop (10), t022_r026 (9, routing/log wording). Existing linear fixtures remain valid because
an absent `when` means unconditional, so the other ~150 tests stay green.

**This is the smallest shippable step that yields a terminating loop.** A class with `P5 → P1` guarded by
"work remains" and `P5 → done` guarded by its complement reaches `done` and stops. Nothing in Tier 1 or 2
is needed for that.

**Supersession — tier-0 edge selection (2026-07-29, individual human ruling on `AR-03` of the
doctrine-convergence issue, `i-016`):** the "Proposed" tier 0 above — a transition's `when` list drawn from
the general seven-kind guard vocabulary, with declaration-order first-*passing*-edge routing selecting among
ordinary edges — describes migration history only, not the target design. Billy ruled individually that edge
selection is verdict-only: ordinary guards stay readiness AND-gates ("can we move"), never selectors ("where
to"); the only selecting edges are typed-verdict edges. This is the opposite of the `08` §4 item 1 RECOMMENDED
default (ship this section's general `when` as tier 0, layering verdicts on top as an eighth guard kind) —
that recommendation was not adopted. Settled position: the doctrine-convergence issue's design.md, "Individual
human rulings", "Edge selection is verdict-only (`AR-03`)"; see also `07-conceptual-model.md` §3, whose
matching sentence this ruling reinstates rather than demotes.

## 2. Tier 1 — named runs

**Today.** `R-021` promises the data model allows N Runs (spec.md:27; ADR-0007, design.md:55), but every
persistence path is a project-level literal: state (src/state.rs:56), log (src/scheduler.rs:258, 361),
lock (src/scheduler.rs:238, 800), evidence (src/pin.rs:17), and the flat layout is itself a requirement
(`R-024`, spec.md:30; ADR-0008, design.md:63). Admission is "a State File exists ⇒ refuse"
(src/scheduler.rs:242-247; `R-022`, spec.md:28).

**Proposed.** Run residency: `.arca/run/<name>/` owns `state.toml`, `log.md`, `evidence.toml`, `rtm.lock`,
and (see §4) the pinned class copy. `rtm start <name>` creates it; `rtm step <name>` / `rtm status <name>`
address it; the argument-free forms stay legal only while exactly one Run exists. This supersedes `R-022`,
`R-023`, `R-024` and needs successor decisions to ADR-0007/ADR-0008 — a spec/design edit, not just code.

**Supersession — run residency and addressing (2026-07-29, ruling `AR-01`/`AR-02` of the doctrine-convergence
issue, `i-016`):** the paragraph above is superseded on three points. (1) Residency: the adopted canonical
scheme is plural `.arca/runs/<run-id>/`, not the singular `.arca/run/<name>/` above — `06`'s singular spelling
was explicitly named the minority spelling to drop (`08-adversarial-review.md` §2 AR-01; adopted `08` §4 item
6). (2) Addressing: run addressing is the flag form `--run <id>`, always required, not the positional
`rtm start <name>` / `rtm step <name>` form above (`08` §4 item 7, `AR-02`). (3) Optionality: "the
argument-free forms stay legal only while exactly one Run exists" does not hold — a missing `--run` value
always refuses with the roster, with no unambiguous-registry exception (same ruling; this also resolves open
question 4 below). Settled position: `04-run-identity.md` §2.1-§2.3; design.md's "Adopted defaults" items 6
and 7.

Migration of existing flat projects: the house already has the precedent — the engine *refuses and
instructs* when it finds a legacy lock name, and explicitly does not modify it (src/scheduler.rs:110-129).
Flat `state.toml` should get the same treatment: a named refusal with the exact migration command, never a
silent move.

Code touch points: `StateStore` takes a run directory instead of a root (src/state.rs:56); every path join
in `start`/`step`/`status`/`initialize_state`/`record_missing_prerequisite` (state writes at
src/scheduler.rs:270, 401, 812, 825) re-roots; `InvocationLock` becomes per-run (src/scheduler.rs:105-177);
`Evidence::load`/`write` re-root (src/pin.rs:57, 151); the CLI grows the name argument
(src/cli.rs:53-69, 117-165; `src/bin/rtm.rs:19` is untouched glue). Abandon and trial lifecycles re-root
with it (src/abandon.rs, src/blocked.rs surfaces; t051-t054).

**Test blast radius (Tier 1).** This is the expensive tier — it rewrites fixtures across the widest set of
families: t012_r022 (singleton, 2), t013_r023 (no run-id, 1), t014_r024 (flat layout, 1), t023_r015
(concurrency, 1), t015_r027 (corrupt state, 1), t029_r007 (4), t044_start_policy (5), t051_abandon (6),
t043_goal_freeze (5), t042_gate_pin (6), t022_r026 (9), t026_r030 (2), t024_r028 (2), t045_bootstrap_doctor
(7), t052-t054 trials (25). Roughly 75-80 of the 199 touch a path that moves. The parser/doctor families
(t055-t058, 43 tests) are almost untouched — which is the strongest argument for landing Tier 0 first,
while the fixture ground is still.

## 3. Tier 2 — child instances

**Today.** Nothing exists; the goal explicitly defers process management ("no process spawning… a future
decision, not a dormant code path", index.md:14; print-first `R-030`, spec.md:36; ADR-0010, design.md:84).
`active_refs` sits unused in the State File (src/model.rs:200; `R-025`, spec.md:31) — a ready-made place
for child names.

**Proposed.** Children are ordinary named Runs (Tier 1 objects), created when a parent phase's declared
child list is instantiated at entry, recorded in the parent's `active_refs`. The join is *a guard kind*
("all named children hold a terminal status"), evaluated like any other guard — no event loop, no spawn,
print-first preserved. Child failure surfaces as a guard refusal naming the child (`R-017`/`R-019`
semantics, spec.md:23-25), never as automatic cancellation.

Hard prerequisite: Tier 1 residency. Without per-run state, log, lock, and evidence there is nothing for a
child to own; with them, Tier 2 is mostly a parser addition (child declaration), one new guard kind
(src/machine.rs:521-544 gains an arm, src/scheduler.rs:467-497 gains a dispatch), doctor lints
(instantiation cycles, dangling class references), and completion semantics — which forces the engine to
finally write `passed`/`failed` (today it never does: only src/scheduler.rs:251 and :823 write status).

## 4. The runbook-freeze gap — position

**Today.** The class is re-read every step (src/scheduler.rs:312-313) and only *goal* drift is checked
(src/scheduler.rs:326-340, `ETB-003`). Evidence pins the engine identity (src/scheduler.rs:284-289,
`ETB-001`) and every gate artifact (src/scheduler.rs:706-755), but never the runbook itself. A mid-Run
merge can swap guards or reroute edges silently; the only accidental tripwire is the undeclared-phase
refusal (src/scheduler.rs:316-324), which fires only if the *current* phase name vanished.

**Position: fix it with (or immediately before) named runs; it is not a blocker for conditional edges;
and per-run residency does not make it moot — it makes the gap worse.** Reasons:

- Tier 0 does not widen the hole: conditional edges are read from the same reload, under the same lock,
  and the drift risk is unchanged in kind. Sequencing the fix before Tier 0 buys nothing.
- Tier 1 without a fix *multiplies* the hole: N concurrent Runs sharing one mutable `.arca/ratmac.toml`
  means any run can have its machine swapped by an edit made for another. The moment residency exists, the
  class must be pinned per Run.
- The cheap interim fix follows the `ETB-001` pattern exactly: at `rtm start`, record the runbook content
  hash in Run evidence (beside src/scheduler.rs:284-296); at `step`, append a "runbook drift" failure the
  same way goal drift is appended, not short-circuited (src/scheduler.rs:326-340). ~30 lines plus tests.
- The full fix belongs to residency: copy the class into `.arca/run/<name>/` at start and read only the
  copy thereafter, retiring reload-per-step (src/scheduler.rs:312) for stepping while keeping live reads
  for `doctor`/`scaffold`. A human-confirmed re-pin (the `PGE-006` shape: an escape a human confirms,
  src/machine.rs:388-396) covers the legitimate "the runbook was wrong" case.

**Supersession — runbook pin mechanics (2026-07-29, adopted default, `08` §4 item 8, of the
doctrine-convergence issue, `i-016`):** the "full fix" above — copying the class into a per-run directory and
reading only that copy — is not adopted. Billy adopted the batch default instead: the runbook pin stays
hash-only (the interim fix earlier in this section, and `RBP` in the bundle list below); a per-run copy is
revisited only if a real drift case demands it. The `.arca/run/<name>/` path in the bullet above is
additionally stale — see the residency supersession note at §2. Settled position: design.md's "Adopted
defaults" item 8; this also resolves open question 2 below.

## 5. Proposed issue bundles (house style, route order)

Modeled on the four-issue route of index.md:56-61 (`RBS` → `TRP` → `DRD` → `AAL`). Prefixes chosen to
avoid every prefix already integrated (RAT, EXT, ETB, PGE, AOI, ORS, TWL, RBS, TRP, DRD, AAL).

1. **i-0xx-conditional-guarded-edges — `CGE-001`…`CGE-006` (~6 requirements).** Scope: `when` predicates on
   ordinary transitions using the existing guard vocabulary; declaration-order first-passing-edge routing;
   refusal names every candidate; doctor lints for shadowed and ambiguous edges; runbook-spec, scaffold,
   and repair-table rows; an acceptance fixture where a cyclic class terminates.
   *Smallest shippable step to a terminating loop: this bundle — concretely `CGE-001` (parse) +
   `CGE-002` (routing), proven by the `CGE-006` fixture.*
   *(Scope corrected by the `AR-03` ruling — see the tier-0 supersession note at §1: edge selection is
   verdict-only, not a general `when` list.)*
2. **i-0xx-runbook-pin — `RBP-001`…`RBP-003` (~3 requirements).** Scope: runbook content hash recorded in
   Run evidence at start (`ETB-001` pattern); step refuses on runbook drift, appended beside goal drift;
   an explicit human-confirmed re-pin. Small, independent, closes today's hole before residency raises the
   stakes.
3. **i-0xx-named-run-residency — `NRI-001`…`NRI-008` (~8 requirements).** Scope: `.arca/run/<name>/`
   residency for state, log, lock, evidence; named `start`/`step`/`status`; argument-free forms only when
   unambiguous; flat-layout refusal with explicit migration instruction (legacy-lock precedent,
   src/scheduler.rs:110-129); per-run class copy superseding `RBP`'s hash; abandon/trial lifecycle
   re-rooted; successor decisions to ADR-0007/ADR-0008 and supersession of `R-022`/`R-023`/`R-024`.
   *(Path, argument-free form, and per-run-copy scope corrected by the `AR-01`/`AR-02` supersession note at
   §2 and the pin-mechanics note at §4: canonical path is plural `.arca/runs/<run-id>/`, `--run` stays always
   required, and the pin stays hash-only.)*
4. **i-0xx-child-instances — `CHI-001`…`CHI-006` (~6 requirements).** Scope: child-run declaration in the
   class; children as ordinary named Runs recorded in `active_refs`; a join guard kind; child failure as
   refusal; doctor lints for instantiation cycles; terminal-status writes (`passed`/`failed`) and
   abandonment cascade rules.

Route order `CGE → RBP → NRI → CHI`. `RBP` could fold into `NRI`, but keeping it separate ships a real
integrity fix months earlier for the cost of three requirements.

**Supersession — route order (2026-07-30, propagating the accepted route inversion of the
dependency-strata split):** the route order above is superseded. The accepted route (human ruling,
2026-07-29, recorded in the doctrine-convergence issue's design.md, "Dependency route", and restated in
the run-residency and machine-composition issues' spec.md): run residency (`i-017-run-residency`) lands
first; the verdict-routed execution core (`i-016-fsm-doctrine-convergence`) depends on it; machine
composition (`i-018-machine-composition`) depends on both. `NRI`-shaped work therefore precedes the
`CGE`-shaped core; the bundle list above binds migration history only. (See also
`07-conceptual-model.md` §10's route-order supersession note.)

## Open questions for the human

1. Prefix approval: `CGE`, `RBP`, `NRI`, `CHI` — any collision with dict.md entries or taste objections?
   **Resolved (2026-07-29):** accepted, no collision (`08` §4 item 4; batch human sign-off, recorded in
   design.md).
2. Class pin mechanics in `NRI`: hash-only (keeps `TRP-005`'s "one reader, one source" clean) or full
   per-run copy (stronger, but two files can now disagree and the runbook-spec must say which wins)?
   **Resolved (2026-07-29):** hash-only; a per-run copy is revisited only if a real drift case demands it
   (`08` §4 item 8; batch human sign-off, recorded in design.md; see the pin-mechanics supersession note
   at §4).
3. Should the engine ever write `passed` at a terminal phase, and if so does that belong in `CGE-006`
   (loop termination needs an observable end) or strictly in `CHI` (children need terminal statuses)?
   Today no code path writes it (only src/scheduler.rs:251 and :823 write status). **Resolved (2026-07-29):**
   yes — the engine writes `passed` on arriving at a phase with no ordinary out-edge, and `abandoned` via the
   abandon path (individual human ruling on `AR-05`, recorded in design.md); which bundle owns the write is
   not fixed by the ruling and stays an implementation-sequencing detail.
4. Is the argument-free `rtm step`/`rtm status` form permanent for single-Run projects, or deprecated once
   named runs exist? This decides how much of t013_r023 and t044_start_policy survives versus rewrites.
   **Resolved (2026-07-29):** deprecated — `--run` stays always required once named runs exist, with no
   argument-free exception (`08` §4 item 7, `AR-02`; batch human sign-off, recorded in design.md; see the
   addressing supersession note at §2).
5. Flat-layout migration: confirm the refuse-and-instruct stance (no auto-migration), matching the legacy
   lock precedent at src/scheduler.rs:110-129. **Resolved (2026-07-29):** confirmed — refuse-and-instruct, no
   auto-migration (`08` §4 item 9; batch human sign-off, recorded in design.md).
6. Does `blocked_route` (`PGE-006`) interact with `when`? Proposal assumes blocked routes never carry
   predicates — a human hold needs no machine condition — but that should be a written `CGE` decision.
   **Resolved (2026-07-29):** no interaction — blocked routes carry no predicates and sit outside
   exhaustiveness and first-passing order (`08` §4 item 2; batch human sign-off, recorded in design.md;
   consistent with the verified route-kind separation, `AR-14`).

---

## Supersession note — 2026-07-30 atomic cut

The dependency sentence above naming one “verdict-routed execution core” is superseded. Billy split
the pending bundle into input-routed transitions (`FDC-001`) at
[i-016-fsm-doctrine-convergence](../../issue/i-016-fsm-doctrine-convergence/index.md), Run completion
(`FDC-002`) at [i-020-run-completion](../../issue/i-020-run-completion/index.md), and input delivery
and durability (`FDC-003`) at
[i-019-input-delivery-durability](../../issue/i-019-input-delivery-durability/index.md).

Assumed dependency forecast, revocable at planning step 1: integrated Run residency precedes
input-routed transitions; input delivery follows input-routed transitions; Run completion is
routing-independent; machine composition follows the contracts it consumes. This note changes no
migration-cost finding and preserves the original route text as history.
