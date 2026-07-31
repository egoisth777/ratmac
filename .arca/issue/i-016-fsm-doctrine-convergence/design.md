# Issue design
 
## Current scope after the 2026-07-30 atomic cut

This file remains the shared decision history for the doctrine-convergence split; moved issues cite it and never copy its adversarial-review rulings. Current requirement ownership is narrower:

- `FDC-001` remains here: the Machine Class declares each branching state's closed legal input list, ordinary outgoing edges map its values uniquely and completely, and the Engine selects by the one transition input extracted from the judge-authored verdict record.
- `FDC-003` moved to the [input-delivery and durability issue](../i-019-input-delivery-durability/design.md): external delivery and atomic consume/archive before state advance.
- `FDC-002` moved to the [Run-completion issue](../i-020-run-completion/design.md): Engine-written `passed`, durable abandonment, and deferred `failed`.

Billy's 2026-07-30 clarification separates four senses older text sometimes called “verdict”: the class's legal input list, one transition input value, the live judge-authored verdict record, and its archived evidence. The accepted goal term `Verdict slot` and the current `verdict.toml` reservation are unchanged by this documentation cut. Exact Machine Class keys still require acceptance into the runbook-format single source of truth.

The witnessed-verdict mechanism remains deferred because signer identity is outside the Engine. Judge independence remains solely in the machine-composition issue under `FDC-009` and `FDC-010`.

*Split pointer (2026-07-29): the decision records below that settle run residency and machine composition now bind asks moved to the run-residency issue ([i-017-run-residency](../i-017-run-residency/index.md)) and the machine-composition issue ([i-018-machine-composition](../i-018-machine-composition/index.md)); everything stays here unchanged as evidence history the new issues cite.*

## Dependency route (human ruling, 2026-07-29)

Route (human ruling, 2026-07-29): run residency (`i-017-run-residency`) lands first; the
verdict-routed execution core (`i-016-fsm-doctrine-convergence`) depends on it; machine composition
(`i-018-machine-composition`) depends on both. Verdict routing and verdict consumption need a defined
per-Run verdict-slot address and Run-evidence location - contracts `FDC-004` through `FDC-006` define
this solely in [../i-017-run-residency/spec.md](../i-017-run-residency/spec.md); this file never
restates them.

## Proposed mechanics

Work the ledger one AR at a time, in an order that keeps later answers from moving earlier ground:

1. **CONTRADICTIONs first** (`AR-01`-`AR-04`): every downstream answer depends on which source wins run
   residency, addressing, tier 0, and the terminal vocabulary. Each closes by choosing one reading and
   amending the losing file(s).
2. **GAPs next** (`AR-05`, `AR-07`, `AR-09`-`AR-12`): each closes by writing the missing specification
   into the file the review implicates, or by recording that the gap is deliberate and who owns it.
3. **TENSIONs batched** (`AR-06`, `AR-08`), together with the curated decision list in `08` §4, as one
   decision sitting for Billy - options prepared in writing, outcome recorded, no agent choice.
4. **VERIFIED closed immediately** (`AR-13`, `AR-14`) by pointing the ledger row at the review itself.

Each resolution is two motions: an edit to the implicated research file(s), then one ledger row flip in
[test-plan.md](test-plan.md) - status off `open`, resolution pointing at the doc or decision that settled
it. No AR is resolved by prose in this issue alone; the research files are where doctrine lives.

Sources under `.arca/research/re-ratmac-FSM/`:

- [04-run-identity.md](../../research/re-ratmac-FSM/04-run-identity.md)
- [05-invocation-join.md](../../research/re-ratmac-FSM/05-invocation-join.md)
- [06-migration-cost.md](../../research/re-ratmac-FSM/06-migration-cost.md)
- [07-conceptual-model.md](../../research/re-ratmac-FSM/07-conceptual-model.md)
- [08-adversarial-review.md](../../research/re-ratmac-FSM/08-adversarial-review.md)

## Authority ordering (human ruling, 2026-07-29)

Billy ruled on 2026-07-29: the conceptual model
([07-conceptual-model.md](../../research/re-ratmac-FSM/07-conceptual-model.md)), as corrected by the
adversarial review ([08-adversarial-review.md](../../research/re-ratmac-FSM/08-adversarial-review.md)),
speaks for the **target** system. The migration study
([06-migration-cost.md](../../research/re-ratmac-FSM/06-migration-cost.md)) binds only migration and
sequencing facts about the current state. The current engine's source code is evidence of the present
and raw material to transform — never authority over target design. Where a recommendation in `08` §4
cites current code in its rationale, that citation is prior art, not a constraint.

## Adopted defaults (batch human sign-off, 2026-07-29)

Billy adopted, in one sitting on 2026-07-29, the RECOMMENDED default of every `08` §4 item except
three he is ruling on individually: the tier-0 conditional-edge definition (item 1, tied to the
tier-0 contradiction, `AR-03`), whether the engine writes the `passed`/`failed` terminals (item 3,
tied to the unauthorized-statuses gap, `AR-05`), and verdict-consumption write ordering (item 12,
tied to the crash-safety tension, `AR-06`). The fifteen adopted items, each condensed from `08` §4:

- Item 2 — Blocked routes carry no predicates and sit outside exhaustiveness and first-passing
  order; already true in code, costs one specification row. `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 4 — Accept the `CGE`/`RBP`/`NRI`/`CHI` prefixes; no collision with any integrated prefix,
  each earns its dict.md entry in its landing. `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 5 — Exhaustiveness lint errors on a declared value with no covering edge; mixed
  verdict/non-verdict edge sets are allowed but warned, since forbidding mixing would forbid a
  reasonable machine. `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 6 — Canonical run path scheme is the review's layout (`AR-01`): plural `runs`, one id
  namespace, verdict slots and spawn ledgers nested under a run; restores "the listing is the
  registry", removes the namespace collision, gives verdict addressing a computable base. `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 7 — Run addressing is `--run <id>`, always required; a missing value refuses with the
  roster; the only form that keeps recorded transcripts self-describing for behavioral evidence
  (`AR-02`). `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 8 — Runbook pin stays hash-only; revisit a per-run copy only if a real drift case demands
  it; a copy creates two files that can disagree. `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 9 — Flat-layout migration refuses and instructs, never auto-migrates; follows the existing
  lock-refusal precedent of refusing without modifying. `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 10 — The one-active-Run cap lifts entirely; any cap below the fan-out width refuses
  mid-spawn and makes the child bundle unusable (`AR-09`). `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 11 — Ids are never reused after abandon; respawn mints a new id and the ledger entry
  records the superseded one, preserving failure evidence and unforgeable addresses (`AR-09`). `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 13 — The cyclic-class termination rule replaces "monotone facts" with a checkable rule:
  every phase on a cycle needs one out-edge guarded by receipt- or contract-class guards only;
  monotonicity is not a property the vocabulary exposes, kind membership is (`AR-07`). `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 14 — The spawn ledger lives under the parent run directory; join coverage stays with the
  record contract plus the prompt for now; placement dissolves the registry conflict at zero cost
  (`AR-08`). `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 15 — `spawn` is ordinary motion with no phrase; `respawn` and abandon-with-run-id require
  phrases naming the run id; exceptional motion needs a human, ordinary motion needs neither
  (`AR-11`). `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 16 — A retired Run's history archives into the tracked journal on abandon; no consolidated
  read view yet; git-ignored scratch has no durability, a read view is cheap to add later. `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 17 — Child-as-reviewer lands first; the witnessed verdict verb is deferred, since it needs
  signer identity, which `ORS-001` deliberately keeps out of the Engine. `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`
- Item 18 — Keep the literal `ticket =` form; keep `min` with default 1; reuse the trial worktree
  verbs for child worktrees; all three are reversible later and none blocks a bundle. `assumed: adopted RECOMMENDED default — revocable; if overturned, redo only affected pieces`

These adoptions settle six ledger findings: run residency (`AR-01`, item 6), run addressing
(`AR-02`, item 7), the termination rule (`AR-07`, item 13), spawn ledger placement (`AR-08`,
item 14), the cap and id reuse (`AR-09`, items 10 and 11), and verb authorization (`AR-11`,
item 15); [test-plan.md](test-plan.md) records each flip. No adopted item claims the
runbook-format supersession, so that gap (`AR-10`) stays open; the terminal-vocabulary
contradiction (`AR-04`) also stays open — no §4 item addresses it directly and it leans on the
held terminals ruling.

*Superseded (2026-07-29, human ruling, review fold-in S3): item 5's "mixed verdict/non-verdict edge
sets are allowed but warned" default no longer holds. A branching Phase now declares a closed verdict
input list; every ordinary outgoing edge carries exactly one unique value from that list; missing
coverage, duplicate coverage, and out-of-list values are errors; mixed labelled and unlabelled
ordinary edges are forbidden; a straight-line Phase may keep one unlabelled ordinary edge; blocked
routes stay outside selection and exhaustiveness checks — see `FDC-001` in [spec.md](spec.md). Format,
parser, and doctor changes for this rule trace to [.arca/runbook-spec.md](../../runbook-spec.md).*

*Superseded (2026-07-29): the two findings left open above — the terminal vocabulary (`AR-04`) and the format gap (`AR-10`) — are closed by the individual human rulings below.*

## Individual human rulings (2026-07-29)

Billy ruled individually on 2026-07-29 on the five findings held out of the batch sign-off,
closing the ledger. Each entry below is a human ruling, not an assumption: none is revocable
by an agent, and overturning one takes a new ruling.

- **Edge selection is verdict-only (`AR-03`).** Ordinary guards are readiness AND-gates —
  "can we move" — never selectors — "where to"; the only selecting edges are typed-verdict
  edges. Rationale: routing ambiguity is excluded by construction, together with the
  exhaustiveness lint. Consequences: verdict slots and per-run residency belong to the first
  increment; the migration study's tier-0 definition (when-lists over the guard kinds) binds
  migration history only.
- **The engine writes terminal statuses (`AR-05`).** The engine itself writes them: `passed`
  on arriving at a phase with no ordinary out-edge; `abandoned` via the abandon path.
  Rationale: this states the previously unstated dependency of the join design in
  [05-invocation-join.md](../../research/re-ratmac-FSM/05-invocation-join.md) §3/§4 — the
  join's readable stable fact now exists by ruling.
- **One fixed terminal enum (`AR-04`).** Terminal status values are one fixed engine-level
  enum shared by all runbooks: `passed` / `failed` / `abandoned`. Rationale: richer outcome
  semantics live in verdicts and evidence, never in the terminal status. Consequence: no
  per-runbook terminal vocabularies.
- **Verdict consumption order (`AR-06`).** Archive the consumed verdict and clear the slot
  before the State File write; archive into Run evidence, not history. Rationale: a
  stale-verdict replay is impossible by this ordering. Consequence: the crash worst case is
  a consumed-but-unadvanced run that waits for a fresh verdict — a safe re-judge.

  *Refined (2026-07-29, human ruling, review fold-in S5): the two-write ordering above is one atomic
  filesystem motion, not two sequential writes.*

  1. Validate the live verdict.
  2. Atomically rename its slot into immutable Run evidence, on the same filesystem as the slot — one
     operation both archives the verdict and clears its live slot.
  3. Derive a collision-free evidence name from the Phase and the next on-disk attempt number.
  4. Write the successor State File only after the rename succeeds.
  5. If interrupted after the rename but before the State File write, the Run remains in the old
     Phase and requires a fresh verdict; the archived verdict never replays.
  6. Test interruption before the rename, after the rename, and after the State File write.

  No journal, no separate recovery subsystem. Same-filesystem is load-bearing on Windows because
  cross-volume rename is not atomic there; it holds today because all of `.arca/` is one tree, and
  this design states that dependency rather than assuming it.
- **Mechanical closure of the format gap (`AR-10`).** Two parts. (a) The runbook format is
  explicitly extended by this design to carry the class and spawn tables
  [05-invocation-join.md](../../research/re-ratmac-FSM/05-invocation-join.md) §1 introduces;
  this supersedes the format-spec restriction the review cites (`RBS-004`, the
  "declares Phases and transitions and nothing else" rule whose breach is an `RB103`
  refusal). (b) The canonical spelling is `blocked-route` (hyphen), matching the parser, the
  specification, and the working rules; the `05` §7 example is corrected accordingly.

- **Terminal vocabulary corrected: `failed` deferred (review fold-in S4, human ruling, Billy,
  2026-07-29 — supersedes "One fixed terminal enum (`AR-04`)" above).** `passed` is written when
  `rtm start` or `rtm step` arrives at a Phase with no ordinary outgoing edge, unchanged. `abandoned`
  is a durable terminal event written before the active state is retired — never a surviving State
  File value. `failed` is deferred until a later issue defines a concrete, engine-observable failure
  event: no failure command is added, and guard refusal is not reinterpreted to populate it, since
  refusal must leave Run state unchanged. This is a human ruling, not an agent disposition: it is not
  revocable by an agent, and overturning it takes a new ruling. Consequence: the terminal-vocabulary
  ledger rows (`AR-04`, `AR-05`) reopen in [test-plan.md](test-plan.md) until the deferred `failed`
  outcome receives a complete contract.

## Convergence status (review fold-in, 2026-07-29)

The rulings and adoptions above are recorded here, but per this file's own method (see Proposed
mechanics), a finding is not resolved by issue prose alone — the implicated research file(s) must
also be amended. Ten ledger rows in [test-plan.md](test-plan.md) — `AR-01` through `AR-09` and
`AR-11` — cite only this design record, with no corresponding supersession block yet landed in the
implicated research file(s); those rows reopen to `open` with reason "awaiting research supersession
blocks; batched adversarial reread pending". `AR-10` and `AR-12` keep `resolved`: their corrections
already landed in [05-invocation-join.md](../../research/re-ratmac-FSM/05-invocation-join.md) and
[07-conceptual-model.md](../../research/re-ratmac-FSM/07-conceptual-model.md) respectively. `AR-13`
and `AR-14` keep `resolved`: verified sound, no correction was ever required.

The adversarial reread happens once, batched over every corrected research file, after all ten
supersession blocks land — never once per row. Only that single batched reread may restore a row to
`resolved`.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
