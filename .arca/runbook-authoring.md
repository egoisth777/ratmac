# Authoring a runbook

This file is procedure: how to write a runbook and how to repair one. It
defines nothing. Every fact about what may appear in a runbook lives in the
[runbook specification](runbook-spec.md), and every term below that means
something schematic is a link into it.

Write against the specification, not against an example. An example teaches its
accidents along with its rules.

## The loop

    rtm scaffold path/to/ratmac.toml      # start from something already clean
    <edit it>                             # one change at a time
    rtm doctor --json path/to/ratmac.toml # ask what is wrong
    <repair by code>                      # look the code up below
    <repeat until the exit code is 0>

The exit code is the verdict: `0` clean, `1` warnings only, `2` at least one
error. Branch on the code, never on the text.

Each finding in the JSON carries four fields:

- `code` - the stable name of the defect class. This is what you repair by.
- `severity` - `error` or `warning`.
- `location` - where to go: the Phase, the guard inside it, or the edge.
- `message` - prose for a human. A repair loop may read it, but never needs to.

Repair one finding, then run the doctor again. Findings interact: fixing the
first often changes the rest, and fixing them in a batch hides which repair
did what.

## Repair table

One row per code the doctor can emit. The last column is the smallest
mechanical repair - the one a script can make without understanding the
runbook. It is a floor, not a ceiling: a human author usually has a better
answer, and the row says so.

| Code | What the doctor saw | Repair | Action |
| :--- | :--- | :--- | :--- |
| `RB101` | Nothing readable at the path you named. | Create it with `rtm scaffold <path>`, or correct the path. | `restore-file` |
| `RB102` | The file is not valid TOML, so nothing else can be judged. | Fix the syntax the message quotes; if the draft is beyond saving, start again from a scaffold. | `restore-file` |
| `RB103` | A key appears where the schema declares a [closed set](runbook-spec.md#top-level). | Delete it, or move it to the table that accepts it. | `restore-location` |
| `RB104` | Runtime status was written into the definition. | Delete it - status belongs to the Run, and the [schema](runbook-spec.md#top-level) has no dimension for it. | `restore-file` |
| `RB105` | A required field is missing at the named location. | Add it; the [Phase](runbook-spec.md#phases) and [edge](runbook-spec.md#transitions) tables say which fields are required. | `restore-location` |
| `RB106` | A guard names a kind outside the [closed vocabulary](runbook-spec.md#guard-kinds). | Rename it to a kind in that table, or delete the guard. | `restore-location` |
| `RB107` | A guard carries a field its kind does not accept. | Delete the field, or switch to the kind that accepts it - see [Guard kinds](runbook-spec.md#guard-kinds). | `restore-location` |
| `RB108` | An edge names a Phase the runbook does not declare. | Declare that Phase, or delete the edge. | `drop-transition` |
| `RB109` | An edge carries a [freeze marker](runbook-spec.md#transitions) that is not the one allowed. | Use the allowed value, or remove the marker. | `drop-transition` |
| `RB110` | A key carries the wrong type of value. | Give it the type its [table](runbook-spec.md#phases) states. | `restore-location` |
| `RB201` | The runbook declares no Phase at all. | Start again from a scaffold; a machine needs somewhere to be. | `restore-file` |
| `RB202` | Every Phase has a way in, so a Run has nowhere to start. | Remove one edge to break the cycle. | `break-cycle` |
| `RB203` | More than one Phase has no way in, so the entry point is ambiguous. | Route the extra entry points into the machine, leaving exactly one. | `merge-initial` |
| `RB204` | A Phase cannot be reached from the initial Phase. | Connect it if it is meant to run; delete it if it is dead. | `drop-phase` |
| `RB205` | The machine has more than one ending. | Route one ending into the other. If several endings are intended, keep them and accept the warning. | `connect-terminal` |
| `RB206` | The same edge is declared twice. | Delete the duplicate; it adds no route. | `drop-transition` |
| `RB207` | An edge leaves and enters the same Phase. | Delete it, or point it at a Phase that makes progress. | `drop-transition` |
| `RB301` | A command gate names a program that cannot be pinned. | Name a program that exists, or mark the gate exempt if it deliberately runs unpinned code. | `pin-command` |
| `RB302` | A gate's verdict rests on content the agent under test can write. | Delete the gate, or point it at something that agent cannot write. If the weaker gate is intended, keep it and accept the warning. | `drop-guard` |
| `RB401` | A prompt or gate contract tells an agent to write a Scheduler-owned file. | Rewrite the sentence so the agent produces evidence instead - see [Ownership](runbook-spec.md#ownership). | `restore-location` |

## Action vocabulary

The action tokens above are the mechanical repairs, defined here so a script
and a human read the same table:

| Action | Meaning |
| :--- | :--- |
| `restore-file` | Replace the whole file with a fresh scaffold. |
| `restore-location` | Replace what is at the named location with the scaffold's version of it, dropping what was added there. |
| `drop-transition` | Delete the edge the location names. |
| `drop-phase` | Delete the Phase the location names, and every edge touching it. |
| `break-cycle` | Delete the last edge, breaking the cycle that leaves no entry point. |
| `merge-initial` | Add an edge from another entry candidate into this one, so only one remains. |
| `connect-terminal` | Add an edge from this ending to another one, leaving a single ending. |
| `pin-command` | Mark the named command gate exempt from pinning. |
| `drop-guard` | Delete the guard the location names. |

## Before you finish

- The doctor exits `0` with no findings. Warnings are findings.
- Run it once more after your last edit: a repair can introduce a defect.
- Read the [specification](runbook-spec.md) for anything this file did not
  answer. If it answers something the specification should own, the
  specification is wrong, not this file.
