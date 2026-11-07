# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| SDCV-001 | SDC-001 | `.arca/schema.md` carries the discard guard as one findable rule under the `SDC-001` heading: the named discard commands, the look-first step (`git status`, `git diff`), and save-or-park before any discard, with the restoration-is-not-a-discard sentence. `.arca/dict.md` defines Discard command and Park consistently with it. Behavioral proof of contributor compliance is future work owned by [i-015](../../deferred/i-015-cycle-as-runbook/spec.md#requirement-records); authority presence is this issue's acceptance. |
| SDCV-002 | SDC-002 | The `SDC-002` heading fixes the checkpoint subject `t-<id>: checkpoint - not a landing` and the restore command `git restore --source=<checkpoint> --staged --worktree -- <paths>` byte-for-byte; the dict.md Checkpoint entry repeats both strings exactly; the clean-tree verification (`git status --porcelain` prints nothing) is stated. |
| SDCV-003 | SDC-003 | The P5 row, the Units exception paragraph, and the Evidence-receipts paragraph in `.arca/schema.md` state one identical order - green, checkpoint, checks, restore, evidence, amended green landing, merge - with the gap record's `mutation-kill` list as the sole evidence home. `.arca/tpl/residual.md` carries the `mutation-kill` field and its note; `.arca/tpl/ticket.md` carries the pointer-only comment under `residual-ids`. Interruption recovery is defined for mid-damage, pre-amend, and stray-checkpoint cases. |
| SDCV-004 | SDC-004 | The integration landing preserves every pre-existing byte under `.arca/ticket/archive/`, `.arca/residual/archive/`, and `.arca/issue/archive/`: the only archive write is the authorized move of this bundle to `.arca/issue/archive/i-022-safe-deliberate-damage/` with relative links gaining one `../` level. `t-064.md` stays byte-identical; its backup-copy lesson is superseded only in the new schema prose. |
| SDCV-005 | SDC-005 | `.arca/issue/deferred/i-015-cycle-as-runbook/spec.md` `PCR-001` names the dirty-tree refusal and the intake gate's working-authority acceptance, dated 2026-08-03, disposition still `deferred`; no automation code, hook, or guard exists in this landing. |
| SDCV-006 | SDC-001, SDC-002, SDC-003, SDC-004 | `python tools/check_links.py` exits 0: this same integration landing taught it the working-authority branch - an accepted ask resolves to its requirement-ID heading in `.arca/schema.md` when no goal spec row carries it, and an ask resolving to neither fails by name. Differential evidence recorded at integration: before the tool edit, this bundle's four accepted `SDC` IDs were the tool's only failures (zero dangling links, five-file shape intact); after it, green; a probe row (`SDC-999`, accepted, resolving nowhere) failed by name and was removed, restoring the spec byte-identically. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | unaffected | - |
| `.arca/goal/ubi-lang.md` | unaffected | - |
| `.arca/goal/spec.md` | unaffected | - |
| `.arca/goal/design.md` | unaffected | - |
| `.arca/goal/test-list.md` | unaffected | - |

The goal rows are unconditional: these are working-process rules for contributors, not behavior of
the running program. Machine enforcement, when selected, enters the goal through i-015 - never
through this bundle.

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | - |
| `.arca/schema.md` | updated | P1 working-authority branch, P5 fixed order, Units exception paragraph, [Deliberate damage and discard safety](../../../schema.md#deliberate-damage-and-discard-safety) (`SDC-001`..`SDC-004` headings), Evidence-receipts alignment; reverse: this bundle. |
| `.arca/dict.md` | updated | Deliberate-damage check, Discard command, Checkpoint (safety commit), Park. |
| `.arca/tpl/residual.md` | updated | `mutation-kill` field plus the single-evidence-home note. |
| `.arca/tpl/ticket.md` | updated | Pointer-only comment under `residual-ids` (SDC-003). |
| `.arca/issue/deferred/i-015-cycle-as-runbook/spec.md` | updated | `PCR-001` carrier extension for `SDC-005` (duplicate). |
| `.arca/wishlist.md` | updated | The source wish left the pool at this bundle's creation - promotion made this bundle its sole carrier; provenance is recorded in [index.md](index.md). |
| `tools/check_links.py` | updated | Working-authority branch for accepted asks, landed in this same integration landing; verified by the SDCV-006 differential and probe. |
| `.arca/log.md` | updated | One P1 integration line, appended per the append-only history rule. |
