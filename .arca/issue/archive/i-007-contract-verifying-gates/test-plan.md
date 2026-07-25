# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `PGEV-001-issue-shape` | `PGE-001`–`PGE-007` | The pending issue contains exactly the five required populated files with matching identity and provenance, resolved relative routes, and no template markers. |
| `PGEV-002-intake-contract` | `PGE-001` | QA fixture via `cargo test -p ratmac-qa` (suite names assigned at P4): a correctly integrated fixture batch passes the intake gate; negative fixtures — status `integrated` with an accepted requirement ID absent from the goal, and a dangling reverse link — are refused naming the offending artifact. |
| `PGEV-003-record-contract` | `PGE-002` | Positive fixture with complete residual and ticket records passes; negatives — `satisfied` residual without evidence refs, a gap owned by zero or two tickets, a dependency cycle, a ticket missing its hidden-lane assessments — each refuse naming the specific record. |
| `PGEV-004-sensitivity-receipts` | `PGE-003` | A fixture whose planned test exists and carries a baseline-failure receipt passes; the identical fixture with the receipt removed and replaced by a log line containing the ticket id plus expected-red wording is refused identifying the receiptless planned test. |
| `PGEV-005-ownership-audit` | `PGE-004` | An executable audit over the active Runbook prompts and gate contracts finds no instruction to write `.arca/state.toml`, `.arca/log.md`, or `.arca/rtm.lock`; a deliberately violating fixture prompt makes the audit fail (negative), proving the audit is sensitive. |
| `PGEV-006-completion-verifies` | `PGE-005` | A fixture ticket whose declared focused, regression, hidden-lane, and quality commands run green (or carry fresh matching receipts) passes the completion gate; the negative fixture that only relabels ticket `passed` and residuals `satisfied` with no receipts is refused naming the first missing receipt. |
| `PGEV-007-blocked-route` | `PGE-006` | In a fixture Run, human-authorized `held` plus a linked blocker record routes the Run onward with ticket not-passed and residuals unproven; without authorization or without the blocker link the request is refused; Scheduler-owned files are byte-identical across refusals. |
| `PGEV-008-no-vacuous-pass` | `PGE-001`–`PGE-007` | Gap analysis on a tree with no mechanized contract gates classifies these requirements `missing`; no check may report them satisfied by absence of a loop. |
| `PGEV-009-run-abandonment` | `PGE-007` | Positive: in a fixture Run holding a broken active Run, an explicitly human-authorized abandonment makes RTM record a terminal abandoned event or equivalent evidence and retire the admission state and lock, after which a fresh Run start in the same fixture succeeds. Negative: the identical request without human authorization is refused atomically, with `.arca/state.toml`, `.arca/log.md`, and `.arca/rtm.lock` byte-identical across the refusal. Recovery: with a stale lock present, the authorized path retires the lock through the Engine — no bypass exists and no agent deletes or edits a Scheduler-owned file at any point. |

All checks run through the QA harness in isolated fixture projects; none require commit, push, deployment, network access, or global installation.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/current/index.md` | updated | Link `i-007-contract-verifying-gates` and summarize contract-verifying gates plus the blocked route and Run abandonment. |
| `.arca/current/ubi-lang.md` | updated | Define contract gate, evidence receipt, sensitivity receipt, agent-writable evidence artifact, blocked route, blocker record, and Run abandonment. |
| `.arca/current/spec.md` | updated | Integrate `PGE-001`–`PGE-007` with stable requirement IDs. |
| `.arca/current/design.md` | updated | Record the accepted gate, receipt, ownership, and blocked-route mechanics. |
| `.arca/current/test-list.md` | updated | Add `PGEV-002`–`PGEV-009`, including every negative refusal and abandonment case. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | updated | Point agents at the agent-writable evidence location and the human-only `held` and Run-abandonment authorizations; the never-edit-Scheduler-files rule is unchanged and becomes obeyable in P4. |
| `.arca/index.md` | updated | Record the evidence-artifact ownership split (agent-writable receipts versus Scheduler-owned log), ticket `held` linkage to a blocker record, and the receipt obligation behind `satisfied` and `passed`. |
| `.arca/tpl/ticket.md` | updated | Add the blocker-link field consumed by the blocked route. |
| `.arca/state.toml`, `.arca/log.md`, `.arca/rtm.lock` | unaffected | Issue creation mutates no Scheduler-owned runtime artifact; later implementation — including the abandonment route — writes them only through `rtm`. |
