# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at integration.

`PCR` expands to **P-Cycle Runbook** - this issue's requirement-ID prefix, defined in
[ubi-lang.md](ubi-lang.md) as the ID convention now requires.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `PCR-001` | `.arca/ratmac.toml` declares the P1-P5 cycle this repository runs - its Phases, its prompts, and its Exit Guards over `.arca/` - replacing the demonstration machine now there. | proposed | The engine's thesis is that progress is proven by machine-checked guards. The one process ratmac governs has never been run by ratmac; every stage boundary is currently enforced by a contributor remembering the rules. | — (pending P1) |
| `PCR-002` | `rtm status` is the answer to "where are we". The stage-derivation lookup in `.arca/index.md` becomes a fallback for a repository with no active Run, or is retired. | proposed | Two oracles for the same question is one oracle too many; `.arca/index.md` already warns it is orientation and never evidence, yet it is what everyone reads. | — (pending P1) |
| `PCR-003` | "Open ticket" is machine-checkable. A landed ticket is distinguishable from an executing one without reading prose - by an authorized ticket archive move (the issue rule in `.arca/schema.md`, Evidence and archive rules, extended to tickets) or by an evidence-derived predicate. | proposed | Discovered at this cycle close: 27 landed tickets sit in `.arca/ticket/`, the stage lookup says "open tickets -> P4/P5", and the 2026-07-24 close nonetheless declared Idle with the same shape. A guard cannot read that; a human has been silently supplying the missing predicate. | — (pending P1) |
| `PCR-004` | Once a Run is live, the landing line is appended by `rtm`, not by a contributor editing `.arca/log.md`. | proposed | Not a contradiction in the working rules - `.arca/schema.md` legalises contributor appends precisely and only while no `.arca/state.toml` exists. The gap bites at the endpoint: with the cycle Run alive that carve-out closes, and no `rtm` command performs the append. A Phase Prompt may never instruct the write (`RB401`), so the cycle runbook cannot carry the instruction itself. | — (pending P1) |
| `PCR-005` | The cycle runbook is doctor-clean: `rtm doctor` exits `0` on it, with no gate whose verdict rests on content the agent under test can write. | proposed | The demonstration runbook currently earns two `RB302` warnings for exactly that shape. A process machine that grades an agent on files the agent writes proves nothing, and shipping it as the reference runbook would teach the pattern. | — (pending P1) |
| `PCR-007` | The P4/P5 loop keeps its per-ticket gates (`sensitivity_receipts`, `completion_gate`) while `.arca/ratmac.toml` stays pure, read-only, and free of any ticket id. The mechanism is an explicit P1 decision between the options in [design.md](design.md). | proposed | Both kinds require a literal `ticket` field, and the runbook is human-authored and read-only at runtime (R-010, R-013) with no interpolation in the format. As written, gating ticket t-058 means editing the runbook every loop turn. This - not the log file - is the mechanical reason the cycle is not a runbook today. | — (pending P1) |

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
