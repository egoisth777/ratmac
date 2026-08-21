# The edition ledger records, never predicts

```yaml
issue-id: "i-034-edition-ledger-recording-order"
provenance: "Found by the 2026-08-21 sprint setup: building the stable engine at edition-003's own commit refused with a ledger/tag disagreement, because that commit's ledger row cites the pre-correction hash f12e8de... while the tag points at 929c583... - the row can never cite the landing it is part of"
status: "integrated"
```

## Summary

The edition rule (`EDN-003`, schema.md Editions) said the ledger row "is
written in the landing that cuts the edition" while also requiring the row to
record "the commit each edition was cut at" - a self-reference no landing can
satisfy: a commit cannot contain its own hash. History already shows the
symptom twice: the edition-002 row was "missed in the closing landing; added
now, one commit later", and the edition-003 row was first recorded wrong and
corrected one landing later. The consequence is concrete: the stable-channel
bootstrap, run at the tagged commit, reads that commit's own stale ledger and
refuses a healthy edition. This issue corrects the recording order in the
working rules and makes the bootstrap resolve stable from the invoking
project's current ledger while building the tagged commit in a clean
checkout.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
