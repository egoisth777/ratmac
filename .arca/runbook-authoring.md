# Authoring a runbook

This file is procedure: how to write a runbook and how to repair one. It
defines nothing. Every fact about what may appear in a runbook lives in the
[runbook specification](runbook-spec.md), and every term below that means
something schematic is a link into it.

Write against the specification, not against an example. An example teaches its
accidents along with its rules. The [State Prompt](runbook-spec.md#states) and
where your machine ends are defined there: a [terminal State](runbook-spec.md#transitions)
is structural, and the Engine — never the runbook — writes the lifecycle status
when a Run reaches one.

A [Run Record](runbook-spec.md#ownership) is `.ratmac/runs/<run-id>/run.toml`;
its [`state` machine-position field](runbook-spec.md#ownership) is
Engine-owned: agents read it, never write it.

## The loop

    rtm scaffold path/to/ratmac.toml      # start from something already clean
    <edit it>                             # one change at a time
    rtm doctor --json path/to/ratmac.toml # ask what is wrong
    <repair by code>                      # look the code up below
    <repeat until the exit code is 0>

The [doctor contract](runbook-spec.md#diagnostics) defines the exit-code
verdict and machine-readable finding fields. Branch on a finding's `code`,
never its prose `message`.

Repair one finding, then run the doctor again. Findings interact: fixing the
first often changes the rest, and fixing them in a batch hides which repair
did what.

## Repair table

One row per [code the doctor can emit](runbook-spec.md#diagnostics). The last column is the smallest
mechanical repair - the one a script can make without understanding the
runbook. It is a floor, not a ceiling: a human author usually has a better
answer, and the row says so.

| Code | What the doctor saw | Repair | Action |
| :--- | :--- | :--- | :--- |
| `RB101` | Nothing readable at the path you named. | Create it with `rtm scaffold <path>`, or correct the path. | `restore-file` |
| `RB102` | The file is not valid TOML, so nothing else can be judged. | Fix the syntax the message quotes; if the draft is beyond saving, start again from a scaffold. | `restore-file` |
| `RB103` | A key appears where the schema declares a [closed set](runbook-spec.md#top-level). | Delete it, or move it to the table that accepts it. | `restore-location` |
| `RB104` | Runtime status was written into the definition. | Delete it - status belongs to the Run, and the [schema](runbook-spec.md#top-level) has no dimension for it. | `restore-file` |
| `RB105` | A required field is missing at the named location. | Add it; the [State](runbook-spec.md#states) and [edge](runbook-spec.md#transitions) tables say which fields are required. | `restore-location` |
| `RB106` | A guard names a kind outside the [closed vocabulary](runbook-spec.md#guard-kinds). | Rename it to a kind in that table, or delete the guard. | `restore-location` |
| `RB107` | A guard carries a field its kind does not accept. | Delete the field, or switch to the kind that accepts it - see [Guard kinds](runbook-spec.md#guard-kinds). | `restore-location` |
| `RB108` | An edge names a [State](runbook-spec.md#transitions) the runbook does not declare. | Declare that [State](runbook-spec.md#transitions), or delete the edge. | `drop-transition` |
| `RB109` | An edge carries a [freeze marker](runbook-spec.md#transitions) that is not the one allowed. | Use the allowed value, or remove the marker. | `drop-transition` |
| `RB110` | A key carries the wrong type of value. | Give it the type its [table](runbook-spec.md#states) states. | `restore-location` |
| `RB111` | The runbook still declares the pre-cutover [`phases`](runbook-spec.md#states) table. | Rename it to [`states`](runbook-spec.md#states) (and any `[[phases.<name>.spawns]]` to `[[states.<name>.spawns]]`, `[classes.<name>.phases]` to `[classes.<name>.states]`); nothing is migrated automatically. | `rename-states` |
| `RB112` | A [per-item guard](runbook-spec.md#guard-kinds) does not address exactly one item: it declares both `ticket` and `ticket-binding`, neither, or an empty one. | Keep one address. Write `ticket` when the runbook knows the item; write `ticket-binding` when the caller supplies it at spawn. | `restore-location` |
| `RB601` | The [`roots`](runbook-spec.md#roots) declaration is not a table of safe non-empty relative paths. | Replace it with named non-empty role/path pairs; remove an absolute or escaping path. | `restore-location` |
| `RB602` | A guard names an undeclared root role, including a fixed contract role. | Declare that role in `[roots]`, or rename/remove the guard reference. | `restore-location` |
| `RB603` | A declared root path is absent or unreadable in this repository. | Create or restore the intended directory, or change the role to an existing repository-relative directory. | `restore-location` |
| `RB604` | A declared root overlaps the Engine runtime root. | Point the role at a non-overlapping project directory; never use the Engine root or an ancestor of it. | `restore-location` |
| `RB201` | The runbook declares no [State](runbook-spec.md#transitions) at all. | Start again from a scaffold; a machine needs somewhere to be. | `restore-file` |
| `RB202` | Every [State](runbook-spec.md#transitions) has a way in, so a Run has nowhere to start. | Remove one edge to break the cycle. | `break-cycle` |
| `RB203` | More than one [State](runbook-spec.md#transitions) has no way in, so the entry point is ambiguous. | Route the extra entry points into the machine, leaving exactly one. | `merge-initial` |
| `RB204` | A [State](runbook-spec.md#transitions) cannot be reached from the initial [State](runbook-spec.md#transitions). | Connect it if it is meant to run; delete it if it is dead. | `drop-state` |
| `RB205` | The machine has more than one ending. | Route one ending into the other. If several endings are intended, keep them and accept the warning. | `connect-terminal` |
| `RB206` | The same edge is declared twice. | Delete the duplicate; it adds no route. | `drop-transition` |
| `RB207` | An edge leaves and enters the same [State](runbook-spec.md#transitions). | Delete it, or point it at a [State](runbook-spec.md#transitions) that makes progress. | `drop-transition` |
| `RB208` | A [State's](runbook-spec.md#states) [`inputs`](runbook-spec.md#states) declaration is empty, duplicated, or not a list of non-empty strings. | Replace it with the intended closed list of unique exact values. | `restore-location` |
| `RB209` | A branching [State](runbook-spec.md#states) has no [`inputs`](runbook-spec.md#states) list. | Declare and label the intended closed branch manually. The safe mechanical fallback keeps its first ordinary edge as an unlabelled straight line. | `straighten-branch` |
| `RB210` | A legal transition value has no ordinary outgoing edge. | Add the missing labelled edge or remove the unintended list value manually. The safe mechanical fallback keeps the first ordinary edge as an unlabelled straight line. | `straighten-branch` |
| `RB211` | Two ordinary outgoing edges carry the same transition value. | Delete the duplicate edge, or relabel it with the one uncovered legal value. | `drop-transition` |
| `RB212` | An ordinary edge label is foreign, mixed with unlabelled branch edges, or forbidden on a straight line or terminal. | Repair the exact labels manually. The safe mechanical fallback removes the transition-value contract and keeps the first ordinary edge as a straight line. | `straighten-branch` |
| `RB213` | A blocked route carries an [`input`](runbook-spec.md#transitions) label. | Delete that field from the blocked route; blocked routes never participate in transition selection. | `drop-transition` |
| `RB214` | A cycle keeps a [State](runbook-spec.md#transitions) with no receipt- or contract-class guard, so nothing proves the loop ends. | Give every [State](runbook-spec.md#transitions) on the cycle a [receipt- or contract-class guard](runbook-spec.md#guard-kinds). The mechanical fallback deletes the last edge, breaking the cycle. | `break-cycle` |
| `RB301` | A command gate names a program that cannot be pinned. | Name a program that exists, or mark the gate exempt if it deliberately runs unpinned code. | `pin-command` |
| `RB302` | A gate's verdict rests on content the agent under test can write. | Delete the gate, or point it at something that agent cannot write. If the weaker gate is intended, keep it and accept the warning. | `drop-guard` |
| `RB401` | A prompt or gate contract tells an agent to write a Scheduler-owned file. | Rewrite the sentence so the agent produces evidence instead - see [Ownership](runbook-spec.md#ownership). | `restore-location` |
| `RB501` | The classes table is malformed at the named location. | Rewrite it as a table of named class bodies - see [Classes and spawns](runbook-spec.md#classes-and-spawns) - or delete it. | `restore-location` |
| `RB502` | A class's binding declarations are malformed. | Rewrite each binding as a small table carrying at most a boolean requirement flag, or delete the declaration. | `restore-location` |
| `RB503` | A [State's](runbook-spec.md#classes-and-spawns) spawn declarations are malformed. | Rewrite them as an array of tables, each naming a declared class and a child, or delete the declaration. | `restore-location` |
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
| `drop-state` | Delete the [State](runbook-spec.md#states) the location names, and every edge touching it. |
| `break-cycle` | Delete the last edge, breaking the cycle that leaves no entry point. |
| `merge-initial` | Add an edge from another entry candidate into this one, so only one remains. |
| `connect-terminal` | Add an edge from this ending to another one, leaving a single ending. |
| `straighten-branch` | At the named [State](runbook-spec.md#states), remove [`inputs`](runbook-spec.md#states), keep its first ordinary outgoing edge, remove that edge's [`input`](runbook-spec.md#transitions), and delete its other ordinary outgoing edges. Blocked routes are unchanged. |
| `pin-command` | Mark the named command gate exempt from pinning. |
| `drop-guard` | Delete the guard the location names. |

## Before you finish

- The [doctor](runbook-spec.md#diagnostics) exits `0` with no findings. Warnings are findings.
- Run it once more after your last edit: a repair can introduce a defect.
- Read the [specification](runbook-spec.md) for anything this file did not
  answer. If it answers something the specification should own, the
  specification is wrong, not this file.

## Composing

For a composed machine (FDC-009), use
[Classes and spawns](runbook-spec.md#classes-and-spawns) and
[Guard kinds](runbook-spec.md#guard-kinds) in the specification. They define
the declared shape; this procedure does not repeat it. The repair loop is
unchanged: `rtm doctor --json` names composition defects as `RB501`-`RB506`.

Creating and superseding those children is Engine motion, split by kind
(FDC-007). `rtm spawn <spawn name> --run <parent id>` is ordinary checked
motion - no confirmation phrase - legal only while the parent stands in the
[State](runbook-spec.md#classes-and-spawns) declaring that entry; the child lands on the flat roster as an ordinary
Run. `rtm respawn --run <id> --confirm "respawn <id>"` and
`rtm abandon --run <id> --confirm "abandon <id>"` are human-confirmed by
phrases naming that run id, typed at invocation and never read from a file.
A respawn mints a fresh successor id - the superseded record keeps its
address - and retires the superseded Run by the abandon path. Retiring only
a leftover lock, with no live run anywhere, still confirms with the project
directory name.
