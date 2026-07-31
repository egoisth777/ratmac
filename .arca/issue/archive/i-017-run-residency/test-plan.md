# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| RRV-001 | FDC-004 - FDC-006 | Every ask traces to its adopted-default record in the seed's design ([design.md](../i-016-fsm-doctrine-convergence/design.md), Adopted defaults, batch human sign-off 2026-07-29) and to the research sections it condenses (`.arca/research/re-ratmac-FSM/04-run-identity.md`, `05-invocation-join.md`, `06-migration-cost.md`). |
| RRV-002 | FDC-004 | Runs listed under the plural `runs` path with one id namespace; verdict slots nest under their run's directory, and a per-run spawn-ledger path exists there by name only — no ledger contract is exercised by this check, that is machine composition's (`i-018-machine-composition`) to test; `--run <id>` is always required and a missing value refuses with the roster, the refusal recorded as behavioral evidence. |
| RRV-003 | FDC-005 | The runbook pin is hash-only — no per-run copy exists; a planted flat-layout residue produces a refusal that instructs and modifies nothing. |
| RRV-004 | FDC-006 | No active-Run cap is enforced; within the one run-id namespace, an abandoned run's id is never reissued; a respawn mints a new id — no ledger-entry content is exercised by this check, that is machine composition's (`i-018-machine-composition`) to test. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | integrated 2026-07-29 | "Integrated canonical run residency" links back to this issue's [index.md](index.md) and names the Ideal-shape property the fold-in advances. |
| `.arca/goal/ubi-lang.md` | integrated 2026-07-29 | Five terms coined goal-side: the plural `runs` path, run id namespace, verdict slot, spawn-ledger path (location only - its contract stays machine composition's (`i-018-machine-composition`) to coin), flat-layout residue. |
| `.arca/goal/spec.md` | integrated 2026-07-29 | `FDC-004` - `FDC-006` under "Integrated run-residency requirements", each Source cell linking back to [spec.md](spec.md); `R-022` and `R-023` carry their supersession note. |
| `.arca/goal/design.md` | integrated 2026-07-29 | "Canonical run residency and identity (FDC-004-FDC-006)", integrated from this issue's [design.md](design.md). |
| `.arca/goal/test-list.md` | integrated 2026-07-29 | `RRV-001` - `RRV-004` under "Integrated run-residency verification", one per check below; `T-08` and `T-09` carry their supersession note. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | — |
| `.arca/schema.md` | unaffected | — |
