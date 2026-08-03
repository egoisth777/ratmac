# 04 — Run identity and state residency

Wave-2 research, topic 1 of 3. Date: 2026-07-29. Scope: how the Engine stops equating "the Run" with "the directory." Everything below is split into **Today** (what the code does, with `file:line` citations) and **Proposed** (what could change). Nothing here is a decision; ADR authority stays with `.arca/goal/design.md`.

Motivating problem (owner's framing): even with a generic engine that parses any `ratmac.toml` transition table, there is only ONE transition table and ONE `state.toml` on disk. Parallel execution — worktrees, branch-based loops — is not easily realized because Run identity is directory identity.

---

## 0. Verdict in one paragraph

Run identity is directory identity today because exactly one function decides where state lives — `RunArtifacts::for_root` (`src/model.rs:86-93`) — and exactly one predicate decides whether a Run exists — `arca.join("state.toml").exists()` (`src/scheduler.rs:242`). Both are trivially parameterizable by a run id; the in-memory model (`Runs`, `src/model.rs:166-168`: "A plural collection of independent Runs; it has no singleton or identity policy") and the ADRs (ADR-0007 `.arca/goal/design.md:55-61`, ADR-0008 `:63-74`) already anticipate the move. The real work is not the id plumbing; it is (a) state residency — run state must become git-ignored scratch under the primary checkout, which requires relocating the engine's log writes out of the git-tracked `.arca/log.md`; (b) lock decomposition — the shared-log rollback protocol (`src/scheduler.rs:266-279`, `417-431`) is single-writer-only and becomes corrupting under concurrent runs; and (c) runbook pinning — per-run state residency does **not** close the ETB-001-shaped hole that a merge can swap `.arca/ratmac.toml` mid-Run; that needs a content pin, and the pin machinery already exists (`src/pin.rs:5-6`, `44`).

---

## 1. What the code does today

### 1.1 Root = cwd, verbatim

- `src/bin/rtm.rs:5-11` — `project_root` is `env::current_dir()`; failure exits 1. Passed untouched into `ratmac::cli::run_from(args, project_root, …)` at `src/bin/rtm.rs:19`.
- `src/cli.rs:90-104` — `run_from` takes `project_root: impl AsRef<Path>` and threads it into every subsystem. There is no `--root`, no `--run`, no env lookup anywhere in the CLI.
- `src/cli.rs:142` — `Scheduler::open(&project_root)` for `start | status | step` (dispatch at `src/cli.rs:135`).

### 1.2 Artifact paths: one flat set per directory

- `src/model.rs:86-93` — `RunArtifacts::for_root(root)` = `{root}/.arca/state.toml`, `{root}/.arca/log.md`, `{root}/.arca/rtm.lock`. This is the **only** constructor (`src/model.rs:155-158`, `with_artifact_root`).
- `src/state.rs:56` — `StateStore` independently re-derives `root.join(".arca/state.toml")`. Atomic write via temp file `.state.toml.tmp-{pid}-{sequence}` (`src/state.rs:106`) then replace (`src/state.rs:124-130`).
- `src/pin.rs:16-17`, `160-162` — Run evidence is `root/.arca/evidence.toml` (`EVIDENCE_FILE`, `evidence_path`).
- `src/cli.rs:329`, `403`, `425` and `src/abandon.rs:108`, `110`, `161` also join `.arca/…` paths directly. Path knowledge is duplicated across at least five modules; that duplication, not the singleton itself, is most of the migration cost.

### 1.3 The Run record has no identity field

- `src/model.rs:194-202` — `RunState` is exactly seven fields: `phase`, `status`, `goal_revision`, `input_revision`, `output_revision`, `active_refs: Vec<String>`, `blocker`. No id, no name, no created-at. Unknown fields are hard errors (`src/state.rs:80`), so adding a field is a format break for old files.
- `src/graph.rs:100-105` — `MachineGraph` holds phases + transitions only ("Lifecycle information is deliberately not represented here"); `src/graph.rs:152-156` — `MachineState` is just a `Phase`. The graph layer is already identity-free and needs zero changes for N runs.

### 1.4 The singleton refusal and the lock

- `src/scheduler.rs:227-298` — `start`: re-reads the Machine Class from `.arca/ratmac.toml` (`:233` via `load_class`, `:202-212`), takes the invocation lock at `.arca/rtm.lock` (`:238-239`), then the admission check:
  - `src/scheduler.rs:240-246` — "A canonical State File is the durable marker for an active v1 Run. The invocation lock above is transient and is not used for admission." — `if arca.join("state.toml").exists()` → `"cannot start: an active Run already exists for this project"`.
- `src/scheduler.rs:105-173` — `InvocationLock`: create-new-file acquire (`:131-136`), retry loop bounded by `MAX_ATTEMPTS` (`:138-170`), released by RAII `Drop` (`:173`). A legacy lock file bearing the pre-rebrand name is refused, never deleted (`:110-129`), re-checked after acquire (`:144-148`).
- Every mutating entry point re-acquires the same per-directory lock: `step` (`:310-311`), hold/blocked (`:821`), load (`:829`, `:839`), via `invocation_lock_with_retry` (`:794-800`). `abandon` retires it as part of cleanup (`src/abandon.rs:110`, `:161`).

So: **admission** = existence of `state.toml`; **mutual exclusion** = transient `rtm.lock`; both keyed solely on the directory.

### 1.5 The log and its rollback protocol (single-writer assumption)

- `start` creates/opens `.arca/log.md` append-mode, records its old length, and on a failed state write truncates it back (`src/scheduler.rs:258-282`).
- `step` appends the run record and, if the paired state write fails, truncates `log.md` to its pre-append length and restores `state.toml` (`src/scheduler.rs:361-380`, `417-431`).
- This truncate-to-old-length rollback is only sound with exactly one writer. Two concurrent runs appending to one shared log means a rollback by run A can destroy a committed record of run B. This is the strongest *code-level* argument that N runs force per-run logs (§6.3).

### 1.6 Evidence and the runbook are re-read, not pinned

- `src/scheduler.rs:284-296` — at start, evidence records the Stable Engine pin (ETB-001) and the goal baseline revision (ETB-003, `goal_frozen = None` until intake completes).
- The runbook itself is **not** pinned: every `Scheduler::open` re-parses `.arca/ratmac.toml` from the working tree (`src/scheduler.rs:189-198`, `202-212`), and `start` re-parses again (`:233`). `step`'s only drift defense is that the recorded phase must still be declared (`src/scheduler.rs:320-328`: "State File phase … is undeclared in ratmac.toml"). A merge that keeps phase names but rewires transitions changes routing silently between two `rtm step` invocations. See §7.1.

### 1.7 Residency today: tracked, untracked, and unignored

- `.gitignore` (entire file, lines 1-24) ignores only `debug`, `target`, `**/*.rs.bk`, `*.pdb`, `**/mutants.out*/`, and `.arca-private/`. It does **not** mention `.arca/state.toml`, `.arca/rtm.lock`, or `.arca/evidence.toml`.
- `git ls-files .arca` shows `.arca/log.md` and `.arca/ratmac.toml` are **git-tracked** (also `.arca/tpl/state.toml`, a template). No live `state.toml`/`rtm.lock`/`evidence.toml` exists in the current working tree.
- Consequence: an active Run's `state.toml` is an untracked-but-unignored file (accidental-commit hazard), while the engine actively appends to a *tracked* file (`log.md`) — so any Run stepped on a branch makes that branch's `log.md` diverge, and worktree merges collide on it by construction.
- ADR-0007 (`.arca/goal/design.md:55-61`): "Model N Runs, allow 1 active" — "Lifting the limit is additive: allow `start` to create a second Run, grow an optional run-id argument; no breaking change. The Run identity scheme is deferred until the limit lifts (YAGNI)."
- ADR-0008 (`.arca/goal/design.md:63-74`): "N-Run extension path: when ADR-0007's limit lifts, per-Run files move under a runs directory; the v1 flat layout is the one-active-Run projection of that — additive migration, deferred."

This research is exactly that deferred work, done ahead of decision.

---

## 2. Q1 — Named runs: `.arca/runs/<run-id>/state.toml`

### 2.1 Proposed layout

```
.arca/
  ratmac.toml                  # Machine Class — unchanged, tracked, human-authored (ADR-0004)
  runs/                        # git-ignored scratch (§4); directory listing IS the registry (§3)
    r-001/
      state.toml               # same seven fields as today (src/model.rs:194-202), unchanged shape
      log.md                   # per-run engine journal (was .arca/log.md)
      rtm.lock                 # per-run invocation lock (was .arca/rtm.lock)
      evidence.toml            # per-run pins (was .arca/evidence.toml)
```

`state.toml` content is byte-identical to today's shape — no `run_id` field inside the file; the directory name is the identity (single source; duplicating the id inside the file invites divergence, and the doctor can flag a malformed dir name without it).

### 2.2 Minimal diff, enumerated

| Site | Today | Change |
| :--- | :--- | :--- |
| `src/model.rs:86-93` | `for_root(root)` joins `.arca/{state.toml,log.md,rtm.lock}` | add `for_run(root, run_id)` joining `.arca/runs/<id>/…`; keep `for_root` as the v1 one-active-Run projection during migration |
| `src/state.rs:56` | `StateStore::new(&root)` hardcodes `.arca/state.toml` | take the state path (or run dir) as a parameter — removes one of the two independent path derivations |
| `src/pin.rs:160-162` | `evidence_path(root)` = `.arca/evidence.toml` | `evidence_path(root, run_id)` |
| `src/scheduler.rs:189-198` | `Scheduler::open(root)` | `open(root, run_id)`; the stored `StateStore` and lock helper (`:794-800`) become per-run |
| `src/scheduler.rs:238-246` | lock at `.arca/rtm.lock`; admission = flat `state.toml` exists | mint + admission per id (§3.3 gives an atomic mint that subsumes the existence check) |
| `src/scheduler.rs:258`, `:361` | log at `.arca/log.md` | `.arca/runs/<id>/log.md` |
| `src/abandon.rs:108-110`, `:161` | flat `state.toml` / `rtm.lock` | per-run paths; confirm phrase (`src/abandon.rs:8`, `:59`) should name the run id, not just the project dir |
| `src/cli.rs:90-135` | no run argument | parse `--run <id>`; `start` mints and prints the id; `status`/`step`/`hold`/`abandon` require it (policy below) |
| `src/cli.rs:391-425` | doctor's environment report reads the one `state.toml` | enumerate `.arca/runs/*/state.toml` (stays read-only, ORS-002) |

**Supersession — no migration projection (2026-07-30, propagating the migration-window resolution):** the first row's "keep `for_root` as the v1 one-active-Run projection during migration" is superseded. Per §9 open question 4's **Resolved (2026-07-29)** — cut over with a refuse-and-instruct migration; the flat layout is never treated as an implicit default run (`08` §4 item 9; batch human sign-off, recorded in design.md) — and the run-residency issue's (`i-017-run-residency`) `FDC-005`, a flat-layout residue refuses and instructs, never auto-migrates, so no migration window keeps `for_root` alive as an implicit one-active-Run projection. The row's original text stays as written and should be read through this correction.

Untouched: `src/graph.rs`, `src/machine.rs` (parser), guard evaluation, `src/receipt.rs` / `src/completion.rs` gate logic (they receive paths from the scheduler). The diff is path plumbing plus CLI parsing — consistent with ADR-0007's "additive, no breaking change" prediction.

### 2.3 CLI surface: `--run` flag vs positional vs env

| Option | Assessment |
| :--- | :--- |
| `--run <id>` flag | **Recommended.** Explicit in every recorded command, which is exactly what ORS-003 behavioral evidence wants (role scenarios record attempted commands, `.arca/goal/spec.md:96`). No ambient state, greppable in transcripts. |
| Positional (`rtm step r-001`) | Collides with the established positional-path conventions: `rtm doctor <path>` (DRD-005, `.arca/goal/spec.md:142`) and `rtm scaffold <path>` (AAL-002). "Is this arg a run id or a file?" is a new ambiguity class. Reject. |
| Env var (e.g. `RTM_RUN`) | Ambient authority. It leaks into child environments the Main-Agent spawns, giving Subagents a ready-made valid default — the exact opposite of ORS-001's "a Subagent never invokes any `rtm` command" audit posture (`.arca/goal/spec.md:94`), and it is invisible in recorded commands (weakens ORS-003). Reject. |

**Default-run policy.** ADR-0007 explicitly calls "default run when unambiguous" a footgun it avoided (`.arca/goal/design.md:61`). The consistent reading: `--run` is required on every run-addressing command once the layout is per-run; a missing `--run` is a refusal that lists the registry roster (matching the refusal-with-diagnosis culture, ADR-0006). `rtm start` is the one command without `--run` *input* — it mints and prints. A convenience "exactly one run exists → allow omission" rule is possible but contradicts the recorded ADR rationale; flagged as an open question (§9).

---

## 3. Q2 — Run-id semantics

### 3.1 Who mints

The Engine mints at `rtm start`, prints the id to stdout, and the caller (human or Main-Agent per ORS-001) carries it forward. Agents never invent ids — same philosophy as "agents read state, never write it" (`.arca/goal/index.md:5`). An optional `rtm start --run <name>` lets a human pre-name a run; the Engine validates and refuses collisions rather than auto-suffixing (refuse-report-stay, ADR-0006).

### 3.2 Validity (Windows-aware, this is a win32-primary project)

- Pattern: `^[a-z0-9][a-z0-9-]{0,63}$`. Lowercase-only is load-bearing: NTFS is case-insensitive, so allowing `R-001` and `r-001` as distinct ids would create a filesystem collision that the id scheme pretends is legal.
- Refuse Windows reserved device names (`con`, `prn`, `aux`, `nul`, `com1`-`com9`, `lpt1`-`lpt9`) and names ending in `-` (covers the trailing-dot/space class by construction).
- Default minted form: `r-NNN`, zero-padded three digits, next integer above the highest existing `r-NNN` in the registry listing.

### 3.3 Collision handling and the registry: the directory listing IS the registry

No registry file. A `runs.toml` index would be a second source of truth that can disagree with the directory and — if ever tracked — a merge hotspot. The listing of `.arca/runs/` is the registry; the doctor renders it.

Atomic mint without a registry lock (proposed):

1. Write the full run skeleton into `.arca/runs/.tmp-<pid>-<seq>/` (`state.toml` via the existing temp-then-replace writer, `log.md`, `evidence.toml`).
2. `fs::rename(".arca/runs/.tmp-…", ".arca/runs/<id>")`. Same-volume directory rename is atomic on NTFS; if the target exists the rename fails → refusal naming the colliding id.

This makes the filesystem's own exists-check the collision arbiter (exactly the pattern `InvocationLock::try_acquire` already uses for files, `src/scheduler.rs:131-136`), guarantees readers never observe a half-initialized run, and needs no new lock for minting. Concurrent `rtm start` invocations that race for the same auto-minted `r-NNN` resolve as: one wins the rename, the loser's rename fails, loser re-lists and retries once with the next number (bounded, mirrors `MAX_ATTEMPTS` culture) or simply refuses.

---

## 4. Q3 — State residency

### 4.1 The three options

**(a) Tracked (status quo trajectory).** Run state enters history. Every branch that steps a Run diverges `log.md` (tracked today, `git ls-files`) and would diverge `state.toml`/`evidence.toml` if committed. Merging two branches that each ran means merging two interleaved orchestration histories: textual conflicts in `log.md` at best; at worst a fast-forward silently adopts another branch's `phase`, teleporting the machine. Merge also swaps `ratmac.toml` mid-Run (§7.1). This option makes the Run *be* the branch, which is the directory-identity disease in git form.

**(b) Ignored scratch in the PRIMARY checkout under `.arca/runs/` — recommended.** Add `.arca/runs/` to `.gitignore`. Orchestration state never enters the index, so no merge can ever carry it: worktree branches merge *work products* (code, tickets, tests), never machine position. A Run whose work happens in a worktree lives in the primary checkout keyed by id, with the worktree recorded as run data (e.g. in `active_refs`, `src/model.rs:200`, or an evidence field), not as the run's home.

**(c) Ignored scratch per-worktree.** Each worktree gets its own `.arca/runs/`. This recreates directory-identity one level up: the run dies when the worktree is removed (`git worktree remove` deletes untracked files), cross-run status requires scanning every worktree, and the Main-Agent must cd around to step runs — contradicting ORS-002's "run from the project root" anchoring (`.arca/goal/spec.md:95`). Reject except as an emergent side effect (someone *may* start an unrelated run inside a worktree; the engine need not forbid it, the doctor just won't see it from the primary root).

### 4.2 Testing the key claim

> "If all run state lives under `.arca/runs/` in the primary checkout and is git-ignored, worktree merges never carry orchestration state and the collision problem dissolves."

**Verdict: true for the stated scope, with three residues that must be handled or the claim quietly fails.**

1. **The tracked log residue.** `.arca/log.md` is git-tracked and engine-written (`src/scheduler.rs:258-282`, `361-431`). Adding `.arca/runs/` to `.gitignore` does nothing about it — and gitignore never unstages an already-tracked file anyway. The engine's log target must *move* into `.arca/runs/<id>/log.md`; `.arca/log.md` remains as the tracked, human/knowledge-base journal that the engine no longer touches. Without this piece, worktree merges still collide on every Run.
2. **The evidence residue.** `.arca/evidence.toml` (`src/pin.rs:160-162`) is per-Run by definition ("Run evidence", `src/pin.rs:44`) and must move per-run, or N concurrent runs overwrite each other's pins — an ETB-001 integrity failure, not just a merge nuisance.
3. **The runbook residue.** `ratmac.toml` stays tracked and shared (correctly — it is the Machine Class, ADR-0004/0005 template). Merges can therefore still swap the *machine* mid-Run. Residency does not dissolve that; pinning does (§7.1). The claim is about orchestration *state*, and for state it holds.

Also note the cost being accepted: git-ignored state has no git durability. Losing `.arca/runs/` loses machine positions (not work products — those are in branches, tickets, and the tracked journal). Abandon (`PGE-007`) already treats the terminal event as belonging to the append-only history (`src/abandon.rs:15-17`); under per-run logs, retiring a run should archive-or-summarize its `log.md` into a durable location (open question §9).

### 4.3 Worktree visibility mechanics

Root = cwd (`src/bin/rtm.rs:5`), so `rtm` inside a worktree sees the worktree's own `.arca/` and would not find primary-checkout runs. This is *aligned* with policy, not against it: ADR-0003 says the Main-Agent or human calls `rtm step` and Subagents never touch the Scheduler (`.arca/goal/design.md:21-23`); ORS-001 confines invocation the same way. The Main-Agent operates from the primary root; workers in worktrees do work, not orchestration. No `--git-common-dir` resolution logic is needed in v-next — keeping root = cwd is both simpler and a quiet enforcement of the caller policy.

---

## 5. Q4 — Lock scope

### 5.1 Today

One transient `InvocationLock` per directory (`.arca/rtm.lock`), acquired per mutating invocation and dropped at return (`src/scheduler.rs:105-173`, `238-239`, `794-800`). Admission is deliberately *not* the lock (`:240-241`). This is one global lock serializing everything in the directory.

### 5.2 Proposed: per-run lock; mint needs no lock (rename-mint)

- **Per-run step lock:** `.arca/runs/<id>/rtm.lock`, exact `InvocationLock` semantics relocated. Serializes step/hold/abandon/load within one run.
- **Registry-level mutual exclusion:** provided by atomic dir-rename minting (§3.3), so no long-lived global lock is required. If the auto-number retry loop is deemed too clever, a tiny `.arca/runs/.lock` held only during mint is the fallback.
- The flat `.arca/rtm.lock` remains only for the v1-projection compatibility window, then retires.

### 5.3 Race inventory with N runs stepped by one Main-Agent

| # | Race | Resolution |
| :--- | :--- | :--- |
| 1 | `start` vs `start` (same auto-minted id) | rename-mint: one wins, one refuses/retries (§3.3) |
| 2 | `step` vs `step`, same run (two shells) | per-run lock; loser retries then refuses after `MAX_ATTEMPTS` (existing behavior, `src/scheduler.rs:156-166`) |
| 3 | `step` run A vs `step` run B | independent **only after** log and evidence go per-run. Shared `ratmac.toml` is read-only at open (`:191`) — safe. A shared singular log is NOT safe: the rollback truncation (`:417-431`) can destroy the other run's committed record. This race is why the log question (§7.3) is not stylistic. |
| 4 | `abandon` vs `step`, same run | `abandon` must acquire the per-run lock before retiring files (today it references the flat lock, `src/abandon.rs:110`, `:161`) |
| 5 | doctor enumeration vs mint | rename-mint guarantees any visible `.arca/runs/<id>/` is fully formed; the doctor additionally reports (never repairs) stray `.tmp-*` dirs as findings — read-only per ORS-002 |
| 6 | `step` vs external `git merge`/`checkout` in the primary tree | not a lock problem — the runbook can change between invocations regardless of locking. Only pinning (§7.1) addresses it. |

---

## 6. Q5 — Prior art on run identity

(Cross-reference: `02-orchestration-prior-art.md` covers execution/orchestration models broadly; this section is narrowly about how each system *keys* run state.)

**LangGraph — `thread_id` + checkpointer.** State is checkpointed per `(thread_id, checkpoint_ns, checkpoint_id)`; callers pass `configurable: {"thread_id": …}` and the checkpointer (memory/SQLite/Postgres) persists snapshots ([LangGraph persistence docs](https://langchain-ai.github.io/langgraph/concepts/persistence/)). What it gets wrong for ratmac's purposes: `thread_id` is caller-supplied free text with no mint, no validity rule, and no collision policy — two callers reusing an id silently share and extend one history; and the default in-memory saver means identity exists but residency is an afterthought. Lesson: **the engine mints or validates; reuse is a refusal, not a merge.** LangGraph's one strong move — concurrent updates to the same thread raise a hard error (`INVALID_CONCURRENT_GRAPH_UPDATE`, see 02) — is the per-run lock in database clothing.

**Temporal — workflow id + run id + event history.** Two-level identity: a caller-chosen Workflow Id (business key) with an explicit `WorkflowIdReusePolicy`, and a system-minted Run Id (UUID) per execution; state *is* the append-only event history, replayed deterministically ([Temporal docs: Workflows](https://docs.temporal.io/workflows)). Right ideas ratmac should copy: system-minted instance identity distinct from the human name, and an explicit, named reuse policy (ratmac analog: what may reuse `r-001` after abandon? §9). What to avoid: identity welded to a replay runtime — Temporal must detect "nondeterminism" when workflow *code* changes under a live history, a whole error class ratmac dodges by pinning the runbook instead of replaying against it.

**Airflow — `dag_run_id`.** Historically a DAG run's identity was its `execution_date` (logical date): running the same DAG twice for the same date was structurally impossible, and years of operator pain led to AIP-39 decoupling run identity from schedule semantics ([Airflow DAG Runs docs](https://airflow.apache.org/docs/apache-airflow/stable/core-concepts/dag-run.html), [AIP-39](https://cwiki.apache.org/confluence/display/AIRFLOW/AIP-39+Richer+scheduler_interval)). Lesson, stated bluntly: **never key run identity on an attribute of the work.** Keying identity on the directory is an `execution_date`-class mistake — same shape, different attribute.

**Actor systems — addresses vs incarnations.** Erlang: PIDs are system-minted and unforgeable; registered names are a human-facing registry where re-registering a taken name is an error ([Erlang reference: processes](https://www.erlang.org/doc/system/ref_man_processes.html)). Akka: an actor *path* (address) can be reused after restart, so identity includes a per-incarnation UID precisely because path-as-identity produced stale-reference bugs ([Akka: actor references, paths and addresses](https://doc.akka.io/libraries/akka-core/current/general/addressing.html)). Lesson: **address ≠ identity.** The directory is ratmac's address; the run id is the incarnation. Today they are the same value, which is exactly the bug class Akka added UIDs to kill.

---

## 7. Q6 — Interaction with existing requirements

### 7.1 ETB-001 and runbook pinning (the merge-swaps-the-machine hole)

ETB-001 (`.arca/goal/spec.md:66`) pins *gate artifacts* by resolved path + content hash, refusing on drift. The runbook enjoys no such pin: it is re-parsed from the working tree at every `Scheduler::open` (`src/scheduler.rs:191`, `202-212`), and the only mid-Run defense is the phase-undeclared refusal (`:320-328`). Per-run state residency **neither fixes nor worsens** this: the runbook is shared by design, and a `git merge` in the primary checkout still swaps it between invocations of the same Run.

Proposed fix, reusing existing machinery rather than inventing any: at `start`, record `runbook_sha256` (SHA-256 of `.arca/ratmac.toml` bytes) in the run's `evidence.toml`; at every `open`, re-hash and on mismatch refuse naming observed and expected identity — the literal ETB-001 sentence shape applied to the runbook itself. Adopting a changed runbook mid-Run becomes an explicit, human-confirmed command (culture-match: PGE-006 blocked-route, PGE-007 abandon `--confirm`). A frozen *copy* under `.arca/runs/<id>/runbook.toml` was considered and rejected: it forks the single written authority (RBS-004's spirit) and hides drift instead of refusing it.

Per-run residency actually *helps* here indirectly: evidence becomes per-run (§4.2 residue 2), so N runs can hold N different runbook pins — meaning a runbook edit can be adopted by new runs while old runs finish under their recorded pin, each refusal naming its own expected hash.

### 7.2 ORS-001..003

- **ORS-001** (caller policy, `.arca/goal/spec.md:94`): unchanged by `--run`; the Engine still gains "no caller identity, authentication, or authorization state." The flag is an argument, not an identity. The env-var option was rejected in §2.3 specifically because it would create ambient state adjacent to this requirement.
- **ORS-002** (bootstrap/doctor, `:95`): the doctor's report of "state-file presence/phase" generalizes to a registry roster: id, phase, status, blocker per run, plus findings for invalid dir names and stray `.tmp-*` dirs. Still writes nothing. "With no active Run names the next legitimate action" generalizes to "with an empty registry, names `rtm start`."
- **ORS-003** (behavioral evidence, `:96`): the role-scenario harness gains cases — a Subagent invoking `rtm step --run r-001` must fail the check exactly as argument-free invocations do; a Main-Agent stepping two runs interleaved is a new *positive* scenario. Explicit `--run` on the command line is what makes those transcripts auditable.

### 7.3 The append-only log: singular or per-run?

Per-run, for three independent reasons: (1) correctness — the rollback-truncation protocol is single-writer (§1.5, §5.3 race 3); (2) residency — the singular log is git-tracked, and engine writes to a tracked file re-import the merge-collision problem the whole proposal exists to dissolve (§4.2 residue 1); (3) scope hygiene — `.arca/goal/index.md:15` already declares "no agent-journal/log-merge reconciliation across parallel worktrees" a non-goal; per-run logs keep that non-goal cheap forever, because there is nothing to reconcile. The singular `.arca/log.md` survives as the tracked human/project journal the engine never writes.

---

## 8. Concrete proposed shapes (all "proposed", none "today")

```toml
# .arca/runs/r-001/state.toml — unchanged seven-field shape (src/model.rs:194-202)
phase = "P2"
status = "executing"
goal_revision = "2abaec4"
input_revision = ""
output_revision = ""
active_refs = []
blocker = ""
```

```toml
# .arca/runs/r-001/evidence.toml — existing serializer (src/pin.rs:122) plus two additive keys
runbook_sha256 = "<64 hex chars>"   # §7.1: refuse on drift, adopt only by explicit human confirm
worktree = "trial/i-016"            # optional, informational: where this Run's work happens
```

```gitignore
# .gitignore addition
.arca/runs/
```

CLI (proposed): `rtm start [--run <name>]` prints the minted id · `rtm status|step|hold|abandon --run <id>` · missing/unknown `--run` refuses with the roster · `rtm doctor` lists the registry read-only.

---

## 9. Open questions for the human

1. **Id reuse policy after abandon:** may a retired id (e.g. `r-001`) ever be minted again? Temporal makes this an explicit named policy; silence here will become a bug report. (Suggest: never reuse; abandoned run dirs are renamed `r-001.abandoned` or archived, so the namespace only grows.) **Resolved (2026-07-29):** never reuse; respawn mints a new id and the ledger entry records the superseded one (the id-reuse gap, `AR-09`; `08` §4 item 11; batch human sign-off, recorded in design.md).
2. **Exactly-one-run ergonomics:** ADR-0007 rejected "default run when unambiguous," but that was recorded while the limit was 1. Does the rejection stand for the N-run CLI, or is `--run` optional when the registry has exactly one entry? **Resolved (2026-07-29):** the rejection stands — `--run <id>` is always required, no unambiguous-registry exception; a missing value refuses with the roster (the run-addressing contradiction, `AR-02`; `08` §4 item 7; batch human sign-off, recorded in design.md).
3. **Abandon durability:** when a git-ignored run is retired, should its `log.md` be summarized/archived into a tracked location (which one?), or is losing engine journals with the scratch dir acceptable? **Resolved (2026-07-29):** archive a retired run's history into the tracked journal on abandon; no consolidated read view yet (`08` §4 item 16; batch human sign-off, recorded in design.md).
4. **Migration window:** support the flat v1 layout as an implicit "default run" during transition (ADR-0008 calls the flat layout the one-active-Run projection), or cut over with a one-time refusal that names the migration step? **Resolved (2026-07-29):** cut over with a refuse-and-instruct migration; the flat layout is never treated as an implicit default run (`08` §4 item 9; batch human sign-off, recorded in design.md).
5. **Runbook pin adoption:** is the mid-Run "adopt new runbook hash" confirmation a new CLI verb, an extension of `hold`, or reserved to `start` only (old runs must finish or be abandoned under their pin)?
6. **Active-run cap:** with named runs, does ADR-0007's "allow 1 active" lift entirely, or become a configurable cap (e.g. refuse `start` when K runs are `executing`)? **Resolved (2026-07-29):** the cap lifts entirely — any cap below the fan-out width would refuse mid-spawn (the cap-and-id-reuse gap, `AR-09`; `08` §4 item 10; batch human sign-off, recorded in design.md).

---

## Supersession note — 2026-07-30 atomic cut

Billy split the pending doctrine-convergence execution bundle without changing this research's
run-identity findings or the integrated residency requirements. Current pending homes are:
input-routed transitions (`FDC-001`) in
[i-016-fsm-doctrine-convergence](../../issue/archive/i-016-fsm-doctrine-convergence/index.md), Run completion
(`FDC-002`) in [i-020-run-completion](../../issue/archive/i-020-run-completion/index.md), and input delivery
and durability (`FDC-003`) in
[i-019-input-delivery-durability](../../issue/archive/i-019-input-delivery-durability/index.md). The
accepted `Verdict slot` and current `verdict.toml` reservation remain unchanged; any physical rename
needs an explicit accepted-goal, source, test, and migration change.
