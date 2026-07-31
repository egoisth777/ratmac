# 07 — Conceptual Model: the deterministic/nondeterministic split

Wave-2 research, synthesis input 4 of 4. Date: 2026-07-29. Scope: the
conceptual constitution of the ratmac design as settled between the owner
and the lead session — what kind of thing the tool is, which plane each
piece of content lives on, and how judgment is admitted into routing.
Unlike `04-run-identity.md`, `05-invocation-join.md`, and
`06-migration-cost.md`, which propose, this file records settled doctrine
in a normative voice; ADR authority still stays with
`.arca/goal/design.md`, and §12 lists the only items left open.

**Reconstruction note** (2026-07-29, resolving the provenance-gap
finding, AR-12, in `08-adversarial-review.md`): parts of this file —
sections 3–4 and the connective prose — were reconstructed from working
context rather than transcribed verbatim from settled text; it records
positions, not yet doctrine. Per-section verification rulings live in
`08` §3: §2 survives only as an authored position; §3's closing tier-0
paragraph is demoted; §5's consumption and termination paragraphs are
demoted (the latter to guidance); §9 keeps only its route-kind check,
the other two demoted to open; §10 is demoted to a design position.
Substantive amendments await human rulings.

---

## 1. The four-artifact architecture

"Workflow" is a conflation of two things:

- **Control** — which motions are legal: deterministic, serializable,
finitely enumerable, checkable by a machine before any work exists.
- **Work** — what a motion means and how to perform it well:
nondeterministic, judgment-laden, expressible only as prose addressed
to an intelligence.

Storing both in one artifact forces either an engine that pretends to
understand prose or humans encoding judgment as fake structure until
the structure lies. ratmac refuses the conflation: four artifacts, each
with exactly one writer class and a known reader set.

1. **The runbook** (Machine Class, `.arca/ratmac.toml`, ADR-0004/0005).
 Human-written. Its *typed sections* — phases, edges, guards, verdict
 declarations — are the control surface. Its *manual prose* — the
 Phase Prompts (ADR-0009) — is the work surface.
2. **The Run** (State File `.arca/state.toml` plus Run evidence,
 ADR-0005/0008). Engine-written only. Position, history, bindings
 (05 §2), and the pinned runbook hash (04 §7.1) live here and
 nowhere else.
3. **Work artifacts on disk** — tickets, evidence receipts, verdict
 records, the code itself. Agent-written; the only channel by which
 agent output becomes visible to routing.
4. **The log** (`.arca/log.md`). Append-only journal. Every transition
 and every human confirmation leaves a trace here; nothing else does.

**Supersession — Run residency path (2026-07-29, ruling recorded as `FDC-004` of the run-residency
issue, `i-017-run-residency`):** item 2 above cites a single flat `.arca/state.toml`; the canonical
scheme is per-run: each Run's State File and evidence live under its own `.arca/runs/<run-id>/`
directory, one flat run-id namespace, no singular default path (adopted defaults,
`08-adversarial-review.md` §4 item 6, `AR-01`; batch human sign-off, recorded in design.md). Settled
position: `i-017-run-residency`'s spec.md, `FDC-004`.

## 2. What a state is

Classical FSM theory (Myhill–Nerode) defines a state as an equivalence
class of histories: everything about the past that matters to the
future. In ratmac the history's full detail lives on disk — artifacts,
log, evidence — so a state cannot and does not encode it. The settled
definition: **a state is a named equivalence class of adjudicated
progress** — not "what happened" (disk records that) but "what the
machine has ruled as satisfied." The state is the boundary marker
between facts-on-disk and facts-the-machine-acknowledges: an acceptance
cursor, not a snapshot of work. A state pins exactly three things:

- **Observation set** — its out-edge guard list: the lens through which
the machine looks at the world here.
- **Obligation** — which manual chapter the agent must read.
- **Permissions** — which verbs are legal here (e.g. spawn only in a
spawn phase).

Formally: δ : State × GuardVector → State, where GuardVector =
π_s(disk snapshot) — the projection of the disk through the *current*
state's out-edge guards, a finite bit-vector. The projection is
state-dependent: the same disk projects to different bits in different
states. This is *why* ratmac stays a finite-state machine: unbounded
world content is outsourced to disk; the state keeps only the finite
adjudication class. States carry no data — data-carrying states would
make δ an infinite table, a covert pushdown machine. Bindings belong to
the Run (write-once, 05 §2), content belongs to disk; states hold
acknowledgment only.

The author's cutting criterion: **cut a new state if and only if the
machine needs to be able to refuse or route there.** State count =
number of adjudication points, not number of work steps. Too fine:
node-as-function — a deterministic engine micromanaging
nondeterministic work, the graph-runtime failure mode. Too coarse: one
state for everything — nowhere to refuse, a regression to the
soft-constraint (hooks plus markdown docs) era.

## 3. The control plane: what the engine may decide

Control is a finite-state machine and nothing more. Machine state is
the Phase (ADR-0001). Ordinary routing is a function of the runbook and
the current phase alone: each phase has at most one ordinary successor,
and no lifecycle field of any Run ever selects among edges (invariant
restatement, 05 §6). Guards do not select; they *permit or refuse* the
already-determined successor. A refused step is not a branch — it is
the same step, later, under refuse/report/stay (ADR-0006).

Two constitutional properties. **Determinism given the filesystem:**
replaying `step` against the same tree of files yields the same
refusal-or-transition; guards may consult any on-disk fact that is
stable once satisfied, never their own Run's `status`, `blocker`, or
claim (05 §6). **Stability for the life of a Run:** the runbook hash is
pinned into the Run (04 §7.1; 06 §4), so a merge cannot swap the
machine mid-Run.

Conditional edges (tier 0, 06 §1) extend which-edge selection only
through declared, typed constructs — verdict-routed edges (§5) — never
through prose. Because control is typed, closed, and deterministic, it
is diffable, reviewable, and lintable before any Run exists (§9).

**Reversal note (2026-07-29, individual human ruling on `AR-03` of the doctrine-convergence issue,
`i-016`, superseding this file's own reconstruction note above):** the reconstruction note's
per-section table, quoting `08-adversarial-review.md` §3, characterized this paragraph as "demoted."
That characterization is itself superseded: Billy's later individual ruling ("Edge selection is
verdict-only") confirms the claim above as written — ordinary conditional edges never select, only
typed verdict edges do — and reinstates it as doctrine rather than an unverified position. The
paragraph that was actually wrong is `06-migration-cost.md` §1's general `when`-list tier-0
definition, which the same ruling states binds migration history only (see the supersession note
there). Settled position: `FDC-001` in the doctrine-convergence issue's spec.md; design.md's
"Individual human rulings," "Edge selection is verdict-only (`AR-03`)."

**Supersession — a branching phase selects by verdict (2026-07-30, propagating the verdict-routed
selection ruling: design.md's "Individual human rulings," "Edge selection is verdict-only (`AR-03`)"
of the doctrine-convergence issue, `i-016`, as sharpened by the closed-list ruling recorded in §9's
"Resolved (2026-07-29)"):** the first paragraph's sentence — "each phase has at most one ordinary
successor, and no lifecycle field of any Run ever selects among edges (invariant restatement, 05 §6).
Guards do not select; they *permit or refuse* the already-determined successor." — is superseded, and
the 05 §6 restatement it cites carries its own correction note. Under the ruling, a branching phase
declares a closed verdict input list and every ordinary outgoing edge carries exactly one unique
value from that list (§5 "Typing"; §9 "Resolved (2026-07-29)"): such a phase has several ordinary
out-edges, and guards select — the verdict-guarded edge whose value matches the live verdict is the
edge taken. Ordinary non-verdict guards remain readiness AND-gates — "can we move," never "where
to." Only for a straight-line phase, which may keep one unlabelled ordinary edge, does the sentence
read as before. Settled position: `FDC-001` in the doctrine-convergence issue's spec.md; design.md's
"Individual human rulings," "Edge selection is verdict-only (`AR-03`)."

## 4. The work plane: what the engine must not decide

Work is the complement: everything that requires understanding. The
engine never interprets narration. Phase Prompts are manual prose,
opaque to the CLI — read by the agent, never parsed for routing. The
engine's interface to the work plane is mechanical: agents may request
`next`; guards decide (ADR-0002); Subagents never touch the Scheduler
(ADR-0003).

For agents, one obligation: **output must land on disk to matter** — a
claim of completion routes nothing; a receipt with a digest, a ticket
record in schema shape, a child Run's terminal State File route
(PGE-001–PGE-007: a status edit can no longer route the loop). For the
engine, one prohibition: verify judgment's *artifacts* — hashes,
shapes, exit codes — never *simulate* judgment. The two planes meet
only at guards, and guards judge artifacts, not narration (05 §3): that
sentence is the trust model in miniature.

**Confirmed (2026-07-29):** the reconstruction note above lists no per-section ruling for this
paragraph, but `08-adversarial-review.md` §3's table separately flagged the clause "a child Run's
terminal State File route" as presuming the engine writes terminal statuses — a fact §5 stated but
04-06 left unauthorized. The individual ruling on `AR-05` ("The engine writes terminal statuses") now
authorizes exactly that presumption: the engine itself writes `passed` on arriving at a phase with no
ordinary out-edge, and `abandoned` via the abandon path. Settled position: `FDC-002` in the
doctrine-convergence issue's spec.md; design.md's "Individual human rulings."

## 5. Verdicts: the bridge from judgment into routing

Sometimes routing genuinely depends on the outcome of judgment: a
review phase must send the Run one way on "approve" and another on
"rework." The design admits this without breaking §3 by making the
judgment cross the boundary as a typed, on-disk fact. The settled
protocol has five parts.

**Typing.** A node's verdict value domain *is* its out-edge alphabet.
The FSM section declares a closed enum per adjudicated phase (e.g.
`values = ["pass", "rework_impl", "replan"]`), each out-edge guarding
on exactly one value. The judge's job: map evidence to one element of
the closed set, plus rationale; the manual prose explains each option.

**Addressing.** Verdicts are per decision point: one live slot per
phase, at an engine-computed path — e.g.
`.arca/runs/<run-id>/verdict/<phase>.toml`. The runbook guard declares
only `kind = "verdict"` plus the expected value; the engine derives the
path from (run, phase); paths are computed in code, never written in
data (the bindings principle, 05 §2). Per-phase verdict files are
intended: each state's out-edges read only its own slot (§2: the state
selects the lens). Slot residency follows Run state residency (04 §4;
NRI in 06 §5). In fan-out, children's verdicts live in their
own run directories; the parent join reads engine-witnessed child
terminal state, never a shared pool.

**Consumption.** When a transition fires, the engine — as part of δ's
deterministic effect, the same tier as writing the State File —
archives the consumed verdict into the Run's append-only evidence
(which edge, when, on what verdict) and clears the slot. At any moment
a phase has at most one live verdict, this iteration's; history is all
on the ledger. Rework verdicts are thereby re-earned every iteration
*mechanically*, so the stale-verdict infinite loop cannot occur. This
reconciles with the stable-once-true invariant (05 §6): that invariant
covers *foreign terminal* facts — a child's `passed`, once true,
forever true — while a rework verdict is a ruling on one iteration of
work; its stability is scoped to that iteration, and the scope ends
when the edge fires. The archived-record format is open (§12); the
archival requirement is not.

**Supersession — verdict consumption is one atomic motion (2026-07-29, individual human ruling on
`AR-06` of the doctrine-convergence issue, `i-016`, refined 2026-07-29, review fold-in S5):** the
paragraph above describes archiving and clearing the slot as part of δ's effect but leaves the exact
ordering against the State File write unstated — that gap is `08-adversarial-review.md`'s
crash-safety tension, `AR-06`. Billy ruled: archive the consumed verdict and clear the slot *before*
the State File write, as one atomic filesystem motion — validate the live verdict, atomically rename
its slot into immutable Run evidence (same filesystem as the slot; this is load-bearing on Windows,
where cross-volume rename is not atomic), derive a collision-free evidence name from the Phase and the
next on-disk attempt number, and only then write the successor State File. If interrupted after the
rename but before the State File write, the Run stays in the old Phase and needs a fresh verdict — the
archived verdict never replays, restoring (under this specific ordering) the impossibility claim this
section already asserts. Settled position: `FDC-003` in the doctrine-convergence issue's spec.md;
design.md's "Individual human rulings," "Verdict consumption order (`AR-06`)" and its S5 refinement;
this also answers open question 4 below (archived into Run evidence, not the log).

**Two expressions of "bad work."** Refusal: no edge passes, the Run
stays; meaning "not finished yet" — the cheapest implicit loop, no edge
drawn. Explicit rework edge: verdict = rework, a *recorded*
adjudication event — log entry, an iteration boundary, possibly
resetting artifacts. Criterion: missing things → refusal; a formal
ruling of "complete but unacceptable" → rework edge.

**Termination.** The engine never counts iterations — counting is
engine memory and violates purity. Loops terminate only via exit-edge
guards over *monotone* facts (evidence accumulates and never retreats)
or a human abandon verb (§7). Structure does not guarantee termination;
monotone acceptance does.

**Supersession — termination is a checkable guard-kind rule, not "monotone" (2026-07-29, adopted
default, `08` §4 item 13, of the doctrine-convergence issue, `i-016`; `AR-07`):** "monotone facts"
above names a property the guard vocabulary cannot check — monotonicity is not a property the
vocabulary exposes. The adopted replacement is mechanical: every phase on a cycle needs one out-edge
guarded by receipt- or contract-class guards only; that is what doctor lints, not monotonicity.
Settled position: `FDC-008` in the machine-composition issue's (`i-018-machine-composition`) spec.md;
design.md's "Adopted defaults" item 13.

## 6. Who may judge: separating worker from verdict

A guard whose verdict rests on content the agent under test can write
proves less than one that does not — the runbook-spec already treats
this as a reportable smell (`RB302`). Applied to verdicts, the
principle is **worker ≠ judge**: the party that produced the work
should not be the party that writes the verdict record on it.

Two realizations, in order of strength:

1. **Child-as-reviewer (v1).** A separate child instance (05 §§3–4)
 whose only work product is the verdict record; the parent's phase
 joins on it. No new engine surface, but the separation is protocol
 discipline: nothing stops a misconfigured runbook from letting the
 worker write the slot.
2. **Engine-witnessed verb.** A future `rtm verdict` verb through which
 the verdict enters at the CLI: the engine records the signer
 identity, checks signer ≠ worker, and writes the record itself,
 making the separation mechanical rather than customary — the same
 hardening move the confirmation verbs already made for human acts
 (§7).

Which lands first, and whether the witnessed verb is worth its
identity machinery, is open (§12). Not open: self-review is weak
evidence; where tolerated, it is declared and visible.

**Resolved (2026-07-29):** child-as-reviewer (v1) lands first; the witnessed verdict verb is deferred,
since it needs signer identity, which `ORS-001` deliberately keeps out of the Engine — the "worth its
identity machinery" question is answered by deferral, not by a rejection (adopted default, `08` §4
item 17, batch human sign-off, recorded in design.md; settled position: `FDC-010` in the
machine-composition issue's, `i-018-machine-composition`, spec.md). This also resolves open question 1
and question 5 below.

## 7. Human judgment: confirmation verbs and exceptional motion

Human judgment enters the system as **confirmation verbs**: an exact
phrase typed at invocation — never read from a file an agent can write
(schema, blocked route; ORS-001 keeps the engine free of caller
identity, so the typed phrase *is* the authorization).

Confirmation verbs are the **only source of exceptional motion**:
`hold` takes a blocked route that `rtm step` would never take, marking
the ticket `held` with its `blocker-ref` (PGE-006); `abandon` ends a
Run outside its graph, with rollback if interrupted (PGE-007 posture:
unconfirmed refuses before the first write). Every such act appends its
trace to the log, so exceptional motion is exactly as replayable as
ordinary motion. The symmetry: ordinary motion needs no human — graph
and artifacts suffice; exceptional motion needs nothing *but* a human —
no accumulation of agent output, statuses, or elapsed time ever adds up
to a hold or an abandonment.

## 8. Why the split must exist

The root theorem: **determinism is a minimum, not an average.** One
nondeterministic component on the control path makes the whole control
path nondeterministic — there is no "mostly deterministic," as there is
no "mostly memory-safe." A graph runtime that lets a model route
control forfeits every determinism claim at that moment. Total
separation is therefore not a style choice but a logical necessity. The
split purchases six goods:

1. **An unpersuadable "no."** An LLM's power comes from persuadability,
 which disqualifies it as judge forever; constraints enforced by a
 persuadable entity are suggestions — the lesson of the
 hooks-plus-docs era. Guards are unpersuadable: they cannot read
 narration at all.
2. **Replay: science, not anecdote.** Same snapshot, same edge (§3);
 incident review is reading a ledger, not statistically re-running a
 process.
3. **Bounded trust surface.** Because the state is an adjudicated
 summary (§2), trusting "the Run is at P4" costs O(out-edge guards),
 not O(everything the agent ever did).
4. **Amnesia immunity.** LLMs are worst at long-horizon bookkeeping;
 the split externalizes "where are we, what is accepted" into engine
 state, so any fresh agent can be dropped in mid-flight: read state,
 read chapter, continue. Comparative advantage: machines do exact
 bookkeeping and pitiless refusal at zero tokens and zero latency;
 LLMs do judgment and work.
5. **Pluggable judge slots.** Files-not-function-calls as the interface
 makes actor swapping free (§6).
6. **Write-time guarantees.** The runbook is finite plain data, so
 doctor statically checks reachability, dangling verdict values, and
 exhaustiveness in polynomial time before any Run exists (§9). You
 cannot statically analyze a prompt.

Honest boundaries: (i) tasks shorter than the ceremony don't deserve
a machine — mitigated, not erased, by the class library (§10); (ii)
work with no articulable adjudication points cannot support an FSM —
forcing one produces guard-less ritual; (iii) the engine guarantees
*process integrity*, not *outcome quality* — a garbage verdict is
routed faithfully. The split does not eliminate judgment risk; it
*locates* it — who, when, on what evidence, recorded where. The
accurate slogan is not "better decisions" but "every decision has an
address."

The moat: the direction is irreversible. A node-as-function engine
cannot retrofit "narration cannot move state" without destroying its
own API, because its nodes *are* the workers; ratmac starts on the far
side of that wall. Refusal-first is already in the codebase's bones —
locks, refuse-and-instruct, guards-judge-artifacts precedents — and
self-hosting (ratmac develops ratmac; the P1–P5 sprint machine is the
first runbook, 05 §7) is the standing proof.

Closing, near verbatim from the conversation: the LLM is the first
component in software history with extreme capability and zero
reliability, while every prior architecture assumed components are
reliable; the separation is not fencing a process — it is turning
"process" itself from prayer into physics.

## 9. Static checkability: what doctor can prove before any Run

Because control is typed and closed, `rtm doctor` can check the whole
machine before any work exists (§8, good 6). On top of shape checks
and the proposed structure-discipline family (RB5xx, 05 §5), the
verdict construct (§5) makes three checks statable:

- **Verdict exhaustiveness, both directions.** Every declared verdict
value must have a covering edge — else a verdict can strand the Run —
and every verdict-guarded edge must name a declared value. This also
closes the doctor gap recorded in 06: two ordinary out-edges to
different destinations are today un-linted; with typed verdicts,
out-edge divergence must be carried by the value domain, and silent
first-passing-edge misroutes are blocked by construction.
- **Order coherence.** Guarded edges evaluate in declaration order,
first passing edge wins; the lint must reason about shadowing, not
merely coverage.
- **Route-kind separation.** Blocked routes are never taken by `step`
(PGE-006), so they neither satisfy exhaustiveness nor participate in
first-passing order; a runbook that leans on a blocked route to
"cover" a verdict value is routing human exception as ordinary
motion, and doctor should say so.

Severity (error versus warning) and mixed verdict/non-verdict edge sets
are open (§12). The direction is doctrine: every construct added to
control arrives with its static checks, or it is not control — it is
work wearing control's clothes.

**Resolved (2026-07-29):** superseded twice over — first by the batch default (`08` §4 item 5:
exhaustiveness errors, mixed verdict/non-verdict edge sets allowed with a warning), then by a later,
stricter human ruling (review fold-in S3) that supersedes that default outright: a branching phase
declares one closed verdict input list; every ordinary outgoing edge carries exactly one unique value
from it; missing coverage, duplicate coverage, and out-of-list values are all errors (not warnings);
mixed labelled and unlabelled ordinary edges are forbidden; a straight-line phase may keep one
unlabelled ordinary edge; blocked routes stay outside selection and exhaustiveness checks. Settled
position: `FDC-001` in the doctrine-convergence issue's spec.md; design.md's superseded-item-5 note.
This also resolves open question 3 below.

## 10. Composition: engine, machines, tasks

The system is three layers: **ratmac : machine : task :: kernel :
program : process.** The engine parses many classes and runs many
instances, knowing nothing of any particular machine. "The system" =
engine + class library + protocol (the skill) — not any single machine.
A single giant machine is rejected on five counts: one global scalar
position, zero parallelism, global coupling on every process change,
doctor blowup, and the full ceremony tax on short tasks. Small machines
form a library, as functions against one giant main().

**spawn/join is the composition primitive** (05 §§3–4), not merely
parallelism. Subroutine call = a phase spawning exactly one child
(min = 1), joining on its terminal: the entire child machine appears to
the parent as one node. Parallel composition = N children, join on all.
One primitive, both forms, zero new mechanism. The protocol
unification, stated exactly: **to the parent, a child machine is a
large judge.** Node scale: a judge reads evidence and writes a verdict
record (§§5–6). Machine scale: a child run completes and leaves an
engine-witnessed terminal record (05 §4). A child's terminal vocabulary
*is* its out-edge alphabet when viewed as a node — the same concept as
a phase's verdict value enum, at two scales.

**Machine signature.** In-parameters = the binding schema (write-once,
05 §2); out-parameters = the terminal vocabulary
(succeeded/failed/abandoned). Zero other coupling: the child never
reads the parent's workspace, the parent never inspects the child's
process; values in, terminal plus disk artifacts out — function-call
semantics.

**Supersession — fixed terminal enum, not per-class variability (2026-07-29, individual human ruling
on `AR-04` of the doctrine-convergence issue, `i-016`, refined 2026-07-29, review fold-in S4):** two
corrections to the text above. (1) The closing sentence of the preceding paragraph — "a child's
terminal vocabulary *is* its out-edge alphabet when viewed as a node... the same concept as a phase's
verdict value enum" — reads as implying each machine class may define its own terminal vocabulary, the
way each phase defines its own verdict enum; that implication is superseded. Terminal status values
are one fixed engine-level enum shared by all runbooks, never a per-class alphabet; richer,
class-specific outcome semantics live in verdicts and evidence, never in the terminal status itself.
(2) The "Machine signature" bullet's parenthetical, `(succeeded/failed/abandoned)`, both misnames and
overstates the enum: the settled spelling is `passed`, and `failed` is deferred — no failure command
exists yet, and guard refusal is never reinterpreted to populate it, since refusal must leave Run state
unchanged; only `passed` and `abandoned` are live today. Settled position: `FDC-002` in the
doctrine-convergence issue's spec.md; design.md's "Individual human rulings," "One fixed terminal enum
(`AR-04`)" and its S4 refinement.

**Run-as-call, not graph flattening.** Inlining a sub-machine's graph
into the parent loses four things: the child's independent ledger and
identity (04 §3), its own worktree, N-instance parallelism of the same
sub-machine (flattening needs N copies), and interface discipline (a
shared namespace is coupling smuggling). Run-as-call keeps all four
and keeps every machine small. The print-first dividend (ADR-0010): a
parked parent costs zero compute — no daemon exists; parked is a line
of disk state — so deep nesting is nearly free. Composition is cheap
precisely because the engine never runs anything.

**Short tasks** (revising §8's boundary (i)): with a class library, a
short task instantiates a small three-phase machine — fix-bug:
reproduce → fix → verify. The ceremony floor drops and authoring cost
amortizes across reuse; a long task is a top-level machine whose phases
spawn small machines — same engine, same semantics, scale-free.

**Discipline, three rules.** (1) The class-reference graph must be
acyclic — no recursion; doctor RB-family lints (05 §5). (2) Coupling
only through the signature; everything else is smuggling. (3) A child
pins its class hash at spawn, reusing the runbook-pin mechanism (06
§4) — otherwise the class can be edited while the parent is parked, and
the join would judge a different machine than was spawned.

Design verdict for ratmac: the CLI must be a generic engine —
multi-class parsing, named runs, generalized guards — not a treadmill
for the sprint machine. The route order stands (CGE → RBP → NRI → CHI,
06 §5), with CHI elevated: not a parallelism feature but *the*
composition primitive — the load-bearing wall of the
machines-as-library vision.

**Supersession — the route order does not stand (2026-07-30, propagating the accepted route
inversion of the dependency-strata split):** the sentence above — "The route order stands (CGE → RBP
→ NRI → CHI, 06 §5)" — is superseded, together with 06 §5's order itself (see the supersession note
there). The accepted route (human ruling, 2026-07-29, recorded in the doctrine-convergence issue's
design.md, "Dependency route", and restated in the run-residency and machine-composition issues'
spec.md): run residency (`i-017-run-residency`) lands first; the verdict-routed execution core
(`i-016-fsm-doctrine-convergence`) depends on it; machine composition (`i-018-machine-composition`)
depends on both. CHI's elevation as the composition primitive is unaffected.

## 11. The placement table

The constitution compresses to one table: for any piece of content,
who writes it and who reads it; no per-case doctrine is needed.


| Content                                             | Lives in                | Written by                     | Read by                                       |
| --------------------------------------------------- | ----------------------- | ------------------------------ | --------------------------------------------- |
| FSM structure: phases, edges, guards, verdict enums | runbook, typed sections | humans                         | CLI + doctor                                  |
| Work instructions                                   | runbook, manual prose   | humans                         | agent (opaque to the CLI)                     |
| Agent judgments: verdict records, receipts          | on-disk artifacts       | agent / judge (§6)             | guards                                        |
| Human judgments                                     | confirmation verbs      | the human, typed at invocation | engine at that instant; log traces after (§7) |
| Position, history, bindings, runbook hash           | the Run                 | engine only                    | CLI, guards, humans                           |


Read by column: one writer class per row; the CLI and doctor see only
typed structure; the agent alone reads prose; guards see only disk.
Every trust argument in files 04–06 defends one cell of this table.

## 12. Open questions

1. **Witnessed verdict verb:** should the engine-witnessed `rtm  verdict` verb with signer ≠ worker checking (§6) be pursued, or is
 worker/judge separation left to protocol discipline — prompts plus
 child-as-reviewer wiring? **Resolved (2026-07-29):** for now, left to protocol discipline —
 child-as-reviewer lands first; the witnessed verb is deferred, not rejected, since it needs signer
 identity that `ORS-001` keeps out of the Engine (see the resolved pointer at §6; adopted default,
 `08` §4 item 17, recorded in design.md; settled position `FDC-010`, `i-018-machine-composition`).
2. **Verdict slot paths:** what is the exact verdict slot path scheme,
 and how does it interact with per-run state residency (04 §4; the
 NRI series in 06 §5)? **Resolved (2026-07-29):** per-run, at an engine-computed path under the
 run's own directory — canonical residency is `FDC-004` (`i-017-run-residency`); see the residency
 supersession note at §1.
3. **Exhaustiveness lint scope:** is the verdict-exhaustiveness check
 an error or a warning, and how does it interact with
 declaration-order first-passing semantics and with `blocked_route`
 (PGE-006)? **Resolved (2026-07-29):** error, not warning, and stricter than either prior draft —
 see the resolved pointer at §9: a closed verdict input list, exhaustive by construction, with
 blocked routes staying outside both selection and exhaustiveness (`FDC-001`, review fold-in S3,
 recorded in design.md).
4. **Archived-verdict record:** does the archived verdict record format
 (§5) join `.arca/evidence.toml`, or the log? **Resolved (2026-07-29):** Run evidence, never the
 log — see the consumption supersession note at §5 (`FDC-003`; `AR-06`, refined review fold-in S5,
 recorded in design.md).
5. **Sequencing:** child-as-reviewer (v1) versus the witnessed verb —
 which lands first? **Resolved (2026-07-29):** child-as-reviewer (v1) first; the witnessed verb is
 deferred — see the resolved pointer at §6 (adopted default, `08` §4 item 17, recorded in design.md;
 `FDC-010`).
6. **Mixed routing:** may a phase declare BOTH a verdict enum and
 non-verdict guarded edges, and what does doctor say when it does? **Resolved (2026-07-29):** no — a
 branching phase's ordinary out-edges must be entirely verdict-labelled (one unique value each) or,
 for a straight-line phase, one unlabelled edge; mixed labelled/unlabelled sets are forbidden by
 construction, and doctor's exhaustiveness/duplicate/out-of-list checks are errors — see the resolved
 pointer at §9 (`FDC-001`, review fold-in S3, recorded in design.md).


---

## Supersession note — 2026-07-30 atomic cut

Billy split the pending execution bundle while preserving the rulings reconstructed in this file.
Current requirement homes are:

- input-routed transitions (`FDC-001`) —
  [i-016-fsm-doctrine-convergence](../../issue/i-016-fsm-doctrine-convergence/spec.md);
- Run completion (`FDC-002`) —
  [i-020-run-completion](../../issue/i-020-run-completion/spec.md);
- input delivery and durability (`FDC-003`) —
  [i-019-input-delivery-durability](../../issue/i-019-input-delivery-durability/spec.md).

Accordingly, earlier sentences saying `FDC-002` or `FDC-003` live in the doctrine-convergence
issue's specification are historical and point through its split record. The class's legal input
list, one transition input value, the live judge-authored verdict record, and the archived verdict
are now distinct terms. The accepted `Verdict slot` name remains unchanged. Witnessed judgment
remains deferred, and judge independence remains in the machine-composition issue.
