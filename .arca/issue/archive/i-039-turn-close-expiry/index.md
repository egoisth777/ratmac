# Turn-close verification respects explicit expiry

```yaml
issue-id: "i-039-turn-close-expiry"
provenance: "Billy, 2026-09-18: go, fix first - promoted the wishlist entry about ticket closure ignoring explicit lane expiry after the recovery turns"
status: "integrated"
```

## Summary

Ticket closure currently reruns raw tests and stops at a deliberately expired
crate. The sweep already distinguishes passing, expired, and failing crates.
Use that existing policy for the final close confirmation; current failures
must still refuse, and expiry markers must remain explicit, unchanged records.

This advances the Self-hosted and Every boundary machine-checked Ideal-shape
properties. No Engine behavior, runbook guard kind, or new expiry policy is asked for.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## History

- 2026-09-18: filed from Billy's promoted expiry-close wish; the five-file shape check passed before the new Run started.
- 2026-09-18: planning accepted TCE-001 into the goal; the declared root-once verifier reuses sweep policy and preserves the existing per-lane default. The matching working procedure changes before implementation.
