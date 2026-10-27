# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Changed pinned file refuses (`LRPV-001`) | [LRP-001](spec.md#requirement-records) | In a fixture like the bootstrap doctor suite's whose runbook has a second State and one transition, the Engine planted at `target/debug` starts `run-001` from the fixture root, so the Engine itself writes the record (`status = "planned"`) and the pin. One byte is then appended to that binary. `pwsh -NoProfile -File tools/rtm.ps1` exits 1; standard error starts `bootstrap refused: the resolved Engine is not the pinned Engine` and names `run-001`, its `evidence.toml`, the observed and expected path and hash, and `rtm abandon --run run-001 --confirm "abandon run-001"`; standard output has no `Engine:` or `Pin:` line; and the whole fixture is byte-identical before and after. The same holds with the record's status line changed to `executing`. Today it exits 0 and prints `Pin: no pin recorded in .ratmac/evidence.toml`. Candidate: `test/qa/tests/t045_bootstrap_doctor.rs`, replacing `fn bootstrap_refuses_pin_mismatch`. |
| Matching and other pins pass (`LRPV-002`) | [LRP-001](spec.md#requirement-records) | With `run-001` started as above and the binary unchanged, the bootstrap exits 0 and prints `Pin: matches the pin of run-001`. A copy of the Engine at another path in the fixture then starts `run-002`, which pins that copy: the bootstrap still exits 0 and prints the `run-001` line, then `Pin: run-002 pinned another Engine:` naming the copy's path. Three more Run folders, each with an `evidence.toml` pinning the resolved path with another hash, add no line and no refusal: one whose record reads `status = "passed"`, one with no `run.toml`, and one whose `run.toml` is a folder. With no live Run, the one pin line is `Pin: no live Run in` naming the fixture's `.ratmac`. Nothing is written. Today every case prints `Pin: no pin recorded in .ratmac/evidence.toml`. Candidate: `test/qa/tests/t045_bootstrap_doctor.rs`. |
| Flat pin file never read (`LRPV-003`) | [LRP-001](spec.md#requirement-records) | With no live Run and a flat `.ratmac/evidence.toml` whose `[engine]` names the resolved binary with another hash, the bootstrap exits 0, prints `Pin: no live Run in` naming the fixture's `.ratmac`, never names the flat file, and writes nothing. Today it exits 1 with "the resolved Engine is not the pinned Engine". Candidate: `test/qa/tests/t045_bootstrap_doctor.rs`. |
| A linked worktree reads the shared root (`LRPV-004`) | [LRP-001](spec.md#requirement-records) | The fixture is a Git repository with one commit holding its two-State runbook, source, and `tools/rtm.ps1`, with `target/` ignored, and a linked worktree beside it whose `target/debug` holds the planted Engine. That Engine, run in the worktree, starts `run-001`, whose record lands under the primary checkout's `.ratmac/runs/` and whose pin names the worktree's binary. The worktree's bootstrap exits 0 with `Pin: matches the pin of run-001`; after one byte is appended to that binary it exits 1, naming `run-001` and the evidence file under the primary checkout. With `GIT_DIR` set to the primary checkout's Git folder in the bootstrap's environment, both outcomes stay the same. Today both runs exit 0 with `Pin: no pin recorded in .ratmac/evidence.toml`. Candidate: `test/qa/tests/t045_bootstrap_doctor.rs`. |
| No readable pin is named, never fatal (`LRPV-005`) | [LRP-001](spec.md#requirement-records) | With `run-001` started as above, its `evidence.toml` is replaced first by a folder of that name, then by a file with no `[engine]` table. Each time the bootstrap exits 0, prints `Pin: run-001 has no readable pin in` naming that path, shows no PowerShell error record on standard error, and writes nothing. Today both exit 0 and print `Pin: no pin recorded in .ratmac/evidence.toml`. Candidate: `test/qa/tests/t045_bootstrap_doctor.rs`. |
| Regression private lane (`HT-145-01`) | [LRP-001](spec.md#requirement-records) | Oracle in ticket t-145's hidden-test table; private suite under `test-hidden/t-145/`. |
| Input/Routing private lane (`HT-145-02`) | [LRP-001](spec.md#requirement-records) | Oracle in ticket t-145's hidden-test table; private suite under `test-hidden/t-145/`. |
| Lifecycle/Model private lane (`HT-145-03`) | [LRP-001](spec.md#requirement-records) | Oracle in ticket t-145's hidden-test table; private suite under `test-hidden/t-145/`. |
| Durability/Recovery private lane (`HT-145-04`) | [LRP-001](spec.md#requirement-records) | Oracle in ticket t-145's hidden-test table; private suite under `test-hidden/t-145/`. |
| Output/Filesystem private lane (`HT-145-05`) | [LRP-001](spec.md#requirement-records) | Oracle in ticket t-145's hidden-test table; private suite under `test-hidden/t-145/`. |
| Cross-Feature private lane (`HT-145-06`) | [LRP-001](spec.md#requirement-records) | Oracle in ticket t-145's hidden-test table; private suite under `test-hidden/t-145/`. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

The private-lane rows (`HT-`) were added when the tickets were cut on 2026-10-27, so the selected-issue binding (WCP-003) permits the hidden lanes each owning ticket declares; they add no requirement.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../goal/spec.md#integrated-live-run-pin-requirements) |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../goal/spec.md#integrated-live-run-pin-requirements) |
| `.arca/goal/spec.md` | updated | [Accepted authority](../../goal/spec.md#integrated-live-run-pin-requirements) |
| `.arca/goal/design.md` | updated | [Accepted authority](../../goal/spec.md#integrated-live-run-pin-requirements) |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../goal/spec.md#integrated-live-run-pin-requirements) |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | unaffected | No working rule changes; the goal row governs. |
| `.arca/wishlist.md` | unaffected | No wish carries this issue. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
