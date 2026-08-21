# ratmac design

Decision record. Each section replaces a former ADR and keeps its id as an anchor. All decisions accepted 2026-07-22.

## Machine state is the Phase (ADR-0001)

**Context.** The seed handoff (`ratmac.md`, removed 2026-07-22 after full extraction; never committed — see `.arca/log.md`) listed `blocked` both as a machine state and as a `status` value, and carried `status` (`planned|executing|blocked|passed|failed`) as a second enum beside `phase`. The true state space was ambiguous: `phase` alone, or `phase × status`.

**Decision.** Machine state = Phase, nothing else. `status` is a phase-local lifecycle field the Scheduler records; it is NOT part of the Machine graph and transitions never branch on it in the definition. `blocked` is a `status` value only; it is removed from the state list.

**Consequences.** Machine definition files declare Phases and transitions only — no status dimension. The State File keeps both fields: `phase` (machine position) and `status` (lifecycle inside it). Any document listing `blocked` among the machine states is wrong by this decision and must drop it.

## Agents may request `next`; guards decide (ADR-0002)

**Context.** Core rule: agents never decide transitions. But someone must invoke the transition request. Candidates: human only (friction: human becomes the loop's clock), scheduler self-loop (builds the daemon now, against print-first), or agent-invoked.

**Decision.** Requesting is not deciding. An agent may request a transition (the handoff's `next`, now `rtm step`) when it believes the Phase is done. Exit Guards accept or refuse deterministically; the agent cannot talk its way past a guard because guards check artifacts, not claims. `start` remains user-only: loop entry is never agent-initiated (unchanged from handoff).

**Consequences.** The request must be safe to call at any time: a refused request changes nothing but emits the failure report. Guard quality is the security boundary — a weak guard is the only way an agent "decides" a transition. The open question (WHICH agent may request) was settled in ADR-0003.

## Main-Agent or human calls `rtm step`; Subagents never touch the Scheduler (ADR-0003)

**Context.** ADR-0002 allowed agents to request transitions but left open which agent. The concern recorded at the time: ticket work is fanned out to Subagents in worktrees, and if any agent may call `step`, the Machine reaches down into every worker.

**Decision.** Only the Main-Agent (main checkout) or the human invokes `rtm step`. Subagents check out tickets, do the work, and READ state; they never invoke `rtm`. The state-write invariant is unchanged: the Scheduler is the sole writer of State Files. The Main-Agent "writes state" only in the sense of invoking `rtm`; it never edits a State File directly.

**Consequences.** Subagents need zero Scheduler awareness — the Machine is invisible below the Main-Agent. Ticket→worktree parallelism stays inside a Phase; Exit Guards check the merged result. The CLI needs no caller authentication in v1; the policy is a documented rule, not enforced code (revisit if violated in practice).

## Machine Class file is TOML, `ratmac.toml` (ADR-0004)

**Context.** The handoff left the definition format open (TOML vs JSON); the requirement was to pick for rigor. The file is human-authored and reviewed (never agent-authored), so reviewability matters as much as parse strictness.

**Decision.** TOML. File name: `ratmac.toml` (term from session owner). JSON: strictest parsing but no comments — disqualified for a reviewed, human-written definition. YAML: house style in `.arca`, but typing footguns (implicit bool/number coercion) and anchors add ambiguity. TOML: strict spec, comments, first-class Rust support (`serde` + `toml` crates).

**Consequences.** The engine parses with `serde`/`toml`; unknown keys are hard errors (rigor over leniency). Comments in `ratmac.toml` are the place for phase intent notes — they never reach agents. State File format was decided separately (settled in ADR-0008).

## Machine Class vs Run — template and instantiation (ADR-0005)

Accepted with layout details pending; both open points were later settled (see below).

**Context.** The Scheduler must be general enough to read `ratmac.toml` as a state-machine "class" and create running instances per active run (template vs instantiation). Design was delegated to the session.

**Decision.** `ratmac.toml` = Machine Class: pure template, no runtime state inside it. `rtm start` instantiates a Run from the class. A Run owns: its State File, its Transition Log, its lockfile. The Scheduler arbitrates concurrent access per Run via the lockfile; the class file is read-only at runtime. The engine holds zero project knowledge: this project's own P1–P5 cycle is merely the first Machine Class.

**Formerly open, now settled.** Run identity / targeting and concurrent-Run count → ADR-0007 (model N, allow 1 active). On-disk layout → ADR-0008 (`.arca/state.toml` + `.arca/log.md`, `.arca/goal/` retired).

## Guard failure — refuse, report, stay (ADR-0006)

**Context.** `rtm step` evaluates the current Phase's Exit Guards; a failing guard needs defined semantics. Candidates: refuse and stay; bounded retries then `blocked`; `blocked` immediately. Criteria set by session owner: non-blocking, simplest, elegantly minimal.

**Decision.** Refuse + report, stay. A refused `step` changes NOTHING: Phase unchanged, Status unchanged, no counter, no log entry beyond the refusal report. The report names the failing guard and states observed vs expected fact (e.g. `files_exact: .arca/issue/42/ missing spec.md`). `rtm step` is idempotent under failure — safe to re-run any number of times. `blocked` keeps its distinct meaning from the handoff: missing ENTRY prerequisites (e.g. P4/P5 Execute client-supplied `test_root`, `run_command`, ...) set Status `blocked`. Exit-guard failure never does.

**Consequences.** No retry counter in the State File; the format stays minimal. Thrash detection is social: repeated identical refusals are visible to the Main-Agent/human; a counter can be added later without format breakage. Guard reports must be actionable (observed vs expected), since they are the agent's only fix signal.

## Model N Runs, allow 1 active (ADR-0007)

**Context.** ADR-0005 made Run a first-class instance of a Machine Class but left the concurrent-Run count open. Criteria set by session owner: elegantly minimal, simple, extensible.

**Decision.** Data model: Runs are plural — nothing in formats or engine assumes a singleton. v1 CLI: at most ONE active Run per project; `rtm start` refuses while a Run is active. Therefore `rtm step` and `rtm status` take no run-id in v1; they target the active Run.

**Consequences.** Zero CLI ambiguity for agents — the exact footgun of "default run when unambiguous" is avoided. Lifting the limit is additive: allow `start` to create a second Run, grow an optional run-id argument; no breaking change. The Run identity scheme is deferred until the limit lifts (YAGNI). The on-disk layout must not preclude N Runs (settled in ADR-0008).

## State layout — project-level plus per-Run files (ADR-0008, superseded in part by FDC-004)

**Context.** An inherited `.arca/goal/` folder (`current.md` YAML, `log.md`) predated the Scheduler. The session owner removed the folder in favor of a general-purpose state file; the format then settled on TOML for rigor (consistent with ADR-0004). The original one-Run projection placed every runtime file directly under `.arca/`; `FDC-004` later lifted that cap and superseded the flat Run-file paths.

**Decision.** Project-level files remain directly under `.arca/`:

- `.arca/ratmac.toml` — Machine Class (human-written, ADR-0004).
- `.arca/log.md` — Transition Log, append-only and human-readable.
- `.arca/rtm.lock` — one invocation lock for the local repository.

Each Run owns `.arca/runs/<id>/state.toml` as its State File and `.arca/runs/<id>/evidence.toml` as its Run evidence. Verdict and spawn-ledger locations also nest under that same addressed Run. The State File retains `phase`, `status`, `goal_revision`, `input_revision`, `output_revision`, `active_refs`, and `blocker`, and only the Scheduler writes it.

**Consequences.** Listing `.arca/runs/` is the registry; no flat `.arca/state.toml` or `.arca/evidence.toml` is live state. State parse errors remain hard errors: a corrupt addressed State File halts the invocation with a report, never a guess. This correction was integrated from [i-021-state-file-path-correction](../issue/archive/i-021-state-file-path-correction/design.md); it changes the stale goal wording, not the already-landed residency behavior.

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

The full rebrand requirements and verification map are recorded in [i-001-ratmac-rebrand](../issue/archive/i-001-ratmac-rebrand/test-plan.md).

## External repository identity cutover (EXT-001–EXT-006)

The external identity is a one-ticket, operator-controlled cutover layered on the frozen internal `ratmac`/`rtm` goal. It does not alter Rust behavior, Machine/Run/Phase/Status semantics, persisted data, or lock policy.

### Preparation and evidence boundary

Before any mutation, the ticket records the current commit, branch/worktrees, clean status, exact `origin`, Git top-level, checkout basename, `.git/config` identity, target-path collision result, process/path safety, `gh auth status`, target-slug availability, and rename authorization. It inventories old-slug hits by active tracked reference, generated/owned metadata, `.git` metadata, issue records, archived tickets, and append-only log. Only active tracked references and generated assets owned by their tools are changed in the preparatory commit; `.arca/log.md` and archived issue/ticket records are byte-for-byte historical allowlist entries. The preparatory commit is the repository-tracked evidence boundary.

### Ordered cutover and rollback

After the preparatory gates pass, checkpoint A is the committed clean tree and captured old slug/origin/path. Rename the GitHub repository through the authenticated API/`gh`, then checkpoint B verifies `egoisth777/ratmac` by API and `gh repo view`. Update `origin` to exactly `git@github.com:egoisth777/ratmac.git` and checkpoint C verifies `.git/config` and `git remote get-url origin` without pushing. Stop all processes using the old checkout, move the directory to `E:/repos/projs/skill-dev/ratmac`, reopen from that path, and checkpoint D verifies Git top-level and basename. No commit can be made *after* the local move merely to record the external mutation; path/remote/API outputs are operational evidence captured by the ticket run, while tracked preparation remains in the pre-cutover commit.

If any checkpoint fails, do not force-push, delete history, bypass locks, or continue with competing identities. Restore the GitHub slug through the authenticated API, restore the captured old origin, move the checkout back when safe, and revert only the unpushed active-reference preparation through a reviewable Git operation. Preserve commits, logs, archives, and working data; record the recovery result.

### Final acceptance

From the reopened checkout, run API and `gh repo view` checks, exact remote/path/.git checks, active-reference audit with the historical allowlist, clean status, `git diff --check`, formatting, linting, full Rust and QA/hidden suites, current T-001–T-022 behavior checks, integrated VR-001–VR-008 checks, and real `rtm` smoke/help/error checks. Acceptance requires all pass and no unallowlisted old external identity remains.

## Engine trust boundary (ETB-001–ETB-003)

**Context.** A self-hosted Runbook run routed every phase transition through a command guard that rebuilt mutable candidate QA code, printed refusals stripped of the gate's diagnostics, and cited a pre-integration goal hash in every residual. Integrated from [i-006-engine-trust-boundary](../issue/archive/i-006-engine-trust-boundary/design.md).

**Decision.**

- *Pinning.* Gate predicates that need project knowledge are folded into the pinned `rtm` binary itself (`rtm gate <predicate>`), so the Stable Engine pin covers all routing logic and there is a single trust surface. Where an external gate program is unavoidable, its resolved path and SHA-256 are recorded in Run evidence no later than first guard use and re-hashed at every evaluation; a mismatch refuses naming observed and expected identity. A guard command that would compile the workspace at evaluation time is rejected at Runbook validation or pin time, not silently executed.
- *Run evidence file.* Run evidence is the Scheduler-owned `.arca/runs/<id>/evidence.toml`: an `[engine]` table with the running Engine's resolved path and SHA-256, written at Run start, plus one `[[gate]]` entry per pinned gate artifact (declared program, resolved path, SHA-256) written no later than first guard use. It is deliberately separate from that Run's `.arca/runs/<id>/state.toml`, whose seven fields (R-025) stay unchanged.
- *Exemption.* Non-project probe commands (for example `rustc --version`) are marked `exempt = true` in the guard table so the pin rule stays enforceable without forbidding toolchain checks. An unmarked command guard is treated as project-derived and must resolve to a regular executable file: a directory or a symlink has no stable identity and is refused instead of pinned.
- *Diagnostics.* The `command_exit` evaluator replaces null stdio with a bounded capture of the child's stderr — last 4096 bytes, deterministic — embedded in the `GuardFailure` observed text, with an explicit `…truncated` marker on overflow and the fixed text `no diagnostic emitted` when the child is silent.
- *Freeze.* The goal content hash is computed inside the transition that closes intake integration. `baseline_revision` (Run creation) and `goal_revision` (post-integration freeze) are distinct Run-evidence fields; each later transition request re-verifies the frozen hash until batch closure and refuses on drift.
- *Freeze mechanics.* The Runbook marks the intake-completion transition `freeze = "goal"`; it is the only recognised freeze. The goal revision is a SHA-256 over every file under `.arca/goal/` - relative path and bytes, in sorted order - so an added, renamed, or removed file is drift even when no file is edited. Run evidence carries `[goal] baseline` (Run start) and `[goal] frozen` (intake completion) as distinct fields; the frozen value is mirrored into the existing `goal_revision` State File field, which is what gap analysis prints. At the boundary the frozen evidence is written before the State File, so an interrupted freeze leaves the Run unfrozen rather than half-frozen. A drift failure is appended to the guard failures of the same transition request rather than short-circuiting them, so a pin refusal and a drift refusal are reported together.

**Consequences.** Guard evaluation is a hash-verified, build-free operation; refusals name the artifact to repair; residual records cite a freeze that actually describes the classified requirements.

## Contract-verifying gates and honest routing (PGE-001–PGE-007)

**Context.** Mechanized P1–P5 gates checked shapes and statuses, not work: statuses could be relabeled past integration, evidence, sensitivity, and completion. Integrated from [i-007-contract-verifying-gates](../issue/archive/i-007-contract-verifying-gates/design.md).

**Decision.**

- *Gate implementation.* Every phase predicate lives inside the pinned gate boundary (in-process in `rtm`), parsing issue, residual, and ticket records through the same schema code paths as the shape check, so contract and shape cannot drift apart. `intake_contract` enumerates `.arca/issue/<issue-id>/`, `.arca/issue/deferred/<issue-id>/`, and `.arca/issue/archive/<issue-id>/` as one unique issue-id namespace and parses the exact `accepted|rejected|duplicate|deferred` ask dispositions from each `spec.md`, never from status alone. At the intake-completion boundary it requires the whole bundle under `deferred/` with status `deferred` if and only if at least one ask remains deferred; excludes deferred asks from archived `integrated|rejected` bundles; resolves each accepted requirement ID verbatim into the goal; lets a duplicate ask reuse an expectation already represented there without adding its proposed ID as a new row; requires every integrated bundle to have no deferred ask and at least one accepted-or-duplicate disposition; and checks live intake/deferred links in both directions. Unknown dispositions refuse, while archived links are frozen provenance rather than live links for this predicate.
- *Receipts.* `.arca/evidence/` is agent-writable and holds one structured file per executed check — command, working directory, target refs (planned-test ID, residual ID), exit status, and a SHA-256 over captured output — plus a per-ticket index. The P4 gate resolves each planned-test ID to a receipt carrying a failing baseline or mutation kill; the P5 gate re-executes the ticket's declared commands or verifies fresh receipts whose digests match re-hashed output. Receipts are evidence inputs, never Scheduler state.
- *Ownership.* Prompts direct agent-authored notes to the ticket file or `.arca/evidence/`; an executable prompt audit scans active Runbook prompts and gate contracts for Scheduler-owned paths.
- *Blocked route.* The human `hold t-<id>` convention is the authorization; the held ticket carries a `blocker-ref` to a new five-file issue (preferred) or a named residual. Route predicate `p5-blocked` verifies held-plus-linked state and routes to intake, leaving ticket status `held` and residuals untouched.
- *Abandonment.* `rtm abandon` requires an explicit human confirmation phrase, checks authorization before the first write, appends the terminal abandoned event to the Scheduler-owned log itself, then retires the active State File and the lock; no terminal value is ever written into the State File (wording corrected at the FDC-002 integration). Stale-lock recovery routes through this same authorized path; no bypass flag exists.

**Consequences.** A status edit can no longer route the loop; honest blockage and honest abandonment both have mechanized, human-authorized exits; ownership of Scheduler-owned files is obeyable.

## Reviewable snapshots and honest acceptance oracles (AOI-001–AOI-003)

**Context.** Green gates were computed over largely untracked candidate content, and the committed external-identity acceptance test failed the workspace suite over an authorized archive move while demanding live GitHub facts in default runs. Integrated from [i-008-honest-acceptance-oracles](../issue/archive/i-008-honest-acceptance-oracles/design.md).

**Decision.**

- *Snapshot audit.* A QA-side helper (callable later from the pinned gate boundary) runs `git status --porcelain` scoped to declared evidence roots — product sources, `test/`, and `.arca/` contributor artifacts by default — refuses on undeclared untracked or unstaged entries, and emits a manifest of sorted path, tracking state, and SHA-256 rows stored beside the evidence that cites it.
- *Archive-aware oracle.* The history-preservation oracle resolves both authorized directions. It accepts an unchanged history path or a complete move to `.arca/issue/archive/<issue-id>/` of a finished five-file bundle with no deferred ask: status `rejected`, or status `integrated` with at least one accepted-or-duplicate disposition, including valid duplicate-only integration that adds no new goal row. It compares bytes at the destination except required relative-link rewrites. When the archived `spec.md` itself contains a deferred disposition, it also accepts exact complete restoration of that same bundle to `.arca/issue/deferred/<issue-id>/`, allowing only the `index.md` status change to `deferred` and required live inbound/outbound link retargeting; no replacement carrier or other historical-prose edit is preservation. The whole bundle moves together, identity stays unique across all three locations, and inbound links inside other already archived records remain frozen and byte-identical. Content mutation outside those mechanical changes, partial moves, invalid status/disposition moves, and duplicate carriers stay loud failures.
- *Opt-in lane.* The environment-coupled release acceptance lane is `#[ignore]`-marked with an explicit runtime opt-in (`RATMAC_RELEASE_ACCEPTANCE=1`), plus one always-running reporter test that prints whether the lane ran or was skipped.
- *Schema.* The three-location issue namespace, completed-archive and deferred-restoration authorizations, frozen archived-link rule, and reviewable-snapshot rule are recorded as durable working guidance in `.arca/index.md`.

**Consequences.** Evidence describes a tree a reviewer can reconstruct; authorized archiving and exact archive-to-deferred restoration are preservation, not mutation; frozen archived provenance stays unchanged; ordinary branch work is not blocked by operator-cutover facts.

## Operable Run start and honest role evidence (ORS-001–ORS-003)

**Context.** A fresh session could not perform the documented first step: no project-local bootstrap, stale user-only help text, and role tests that asserted wording while claiming behavioral proof. Integrated from [i-009-operable-run-start](../issue/archive/i-009-operable-run-start/design.md).

**Decision.**

- *Policy surfaces.* `rtm start` help, `AGENTS.md`, `.arca/index.md`, and any canonical skill state one policy: human may start; Main-Agent may start only after explicit human Run-start sign-off; Subagent never invokes `rtm`. A QA audit scans active surfaces for retired user-only or never-agent-start wording. Conversational sign-off suffices; the Engine gains no caller identity or authorization state.
- *Bootstrap and doctor.* A read-only `rtm doctor` subcommand plus one repo-local launcher resolves the Engine binary from the project-local build or recorded pin path, hashes it, compares against the pin when present, and prints path, hash, Runbook validity, and state summary — offline and side-effect free. With no active Run it distinguishes `.arca/ratmac.toml` (human-authored Runbook) from `.arca/state.toml` (Scheduler-owned runtime state) and names the next legitimate action.
- *Behavioral harness.* Role scenarios are recorded transcripts of attempted commands or tool calls; QA asserts exactly one start invocation for the signed-off Main-Agent scenario and zero `rtm` invocations for unsigned and Subagent scenarios, with one deliberately violating transcript that must fail. Each check records its evidence kind (behavioral or guidance-consistency) so residual classification cannot conflate them.

**Consequences.** A fresh session can orient and start without ad-hoc installs; the caller policy is stated once and audited; invocation claims are proven by invocation records.

## Trial-worktree lifecycle (TWL-001–TWL-010)

**Context.** Repeated experiments on the experiment base had no lifecycle; the one observed trial died as an abandoned uncommitted branch whose evidence was discarded. Integrated from [i-010-trial-worktree-lifecycle](../issue/archive/i-010-trial-worktree-lifecycle/design.md).

**Decision.**

- *Interface.* One PowerShell 7 script `tools/trial.ps1` with verbs `start`, `status`, `finish`, `sync`, invoked as `pwsh -File tools/trial.ps1 <verb>` from the repository root, using plain Git and built-in cmdlets only. Exit 0 on success; non-zero refusals print one named reason plus guidance on stderr. The experiment base is the fixed constant `exp/ratmac-deterministic` (TWL-001), not a parameter: the interface offers no way to start a trial from another branch.
- *Dry-run.* Every mutating verb computes a plan first; `status` prints those plans — planned mutations plus recovery commands — and applies nothing.
- *Start.* Preflight (branch equals base, clean porcelain, no collision across `refs/heads/trial-*`, archive tags, registered worktrees, sibling directory, and `trials/*/`), then a single `git worktree add -b <trial-branch> <sibling-path> <base>`, then post-verification. Partial failure removes only the newly registered worktree without force and compare-and-deletes the new branch ref only if it still points at the recorded base commit; a failed rollback prints exact manual recovery commands.
- *Identity.* Trial number = max over `refs/heads/trial-*`, `refs/tags/trial-archive/*`, and `trials/trial-*/`, plus one, zero-padded to three digits. Worktree path = `<repo-parent>/<repo-basename>-<trial-branch>`. Archive tag = `trial-archive/<trial-branch>`, annotated, message carrying base and terminal commits plus the verdict line.
- *Finish order.* Preflights, then: create and verify the annotated tag at the trial tip (refuse if a same-named tag targets another commit); commit the durable log alone on the base as `trial(<trial-branch>): archive durable log`; `git worktree remove` without `--force`; compare-and-delete `refs/heads/<trial-branch>` via `git update-ref -d` only while the branch still points at the recorded terminal commit preserved by the verified tag. Status recognizes tag-only and log-only intermediate states and prints resume commands.
- *Windows locks.* Self-lock (working directory inside the worktree) refuses with a `cd` hint; an in-use directory refusal names the held sibling path and suggests closing shells rooted there — no `Remove-Item -Force`, no process enumeration or kill.
- *Sync.* Preflight clean base, then plain `git merge main`; on conflict exit non-zero listing conflicted files and leave the in-progress merge visible — no abort, reset, rebase, or force flag anywhere in the script.
- *Guidance and tests.* `.arca/index.md` gains the entry point, ownership split, and Windows working-directory rule. QA fixtures build throwaway repositories under a temp directory, shell out to `pwsh -File tools/trial.ps1`, and assert ref/worktree/tag snapshots around every positive and negative case, including a deterministic lock fixture; the one non-fixture check is the read-only `status` smoke in this checkout.

**Consequences.** Trials are free to fail: contained, numbered, reversible from their archive tag, and durable only through their log.

## Machine Class made first-class (RBS-001–RBS-005, TRP-001–TRP-006, DRD-001–DRD-007, AAL-001–AAL-004)

**Context.** The runbook is the product, but its definition lived only in code: `machine.rs` parsed a schema nobody had written down and then discarded the guards, `scheduler.rs` re-read the same file to evaluate them, `rtm doctor` judged validity with a third reader (a bare `toml::Value` syntax check), and an agent asked to write a runbook could only imitate an existing one. Integrated from [i-011-runbook-spec](../issue/archive/i-011-runbook-spec/design.md), [i-012-typed-runbook-parser](../issue/archive/i-012-typed-runbook-parser/design.md), [i-013-deep-rtm-doctor](../issue/archive/i-013-deep-rtm-doctor/design.md), and [i-014-agent-authoring-loop](../issue/archive/i-014-agent-authoring-loop/design.md).

**Decision.**

- *Home of the specification.* The runbook specification is `.arca/runbook-spec.md`, a shop-lane authority beside `schema.md` and `dict.md`, routed from `.arca/index.md`. It is deliberately outside `.arca/goal/`: the goal states that the specification must exist and be the single authority, while the specification itself is written and corrected by the build. Inside the bundle its authoring would be a goal edit after the freeze (`schema.md`, "the goal does not move") and every later correction would read as `ETB-003` goal drift.
- *Single authority.* Schema facts live in the specification and nowhere else. `ubi-lang.md`'s `Exit Guard` entry stops enumerating kinds and routes to the specification instead; the authoring instructions link rather than restate; the guard-kind table in the specification and the `GuardKind` enum in code are checked against each other by test (`RBSV-001`).
- *One parser.* `MachineClass` becomes the only reader of runbook TOML. `GuardKind` is a closed enum; each variant owns its typed fields, so a field foreign to a kind cannot parse. Guards are retained on the phase definition, and `PhaseDefinition::guards()` is what the Scheduler evaluates, what prompt rendering lists, and what `pending_guard_labels` reports. Deserialization stays hand-rolled over `toml::Value` inside `machine.rs` rather than serde-derived: the existing refusals carry located, prose-actionable messages (`unknown key "x" in phase "build"`) that serde's derived errors do not, and `MachineClassParseError` is already the caller-facing refusal type. What `TRP-001` requires is one reader, not one crate feature.
- *Named refusal.* `Scheduler::load_machine` loses its `NotFound → MachineGraph::default()` arm; an absent or unreadable `.arca/ratmac.toml` refuses by name through `StateError`, so `rtm status`/`step`/`start` all say the same thing.
- *Doctor as a finding list.* Diagnosis produces `Vec<Finding>` — `{code, severity, location, message}` — from four passes over the parsed class: parse/schema, graph, guard lint, ownership. One list, two renderings: `--json` emits it verbatim, the human form prints one line per finding. Codes are stable and grouped: `RB1xx` parse and schema, `RB2xx` graph, `RB3xx` guard lint, `RB4xx` ownership. The table lives once in the specification's diagnostics section and is asserted against the Engine's table.
- *Severity and exit.* Errors are defects that would make the Engine refuse or misroute; warnings are honest smells the author may accept (an agent-writable guard, a self-loop). `rtm doctor` exits `0` clean, `1` warnings only, `2` on any error. Argument-free `rtm doctor` keeps the `ORS-002` environment report and appends the runbook findings; `rtm doctor <path>` reports findings for that file alone, so the authoring loop can validate a draft outside `.arca/`.
- *Authoring.* `rtm scaffold <path>` writes the smallest runbook that is doctor-clean — two phases, one transition, one `exempt` toolchain guard — and refuses an existing path rather than overwriting. `.arca/runbook-authoring.md` carries the loop (scaffold → edit → `rtm doctor --json <path>` → repair by code → repeat) and one repair row per code, with every schema statement a link into the specification.

**Consequences.** The schema has one written source, one reader, one validator, and one repair vocabulary. A runbook defect is named by a stable code before it can become a mid-Run refusal, and an agent can author a runbook from the specification instead of from an example's accidents.

## Canonical run residency and identity (FDC-004–FDC-006)

**Context.** ADR-0007 modelled Runs as plural but capped v1 at one active Run and deferred the Run identity scheme "until the limit lifts"; ADR-0008 only promised the on-disk layout would not preclude N Runs. Nothing said where a Run lives, what addresses it, or what happens to an id after abandonment — so a run directory could be re-entered by a later Run and overwrite the evidence of the earlier one. This section lifts the limit and states residency. Integrated from [i-017-run-residency](../issue/archive/i-017-run-residency/design.md), which condenses the settled research on run identity, the invocation join, and migration cost; the decision records stay in the split seed, the verdict-routed execution core issue ([i-016-fsm-doctrine-convergence](../issue/archive/i-016-fsm-doctrine-convergence/design.md)), as adopted defaults under the 2026-07-29 batch sign-off.

**Decision.**

- *Residency is the registry.* Runs live under the plural `runs` path, one directory per run id in a single id namespace, so listing that path IS the roster: run identity is read off artifacts, never off a narrated index. Verdict slots nest under their own run's directory, which gives verdict addressing a computable base and keeps a recorded transcript self-describing.
- *Reserved location, foreign contract.* A per-run spawn-ledger path is reserved by name under the run's directory and nothing more. Its contract — contents, when written, meaning — belongs to the machine-composition issue ([i-018-machine-composition](../issue/archive/i-018-machine-composition/spec.md)). The goal reserves a location here because a requirement may not cite a contract defined nowhere in its own scope, and because a ledger without a spawn verb has nothing to record.
- *Addressing.* `--run <id>` is always required; a missing value refuses and prints the roster. No default-when-unambiguous rule: that is the footgun ADR-0007 avoided by taking no run-id at all, and an always-required argument keeps the property once several Runs are live.
- *Uncapped, never reused.* There is no active-Run cap. Any cap below the fan-out width would refuse mid-spawn and leave a partial child bundle unusable. Within the one namespace an id is never reissued after abandon, so a failed Run's evidence keeps its address and no later Run can occupy it.
- *Pin stays hash-only.* The runbook pin remains a hash, with no per-run copy of the runbook until a drift case is demonstrated: two files that can disagree is a defect source, and the hash already names the mismatch.
- *Residue refuses.* Meeting a flat-layout residue — a pre-plural run directory on disk — the Engine refuses and instructs, and migrates nothing. This follows the existing lock-refusal precedent: name the observed fact and the repair, modify nothing.
- *Supersession.* `FDC-004` and `FDC-006` supersede the v1 clauses ADR-0007 marked liftable: `R-022` (at most one active Run) and `R-023` (no run-id argument), and with them checks `T-08` and `T-09`. `FDC-004` also supersedes `R-024`/`R-025` only where their one-Run projection puts State and evidence files flat under `.arca/`; [i-021-state-file-path-correction](../issue/archive/i-021-state-file-path-correction/design.md) records that reconciliation. ADR-0007's plural data model is unchanged.

**Consequences.** Run identity becomes durable: an address, once minted, names one Run forever, and the record of a finished or abandoned Run cannot be overwritten by whoever works next. The residency layout is stated before anything is built on it, so verdict routing and machine composition can be written in terms of a defined address instead of coining one each. One authority clash surfaced at the 2026-07-29 P1 close and was closed before this section was allowed to bind: steering's Non-goals read "one Run at a time", which is Authored identity and binds harder than any goal row, so that clause moved first — narrowed to "one repository, local disk" with Runs plural and uncapped inside it, on the same sign-off that accepted `FDC-006`. The tenancy non-goal itself is untouched; only the v1 concurrency cap ADR-0007 marked liftable was lifted.

## Input-routed transitions (FDC-001)

**Context.** `MachineGraph::transition_for` selects the first ordinary transition declared from the current Phase. Guards can refuse movement but cannot say which destination judgment selected, so a branching Machine routes by file order and convention. Integrated from [i-016-fsm-doctrine-convergence](../issue/archive/i-016-fsm-doctrine-convergence/design.md).

**Decision.**

- *Accepted format.* A branching `[phases.<name>]` carries `inputs = ["<value>", ...]`. Each ordinary `[[transitions]]` row from that Phase carries `input = "<value>"`. Values are exact, non-empty strings. The list is closed and unique; every value has exactly one ordinary edge, and every ordinary edge has exactly one listed value. Two rows may share a destination when their input values differ.
- *Straight lines and blocked routes.* A Phase with one ordinary outgoing edge declares no `inputs`, and that edge declares no `input`. A Phase with no ordinary outgoing edge is structurally terminal. A blocked route never declares `input` and is excluded from ordinary coverage and selection.
- *Static refusals.* The format source assigns `RB208` to a malformed `inputs` declaration, `RB209` to a branching Phase with no list, `RB210` to missing coverage, `RB211` to duplicate coverage, `RB212` to a foreign, mixed, or forbidden ordinary label, and `RB213` to an input-labelled blocked route. Existing type and unknown-key failures remain `RB110` and `RB103`. The implementation ticket updates `.arca/runbook-spec.md`, the parser, doctor table, scaffold, and authoring repairs together so the executable and written code sets never diverge.
- *Runtime order.* `rtm step` first evaluates every retained guard in declaration order. A refusal leaves the Run and any live input untouched. For a branch, the Engine then obtains one transition input through `FDC-003`, validates it against the current Phase's list, and selects the unique matching ordinary transition; declaration order has no routing effect. A straight-line step selects its sole ordinary transition without an input.
- *Disclosure.* The generated Phase Prompt lists the current Phase's legal input values but never its destinations or any other Phase. This gives an external evidence reviewer the closed answer vocabulary without exposing the Machine graph, preserving `R-029`.

**Consequences.** Readiness and selection are separate contracts. The Machine Class proves branch completeness before execution, and runtime routing becomes a pure function of current Phase plus validated transition input.

## Durable transition-input delivery (FDC-003)

**Context.** Selection needs one durable handoff from evidence review to the Engine. Run residency reserved `.arca/runs/<id>/verdict.toml` but deliberately gave it no contents or lifecycle, so the Engine cannot safely consume or retire an input today. Integrated from [i-019-input-delivery-durability](../issue/archive/i-019-input-delivery-durability/design.md).

**Decision.**

- *One live record.* `.arca/runs/<id>/verdict.toml` is the Verdict slot. It is absent when empty, including after `rtm start`; an empty placeholder is not a verdict. A published record is strict TOML with exactly three non-empty string fields: `phase`, `input`, and `rationale`. `phase` must equal the addressed Run's current Phase, and `input` must be in that Phase's closed list.
- *External judgment.* A designated agent or human acting as the external evidence reviewer writes and publishes the record; the Engine stores no reviewer identity, chooses no reviewer, and never authors or substitutes the input. Publication uses a temporary sibling plus rename so the Engine sees either the prior complete record or the new complete record, never a partial write.
- *Consume before advance.* After guards and record validation, the Scheduler creates the Run-local `verdicts/` evidence directory if needed and renames the live record to the next unused `verdicts/<nnnnnn>.toml`, where the positive, zero-padded sequence is monotonic across the Run. The archived record retains all three fields, is never overwritten or deleted by the Engine, and is the decision history.
- *Interruption boundary.* The archive rename is the consumption point and occurs before the successor State File write. Before it, any refusal leaves live record and State File byte-identical. After it, a process interruption leaves the old State File and no live record, so retry requires a fresh verdict; the archived record cannot replay. A failure to write the successor does not roll back or reuse consumed judgment.
- *Wrong-time input.* A malformed record, a phase mismatch, a value outside the current list, a missing branch record, or a live record presented to a straight-line Phase refuses without transition. Completed or abandoned lifecycle behavior remains outside this requirement.

**Consequences.** One judgment can cause at most one transition. Run evidence records every consumed input without making the Engine a judge, and repeated visits cannot overwrite earlier decisions.


## Run completion (FDC-002)

**Context.** Runs advanced but never ended: start always wrote `planned`, step carried the prior status forward, and completion existed only as prose. A composition join and a cycle runbook both need a terminal fact the Engine itself wrote. Integrated from [i-020-run-completion](../issue/archive/i-020-run-completion/design.md).

**Decision.** The end of a Run is Engine-observable and Engine-written. A state is terminal when it has no ordinary outgoing edge; entering it completes ordinary execution. `rtm start` beginning in a terminal state and `rtm step` arriving at one write status `passed` in the same atomic State File replacement that records the position. A passed Run admits no further transition. Explicit abandonment appends its durable terminal event — naming the addressed Run, its last phase, status, and goal revision — to append-only history before any active state is retired; `abandoned` is never a surviving State File value. Guard refusal stays non-terminal and leaves Run state byte-identical. No path writes `failed`: the value remains legal vocabulary with no Engine write path until a later issue names a concrete Engine-observable failure event.

- *Terminal recognition.* Structural only: no ordinary outgoing edge, where ordinary means not a blocked route. The Machine Class never declares lifecycle status (R-002/R-003); the parser and doctor already use this exact edge definition.
- *Two write points.* Start-in-terminal and arrival-at-terminal. `passed` is written only by ordinary motion; a human hold never writes it.
- *Terminal Runs refuse motion.* `rtm step` and a human hold refuse a passed Run by name, before guard or route work, leaving state untouched.
- *Durable abandoned event.* The synced event append precedes retirement; the existing all-or-none compensation keeps event and retirement consistent.
- *Deferred failed.* A guard refusal is not failure and no failure command exists.

**Consequences.** A join or a stage oracle reads `passed` from the State File without trusting narration. A Phase whose only outgoing edge is a blocked route is structurally terminal; once its Run passes, that blocked route is unreachable.

## Machine composition (FDC-007–FDC-012)

**Context.** The Engine ran exactly one machine per project wish: no Run could create another, review arrived as a hand-delivered verdict, and the spawn-ledger location `FDC-004` reserves had no contents. The four preceding contracts — per-Run residency, input-routed selection, durable input delivery, Engine-written completion — were sequenced for exactly this consumer. Integrated from [i-018-machine-composition](../issue/archive/i-018-machine-composition/design.md); research ground in `.arca/research/re-ratmac-FSM/05-invocation-join.md` and `07-conceptual-model.md`.

**Decision.** One Run creates and consumes other Runs as checked ordinary motion. `rtm spawn` instantiates a child class the parent's runbook declares into an ordinary flat top-level Run and appends its entry to the parent's spawn ledger; a `join` guard on the parent's out-edge reads each ledger child's Engine-written terminal fact; `respawn` and abandon-with-run-id are human-confirmed by phrases naming the run id; every Phase on a cycle keeps at least one receipt- or contract-guarded out-edge; the runbook format carries the class and spawn tables; composition is capped at one level.

- *Spawn is ordinary motion (FDC-007).* No confirmation phrase; legal only while the parent occupies the spawning Phase. A child is an ordinary Run under the plural `runs` path — same State File, lock, verdict slot, evidence, and terminal facts; nothing child-shaped exists in the runtime contract, and run ids stay one namespace (FDC-004/FDC-006).
- *The ledger fixes the join's expected set (FDC-011).* Scheduler-owned, append/annotate-only, at the reserved per-run path: each entry carries the child run id, class, binding values, the git revision at spawn, and the workspace when one is created. Abandon flips only the abandoned mark; respawn appends the successor entry naming the superseded id. A ledger entry whose child directory is missing refuses loudly — the expected set never silently shrinks.
- *The join reads Engine-written facts.* A join passes iff every non-abandoned ledger entry's child stands at a graph-terminal phase with status `passed` and the satisfying count meets the declared minimum; a refusal names every non-satisfying child. The fact it reads is the `FDC-002` terminal write — Scheduler-authored, stable once true, not agent-writable.
- *Waiting is refusal.* No new wait machinery: while the join fails, the parent's `rtm step` refuses and the Run parks in its spawn/join Phase. Parent position stays a single scalar; plurality lives in the child State Files on disk; `active_refs` names the open spawn so `rtm status` answers what the Run waits on.
- *Authorization split (FDC-007) and supersession (FDC-006).* `respawn` and abandon-with-run-id demand phrases naming the run id. Respawn mints a fresh id for the same bindings and never overwrites: the superseded child's record and evidence keep their address.
- *Cycle termination by kind membership (FDC-008).* A static doctor check: every Phase on a cycle carries at least one out-edge guarded by receipt- or contract-class guards only. No monotone-fact prose survives; termination is guard-kind membership.
- *Format extension (FDC-009).* `.arca/runbook-spec.md` — the single format authority (RBS-004) — grows the class and spawn tables; the prior format content that would refuse them is superseded. Static validation proves a spawn table names a declared class and its binding names equal the child class's required set. Canonical spelling stays `blocked-route`.
- *Child-as-reviewer (FDC-010).* The judgment a parent's branching Phase consumes may be authored by a spawned child machine; the Engine still makes no judgment and chooses no reviewer (`FDC-003` posture), and the witnessed verdict verb stays deferred because signer identity remains outside the Engine (`ORS-001`).
- *One level deep (FDC-012).* A spawn addressed to a Run recorded as a child in any spawn ledger refuses naming the cap. Lifting the cap is additive.

**Consequences.** A parent machine finishes on durable facts child Engines wrote — no human courier between machines. The composed picture self-hosting needs — one child per cut ticket, a join that closes the sprint — becomes expressible as runbook data, and every new surface stays inside the existing postures: one writer for state and ledger, refusal over guessing, guards judging artifacts.

## Full doctor executable fingerprint (DFP-001)

**Context.** The argument-free environment report already resolves the exact current executable, computes its SHA-256, and stays write-free, but renders only an abbreviated digest. Integrated from [i-023-doctor-full-fingerprint](../issue/archive/i-023-doctor-full-fingerprint/design.md); the archived trial is evidence only and supplies no implementation bytes.

**Decision.** The human environment report renders all 64 lowercase hexadecimal characters of the SHA-256 already computed for the exact current executable. Executable selection and hashing, pin and trust behavior, runtime-state reporting, Runbook diagnosis and findings, arbitrary-path diagnosis, and `--json` remain governed by `ORS-002` and `DRD-005` and are unchanged.

**Consequences.** A reader can independently identify the exact Engine bytes from the report without changing any machine decision or write boundary.

## Engine namespace and repository-scoped runtime (ADR-0011)

**Context.** ADR-0008 puts Engine-owned files under `.arca/` and relies on one project-level invocation lock. That gives linked worktrees checkout-local runtime state and makes unrelated Run motion wait behind one another. Integrated from [i-024-engine-namespace-split](../issue/archive/i-024-engine-namespace-split/design.md).

**Decision.** The Engine owns exactly one `.ratmac/` Engine root at the primary checkout root. It holds the Machine Class `ratmac.toml`, `runs/`, the durable `mint.toml` record, `locks/`, the Scheduler-only transition log `log.md`, and tracked receipts under `evidence/<run-id>/`; `.arca/log.md` is human-only. In a linked Git worktree, runtime resolves the primary checkout's Engine root; without Git, it resolves `.ratmac/` at the current checkout root. The Machine Class is read from the invoking checkout's tracked `ratmac.toml`, so an edit that changes a live Run's pin refuses under `FDC-005` rather than silently having no effect. `rtm status` and `rtm doctor` report the resolved Engine root.

Minting reads and advances the durable mint record under a short root lock, so deleting a Run directory cannot reissue its id. The root lock covers minting and roster or ledger mutation. A per-Run lock covers motion on one Run; when both locks are necessary, acquisition is root before Run, and guard evaluation holds no root lock. `rtm spawn --workspace <path>` canonicalizes and records the child workspace binding; without the flag the child inherits its parent's workspace, and its guards and motion resolve against that recorded workspace.

The top-level runbook `[roots]` table maps role names to repository-relative paths, so guards name roots rather than hard-coded workflow paths. Static validation gives distinct diagnostics for an undeclared root name, a missing root path, and a root overlapping the Engine root; no `.arca` path literal remains in Engine source. Every entry point refuses and instructs without moving anything when it finds `.arca/ratmac.toml`, `.arca/runs/`, `.arca/rtm.lock`, or a flat `.arca/state.toml`; archived `.arca/evidence/` receipts are inert history.

**Consequences.** All linked worktrees share one roster, Run-id namespace, lock domain, and Scheduler-owned transition log, while different Runs can move independently. Runtime entries under `.ratmac/` are Git-ignored; the Machine Class and run-scoped receipts remain tracked, preventing Run state from entering a ticket branch and keeping parallel sibling receipts collision-free on merge. This decision supersedes ADR-0008's state-layout path spellings.

## State vocabulary and the Run Record (ADR-0012)

**Context.** `ADR-0001` made the machine position the only dimension of machine state and named it `Phase`, and `ADR-0003` named the Engine's per-Run file the State File. "Phase" reads as a stage of a linear process, so the written schema teaches a pipeline where the product is a general state machine, and the word "state" was already carrying three loads at once: the graph position, the persisted file, and the whole runtime record. Integrated from [i-025-state-vocabulary](../issue/archive/i-025-state-vocabulary/design.md).

**Decision.** Three words are settled first, then the rename lands on the freed word.

- *One word each.* **State** is the position in the machine graph. **Run Record** is the one file the Engine writes for one Run. **Run** is the whole live instance. `status` is unchanged: Engine-owned lifecycle, five values, never a position. "State File" and "Phase" are retired as live terms; **State Prompt** replaces **Phase Prompt**.
- *Format surface.* The runbook declares `[states.<name>]`, `[[states.<name>.spawns]]`, and `[classes.<name>.states]`; `from` and `to` name States. Every other key, guard kind, rule, and diagnostic code of `.arca/runbook-spec.md` is untouched, so `RBS-004`'s single-authority rule holds with only its nouns changed.
- *On-disk surface.* The Run Record is `.ratmac/runs/<run-id>/run.toml` and its position field is `state`. Renaming the field without the file would leave the ambiguity on the path an operator reads most; renaming the file without the field would do the same inside it. Strict parse, atomic replacement, and the single Scheduler writer are unchanged.
- *Residue refuses.* A runbook declaring `phases`, and a Run Record at the pre-cutover filename or carrying the pre-cutover field, each refuse before any read, join, parse, or write, naming the artifact and the repair. This is the third instance of the posture `FDC-005` and `ENS-009` already set: never migrate in place, never guess. A generic unknown-key error is not enough — the author must be told the word changed.
- *Codes are identity, text is not.* Each existing defect class keeps its exact diagnostic code and gains new message text; the pre-cutover runbook residue gets one new code. `DRD-006`'s promise that callers branch on codes survives the rename.
- *Names only.* No behavior moves in this cutover, which is what makes it reviewable: every existing check keeps its meaning and is expected to pass unchanged apart from the spellings it asserts.
- *History is evidence.* Archived bundles, archived tickets, archived gap records, and `.arca/log.md` keep their bytes. The audit that proves no live surface says `Phase` carries them as an enumerated allowlist, never an open-ended skip, so the allowlist itself stays reviewable.

**Consequences.** This decision supersedes `ADR-0001`'s and `ADR-0003`'s term spellings and the `state.toml` filename that `ADR-0011` recorded; the decisions themselves — one position dimension, status outside the graph, one writer — are unchanged. Every Run Record, runbook, and message produced before the cutover is refused rather than read, so the cutover is a hard boundary with no dual-reading window. The `[roots]` table, guard kinds, spawn and join contracts, and lock and mint rules keep their shape.

## One engine binary per build target (ADR-0013)

**Context.** The repository declares the Engine command twice. The root package builds `rtm`
from `src/bin/rtm.rs`; the test package builds a second command from that same source file
with the test-only pause points compiled in. Both land at `target/debug/rtm`, so cargo warns
about an output filename collision and the last writer wins. Measured at `f9692cf`: after a
whole-repository build the file carries no pause-point wiring and the hold barrier check
`t050_blocked_route::runbook_swap_before_hold_state_write_refuses_without_a_half_route` fails
with "hold did not reach the pre-State snapshot barrier"; after a test-package build it
carries the wiring and the same check passes. The suite's colour therefore reports build
order, and that sentence is the evidence half of every ticket the shop lands.

**Decision.** Two targets, two names, one source file.

- *The shipped command keeps its name.* The root package still builds `rtm` from
  `src/bin/rtm.rs`. Nothing about bootstrap, pin check, doctor identity, or the pause-point
  boundary moves: the shipped command is still built without the test-only feature.
- *The test copy is named for what it is.* The test package's target gets its own name, so it
  writes its own output file, and every test that launches the Engine names that target. A
  test therefore always launches the build it was compiled against.
- *The rule is stated in the shop's own words.* A check reads the package manifests, resolves
  each build target to its output file, and fails naming both declarations when two targets
  agree. Reading the declaration rather than the build keeps the check independent of how the
  toolchain words its warning, and of whether the toolchain keeps warning at all.
- *Rejected: compile the pause points everywhere.* Turning the test-only feature on in the
  shipping package would make both copies identical and the collision harmless. The pause
  points read environment variables and stop the Engine mid-write; that belongs in a test
  build and nowhere near a shipped command.

**Consequences.** `cargo test --workspace` becomes admissible evidence, which is what
`SVC-007`'s behaviour-unchanged proof needs. No Engine behaviour changes: routing, guards,
locking, receipts, and exit codes are untouched, and the rename is mechanical - a target name
and the constant every test uses to find it.


## The Engine has no work-item concept (ADR-0014)

**Context.** Two rules in force contradicted each other. `ENS-001` says the Engine writes no
file under `.arca/`; the working rules and `PGE-006` said an authorized `rtm hold` marks the
ticket file `held` with its blocker, and `src/blocked.rs` did exactly that - reading the
ticket to check its status, rewriting two of its fields, and refusing on a work-item shape
the Engine invented (`a complete five-file issue folder or a named residual record`). The
completion gate then re-read that same contributor file to learn a ticket was held. One
contributor file was both the Engine's write target and the Engine's index.

**Decision.** The Engine is a generic state-machine runner, so it does not know what a ticket
is. The narrow fix - name the write as an allowed exception - was refused for the wider one.

- *Pause is Run state.* `rtm hold` writes the paused mark and the blocker reference into the
  Run Record under the Run lock, and appends one entry to the Engine transition log. Nothing
  else is written, and nothing under a workflow root is written at all.
- *The blocker is an opaque reference.* The Engine checks that it exists and resolves beneath
  a declared runbook root, and nothing more. "A five-file issue folder or a named residual"
  is this shop's rule about its own records, enforced by this shop's own intake check, not by
  a generic runner.
- *One reader, one source.* The completion gate learns that work is paused from the Run
  Record, never from a document a contributor writes. A gate that reads the agent's own file
  to decide whether the agent may pass is exactly the evidence-not-claim rule inverted.
- *Marks are shop actions.* A human-readable `held` mark on a ticket is still useful, so the
  working rules keep asking a contributor to write one. A contributor writing it is not an
  Engine write, and no Engine decision may depend on it.
- *Names stay generic.* No Engine argument, message, refusal, field, or path may spell a
  work-item document, its fields, or its filename shape.

**Consequences.** `PGE-006`'s ticket-file mechanics are superseded: the honest blocked route
survives unchanged in what it guarantees - human confirmation, a declared blocked edge, a
routed Run, an unproven residual, a refusing completion gate - while the place the fact lives
moves into Engine-owned state. `ENS-001` becomes true rather than nearly true. The known
remainder is named, not hidden: `src/completion.rs` still parses a ticket document for the
checks a ticket declares (`## Merge Gate`, `HT-nnn-nn` lane ids, a `ticket-id` receipt
field), which is the same leak in a different place; it is filed as its own wish rather than
folded into this ruling, because removing it redesigns the completion gate's contract.

## The shop's own cycle as a runbook (ADR-0015)

**Context.** The engine was built to run a declared process, and the only process it has ever
run is a demonstration machine that builds a file called `release.txt`. The cycle this
repository actually follows lives as prose in the working rules, and "where are we" is
answered by a person reading a lookup table. The issue about running the cycle as a runbook
([i-015](../issue/archive/i-015-cycle-as-runbook/index.md)) waited three planning passes for the
contracts beneath it; those are landed, so the question is no longer whether but in what
shape.

**Decision.** Six rulings, each forced by something already in force rather than chosen for
taste.

- *One sprint is one Run.* The format fixes this, not preference. The initial State is the one
  State with no inbound ordinary edge, and its absence is the error `RB202`. A rest State that
  routed back to intake would give every State an inbound edge and no machine would parse. So
  the cycle machine runs from an intake State to a terminal rest State, reaching the
  Engine-written `passed` fact there, and the next sprint is the next Run.
- *The stage is the State.* Because a sprint is a Run, the addressed report answers the stage
  question directly from the Run Record while the sprint is live. Between sprints there is no
  Run and no answer, so the tree-derived lookup survives as the labelled fallback for exactly
  that window and is never a competing second answer.
- *The work item is addressed by a binding, not by a name in the file.* The per-item gates need
  to know which item they judge, and a read-only runbook may not carry the identifier. The
  earlier proposal bound the target through the Run Record's active references and had to
  invent a derivation to stop a stale value from choosing what the gates grade. Composition
  removes the problem instead of guarding it: the stage that opens the ticket turns declares
  one child class, each turn is spawned with its address supplied as a binding value, and the
  Engine records that value in the append-only spawn ledger. A value written once at spawn and
  never rewritten cannot go stale, the runbook holds no identifier, and the Engine still reads
  an opaque string - `NRR-001` holds. The remaining literal-address form stays exactly as it
  is; this adds the bound form beside it.
- *A doctor-clean cycle rules out the file-shaped guards.* Any `files_exact` or `file_contains`
  guard over a path outside the Engine's own tree raises the `RB302` warning, which makes the
  doctor exit `1`. Requiring exit `0` therefore constrains the cycle to the contract-, receipt-,
  join-, and command-class guards, which is the honest test of whether the closed vocabulary
  can express a real process. It also means the branch out of the gap-check stage is routed by
  a declared transition input rather than a guard: no guard kind can say "no gap remains", and
  inventing one is out of scope. The evidence behind that input is not lost - the record gate
  reads the same records on the way in, and the next Run re-derives the judgment at its own gap
  check.
- *Working-authority requirements are first-class at intake.* The intake gate resolves an
  accepted ask to a goal row today. This repository's own tree would be refused by it, because
  rules that bind the contributor rather than the program resolve to headings in the working
  authority and deliberately mint no goal row. The gate learns that second resolution, and
  refuses only an ask that resolves to neither.
- *Damage runs from a checkpoint, and a guard proves it.* The step into the deliberate-damage
  stage carries a command guard that observes whether the tree holds an uncommitted change to a
  tracked file. That is the exact hazard the safety-commit rule was written for: the
  composition-format turn destroyed a tracked file by restoring stale index bytes. A file that
  was never added is invisible to an exit code and is also untouched by the restore this rule
  protects, so the guard is weaker than the prose in a place where the prose is not load-bearing.

**Consequences.** The runbook that governs this repository is authored under the manual cycle
one last time and governs the sprint after it; a machine cannot bootstrap its own first Run.
`PCR-004` is rejected rather than deferred: the landing line stays a human act, because the
working rules now make `.arca/log.md` human-only and `NRR-001` forbids the Engine writing
under a workflow root, so the property that history cannot be rewritten by whoever is working
is carried by the Engine-owned transition log instead. Two limits are named rather than hidden:
the dirty-tree guard cannot see a file that was never added, which needs a repository-state
guard kind that steering already carries as deferred debt; and the completion gate still parses
a contributor's document for the checks it declares, the remainder `ADR-0014` named and left
to its own wish.

## The engine teaches its own operation (ADR-0016)

**Decision.** The operator protocol ships two ways, per Billy's 2026-08-18 ruling: the
status/step renderers derive guard expectations from the parsed guard declaration and end
every outcome with one truthful `next:` line (`AOP-001`, `AOP-002`), and a sibling of the
scaffold writes the thin `ratmac-operator` skill folder - one folder, never overwrites,
engine-identity stamp, no flag enumeration (`AOP-003`, `AOP-004`). An MCP server was judged
an adjunct (unreachable for plain CLI agents, protocol churn); a scaffold-emitted AGENTS.md
stub stays open for a later issue if skill activation proves unreliable.

**Consequences.** Rendering is derived, never hand-kept, so a future guard kind that forgets
its rendering fails a golden test rather than shipping a silent gap; a fabricated `next:`
hint is worse than none, so an unsupportable line is omitted. The skill teaches invariant
behavior only and points at the CLI for everything current, so it cannot teach stale flags.

## The ledger row records, never predicts (ADR-0017)

**Decision.** Stable resolution reads the invoking project's current checkout - the ledger
row and the tag must agree there - and the engine is then located or built from the tagged
commit in a clean separate checkout whose tree is identical to that commit (`ELR-002`). The
working-rules side (`ELR-001`, schema Editions) moved the row's writing to the recording
landing that follows the tag, because a commit cannot contain its own hash.

**Consequences.** A stable engine is buildable from any healthy `main` without hand edits;
the tagged commit's own stale ledger is expected, not a defect. The build checkout's tree
must match the tagged commit exactly, so a workaround that overlays files into it is a
refusal - the class of trust leak the 2026-08-21 sprint setup exposed.

