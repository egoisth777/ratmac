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
| R-007 | `rtm start` is user-only; loop entry is never agent-initiated. | ADR-0002 |
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
| RAT-001 | Active SSOT uses `ratmac` and `rtm`, while `ratmac.toml` and Machine Class terminology remain canonical. | [issue RAT-001](../issue/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-002 | Rust package, crate, library, dependency, import, and binary surfaces become `ratmac`/`rtm`. | [issue RAT-002](../issue/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-003 | Every active user-facing command route and diagnostic uses `rtm`; the legacy `schd` spelling is not advertised. | [issue RAT-003](../issue/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-004 | Tests, fixtures, and QA exercise canonical names without changing scheduler semantics. | [issue RAT-004](../issue/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-005 | Active references are inventoried and updated; append-only logs and archived tickets remain historical allowlist entries. | [issue RAT-005](../issue/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-006 | Cargo and checked-in generated assets are regenerated by their owning tools. | [issue RAT-006](../issue/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-007 | Clean cutover excludes a `schd` alias; persistent data stays, and legacy lock handling is explicit and safe. | [issue RAT-007](../issue/i-001-ratmac-rebrand/spec.md#requirement-records) |
| RAT-008 | Acceptance includes stale-name audit, metadata checks, full tests, quality gates, and `rtm` smoke runs. | [issue RAT-008](../issue/i-001-ratmac-rebrand/spec.md#requirement-records) |

## Integrated external identity requirements

| Req ID | Requirement | Source |
|---|---|---|
| EXT-001 | The external GitHub repository identity changes from `egoisth777/arca-scheduler` to `egoisth777/ratmac`; acceptance inspects the real GitHub identity and `.git` metadata, not tracked labels alone. | [issue EXT-001](../issue/i-002-ratmac-external-identity/spec.md#requirement-records) |
| EXT-002 | Canonical `origin` is exactly `git@github.com:egoisth777/ratmac.git`, and the checkout moves from `E:/repos/projs/skill-dev/arca-scheduler` to `E:/repos/projs/skill-dev/ratmac`; acceptance verifies remote and actual path/basename. | [issue EXT-002](../issue/i-002-ratmac-external-identity/spec.md#requirement-records) |
| EXT-003 | Every active link, badge, repository URL, owner/slug reference, and checked-in repository metadata changes to `egoisth777/ratmac` or the canonical origin; `.arca/log.md` and archived issue/ticket records remain byte-for-byte unchanged in the explicit historical allowlist. | [issue EXT-003](../issue/i-002-ratmac-external-identity/spec.md#requirement-records) |
| EXT-004 | Before mutation, collision, authentication, authorization, remote availability, path, process, branch/worktree, and clean-tree preflight checks are recorded; the cutover is ordered with checkpoints and a reversible rollback that preserves work and Git arbitration. | [issue EXT-004](../issue/i-002-ratmac-external-identity/spec.md#requirement-records) |
| EXT-005 | Final acceptance directly verifies GitHub API and `gh repo view`, exact origin, checkout top-level/basename, `.git` identity, active references plus historical allowlist, clean state, and every existing behavior/rebrand/project gate. | [issue EXT-005](../issue/i-002-ratmac-external-identity/spec.md#requirement-records) |
| EXT-006 | This issue's planning pass performs no GitHub rename, filesystem move, origin mutation, source/documentation implementation, push, deploy, or issue integration beyond recording the integrated plan. | [issue EXT-006](../issue/i-002-ratmac-external-identity/spec.md#requirement-records) |
