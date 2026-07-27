# Trial log: trial-001-issue-authoring

## Identity

- trial: trial-001-issue-authoring
- base commit: 6d966bca1d0aa727839972ef98357aec74013f2d
- terminal commit: 3df15b8620248ff57db2059be201b18dd2e68415

## Hypothesis

Authoring one new issue from the blanks in `.arca/tpl/issue/` is a supported,
checkable act. Expected: the contributor can derive the next issue number,
scaffold the five files, fill them honestly, and get a mechanical verdict on
the result before P1 folds it in.

## Procedure

1. Read all five blanks and searched `tools/` and `src/` for any issue
   scaffold affordance.
2. Derived the next issue number by hand and investigated the gap at i-003,
   i-004 and i-005.
3. Created the folder by `mkdir` plus `cp` of the five blanks, and ran the
   only runnable contributor check against the untouched copies, then again
   with `status` set to `pending`, then again with `status` set to
   `integrated` and everything else still blank.
4. Wrote a throwaway probe calling the product gate `ratmac::contract::gate_intake`
   over the same folder, and read the gate body to confirm what it inspects.
5. Then authored the issue for real: filled all five files end-to-end as a
   contributor would, with six requirement records, a term table, proposed
   mechanics, nine verification rows, and both trace tables.
6. Ran both checks again against the finished, `pending` issue.
7. Probed the disposition parser with a row whose disposition column says
   rejected and whose rationale contains the word accepted.

## Commands and tests

- `python tools/check_links.py` at four states of the folder.
- `cargo test -p ratmac-qa --test trial001_probe -- --nocapture --test-threads=1`
  (`test/qa/tests/trial001_probe.rs`, throwaway, not for merge).
- `grep` over `tools/*.ps1`, `tools/*.py`, `src/*.rs` for scaffold affordances,
  and over the seven issue folders for the hand-repeated disposition note.
- `git log --all --oneline --diff-filter=A` over `.arca/issue/i-003*`, `i-004*`,
  `i-005*`; `git reflog --all`.

## Observations

1. The checks are inverted. A folder that is nothing but 30 unfilled
   placeholders, with `issue-id` literally the placeholder and a placeholder
   provenance, PASSES both checks as long as `status` reads `integrated`:
   `check_links.py` printed `intake shape check: all links resolve, all
   accepted requirement IDs present` and exited 0, and `gate_intake` returned
   PASS. The finished, correctly authored issue with zero placeholders is
   REFUSED by both, for the single reason that its status is `pending`. Status
   is the only discriminator either check applies, and status is a P1 output,
   not an author input. A garbage issue passes and a good issue fails.

2. Three of the four front-matter rules stated in `.arca/index.md` are
   enforced by nothing. Reading the `gate_intake` body confirms it checks only
   five-file shape, terminal status, accepted IDs present in the goal, and
   link resolution. Issue-id equal to folder name, non-empty origin, and no
   unfilled template blanks are asserted in prose and mechanized nowhere,
   although the same file says the schema gate is mechanized by rtm Exit
   Guards.

3. The blanks demand integrator fields from the author. `.arca/index.md`
   assigns requirement IDs and dispositions to P1, yet `spec.md` asks the
   author for both, and `test-plan.md` asks for `updated` or `unaffected` goal
   trace rows that only exist after integration. This is not theoretical: five
   of the seven issues written so far hand-repeat the identical sentence
   `Dispositions below record the author's proposed decision; P1 confirms or
   revises them at integration.` It appears zero times in the blank. Every
   issue from i-006 onward carries it; i-001 and i-002 do not. A workaround
   repeated five times has never been promoted into the template.

4. The disposition parser reads the whole row. `accepted_requirements`
   lowercases the entire line and tests whether it contains the word accepted,
   so the probe row whose disposition column says rejected and whose rationale
   reads `we have not accepted this, and will not` is counted as an accepted
   requirement. Any rationale mentioning the word mis-classifies its own row.

5. No scaffold exists, and no numbering rule is written. Authoring is `mkdir`
   plus `cp` plus 30 hand edits. i-003, i-004 and i-005 have no trace in
   `.arca/log.md`, in any document, or in git history on any ref, so the
   numbering carries a permanent silent hole with nothing stating whether the
   next number is highest-plus-one or the lowest free slot. Trials are treated
   far better: TWL-002 makes `trial.ps1` derive branch, worktree, tag and
   durable-log identity and collision-check all four.

6. The repository already knows how to do this correctly one directory away.
   The trial-log validator in `tools/trial.ps1` refuses a missing section, an
   empty section, and an unfilled angle-bracket placeholder. Trial logs get
   exactly the placeholder check that issue folders lack.

7. Minor: the issue blanks are CRLF and use double-brace placeholders, while
   the trial-log blank uses angle brackets. A scaffold must preserve the line
   endings to keep diffs clean.

## Verdict

adopt: issue authoring is unscaffolded and inversely checked - a blank folder passes both checks while a fully authored one is refused, and three stated front-matter rules are mechanized nowhere.

## Recommendations

File `i-011-issue-authoring-scaffold` on `main`, main-first per TWL-007. It was
authored in full during this trial and is ready to copy across; its content is
the deliverable, and the trial branch is not. It proposes `IAS-001` through
`IAS-005` accepted and `IAS-006` deferred:

1. `IAS-001` adds placeholder, issue-id-equals-folder-name and non-empty
   provenance checks to `gate_intake`. This is the load-bearing item: it
   closes a hole inside accepted requirement PGE-001 rather than adding new
   surface.
2. `IAS-002` gives the pending state a real check by sharing one
   implementation with the intake gate and parameterizing the terminal-status
   rule, so the two cannot drift the way `check_links.py` and `gate_intake`
   already have.
3. `IAS-003` adds the scaffold with derived numbering and collision refusal,
   modelled directly on `trial.ps1` start.
4. `IAS-004` moves the five-times-repeated disposition sentence into the blank
   and separates author fields from integrator fields.
5. `IAS-005` fixes the disposition parser to read its column.

Do not carry `test/qa/tests/trial001_probe.rs` across; it is a throwaway.
Reuse the blank folder state at trial commit `e77f679` as the fixture for
`IASV-001` by reconstructing it, not by merging it.

## Artifacts and diffs

- Trial branch `trial-001-issue-authoring`, forked at 6d966bc.
- Commit `e77f679` - the deliberately blank probe folder, the state that passes
  both checks.
- Commit `3df15b8` - terminal; the fully authored five-file issue plus the
  parser probe.
- `.arca/issue/i-011-issue-authoring-scaffold/` - the authored issue, five
  files, zero placeholders, status `pending`.
- `test/qa/tests/trial001_probe.rs` - throwaway probe printing the gate verdict
  on the authored issue and the disposition mis-classification.
- `src/contract.rs` lines 79 to 201 for `gate_intake`, lines 478 to 500 for
  `accepted_requirements`.
- `.arca/index.md` lines 106 to 108 for the four stated front-matter rules.
