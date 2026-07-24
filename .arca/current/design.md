# ratmac design

Decision record. Each section replaces a former ADR and keeps its id as an anchor. All decisions accepted 2026-07-22.

## Machine state is the Phase (ADR-0001)

**Context.** The seed handoff (`ratmac.md`, removed 2026-07-22 after full extraction; never committed — see `.arca/log.md`) listed `blocked` both as a machine state and as a `status` value, and carried `status` (`planned|executing|blocked|passed|failed`) as a second enum beside `phase`. The true state space was ambiguous: `phase` alone, or `phase × status`.

**Decision.** Machine state = Phase, nothing else. `status` is a phase-local lifecycle field the Scheduler records; it is NOT part of the Machine graph and transitions never branch on it in the definition. `blocked` is a `status` value only; it is removed from the state list.

**Consequences.** Machine definition files declare Phases and transitions only — no status dimension. The State File keeps both fields: `phase` (machine position) and `status` (lifecycle inside it). wishwillow's `.arca/index.md` states table must drop `blocked` as a state (doc update owed to wishwillow).

## Agents may request `next`; guards decide (ADR-0002)

**Context.** Core rule: agents never decide transitions. But someone must invoke the transition request. Candidates: human only (friction: human becomes the loop's clock), scheduler self-loop (builds the daemon now, against print-first), or agent-invoked.

**Decision.** Requesting is not deciding. An agent may request a transition (the handoff's `next`, now `rtm step`) when it believes the Phase is done. Exit Guards accept or refuse deterministically; the agent cannot talk its way past a guard because guards check artifacts, not claims. `start` remains user-only: loop entry is never agent-initiated (unchanged from handoff).

**Consequences.** The request must be safe to call at any time: a refused request changes nothing but emits the failure report. Guard quality is the security boundary — a weak guard is the only way an agent "decides" a transition. The open question (WHICH agent may request) was settled in ADR-0003.

## Main-Agent or human calls `rtm step`; Subagents never touch the Scheduler (ADR-0003)

**Context.** ADR-0002 allowed agents to request transitions but left open which agent. wishwillow fans tickets out to Subagents in worktrees; if any agent may call `step`, the Machine leaks downward into every worker.

**Decision.** Only the Main-Agent (main checkout) or the human invokes `rtm step`. Subagents check out tickets, do the work, and READ state; they never invoke `rtm`. The state-write invariant is unchanged: the Scheduler is the sole writer of State Files. The Main-Agent "writes state" only in the sense of invoking `rtm`; it never edits a State File directly.

**Consequences.** Subagents need zero Scheduler awareness — the Machine is invisible below the Main-Agent. Ticket→worktree parallelism stays inside a Phase; Exit Guards check the merged result. The CLI needs no caller authentication in v1; the policy is a documented rule, not enforced code (revisit if violated in practice).

## Machine Class file is TOML, `ratmac.toml` (ADR-0004)

**Context.** The handoff left the definition format open (TOML vs JSON); the requirement was to pick for rigor. The file is human-authored and reviewed (never agent-authored), so reviewability matters as much as parse strictness.

**Decision.** TOML. File name: `ratmac.toml` (term from session owner). JSON: strictest parsing but no comments — disqualified for a reviewed, human-written definition. YAML: house style in `.arca`, but typing footguns (implicit bool/number coercion) and anchors add ambiguity. TOML: strict spec, comments, first-class Rust support (`serde` + `toml` crates).

**Consequences.** The engine parses with `serde`/`toml`; unknown keys are hard errors (rigor over leniency). Comments in `ratmac.toml` are the place for phase intent notes — they never reach agents. State File format was decided separately (settled in ADR-0008).

## Machine Class vs Run — template and instantiation (ADR-0005)

Accepted with layout details pending; both open points were later settled (see below).

**Context.** The Scheduler must be general enough to read `ratmac.toml` as a state-machine "class" and create running instances per active run (template vs instantiation). Design was delegated to the session.

**Decision.** `ratmac.toml` = Machine Class: pure template, no runtime state inside it. `rtm start` instantiates a Run from the class. A Run owns: its State File, its Transition Log, its lockfile. The Scheduler arbitrates concurrent access per Run via the lockfile; the class file is read-only at runtime. The engine holds zero project knowledge: wishwillow's P1–P5 loop is merely the first Machine Class.

**Formerly open, now settled.** Run identity / targeting and concurrent-Run count → ADR-0007 (model N, allow 1 active). On-disk layout → ADR-0008 (`.arca/state.toml` + `.arca/log.md`, `.arca/current/` retired).

## Guard failure — refuse, report, stay (ADR-0006)

**Context.** `rtm step` evaluates the current Phase's Exit Guards; a failing guard needs defined semantics. Candidates: refuse and stay; bounded retries then `blocked`; `blocked` immediately. Criteria set by session owner: non-blocking, simplest, elegantly minimal.

**Decision.** Refuse + report, stay. A refused `step` changes NOTHING: Phase unchanged, Status unchanged, no counter, no log entry beyond the refusal report. The report names the failing guard and states observed vs expected fact (e.g. `files_exact: .arca/issue/42/ missing spec.md`). `rtm step` is idempotent under failure — safe to re-run any number of times. `blocked` keeps its distinct meaning from the handoff: missing ENTRY prerequisites (e.g. P4/P5 Execute client-supplied `test_root`, `run_command`, ...) set Status `blocked`. Exit-guard failure never does.

**Consequences.** No retry counter in the State File; the format stays minimal. Thrash detection is social: repeated identical refusals are visible to the Main-Agent/human; a counter can be added later without format breakage. Guard reports must be actionable (observed vs expected), since they are the agent's only fix signal.

## Model N Runs, allow 1 active (ADR-0007)

**Context.** ADR-0005 made Run a first-class instance of a Machine Class but left the concurrent-Run count open. Criteria set by session owner: elegantly minimal, simple, extensible.

**Decision.** Data model: Runs are plural — nothing in formats or engine assumes a singleton. v1 CLI: at most ONE active Run per project; `rtm start` refuses while a Run is active. Therefore `rtm step` and `rtm status` take no run-id in v1; they target the active Run.

**Consequences.** Zero CLI ambiguity for agents — the exact footgun of "default run when unambiguous" is avoided. Lifting the limit is additive: allow `start` to create a second Run, grow an optional run-id argument; no breaking change. The Run identity scheme is deferred until the limit lifts (YAGNI). The on-disk layout must not preclude N Runs (settled in ADR-0008).

## State layout — flat `.arca/state.toml` + `.arca/log.md` (ADR-0008)

**Context.** wishwillow's `.arca/current/` folder (`current.md` YAML, `log.md`) predates the Scheduler. The session owner removed the folder in favor of a general-purpose state file; the format then settled on TOML for rigor (consistent with ADR-0004).

**Decision.** Scheduler-owned files, all directly under `.arca/`, no folder:

- `.arca/ratmac.toml` — Machine Class (human-written, ADR-0004).
- `.arca/state.toml` — State File: `phase`, `status`, `goal_revision`, `input_revision`, `output_revision`, `active_refs`, `blocker`. Written ONLY by the Scheduler.
- `.arca/log.md` — Transition Log, append-only, human-readable.
- `.arca/rtm.lock` — lockfile while an `rtm` invocation runs (one active Run in v1, ADR-0007).

**Consequences.** `.arca/current/` is retired; wishwillow is asked via wish file, not edited by this project (see ADR-0005 consequences discipline). N-Run extension path: when ADR-0007's limit lifts, per-Run files move under a runs directory; the v1 flat layout is the one-active-Run projection of that — additive migration, deferred. State parse errors are hard errors: a corrupt `rtm` invocation halts with a report, never a guess.

## Phase Prompt — inline prose in `ratmac.toml`, guard list generated (ADR-0009)

**Context.** The "agent sees only its Phase" mechanism needs a prompt source. Candidates: inline in `ratmac.toml`; per-phase md files (drift risk, needs load-time cross-file validation); fully generated from guards (zero drift, loses human intent). Choice delegated: minimally elegant, fitting the owner's values (one purpose per construct, no drift, no extra files).

**Decision.** Each `[phases.X]` in `ratmac.toml` carries a `prompt` field: short human prose stating the Phase's needs-and-produces intent. The Scheduler renders the final Phase Prompt as: inline prose + a mechanically generated list of the Phase's Exit Guards. The Phase Prompt is the ONLY machine information an agent ever receives — never the flowchart, never other Phases.

**Consequences.** One file owns the whole class: definition and prompts cannot drift; no orphan-file checks needed. Guards stay checks, prose stays intent — no conflation. Prose must stay short; TOML multiline strings make long prose painful, and that friction is intentional (anti-nag).

## Print-first invocation (ADR-0010)

**Context.** Handoff open question: the Scheduler spawns `claude -p` headless per Phase, vs printing the phase-scoped prompt for an interactive session. The handoff proposed print-first; confirmed in session.

**Decision.** v1 prints. `rtm step` (on a successful transition) and `rtm status` print the current Phase Prompt (ADR-0009) to stdout. The Main-Agent or human feeds it into the working session. No process management, no spawn flag.

**Consequences.** Ships fastest; engine correctness is proven before any daemon concerns (timeouts, auth, output capture) exist. Spawn mode, if ever needed, is a future decision record — not a dormant code path.

## Rebrand compatibility decisions (RAT-007)

- Command: clean cutover; only `rtm` ships, with no legacy `schd` alias or unbounded shim.
- Package/crate: clean cutover to `ratmac`; no old-name re-export or duplicate package.
- Persisted data: `.arca/ratmac.toml`, `.arca/state.toml`, and `.arca/log.md` remain unchanged in layout and contents.
- Transient lock: use `.arca/rtm.lock`; if `.arca/schd.lock` exists, refuse safely and require explicit operator migration/removal; never silently delete or bypass it.
- Historical records: preserve append-only `.arca/log.md` and archived ticket wording in an explicit audit allowlist.

The full rebrand requirements and verification map are recorded in [i-001-ratmac-rebrand](../issue/i-001-ratmac-rebrand/test-plan.md).

## External repository identity cutover (EXT-001–EXT-006)

The external identity is a one-ticket, operator-controlled cutover layered on the frozen internal `ratmac`/`rtm` goal. It does not alter Rust behavior, Machine/Run/Phase/Status semantics, persisted data, or lock policy.

### Preparation and evidence boundary

Before any mutation, the ticket records the current commit, branch/worktrees, clean status, exact `origin`, Git top-level, checkout basename, `.git/config` identity, target-path collision result, process/path safety, `gh auth status`, target-slug availability, and rename authorization. It inventories old-slug hits by active tracked reference, generated/owned metadata, `.git` metadata, issue records, archived tickets, and append-only log. Only active tracked references and generated assets owned by their tools are changed in the preparatory commit; `.arca/log.md` and archived issue/ticket records are byte-for-byte historical allowlist entries. The preparatory commit is the repository-tracked evidence boundary.

### Ordered cutover and rollback

After the preparatory gates pass, checkpoint A is the committed clean tree and captured old slug/origin/path. Rename the GitHub repository through the authenticated API/`gh`, then checkpoint B verifies `egoisth777/ratmac` by API and `gh repo view`. Update `origin` to exactly `git@github.com:egoisth777/ratmac.git` and checkpoint C verifies `.git/config` and `git remote get-url origin` without pushing. Stop all processes using the old checkout, move the directory to `E:/repos/projs/skill-dev/ratmac`, reopen from that path, and checkpoint D verifies Git top-level and basename. No commit can be made *after* the local move merely to record the external mutation; path/remote/API outputs are operational evidence captured by the ticket run, while tracked preparation remains in the pre-cutover commit.

If any checkpoint fails, do not force-push, delete history, bypass locks, or continue with competing identities. Restore the GitHub slug through the authenticated API, restore the captured old origin, move the checkout back when safe, and revert only the unpushed active-reference preparation through a reviewable Git operation. Preserve commits, logs, archives, and working data; record the recovery result.

### Final acceptance

From the reopened checkout, run API and `gh repo view` checks, exact remote/path/.git checks, active-reference audit with the historical allowlist, clean status, `git diff --check`, formatting, linting, full Rust and QA/hidden suites, current T-001–T-022 behavior checks, integrated VR-001–VR-008 checks, and real `rtm` smoke/help/error checks. Acceptance requires all pass and no unallowlisted old external identity remains.
