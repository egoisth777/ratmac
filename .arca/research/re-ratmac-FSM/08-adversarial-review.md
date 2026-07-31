# 08 — Adversarial review of the wave-2 design body

Wave-2 research, adversarial pass over topics 1–4. Date: 2026-07-29. Scope: `04-run-identity.md`,
`05-invocation-join.md`, `06-migration-cost.md`, `07-conceptual-model.md` read as one body of design and
attacked for contradiction, invariant violation, doctrine collision, unverified reconstruction, and
implementation-blocking gaps. Nothing here is a decision. ADR authority stays with `.arca/goal/design.md`;
format authority stays with `.arca/runbook-spec.md`. Every code claim below was opened before it was cited.

---

## 1. Verdict summary

The four documents are individually strong and jointly unbuildable as written. Three hard contradictions sit
on the critical path: the four files use three incompatible run-residency layouts, two incompatible
run-addressing surfaces, and two incompatible definitions of what a conditional edge *is* — and the third of
those breaks the route order all four files claim to share. Below that, one whole class of design (the
relational join of `05`, the composition story of `07`) is specified over Run statuses that no code path
writes and no decision has authorized. The core invariants survive better than the artifacts do: determinism
given a filesystem snapshot holds everywhere I could test it, and the join's reconciliation with the
no-routing-on-lifecycle rule is sound. What does not survive is *monotonicity* — `07`'s verdict protocol
makes a fact deliberately non-monotone and then grounds loop termination in monotone facts, with no
mechanism able to tell the two apart. Finally, `07` presents reconstructed material in a normative voice
with no provenance marker anywhere in the file, which is the single most dangerous property of the set,
because `07` is the file whose sentences read like requirement text.

| ID | Severity | Finding | Implicated |
| :-- | :-- | :-- | :-- |
| AR-01 | CONTRADICTION | Three run-residency layouts: `.arca/runs/<id>/`, `.arca/run/<name>/`, and a spawn-name namespace that collides with the run-id namespace | 04 §2.1, §3.3; 05 §1, §3; 06 §2; 07 §1, §5 |
| AR-02 | CONTRADICTION | Run addressing: flag-only-and-always-required versus positional-and-optional-when-unique | 04 §2.3, §9.2; 05 §1; 06 §2, open Q4 |
| AR-03 | CONTRADICTION | Two definitions of tier 0; `07`'s version needs per-run residency and so breaks the shared route order | 06 §1, §5; 07 §3, §9 |
| AR-04 | CONTRADICTION | Terminal vocabulary declared both fixed and class-declared in one paragraph; neither matches `Status` | 05 §4; 06 §3; 07 §10; `src/model.rs:12-18` |
| AR-05 | GAP | The join is specified entirely over `passed`/`failed`, which no code writes and no decision authorizes | 05 §3, §4; 06 §3, open Q3 |
| AR-06 | TENSION | Verdict consumption adds two writes to an already non-atomic sequence; a crash re-fires a stale verdict | 07 §5; `src/scheduler.rs:401-435` |
| AR-07 | GAP | Termination rests on "monotone facts"; the guard vocabulary has no monotone class and doctor cannot check one | 07 §5, §8, §9 |
| AR-08 | TENSION | The spawn ledger sits at registry level, where `04` forbids a second source of truth | 04 §3.3; 05 §3, open Q2 |
| AR-09 | GAP | The one-active-Run cap and id reuse are open in `04` yet silently answered by `05` and `07` | 04 §9.1, §9.6; 05 §1, §4; 07 §10 |
| AR-10 | GAP | Runbook-format supersession never claimed; the self-hosting runbook would refuse today | 05 §1, §7; `.arca/runbook-spec.md:9-22, 44` |
| AR-11 | GAP | New human verbs arrive without an authorization model; the cited abandon form drops its confirmation | 04 §2.2; 05 §1, §4; `.arca/schema.md`, PGE-007 |
| AR-12 | GAP | `07` carries no provenance marker distinguishing settled doctrine from reconstruction | 07 front matter, §§3-5, §10 |
| AR-13 | VERIFIED | The join's reconciliation with no-routing-on-lifecycle is sound as argued | 05 §6; `src/graph.rs:131-144`; `src/machine.rs:480-482` |
| AR-14 | VERIFIED | Route-kind separation for blocked routes agrees across files and with code | 06 open Q6; 07 §9; `src/graph.rs:139-144` |

---

## 2. Findings in detail

### AR-01 — CONTRADICTION: three run-residency layouts

`04 §2.1` proposes `.arca/runs/<run-id>/` holding `state.toml`, `log.md`, `rtm.lock`, `evidence.toml`, with
ids matching `^[a-z0-9][a-z0-9-]{0,63}$` (`04 §3.2`). `06 §2` proposes `.arca/run/<name>/` — singular
directory — holding the same four files plus a pinned class copy. `07 §5` writes verdict slots to
`.arca/runs/<run-id>/verdict/<phase>.toml` (plural, agreeing with `04`), while `07 §1` simultaneously names
the Run as "State File `.arca/state.toml`", the flat v1 path. That is three spellings in four files.

Worse than the spelling: `05 §1` uses the plural form but puts a *spawn name* in the first segment —
child roots at `.arca/runs/tickets/t-042/` and a ledger at `.arca/runs/tickets/ledger.toml` (`05 §1` step 3,
`05 §3`). `tickets` satisfies `04 §3.2`'s id grammar exactly, so under `04` the doctor enumerating
`.arca/runs/*/state.toml` (`04 §2.2`, last row) reads `.arca/runs/tickets/` as a malformed run, and
`04 §3.3`'s rename-mint can be asked to mint an id that is already a spawn namespace. Two schemes are
competing for one flat namespace.

Why it matters: `07 §5`'s whole addressing argument is "the engine derives the path from (run, phase);
paths are computed in code, never written in data". A path computed in code from a scheme nobody has chosen
is not computed — it is guessed at three sites.

**PROPOSED canonical scheme.** Plural `runs`, one flat id namespace, everything else nested *under* a run:

```
.arca/runs/<run-id>/            # git-ignored; every direct child is a Run, nothing else
  state.toml  log.md  rtm.lock  evidence.toml
  verdict/<phase>.toml          # live verdict slot (07 §5)
  spawn/<spawn-name>/ledger.toml  # this Run's expected child set (05 §3)
```

Child Runs are ordinary top-level Runs; the ledger entry carries the child's run id plus its binding map
rather than owning the child's directory. This keeps `04 §3.3` literally true, removes the path-shaped id of
`05 §1`, gives `06` a single spelling, and puts the verdict slot where `07 §5` already wanted it. `06`'s
singular `run` is the minority spelling (one file against three) and is the one to drop.

### AR-02 — CONTRADICTION: run addressing surface

`04 §2.3` tabulates three options and rejects positional explicitly: "Collides with the established
positional-path conventions: `rtm doctor <path>` … and `rtm scaffold <path>`. … Reject." `06 §2` then
specifies exactly the rejected form: "`rtm start <name>` creates it; `rtm step <name>` / `rtm status <name>`
address it." `05 §1` uses the flag form `rtm step --run tickets/t-042` — consistent with `04`'s choice but
carrying a value with a path separator, which `04 §3.2`'s grammar forbids.

The optionality question is contradicted in the same place. `04 §2.3` records that ADR-0007 called
"default run when unambiguous" the avoided footgun (`.arca/goal/design.md:61`) and flags relaxation as open
(`04 §9.2`). `06 §2` asserts the relaxation as settled — "the argument-free forms stay legal only while
exactly one Run exists" — and then re-opens it as its own open question 4. A document that both asserts and
questions the same fact cannot be implemented from.

Verified against code: `src/cli.rs:135-142` today refuses with the literal message
`"{command} accepts no run-id or extra arguments"` for `start | status | step`, so both proposals are new
surface and one of them must rewrite that refusal.

**PROPOSED.** Keep `04`'s reasoning (flag, always required, refusal lists the roster) because it is the only
option that leaves recorded transcripts self-describing, which the behavioral-evidence requirement
(`ORS-003`, `.arca/goal/spec.md`) depends on. Treat `06 §2`'s positional form as superseded prose, not a
rival proposal.

### AR-03 — CONTRADICTION: what a conditional edge is, and the route order it breaks

`06 §1` defines tier 0 precisely: transitions gain an optional `when` list holding *the existing guard
vocabulary* — "No new predicate language: the seven guard kinds … already express file shape, file content,
command exit, receipts, and contracts" — with declaration-order first-passing-edge routing.

`07 §3` defines it differently: "Conditional edges (tier 0, 06 §1) extend which-edge selection only through
declared, typed constructs — verdict-routed edges (§5) — never through prose." Read strictly, that says the
*only* conditional edges are verdict-routed ones. `07 §9` then builds on that reading: "with typed verdicts,
out-edge divergence must be carried by the value domain, and silent first-passing-edge misroutes are blocked
by construction." Under `06`'s tier 0 that sentence is false — out-edge divergence may be carried by any of
the seven guard kinds, and first-passing-edge misroutes are exactly what `06 §1` point 4 proposes new doctor
lints for, precisely because they are *not* blocked by construction.

The consequence is structural, not verbal. `07 §5` addresses verdict slots under the per-run directory,
which is the named-run-residency bundle (`NRI`), third in the route. So `07`'s tier 0 depends on the third
bundle, while all four files repeat the order conditional-guarded-edges (`CGE`) → runbook-pin (`RBP`) → `NRI`
→ child-instances (`CHI`) — `06 §5` states it, `07 §10` affirms "the route order stands". Either `CGE` ships
`06`'s general `when` (and `07 §9`'s exhaustiveness argument does not apply to it), or `CGE` ships verdicts
(and `CGE` now depends on `NRI`, and the route order is wrong). This is the sharpest defect in the set.

**PROPOSED.** Ship `06`'s general `when` as `CGE`, and treat the verdict enum as a *later, narrower* guard
kind layered on top — a `verdict` guard kind is just an eighth entry in the vocabulary, usable inside a
`when` list, whose slot path needs per-run residency. Then `07 §9`'s three checks become `NRI`-era or
`CHI`-era doctor lints scoped to phases that declare a verdict enum, and the general shadowing lints of
`06 §1` point 4 carry the `CGE` era alone.

### AR-04 — CONTRADICTION: terminal vocabulary

Ground truth: `src/model.rs:12-18` declares `Status` as exactly `Planned | Executing | Blocked | Passed |
Failed`. There is no `Abandoned`. `src/abandon.rs` does not write a terminal status at all: it appends a
terminal event to the history (`src/abandon.rs:129`) and then *removes* `.arca/state.toml`
(`src/abandon.rs:108`, `:222`), so an abandoned Run has no State File rather than a terminal one.

`05 §4` handles this correctly: its table maps a *parent-view* vocabulary (succeeded / failed / abandoned)
onto on-disk facts, with `abandoned` living in the ledger rather than in any status field. `07 §10`
flattens the distinction twice in one paragraph. First it fixes the out-parameter set: "out-parameters = the
terminal vocabulary (succeeded/failed/abandoned)". Then it declares the same set open: "A child's terminal
vocabulary *is* its out-edge alphabet when viewed as a node — the same concept as a phase's verdict value
enum". A vocabulary cannot be both a fixed triple and whatever the class author declared.

**PROPOSED.** Adopt `05 §4`'s two-level statement as canonical and delete the fixed triple from `07 §10`:
the engine's status vocabulary stays the five values in `src/model.rs`; the parent-view terminal alphabet of
a child class is derived (terminal phase reached, with status `passed`), and `abandoned` is a ledger fact,
never a status.

### AR-05 — GAP: the join is built on statuses nothing writes

`05 §3` evaluation step 2 requires the child's `status = "passed"` at a graph-terminal phase; `05 §4` maps
the failed case to `status = "failed"`. A grep of the whole source tree for status writes returns exactly
four sites: `src/model.rs:161` and `src/scheduler.rs:823` write `Blocked`; `src/scheduler.rs:251` and `:298`
write `Planned`. No code path writes `Passed` or `Failed` — `06 §0` states this and it verifies.

So the entire truth-domain of the join is currently unwritable, and whether it should become writable is
`06`'s open question 3, undecided. `05` does not list this as a dependency anywhere, including in its tier
dependency summary. Any attempt to implement `05 §3` stops here.

**PROPOSED.** Decide the terminal write inside `CGE`, not `CHI`: `06 §1` already argues that the smallest
shippable value of tier 0 is a cyclic class that terminates, and a loop that terminates with no observable
end is not observably terminated. Writing `passed` when a step arrives at a phase with no ordinary out-edge
is one condition and one write, and it makes `CGE`'s acceptance fixture checkable without inventing an
oracle.

### AR-06 — TENSION: verdict consumption versus crash safety

`07 §5` requires that "when a transition fires, the engine — as part of δ's deterministic effect … archives
the consumed verdict into the Run's append-only evidence … and clears the slot", concluding that "the
stale-verdict infinite loop cannot occur". That conclusion is exactly as strong as the atomicity of the
clear, and today there is none.

`src/scheduler.rs` orders one step's writes as: freeze evidence (`:391-398`, only on a freezing edge), then
the State File (`:401`), then the history append (`:406-413`). The rollback at `:414-435` truncates the
history to its recorded length and restores the prior state — but it runs only when the append call
*returns an error* (`if let Err(append_error) = append_result`). Process death is not an error return.
So the current sequence already has a crash window in which the State File has advanced and the history has
no line for it. `07 §5` proposes to append two more writes to that sequence.

The damaging window is specific: crash after the State File advances and before the slot is cleared. The
Run is now at the successor phase with a live verdict still sitting in the predecessor's slot. On a rework
loop — the only shape verdicts exist for — the machine returns to that phase and the stale verdict fires the
same edge with no new work. That is the precise failure `07 §5` says is mechanically impossible.

`06 §0` and `04 §1.5` both note the truncate-to-old-length rollback and correctly identify it as
single-writer-only; neither connects it to verdict consumption, because neither was written against `07`.

**PROPOSED ordering, grounded in the repo's own precedent.** `src/scheduler.rs:385-388` records the rule for
exactly this class of problem: "Evidence is written before the State File, so an interrupted freeze leaves
the Run unchanged rather than half-frozen." Apply it: archive the verdict, clear the slot, *then* write the
State File. A crash then leaves a Run that has not advanced and a verdict that must be re-earned — a lost
verdict, which costs one review, versus a silent re-fire, which costs correctness. Refusal is the default
answer to everything unproven; this ordering is that rule in write order.

### AR-07 — GAP: "monotone facts" is unenforceable

`07 §5` closes with a termination theorem: "Loops terminate only via exit-edge guards over *monotone* facts
(evidence accumulates and never retreats) or a human abandon verb. … Structure does not guarantee
termination; monotone acceptance does." `07 §8` good 6 then claims doctor "statically checks reachability,
dangling verdict values, and exhaustiveness in polynomial time".

Monotonicity is not among them, and cannot be. Of the seven guard kinds (`src/machine.rs:28-55`),
`files_exact` and `file_contains` read agent-writable content — a file that exists can be deleted, which is
why the runbook specification already flags them as a reportable smell (`RB302`, `.arca/runbook-spec.md:111`,
implemented at `src/doctor.rs:382`). `command_exit` is non-monotone by definition. The receipt and contract
gates are the closest to monotone, and even they retreat when a receipt goes stale by construction, which
the completion gate deliberately supports (`.arca/schema.md`, completion gate: "Edit the work after the check
and the receipt goes stale by construction").

Meanwhile `07 §5` makes the verdict itself deliberately non-monotone: the slot is cleared on consumption.
So the same section both requires monotone facts for termination and introduces the design's only
intentionally non-monotone fact. Both can hold only under an unstated rule — *rework edges may be
verdict-guarded, exit edges may not* — which no file states and no lint could check without a monotone-class
taxonomy that does not exist.

**PROPOSED.** Demote the termination sentence from theorem to authoring guidance until a monotone guard
class is defined, and state the missing rule explicitly if verdicts land: at least one out-edge of every
phase on a cycle must be guarded by receipt- or contract-class guards only. That is checkable — it is a
membership test over kinds — where "monotone" is not.

### AR-08 — TENSION: spawn ledger versus the no-registry rule

`04 §3.3` is categorical: "No registry file. A `runs.toml` index would be a second source of truth that can
disagree with the directory … The listing of `.arca/runs/` is the registry." `05 §3` introduces
`.arca/runs/<spawn>/ledger.toml` and defends it on the grounds that disagreement is the *point*: "A deleted
child directory must make the join refuse loudly … not silently shrink N."

These are reconcilable, and the reconciliation is entirely a placement question. `04`'s rule governs global
run *enumeration*; `05`'s ledger fixes one parent's *expected child set*, which is a Run artifact, not a
registry. As written, though, the ledger sits at registry level in the same namespace as run ids, which is
where `04`'s rule bites. AR-01's proposed layout moves it to `.arca/runs/<parent-id>/spawn/<name>/ledger.toml`
and the tension disappears without either file conceding a principle. `05`'s own open question 2 already
asks about placement; it just does not know it is answering `04`.

### AR-09 — GAP: the cap and id reuse are open, and silently answered

`04 §9.6` leaves open whether the one-active-Run cap lifts entirely or becomes configurable; `04 §9.1` leaves
id reuse after abandon open and suggests never reusing. Both are then assumed away downstream.

The cap: `05 §1` states children "step concurrently without touching the parent's lock", and `05 §7`'s
walkthrough runs a parent plus three children. `07 §10` says the engine "runs many instances". Any
configurable cap smaller than the fan-out width refuses mid-spawn, so `05` and `07` require the cap to lift
entirely — a choice neither makes visible.

Id reuse: `05 §3` derives the child id "from binding values", and `05 §4` defines `rtm respawn` as
instantiating "a fresh child run for the same bindings". Same bindings derive the same id, so respawn *is*
id reuse. `05`'s open question 4 asks whether the superseded directory is archived or overwritten without
noticing that the answer decides `04`'s question 1 in the opposite direction from `04`'s own suggestion.
Separately, deriving an id from binding values is unspecified for any class with more than one binding.

### AR-10 — GAP: format supersession never claimed, and the self-hosting runbook would refuse

`.arca/runbook-spec.md:9-22` is the single written authority for the format (`RBS-004`) and says a runbook
"declares Phases and transitions and nothing else"; any other top-level key is `RB103`. `05 §1` adds a
top-level `[classes.<name>]` table and a `spawn` table inside a phase. Both are legitimate proposals, but
`05` never names the supersession, while `06 §2` is scrupulous about naming exactly this kind of debt
("This supersedes `R-022`, `R-023`, `R-024` and needs successor decisions to ADR-0007/ADR-0008"). The
asymmetry means a reader planning the work will underestimate `05` by one specification edit plus the tests
that hold the specification and the engine's code table to each other.

Concretely checkable today: `05 §7`'s "full proposed runbook, end to end" writes `blocked_route = true`.
The parser accepts only `blocked-route` (`src/machine.rs:373-375`), the specification documents only
`blocked-route` (`.arca/runbook-spec.md:44`), and the working rules use `blocked-route` too. The
self-hosting proof runbook would refuse with `RB103` on its own escape edge.

### AR-11 — GAP: new human verbs without an authorization model

`05` introduces `rtm spawn` (§1), `rtm respawn` (§4), and cites `rtm abandon --run tickets/t-042 --reason
"..."` (§4). The abandon form drops `--confirm` entirely, though the confirmation phrase typed at invocation
*is* the authorization (`.arca/schema.md`, "Abandoning a Run"; PGE-007), and `04 §2.2` separately proposes
that the phrase should name the run id rather than the project directory. `respawn` is described as
"human-confirmed" with no phrase specified. `spawn` is invoked by the Main-Agent per `05 §7`'s prompt, which
makes it the first ordinary-motion verb that writes new Run directories — its relation to the caller policy
(`ORS-001`) is unstated.

This matters because `07 §7` makes confirmation verbs the *only* source of exceptional motion. Every new
verb either is one, and needs its exact phrase, or is not, and needs to be shown to be ordinary motion.

### AR-12 — GAP: `07` carries no provenance marker

The brief for this review states that `07`'s sections 3–4 and connective prose were partially reconstructed
rather than transcribed. No such note exists in the landed file. Its front matter asserts the opposite —
"records settled doctrine in a normative voice" — and its only hedge is one line at `07:270` ("near verbatim
from the conversation") attached to the closing quotation, not to the reconstructed sections. `03` sets the
house precedent with an explicit confidence note correcting three of its own claims (`03:6`); `07` should
carry the equivalent.

This is the highest-leverage defect in the set for a mundane reason: `07` is the only file written in
requirement voice, so it is the file whose sentences will be copied into a goal.

### AR-13 — VERIFIED: the join does not violate no-routing-on-lifecycle

`05 §6`'s three distinctions hold against the code. Selection versus gating: `transition_for`
(`src/graph.rs:131-136`) picks the first non-blocked edge from the phase name alone and a join guard does
not touch it. Own versus foreign lifecycle: the rule the parser actually enforces is that a guard table may
not contain a `status` key (`src/machine.rs:480-482`, `RB104`), which is about the guard's own declaration,
and `step` already consults an engine-written on-disk fact — the frozen goal revision in Run evidence —
inside gating (`src/scheduler.rs:329-340`). Stability: `05 §6`'s claim is correctly stated as monotone
*between human interventions*, which is what the invariant says, not more.

One qualifier the file should absorb: `rtm respawn` can replace a `passed` child with a fresh `planned` one,
so the fact is stable-between-human-acts rather than stable-forever. `05 §4` implies this; `07 §5` restates
the invariant as covering "foreign *terminal* facts — a child's `passed`, once true, forever true", which is
stronger than `05` earned.

### AR-14 — VERIFIED: route-kind separation is consistent

`07 §9`'s third check — blocked routes neither satisfy exhaustiveness nor participate in first-passing order
— agrees with `06`'s open question 6 (which proposes the same and asks for it in writing) and with the code:
`transition_for` excludes blocked routes (`src/graph.rs:131-136`), `blocked_route_for` is a separate lookup
(`:139-144`), and initial-phase selection already ignores them (`src/scheduler.rs:757-769`). This one needs
only to be written down.

---

## 3. Ruling on `07`'s reconstructed sections

| Section | Ruling |
| :-- | :-- |
| §1 four artifacts | **Survives**, with one correction: it lists "history" as living in the Run and also names the log as the history artifact. Its Run row cites the flat `.arca/state.toml` while the rest of the file assumes per-run residency (AR-01). |
| §2 what a state is | **Survives as authored position, not doctrine.** No ADR defines a state this way; ADR-0001 defines machine state as the Phase and stops. The cutting criterion and the finite-projection argument are new and good, and should be admitted through the normal intake path rather than read as settled. |
| §3 control plane | **Survives except its last paragraph.** The routing restatement is `05 §6` verbatim and verifies against `src/graph.rs:131-136`. The tier-0 sentence is wrong (AR-03) and must be demoted. |
| §4 work plane | **Survives.** Every clause traces to an existing decision: ADR-0002 (agents request, guards decide), ADR-0003 (subagents never touch the Scheduler), ADR-0009 (prompts are prose), PGE-001..007 (a status edit cannot route). One overreach: "a child Run's terminal State File route" presumes AR-05. |
| §5 verdicts | **Split.** Typing and addressing survive as proposals. Consumption is demoted (AR-06): the archival requirement is defensible, the impossibility claim is not. The termination paragraph is demoted to guidance (AR-07). |
| §6 worker ≠ judge | **Survives as principle**; `RB302` is real and implemented (`src/doctor.rs:382`). Sequencing is already open in `07 §12` Q5 and stays open. |
| §9 static checks | **One of three survives.** Route-kind separation is verified (AR-14). Exhaustiveness and order coherence are demoted to open — `07 §12` Q3 already concedes severity and mixed-edge behavior, and AR-03 shows the claim that they block misroutes by construction is false under `06`'s tier 0. |
| §10 composition | **Demoted to design position.** "spawn/join is the composition primitive" rests on no decision record, and `.arca/goal/index.md:14` currently records the opposite non-goal ("No process spawning or process management in v1; spawn mode, if ever needed, is a future decision"). The proposal survives that non-goal on its merits — `rtm spawn` creates directories, not processes, so print-first is intact — but the non-goal must be amended explicitly, not read around. The machine-signature triple is contradicted (AR-04). |
| §11 placement table | **Survives** as the best compression in the set, with the §1 correction applied and "bindings, runbook hash" marked proposed rather than current. |

---

## 4. Curated decision list

Merged from `04 §9` (6), `05` open questions (7), `06` open questions (6), `07 §12` (6), plus this review;
deduped and ranked by what blocks the first bundle first. Every answer is RECOMMENDED, none decided.

| # | Decision | RECOMMENDED default | Rationale |
| :-- | :-- | :-- | :-- |
| 1 | What is a conditional edge (AR-03; 06 §1 vs 07 §3)? | `06`'s `when` list over the existing seven guard kinds; verdicts become an eighth kind later | Keeps the first bundle free of per-run residency and preserves the route order all four files claim |
| 2 | Do blocked routes carry predicates (06 Q6; 07 §9)? | No; and they neither satisfy exhaustiveness nor take part in first-passing order | Already true in code; writing it down costs one specification row |
| 3 | Does the engine write `passed`/`failed`, and where (AR-05; 06 Q3)? | Yes, in the first bundle: `passed` on arriving at a phase with no ordinary out-edge | A terminating loop with no observable end cannot be tested; and the join later needs it anyway |
| 4 | Prefix approval `CGE`/`RBP`/`NRI`/`CHI` (06 Q1)? | Accept | No collision with any integrated prefix; each needs a dict.md entry in its landing |
| 5 | Exhaustiveness lint severity, and mixed verdict/non-verdict edges (07 Q3, Q6)? | Error for a declared value with no covering edge; mixed sets allowed but warned | Stranding a Run is a defect; forbidding mixing outright would forbid a verdict edge beside a receipt edge, which is a reasonable machine |
| 6 | Canonical run path scheme (AR-01)? | AR-01's layout: plural `runs`, one id namespace, verdict slots and spawn ledgers nested under a run | Restores "the listing is the registry", removes the namespace collision, and gives verdict addressing a place to be computed from |
| 7 | Run addressing: flag or positional, ever optional (AR-02; 04 Q2; 06 Q4)? | `--run <id>`, always required; missing value refuses with the roster | Only form that keeps recorded transcripts self-describing for behavioral evidence |
| 8 | Runbook pin: hash-only or per-run copy (06 Q2; 04 Q5)? | Hash-only in the pin bundle; revisit the copy only if a real drift case demands it | One reader, one source; a copy creates two files that can disagree and a rule about which wins |
| 9 | Flat-layout migration (06 Q5; 04 Q4)? | Refuse and instruct, never auto-migrate | Exact precedent exists: the legacy lock refusal at `src/scheduler.rs:110-129` refuses without modifying |
| 10 | Does the one-active-Run cap lift entirely (AR-09; 04 Q6)? | Lift entirely | Any cap below the fan-out width refuses mid-spawn, which makes the child bundle unusable |
| 11 | Id reuse after abandon, and respawn generations (AR-09; 04 Q1; 05 Q4)? | Never reuse; respawn mints a new id and the ledger entry records the superseded one | Preserves the failure evidence the retrospective wants and keeps ids unforgeable addresses |
| 12 | Verdict consumption ordering and archive destination (AR-06; 07 Q4)? | Archive and clear *before* the State File write; archive into Run evidence, not the history | Follows the freeze precedent at `src/scheduler.rs:385-388`: an interruption leaves the Run unchanged, not half-consumed |
| 13 | Termination rule for cyclic classes (AR-07)? | Replace "monotone facts" with a checkable rule: every phase on a cycle needs one out-edge guarded by receipt- or contract-class guards only | "Monotone" is not a property the vocabulary exposes; kind membership is |
| 14 | Spawn ledger placement, and join coverage (AR-08; 05 Q1, Q2)? | Under the parent run directory; coverage stays with the record contract plus the prompt for now | Placement dissolves the registry conflict at zero cost; a `covers` field can wait for a real machine that needs it |
| 15 | Authorization for `spawn` / `respawn` / abandon-with-run-id (AR-11)? | `spawn` is ordinary motion (no phrase); `respawn` and `abandon` require phrases naming the run id | Exceptional motion needs a human and nothing but a human; ordinary motion needs neither |
| 16 | Per-run history durability, and a consolidated view (04 Q3; 05 Q3)? | Archive a retired Run's history into the tracked journal on abandon; no consolidated read view yet | Git-ignored scratch has no durability; a read view is cheap to add once anyone misses it |
| 17 | Witnessed verdict verb, and which realization lands first (07 Q1, Q5)? | Child-as-reviewer first; defer the witnessed verb | The witnessed verb needs signer identity, which `ORS-001` deliberately keeps out of the Engine |
| 18 | Literal `ticket =` deprecation, `min` semantics, child worktree lifecycle (05 Q5, Q6, Q7)? | Keep the literal form; keep `min` with default 1; reuse the trial worktree verbs | All three are reversible later and none blocks a bundle |

---

## 5. What I verified, and how

Read in full: the four subject files, `.arca/goal/design.md`, `.arca/runbook-spec.md`, `.arca/schema.md`
(as loaded project instructions), `src/model.rs`, `src/graph.rs`.

Read in part, at the exact regions cited by the subject files: `src/machine.rs:1-100` (guard vocabulary and
accepted fields), `:360-490` (transition key set, unknown-key refusal, the guard `status` rejection);
`src/scheduler.rs:105-150` (the lock and the legacy-name refusal), `:185-300` (open, class loading, start
admission, evidence at start), `:300-437` (step, drift check, write ordering, rollback), `:439-563` (guard
dispatch and the receipt and contract evaluators), `:750-870` (initial phase, per-invocation lock, status);
`src/state.rs:50-60`; `src/pin.rs:1-20`; `src/doctor.rs:230-260`, `:320-345`, plus a code grep confirming
`RB301`, `RB302`, `RB401`, `RB205` are emitted; `src/abandon.rs` grep for terminal handling;
`src/cli.rs:88-145`; `.arca/goal/spec.md` rows `R-001..R-004`, `R-010..R-014`, `R-019..R-026`, `R-030`,
`ETB-001..003`, `ORS-001..003`, `DRD-003..007`; `.arca/goal/index.md:5`, `:14-15`, `:56-61`;
`.arca/ratmac.toml` in full.

Whole-tree greps: every write site of a `Status` value in `src/` (four sites, none terminal); every
occurrence of the blocked-route key in the specification, the authoring guide, and the doctor; every
occurrence of a reconstruction or confidence disclaimer across the research folder (present in `03`, absent
in `07`).

Not re-verified, and flagged as such: `06 §0`'s test counts (199 workspace tests, 182 behavior functions)
and its per-family blast-radius numbers — I did not run the suite, since this pass is read-only and the
counts do not change any finding. `04 §6`'s external prior-art citations were not fetched; they carry no
weight in any finding above.

Token sweep: this file contains neither pre-rebrand identity token; the audit's scan set includes
`.arca/research/`, and the two tokens were located by reading the audit test itself
(`test/qa/tests/t034_rat005.rs:6-8`), which spells them by concatenation for the same reason.
