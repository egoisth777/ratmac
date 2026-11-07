# Safe reversible trial-worktree lifecycle

```yaml
issue-id: "i-010-trial-worktree-lifecycle"
provenance: "User request, 2026-07-24: provide a simple, controllable, Git-reversible automation for repeated experiment trials on the freshly renamed experiment base exp/ratmac-deterministic (the request's single exp/redmac-deterministic spelling is normalized to that explicit renamed base), with each trial on a disposable worktree branch, a durable advisor-authored log archived to the base, and fixes flowing main-first. Corroborated by the observed failed deterministic branch trial already recorded as provenance of i-009-operable-run-start: the abandoned, uncommitted experiment branch (formerly feat/ratmac-rombook-test at baseline e68bc51) whose work was discarded with its untracked artifacts, leaving no durable log, no preserved terminal commit, and no safe documented teardown. At issue creation the checkout sits on exp/ratmac-deterministic at e68bc51 with the twenty pending i-006..i-009 files staged and no .arca/state.toml; this issue is created without committing or staging anything."
status: "integrated"
```

## Summary

Repeated experiments on the experiment base `exp/ratmac-deterministic` currently have no lifecycle: the one observed deterministic branch trial died as an abandoned, uncommitted branch whose evidence was discarded, and nothing constrains how the next trial starts, where it lives, what survives it, or how its workspace is torn down on Windows without force flags or guessed process kills. This issue asks for a minimal, repo-local, Git-reversible trial-worktree lifecycle: a trial starts only from a clean committed base tip under a deterministic identity (`trial-<nnn>-<topic-slug>` branch in a deterministic sibling linked worktree), start is atomic or fully rolled back, the Advisor authors a structured `trial-log.md` whose durable copy `trials/<trial-branch>/trial-log.md` is the only trial content ever committed to the base, and finish tags the terminal trial commit immutably before removing the worktree and deleting the branch — refusing safely on dirty state, invalid logs, or Windows directory locks. Status/dry-run previews every planned mutation with recovery commands; fixes flow main-first with merge-only sync and visible conflicts; trial implementation is never merged into `main` or the base; ownership (human or Main-Agent runs lifecycle mutations, Advisor writes the log, Subagents touch neither) and the Windows working-directory constraint are explicit.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Integration

Folded into the goal on 2026-07-24 (P1). Every requirement disposition in [spec.md](spec.md) was confirmed `accepted`.

| Goal artifact | What it now carries |
| :--- | :--- |
| [Goal specification](../../../current/spec.md) | Requirement records `TWL-001`–`TWL-010` |
| [Goal design](../../../current/design.md) | The accepted mechanics for this issue |
| [Goal test list](../../../current/test-list.md) | Checks `TWLV-002`–`TWLV-012` |
| [Goal ubiquitous language](../../../current/ubi-lang.md) | This issue's terms |
| [Goal index](../../../current/index.md) | Reverse link to this issue |
