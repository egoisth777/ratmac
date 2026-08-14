# Every contract gate is proven on a repository with history

```yaml
issue-id: "i-030-gates-are-proven-on-history"
provenance: "Advisory on the run-002 postmortem, 2026-08-10: the record contract was permanently unpassable and the suite reported green for months, and i-029's own summary names the cause - every existing check builds a fixture with no past. i-029's ARF-003 closes that blind spot for one gate; nothing closes it for the others."
status: "pending"
```

## Summary

The record contract's defect was not found by a test. It was found by a stalled Run, because
every fixture repository the contract-gate checks build is born at the current freeze: no
archived record, no older goal revision, no retired Run, no landed ticket. A gate that is
unpassable on real history looks green forever on fixtures without one.

`ARF-003` (i-029) demands a fixture with a past for the record contract specifically. This
issue generalizes it: **every** gate that judges repository contents - the record contract,
the intake contract, the completion gate, the sensitivity-receipt gate, the edition audit's
callers, and any gate added later - must be exercised by at least one fixture carrying the
kind of history that gate walks. Otherwise the next unpassable gate is discovered the same
way this one was: by a Run that cannot move.

This is a rule about the checks, not about the gates: no gate's verdict changes. The
deliverable is fixtures and a working rule that keeps new gates from shipping without one.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
