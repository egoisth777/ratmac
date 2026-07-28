# ratmac specification

Requirement records distilled from the accepted decisions. Sources cite the decision anchors in [design.md](design.md).

| Req ID | Requirement | Source |
|---|---|---|
| R-001 | Machine state is the Phase, nothing else; Phase is the only dimension of machine state. | ADR-0001 |
| R-002 | `status` is a phase-local lifecycle enum (`planned\|executing\|blocked\|passed\|failed`) recorded by the Scheduler; it is not part of the Machine graph and transitions never branch on it in the definition. | ADR-0001 |
| R-003 | Machine Class files declare Phases and transitions only — no status dimension. | ADR-0001 |
| R-004 | `blocked` is a Status value only, never a machine state; it means missing entry prerequisites and is set only by the Scheduler. | ADR-0001, ADR-0006 |
| R-005 | An agent may request a transition (`rtm step`) when it believes the Phase is done; Exit Guards accept or refuse deterministically. | ADR-0002 |
| R-006 | Exit Guards check artifacts (filesystem shape, file content, command exit code), never agent claims. | ADR-0002 |
| R-007 | Loop entry is never self-initiated by an agent: a human may invoke argument-free `rtm start`, and the Main-Agent only after explicit human Run-start sign-off. The former user-only wording is superseded by ORS-001. | ADR-0002, ORS-001 |
| R-008 | Only the Main-Agent or the human invokes `rtm step`; Subagents read state and never invoke `rtm`. | ADR-0003 |
| R-009 | The Scheduler is the sole writer of State Files; no agent edits a State File directly. | ADR-0003, ADR-0008 |
| R-010 | The Machine Class file is TOML, named `ratmac.toml`; human-written and reviewed, never agent-authored. | ADR-0004 |
| R-011 | Parsing is strict: unknown keys in `ratmac.toml` are hard errors. | ADR-0004 |
| R-012 | Comments in `ratmac.toml` hold phase intent notes and never reach agents. | ADR-0004 |
| R-013 | `ratmac.toml` is a Machine Class: a pure template with no runtime state, read-only at runtime. | ADR-0005 |
| R-014 | `rtm start` instantiates a Run from the class; each Run owns its State File, Transition Log, and lockfile. | ADR-0005 |
| R-015 | The Scheduler arbitrates concurrent access per Run via the lockfile (`.arca/rtm.lock`). | ADR-0005, ADR-0008 |
| R-016 | The engine holds zero project knowledge; wishwillow's P1–P5 loop is merely the first Machine Class. | ADR-0005 |
| R-017 | A failing Exit Guard makes `rtm step` refuse, report, and stay: Phase unchanged, Status unchanged, no counter, no log entry beyond the refusal report. | ADR-0006 |
| R-018 | Exit-guard failure never sets `blocked`. | ADR-0006 |
| R-019 | The refusal report names the failing guard and states observed vs expected fact. | ADR-0006 |
| R-020 | `rtm step` is idempotent under failure — safe to re-run any number of times. | ADR-0006 |
| R-021 | The data model allows N Runs; nothing in formats or engine assumes a singleton. | ADR-0007 |
| R-022 | v1 CLI allows at most one active Run per project; `rtm start` refuses while a Run is active. | ADR-0007 |
| R-023 | `rtm step` and `rtm status` take no run-id in v1; they target the active Run. | ADR-0007 |
| R-024 | Scheduler-owned files sit flat under `.arca/`, no folder: `ratmac.toml`, `state.toml`, `log.md`, `rtm.lock`. | ADR-0008 |
| R-025 | `.arca/state.toml` is the State File with fields `phase`, `status`, `goal_revision`, `input_revision`, `output_revision`, `active_refs`, `blocker`; written ONLY by the Scheduler. | ADR-0008 |
| R-026 | `.arca/log.md` is the append-only, human-readable Transition Log. | ADR-0008 |
| R-027 | State parse errors are hard errors: a corrupt `state.toml` halts `rtm` with a report, never a guess. | ADR-0008 |
| R-028 | Each `[phases.X]` in `ratmac.toml` carries a `prompt` field of short prose; the Scheduler renders the Phase Prompt as that prose plus a mechanically generated list of the Phase's Exit Guards. | ADR-0009 |
| R-029 | The Phase Prompt is the only machine information an agent ever receives — never the flowchart, never other Phases. | ADR-0009 |
| R-030 | v1 is print-first: `rtm step` (on a successful transition) and `rtm status` print the current Phase Prompt to stdout; no process management, no spawn flag. | ADR-0010 |

## Integrated rebrand requirements

| Req ID | Requirement | Source |
|---|---|---|
| RAT-001 | Active SSOT uses `ratmac` and `rtm`, while `ratmac.toml` and Machine Class terminology remain canonical. | [issue RAT-001](../issue/archive/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-002 | Rust package, crate, library, dependency, import, and binary surfaces become `ratmac`/`rtm`. | [issue RAT-002](../issue/archive/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-003 | Every active user-facing command route and diagnostic uses `rtm`; the legacy `schd` spelling is not advertised. | [issue RAT-003](../issue/archive/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-004 | Tests, fixtures, and QA exercise canonical names without changing scheduler semantics. | [issue RAT-004](../issue/archive/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-005 | Active references are inventoried and updated; append-only logs and archived tickets remain historical allowlist entries. | [issue RAT-005](../issue/archive/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-006 | Cargo and checked-in generated assets are regenerated by their owning tools. | [issue RAT-006](../issue/archive/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-007 | Clean cutover excludes a `schd` alias; persistent data stays, and legacy lock handling is explicit and safe. | [issue RAT-007](../issue/archive/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-008 | Acceptance includes stale-name audit, metadata checks, full tests, quality gates, and `rtm` smoke runs. | [issue RAT-008](../issue/archive/i-001-ratmac-rebrand/spec.md#requirement-records) |

## Integrated external identity requirements

| Req ID | Requirement | Source |
|---|---|---|
| EXT-001 | The external GitHub repository identity changes from `egoisth777/arca-scheduler` to `egoisth777/ratmac`; acceptance inspects the real GitHub identity and `.git` metadata, not tracked labels alone. | [issue EXT-001](../issue/archive/i-002-ratmac-external-identity/spec.md#requirement-records) |
| EXT-002 | Canonical `origin` is exactly `git@github.com:egoisth777/ratmac.git`, and the checkout moves from `E:/repos/projs/skill-dev/arca-scheduler` to `E:/repos/projs/skill-dev/ratmac`; acceptance verifies remote and actual path/basename. | [issue EXT-002](../issue/archive/i-002-ratmac-external-identity/spec.md#requirement-records) |
| EXT-003 | Every active link, badge, repository URL, owner/slug reference, and checked-in repository metadata changes to `egoisth777/ratmac` or the canonical origin; `.arca/log.md` and archived issue/ticket records remain byte-for-byte unchanged in the explicit historical allowlist. | [issue EXT-003](../issue/archive/i-002-ratmac-external-identity/spec.md#requirement-records) |
| EXT-004 | Before mutation, collision, authentication, authorization, remote availability, path, process, branch/worktree, and clean-tree preflight checks are recorded; the cutover is ordered with checkpoints and a reversible rollback that preserves work and Git arbitration. | [issue EXT-004](../issue/archive/i-002-ratmac-external-identity/spec.md#requirement-records) |
| EXT-005 | Final acceptance directly verifies GitHub API and `gh repo view`, exact origin, checkout top-level/basename, `.git` identity, active references plus historical allowlist, clean state, and every existing behavior/rebrand/project gate. | [issue EXT-005](../issue/archive/i-002-ratmac-external-identity/spec.md#requirement-records) |
| EXT-006 | This issue's planning pass performs no GitHub rename, filesystem move, origin mutation, source/documentation implementation, push, deploy, or issue integration beyond recording the integrated plan. | [issue EXT-006](../issue/archive/i-002-ratmac-external-identity/spec.md#requirement-records) |

## Integrated Engine trust boundary requirements

| Req ID | Requirement | Source |
|---|---|---|
| ETB-001 | Guard evaluation must not compile, fetch, or rebuild project source at evaluation time. A guard command running project-derived logic must run a pinned gate artifact whose resolved path and content hash are recorded in Run evidence no later than first guard use; evaluation against a differing observed hash is refused naming observed and expected identity. Commands reading no project state (toolchain probes) are exempt but must be identifiable as such. | [issue ETB-001](../issue/archive/i-006-engine-trust-boundary/spec.md#requirement-records) |
| ETB-002 | A failed command guard's refusal must carry, besides program identity and expected-vs-observed exit facts, a bounded diagnostic captured from the command's stderr (or a declared structured channel). The bound is deterministic and documented; overflow is truncated with an explicit marker; a silent command yields an explicit no-diagnostic statement rather than an omitted field. | [issue ETB-002](../issue/archive/i-006-engine-trust-boundary/spec.md#requirement-records) |
| ETB-003 | The goal revision cited by gap analysis is frozen at the intake-completion boundary (P1→P2), not at Run start. Run evidence records the start baseline revision and the frozen goal revision as distinct fields. After the freeze and until batch closure, a change to `.arca/goal/` is detected at the next transition request and refused as goal drift naming frozen and observed revisions. | [issue ETB-003](../issue/archive/i-006-engine-trust-boundary/spec.md#requirement-records) |

## Integrated contract-verifying gate requirements

| Req ID | Requirement | Source |
|---|---|---|
| PGE-001 | The intake-completion gate verifies integration mechanically: every direct issue folder ends `integrated` or `rejected` with the five-file shape intact, every accepted requirement ID stated by an integrated issue exists in the goal authority, and forward/reverse links resolve. A status claim without the requirement in the goal, or a dangling link, refuses naming the offending artifact. | [issue PGE-001](../issue/archive/i-007-contract-verifying-gates/spec.md#requirement-records) |
| PGE-002 | The gap-analysis and planning gates validate record contracts: exactly one residual per frozen-batch requirement citing the frozen goal revision; `satisfied` only with concrete evidence references; every `missing`/`partial` residual owned by exactly one ticket; acyclic ticket dependencies; each ticket carrying its required planning sections. Violations refuse naming the specific record. | [issue PGE-002](../issue/archive/i-007-contract-verifying-gates/spec.md#requirement-records) |
| PGE-003 | Test-first completion is evidenced by executable sensitivity receipts: the P4 gate accepts only structured per-planned-test receipts proving the test exists as a runnable test and produced a recorded baseline failure or controlled mutation kill. No free-text log line, filename convention, or status field satisfies the gate; a planned test without receipt refuses naming that test. | [issue PGE-003](../issue/archive/i-007-contract-verifying-gates/spec.md#requirement-records) |
| PGE-004 | Phase Prompts and gate contracts never require an agent to write a Scheduler-owned artifact. Agent-authored evidence lives in agent-writable evidence artifacts or is submitted through an explicit `rtm` command that performs the Scheduler-owned append itself. The active prompt set and gate predicates are auditable for this property. | [issue PGE-004](../issue/archive/i-007-contract-verifying-gates/spec.md#requirement-records) |
| PGE-005 | The implementation-completion gate runs, or verifies receipts for, the executing ticket's focused tests, affected regressions, applicable hidden lanes, and required quality gates before the ticket may be `passed` and its residuals `satisfied`. Receiptless relabeling refuses naming the first missing receipt; receipts identify commands, targets, and results so an audit can re-derive the claim. | [issue PGE-005](../issue/archive/i-007-contract-verifying-gates/spec.md#requirement-records) |
| PGE-006 | The build loop includes an honest, human-authorized blocked route: an executing ticket blocked for an out-of-scope reason may be `held` by a human with a linked blocker record (a new five-file issue or named residual), after which the Run may route to intake or planning while the ticket stays not-passed and its residuals unproven. Without human authorization the route refuses, leaving Scheduler-owned state and history unchanged. | [issue PGE-006](../issue/archive/i-007-contract-verifying-gates/spec.md#requirement-records) |
| PGE-007 | Run abandonment is human-authorized and safe: on authorization RTM itself records a terminal abandoned event and retires the active Run's admission state including the lock, so a fresh Run can start. Agents never delete or edit `.arca/state.toml`, `.arca/log.md`, or `.arca/rtm.lock`, and no stale lock is bypassed rather than retired. An unauthorized request refuses atomically with those files byte-identical. | [issue PGE-007](../issue/archive/i-007-contract-verifying-gates/spec.md#requirement-records) |

## Integrated acceptance-oracle integrity requirements

| Req ID | Requirement | Source |
|---|---|---|
| AOI-001 | Acceptance and merge-gate evidence must be reproducible from a reviewable snapshot: recording evidence enumerates the declared evidence roots and refuses (or explicitly enumerates as exceptions) untracked or unstaged content there, and carries a snapshot manifest of per-path tracking state plus content digest sufficient to re-derive what was tested. Evidence over undeclared untracked content is invalid and mechanically detectable. | [issue AOI-001](../issue/archive/i-008-honest-acceptance-oracles/spec.md#requirement-records) |
| AOI-002 | The contributor schema authorizes completed-issue archive moves to `.arca/issue/archive/<issue-id>/` preserving identity, five-file shape, and content except required relative-link updates, with `i-<nnn>` uniqueness spanning active and archived issues. Every history-preservation oracle treats a complete authorized move as preservation by comparing content at the archived destination, while still failing on content mutation, partial moves, or moves of non-completed issues. | [issue AOI-002](../issue/archive/i-008-honest-acceptance-oracles/spec.md#requirement-records) |
| AOI-003 | Environment-coupled release acceptance (live GitHub identity, exact origin, branch, clean worktree) must not gate the default suite: plain `cargo test --workspace` passes on a feature branch with in-progress contributor artifacts present, and the release acceptance lane is visibly reported as skipped rather than silently absent. The lane runs only under an explicit documented opt-in and then fails with its own diagnostics when unsatisfied. | [issue AOI-003](../issue/archive/i-008-honest-acceptance-oracles/spec.md#requirement-records) |

## Integrated operable Run-start requirements

| Req ID | Requirement | Source |
|---|---|---|
| ORS-001 | Caller policy: a human may invoke argument-free `rtm start` directly; the Main-Agent may invoke it only after explicit human Run-start sign-off for the current target project; a Subagent never invokes any `rtm` command. Every active caller-facing surface states this one policy and an executable audit finds no active user-only or blanket never-agent-start wording. The Engine gains no caller identity, authentication, or authorization state. Supersedes the former user-only rule in R-007. | [issue ORS-001](../issue/archive/i-009-operable-run-start/spec.md#requirement-records) |
| ORS-002 | A deterministic project-local bootstrap exists: one documented command, run from the project root, that locates or builds the Stable Engine binary, verifies it against the recorded pin when present, and reports resolved path and identity — with no global installation, PATH mutation, or network fetch. A read-only doctor reports Engine identity, Runbook presence/validity, and state-file presence/phase, and with no active Run names the next legitimate action while distinguishing `.arca/ratmac.toml` from `.arca/state.toml`. The doctor writes nothing. | [issue ORS-002](../issue/archive/i-009-operable-run-start/spec.md#requirement-records) |
| ORS-003 | Caller-policy verification is honest about evidence kind: behavioral claims about invocation may be recorded as proven only by behavioral evidence from role scenarios recording attempted commands or tool calls; wording-only checks are recorded as guidance-consistency evidence and cannot satisfy a behavioral requirement. The behavioral harness is sensitive: a scenario in which an unauthorized caller invokes `rtm` fails the check. | [issue ORS-003](../issue/archive/i-009-operable-run-start/spec.md#requirement-records) |

## Integrated trial-worktree lifecycle requirements

| Req ID | Requirement | Source |
|---|---|---|
| TWL-001 | A trial starts only from the experiment base `exp/ratmac-deterministic` at a clean committed tip (HEAD equals the base ref; no staged, unstaged, or untracked changes). Any dirty form, or a colliding trial branch, registered worktree, sibling directory, archive tag, or durable log destination, refuses with a named reason and zero mutation of refs, index, working tree, tags, and worktree registrations. | [issue TWL-001](../issue/archive/i-010-trial-worktree-lifecycle/spec.md#requirement-records) |
| TWL-002 | Trial identity is deterministic and computed before mutation: branch `trial-<nnn>-<topic-slug>` with `<nnn>` zero-padded to at least three digits and slug matching `[a-z0-9]+(-[a-z0-9]+)*`; the linked worktree is a sibling directory derived from repository basename plus trial branch. The default number is one greater than the highest occupied by any live trial branch, archive tag, or durable log directory; an explicit number is accepted only after the same collision check. | [issue TWL-002](../issue/archive/i-010-trial-worktree-lifecycle/spec.md#requirement-records) |
| TWL-003 | Trial start is observably atomic: on success the trial branch exists at exactly the base tip with its worktree registered at the derived path, both reported; on any failure no new branch ref, tag, worktree registration, or sibling directory persists, and if rollback itself fails the interface prints exact manual recovery commands while mutating nothing further. | [issue TWL-003](../issue/archive/i-010-trial-worktree-lifecycle/spec.md#requirement-records) |
| TWL-004 | The Advisor authors a structured `trial-log.md` committed on the trial branch covering identity, base and terminal commits, hypothesis, procedure, commands and tests, observations, verdict, recommendations, and artifact/diff references. Validity is mechanically checkable — every required section present and non-empty, identity facts consistent with the actual branch and commits. Its durable destination is `trials/<trial-branch>/trial-log.md` committed on the experiment base: the only trial content that outlives the trial. | [issue TWL-004](../issue/archive/i-010-trial-worktree-lifecycle/spec.md#requirement-records) |
| TWL-005 | Finish runs from the base checkout (clean experiment base; invoking working directory outside the trial worktree) against a clean trial worktree with a valid committed log, in fixed observable order: (1) create and verify the annotated archive tag at the terminal trial commit before any deletion; (2) commit the durable log alone on the base; (3) remove the linked worktree; (4) delete the trial branch only after tag verification confirms the terminal commit is preserved. A failing step refuses it and all later steps with a named reason. | [issue TWL-005](../issue/archive/i-010-trial-worktree-lifecycle/spec.md#requirement-records) |
| TWL-006 | A status/dry-run operation reports without mutation: experiment base and tip, cleanliness, live and archived trials, the next inferred identity (branch, worktree path, archive tag, durable destination), and for each mutating operation the exact planned mutations plus recovery commands. Read-only-ness is observable: refs, index, working tree, tags, and worktree registrations are byte-identical before and after. | [issue TWL-006](../issue/archive/i-010-trial-worktree-lifecycle/spec.md#requirement-records) |
| TWL-007 | Fixes flow main-first: defects exposed by a trial are fixed on local `main` through its ordinary loop, never authored on the experiment base, which receives `main` only through an explicit merge/sync started from a clean base checkout. The interface offers no reset, rebase, or force variant; a conflicting merge leaves the conflicted state visible with no automatic resolution and no automatic abort. | [issue TWL-007](../issue/archive/i-010-trial-worktree-lifecycle/spec.md#requirement-records) |
| TWL-008 | Containment and reversibility: no lifecycle operation merges trial implementation content into `main` or the experiment base — the durable log is the only file a finish adds — and because the archive tag preserves the terminal commit, deleting branch and worktree is reversible through documented recovery commands printed in dry-run and finish output. | [issue TWL-008](../issue/archive/i-010-trial-worktree-lifecycle/spec.md#requirement-records) |
| TWL-009 | The lifecycle is exposed through one minimal repo-local entry point offering exactly trial start, status/dry-run, finish, and base sync, runnable from the repository root with tools this project already requires. No lifecycle operation pushes, deploys, fetches from the network, installs globally, or mutates PATH or global Git configuration. | [issue TWL-009](../issue/archive/i-010-trial-worktree-lifecycle/spec.md#requirement-records) |
| TWL-010 | Ownership and environment constraints are explicit and enforced where mechanically possible: lifecycle mutations are invoked only by the human or Main-Agent from the primary checkout with the experiment base checked out; the Advisor authors log content only; Subagents invoke neither lifecycle mutations nor `rtm`. Finish refuses when its working directory lies inside the trial worktree, and an OS refusal to remove a locked directory surfaces as a safe named refusal with guidance — never a forced removal, never a guessed process kill. | [issue TWL-010](../issue/archive/i-010-trial-worktree-lifecycle/spec.md#requirement-records) |

## Integrated runbook-specification requirements

| Req ID | Requirement | Source |
|---|---|---|
| RBS-001 | The Machine Class has a prose specification of its own at `.arca/runbook-spec.md`, routed from `.arca/index.md`: file format, top-level shape, phase fields, transition fields, and which keys are required versus optional. Code implements that specification; the specification is never read back out of code. | [issue RBS-001](../issue/archive/i-011-runbook-spec/spec.md#requirement-records) |
| RBS-002 | The specification enumerates the guard-kind vocabulary as a closed list: every kind the Engine accepts, its semantics, its required fields, its forbidden fields, and its exemptions. A kind absent from that list is not a guard. | [issue RBS-002](../issue/archive/i-011-runbook-spec/spec.md#requirement-records) |
| RBS-003 | The specification states runbook ownership: which artifacts are Scheduler-owned, which are agent-writable, and that a prompt or guard contract directing an agent to write a Scheduler-owned artifact is a defect. Every ownership rule names its enforcer or is explicitly marked prose-only. | [issue RBS-003](../issue/archive/i-011-runbook-spec/spec.md#requirement-records) |
| RBS-004 | The specification is the single authority for runbook schema. The parser, the doctor, and the authoring instructions cite it and restate no schema fact; a second definition of a schema term outside it is a defect. | [issue RBS-004](../issue/archive/i-011-runbook-spec/spec.md#requirement-records) |
| RBS-005 | The specification preserves decided behavior and back-references it: R-002/R-003 (no status dimension), R-011 (unknown keys are hard errors), R-028 (per-phase prompt), ETB-001/ETB-002 (pinned command guards and bounded diagnostics), ETB-003 (`freeze = "goal"`), PGE-003/PGE-005 (receipt and completion gate kinds), PGE-006 (`blocked-route`). It formalizes what is; it changes no decided behavior. | [issue RBS-005](../issue/archive/i-011-runbook-spec/spec.md#requirement-records) |

## Integrated typed-parser requirements

| Req ID | Requirement | Source |
|---|---|---|
| TRP-001 | One typed parse: the runbook is deserialized exactly once into a typed Machine Class, and every consumer — Scheduler guard evaluation, prompt rendering, pending-guard listing — reads that typed value. No second TOML walk of the runbook exists in `src/`. | [issue TRP-001](../issue/archive/i-012-typed-runbook-parser/spec.md#requirement-records) |
| TRP-002 | Guard kinds are a closed typed enum. An unknown kind is a parse error naming the kind and its location; it is never a skipped guard, a deferred failure, or a runtime surprise. | [issue TRP-002](../issue/archive/i-012-typed-runbook-parser/spec.md#requirement-records) |
| TRP-003 | Per-kind field validation runs at parse time against the guard-kind table of `RBS-002`: a field a kind requires must be present, and a field foreign to that kind is refused. Each refusal names the kind and the offending field. | [issue TRP-003](../issue/archive/i-012-typed-runbook-parser/spec.md#requirement-records) |
| TRP-004 | Guards are retained through the parse: every guard authored in the runbook is present on the parsed Machine Class, in declaration order, for every kind. | [issue TRP-004](../issue/archive/i-012-typed-runbook-parser/spec.md#requirement-records) |
| TRP-005 | A missing or unreadable runbook is a named refusal surfaced to the caller. No code path substitutes an empty machine for an absent Machine Class. | [issue TRP-005](../issue/archive/i-012-typed-runbook-parser/spec.md#requirement-records) |
| TRP-006 | R-002/R-003/R-011 semantics are preserved exactly, and the default and opt-in lanes stay green: this is a refactor, and decided behavior moves only through an issue. | [issue TRP-006](../issue/archive/i-012-typed-runbook-parser/spec.md#requirement-records) |

## Integrated deep-doctor requirements

| Req ID | Requirement | Source |
|---|---|---|
| DRD-001 | `rtm doctor` diagnoses through the `TRP-001` parser. The doctor module performs no TOML walk of its own, so a file the doctor calls clean is a file the Engine can run. | [issue DRD-001](../issue/archive/i-013-deep-rtm-doctor/spec.md#requirement-records) |
| DRD-002 | Graph checks over the parsed Machine Class report: no phases declared, an ambiguous initial phase, a phase unreachable from the initial phase, a phase with no outgoing transition, a duplicate edge, and a self-loop. A well-formed file can still describe a broken machine. | [issue DRD-002](../issue/archive/i-013-deep-rtm-doctor/spec.md#requirement-records) |
| DRD-003 | Guard lint reports, beyond parseability: a `command_exit` guard that is neither `exempt` nor resolvable to a pinnable regular file, and a guard whose verdict rests on agent-writable content (`files_exact`, `file_contains`). | [issue DRD-003](../issue/archive/i-013-deep-rtm-doctor/spec.md#requirement-records) |
| DRD-004 | `ownership::audit_ownership` (PGE-004) is wired into the doctor over the runbook's prompts and guard contracts; the doctor duplicates no audit logic. | [issue DRD-004](../issue/archive/i-013-deep-rtm-doctor/spec.md#requirement-records) |
| DRD-005 | `rtm doctor <path>` validates an arbitrary runbook file, inside or outside the project, and remains read-only; argument-free `rtm doctor` keeps its `ORS-002` report of Engine identity, runbook validity, and runtime state. | [issue DRD-005](../issue/archive/i-013-deep-rtm-doctor/spec.md#requirement-records) |
| DRD-006 | Findings are machine-readable: one finding carries a stable code, a severity, a location, and a message; `--json` emits the finding list, and the human rendering formats the same list. The code table is documented in the runbook specification and matches the Engine's table. | [issue DRD-006](../issue/archive/i-013-deep-rtm-doctor/spec.md#requirement-records) |
| DRD-007 | Exit codes are differentiated: `0` clean, `1` warnings only, `2` at least one error (a parse refusal is an error). A caller branches on the code, never on the text. | [issue DRD-007](../issue/archive/i-013-deep-rtm-doctor/spec.md#requirement-records) |

## Integrated authoring-loop requirements

| Req ID | Requirement | Source |
|---|---|---|
| AAL-001 | Agent-facing authoring instructions exist at `.arca/runbook-authoring.md`, routed from `.arca/index.md`, carrying procedure only: start from the scaffold, edit, run the doctor, repair by code. Every schema fact in it is a link into the runbook specification; it defines no schema term of its own. | [issue AAL-001](../issue/archive/i-014-agent-authoring-loop/spec.md#requirement-records) |
| AAL-002 | `rtm scaffold <path>` writes a minimal valid runbook at a path that does not yet exist and refuses rather than overwriting one that does. Its output passes `rtm doctor <path>` clean (exit `0`), enforced by test so it stays true. | [issue AAL-002](../issue/archive/i-014-agent-authoring-loop/spec.md#requirement-records) |
| AAL-003 | The write → doctor → repair loop consumes `rtm doctor --json`: the instructions carry one repair row per stable diagnostic code, and that table is checked against the Engine's code table by test, so a repair addresses a named code rather than a guess. | [issue AAL-003](../issue/archive/i-014-agent-authoring-loop/spec.md#requirement-records) |
| AAL-004 | The authoring surface builds on `TRP-*` and `DRD-*` and cites `RBS-*`: a seeded-defect runbook is repaired to doctor-clean using only the scaffold, the `--json` diagnostics, and the instructions — without reading `src/`. | [issue AAL-004](../issue/archive/i-014-agent-authoring-loop/spec.md#requirement-records) |
