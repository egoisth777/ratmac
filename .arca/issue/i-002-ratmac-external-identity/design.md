# Issue design

## Proposed mechanics

This is an incoming change proposal; these mechanics carry no authority until the issue is integrated into the frozen goal.

1. **Preflight without mutation.** Record the current commit, branch/worktrees, clean status, exact `origin`, checkout path, and active-reference inventory. Confirm `gh` authentication and rename permission, that `egoisth777/ratmac` is available or is the expected rename target, and that `E:/repos/projs/skill-dev/ratmac` does not collide with another checkout or process. Stop before mutation if any check fails.
2. **Prepare and gate tracked references.** Classify old-slug hits as active, generated, `.git` metadata, append-only, or archived. Update only active links, badges, and repository metadata to the new slug/canonical origin; preserve `.arca/log.md` and archived issue/ticket bytes. Run the existing project gates before the external cutover and record the checkpoint and rollback information.
3. **Perform the external cutover in order.** Rename the GitHub repository through an authenticated GitHub API/`gh` operation, verify `egoisth777/ratmac`, update the local `.git/config` `origin` to the exact canonical SSH URL, verify fetch/push identity without pushing, then move the checkout to `E:/repos/projs/skill-dev/ratmac` only after all processes release the old path. Reopen from the new path and verify its Git top-level and basename.
4. **Verify and recover safely.** Run the API/`gh` checks, remote/path checks, `.git` inspection, active-reference audit, clean-state check, and every existing project gate again. If a checkpoint fails, do not force-push, delete history, or leave two competing identities: restore the captured old GitHub slug through the authenticated API, restore the old `origin`, move the checkout back when safe, and revert only the unpushed active-reference change through a reviewable Git operation. Preserve `.arca/log.md`, archives, commits, and working data throughout; record any recovery outcome.

## Explicit exclusions

- No implementation, repository rename, filesystem move, `.git` mutation, source or documentation edit, push, deploy, or issue integration is performed by this incoming issue artifact.
- No change to Rust behavior, Machine/Run/Phase/Status semantics, command compatibility, lock handling, persisted data, or the internal `ratmac`/`rtm` rebrand is proposed.
- No rewriting of append-only `.arca/log.md`, archived issue/ticket records, or ignored build output is allowed; old external identity may remain only in the documented historical allowlist and required migration records.
