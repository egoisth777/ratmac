# Issue specification

Dispositions were confirmed at the 2026-09-17 planning pass (P1) under
Billy's next-issue authorization - all four asks accepted as filed, the
acceptance itself logged as an assumption the user may revise. None of the
four asks presumes an Engine
feature: the triage rule binds contributors, the ports are edits inside
untracked hidden crates plus one qa helper, and the roster and check
changes live in shop tooling the Engine already knows nothing about
(ADR-0020). Every ask therefore resolves to the working authority, the way
the edition and history-gate requirements did; the three executable ones
are measured by gap records and worked by tickets under `PCR-008`.

`RLR` is this issue's stable requirement-ID prefix - **Red-lane recovery** -
defined in [ubi-lang.md](ubi-lang.md).

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `RLR-001` | A red, unexpired sweep verdict is classified before any act, by cause, into exactly one triage class: **retired contract** - the lane asserts an interface, path, record key, or wording a later authorized landing replaced, and that landing is named; **fixture drift** - the lane's own scaffold no longer builds what the Engine accepts while the behaviour under test is unchanged; **live regression** - the Engine violates a requirement still frozen in the goal. The class fixes the act. The first two are ported: fixture and expectations rewritten to today's spelling, every lane id and the requirement it was cut for preserved. A live regression is fixed in the Engine under that requirement's own gap record - re-judged `partial` and moved back to the active folder - and the lane's assertion is left as it stands. An expiry marker is reserved for a lane whose behaviour under test no longer exists at all; a lane older than every edition has no last-good edition to name and is therefore ported, never expired. No lane is expired, deleted, or loosened to silence a red verdict, and the ticket that does the act records each crate's class and the named landing. | accepted | The sweep's `red` means "live regression" only if nothing else can produce it (`LNR-002`); without a rule, the cheapest exit from a red guard is a marker, and `t-058`..`t-070` show why that would lie - none of them ever passed at any edition, so a marker naming one would be invented history. The triage of 2026-09-17 (design.md) found every one of the twenty red verdicts to be a retired contract or fixture drift and not one live regression, which is exactly the finding a marker would have hidden. | [schema.md](../../../schema.md#rlr-001---triage-precedes-the-act) |
| `RLR-002` | The thirteen pre-split crates `t-058`..`t-070` are ported to the current Engine and read `pass` in a fresh sweep, each keeping its six lane ids and the requirement each lane was cut for. The port covers the engine root (`.arca/` -> `.ratmac/`, `state.toml` -> `run.toml`, `.arca/rtm.lock` -> `.ratmac/locks/`), the Run Record and verdict key `phase` -> `state` with `Phase:` -> `State:` in reports, the blocker-based `rtm hold` in place of the retired ticket argument (`t-063`), and the retired public `doctor::render_json` replaced by a JSON-stability oracle over `rtm doctor --json` (`t-064`, `t-067`, `t-070`). | accepted | Every one of the thirteen is refused at its first `rtm start` by the pre-split residue guard (`src/scheduler.rs`, `refuse_flat_residue_at`), five of them before that at build time; the behaviours they were cut to prove - routing (FDC-001), completion (FDC-002), verdict delivery (FDC-003), addressing (FDC-004..FDC-006), motion authorization (FDC-007), termination (FDC-008), composition (FDC-009..FDC-012) - all survive in `src/` under the new spelling, so these are the only lanes proving those contracts adversarially and porting keeps that proof alive. | [schema.md](../../../schema.md#rlr-002---the-pre-split-crates-are-ported) |
| `RLR-003` | The seven post-split crates `t-083`, `t-085`, `t-092`, `t-093`, `t-095`, `t-096`, `t-100` read `pass` in a fresh sweep: `t-083` and `t-085` are ported to the edition-004 command surface (the `next:` teach line and the `skill` verb), `t-092` and `t-093` cite the goal freeze the Engine mints at the step into `cut-tickets` instead of the empty pre-freeze value, the qa baseline helper's path comparison (`test/qa/src/baseline.rs`, `freeze_paths` via `canonical()`) stops carrying a trailing newline so `t-085`'s terminal-path lane compares what it lists, and `t-095`, `t-096`, `t-100` - green since the edition-004 tag repair - are re-swept rather than touched. | accepted | These are the lanes for the shop's own cycle (the state prompt, the cutover baseline, the Plan-Build Runbook traversal, per-Run answering, the edition audit, the history gates); left red they hide any real regression in the cycle behind a known one. The triage found each cause in a later authorized landing (`t-103`/`t-104` for the teach line and verb; `ARF-001` for the `goal-sha256:` citation demand) or in stale state (the edition-004 tag repaired after the 10:31Z sweep), and one helper defect that makes a passing listing compare as absent. | [schema.md](../../../schema.md#rlr-003---the-post-split-crates-are-ported-or-re-swept) |
| `RLR-004` | The sweep roster in `.ratmac/lanes.toml` names every landed hidden crate, and the landing that adds a hidden crate adds it to the roster in the same change - `t-108` joins now. The `check` verb refuses a stray entry (a crate in the folder the roster does not declare) the way it already refuses a missing one, so a landed crate can never be skipped silently. With the roster complete, `python tools/sweep_lanes.py check` exits `0` on this repository - every rostered crate `pass` or `expired`, no stray - and `sweep --verify-expired` shows each marker's lane (`t-078`, `t-079`) still refusing, so no marker outlives its lane's recovery. | accepted | The roster is what makes "every landed lane" a checked claim rather than a folder listing (`LNR-001`), but nothing adds to it: `t-108` landed its own six-lane crate and the roster still ends at `t-107`, and `check` reports a stray as information only, so the close would pass with one landed crate never run - the exact silence `i-037` set out to end. | [schema.md](../../../schema.md#rlr-004---the-roster-follows-the-landings-and-the-close-guard-passes) |

## Acceptance criteria

- A fresh sweep over this repository reads every crate `t-058`..`t-108`
  `pass` except `t-078` and `t-079`, which read `expired` naming
  `edition-001`; the summary line agrees with the rows; no stray entry.
- Every ported crate keeps exactly its six lane ids, and each lane's oracle
  still names the requirement it was cut for; a review comparing the ported
  lane to its pre-port source finds renames, fixture rewrites, and citation
  reorderings only - no deleted assertion, no loosened equality.
- `python tools/sweep_lanes.py check` exits `0`; a fixture folder holding a
  crate the roster does not declare makes `check` exit `2` naming it.
- `python tools/sweep_lanes.py sweep --verify-expired` shows `t-078` and
  `t-079` still red when run, so their markers stay honest.
- The step out of this repository's `close` State no longer refuses on the
  lane-sweep guard.
