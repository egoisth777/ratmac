# Issue design

## Proposed mechanics

### The triage of 2026-09-17

Every red verdict in the first sweep report was run again from inside its
crate against the tree at the archive-move landing of 2026-09-17 and its
failure traced to the landing that changed what the lane asserts. No live
regression was found.

| Crate | Failure | Class | Named landing / cause | Act |
| :--- | :--- | :--- | :--- | :--- |
| `t-058`, `t-059`, `t-060` | all six lanes panic at the fixture's first `rtm start`: `pre-split Engine residue <tmp>/.arca/ratmac.toml exists` | retired contract | engine-namespace split `t-071` (root `.ratmac/`), `t-081` (`state.toml` -> `run.toml`, `phase` -> `state`), `t-083` (`Phase:` -> `State:`); `t-060` also pins `.arca/rtm.lock`, now `.ratmac/locks/root.lock` | port - renames only; `t-058`'s created-file list gains `.ratmac/mint.toml` |
| `t-061`, `t-062` | build refused: no field `phase` on `RunState`; `t-061` also `phase_prompt` -> `state_prompt` | retired contract | `t-081`, `t-083`; `t-062`'s verdict helper writes `phase =` where `src/verdict.rs` demands exactly `state`, `input`, `rationale` | port - renames plus re-deriving each refusal reason in `t-062`'s case table |
| `t-063` | build refused: `HoldRequest` has no field `ticket` | retired contract | `t-087` (the Engine has no work-item concept): `rtm hold --run <id> --blocker <ref> --confirm "hold <id>"` | port - HT-063-03 rebuilt for the blocker-based hold with a declared `[roots]` entry and a resolving blocker |
| `t-064`, `t-067`, `t-070` | build refused: no `render_json` in `doctor` | retired contract | `t-078` retired the public findings-only renderer for `Diagnosis::render_json` (crate-private, `engine_root` first) behind `rtm doctor --json`; fixtures also pre-split | port - fixture to `.ratmac/`, JSON-stability oracle over the CLI's `--json` output |
| `t-065`, `t-066`, `t-068`, `t-069` | all six lanes panic at `Scheduler::open` with the residue refusal | fixture drift | scaffolds hand-build `.arca/goal` + `.arca/ratmac.toml` and read `.arca/runs/<id>/state.toml`; `t-069`'s verdict helper writes `phase =` | port - scaffold to `.ratmac/`, `run.toml`, `state` key |
| `t-083` | `ht_083_02`: status prints report + prompt + `next: rtm step --run run-001` where the lane expects report + prompt | retired contract | `t-103` (AOP-001/002 teach line, `src/teach.rs::status_next`) | port - expected string gains the teach line |
| `t-085` | `ht_085_05`/`ht_085_06`: the cutover baseline's usage and status blocks differ by the `skill` verb and the teach line; `ht_085_03`: `.ratmac/log.md` listed but compared absent | retired contract + helper defect | `t-104` (`skill` verb), `t-103` (teach line); `test/qa/src/baseline.rs::Pair::freeze_paths` returns newline-terminated paths through `canonical()` | port - baseline vocabulary extended for the edition-004 surface; the helper trims its lines |
| `t-092`, `t-093` | cut-tickets exit refused: `res-100.md records no parseable frozen-goal-bundle-revision citation` | fixture drift | `ARF-001` (`src/contract.rs`, the `goal-sha256:` demand); the fixture reads `evidence.toml` at `gap-check`, before `src/scheduler.rs` mints the freeze at the step into `cut-tickets` | port - read the freeze after the Engine mints it |
| `t-095`, `t-096`, `t-100` | six of six green today | stale verdict | the edition-004 tag lagged its ledger row until the repair of 2026-08-25 15:22 (-07:00), after the 10:31Z sweep | re-sweep only |

None of the thirteen pre-split crates has a last-good edition: the split
(`t-071`, 2026-08-06) and every rename above are ancestors of `edition-001`
(2026-08-12), so an expiry marker could only name an edition the lane never
passed at. That is why `RLR-001` makes such a lane port-only.

### Where each ask lands

- `RLR-001` is a working rule: a requirement-ID heading in `.arca/schema.md`
  under a new "Landed-lane recovery" section beside the Editions section,
  mirroring how `EDN-001`..`EDN-003` bind. It carries no executable
  deliverable and so mints no gap record and no ticket; its proof is the
  per-crate class table each recovery ticket records.
- `RLR-002` and `RLR-003` are executable: two tickets, one per crate
  generation, each with a focused test in `test/qa/tests/` that reads the
  sweep report and demands `pass` for its crates, and six hidden lanes of its
  own. The ports happen inside `test-hidden/` (untracked, so the ticket's
  completion receipts declare the ported crates among their tree roots and
  the sweep report is the tracked evidence). The `t-085` helper fix touches
  `test/qa/src/baseline.rs`.
- `RLR-004` is executable and small: `.ratmac/lanes.toml` gains `t-108`;
  `tools/sweep_lanes.py check` parses the report's stray-entries line and
  refuses on it; the roster duty is one sentence in the working rules. It is
  owned by the second ticket, which is the one whose green makes the close
  guard exit `0`.
- Nothing here edits `.ratmac/ratmac.toml`: the sprint Run pins the runbook
  (`FDC-005`) and the wired guard already reads the report.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
