# Rebrand arca-scheduler to ratmac and schd to rtm

```yaml
issue-id: "i-001-ratmac-rebrand"
provenance: "User request: rebrand arca-scheduler as ratmac and rename the CLI command from schd to rtm comprehensively across the knowledge SSOT and project."
status: "integrated"
```

## Summary

Replace the active product identity `arca-scheduler` with `ratmac` and the user-facing CLI command `schd` with `rtm` across the knowledge SSOT and the Rust project. The change covers names that compile into packages, crates, binaries, diagnostics, documentation, fixtures, tests, and generated package metadata; it must not alter scheduler behavior or Machine Class semantics. Existing historical records and append-only logs are preserved under the working rules, with their treatment explicitly recorded. This issue creates only the scoped change request; it does not implement the rebrand.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements and decisions | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Integration

The accepted requirements are folded into the frozen goal bundle:

- [Goal front door](../../../current/index.md)
- [Goal language](../../../current/ubi-lang.md)
- [Goal specification](../../../current/spec.md)
- [Goal design](../../../current/design.md)
- [Goal verification](../../../current/test-list.md)

Reverse requirement traces are carried in the goal specification and verification table as `RAT-001` through `RAT-008`.
