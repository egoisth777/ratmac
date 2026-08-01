# The P-cycle as the real runbook

```yaml
issue-id: "i-015-cycle-as-runbook"
provenance: "Discovered while closing the i-011..i-014 sprint, 2026-07-28 - steering.md Current sprint endpoint is unmet and the frozen goal carries no requirement for it; schema.md, The only road back"
status: "deferred"
```

## Summary

The sprint route landed the Machine Class as first-class data - written specification, one typed reader,
deep doctor, authoring loop - but the endpoint the sprint was aimed at is not reached, and the frozen goal
never carried a requirement for it. `.arca/ratmac.toml` is still a demonstration machine (`build` ->
`build-review` -> `build-done`); the P1-P5 cycle this repository actually runs lives as prose in
`.arca/schema.md`, and "where are we" is answered by a human reading a lookup table in `.arca/index.md`
rather than by `rtm status`. Making the cycle the real runbook is the point of the engine: until then
ratmac is a state machine that has never been asked to run the one process it exists for.

Closing that gap surfaced four sub-gaps that are not cosmetic - they are the reason it has not happened
yet. They are recorded here as requirements, not as intentions.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Disposition log

- 2026-07-29: deferred at the 2026-07-29 planning pass (status stays `pending`) — it needs the
  dependency strata beneath it: integrated Run residency (`i-017-run-residency`), input-routed
  transitions (`i-016-fsm-doctrine-convergence`), input delivery and durability
  (`i-019-input-delivery-durability`), Run completion (`i-020-run-completion`), and machine
  composition (`i-018-machine-composition`). The additions record Billy's 2026-07-30 atomic cut;
  requirement identifiers stayed unchanged. The ordering among the new concerns is an assumed
  dependency forecast, revocable at planning step 1.
- 2026-07-30: all six asks were dispositioned `deferred`; the issue closes this batch as `integrated` with zero accepted goal rows, satisfying the planning-step terminal status without pretending the asks landed. Steering retains the direction in Horizon; a later planning issue may carry the deferred asks when their dependencies land.
