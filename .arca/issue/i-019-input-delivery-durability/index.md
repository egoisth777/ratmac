# Input delivery and durability

```yaml
issue-id: "i-019-input-delivery-durability"
provenance: "Billy, 2026-07-30 - authorized the atomic cut of i-016-fsm-doctrine-convergence so input delivery and durability stand apart from transition selection and Run completion"
status: "pending"
```

## Summary

This issue owns how one externally authored judgment reaches one addressed Run and survives consumption without replay. The judgment record carries one transition input value; the Engine validates and consumes that value but never makes the judgment. Consumption is one atomic move from the live verdict slot into immutable Run evidence before the successor State File is written.

It retains requirement `FDC-003` under its existing identifier. It depends on the input-routed-transition issue ([i-016-fsm-doctrine-convergence](../i-016-fsm-doctrine-convergence/index.md)), which defines the legal values and their edge mapping. It does not define witnessed or human-signed judgment: that mechanism remains deferred because the Engine carries no signer identity. Judge independence remains in the machine-composition issue ([i-018-machine-composition](../i-018-machine-composition/index.md)).

The doctrine-convergence issue remains the evidence seed: its adversarial-review ledger and decision records are cited here, never copied.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
