# Full `rtm doctor` executable fingerprint

```yaml
issue-id: "i-023-doctor-full-fingerprint"
provenance: "Human promotion, 2026-08-04, from archive tag trial-archive/trial-002-doctor-full-fingerprint and durable log on exp/ratmac-deterministic."
status: "pending"
```

## Summary

This pending, single-ask issue requires argument-free `rtm doctor` to report the complete SHA-256
fingerprint of the exact executable it is running. The report must contain all 64 lowercase hexadecimal
characters and remain write-free. Executable selection, pin/trust behavior, state reporting, Runbook
findings, and `--json` behavior remain unchanged.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
