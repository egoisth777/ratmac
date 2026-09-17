# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Turn housekeeping (`THK`) | This issue's stable requirement-ID prefix: mechanizing one work item turn's open and close choreography — the steps the working rules fix in prose today — into commands that cannot skip them. |
| Item turn | One build turn over one work item: a linked worktree on a branch named after the item, opened at turn start and closed at green. The 2026-08-21 sprint's t-102 through t-105 were four item turns run by hand. |
| Carried lanes | The untracked lanes root that travels with a turn — copied into the worktree at open (build output skipped), authored there, copied back to the primary checkout before any removal. In this repository `test-hidden/`. Because it is gitignored and committed nowhere, a worktree holding it holds its only copy. |
| Only-copy refusal | The guard that refuses a destructive removal while the worktree holds an artifact that exists nowhere else, naming the artifact and the copy-back that would release it. No flag bypasses it; a named declaration of obsolescence clears it. |
| Trunk | The primary checkout's main branch, which a green turn merges into. `main` here; declared runbook data, not a hardcoded name. |
