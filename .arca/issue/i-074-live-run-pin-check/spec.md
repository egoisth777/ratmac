# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `LRP-001` | The project-local bootstrap (`tools/rtm.ps1`), run from the primary checkout or a linked worktree, checks the Engine it resolves against the pin of every live Run in the one shared Engine root, and never reads the flat `.ratmac/evidence.toml`. After the `Engine:` and `sha256:` lines it prints one `Pin:` line per live Run, in Run-id order, saying that the resolved Engine matches that Run's pin, that the Run pinned another file, or that the Run has no readable pin; with no live Run, its one `Pin:` line says so. Its only pin refusal (`bootstrap refused:` on standard error, exit 1, no `Engine:` or `Pin:` line) comes when the resolved Engine is a live Run's pinned file and no longer has that pin's hash; it names each such Run with its pin file, the observed and expected path and hash, and the full command that retires the Run. It writes nothing. This supersedes the recorded-pin clauses of `ORS-002` and `ECP-002`. | accepted | Billy's 2026-10-27 planning decision: refuse only when the pinned file changed, so the default and nightly commands keep working while a sprint Run pins the stable Engine. | [Accepted authority](../../goal/spec.md#integrated-live-run-pin-requirements) |

## Evidence and boundaries

- The bootstrap ([rtm.ps1](../../../tools/rtm.ps1)) reads one pin file: `function Get-EnginePin` (lines 67-87) takes the `[engine]` table's `resolved` and `sha256` from the file it is given, and the main flow gives it `.ratmac/evidence.toml` under the checkout that holds the script (`$root`, line 253; lines 301-302). With a pin there it refuses on any hash difference, whatever the path, with "the resolved Engine is not the pinned Engine" and the guidance "the pin was recorded by the active Run in .ratmac/evidence.toml [engine]" (lines 304-312). Otherwise it prints `Pin: no pin recorded in .ratmac/evidence.toml` (line 317) or `Pin: matches .ratmac/evidence.toml [engine]` (line 319).
- The Engine writes no such file. At Run start it writes the pin into the Run's own `.ratmac/runs/<id>/evidence.toml` ([scheduler.rs](../../../src/scheduler.rs), lines 1820-1822; `EVIDENCE_FILE`, [pin.rs](../../../src/pin.rs) line 17). `fn engine_identity` (pin.rs, lines 247-258) records `resolved` as the running Engine's path written with `/`, its hash, and the build-time channel and source commit.
- Observed on 2026-10-27: `pwsh -NoProfile -File tools/rtm.ps1 -Channel stable` reported `Engine: E:\hive\packs\projs\skills\ratmac-stable-edition-009\target\debug\rtm.exe` with sha256 `74ba7d6ad4ee733dac7d90a1fadae804f292527e64e0d18559cedcc07491daf1`, printed `Pin: no pin recorded in .ratmac/evidence.toml`, and exited 0, while `run-060` was live (`status = "planned"`) with `.ratmac/runs/run-060/evidence.toml` pinning `E:/hive/packs/projs/skills/ratmac-stable-edition-009/target/debug/rtm.exe` and that same hash. So every real run reports no pin and compares nothing.
- Nothing else compares a live Run's pinned bytes. `Evidence::confirm_engine` (pin.rs line 151) is called only by [t101_engine_channels.rs](../../../test/qa/tests/t101_engine_channels.rs), and the doctor's live-Run check (`fn live_run_findings`, [channel.rs](../../../src/channel.rs) lines 114-168) compares channel and source commit, not bytes. It counts a Run as live by `fn run_is_live` (lines 170-180): a `run.toml` line reading `status = "planned"` or `status = "executing"`, and a record it cannot read is not live.
- The shared Engine root is resolved by the Engine (`fn git_repository`, [root.rs](../../../src/root.rs) from line 578, with Git queries made by `fn git_command`, which clears `GIT_DIR`, `GIT_COMMON_DIR`, and `GIT_WORK_TREE`) and, query for query, by the issue-binding validator (`def engine_root`, [verify_issue_binding.py](../../../tools/verify_issue_binding.py) lines 68-121, through `def git`, lines 43-65). The bootstrap has no such resolution; from a linked worktree, `$root/.ratmac` is the worktree's own folder, which holds no Runs.
- The bootstrap suite encodes the old file. [t045_bootstrap_doctor.rs](../../../test/qa/tests/t045_bootstrap_doctor.rs) `fn bootstrap_refuses_pin_mismatch` (from line 275) writes its pin into the fixture's `.ratmac/evidence.toml` (lines 280-289) and expects the refusal to name that file (line 327); `fn bootstrap_resolves_offline` expects "no pin recorded" (line 238). Its fixture runbook has one State, and in a scratch copy on 2026-10-27 the edition-009 Engine's `rtm start` with that runbook wrote `status = "passed"`; with a second State and one transition it wrote `status = "planned"` and a pin whose `resolved` uses `/`.
- Out of scope: where the bootstrap finds or builds the Engine, which belongs to the [cargo built bootstrap engine issue](../i-064-cargo-built-bootstrap-engine/spec.md) (`SVE-002`); Engine-side pin checks; and the doctor's findings.

## Assumptions

- "Live" follows the doctor exactly, so a blocked, passed, or failed Run, and a Run folder whose record cannot be read, adds no `Pin:` line.
- A pin is the `[engine]` table's `resolved` and `sha256`, read with the field names the Engine writes. Channel and source commit are not compared: the bootstrap cannot learn a binary's provenance without running it.
- Matching is by file first, then by bytes. A live Run that pinned another file never causes a refusal, which narrows today's rule (any hash difference refused) to the case Billy chose.
- The bootstrap resolves the shared Engine root the way the Engine does, with the three Git variables cleared for its Git queries; any failed or unsuitable query falls back to the checkout's own `.ratmac/`, as the Engine's resolution does.
- A live Run whose `evidence.toml` is absent, cannot be read, or carries no `[engine]` hash gets a `Pin:` line naming that file and causes no refusal; the bootstrap never stops on a PowerShell error there.
- The refusal keeps today's reason and its observed/expected lines, now once per changed Run.
- No new tool or dependency: PowerShell and Git are already used.

## Dependencies

- None on other selected issues. The cargo built bootstrap engine issue (`i-064-cargo-built-bootstrap-engine`, `SVE-002`) edits the same lines of `tools/rtm.ps1` and the same bootstrap suite; neither needs the other, but their tickets land one after the other.
- Must not break: the rest of the project-local bootstrap (`ORS-002`), channel resolution (`ECP-002`), the stable build checkout (`ELR-002`), the single shipped command (`DEB-001`), Engine-root resolution (`ENS-002`), the doctor suite, the private bootstrap lane (`t-102`), and the bootstrap's rule that it writes only the declared build output (`fn bootstrap_writes_only_declared_build_output`).

## Open decisions

- None. Billy settled the refusal scope on 2026-10-27; the mechanics follow existing rules.

The disposition is set at integration; the linked forward authority then governs implementation.
