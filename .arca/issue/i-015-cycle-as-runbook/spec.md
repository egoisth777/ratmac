# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at integration.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `PCR-001` | `.arca/ratmac.toml` declares the P1-P5 cycle this repository runs - its Phases, its prompts, and its Exit Guards over `.arca/` - replacing the demonstration machine now there. | proposed | The engine's thesis is that progress is proven by machine-checked guards. The one process ratmac governs has never been run by ratmac; every stage boundary is currently enforced by a contributor remembering the rules. | — (pending P1) |
| `PCR-002` | `rtm status` is the answer to "where are we". The tree-derivation lookup in `.arca/index.md` becomes a fallback for a repository with no active Run, or is retired. | proposed | Two oracles for the same question is one oracle too many; `.arca/index.md` already warns it is orientation and never evidence, yet it is what everyone reads. | — (pending P1) |
| `PCR-003` | "Open ticket" is machine-checkable. A landed ticket is distinguishable from an executing one without reading prose - by an authorized ticket archive move (the issue rule in `.arca/schema.md`, Evidence and archive rules, extended to tickets) or by an evidence-derived predicate. | proposed | Discovered at this cycle close: 27 landed tickets sit in `.arca/ticket/`, the stage lookup says "open tickets -> P4/P5", and the 2026-07-24 close nonetheless declared Idle with the same shape. A guard cannot read that; a human has been silently supplying the missing predicate. | — (pending P1) |
| `PCR-004` | The `.arca/log.md` ownership contradiction is resolved before the cycle runs: the working rules require every contributor to append a landing line, while `PGE-004` makes the file Scheduler-owned and forbids a prompt from instructing an agent to write it. | proposed | The P-cycle's own Phase Prompts must pass `rtm doctor`'s ownership audit (`RB401`). Today the honest runbook of this repository fails its own lint - the schema already names the way out ("an explicit `rtm` command that performs the Scheduler-owned append itself"), but no such command exists. | — (pending P1) |
| `PCR-005` | The cycle runbook is doctor-clean: `rtm doctor` exits `0` on it, with no gate whose verdict rests on content the agent under test can write. | proposed | The demonstration runbook currently earns two `RB302` warnings for exactly that shape. A process machine that grades an agent on files the agent writes proves nothing, and shipping it as the reference runbook would teach the pattern. | — (pending P1) |
| `PCR-006` | The Engine holds no project knowledge the runbook could hold: the `.arca/issue`, `.arca/ticket`, `.arca/residual`, and `.arca/goal` paths baked into `contract.rs`, `blocked.rs`, and `goal.rs` (R-016) are declared by the runbook instead. | proposed | The cycle runbook cannot declare its own paths while the Engine already knows them, and R-016 is the one debt that makes "the engine is generic" false in the source rather than in the story. Steering currently defers this; it is a dependency here, and P1 may split it. | — (pending P1) |

## Acceptance criteria

- `.arca/ratmac.toml` describes the P1-P5 cycle, and `rtm start` plus `rtm step` drive it without a human
  restating a rule the runbook should have carried.
- `rtm status` names the current stage of this repository, and any second answer to that question is either
  derived from it or explicitly labelled a no-Run fallback.
- `rtm doctor` exits `0` on the cycle runbook.
- No Phase Prompt or guard contract in it instructs an agent to write a Scheduler-owned file.
