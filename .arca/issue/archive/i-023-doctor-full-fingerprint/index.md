# Full `rtm doctor` executable fingerprint

```yaml
issue-id: "i-023-doctor-full-fingerprint"
provenance: "Human promotion, 2026-08-04, from archive tag trial-archive/trial-002-doctor-full-fingerprint and durable log on exp/ratmac-deterministic."
status: "integrated"
ideal-shape-property: "Every boundary machine-checked"
```

## Summary

This single-ask issue requires argument-free `rtm doctor` to report the complete SHA-256
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

## P1 disposition

On 2026-08-04, `DFP-001` was accepted into the goal's integrated full doctor executable fingerprint requirement, design, and verification sections. The complete bundle was archived. The full machine-derived executable identity advances **Every boundary machine-checked**.
