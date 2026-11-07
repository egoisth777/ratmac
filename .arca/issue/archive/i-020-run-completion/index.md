# Run completion

```yaml
issue-id: "i-020-run-completion"
provenance: "Billy, 2026-07-30 - authorized the atomic cut of i-016-fsm-doctrine-convergence so Run completion stands apart from transition selection and input durability"
status: "integrated"
```

## Summary

This issue owns the Engine-observable end of a Run. Starting in or advancing into a terminal state writes `passed`. Explicit abandonment records a durable terminal event before retiring active state. Guard refusal remains non-terminal and changes nothing. The `failed` outcome stays deferred until a separate issue defines a concrete event the Engine can observe.

It retains requirement `FDC-002` under its existing identifier and has no dependency on input-routed branching or verdict consumption. It builds on the integrated per-Run residency contract only for the State File and evidence address. The doctrine-convergence issue ([i-016-fsm-doctrine-convergence](../i-016-fsm-doctrine-convergence/index.md)) remains the evidence seed: its adversarial-review ledger and decision records are cited here, never copied.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## P1 disposition

- 2026-07-30: `FDC-002` was dispositioned `deferred`; the issue closes this batch as `integrated` with zero accepted goal rows. Steering retains Run completion as the next Horizon direction for a later planning issue.

- 2026-07-31 correction: the preceding `integrated` conclusion is superseded. The Run-completion ask remains `deferred`, so the same five-file bundle stays live in the Deferred issue buffer with status `deferred`; no replacement issue carries it.

- 2026-08-03: `FDC-002` dispositioned `accepted`; the requirement entered the goal (spec/design/test-list/ubi-lang/index) and this bundle moved whole to the archive. The Ideal-shape property advanced is One writer, append-only.
