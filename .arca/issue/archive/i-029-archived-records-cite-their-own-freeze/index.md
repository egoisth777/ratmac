# The record contract cannot pass on a repository with history

```yaml
issue-id: "i-029-archived-records-cite-their-own-freeze"
provenance: "Found by the first real Run of the shop's own cycle on 2026-08-10: stepping out of the ticket-cutting stage refused with 127 defects, every one an archived gap record citing the goal revision that was frozen when it was judged, and no live record at fault."
status: "integrated"
```

## Summary

The record contract counts gap records over the active folder and the archive as one
namespace - correctly, so a requirement cannot look unproven merely because its record
was archived. But the same rule then demands that **every** record it counted cite the
revision frozen for the Run doing the counting.

An archived record cannot satisfy that and must not be edited to: it cites the revision
that was frozen when it was judged, and the archive rule preserves its bytes. So the gate
refuses forever on any repository whose goal has ever changed - including this one, where
all 127 archived records fail and no live one does.

The engine is faithful to its requirement. The requirement is what cannot hold.

This was invisible until the engine ran its own cycle: every existing check builds a
fixture repository with no history, where the only records are the ones the fixture just
wrote at the current freeze.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
