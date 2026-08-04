# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at integration.

`PCR` expands to **P-Cycle Runbook** - this issue's requirement-ID prefix, defined in
[ubi-lang.md](ubi-lang.md) as the ID convention now requires.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `PCR-001` | `.arca/ratmac.toml` declares the P1-P5 cycle this repository runs - its Phases, prompts, and Exit Guards over `.arca/` - replacing the demonstration machine. Extended 2026-08-03 (safe deliberate-damage integration, [i-022](../../archive/i-022-safe-deliberate-damage/index.md)): those Exit Guards include the automatic dirty-tree refusal before any deliberate-damage step, and the Engine's intake gate accepts an accepted ask that resolves to an explicit working-authority requirement-ID heading, not only to a goal row. | deferred | The real cycle depends on routing, delivery, completion, and composition contracts not all integrated in this pass. 2026-08-03: future automation of the discard guard has one owner - this ask. `src/contract.rs` stays untouched today because no Run drives this repository and the public suite exercises the contract gates on fixture trees only (`test/qa/tests/t048_contract_gates.rs`; the intake shape likewise in `t040_archive_oracle.rs`); teaching them the working-authority branch is this issue's selection-time work. | - (deferred 2026-07-30) |
| `PCR-002` | `rtm status` answers “where are we”; the tree-derived lookup becomes only the no-live-Run fallback or is retired. | deferred | One stage oracle is required, but the real cycle Run must exist first. | — (deferred 2026-07-30) |
| `PCR-003` | “Open ticket” is machine-checkable, distinguishing landed work from executing work without prose inference. | deferred | The current tree misclassifies landed active-folder tickets; the mechanism belongs with the real cycle. | — (deferred 2026-07-30) |
| `PCR-004` | Once a Run is live, `rtm`, not a contributor, appends the landing line to `.arca/log.md`. | deferred | The cycle needs an Engine-owned close operation that does not yet exist. | — (deferred 2026-07-30) |
| `PCR-005` | The cycle runbook is doctor-clean, with no gate whose verdict rests on content writable by the agent under test. | deferred | Honest self-hosting follows the missing execution layers. | — (deferred 2026-07-30) |
| `PCR-007` | The P4/P5 loop keeps per-ticket sensitivity and completion gates while the runbook stays read-only and free of literal ticket ids. | deferred | The ticket-binding mechanism remains a planning decision for the self-hosting pass. | — (deferred 2026-07-30) |

`PCR-006` was dropped at review on 2026-07-28: extracting the hard-coded `.arca/issue|ticket|residual|goal`
paths from the Engine (R-016) buys nothing until a second project exists, and `.arca/steering.md` already
defers it. The numbering keeps its hole so the decision stays legible.

## Acceptance criteria

- `.arca/ratmac.toml` describes the P1-P5 cycle, and `rtm start` plus `rtm step` drive it without a human
  restating a rule the runbook should have carried.
- `rtm status` names the current stage of this repository, and any second answer to that question is either
  derived from it or explicitly labelled a no-Run fallback.
- `rtm doctor` exits `0` on the cycle runbook.
- No Phase Prompt and no gate contract in it instructs an agent to write a Scheduler-owned file.
- Every guarantee `PGE-003` and `PGE-005` already carry per ticket is still carried per ticket.
