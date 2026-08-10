# State, not Phase, for the machine position

```yaml
issue-id: "i-025-state-vocabulary"
provenance: "Promoted from the wishlist entry 'Use State, not Phase, for machine position' (Billy, filed 2026-07-30, promoted 2026-08-10) in [.arca/wishlist.md](../../wishlist.md)."
status: "pending"
```

## Summary

The Engine calls the position in the machine graph a `Phase`. That word says
"one step of a linear process", which is not what the thing is: a runbook is a
general state machine whose position can branch, loop back, and end anywhere.
Everyone who reads the runbook format learns the wrong model from the word
itself, and the format is meant to be learned from the written schema alone.

Renaming the position to `State` collides with two words already in use: the
per-Run file the Engine writes is called the `State File`, and the lifecycle
value inside it is called `status`. This issue removes the collision before the
rename, by giving three separate names to three separate things:

- **State** — where the Run currently is in the machine graph. Nothing else.
- **Run Record** — the one file the Engine writes for one Run. It records the
  Run's `state`, its `status`, the revisions in play, and its blocker.
- **Run** — the whole live instance of a machine: its Run Record, its evidence,
  its lock, its ledger.

`status` keeps its present meaning and its present five values, and stays the
only lifecycle word.

The cutover is the runbook format, the Run Record, every message a caller
reads, the Engine source, the tests, and the working rules. Old bytes are not
rewritten: archived issues, tickets, gap records, and the history file keep
their wording, and a project still carrying the old spelling is refused with
instructions rather than migrated in place.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
