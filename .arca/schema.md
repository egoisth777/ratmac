# Schema - the working rules

This file **binds** contributors (human or agent): the planning pass, the
build loop, issues, gap records, tickets, evidence, ownership, bootstrap,
trials. It is law, not orientation.

To locate anything - paths, the project manifest, steering, the system map -
read [index.md](index.md): that is the index; this is the schema it points to.

# Two rule sets — read this first

There are two sets of rules. **They never limit each other.**

1. **Product rules** — `.arca/goal/` (order of authority: `spec.md` &gt; `design.md` &gt; `test-list.md`). They say
 what ratmac-the-program does while it is running and handling a wish. They bind the program's behavior
 and what its tests expect — nothing else.
2. **Working rules** — this file. It says what a contributor (person or agent) does while building the
 program: gap records, tickets, tests, code.

If a goal sentence seems to forbid a working file (e.g. "no working files", "creates no code/tickets/tests"),
it is talking about the running program, never about your workspace (`.arca/residual/`,
`.arca/ticket/`, `.arca-private/`, `test/`, source code). When two rules seem to clash: decide by which set
each belongs to, add one log line (`conflict-resolved: <refs> — <one-line reason>`), and keep going. **A rule
clash is never a reason to stop work.**

# Entry — no magic words

Any user request that touches this work — an issue, a gap, a ticket, tests, code, a fix, a test run — **is**
an entry. Start at the nearest step and catch up the earlier ones yourself: check each earlier step's finish
line; if it is unmet, produce the missing piece yourself, minimally. Never ask the user to run a step.


| User intent (examples)                                                         | Enter at |
| :------------------------------------------------------------------------------ | :-------- |
| "new issue", "integrate the issues"                                            | P1       |
| "what's missing", "where are the gaps"                                         | P2       |
| "plan the work", "cut tickets"                                                 | P3       |
| "write/implement the tests", "implement the rust tests from test/test-list.md" | P4       |
| "implement", "fix", "run QA"                                                   | P5       |


`start issue loop` still works (enters P1). Once entered, run on your own all the way through the build loop
without prompting. Return to Idle when no gap record says `missing` or `partial`, or when the user says stop.

# Never get stuck — the answer ladder

`blocked` is a status only the scheduler sets (missing entry prerequisites, .arca/goal/design.md, ADR-0006) — never your reason to stop. When something you need is missing, in this order:

1. **Work it out** from the defaults table or from how the repo already does things.
2. **Pick the safest reasonable value**; log `assumed: <what> — <why>`; keep going. A guess can be undone:
 if the user corrects it, redo the affected pieces — don't start over.
3. **Ask** — put every open question into one message, and meanwhile keep doing all work that does not
 depend on the answers. Put the open question in that one message; when entry prerequisites are missing, `rtm` records them in `blocker` in `state.toml`.

An unanswered question pauses only the piece that needs it, never the whole.

## Defaults


| Input                                  | Default                                                                                               |
| :-------------------------------------- | :----------------------------------------------------------------------------------------------------- |
| Goal freeze                            | Note the current git HEAD of `.arca/goal/` as the frozen version. Writing that note IS the freeze. |
| Ticket approval                        | Approved the moment it is created. The user may say `hold t-<id>` to pause that one ticket.           |
| `test_root`                            | `test/` (Rust harness: cargo test crate under `test/qa/`; create it if absent).                       |
| `discovery_command` / `run_command`    | By language — Rust: `cargo test`; otherwise look at the repo and pick.                                |
| `fixture_setup`                        | Copies of test data in a temp folder, kept separate per test; none when not needed.                   |
| `private_artifact_root` (hidden tests) | `.arca-private/` (kept out of git), created when first needed.                                        |


# The work — one straight pass, then a loop

The work has two parts with different shapes:

- **Planning (P1 → P2 → P3)** runs **straight through, once**, each time new issues come in. It never loops.
Many issues become one frozen goal, the goal is compared against reality, and each gap becomes one ticket.
- **Building (P4 → P5)** is **a loop: one full turn per ticket**. This is where nearly all the time goes.
For each ticket: write its tests, try to poke holes in them, write the code, run everything, fix until
green, review, take the next ticket.

The two parts meet at the gap check (P2). When the last ticket is done, do the gap check again: nothing
missing → rest (Idle). Something still missing → cut more tickets and keep building. The gap check is how the
loop knows when it is finished.

**The only road back:** if, while building, you find the goal itself is wrong or incomplete — do **not**
touch the goal. Write a **new issue** into `.arca/issue/`. It gets folded in on the next planning pass. From
the moment the goal is frozen until the last ticket is done, the goal does not move.

## The issue folder

- Each direct child of `.arca/issue/` is a folder holding exactly five files created from `.arca/tpl/issue/`:
`index.md` (front door: identity, where it came from, status, links), `ubi-lang.md` (issue-specific words,
or `No issue-specific terms.`), `spec.md` (what is asked for, plus what was decided about each ask),
`design.md` (suggested how — carries no weight until folded into the goal), `test-plan.md` (how to prove it
works, plus traces).
- Naming: folder name = `issue-id` = `i-<nnn>-<condensed-name>` — zero-padded number plus a short dashed name
taken from the title (2–4 words, e.g. `i-007-continuous-qa`). `<nnn>` alone guarantees uniqueness and
order; the name part is set at creation and does not change if the title changes later.
- `index.md` front matter: `issue-id` equal to the folder name, non-empty origin, status
`pending|integrated|rejected`; relative links to the other four files; no unfilled template blanks.
- The schema gate is mechanized by `rtm` Exit Guards; agents don't hand-run it. See .arca/goal/design.md, ADR-0006 and ADR-0009.
A failed check is a **fix you do right away**, not a stop; log the check and the fix.
- Creating an issue is a direct act: write the five files from the blanks, run the shape check, done. It
enters no loop step and touches neither `.arca/state.toml` nor `.arca/log.md`; the next planning pass folds it in.

## The steps (P1–P5)

**Planning — straight through, once per batch of issues:**


| Step                  | Does                                                                                                                                                         | Finish line                                                                                                                                                                                            |
| :--------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------ | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **P1** Fold in issues | Work every `pending` issue into the goal: give each ask a stable requirement ID and a decision `accepted                                                     | rejected                                                                                                                                                                                               |
| **P2** Find the gaps  | Freeze the goal (note git HEAD), then compare each requirement against what actually exists; write one record in `.arca/residual/` per requirement: `missing | partial                                                                                                                                                                                                |
| **P3** Cut tickets    | Turn each `missing                                                                                                                                           | partial`record into one small, self-contained, provable piece of work in`.arca/ticket/`(from`.arca/tpl/ticket.md`); order them so a ticket that needs another comes after it; **approved on creation** |


**Building — one full turn per ticket, in order:**


| Step                             | Does                                                                                                                                                                                                                                                                                                    | Finish line                                                                                                   |
| :-------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------- |
| **P4** Write this ticket's tests | Turn the ticket's planned checks (planned-test-ID → test function, recorded in the ticket) into runnable tests; then re-read them trying to poke holes: would they catch a wrong answer, do they cover the edges, does each stand alone; run them — they should fail, since the code is not written yet | Every planned check for this ticket runs as a real test; hole-poking notes logged                             |
| **P5** Write the code            | Implement; run **every test so far** (all earlier tickets' plus this one's); run the hidden test lanes (test code in `.arca-private/`, listed in the ticket with `hidden-id`, `goal-contract-ref`, `category`, `oracle`, `owner`); fix and re-run until all green; short review; take the next ticket   | All tests green including hidden lanes. No tickets left → redo P2's gap check → Idle when nothing is `missing |


```mermaid
flowchart LR
    IDLE([Idle]) -->|new issues / any work request| P1
    subgraph PLAN["Planning — straight through, once"]
        P1[P1 Fold in issues] --> P2[P2 Find the gaps] --> P3[P3 Cut tickets]
    end
    subgraph BUILD["Building — one turn per ticket"]
        P4[P4 Write this ticket's tests] --> P5[P5 Code + all tests + fix + review]
        P5 -->|next ticket| P4
    end
    P3 --> P4
    P5 -->|no tickets left| P2
    P2 -->|nothing missing| IDLE
    P5 -. "goal wrong or incomplete? file a NEW issue — folded in on the next planning pass" .-> P1
```

Hidden/breaking test lanes (run inside P5's full test run; frozen versions, separate test data, recorded
random seeds; one coordinator merges results by fingerprint and keeps the artifacts — a result that
contradicts another gets a log line plus a follow-up ticket, not a stop):


| Lane                | Owns                                                                                                      |
| :------------------- | :--------------------------------------------------------------------------------------------------------- |
| Regression          | Things that once worked still work; settled issues stay settled                                           |
| Input/Routing       | Malformed and hostile input, parsing edges, and input reaching the right handler                          |
| Lifecycle/Model     | Objects move between states only in allowed ways; not crash/restart concerns                              |
| Durability/Recovery | Crashes, restarts, and coming back up with nothing lost                                                   |
| Output/Filesystem   | Issue folder shape, required files, relative links, never touching user files, writing only where allowed |
| Cross-Feature       | Two or more features used together                                                                        |


# State

- `.arca/state.toml`: `phase`, `status: planned|executing|blocked|passed|failed`, versions in play,
`blocker` (the missing entry prerequisite recorded by `rtm` — nothing else).
- `.arca/log.md`: new lines only, never edited; one line per step change, guess, rule-clash
decision, or fix.

# Caller policy for `rtm`

One policy, and the same one on every surface (goal `ORS-001`, which supersedes the earlier rule reserving start for humans alone):

- A human may invoke argument-free `rtm start` directly.
- The Main-Agent may invoke `rtm start` only after explicit human Run-start sign-off for the current target project;
conversational sign-off is enough, and nothing in the Engine records it.
- A Subagent never invokes any `rtm` command; it reads state and does the ticket work.
- Only the Main-Agent or the human invokes `rtm step`; the Scheduler stays the sole writer of `.arca/state.toml`.

# Evidence and archive rules

These are durable working rules; the goal's `AOI-001`–`AOI-003` bind the program that mechanizes them.

- **Authorized archive move.** A completed issue folder — `index.md` status `integrated` or `rejected` — may move to
`.arca/issue/archive/<issue-id>/`, keeping its issue-id, its five-file shape, and its bytes, except relative links
that must gain one `../` level. Live links pointing at it are updated in the same change, and `i-<nnn>` numbers stay
unique across active and archived issues. A complete move IS preservation: every history oracle compares content at
the archived destination. A partial move, a content change, or archiving a non-completed issue is a failure.
- **Reviewable snapshot.** Evidence may only claim what a reviewer can reconstruct. When you record acceptance or
merge-gate evidence, every file under the declared evidence roots (`src/`, `test/`, `.arca/`) must be tracked or
staged; anything untracked or unstaged is either committed, staged, or declared as an explicit exception in the
record. Store the snapshot manifest — path, tracking state, SHA-256 — beside the evidence that cites it
(`ratmac_qa::snapshot::record_snapshot`).
- **Append-only history.** `.arca/log.md` is the one history file that changes in place, and only by appending: its
recorded prefix must survive byte for byte. A rewrite of any earlier line is a preservation failure, exactly like an
edit to an archived issue file. Who appends depends on who is driving: while no `rtm` Run drives this repository -
no `.arca/state.toml` - the contributor loop appends one line per closure. Once a Run is active the Engine owns the
file (see [Evidence receipts](#evidence-receipts)); then no agent writes it, and `rtm` records every entry.
- **Out-of-ticket trace.** Work landed outside the ticketed system — docs, config, tooling, harness edits — still
appends one `- YYYY-MM-DD: <what landed, where, why>` line to `.arca/log.md` before the session ends. Subsequent
sessions read the log first instead of reconstructing changes from `git diff`/history.
- **Release acceptance lane opt-in.** Environment-coupled release checks (live GitHub identity, exact origin, branch,
clean worktree) run only with `RATMAC_RELEASE_ACCEPTANCE=1`. Plain `cargo test --workspace` skips that lane and
prints the skip; never make branch work depend on operator-cutover facts.

## Blocked route

A ticket blocked for an out-of-scope reason is held, never quietly passed:

```text
rtm hold <ticket-id> --blocker <issue folder or residual> --confirm "hold <ticket-id>"
```

The confirmation phrase is the human's act - typed at invocation, never read
from a file an agent can write. The Engine keeps no caller identity (ORS-001);
it checks only that the exact phrase was typed. The blocker must resolve to a
complete five-file issue folder or a named residual record, and the current
Phase must declare a blocked route:

```toml
[[transitions]]
from = "build"
to = "intake"
blocked-route = true
```

`rtm step` never takes a blocked route, so ordinary routing stays
deterministic. An authorized hold marks the ticket `held` with its
`blocker-ref`, routes the Run, and appends one history entry; the ticket stays
not-passed, its residuals stay unproven, and the completion gate refuses it by
name. Anything else refuses before the first write, and an interrupted hold
rolls every touched file back: the Run is pre-route or fully routed, never in
between.

## Abandoning a Run

A Run that cannot be repaired is retired by `rtm`, never by hand:

```text
rtm abandon --confirm "abandon <project directory name>"
```

Agents never delete or edit `.arca/state.toml`, `.arca/log.md`,
`.arca/evidence.toml`, or `.arca/rtm.lock`; this command is the only path that
retires them. On the exact phrase - typed at invocation, never read from a file

- `rtm` records a terminal abandoned event naming the retired Phase, status,
and goal revision, then retires the admission state, the Run evidence, and the
lock, so a fresh `rtm start` can begin and records its own baseline and pins.

A stale lock is retired through this same path; no bypass flag exists.
Everything unconfirmed refuses before the first write, and a retirement that
cannot finish restores every file it touched - the Run stays active rather than
half retired, and re-running the confirmed command finishes the job. A leftover
lock with no admission state is retired without a second terminal event.

## Completion gate

Passing a ticket is evidence, not a status edit. `completion_gate` reads the
ticket's declared work - its planned tests, its hidden lanes, and every
backticked command in its Merge Gate - and requires one receipt per check at
`.arca/evidence/<ticket-id>/completion/<check>.toml`, recording the command,
working directory, exit status, output digest, and the source roots with their
digest at the time the check ran.

A receipt counts only when it is green (`exit-status = 0`), its digest
re-derives from its own recorded output, its command names a program that
exists, and its `tree-sha256` still matches the declared roots. Edit the work
after the check and the receipt goes stale by construction. A receipt for a
check the ticket never declared refuses too.

The gate verifies rather than runs: ETB-001 forbids rebuilding project source
at evaluation time, so the agent runs the check and records it, and the gate
re-derives the claim. A refusal names the first missing receipt and writes
nothing.

## Contract gates

Two gate kinds read the records themselves, so a status edit cannot route the
loop. Declare them in the Runbook phase that must not be left without them:

- `intake_contract` — every direct issue folder ends `integrated` or
`rejected`, keeps its five-file shape exactly, states accepted requirement
IDs that exist in the goal, and links that resolve in both directions.
- `record_contract` — exactly one residual per requirement, each citing the
frozen goal revision; `satisfied` only with concrete evidence references;
every `missing`/`partial` residual owned by exactly one ticket; acyclic
ticket dependencies; every ticket carrying its five sections and all six
hidden-lane assessments.

A refusal names the offending artifact and what it found.

- **No satisfaction by absence.** A loop that declares no gate of a required
kind classifies that gate's requirement `missing`, whatever its records say,
and the record gate refuses a `satisfied` claim resting on that absence.

## Evidence receipts

`.arca/evidence/` is agent-writable. When a Run drives the loop, agents record
one structured receipt per executed check at
`.arca/evidence/<ticket-id>/<planned-test-id>.toml` (planned-test ID, ticket,
sensitivity kind, command, working directory, test file and name, exit status,
recorded output, and a SHA-256 over that output). The P4 gate
(`kind = "sensitivity_receipts"`) resolves every planned test the ticket
declares to such a receipt; prose lines, filename conventions, and status
fields satisfy nothing, and a passing run is not a sensitivity receipt.

This repository's own loop runs no Run, so no gate consumes receipts here and
none are written. The same property is carried by artifacts a reviewer can
re-derive: every residual cites the exact test file and test names behind each
claim, the mutations that kill each lane, and a snapshot manifest of path,
tracking state, and SHA-256. A claim resting on prose alone is a defect in
either loop.

Scheduler-owned files - `.arca/state.toml`, `.arca/log.md`, `.arca/rtm.lock` -
belong to `rtm` for as long as a Run is active: while one exists, `rtm` writes
them and no agent does. Independently of any Run, no Phase Prompt and no gate
contract may ever instruct an agent to write them - that is the unconditional
rule `ratmac::ownership::audit_ownership` enforces, and it is why an
agent-authored note belongs in `.arca/evidence/` instead.

With no active Run in a repository - no `.arca/state.toml` - the
[append-only history](#defaults) rule governs `.arca/log.md`: the contributor
loop appends one line per closure. That is the only condition under which
anything but `rtm` appends to it.

## Evidence kinds

A claim about what a caller *invoked* is proven only by behavioral evidence: a
recorded role scenario under `test/qa/fixtures/role-scenarios/` listing the
attempted commands or tool calls and whether each was invoked or refrained.
A check over document wording is guidance-consistency evidence. Every emitted
check names its kind first (`ratmac_qa::role::Check::render`), and a
guidance-consistency check can never satisfy a behavioral requirement.

## Bootstrap

One command, run from the project root, makes the Engine usable:

```
pwsh -File tools/rtm.ps1
```

It resolves the Engine from the project-local build - building it there when
absent - hashes it, compares it against the `[engine]` pin in
`.arca/evidence.toml` when a Run recorded one, and prints the resolved path and
SHA-256. A pin mismatch refuses naming observed and expected identity instead
of reporting success.

Nothing is installed, no PATH or global configuration is written, and no
network is used: the build runs offline, and the only paths it may write are
the declared build output, `target` and `Cargo.lock`. To orient afterwards, run
`rtm doctor`: it reports Engine identity, Runbook validity, and runtime state,
names the next legitimate action when no Run exists, and writes nothing.

## Trial worktrees

Experiments live in trials, not on the experiment base. One entry point owns
the lifecycle: `pwsh -File tools/trial.ps1 <verb>`, run from the repository
root with `start`, `status`, `finish`, or `sync` - nothing else, and nothing
that pushes, fetches, installs, or reaches the network.

- `status` is a dry run: it prints what each verb would do and applies nothing.
- `start` opens `trial-<nnn>-<slug>` with its linked worktree beside the
repository, from a clean `exp/ratmac-deterministic` checkout only.
- `finish` archives the trial: annotated tag first, then the durable log
`trials/<trial-branch>/trial-log.md` committed alone on the base, then the
worktree, then the branch. That log is the only trial content that outlives
the trial; `.arca/tpl/trial-log.md` is its blank form.
- `sync` merges `main` into a clean base checkout. Fixes are authored on
`main`; the base never receives them any other way.

**Ownership.** A human or the Main-Agent invokes these verbs, from the primary
checkout with the experiment base checked out. An Advisor authors trial log
content only and invokes no lifecycle verb. A Subagent invokes neither a
lifecycle verb nor `rtm`.

**Working directory (Windows).** Run the verbs from the primary checkout,
never with your working directory inside a trial worktree: Windows refuses to
remove a directory somebody is standing in. `finish` refuses that case by name
and tells you where to `cd`; a worktree held by another shell or editor
refuses the same way - close it, because nothing here forces a removal or
kills a process.

# Rule

Do:

- Enter on any work-shaped request; catch up missing earlier steps yourself.
- Prefer work-it-out &gt; safe guess &gt; ask; log every guess; put all questions in one message.
- Write each ticket's tests in P4, before its code. Keep hidden test code in `.arca-private/`, listed in the
owning ticket.
- Fill in real files from the blanks in `.arca/tpl/`; keep `.arca/log.md` new-lines-only.
- Found a goal problem mid-build? File a new issue in `.arca/issue/` — that is the only road back.

Don't:

- Don't apply `.arca/goal/` product rules to working files — decide by rule set and keep going.
- Don't write state yourself — only `rtm` writes `state.toml`; `blocked` marks missing entry prerequisites only (.arca/goal/design.md, ADR-0006).
- Don't mark a gap record `satisfied` without proof.
- Don't edit the frozen goal while tickets are open.
- Don't stop on failed checks or rule clashes — fix, log, keep going.

# Worked example

User: "I want to implement the rust tests based on test/test-list.md" →

1. Route: **P4**. Catch up: P1 (fold in any `pending` issues, else log the skip) → P2 (freeze = note git
 HEAD; write one gap record per requirement) → P3 (one approved ticket per `missing|partial` record).
2. Loop, one ticket at a time: **P4** — create the `test/qa/` cargo crate if absent; write one `#[test]` per
 TP/CT mapping recorded in the ticket; poke holes in your own tests; `cargo test` (failing is expected).
 **P5** — write the code; `cargo test` full suite; hidden-lane pass; fix; review; next ticket.
3. No tickets left → redo the gap check → Idle.

Zero questions unless something truly cannot be worked out — then one batched message while every independent
piece keeps moving.