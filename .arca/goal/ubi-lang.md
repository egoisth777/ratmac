# ubi-lang — ratmac

Glossary of ubiquitous language. One term, one meaning. Terms not listed here must not be used in docs, code, or CLI output.

| Term | Definition |
|---|---|
| Machine | The state machine as a whole — Phases, transitions, Exit Guards — pure data declared by a Machine Class, never run by agents; an agent sees only its Phase Prompt, never the graph (ADR-0009). |
| Machine Class | The state-machine definition in `ratmac.toml`. Data, not code. A template: declares Phases, transitions, Exit Guards. Human-written and reviewed, never agent-authored. |
| `ratmac.toml` | The Machine Class file, TOML (ADR-0004), at `.arca/ratmac.toml` (ADR-0008). One per project. |
| Run | A live instance of a Machine Class, created by `rtm start` (class vs instantiation, ADR-0005). Each Run owns its State File and Transition Log. |
| Scheduler | The generic engine — Rust CLI, binary `rtm`. Sole writer of State Files. Holds no project-specific knowledge. |
| Phase | A node of the Machine where agent work happens. The ONLY dimension of machine state (ADR-0001). Always say "Phase", never "state", for machine nodes. |
| Status | Phase-local lifecycle (`planned\|executing\|blocked\|passed\|failed`) recorded by the Scheduler. Not part of the Machine graph (ADR-0001). |
| Exit Guard | A predicate over the working tree, evaluated by the Scheduler at `rtm step`. Checks artifacts, never agent claims; passing ALL of a Phase's Exit Guards is the only way to leave it. The closed guard-kind vocabulary — each kind's semantics, required fields, and forbidden fields — is defined once in `.arca/runbook-spec.md` and restated nowhere else (RBS-002, RBS-004). |
| State File | `.arca/state.toml` (ADR-0008). Per-Run machine-readable current state. Written ONLY by the Scheduler; all agents read, never write (ADR-0003). |
| Transition Log | `.arca/log.md` (ADR-0008). Per-Run append-only record of every transition the Scheduler performs. |
| Phase Prompt | What an agent receives for a Phase: inline prose from `ratmac.toml` + the Scheduler-generated Exit Guard list (ADR-0009). The ONLY machine information ever shown to an agent. |
| Main-Agent | The orchestrating agent in the main checkout. May invoke `rtm step`; changes state only through the Scheduler (ADR-0003). |
| Subagent | A worker agent in a ticket worktree. Reads state; never invokes `rtm` (ADR-0003). |
| `rtm start` | Instantiate a Run. A human may invoke it directly; the Main-Agent only after explicit human Run-start sign-off; a Subagent never (ORS-001). |
| `rtm step` | Transition request for a Run (replaces the handoff's `next`). Requesting is not deciding (ADR-0002); Exit Guards decide. A refused `step` changes nothing (ADR-0006). |
| `rtm status` | Read-only report of a Run's Phase, Status, and pending guards. |
| Legacy identity | The superseded spellings `arca-scheduler` and `schd`, retained only in the historical allowlist or explicit migration records. |
| Clean cutover | `ratmac` and `rtm` are the only active product and command spellings; no compatibility alias or package fallback is shipped. |
| External repository identity | The GitHub slug, canonical `origin` URL, checkout directory, `.git` metadata, and active repository-facing links that identify this project outside its Rust code. |
| Canonical repository | The GitHub repository `egoisth777/ratmac`; the superseded `egoisth777/arca-scheduler` slug is not canonical after cutover. |
| Canonical origin | The exact SSH URL `git@github.com:egoisth777/ratmac.git` recorded for the local `origin` remote. |
| Checkout basename | The final local directory name `ratmac`, verified from the checkout's actual path and Git top-level rather than from a display-only label. |
| Historical allowlist | Append-only `.arca/log.md`, archived issue/ticket records, and explicit migration evidence where legacy external identity may remain unchanged as provenance. |
| Safe external cutover | A preflighted, ordered repository rename, origin update, and checkout move with checkpoints and reversible recovery that never discards work, force-pushes, or bypasses Git arbitration. |
| Stable Engine pin | The recorded identity (resolved path plus content hash) of the `rtm` binary that owns the active Run. |
| Pinned gate artifact | A prebuilt executable whose resolved path and content hash are recorded alongside the Stable Engine pin; the only project-derived code a command guard may execute during a Run. |
| Refusal diagnostic | The bounded observed-versus-expected text a refused transition prints, naming the concrete artifact or predicate to repair. |
| Start baseline revision | The content revision of `.arca/goal/` recorded when the Run is created, before any intake integration. |
| Frozen goal revision | The content revision of `.arca/goal/` computed after intake integration completes; the only revision gap analysis and residual records may cite as the freeze. |
| Goal drift | Any change to `.arca/goal/` observed after the frozen goal revision is recorded and before the build batch closes. |
| Contract gate | A mechanized phase gate that verifies the phase's artifact contract and evidence, so status or prose edits alone can never satisfy it. |
| Evidence receipt | A structured, tamper-evident record of one executed check: the command or predicate, the exercised target, the observed result, and a content digest binding them. |
| Sensitivity receipt | An evidence receipt proving a planned test can fail: a recorded baseline failure before implementation or a controlled mutation that flips it. |
| Agent-writable evidence artifact | A file agents may author to carry notes and receipts, distinct from Scheduler-owned files; the append-only log remains Engine-owned. |
| Blocked route | A human-authorized Runbook route that moves a Run forward while an executing ticket is held with a linked blocker, preserving its honest partial evidence. |
| Blocker record | The concrete artifact a held ticket links to — a new five-file issue or a named residual — stating why the ticket cannot pass. |
| Run abandonment | The human-authorized terminal retirement of a broken active Run: RTM records a terminal abandoned event and safely retires the Run's admission state so a fresh Run can start. |
| Reviewable snapshot | Candidate content whose every exercised path is visible to git review (tracked or staged), so the tested tree can be reconstructed and audited from the recorded change. |
| Snapshot manifest | The recorded enumeration binding an evidence claim to its snapshot: per declared root, the git tracking state and a content digest. |
| Declared evidence root | A directory the acceptance evidence claims to have exercised (product sources, QA suites, contributor artifacts). |
| Authorized archive move | Relocating a completed issue folder to `.arca/issue/archive/<issue-id>/` with identity and five-file shape preserved and content unchanged except required relative-link updates. |
| Release acceptance lane | The environment-coupled checks (live GitHub identity, exact origin, branch, clean worktree) that prove an operator cutover, runnable only by explicit opt-in. |
| Default suite | What plain `cargo test --workspace` runs with no opt-in environment configured. |
| Run-start sign-off | Explicit human authorization for the Main-Agent to invoke argument-free `rtm start` for the current target project; conversational instruction suffices, and no token, file, or Engine state encodes it. |
| Project-local bootstrap | One documented command run from the project root that locates or builds the Stable Engine binary, verifies its recorded identity, and reports the resolved path without global installation or PATH mutation. |
| Doctor report | Read-only diagnosis output naming the resolved Engine identity, distinguishing the human-authored Runbook from Scheduler-owned runtime state, and stating the next legitimate action. |
| Behavioral evidence | Proof derived from recorded attempted commands or tool calls in role scenarios — what a caller actually invoked or refrained from invoking. |
| Guidance-consistency evidence | Proof that active guidance texts agree with each other; never a substitute for behavioral evidence on invocation claims. |
| Experiment base | The long-lived local branch `exp/ratmac-deterministic`: every trial starts from its clean committed tip; a finish adds only the durable log, while fixes arrive from local `main` only through explicit merge/sync. |
| Trial | One bounded, numbered experiment attempt with a hypothesis, executed on its own trial branch inside its own trial worktree, concluded by a finish that preserves its log and terminal commit. |
| Trial branch | The branch `trial-<nnn>-<topic-slug>` created at the experiment base tip when a trial starts; it never merges into `main` or the experiment base. |
| Trial number | `<nnn>`: a positive integer zero-padded to at least three digits, inferred as one greater than the highest number occupied by any live trial branch, archive tag, or durable log directory; an explicit override is collision-checked. |
| Topic slug | The short lowercase dashed name in the trial branch, matching `[a-z0-9]+(-[a-z0-9]+)*`; anything else is refused before mutation. |
| Trial worktree | The linked Git worktree of the trial branch, at a sibling directory of the repository root derived deterministically from the repository basename and the trial branch name. |
| Trial log | The Advisor-authored structured `trial-log.md` committed on the trial branch, covering identity, base and terminal commits, hypothesis, procedure, commands and tests, observations, verdict, recommendations, and artifact/diff references. |
| Durable log destination | `trials/<trial-branch>/trial-log.md` committed on the experiment base at finish — the only trial content that outlives the trial. |
| Trial archive tag | The immutable annotated tag, deterministically named from the trial branch, created at the terminal trial commit and verified before any deletion; it makes branch deletion reversible. |
| Terminal trial commit | The trial branch tip at finish time — the commit the trial archive tag must preserve. |
| Trial lifecycle interface | The single documented repo-local entry point offering exactly trial start, status/dry-run, finish, and base sync; offline, push-free, and free of global installation or PATH/global-config mutation. |
| Dry-run preview | Read-only status output naming repository facts, live and archived trials, the next inferred trial identity, and per mutating operation the exact planned mutations and their recovery commands. |
| Recovery commands | The exact Git commands, printed by status/dry-run and finish, that restore a deleted trial branch from its archive tag, re-add its worktree, or resume an interrupted finish. |
| Advisor | The reviewer agent that authors trial-log content and never invokes a lifecycle mutation or any Git write. |
| Windows directory lock | An open handle on the trial worktree directory that blocks removal; it grounds a safe named refusal with guidance — never a forced removal, never a guessed process kill. |
| Main-first fix flow | The policy that defects exposed by trials are fixed on local `main`, then received by a clean experiment base only via explicit merge/sync — never reset, rebase, or force — with conflicts left visible. |
| Runbook specification | `.arca/runbook-spec.md`: the single written authority for the Machine Class format — top-level shape, phase and transition fields, the guard-kind vocabulary, ownership rules, and the diagnostic-code table. Code implements it; nothing restates it. |
| Guard-kind vocabulary | The closed list of guard kinds a runbook may declare, each with its semantics and its per-kind required and forbidden fields. A kind outside the list is not a guard but a parse error. |
| Typed parse | The single deserialization of `ratmac.toml` into the Machine Class, after which every consumer reads the typed value; no module re-reads the file. |
| Named refusal | An error that states what is wrong and where, carried to the caller, as opposed to proceeding on a default value — an absent runbook refuses instead of yielding an empty machine. |
| Finding | One doctor result: a stable code, a severity (`error` or `warning`), a location, and a message. The doctor produces a list of findings; `--json` and the human report are two renderings of that one list. |
| Diagnostic code | The stable identifier of one defect class (`RB1xx` parse and schema, `RB2xx` graph, `RB3xx` guard lint, `RB4xx` ownership). Stable means the same defect yields the same code across runs and releases. |
| Guard lint | The doctor checks on guards beyond parseability: a `command_exit` guard neither `exempt` nor pinnable, and a guard whose verdict rests on agent-writable content. |
| Scaffold | The minimal doctor-clean runbook `rtm scaffold <path>` writes, so authoring begins from valid rather than from blank. |
| Authoring loop | write → `rtm doctor --json` → repair by code, repeated until the doctor reports clean. |
