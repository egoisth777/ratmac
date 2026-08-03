# Authoring a runbook

This file is procedure: how to write a runbook and how to repair one. It
defines nothing. Every fact about what may appear in a runbook lives in the
[runbook specification](runbook-spec.md), and every term below that means
something schematic is a link into it.

Write against the specification, not against an example. An example teaches its
accidents along with its rules. Where your machine ends is also defined there:
a [terminal Phase](runbook-spec.md#transitions) is structural, and the Engine —
never the runbook — writes the lifecycle status when a Run reaches one.

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
| `RB208` | A Phase's [`inputs`](runbook-spec.md#phases) declaration is empty, duplicated, or not a list of non-empty strings. | Replace it with the intended closed list of unique exact values. | `restore-location` |
| `RB209` | A branching Phase has no [`inputs`](runbook-spec.md#phases) list. | Declare and label the intended closed branch manually. The safe mechanical fallback keeps its first ordinary edge as an unlabelled straight line. | `straighten-branch` |
| `RB210` | A legal transition value has no ordinary outgoing edge. | Add the missing labelled edge or remove the unintended list value manually. The safe mechanical fallback keeps the first ordinary edge as an unlabelled straight line. | `straighten-branch` |
| `RB211` | Two ordinary outgoing edges carry the same transition value. | Delete the duplicate edge, or relabel it with the one uncovered legal value. | `drop-transition` |
| `RB212` | An ordinary edge label is foreign, mixed with unlabelled branch edges, or forbidden on a straight line or terminal. | Repair the exact labels manually. The safe mechanical fallback removes the transition-value contract and keeps the first ordinary edge as a straight line. | `straighten-branch` |
| `RB213` | A blocked route carries an [`input`](runbook-spec.md#transitions) label. | Delete that field from the blocked route; blocked routes never participate in transition selection. | `drop-transition` |
| `RB214` | A cycle keeps a Phase with no receipt- or contract-class guard, so nothing proves the loop ends. | Give every Phase on the cycle a [receipt- or contract-class guard](runbook-spec.md#guard-kinds). The mechanical fallback deletes the last edge, breaking the cycle. | `break-cycle` |
| `RB301` | A command gate names a program that cannot be pinned. | Name a program that exists, or mark the gate exempt if it deliberately runs unpinned code. | `pin-command` |
| `RB302` | A gate's verdict rests on content the agent under test can write. | Delete the gate, or point it at something that agent cannot write. If the weaker gate is intended, keep it and accept the warning. | `drop-guard` |
| `RB401` | A prompt or gate contract tells an agent to write a Scheduler-owned file. | Rewrite the sentence so the agent produces evidence instead - see [Ownership](runbook-spec.md#ownership). | `restore-location` |
| `RB501` | The classes table is malformed at the named location. | Rewrite it as a table of named class bodies - see [Classes and spawns](runbook-spec.md#classes-and-spawns) - or delete it. | `restore-location` |
| `RB502` | A class's binding declarations are malformed. | Rewrite each binding as a small table carrying at most a boolean requirement flag, or delete the declaration. | `restore-location` |
| `RB503` | A Phase's spawn declarations are malformed. | Rewrite them as an array of tables, each naming a declared class and a child, or delete the declaration. | `restore-location` |
| `RB504` | A spawn names a class the runbook does not declare. | Declare that class, or delete the spawn entry. | `restore-file` |
| `RB505` | A spawn's binding names do not match the class's required set. | List exactly the required binding names on the spawn entry. | `restore-file` |
| `RB506` | A join gate carries a verdict rule outside the closed vocabulary or a child count below one. | Use the one accepted rule and a count of at least one, or delete the gate. | `drop-guard` |

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
| `straighten-branch` | At the named Phase, remove [`inputs`](runbook-spec.md#phases), keep its first ordinary outgoing edge, remove that edge's [`input`](runbook-spec.md#transitions), and delete its other ordinary outgoing edges. Blocked routes are unchanged. |
| `pin-command` | Mark the named command gate exempt from pinning. |
| `drop-guard` | Delete the guard the location names. |

## Before you finish

- The doctor exits `0` with no findings. Warnings are findings.
- Run it once more after your last edit: a repair can introduce a defect.
- Read the [specification](runbook-spec.md) for anything this file did not
  answer. If it answers something the specification should own, the
  specification is wrong, not this file.

## Composing

To declare a composed machine (FDC-009): put each child class inline under
the [`classes`](runbook-spec.md#classes-and-spawns) table - a class body is a
whole machine under the same rules as the top level - declare the binding
names the class requires under its
[`bindings`](runbook-spec.md#classes-and-spawns) table, and give the spawning
Phase one [`spawns`](runbook-spec.md#classes-and-spawns) entry per child,
naming a declared [`class`](runbook-spec.md#classes-and-spawns), a
Phase-unique [`name`](runbook-spec.md#classes-and-spawns), and a
[`bind`](runbook-spec.md#classes-and-spawns) list covering the class's
required bindings exactly. Guard the Phase that waits on children with the
[`join`](runbook-spec.md#guard-kinds) kind. Declarations are dormant data:
nothing spawns until the Engine's spawn verb does, and a join gate honestly
refuses while no ledger records children. The format is one level deep: class
bodies accept no nested class tables and their Phases no spawn tables
(FDC-012's shape). The repair loop is unchanged: `rtm doctor --json` names
composition defects as `RB501`-`RB506`.

Creating and superseding those children is Engine motion, split by kind
(FDC-007). `rtm spawn <spawn name> --run <parent id>` is ordinary checked
motion - no confirmation phrase - legal only while the parent stands in the
Phase declaring that entry; the child lands on the flat roster as an ordinary
Run. `rtm respawn --run <id> --confirm "respawn <id>"` and
`rtm abandon --run <id> --confirm "abandon <id>"` are human-confirmed by
phrases naming that run id, typed at invocation and never read from a file.
A respawn mints a fresh successor id - the superseded record keeps its
address - and retires the superseded Run by the abandon path. Retiring only
a leftover lock, with no live run anywhere, still confirms with the project
directory name.
