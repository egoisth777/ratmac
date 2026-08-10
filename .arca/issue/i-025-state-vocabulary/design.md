# Issue design

## Proposed mechanics

### 1. Settle the three words before renaming anything

| Idea | Name after the cutover | Where it lives |
| :--- | :--- | :--- |
| Where the Run sits in its machine graph | **State** | Declared in the runbook; recorded in the Run Record; printed in reports and findings |
| The one file the Engine writes for one Run | **Run Record** | `.ratmac/runs/<run-id>/run.toml` |
| The whole live instance of a machine | **Run** | Its Run Record, evidence, lock, and spawn ledger together |
| The Run's lifecycle value | `status` — unchanged | Inside the Run Record; never in a runbook |

Nothing else may be called "state" in a live surface. The word for the artifact
is Run Record, and the word for the runtime instance is Run.

### 2. Runbook format

Only the spelling of the position changes:

- top-level `[states.<name>]` instead of `[phases.<name>]`;
- `[[states.<name>.spawns]]` instead of `[[phases.<name>.spawns]]`;
- `[classes.<name>.states]` instead of `[classes.<name>.phases]`;
- `from` and `to` still hold State names; `inputs`, `guards`, `prompt`,
  `freeze`, `blocked-route`, `roots`, `classes`, `bindings`, and every guard
  kind keep their exact spelling and meaning.

The `status` prohibition is unchanged: `status` still may not appear anywhere in
a runbook, and the top-level `status` key is still its own diagnostic.

### 3. Run Record

`.ratmac/runs/<run-id>/run.toml` carries `state`, `status`, `goal_revision`,
`input_revision`, `output_revision`, `active_refs`, and `blocker`. Strict
parsing, atomic replacement, the single Engine writer, and the seven-field shape
are unchanged; only the file name and the first field's name move. The blank
form under `.arca/tpl/` moves with it.

### 4. Engine source

The type that names a position becomes `State`. The serialized record type
becomes the Run Record type. Two existing names need care rather than a blind
substitution:

- the type presently called `MachineState`, which holds only the current
  position, becomes redundant once the position type is itself `State`: it is
  either named for what it holds or folded into its holder;
- `Status` and every lifecycle value stay exactly as they are.

Module names follow their contents. No function is added or removed by this
work; a diff that changes control flow is out of scope by `SVC-007`.

### 5. Messages, prompts, and findings

Every caller-visible string that says "phase" says "state" instead, including
the State Prompt header, `rtm status` output, the doctor's human report, refusal
text, and the message column of the diagnostic table. Codes do not move
(`SVC-006`): `RB108` stays `RB108` while its message names a State.

### 6. Pre-cutover residue

Two new refusals, built like the pre-split residue preflight that already
exists:

- a runbook whose top level declares `phases` refuses with its own new
  diagnostic code naming the rename and the repair, rather than falling through
  to the generic unknown-key error;
- a Run Record carrying the old position field, or sitting at the pre-cutover
  filename, makes every addressed entry point refuse before its first read, path
  join, parse, or write, naming the artifact and the repair.

Neither path rewrites, moves, or migrates anything, matching the posture
`FDC-005` and `ENS-009` already set for a stale layout.

### 7. The audit and its allowlist

One executable check proves no live surface names the position `Phase`. Its
scope is the Engine source, the tests, the runbook and its blank forms, the
working rules, and the goal bundle's live rows. Its allowlist is enumerated, not
open-ended: the history file, archived issue bundles, archived tickets, archived
gap records, and the superseded goal rows that record earlier decisions. The
audit reads tracked content and does not descend into ignored directories — the
scoping defect that already made two existing audits fail from a nested
checkout.

### 8. Order of work

1. Settle the three words in the working rules and the glossary, so later work
   has one vocabulary to land against.
2. Move the runbook format and its parser, doctor, and scaffold, with the
   pre-cutover runbook refusal in the same step so no author is left guessing.
3. Move the Run Record — its file name, its field, and its residue refusal.
4. Move the remaining messages, reports, and Engine identifiers.
5. Prove the audit and its allowlist last, over the whole tree.

### 9. What could go wrong

- **A blind substitution changes behavior.** `SVC-007` makes unchanged behavior
  a requirement, so every existing check must keep its meaning and only its
  names may move.
- **The audit swallows a real occurrence.** An unbounded skip over history would
  hide a live one; `SVC-008` therefore requires the allowlist to be enumerated.
- **A live Run straddles the cutover.** It cannot: a Run Record written before
  the cutover refuses with instructions, and the operator retires that Run
  through the ordinary confirmed path.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
