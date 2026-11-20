# t-151 / run-076 independent source and oracle review

## Round 1 — Verdict: Rejected (historical; superseded by Accepted round 2 below)

One medium public-oracle gap must be repaired before expensive proof. No production defect was found in the reviewed delta. One additional low-priority classification gap is recorded below. This is source review, not a claim that full proof or mutation checks have run.

Reviewed the unstaged production delta against `d21a3da89d73dea50af3c7f3050fe3687ecf5f03`, that commit's added public function, and the approved ticket. The test commit adds exactly one 367-line block, with no deletions or changes to existing helpers or tests. No tests, builds, lifecycle commands, checkpoint operations, or mutations were executed by this reviewer. The retained run-075 failure and run-076 baseline receipt were read, not reproduced. The reported focused green result is accepted as supplied evidence, not independently rerun.

## Findings

### Medium M1 — denial consuming the remaining window is not exercised

`test/qa/tests/t108_lane_sweep.rs:598-655` tests reaching the deadline during sleep, including exact equality and oversleep. No controlled replacement attempt itself advances the clock to or past the deadline before raising its denial. Consequently the `remaining <= 0` branch at `tools/sweep_lanes.py:339-341` is not challenged.

The production branch is correct. However, deleting it appears to survive every current probe: their pre-sleep `remaining` values are always positive. In a real retry that returns a denial after consuming the remaining window, that deletion would call `sleep(0)` or `sleep(negative)` instead of immediately raising the last denial. A negative sleep masks the required last `PermissionError` with `ValueError`.

Required repair: add deterministic child-local cases in which the second eligible denied replacement advances `Clock.now` to exactly `105.0`, and separately to `105.001`, after a first denial at `100.0`. Require two attempts, only the first 10 ms sleep, the exact second error object, unchanged old-report bytes, and no temporary report. No real five-second wait is needed. The ticket explicitly requires rejection and repair of missing edges before full proof; this is a missing timeout/error-preservation edge, not a requested production rewrite.

### Low L1 — exception class is not independently distinguished from Windows code

`other_errors_are_single_try` checks an `OSError` with errno 5 but no `winerror == 5`. Broadening `except PermissionError` to `except OSError`, while retaining the Windows/code filter, therefore appears to survive the present probes. The production code has the required narrow catch; ordinary native Windows access-denied errors also map to `PermissionError`, so this is lower risk than M1.

Add a Windows-mode plain `OSError(5, label)` with an explicitly assigned `winerror = 5`; require one attempt, no clock sleep, original-error identity, old bytes, and cleanup. This independently locks the ticket's literal requirement that all three conditions—Windows, `PermissionError`, and Windows code 5—are necessary.

No other blocker, medium, or low finding was identified in this pass. Survival statements above are static predictions, not observed mutation results.

## Production assessment

- **Root-cause fit and minimality:** the retained failure occurred after expired verdicts were computed, at `os.replace` with native `PermissionError` / WinError 5. A report-local retry fits that observed publication failure. The original holder and denial duration remain unknown; this mitigates transient denial, not permanent access restrictions. The delta adds only documentation, `time`, and the short replacement loop. No dependency, Engine behavior, runner, verdict, expiry rule, or close guard changes.
- **Deadline:** the first eligible denial initializes one monotonic deadline to now plus five seconds; staging and the first replacement do not consume that window. Later denials do not reset it. Nonpositive remaining time immediately raises. Sleep is `min(0.01, remaining)`. The post-sleep check rejects both equality and oversleep before the next replacement. The code preserves the last caught error with bare `raise`. As with any deadline check, OS-call duration and scheduler suspension are not a hard five-second wall-clock completion guarantee; no late retry is intentionally admitted.
- **Classification:** every failure is reclassified, including failures after an eligible denial. Only Windows `PermissionError` carrying `winerror == 5` enters the wait. Other Windows codes, missing Windows codes, other exception classes, and non-Windows failures retain single-try behavior.
- **Publication and errors:** report construction and crate execution stay outside the loop. One closed, complete same-directory temporary file is reused. The successful publication remains `os.replace`; no unlink-before-replace or in-place destination write was introduced. The existing outer `BaseException` cleanup and error propagation remain unchanged. The existing possibility that cleanup itself fails is neither introduced nor repaired here.

## Public test assessment

- The unobstructed control establishes the valid fixture first, with sorted complete expired rows and an independently checked total. The existing growing-repository, missing-crate, total, expiry, close-guard, and write-scope assertions are not weakened.
- On Windows, `CreateFileW` explicitly permits read/write sharing and denies delete sharing. The wrapper calls the real `os.replace`, observes the actual code-5 denial, and only then releases the handle. The retained baseline output proves that path failed on the original single-try implementation, not on compilation, a helper, or fixture setup. Release is synchronous with the denied attempt; no racing marker or arbitrary release delay is used. This removes the avoidable timing race, though extreme host suspension or external interference can still make a real bounded operation fail.
- Each intercepted attempt checks that the old report is byte-identical, the source is a complete same-directory temporary report, and retries reuse the same source path and bytes. Recovery requires exactly one successful native replacement, identical published bytes, complete rows/total, and no temporary file. Lasting synthetic denial requires unchanged old bytes and cleanup.
- Deterministic probes cover first-denial origin after slow pre-denial work; fixed five-second deadline/no reset; clipped final sleep; exact-at-deadline and overslept retry refusal; last-error identity; ineligible first and subsequent errors; non-Windows behavior; and exactly-once verified crate execution. M1 is the remaining ordinary timing edge. L1 isolates the exception-class axis more precisely.
- The fixture owns its copied script, declaration, crates, report, and clock-probe directory under `TempTree`. Monkeypatches affect only the launched Python child and its imported module, using namespace copies rather than shared parent environment changes. The import child is launched with `-B`; direct script execution does not cache that script. Original source and private markers are not imported or edited. Marker bytes are checked after all probes. There is no silent fixture drift in the reviewed test delta.
- The unchanged default `lane_sweeps` target includes this module, so workspace proof includes the new function as well as the original LNRV-001/002/005 coverage; the explicit standalone target is also available.

## Hidden-lane declaration and full-proof obligation

**`hidden-lanes: []` is acceptable under the frozen archived i-037 plan.** That plan declares LNRV-001 through LNRV-005, with no independently declared hidden IDs. The confirmed protocol binds IDs literally to that issue plan; inventing a new hidden ID would fail that binding. Strengthening the existing LNRV-001 public check does not require editing the frozen issue or minting an ID.

The ticket explicitly assesses all six categories, and the public check supplies independent adversarial oracles: regression of recovery; input/routing through error and platform selection; lifecycle through retry/deadline state; durability/recovery through old-report preservation and recovery; output/filesystem through whole staged bytes, atomic publication, cleanup, and markers; cross-feature through expiry and verified exactly-once execution. These are reasonable category assignments for this narrow report repair. M1 must be closed before accepting their coverage as complete.

The empty list is **not** permission to omit prior private proof. The unchanged roster contains all 93 crates, `t-058` through `t-150`; all remain mandatory full proof through `tools/turn.ps1`, including t-108's atomic-report and write-scope lanes. Original full-workspace checks, affected regressions, and all four ticket quality commands remain mandatory. The correct order remains accepted exact-byte review, complete private/quality proof, safety checkpoint, damage/restoration, landing, and primary-checkout confirmation. Source or oracle repairs invalidate this snapshot's review and affected proof.

## Finite checkpoint mutation recommendations — static only

All variants target checkpoint-tracked `tools/sweep_lanes.py`, one independent change at a time. Use the focused public function as the fast named oracle; retain mandatory checkpoint restoration and restored-green verification through `tools/turn.ps1`. This table recommends probes, not recorded kills. No variant was run. Observed kills belong only in `res-161` after execution.

| Variant | Single deliberate defect | Expected public oracle |
| --- | --- | --- |
| D1 | Return to one replacement attempt | Native observed-denial recovery fails. |
| D2 | Start the deadline before the first replacement | `first_denial_origin` fails after its 20-second synthetic first attempt. |
| D3 | Reset the deadline after every denial | `fixed_deadline_and_last_error` rejects the extra interval or wrong clipped sleep. |
| D4 | Remove the post-sleep deadline check | Exact-boundary or overslept probes reject a late attempt, even if it would succeed. |
| D5 | Remove the nonpositive-remaining check | **Potential current survivor; M1's added at/after-denial cases must kill it.** |
| D6 | Always sleep 10 ms without clipping | The fixed-deadline probe rejects the final sleep instead of 5 ms. |
| D7 | Ignore the Windows-code restriction | Existing code-32/no-code single-try cases fail. |
| D8 | Ignore the Windows-platform restriction | Existing non-Windows synthetic-code-5 case fails. |
| D9 | Catch `OSError` rather than `PermissionError` | **Potential current survivor; L1's plain-error/code-5 case should kill it.** |
| D10 | Write directly to the destination instead of atomic replacement | Native attempt/publication assertions or old-report inspection fail. |
| D11 | Remove failure-path temporary-file cleanup | `clean` fails after deterministic denial. |
| D12 | Re-enter the sweep or recompute crates on retry | Staged-report identity or `recovery_does_not_rerun_crates` rejects changed staging/duplicate calls. |

## Exact reviewed bytes

Tracking states are those of the frozen review snapshot: HEAD is the committed red test at `d21a3da`; only the production file has an unstaged implementation delta. SHA256 values cover working-tree file bytes, not Git-normalized blobs.

| Path | Tracking state | SHA256 |
| --- | --- | --- |
| `tools/sweep_lanes.py` | Tracked; unstaged production modification | `b4f27141d4eac70817efb9e5db555c418c20ecda757dfaee29c2b2046b6070f5` |
| `test/qa/tests/t108_lane_sweep.rs` | Tracked; clean; red-test commit | `92432d11aeb86ed7003e60333e16425590c0dab6d9d123a05e536fb19fcfcb1c` |
| `.arca/ticket/t-151.md` | Tracked; clean; approved plan | `7162fc65c7fc557de9e6fdd4a4e4c2d69da6d5b89f7a57fcb53d415936e84f1b` |

## Round 2 — Current verdict: Accepted

Renewed targeted review of the public author's M1/L1 repair. **Zero open blocker, medium, or low findings.** The round-1 findings, original byte table, and twelve static mutation recommendations above remain historical evidence; this section and its manifest define the currently accepted snapshot.

The source/test repair was frozen before this pass. The unstaged test delta contains only 34 added lines: the plain-error helper, the two consumed-window cases, the classification-list entry, and the new probe invocation. No existing assertion, helper, original test, fixture, or production code was removed or weakened. Working-tree production and ticket hashes are identical to round 1. No tests, builds, variants, lifecycle commands, or Git writes were executed in this pass. The coordinator-reported focused green result—one test passed, 0.67 seconds—is accepted as supplied evidence, not rerun.

### M1 resolved — denial at/after deadline preserves the last error without waiting

`denial_consumes_remaining_window` at `test/qa/tests/t108_lane_sweep.rs:652-674` independently runs exact-boundary `105.0` and overrun `105.001` cases. Both start with an eligible denial at `100.0`, permit one 10 ms sleep, and advance time inside the second replacement attempt before raising a distinct second denial. The oracle requires exactly that second error object, exactly two attempts, exactly the first sleep, unchanged final clock value, old destination bytes, and no temporary residue. The probe is explicitly invoked at line 740. It exercises the previously unchallenged nonpositive-remaining branch without real waiting or any global hook.

Removing the pre-sleep nonpositive-remaining guard would now reach the controlled clock's rejection of zero/negative sleep and fail the original-error identity oracle. This is a static argument, not an observed mutation kill.

### L1 resolved — Windows code 5 does not make a plain OSError retryable

`plain_os_error` at lines 558-564 constructs `OSError(5, label)`, assigns `winerror = 5` afterwards, and asserts the exact exception type is still `OSError`. The Windows-mode classification case at line 681 requires one attempt, no sleep, the same original error, unchanged old bytes, and cleanup. It therefore distinguishes exception type from Windows code and avoids accidentally constructing a `PermissionError` through Windows constructor promotion.

Broadening the production catch to `OSError` would now enter a forbidden retry and fail the classification oracle. This is a static argument, not an observed mutation kill.

### Unchanged conclusions and next proof gate

The round-1 production assessment, native-denial observability, deterministic deadline/classification coverage, child-local isolation, bytecode suppression, atomic-report/cleanup contract, and preservation of original tests remain accepted. The repaired probes close both recorded gaps without widening production scope. No additional findings were identified by this targeted pass.

`hidden-lanes: []` remains acceptable under the frozen i-037 plan and confirmed literal issue binding. All six adversarial categories now have the accepted public coverage described above. This does not waive any private proof: all 93 existing crates, affected original sweep regressions, the workspace suite, and all four declared quality commands remain mandatory through `tools/turn.ps1` before checkpoint/damage. This review grants no completion or mutation-kill claim.

Retain D1–D12 exactly as recorded above for the coordinator. D5 and D9 are no longer identified coverage gaps: the new M1 and L1 oracles respectively provide the expected kills. Execution, checkpoint restoration, restored-green verification, and recording actual kills only in `res-161` remain future coordinator work.

### Current exact reviewed bytes

SHA256 values cover the latest working-tree file bytes. The production modification and the test repair are unstaged relative to the tracked red-test checkpoint; the approved ticket is unchanged. These current hashes supersede the round-1 manifest for acceptance.

| Path | Tracking state | SHA256 |
| --- | --- | --- |
| `tools/sweep_lanes.py` | Tracked; unstaged production modification; unchanged since round 1 | `b4f27141d4eac70817efb9e5db555c418c20ecda757dfaee29c2b2046b6070f5` |
| `test/qa/tests/t108_lane_sweep.rs` | Tracked; unstaged M1/L1 repair additions | `c6c2c1574b638620c89096a6481d91ae6f6e9aeddb8c7cca4a8d5e2ca6b9f8d6` |
| `.arca/ticket/t-151.md` | Tracked; clean; unchanged approved plan | `7162fc65c7fc557de9e6fdd4a4e4c2d69da6d5b89f7a57fcb53d415936e84f1b` |

## Round 3 — 2026-11-20 — Current verdict: Accepted

Read-only re-check of the round-2 acceptance after the coordinator appended one uncommitted line (701) to `.arca/log.md`. **The round-2 acceptance still holds. There are no blocker, medium, or low findings.** Rounds 1–2, both earlier byte tables, and D1–D12 are unchanged above. This pass ran only `git diff/show/status/log/blame/cat-file/hash-object`, `sha256sum`, and file reads. It ran no tests, builds, Engine, lifecycle, or turn commands, and made no Git writes. This pass did not observe the reported results: the private sweep (91 pass, 2 expired, 0 red, 0 missing, 93 crates), the one-time workspace failure at `wcpv_006`, and the focused `wcpv_006` pass afterward. They are accepted as evidence supplied by the coordinator.

### F1 — the log change is a pure one-line append (holds)

`git diff --numstat -- .arca/log.md` shows `1 0`, with one hunk at `@@ -698,3 +698,4 @@` that adds only line 701. The HEAD blob is 339,915 bytes, has sha256 `f0920553d2999e8ccd68ab44f5ec8495e47f9d6616474eaa580b57e47d31133a`, and ends in LF. The first 339,915 bytes of the working file have the same digest. The working file is 340,390 bytes. The 475-byte tail is exactly one LF-terminated line, and neither copy contains a CR. The inventory's frozen prefix (`.ratmac/evidence/run-037/baseline-inventory.toml`: 235,240 bytes, through line 524, `1ac6ec89…09c9e`) is still byte-identical.

### F2 — line 701 is truthful (holds; two informational notes)

- Commit `04af2d9` ("Reopen the lane report publication gap", 2026-11-18T20:57:20-07:00, which is 2026-11-19 UTC) is a real reopening. It deletes `.arca/residual/archive/res-161.md`, whose pre-commit blob matches the inventory sha256 `1cfac14b…f97a`, and creates `.arca/residual/res-161.md` with `status: "partial"` and owner t-151. The same commit appends line 698. That matches the schema's "reopening is a visible move" rule. Line 701's "reopened on 2026-11-19" uses line 698's own log date.
- Line 698 names `res-161` and contains no `reopen` in any case. Line 699 contains `reopened` but does not name `res-161`. Line 700 contains neither. After the frozen prefix, the only lines naming `res-161` as a whole token are 698 and 701. So before line 701, no appended line met `reopened_gap_record`'s same-line test (`t126_shared_private_test_support.rs:961-994`). The causal claim therefore matches the code. Line 701 names `res-161` once and `reopen` three times, and names no other record.
- Note (i): "failed once in t-151's workspace proof" is supplied by the coordinator. No retained output for it exists under `.ratmac/evidence/run-076/`.
- Note (ii): "no earlier line, test, or frozen evidence changed" is true for this trace. Earlier lines, t126, and the run-037 inventories are unchanged, and the lane-sweep report matches HEAD. The separately reviewed t108 additions are a different, earlier change. Neither note is a false claim.

### F3 — reviewed source and oracle bytes are unchanged (holds)

`tools/sweep_lanes.py` = `b4f27141…6070f5` and `test/qa/tests/t108_lane_sweep.rs` = `c6c2c157…b9f8d6`. Both match round 2 exactly. HEAD's t108 is still the round-1 red-test digest `92432d11…fcb1c`, and the working t108 diff is still `34 0` (additions only).

### F4 — nothing was weakened, and the exemption is genuinely met (holds)

`t126_shared_private_test_support.rs`, both run-037 inventories, `.arca/schema.md`, `.arca/residual/`, `.arca/issue/`, and all of `.ratmac/evidence/` have no difference from HEAD. The only exception is this untracked review file. The t126 file and inventories also have no difference since `04af2d9^`. `.ratmac/evidence/lane-sweep/report.md` matches its HEAD blob (sha256 `297eb175…eed`). `reopened_gap_record` is satisfied on all three of its conditions: (a) an appended line names `res-161` and a reopen; (b) exactly one copy exists, because the archive copy is gone and the active copy is present; (c) that copy's first `residual-id:` line is `residual-id: "res-161"`. The pass therefore comes from the committed reopening, with no change to the oracle.

### F5 — no other preserved archived record differs without a trace (holds)

Each of the 542 `[[preserved]]` inventory digests was checked against current bytes. Only two records differ from the inventory:

- `.arca/residual/archive/res-161.md` is absent, as covered in F2/F4.
- `.arca/residual/archive/res-110.md` (`325847d2…1d97`, equal to HEAD) is already traced by committed log line 609 (2026-10-09). That line names `res-110` and a reopen. Exactly one copy exists, the archived one, and it keeps `residual-id: "res-110"`. Commit `77a1472` reopened it and `ffd9eaa` re-archived it.

`sha256sum -c` also flagged `.arca/issue/archive/i-015-cycle-as-runbook/index.md`, but that was a check-mode artifact. Direct hashing of this CRLF file gives exactly the inventory digest `3111c3d3…6b90`, which is also its HEAD digest.

### Declared snapshot exceptions

These working-tree states are not committed or staged:

- Unstaged: `tools/sweep_lanes.py`, `test/qa/tests/t108_lane_sweep.rs`, and `.arca/log.md`.
- Untracked, not reviewed here: `.arca/ticket/t-151-proof-receipt.txt` (`5b54e0ff5cf9ac53a37bbbca5aff2d4b80b9ddb3d44be028b268084be48de033`), `.arca/ticket/t-151-review-record.txt` (`4505406ae38eda735283738f031ffe47dc37c62e447ffc1e47bf1ff95f794e7b`), and this review file.

This review does not waive any private proof or mutation obligation from round 2.

### Round 3 exact bytes

| Path | Tracking state | SHA256 |
| --- | --- | --- |
| `tools/sweep_lanes.py` | Tracked; unstaged production modification; unchanged since rounds 1–2 | `b4f27141d4eac70817efb9e5db555c418c20ecda757dfaee29c2b2046b6070f5` |
| `test/qa/tests/t108_lane_sweep.rs` | Tracked; unstaged M1/L1 repair additions; unchanged since round 2 | `c6c2c1574b638620c89096a6481d91ae6f6e9aeddb8c7cca4a8d5e2ca6b9f8d6` |
| `.arca/ticket/t-151.md` | Tracked; clean; unchanged approved plan | `7162fc65c7fc557de9e6fdd4a4e4c2d69da6d5b89f7a57fcb53d415936e84f1b` |
| `.arca/log.md` | Tracked; unstaged one-line append (line 701) over the HEAD blob `f0920553…1133a` | `6b01667a28e5866f474306e6b177d36fa9764cf7d79a58915f5331f39510a768` |
