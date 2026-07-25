# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `TWLV-001-issue-shape` | `TWL-001`–`TWL-010` | The pending issue contains exactly the five required populated files with matching identity and provenance, resolved relative routes, and no template markers. |
| `TWLV-002-clean-start` | `TWL-001`, `TWL-002`, `TWL-003` | QA fixture via `cargo test -p ratmac-qa` (suite names assigned at P4): in a temporary Git repository on a clean experiment base, start creates `trial-001-<slug>` at the base tip plus its linked worktree at the derived sibling path and reports both; ref, tag, and worktree snapshots show exactly those two additions. |
| `TWLV-003-dirty-or-colliding-start-refused` | `TWL-001`, `TWL-002` | Negative: staged, unstaged, and untracked base variants, plus duplicate branch, registered worktree, sibling directory, tag, and durable-destination collisions, and a malformed slug, each refuse with a named reason; refs, index, working tree, tags, and worktree registrations are byte-identical across every refusal. |
| `TWLV-004-numbering-deterministic` | `TWL-002` | With a live `trial-001-*` branch and an archived trial 002 surviving only as tag and `trials/` directory, inference yields 003 zero-padded; an explicit request for 001 or 002 refuses naming the colliding branch, tag, or directory. |
| `TWLV-005-atomic-start-rollback` | `TWL-003` | Negative: an induced mid-creation failure leaves no new branch ref, tag, worktree registration, or sibling directory — snapshots before and after are identical; the refusal names the failure. |
| `TWLV-006-log-validated` | `TWL-004` | A complete committed `trial-log.md` passes validation; a missing required section, an empty section, and identity facts contradicting the actual branch or commits each refuse before any tag creation or deletion (negative), naming the defect. |
| `TWLV-007-finish-sequence-and-recovery` | `TWL-005`, `TWL-008` | A successful finish leaves: the annotated archive tag at the terminal trial commit, one new base commit containing exactly `trials/<trial-branch>/trial-log.md`, no trial worktree, no trial branch — created in that order; the printed recovery commands then recreate the trial branch pointing at the identical commit. |
| `TWLV-008-finish-refusals-safe` | `TWL-005`, `TWL-010` | Negative: a dirty trial worktree, a missing log, an invoking working directory inside the trial worktree, a held directory handle (deterministic lock fixture), and a same-named tag at a different commit each refuse the failing step and all later steps with a named reason; no forced removal occurs, no process is terminated, already-created tag and log commit survive, and remaining recovery commands are printed. |
| `TWLV-009-dry-run-smoke` | `TWL-006`, `TWL-009` | Real smoke in this checkout plus fixture runs: status/dry-run prints base tip, cleanliness, live and archived trials, next inferred identity, and per-verb planned mutations with recovery commands; refs, index, working tree, tags, and worktree registrations are byte-identical before and after. |
| `TWLV-010-sync-merge-only` | `TWL-007` | A fix committed on fixture `main` reaches the clean experiment base only via the explicit merge path as a merge; a conflicting fixture merge stops non-zero with conflict markers left visible, nothing auto-resolved and nothing auto-aborted; a dirty base sync request refuses without mutation (negative). |
| `TWLV-011-containment` | `TWL-008` | After a finished trial, the base diff against its pre-trial tip contains exactly the durable log file and no trial implementation content; `main` is untouched by every lifecycle verb in all fixtures. |
| `TWLV-012-offline-minimal-interface` | `TWL-009`, `TWL-010` | An executable audit of the interface surface finds exactly the documented verbs and no push, fetch, network, global-install, PATH or global-config mutation, reset, rebase, forced worktree removal, forced branch deletion, or unrelated worktree pruning; branch teardown is the tag-verified compare-and-delete operation. Contributor guidance states the ownership split (human or Main-Agent invoke lifecycle verbs from the primary repository checkout with the experiment base checked out; Advisor authors the log; Subagents invoke neither lifecycle verbs nor `rtm`) and the Windows working-directory rule. |

All checks except the single read-only smoke run through the QA harness in temporary Git repositories and worktrees; none require commit, push, deployment, network access, or global installation.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/current/index.md` | updated | Link `i-010-trial-worktree-lifecycle` and summarize the reversible trial-worktree lifecycle around the experiment base. |
| `.arca/current/ubi-lang.md` | updated | Define Experiment base, Trial, Trial branch, Trial worktree, Trial log, Durable log destination, Trial archive tag, Dry-run preview, and Main-first fix flow. |
| `.arca/current/spec.md` | updated | Integrate `TWL-001`–`TWL-010` with stable requirement IDs. |
| `.arca/current/design.md` | updated | Record the accepted lifecycle mechanics: single repo-local interface, deterministic identity derivation, tag-before-delete finish, guarded deletion, merge-only sync. |
| `.arca/current/test-list.md` | updated | Add `TWLV-002`–`TWLV-012`, including the refusal, rollback, lock, and conflict negatives. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | Remains the one-line pointer to `.arca/index.md`; no policy text lives here. |
| `.arca/index.md` | updated | Add the contributor note naming the trial lifecycle entry point, the ownership split, and the Windows working-directory rule. |
| `tools/trial.ps1` (proposed) | updated | New home of the single-script lifecycle interface once implemented; exact path confirmed at integration. |
| `.arca/state.toml`, `.arca/log.md`, `.arca/rtm.lock` | unaffected | Issue creation and every lifecycle operation touch no Scheduler-owned runtime artifact; no `rtm` command is involved in trials. |
