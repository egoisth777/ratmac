# Issue design

## Proposed mechanics

### The failure this prevents

In the composition-format build turn (`t-064`, archived at
[.arca/ticket/archive/t-064.md](../../../ticket/archive/t-064.md)), a mutation-evidence revert ran
`git checkout -- src/machine.rs` after the green build. The completed green implementation of that
file was uncommitted and had never been staged, so the index still held the red-commit bytes: the
command restored the red state and destroyed the only copy - `git fsck` confirmed git held no blob.
The file was reconstructed in-place from its pinned contract (public tests, hidden lanes, parity
sets) and re-proved green. Both the temporary damage and the completed work were lost to a single
command whose entire purpose was to remove only the damage.

The lesson frozen in that ticket's P5 notes - restore mutation probes from an explicit backup copy,
never from git, while an increment is uncommitted - treated git as the hazard. The hazard was
discarding unsaved work; git done right is the cure. This issue supersedes that lesson (the archived
bytes stay frozen; the supersession lives in the new schema section).

### Decision 1 - the discard guard: look, then save or park (SDC-001)

Before any discard command runs, the contributor looks (`git status`, then `git diff` over anything
it lists) and saves or parks whatever is wanted beyond what the discard is meant to remove. A save
is a commit; a park is `git stash push -m "t-<id>: <what>"`, dropped only after its content lands or
is explicitly declared obsolete. `git clean` and `git reset --hard` count as discard commands at
every moment, not only during damage turns.

Alternatives rejected:

- **Ban version-control restoration entirely.** Rejected by human ruling: restoring saved bytes from
  a checkpoint is strictly safer than hand-written inverse edits, which silently drift from the
  intended clean state. The rule targets discarding *unsaved* work only.
- **Leave it to prose care.** That was the state of the world before `t-064`; it demonstrably fails
  under exactly the pressure it exists for.

### Decision 2 - damage only from a checkpoint (SDC-002)

After the turn's tests are all green, everything is committed as the safety commit, subject exactly
`t-<id>: checkpoint - not a landing`. It is ephemeral: unpublished, unmerged, never a Landing, no
log line. Damage is limited to paths the checkpoint tracks, so the checkpoint provably contains
every byte a restore must bring back. Each undo is checkpoint-sourced and explicit:

    git restore --source=<checkpoint> --staged --worktree -- <paths>

Never plain `git restore <paths>` or `git checkout -- <paths>`: those copy index bytes that may be
stale - the exact `t-064` failure. Never `git clean` as an undo. After each restore:
`git status --porcelain` prints nothing and the tree matches the checkpoint. Hand-written inverse
edits are rejected as the standard undo for the same drift reason as in Decision 1.

The subject line is fixed so a stray checkpoint is recognizable by its bytes alone - by a human, and
eventually by the runbook guard that i-015 owns.

### Decision 3 - fixed turn order and the single evidence home (SDC-003)

The order inside the code-writing step is: tests green -> safety commit -> each deliberate-damage
check from it -> restore and verify -> evidence -> green landing -> merge. Two consequences:

- **Evidence follows its check.** Kills are written only after the observed failure, and only into
  the owning gap record's `mutation-kill` list - the sole physical home. The ticket carries its
  `residual-ids` pointer and no evidence bytes: one home, one writer, no duplication to drift. The
  residual and ticket blanks ([tpl/residual.md](../../../tpl/residual.md),
  [tpl/ticket.md](../../../tpl/ticket.md)) now mechanize this shape.
- **The checkpoint never survives as history.** `git commit --amend` folds it into the final green
  landing: one commit carrying the code, the updated gap record, and the ticket reference, with its
  one required log line. Permanent ticket-branch history stays exactly red-then-green - the Units
  table rows are untouched, and the checkpoint is a stated exception paragraph, not a third landing.

Interruption recovery is defined at every point: mid-damage, restore from the checkpoint and verify;
after the checks but before the amend, the checkpoint holds everything - finish the gap-record
evidence, amend, log, merge; a stray checkpoint on an unmerged ticket branch is never merged as-is.

Alternatives rejected:

- **A third Landing for damage evidence.** Would rewrite the Ticket unit ("two required landings"),
  demand a new log line and evidence address, and put evidence bytes in permanent history twice.
- **Stamp kills into the ticket file.** Two physical homes for one fact; the gap record already owns
  mutation evidence (`res-*` files carry `mutation-kill` today), so the ticket points instead.
- **Run the checks before green (strict reading of the old P5 order).** Evidence produced from a
  pre-green tree proves a test can fail, not that the finished code's guard is live; and it is what
  put a red index under an uncommitted green tree in `t-064`.

### Decision 4 - forward-binding migration (SDC-004)

Existing archived history - two-landing tickets, gap records with pre-green kills, issue bundles -
stays byte-identical. Nothing is re-run or re-stamped: the archive-preservation oracle demands byte
identity, and a re-stamp across 104 records would manufacture work with no new safety. A reviewer
reading older history is told, in the authority, to expect pre-green kills there.

### Decision 5 - automation routed, not built (SDC-005 duplicate)

Machine enforcement - an Exit Guard refusing a dirty tree before any damage step, gates checking the
rule instead of trusting claims - belongs to the cycle that will run this repository as a runbook.
That is the deferred cycle-as-runbook issue: its first ask
([i-015 `PCR-001`](../../deferred/i-015-cycle-as-runbook/spec.md#requirement-records)) now names the
dirty-tree refusal and the intake gate's working-authority acceptance. Building a bespoke hook now
would automate a manual cycle already scheduled for replacement - built twice, thrown away once.

### Integration path

This bundle integrates into the working authority and its satellites only; no goal file changes:

- `.arca/schema.md` - the new "Deliberate damage and discard safety" section (`SDC-001`..`SDC-004`
  headings), the P1 working-authority branch, the P5 fixed order, the Units exception paragraph, and
  the Evidence-receipts alignment sentence. Landed with this pass.
- `.arca/dict.md` - four entries: Deliberate-damage check, Discard command, Checkpoint (safety
  commit), Park. Landed with this pass.
- `.arca/tpl/residual.md` and `.arca/tpl/ticket.md` - the `mutation-kill` field and the pointer-only
  comment. Landed with this pass.
- `.arca/issue/deferred/i-015-cycle-as-runbook/spec.md` - the `PCR-001` carrier extension. Landed
  with this pass.
- `tools/check_links.py` - the working-authority branch, landed with this pass: an accepted ask
  resolves to a requirement-ID heading in `.arca/schema.md` when no goal spec row carries it, and an
  ask resolving to neither fails by name. Differential evidence: pre-edit, this bundle's four
  accepted `SDC` IDs were the tool's only failures; post-edit, green with zero dangling links; a
  nowhere-resolving probe row failed by name and was removed byte-cleanly.

The expectation is written once, in the new working rule, not stamped into old records.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted
forward authority.
