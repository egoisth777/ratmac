# The P-cycle as the real runbook

```yaml
issue-id: "i-015-cycle-as-runbook"
provenance: "Discovered while closing the i-011..i-014 sprint, 2026-07-28 - steering.md Current sprint endpoint is unmet and the frozen goal carries no requirement for it; schema.md, The only road back"
status: "pending"
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
