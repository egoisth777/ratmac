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


## The turn lifecycle is shop tooling beside the trial lifecycle (ADR-0018)

**Decision.** The turn-housekeeping commands ship as one repo-local lifecycle
entry point beside the trial lifecycle - `open` and `close` verbs with a
`status` dry run - not an Engine subcommand and not a runbook-declared
lifecycle the Engine drives, per the 2026-08-24 planning pass confirming the
bundle's proposal: the choreography mutates Git plumbing and untracked lanes
the Engine never owns, the work item stays the opaque binding string a Run
already carries (`PCR-007`, `NRR-001`), and the trial lifecycle proves the
shape on this repository (`TWL-001`-`TWL-010`). Recovery when an interrupted
close cannot resume cleanly belongs to the invoking human or Main-Agent from
the primary checkout (`TWL-010`'s ownership table): the close refuses naming
the completed steps and prints the recovery commands; a Subagent invokes no
lifecycle verb.

**Consequences.** The Engine stays free of work-item and Git-worktree
knowledge, and any project reuses the mechanism by declaring its lanes root,
trunk branch, skip list, stamp field, and landing log as runbook data (the
`[roots]` table shape). The prose duty in the schema's Units-and-git rules
becomes the specification the close mechanizes, so that section's revision
lands with the tickets that prove `THK-001`-`THK-004`; the fixed order -
merge, verified copy-back, stamp, log line, removal, trunk rerun - is
enforced exactly where a slip is destructive, with removal last behind the
only-copy refusal.

## The citation check ships in the qa crate; reachability is any ref (ADR-0019)

**Decision.** The resolving-citation check (`RCR-002`) is carried by the qa
crate walking this repository - the checker that already learns a ticket's
checks from its tags - not by the Engine's `record_contract` gate:
`implementation-revision`'s `git:` citation convention is this shop's
working-file contract, while the engine gate stays generic over parseable
citations. Reachability binds any ref, not `HEAD` alone - an edition tag
holds its commit reachable by design, so a record citing a tagged edition
commit must not fail for not sitting on `HEAD`'s first-parent line. The
stamp-landing order (`RCR-001`) is confirmed as the working rule the stamp
step and the turn close (`THK-002`) mechanize.

**Consequences.** The check runs where the records live and refuses at the
archive move (`RCR-003`); the eleven rotted citations found at filing seed
the enumerated allowlist, each row naming its record, hash, and reason, and
a row that matches nothing fails as stale. Post-rule records get no allowlist
path - the archive move is where a new citation is proved. The Engine's
record contract is untouched, so a project without git-hash citations is
unaffected.

## Landed lanes sweep on demand and expire by in-crate marker (ADR-0020)

**Decision.** The sweep is shop-lane tooling under `tools/`, in the shape of
`tools/check_links.py` - one command enumerating `test-hidden/t-*/` in id
order and writing one report artifact under a declared root; the Engine
learns nothing about lanes and no new build target is minted. The expiry
marker lives inside the crate it marks - the wish's own words, an expiry
marker a lane carries - so it travels with the copy-back discipline and dies
with the crate; a tracked summary may cite it but never replaces it. The
sweep runs on demand, with no standing cadence, and the close wiring is what
makes it load-bearing: one `command_exit`-class guard (`LNR-003`) beside
`EDN-002`'s, refusing only on a `red`, unexpired verdict. Expired lanes are
skipped by default, with an explicit verify mode that runs them anyway so a
marker cannot quietly outlive its lane's recovery.

**Consequences.** Rot becomes a dated, visible fact instead of a silent
class confusion: `red` always means a live regression, `expired` names the
last-good edition, and the pre-split crates become the first visible
decision the marker exists to hold. A runbook that declines to read the
verdict stays legal - the wiring is permissive; the verdict's meaning when
read is what binds.

## Turn-close verification delegates to the declared sweep (ADR-0021)

**Decision.** Turn-close verification remains part of the repo-local turn
lifecycle, not the Engine. The generic turn tool executes the declared rerun
command and stays unaware of expiry markers. Its closed, optional
`lanes-rerun-scope` declaration accepts `per-lane`, the existing default when
omitted, or `root-once`, which runs the command once from the primary
checkout. An unknown value refuses before any mutation, and the dry-run plan
names the selected scope. Existing per-lane declarations remain valid.

This repository selects `root-once` and declares a fresh sweep followed by
the sweep's report check. Both commands use
`--report target/turn-close-lanes.md` and both must succeed: the fresh sweep
prevents an old report from passing, while the check refuses missing or stray
crates and distinguishes passing, explicitly expired, and red crates. The
report is ignored so routine close verification does not dirty the tracked
cycle-close report. The existing sweep validator supplies the policy; turn
close adds no marker parser, automatic expiry operation, or sweep
implementation change.

A close that reaches final verification and fails may retry verification only
after cleanup. This branchless path requires all completed-landing evidence:
no item branch, worktree registration, or worktree folder; an item stamp equal
to the current trunk short tip; the supplied landing log line already present;
and no unrelated tracked dirt. Missing or stale proof refuses by naming the
unmet fact and the repair needed. A valid retry reruns the entire declared
verification without repeating landing, copy-back, stamp, log, worktree
removal, branch deletion, or adding a journal entry. Earlier copy-back and
only-copy refusals remain in force.

**Consequences.** Final close confirmation applies the sweep's established
expiry contract without giving the lifecycle a second interpretation of lane
state. Failures identify the declared command or missing retry proof so the
operator can repair and repeat close safely. No Engine source or Machine Class
change is involved.

## Completion fields are selected by typed runbook data (ADR-0022)

**Context.** `CGD-003` requires the completion gate to derive focused,
hidden-lane, and quality checks from declared lists without understanding a
project's document headings or field names. The observable behavior was
accepted before its Machine Class wire shape, narrow declaration grammar, and
self-host migration were chosen. The earlier workflow proved a front-matter
reader with the current ticket fields, while the Engine still hard-codes the
legacy planned-test identity and infers hidden ids and Merge Gate commands
from prose shape.

**Decision.** The exact guard fields, diagnostics, declaration subset, and
refusal order are owned only by the runbook specification's
[Completion declaration mapping](../runbook-spec.md#completion-declaration-mapping).
The mapping is typed, optional as a complete group for parse compatibility,
and fail-closed at completion when omitted; it reuses the guard's existing
root resolution and exact opaque literal-or-bound address rather than adding a
path or suffix.

One generic selected-string-list reader is extracted from the already-proved
workflow QA reader. QA passes its local field selection, and the Engine passes
the parsed runbook selection; neither gets a second parser. The reader owns
only the documented front-matter subset and adds no YAML or other dependency.
The Engine removes the prose-discovery path and hard-codes no project field
name.

This project's forward mapping is `focused-tests`, `hidden-lanes`, and
`quality-commands`, in focused, hidden-lane, quality order. The older
`planned-test-refs` identity is deliberately not the focused mapping: it
belongs to the earlier planning and sensitivity contract and can name a
different set. This is a forward identity cutover, not a historical rewrite.
Archived declarations and receipts keep their original identities. A new
fixture may use the same literal id in planned and focused lists only when it
does so deliberately; no equality is inferred for older records.

**Consequences.** Receipt kinds, files, Run scoping, freshness and
self-consistency checks, sensitivity behavior, and receipt-defect wording do
not change. A paused Run keeps its existing refusal precedence. An unmapped
runbook remains parseable but a new Engine cannot take its completion guard
and does not fall back to prose.

**Rollout.** Self-hosting requires two landings, both inside the implementing
ticket's scope. The capability landing adds the generic Engine behavior and
the fixture ticket's three declaration tags. It also tracks an exact
`.ratmac/completion-guard.diff` that adds the approved mapping to the two
`completion_gate` declarations and changes nothing else: roots, States,
transitions, the edition guard, and the lane-sweep guard stay byte-for-byte
unchanged. Proof applies that patch to the tracked runbook bytes and then
reverses it back to the byte-identical original.

Candidate traversal tests apply that reviewed patch only to their temporary
runbook. They describe the fixture as the *prepared cutover*, not as the exact
shipped Machine Class, and assert separately that the actual tracked runbook
remains unmapped. The fixture ticket's focused, hidden-lane, and quality tags
land with this capability, so the later activation needs only a fixture-source
switch. The candidate also proves two boundaries against the tracked unmapped
runbook: completion refuses for the missing mapping before reading the
addressed artifact, while `rtm doctor` reports no missing-mapping lint.
`RB113`'s Diagnostics-table row and matching runbook-authoring repair row
land with the Engine source that emits the code, preserving the documented and
emitted diagnostic-code parity.

Stable `edition-007` remains unchanged and drives the actual, unchanged
Machine Class through the rest of `run-030`. The candidate must not be
described as able to complete that unmapped runbook, and the existing Run pin
evidence is never rewritten. The next recording landing's edition-ledger claim
is limited to Engine capability at the tagged revision; it does not claim that
this repository's runbook is already wired.

After `run-030` reaches rest, the activation landing applies the already
tested patch to the tracked Machine Class, removes the temporary patch path and
unmapped assertion from the traversal test, and restores traversal over the
exact shipped bytes. The earlier fixture-tag addition means this landing makes
only the runbook activation and test-source switch; it adds no Engine behavior.
If worktree and recording-landing order requires a direct primary-checkout
landing, the ticket records that conflict resolution in the log. The newly
recorded stable Engine is bootstrapped before another Run starts, while the
completed Run's pin evidence remains untouched.

## Wishlist fulfillment decision (ADR-0023)

The dispatch accepts these independently reviewed contracts. Archived evidence and
original Run pins stay unchanged. The stable driver continues on its pinned runbook.
New runbook fields and pause routes are proven against exact prepared bytes and
activated only at a compatible cycle boundary, following the declared-completion
rollout precedent. Capability proof never claims external enrollment was supplied.
Legacy Runs are never silently upgraded to the new authorization contract.

### WCP-003 — selected issue completion

The self-development workflow binds its selected issue and requirement set to its work item, tested source snapshot, failing-before-implementation evidence, passing-after-implementation evidence, and independent review decision. Completion refuses absent, mismatched, stale, fabricated-by-the-working-agent review authority, or unrelated evidence. A writable success marker, unrelated issue's tests, or a passing child alone cannot satisfy this contract. The parent consumes only the properly bound terminal review result; the Engine remains generic and learns no issue document conventions.

Accepted from [selected issue completion](../issue/i-042-selected-issue-completion/spec.md#requirement-records).

#### Accepted mechanics

Carry opaque selected-issue and item bindings into the governed work. Workflow-owned validation resolves them to the selected requirements and permitted check declarations. Bind the review to the exact source/evidence digest; require rejection and repair before proof completion when it changes. Use the independently authorized review contract for admission of review results, and join its Engine-written terminal result only after authority and snapshot checks. No product-specific issue parser is added to Engine source. Trial lifecycle Git operations remain outside the Run.

The mechanics implement [WCP-003](../issue/i-042-selected-issue-completion/spec.md#requirement-records) and are tested by the [verification plan](../issue/i-042-selected-issue-completion/test-plan.md#verification).


### WCP-007 — distributable plan build profile

A distributable Plan-Build profile contains the Machine Class template, executable artifact schemas and validators, issue/gap/work-item blanks, required terms, and operating instructions needed by its guards. A clean repository with no .arca/schema.md and no inherited system prompt can initialize the profile and drive a complete issue-to-proven-completion sprint using only its packaged contract and Engine output. Profile fields and roots are declared data; the Engine gains no project-specific source code. Missing, malformed, or incompatible profile artifacts refuse with actionable guidance. Existing user files are never overwritten silently, and installation is deterministic and offline.

Accepted from [distributable plan build profile](../issue/i-046-distributable-plan-build-profile/spec.md#requirement-records).

#### Accepted mechanics

Create a manifest describing profile format compatibility and its included schemas, templates, checkers, terms, and Machine Class. Export the complete bundle atomically to an absent target using existing non-overwrite behavior. Parameterize repository root roles and executable check commands through declared profile data. Ensure every shape consumed by a contract guard has one machine-readable definition shipped with the profile and no unstated dependency on this repository's schema. Prove the profile by an isolated clean-repository sprint, using explicit checks and review evidence rather than operator narration.

The mechanics implement [WCP-007](../issue/i-046-distributable-plan-build-profile/spec.md#requirement-records) and are tested by the [verification plan](../issue/i-046-distributable-plan-build-profile/test-plan.md#verification).


### WRS-001 — successor ownership

A spawned replacement becomes available to public Run operations only after its owning parent ledger durably records its identity, class, bindings, workspace, and superseded Run. Concurrent step, spawn, hold, abandon, join, and roster reads must never observe an admitted successor without that ownership. Root-before-Run lock order and root-lock-free guard evaluation remain unchanged. Failed or interrupted replacement preserves the predecessor or records a complete recoverable replacement; an ambiguous write never causes deletion of potentially owned evidence, and identifiers are never reused.

Accepted from [successor ownership](../issue/i-048-successor-ownership/spec.md#requirement-records).

#### Accepted mechanics

- Use the existing mint and ledger boundaries in `src/scheduler.rs`, `src/ledger.rs`, and `src/lock.rs`; make publication one root-locked decision instead of releasing an admitted successor before ownership is durable.

- Prepare the replacement and validate the predecessor before destructive retirement. Keep any uncommitted replacement unavailable to public motion; distinguish absent, complete, and indeterminate ledger writes without guessing. Recovery must finish or safely cancel the same recorded operation rather than minting duplicates.

- Retain the confirmed `respawn <run id>` contract, child workspace and binding inheritance, append-only ledger history, and the one-level spawn cap. Do not hold the root lock while running guards.

#### Scope and assumptions

These mechanics implement the accepted dispatch; runtime ownership and historical preservation remain binding.

No dependency on another new issue. Coordinate with rule-fix recovery if it reuses replacement or publication helpers.


### WRS-002 — child class hold

Both planning and applying an addressed hold resolve the blocked route in the Run's recorded owning class. A child whose current State declares a blocked route can be held, even when its State name overlaps the parent or a sibling class; another class's route never substitutes for its own. The hold preserves Run identity, records blocked status and the opaque blocker only in Engine-owned state, and leaves the parent, siblings, workflow files, pins, and previous evidence unchanged. Confirmation, terminal refusal, containment, stale-plan checks, and write-free refusals remain enforced.

Accepted from [child class hold](../issue/i-049-child-class-hold/spec.md#requirement-records).

#### Accepted mechanics

- Expose or reuse one owning-scope route lookup from the Scheduler instead of reading its top-level graph in `src/blocked.rs`.

- Resolve from the durable ledger class during planning and revalidate under the addressed Run lock during apply. Use the child's recorded workspace for blocker containment.

- A missing class, missing own route, ambiguous ownership, or forged/stale plan refuses by name before writing; do not guess from matching State names.

#### Scope and assumptions

These mechanics implement the accepted dispatch; runtime ownership and historical preservation remain binding.

Independent Engine repair. It precedes exercising pause routes on the Plan-Build child class in the cycle-pause issue.


### WRS-003 — cycle pause routes

The shipped Plan-Build Runbook declares exactly one human-confirmed blocked route for each nonterminal cycle and ticket State, preserving that State as its pause destination. A valid hold keeps the Run address, evidence, child ownership, and pins while recording blocked status and a linked blocker in Engine-owned state. Resume verifies exact human resolution authorization bound to that Run, its currently recorded opaque blocker, held-record digest, and unique hold occurrence, together with its unchanged owning class, State, pins, and existing entry prerequisites; under a governed policy the resolution authorization must also be signed through WRA-001. The Engine verifies authorization, not whether an external cause has actually been fixed. It then atomically clears the paused fact and restores executing status without advancing, retaining the hold, blocker, and resolution authorization in history; resume refusal before publication leaves persisted records and history unchanged; an interrupted committed resume reports recovery pending and is completed before further mutation. Resume neither refreshes receipts nor approves a transition: later completion and spawning still require their normal fresh checks. Ordinary step never takes a blocked route, terminal States declare none, and no pause fabricates passing proof. Activation happens only at a compatible cycle boundary and never by rewriting a live Run's pin.

Accepted from [cycle pause routes](../issue/i-050-cycle-pause-routes/spec.md#requirement-records).

#### Accepted mechanics

- Use a blocked self-route for each working State so the graph position survives the hold; keep blocked as a lifecycle status rather than introducing a State called blocked.

- Reuse the existing confirmed hold and opaque blocker rules. A shop annotation on an affected ticket remains a contributor action, never an Engine write.

- Add one addressed, explicit resume operation whose exact human resolution authorization names both the Run and its currently recorded blocker. Under a governed policy, require a signed resolution decision through WRA-001 bound to those same facts and current trusted identities. The human or authorized reviewer owns the external resolution judgment; the Engine verifies the authorization and binding, not the truth of an external cause. A generic claim, an edited issue status, or deletion of the referenced file supplies no authorization.

- Reuse existing entry-prerequisite checks. Do not introduce a new guard language or issue parser to infer blocker resolution. The signed resolution purpose is exactly `ratmac-resume-v1`. Its canonical intent binds the Run identifier, current State, owning class, recorded workspace, effective pinned authority identities, exact opaque blocker, held-record digest, and hold occurrence. The human confirmation binds the same Run and blocker; under a governed policy it does not replace the signed decision.

- Each committed hold receives a monotonically increasing, never-reused occurrence in the addressed Run's Engine-owned append-only `pauses/<sequence>.toml`. Sequence filenames are positive decimal integers without leading zeroes, starting at 1 and ordered numerically; an unavailable next value refuses instead of wrapping. The record preserves the exact held context and its digest. Resuming and then holding the same Run at the same State against the same blocker creates a new occurrence, so a prior signature cannot resume it. No field is added to the public seven-field Run Record.

- Resume revalidates the owning class and pinned authority, takes the addressed Run lock, and compares the held State, exact blocker, record digest, and occurrence against the authorized request. Only after resolution authorization and existing entry prerequisites pass does it clear the live blocker and set executing status at the same State. Preserve the hold, blocker, and verified resolution authorization as historical facts without claiming the Engine proved an external repair. Never consume a verdict, mint a child, refresh a receipt, or advance a State during resume; ordinary operations do those jobs afterward under their existing checks.

- Hold and resume use an Engine-owned operation journal under the addressed Run lock. Prepare the complete old/new Run Record bytes, pause or resolution evidence, and uniquely identified append-only history event before publication. Atomically publish the complete journal entry as the action's commit decision; unpublished temporary preparation is not an accepted action. Before any later mutation, recover an interrupted operation to its unchanged old state when no commit decision exists, or finish all outputs of the committed action exactly once. Readers never repair: while physical outputs need reconciliation, they report the pending operation instead of presenting a half-cleared pause as ordinary state. A refusal before publication leaves persisted records and history unchanged; failure after publication reports a committed action with recovery pending, never a false unchanged refusal. Committed pause records and history are never removed or rewritten during recovery.

- Update the working procedure before activating the Runbook changes. Stage reviewed changes until existing pinned Runs reach a safe boundary; a current Run cannot inherit new routes by silent mutation. A rule-fix amendment preserves the paused lifecycle; it is not implicit resume permission and supplies no passing transition evidence.

#### Scope and assumptions

These mechanics were selected during integration under Billy's 2026-09-22 dispatch. Implementation remains ticketed and must preserve runtime ownership and historical evidence; this issue claims no completed implementation.

Depends on child-class hold for ticket-stage pauses and WRA-001 for signed resolution authorization under a governed policy. Rule-fix recovery is separate and required only when continuing after a pinned rule itself changes. Its authority amendment and this capability's resume authorization remain separate decisions.


### WRS-004 — rule fix recovery

An explicitly authorized Engine recovery can continue a nonterminal Run stopped by a corrected rule without changing its Run address, owning class, bindings, workspace, or current State, deleting its proof, retiring it, or rewriting its original pins. Authorization is verified under the prior trusted policy or a previously enrolled recovery authority, never the proposed replacement policy. The signed authorization binds the Run, full old and new authority identities, and every enumerated change; a confirmation phrase alone cannot authorize replacement. Recovery cannot enroll its own approver. A Run lacking previously enrolled recovery trust requires explicit independent external enrollment before recovery is eligible. The recovery durably records the identities, affected rule, verified authorization, and evidence boundary as a forward-only amendment. It validates the corrected class and State and independently rechecks all affected prerequisites and proof; stale receipts or agent-authored claims never establish success. Missing authorization, incompatible corrections, terminal Runs, corrupt evidence, and interruption leave the old authority effective or one complete recoverable amendment, never a partial silent repin. An amendment does not resume a paused Run or approve a transition.

Accepted from [rule fix recovery](../issue/i-051-rule-fix-recovery/spec.md#requirement-records).

#### Accepted mechanics

- Propose one addressed recovery action, separate from ordinary step and respawn, with an exact human confirmation bound to the Run and proposed corrected identities. Require a signed authority-replacement decision through WRA-001 as well: confirmation alone is insufficient. Verify signatures using the existing trusted policy or an independently pre-enrolled recovery authority, never keys or rules introduced by the candidate correction.

- Bind the signature to the exact Run, current amendment-chain tip, full old and new Engine/runbook/goal/gate/policy identities where changed, current State/class, and the complete enumerated change set. Missing or extra changes refuse. Guard removal or policy replacement requires that same prior authority's explicit authorization; it cannot silently qualify as a harmless correction. Bootstrap enrollment is outside recovery, performed only by an independently authorized operator; an unenrolled legacy Run stays ineligible, with that prerequisite named.

- Preserve original `evidence.toml`, Run history, receipts, and verdict archives. Each Engine-owned append-only `amendments/<sequence>.toml` is a strict version-1 record with these required fields: `version`, `sequence`, `previous-amendment-digest`, `run`, `context`, `old-authorities`, `new-authorities`, `changes`, `affected-proof-boundary`, `signed-intent`, and `signature-identities`. Context binds State, owning class, bindings, and workspace; both authority records give the complete Engine, runbook, goal, gate-artifact, policy, and enrollment identities, using explicit absence only for a historically absent authority. Changes enumerate every differing authority and affected scope. The proof boundary names the evidence invalidated by those changes. Signed intent contains the exact canonical authorized bytes; signature identities retain the verifying enrolled signers and detached signature evidence. These fields extend neither the public seven-field Run Record nor the original pin record.

- Sequence filenames are positive decimal integers without leading zeroes, starting at 1, contiguous, and ordered numerically; a record's sequence must equal its filename. The first `previous-amendment-digest` is the digest of the original immutable pin record; each later value is the digest of the exact preceding final amendment bytes. Unknown or missing fields, duplicate fields or sequence numbers, gaps, noncanonical sequence names, malformed identities, a broken chain, or disagreement between signed intent and record fields refuse. Temporary files use a distinct preparation suffix and are never chain members. Readers validate the complete chain under prior trusted authorization before selecting its effective head; absence of a chain retains the original pin authority.

- The minimal compatibility rule keeps the same State, class, bindings, and workspace; it refuses removed States, rebinding, terminal promotion, unverifiable old identity, or unlisted changes. No general migration language or rewind is proposed. A goal correction invalidates affected completion evidence rather than treating the old proof as fresh.

- Only the Engine writes an amendment. Under the addressed Run lock, prepare the complete validated signed record, then atomically rename it to its unused final sequence path. That final rename is the authoritative commit point: before it the old head remains effective; after it the new verified head is effective even if the subsequent history append fails. Original pins remain byte-identical. Append the uniquely identified amendment history event afterward; retry reconciles a missing event once without minting another amendment or duplicating a complete event. An incomplete history fragment refuses further mutation until the existing append-only recovery boundary can reconcile it, without truncating history. A post-publication failure reports committed amendment with history pending, never claims the amendment was refused unchanged. Readers ignore temporary preparation and validate final chain members but never write repairs.

- Every governed motion reads the latest verified amendment chain and demands authorization and proof bound to that effective head. Earlier receipts remain historical; an unchanged source digest alone cannot refresh evidence bound to an older authority head. Recovery changes no State or paused lifecycle. An explicit authorized resume remains separate when the Run is paused, and later motion requires fresh checks under the corrected authority. Preserve provenance for removed checks instead of laundering their stale receipts into current passing proof.

#### Scope and assumptions

These mechanics were selected during integration under Billy's 2026-09-22 dispatch. Implementation remains ticketed and must preserve runtime ownership and historical evidence; this issue claims no completed implementation.

Requires WRA-001 in issue i-062 for independently trusted authorization of authority replacement, including prior-policy validation and external enrollment prerequisites. Keep independent from respawn and from authorized resume: recovery preserves the original Run address and paused lifecycle. Child-class hold is required only for the composed-child recovery tests.


### WRS-005 — descriptive run roster

Human-facing Run roster output identifies every canonical address together with its recorded top-level or child role, parent and class for children, recorded binding values when present, and State and lifecycle status when readable. Retired or unreadable records are labeled explicitly rather than silently omitted or guessed. Ordering is stable, metadata comes only from Engine-owned records, and a missing address still refuses instead of implicitly selecting a Run. Listing is read-only and needs no current runbook parse, guard evaluation, or pin rewrite.

Accepted from [descriptive run roster](../issue/i-052-descriptive-run-roster/spec.md#requirement-records).

#### Accepted mechanics

- Keep the canonical identifier listing used internally separate from a human-readable roster renderer. Read the per-Run records and parent ledgers once for the report and use their stored facts, without interpreting an opaque binding as a ticket title.

- Use consistent rendering in missing/invalid-address diagnostics and the roster printed by status. Show unknown or unreadable facts by path and reason; an unreadable ledger must not license describing an uncertain child as top-level.

- Preserve stable ordering and escape control characters in rendered opaque values. No new persistent display name, database, external lookup, or lifecycle command is needed.

#### Scope and assumptions

These mechanics implement the accepted dispatch; runtime ownership and historical preservation remain binding.

No new hard dependency. Shares read-only historical-record handling with terminal-history status; successor publication must not expose an ownership-free row.


### WRS-006 — terminal history status

Addressed status can report a strictly parsed persisted passed Run's identity, State, status, and recorded evidence identities even when the current runbook differs, is unavailable, or cannot parse. This historical view reports that current instructions are unavailable and never renders changed prompts or guards as the old Run's instructions. It neither evaluates guards nor mutates records, receipts, pins, ledgers, history, or locks, and never suggests retirement merely to read a completed Run. Nonterminal motion and reporting retain their existing pin checks; corrupted records still refuse by name.

Accepted from [terminal history status](../issue/i-053-terminal-history-status/spec.md#requirement-records).

#### Accepted mechanics

- Add an explicit read-only terminal-report path before class-dependent Scheduler opening. Use recorded status rather than the changed graph to recognize completed history; validate canonical addressing and residue boundaries first.

- Render persisted facts and recorded identity fields without inventing missing old prompts. Keep the hash-only pin design; this fix needs no reconstructed or per-Run runbook copy.

- A completed Run cannot regain motion through this read path. Preserve existing live-Run drift refusal and terminal step/hold refusal; malformed or unavailable evidence is named and never silently described as verified.

#### Scope and assumptions

These mechanics implement the accepted dispatch; runtime ownership and historical preservation remain binding.

Independent from rule-fix recovery: reading completed facts authorizes no new motion. Coordinate roster formatting for consistent role and uncertainty labels.


### WRS-007 — abandon confirmation hints

All live abandonment hints, examples, and malformed-option diagnostics distinguish addressed retirement from unaddressed leftover-lock cleanup. For an addressed Run they give the exact `abandon <run id>` phrase independent of option order. When an address is missing while admitted Runs exist, the diagnostic first requires `--run <id>`, shows the roster, and teaches an addressed example rather than offering a project-name phrase that cannot retire those Runs. Only a true no-admitted-Run leftover-lock path teaches the project-name phrase. No diagnostic performs retirement or chooses a Run implicitly; confirmation and refusal safety remain unchanged.

Accepted from [abandon confirmation hints](../issue/i-054-abandon-confirmation-hints/spec.md#requirement-records).

#### Accepted mechanics

- Keep `src/abandon.rs::required_phrase` as the phrase authority and make diagnostics use validated request context. Parse addressing before generating phrase-specific option hints; do not invent an address from a single-entry roster.

- Update module examples, command diagnostics, and working instructions together at implementation. Preserve the existing no-live-Run leftover-lock behavior and residue preflight precedence.

- No new confirmation source, bypass flag, or automatic approval is introduced. Displaying the proper command is guidance, not evidence that a human authorized it.

#### Scope and assumptions

These mechanics implement the accepted dispatch; runtime ownership and historical preservation remain binding.

Independent. Reuse the descriptive roster renderer if it lands first, without requiring that presentation change to fix the phrase.


### WEB-001 — portable goal fingerprints

Goal baselines and freezes for newly started Runs use one versioned, deterministic serialization: relative paths use forward slashes, path ordering is bytewise and case-sensitive, records are length-delimited, and CRLF pairs in valid UTF-8 text without NUL bytes become LF while all other content bytes retain their meaning. Identical relative paths and canonical content produce identical revisions across platforms; added, removed, renamed, or semantically edited files change the revision. Binary content is hashed unchanged. Existing unversioned fingerprints keep their original algorithm and semantics for live Runs, including a legacy baseline first frozen after the upgrade; archived stamps stay byte-identical and parseable. An unknown version or an unrepresentable path refuses by name without updating evidence. No existing pin is silently upgraded or accepted merely because it matches either of two algorithms.

Accepted from [portable goal fingerprints](../issue/i-055-portable-goal-fingerprints/spec.md#requirement-records).

#### Accepted mechanics

[goal.rs](../../src/goal.rs) sorts displayed relative paths but hashes raw bytes with newline delimiters. [pin.rs](../../src/pin.rs) stores baseline and frozen revisions as strings; [contract.rs](../../src/contract.rs) consumes the frozen citation. Goal freeze and drift remain the authority in [goal specification](spec.md).

The new revision string is exactly `v2:` followed by 64 lowercase hexadecimal SHA-256 digits. The existing compound citation carries it as `goal-sha256:v2:<digest>`; its other fields retain their existing grammar. A bare 64-digit digest selects the legacy algorithm. All revision producers, comparison consumers, working-record parsers, and proof helpers adopt this grammar together before new stamps are minted. No mandatory field is added to historical records.

The hashed serialization begins with the ASCII bytes `ratmac-goal-v2` followed by one NUL byte, then an unsigned 64-bit big-endian file count. Each file entry contains, in order: an unsigned 64-bit big-endian path-byte length, the UTF-8 relative-path bytes with forward-slash separators, an unsigned 64-bit big-endian canonical-content-byte length, and those content bytes. No delimiter, terminator, padding, or other byte is added. Sort entries by their relative-path bytes in ascending bytewise order before serializing; reject unsupported path encoding instead of using lossy conversion. Preserve case and Unicode code points; do not promise equality for trees whose names differ. Reject any count or length that cannot fit the declared unsigned 64-bit field.

Content is text exactly when it is valid UTF-8 and contains no NUL byte. For that text, replace each CRLF byte pair with LF and change nothing else: preserve lone CR, byte-order marks, and final-newline presence. Every other file is binary and contributes its raw bytes unchanged, including any CRLF pairs. Empty directories contribute no entries; an existing empty goal directory has the zero-file serialization, while an absent goal retains the existing absent-goal behavior. A non-regular file or link whose contents cannot be represented safely refuses rather than following a host-specific target.

Use the recorded version at every drift comparison. A Run whose baseline predates the change keeps that version through its freeze; a newly started Run uses the new version. Historical goal stamps are provenance, not a request to recompute them with today's algorithm. Document how an operator starts a new portable Run without rewriting an old Run or archive.

Scope includes the fingerprint producer, comparison consumers, citation parsers, and their documentation. It does not change runbook byte pins, executable hashes, receipt-output hashes, or snapshot hashes: those attest exact bytes.


### WEB-002 — engine ignore initialization

A documented, idempotent initialization command installs the Engine-root tracking policy before any Run can be admitted: runs/, mint.toml, locks/, and log.md beneath the resolved Engine root are ignored, while ratmac.toml and evidence/ remain eligible for tracking. Initialization preserves unrelated bytes and compatible operator rules, refuses an incompatible effective rule with the exact path and repair, and leaves no partial initialization on failure. Already-indexed runtime files and failures inspecting an available repository's index or effective ignore rules refuse without mutation; they are not treated as absent Git. Repeating it with the same effective policy is a byte-identical no-op. Linked worktrees protect the shared runtime root and their own tracked authoring surfaces. A fresh project's first start cannot create unprotected runtime; no command silently stages, commits, changes global Git configuration, or overwrites a runbook.

Accepted from [engine ignore initialization](../issue/i-056-engine-ignore-initialization/spec.md#requirement-records).

#### Accepted mechanics

[scaffold.rs](../../src/scaffold.rs) deliberately creates exactly one runbook file and no directories. [root.rs](../../src/root.rs) distinguishes the invoking checkout from the shared Engine root. [schema Engine-root tracking policy](../schema.md#ens-012--engine-root-tracking-policy) requires ignoring live runtime while tracking runbook and evidence.

Assumed: add a small `rtm init` command dedicated to Engine storage initialization. Keep `scaffold`'s one-file contract. Initialization creates only the necessary Engine directory and an Engine-local `.gitignore` containing anchored runtime entries; it does not create a Machine Class implicitly. Teach the next scaffold command after success.

For an existing local ignore file, preserve all bytes and append only missing narrowly scoped lines after validating the resulting effective policy. Do not add blanket `.ratmac/` exclusions. An operator's broader inherited rule or a negation that makes runtime visible is a conflict, not permission to rewrite the shared repository ignore file. A refusal names the effective conflicting path and asks the operator to repair that rule. A pre-existing runtime file already in Git's index must also refuse; an ignore pattern cannot untrack it.

In Git repositories, check effective ignoring for representative runtime and authoring paths, including root-level and local rules and the initialized ignore file itself. An absent Git executable or a directory established to be outside a Git repository allows local policy installation with a clear statement that repository-level conflicts cannot be verified; it must not claim to have verified Git behavior. A corrupt repository, an unexpected failed Git query, or inability to inspect its index or effective rule result refuses rather than being classified as no Git. For a linked checkout, install/check the shared root's runtime policy and ensure the invoking checkout's runbook/evidence are not excluded.

Make `start` perform the same idempotent policy preflight before admission, so the existing simple first-start flow cannot bypass protection; plan and validate before mutation and roll back new ignore changes if admission fails. Initialization must serialize concurrent attempts without overwriting another writer. Include this command in residue preflight and help coverage. The accepted goal must explicitly permit this Engine-local ignore file and directory creation; no workflow-root write is needed.


### WEB-004 — residue refusal precedence

For every public operational command and equivalent library entry point, retired-layout residue at the invoking/shared or explicitly addressed project is refused before command-specific option validation, runbook parsing, roster reads, blocker lookup, target existence checks, joins involving runtime data, or mutation. Only the minimal command/target identification, Engine-root resolution, and filesystem inspection required to locate residue may precede that refusal. Thus a recognized command with malformed options reports residue first when residue exists; without residue it reports the ordinary option error. Only documented exact standalone-help forms and an unknown command perform no operational action and remain pure usage responses. All refusals leave the tree byte-identical. A help token mixed into an operational request cannot bypass preflight. A single maintained entry-point matrix covers every public route, including new initialization, recovery, authorization, and nested-protocol commands, and exposes a newly added uncovered operational route.

Accepted from [residue refusal precedence](../issue/i-058-residue-refusal-precedence/spec.md#requirement-records).

#### Accepted mechanics

[cli.rs](../../src/cli.rs) routes doctor before the common residue preflight and parses doctor's options before resolving its target. Scaffold and skill have separate path-taking boundaries. [scheduler.rs](../../src/scheduler.rs) owns refuse_flat_residue and several workspace-specific checks; [scaffold.rs](../../src/scaffold.rs) already preflights before target existence.

Assumed: the precedence order is target discovery, root resolution, residue inspection, then option and operation validation. Exact pure help forms are no arguments, a sole `--help` or `-h`, or a recognized command followed only by `--help` or `-h`; the optional leading executable name accepted by the library wrapper does not change this rule. A help token mixed with other operational arguments is not a bypass and is validated only after residue preflight. Unknown commands remain write-free usage refusals.

For target-taking commands, perform a non-validating scan that identifies the first positional target without interpreting malformed options as paths. Check invoking and clearly addressed roots before reporting later argument defects; if there is no unambiguous target, check the invoking root and then report usage. Document that ambiguity rather than inventing an addressed root. Resolve each relevant project once using the context established by the single-root issue.

Centralize the preflight entry boundary and require an already checked context in command handlers. Public path-taking library wrappers construct that context before any handler-specific reads. Preserve the named legacy-folder ownership exception and all existing residue classes; this issue does not introduce migration or rewrite history.

Inventory the exported Engine operations and command dispatch, including initialization, recovery, review-intent or authorization operations, and the nested launch/receipt endpoint as they land. Drive one parameterized matrix over malformed options, missing addresses, occupied target paths, bad confirmations, and missing blockers, with each known residue shape independently present. Add an observer or injected operation boundary in tests to prove roster/read/parse/write handlers are not called; byte snapshots alone do not prove read ordering. The inventory must have a completeness check against the public route table so a new verb cannot omit the matrix silently.

Confirmed retirement currently intentionally bypasses some live-run checks. Integrate the ruling explicitly: it may still bypass damaged runbook pins to recover a Run, but retired-layout residue remains the universal preflight and never triggers an implicit repair.


### WEB-005 — single root resolution

An Engine invocation resolves each distinct addressed project exactly once and carries that immutable result through residue checks, address selection, scheduler binding, workspace validation, command execution, and reports. The invoking checkout still supplies its own runbook; linked worktrees still share the primary runtime root. A context-bound handler cannot substitute a path-taking resolver or an independently rendered root. Public convenience entry points each establish one context and use context-taking internals. A test can count resolutions and make a second resolution return a different answer, proving every route uses the original value. Distinct explicitly addressed projects may each have one cached result; workspace membership checks must not re-resolve an already known Engine root.

Accepted from [single root resolution](../issue/i-059-single-root-resolution/spec.md#requirement-records).

#### Accepted mechanics

[root.rs](../../src/root.rs) already defines Roots with private paths but exports both Roots::resolve and resolve. [cli.rs](../../src/cli.rs) mixes roots-taking and path-taking helpers. [scheduler.rs](../../src/scheduler.rs) repeats resolution through public opens, residue wrappers, and workspace validation; status comments admit current tests cannot detect replacing the roots-taking path.

Assumed: promote the existing Roots value into the invocation's authority instead of adding a global cache. One owner at dispatch resolves it. Context-bound scheduling, diagnostic rendering, residue checking, and helpers receive a reference or owned clone; they never accept a raw root as a substitute for the context. Retain public path-taking compatibility wrappers as fresh entry points, but keep them out of the internal handler API.

Restrict the production resolver's visibility to entry construction. Separate pure formatting from discovery; reports derive their path only from the same context that opened the Run. For an addressed workspace, validate canonical membership against the context's repository identity rather than starting another root search. A genuinely different explicit target gets its own context, cached per invocation by canonical project identity.

Introduce a small injected resolver boundary for tests, with a call counter and scripted results. Production still uses the existing Git/fallback logic, without a dependency or runtime global. The test double returns a different root or errors if called twice; exercise full dispatch and library entry wrappers so any accidental duplicate is visible even when ordinary Git would answer identically.

Coordinate first with residue precedence. Nested independent invocations get separate contexts and do not inherit the parent's runtime root; they are separate Engine entries, not extra resolutions hidden in one handler.


### WEB-006 — safe nested invocations

A declared nested invocation executes a pinned child program using a separate Engine root and returns, through the parent's dedicated captured pipe from the pinned child Engine endpoint, an Engine-produced terminal receipt bound to that invocation, child Run, class pin, Engine identity, and terminal outcome. The parent advances only on the current child's verified successful terminal receipt; process exit zero, skill-wrapper output, agent-authored receipt files or markers, stale/replayed receipts, failed/held/nonterminal Runs, and mismatched roots or identities cannot pass it. No level reads or writes another level's runtime state as its own. The Engine propagates and validates lineage across nested program boundaries; a repeated declared program/class identity, repeated canonical Engine root, malformed inherited lineage, or maximum-depth overrun refuses before invoking or admitting the child and names the offending lineage. Distinct programs with distinct roots may use the same Engine executable; its digest authenticates the executable and is not a recursion identity. This lineage boundary governs cooperating pinned programs and does not claim to prevent a hostile program from discarding its environment and launching an unrelated top-level process. Ordinary same-root spawn remains its existing one-level composition model, distinct from independently rooted program calls.

Accepted from [safe nested invocations](../issue/i-060-safe-nested-invocations/spec.md#requirement-records).

#### Accepted mechanics

[scheduler.rs](../../src/scheduler.rs) evaluate_command_exit executes a pinned or exempt program and judges its exit code, capturing diagnostics. Its existing spawn ledger and one-level child cap cover shared-root Runs only. [root.rs](../../src/root.rs) intentionally shares runtime across Git worktrees, so a linked worktree is not an independent nested root by default.

Assumed: use an explicit nested-program declaration and a narrow Engine-controlled launch/receipt interface. Do not reinterpret every command_exit process as a child Run, and do not use trace output as evidence. The child target must have a genuinely distinct resolved Engine root; another worktree of the same repository refuses this nesting mode unless the governing root policy explicitly provides isolation.

The parent launches the pinned child Engine protocol endpoint directly, passing the declared program/class/root and an Engine-minted invocation address. The parent owns a dedicated captured pipe from that endpoint. A wrapper skill may drive the child's allowed work, but the endpoint redirects that driver's stdout/stderr separately and does not pass its receipt pipe to the driver or its descendants. The child Engine emits the receipt on its dedicated pipe only after persisting and reading back its own terminal fact. Neither arbitrary wrapper output nor an agent-writable receipt file is accepted as this channel's substitute.

The parent verifies a strict envelope against the expected invocation, child Run, pinned Engine and class, declared root, terminal sequence, and successful outcome; malformed, duplicate, partial, and unsolicited records refuse. For governed children, bind the effective enrollment/policy and authenticated terminal-transition evidence as well, preserving the authorized-review boundary instead of treating a writable signer field as authority. Record the validated receipt under parent-owned evidence without rewriting child-owned records. Define receipt creation/consumption and crash retry so a lost response cannot mint a second child or accept an earlier invocation. Process provenance relies on the pinned endpoint and existing Engine-owned-state boundary; it is not a claim that an attacker allowed to rewrite every runtime record has been defeated.

Propagate the complete lineage in the parent-controlled endpoint protocol and in the process environment inherited by the driver. Every nested endpoint requires those representations to agree before admission; missing, malformed, or truncated context cannot silently become a new top-level call. Use the declared program/class identity and canonical root identity to detect aliases, symlink spellings, and a chain returning through a different binary name. Distinct programs with distinct roots may use exactly the same Engine executable: its digest is a trust pin, never the program identity used for cycle detection. Freeze a documented finite default maximum depth, proposed 16, and allow only a tighter runbook limit. The safety boundary is cooperating pinned programs; environment propagation alone cannot stop a hostile program from discarding that environment and launching an unrelated top-level Engine. No broader security claim is made.

Normal guards remain pinned and cannot rebuild code during evaluation. Parent and child locks cannot form a cycle: reject shared roots and recursion before launch, and never hold the shared root lock while waiting for a child. Parent refusal or cancellation must leave a named resumable child outcome and never invent completion.

Integration must specify the Machine Class fields and terminal receipt schema in the single runbook authority, plus operator guidance for nesting; exact command spelling is a design choice, not permission to omit the black-box receipt contract.


### WEB-007 — deterministic trace channel

One module owns Engine stderr output and an environment-enabled decision trace. The trace is off by default, writes only to stderr, creates no file or persistent state, and performs no formatting or allocation per disabled event. Enabled events cover root selection, transition-input edge selection, guard refusal, lock acquisition/release order, and mint read/write decisions. Each structured record carries a stable event/diagnostic code, reusing the refusal code where one exists, and deterministic fields with no timestamps, durations, absolute host paths, arbitrary child output, secrets, or nondeterministic identifiers. The same normalized input/state/decision sequence produces identical structured trace bytes; these restrictions do not erase or rewrite the established mandatory-error text. Existing stderr sites are routed through the owning module; mandatory errors remain visible when tracing is off. No guard, gate, receipt, or hidden behavioral lane consumes trace text as evidence.

Accepted from [deterministic trace channel](../issue/i-061-deterministic-trace-channel/spec.md#requirement-records).

#### Accepted mechanics

[scheduler.rs](../../src/scheduler.rs) contains direct stderr diagnostics around recovery and lock/fault paths; [Engine entry point](../../src/bin/rtm.rs) writes mandatory errors directly. [cli.rs](../../src/cli.rs) writes normal reports through its provided output writer. Existing compiled fault points are used for deterministic concurrency and recovery testing.

Assumed: `RATMAC_TRACE=1` enables tracing; absent, empty, or `0` disables it, and another value returns a clear usage diagnostic without enabling it. Read this setting once while building invocation context. The small enabled flag is transient configuration, not persisted Engine state or a global mutable event log. The no-allocation requirement covers every disabled trace call; initialization must avoid an allocation when the variable is absent and account separately for reading an explicitly present setting.

Add one module for mandatory diagnostics and optional structured events. Route existing direct stderr writes through it rather than gating away errors users currently need. This distinguishes a required error from an optional decision record while keeping one writer. Preserve established mandatory-error text, including its actionable paths; the deterministic-byte and no-absolute-host-path restrictions apply to structured trace records only. Give those records an unambiguous format so tests can separate them from mandatory diagnostics without changing either meaning. Use a fixed record schema, deterministic field order, escaping, and a fixed event vocabulary; where a refusal has no stable code yet, assign one in its owning error contract and reuse it rather than manufacturing a trace-only guess.

Use root roles and repository-relative paths. Do not copy free-form error/child output, executable absolute paths, environment values, timestamps, process IDs, or lock-owner nonces into trace fields. Represent external paths by a declared role or redacted marker. Stable Run addresses and declared state/input names may appear only when the same input state determines them. Trace-write failure cannot alter a guard verdict or runtime transition; mandatory error handling retains its existing semantics.

Keep compiled pause points: they control timing for fault tests, whereas trace only observes. Hidden behavioral lanes must judge real state, receipts, exits, or outputs already in the product contract, never trace prose. Dedicated public trace-format and routing tests may assert the structured record contract, including an allocation counter around a disabled event; this is output verification, not gate evidence. Guards must not inherit an Engine trace channel as their proof input, and captured child trace cannot contaminate accepted receipt channels.

The implementation inventory must include every direct stderr write in src/, including binary entry points, so no parallel diagnostic owner survives.


### WRA-001 — authorized review transitions

A review-governed Run changes State only after the Engine verifies the required independent group reviews and its designated Approver's authorization of the exact fresh transition intent. Unknown signers, a group member without approval authority, a contributor's unsigned claim, missing review, changed work, altered policy, and replay all refuse before state, verdict consumption, or history mutation. Both ordinary and blocked transitions obey the same rule; reviewer roles remain distinct from mechanical guards.

Accepted from [authorized review transitions](../issue/i-062-authorized-review-transitions/spec.md#requirement-records).

#### Accepted mechanics

Add a generic optional review-policy declaration to the single Machine Class
parser. It describes public identities and roles, the distinct-review count,
and the pinned signature verifier. It has no project, ticket, or issue names.
Strict parsing and doctor findings must reject malformed policies.

Those declarations select an externally enrolled policy; they do not establish
their own trust. Enrollment is controlled outside the builder's write and
signing authority and binds the builder's actual key identity, permitted review
keys and roles, policy digest, and verifier executable digest. Admission checks
the declaration against that enrollment. Relabeling the builder key, replacing
the verifier, or supplying a self-signed policy cannot pass admission. The
implementation must state the host mechanism that protects enrollment rather
than claim an agent-writable file or an unkeyed hash is an authority boundary.

The selected production verifier is the host's OpenSSH `ssh-keygen -Y verify`
with Ed25519 detached signatures, exact enrolled identities, and namespaces
`ratmac-review-v1`, `ratmac-approve-v1`, `ratmac-resume-v1`, and
`ratmac-recover-v1`. It receives canonical
length-framed payload bytes on standard input and is judged by exit status,
never diagnostic prose. Verification also checks its enrolled executable path
and digest; `check-novalidate` is not an authorization operation.

An operator-controlled protected trust origin supplies the enrollment manifest.
An external path alone is insufficient: the implementation must validate its
protected ownership/write boundary or an independently anchored enrollment
signature. The manifest binds project scope, builder actor and all its keys,
reviewers, Approver, minimum count, permitted role combination, and verifier.
Neither an environment override nor a runbook declaration may replace that
origin. No enrollment or signer private material is supplied by this dispatch;
live activation therefore remains an explicit external prerequisite while
implementation and isolated verification proceed. The Engine must refuse
missing enrollment rather than generate a substitute identity.

The selected protection mechanism is the host's ownership and access control,
not a new secret store. The explicitly addressed manifest and every ancestor
must be protected against the actual builder operating-system identity,
including replacement and permission changes. Unix enrollment is root-owned,
with no builder, group, or other write permission along the path. Windows
enrollment is owned by System or Administrators; effective builder-token access
must exclude writing, deletion/replacement, changing ownership, and changing
permissions along the path. Builder ownership, an elevated builder capable of
taking ownership, reparse/symlink paths, and unverifiable protection refuse.
The manifest explicitly binds that builder's operating-system identity and
project scope. Its allowed-signers file is protected by the same ownership and
access checks, pinned by digest, and passed directly to the verifier; an
agent-writable temporary signer file is never substituted for it. Identity/key
entries must agree exactly with the enrolled role map. Tests may inject a trust reader into an isolated verifier unit;
the shipped command has no test override, environment bypass, or self-enroll
command. Actual host-enrollment acceptance is never claimed from that injection.

Canonical intent bytes begin with `ratmac-authorization-v1` and one NUL byte.
Each following field is its unsigned 64-bit big-endian byte length followed by
its UTF-8 bytes, in this fixed order: purpose, project scope, Run id, owning
class, workspace, binding digest, next authorization sequence, hold occurrence,
held-record digest, current State, target State, input,
blocker, Engine identity, runbook identity, goal identity, policy digest,
enrollment digest, builder actor, current declared-work digest, previous
amendment digest, proposed-authority digest, change-set digest, review-set
digest. Inapplicable fields are empty, never omitted; integers use canonical
unsigned decimal without leading zeros. Authorization sequences are Engine-owned,
monotonic, never reused, and bound to the addressed Run; resume also binds the
specific committed hold occurrence. Review-set digests cover the sorted distinct
reviewer identities and exact accepted signature bytes using the same length
framing. Approval cannot reuse a review-purpose signature. The Engine recomputes
the complete intent under the Run lock immediately before consumption.

Expose the canonical transition intent for an external reviewer to inspect
and sign. The scheduler derives it from authoritative Run state, the pinned
policy, the selected transition, and explicitly declared evidence roots.
Reading an intent is not approval. The signer writes a detached signature
outside Engine-owned state; the Engine verifies it against declared public
material immediately before transition consumption and mutation.

Route normal and blocked movement through the same verification boundary.
Commit accepted authorization evidence with the transition's existing
durability discipline. Refusal, interruption, duplicate submission, and replay
must leave a reconstructible pre-transition or completed state.

Use a test verifier only in isolated fixtures, with an explicit non-production
identity; live adoption requires an independently controlled signing identity.
Positive and negative acceptance checks also exercise the actual production
signature verifier with isolated, distinct fixture keys; a fake verifier is
insufficient evidence. Enrollment/configuration tampering, same-key aliases,
and recovery-policy replacement receive explicit negative controls.
The workflow's selected-issue completion change consumes this authority
contract and does not invent a separate approval format.

#### Observable contract


- Membership, approval authority, and review count are explicit, strict, pinned
  declarations. The approver must be a group member; duplicate identities,
  invalid verification keys, impossible counts, and unknown fields refuse.
- The approval binds Run identity, transition sequence, source and destination,
  selected input, frozen policy, and the declared work's current content digest.
  Changing any bound value requires a fresh approval.
- Missing or invalid authorization consumes nothing. Successful authorization
  travels into immutable Run-local transition evidence with the transition.
- Private signing material is never generated, stored, requested, or discovered
  by the Engine. Public keys and a claimed signer name alone cannot authorize.
- A declared review policy applies to every State change, including a hold.
  It cannot be waived by a flag, a writable receipt, or a guard's prose.
- An externally controlled enrollment binds the builder identity, review-group
  public keys, approver, policy digest, and verifier executable digest before
  the Run is admitted. A builder-authored runbook cannot enroll its own trust
  anchor. Membership compares public-key identity as well as labels, so a
  second name for the builder's key is still the builder.
- Any rule or review-policy amendment requires authorization under the prior
  trusted enrollment or its already-enrolled recovery authority, even when it
  keeps the same State. The signed intent enumerates the old and new authority
  identities and changed scope. Recovery cannot bootstrap or weaken trust.
- Historical Runs and their evidence remain readable. Compatibility must be
  explicit; an older Run must never be reported as having authenticated review.

#### Assumptions and boundaries

Assumed: use offline public-key signatures and a pinned verifier already
available on the host, rather than inventing cryptography or contacting a
service. The design must verify that verifier's supported protocol before
choosing it. No new package is authorized merely by this proposal.

Assumed: one independent review and the designated approver's signature are
the minimum policy. A policy may require more distinct reviewers. The builder
identity and every key enrolled to that actor cannot count toward independent
review or act as Approver. An external Approver may also supply one independent
review when the enrolled policy expressly permits that combination; review and
approval remain separate signed purposes.

Assumed: policy adoption is explicit and forward-only. Existing no-policy
Runs keep their historical contract and are not retroactively called reviewed;
the new governed workflow must actually enable the policy before this wish is
closed. Supplying its independent trust anchor is a deployment prerequisite,
not something an agent may fabricate to make a test or a live Run pass.

Threat boundary: signing keys are controlled outside the builder's authority.
An actor who can steal every signer's private key or replace the executable
is outside this verification boundary. An agent-writable digest or a key
generated by the builder is not independent authorization.

Design: [proposed mechanics](../issue/i-062-authorized-review-transitions/design.md#proposed-mechanics).
Proof: [verification checks](../issue/i-062-authorized-review-transitions/test-plan.md#verification).
