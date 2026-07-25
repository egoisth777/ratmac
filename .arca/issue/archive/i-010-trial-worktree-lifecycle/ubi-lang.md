# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Experiment base | The long-lived local branch `exp/ratmac-deterministic` (the renamed experiment branch): every trial starts from its clean committed tip; a trial finish adds only its durable log, while verified product fixes arrive from local `main` only through explicit merge/sync. |
| Trial | One bounded, numbered experiment attempt with a hypothesis, executed entirely on its own trial branch inside its own trial worktree, and concluded by a finish that preserves its log and terminal commit. |
| Trial branch | The branch `trial-<nnn>-<topic-slug>` created at the experiment base tip when a trial starts; it never merges into `main` or the experiment base. |
| Trial number | `<nnn>`: a positive integer zero-padded to at least three digits; inferred by default as one greater than the highest number occupied by any live trial branch, trial archive tag, or durable log directory, so numbers are never reused; an explicit override is collision-checked. |
| Topic slug | The short lowercase dashed name in the trial branch, matching `[a-z0-9]+(-[a-z0-9]+)*`; anything else is refused before mutation. |
| Trial worktree | The linked Git worktree of the trial branch, at a sibling directory of the repository root derived deterministically from the repository basename and the trial branch name. |
| Trial log | The Advisor-authored structured `trial-log.md`, committed on the trial branch, covering trial identity, base and terminal trial commits, hypothesis, procedure, commands and tests run, observations, verdict, recommendations, and artifact/diff references. |
| Durable log destination | `trials/<trial-branch>/trial-log.md` committed on the experiment base at finish — the only trial content that outlives the trial. |
| Trial archive tag | The immutable annotated tag, deterministically named from the trial branch, created at the terminal trial commit and verified before any deletion; never moved or deleted by the lifecycle, it makes branch deletion reversible. |
| Terminal trial commit | The trial branch tip at finish time — the commit the trial archive tag must preserve. |
| Trial lifecycle interface | The single documented repo-local entry point offering exactly trial start, trial status/dry-run, trial finish, and base sync; offline, push-free, and free of global installation or PATH/global-config mutation. |
| Dry-run preview | Read-only status output naming repository facts, live and archived trials, the next inferred trial identity, and — per mutating operation — the exact planned mutations and their recovery commands. |
| Recovery commands | The exact Git commands, printed by status/dry-run and finish, that restore a deleted trial branch from its archive tag or re-add its worktree, or resume an interrupted finish. |
| Advisor | The reviewer agent that authors trial-log content (hypothesis, procedure, observations, verdict, recommendations) and never invokes a lifecycle mutation or any Git write. |
| Windows directory lock | An open handle on the trial worktree directory (for example a shell whose working directory is inside it) that blocks removal; it grounds a safe named refusal with guidance — never a forced removal, never a guessed process kill. |
| Main-first fix flow | The policy that defects exposed by trials are fixed on local `main` through its ordinary nondeterministic development loop, then received by a clean experiment base only via explicit merge/sync — never reset, rebase, or force — with conflicts left visible and unresolved. |
