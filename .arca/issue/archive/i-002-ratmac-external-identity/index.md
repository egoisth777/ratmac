# Rebrand the external repository identity to ratmac

```yaml
issue-id: "i-002-ratmac-external-identity"
provenance: "User request: create a narrowly scoped full external-identity rebrand issue for the GitHub repository, origin, checkout directory, and active repository references."
status: "integrated"
```

## Summary

The internal product and command rebrand is already integrated by [i-001-ratmac-rebrand](../i-001-ratmac-rebrand/index.md), but the external repository identity still presents `arca-scheduler`. This issue requests the complete, narrowly bounded cutover of the GitHub repository slug to `egoisth777/ratmac`, the canonical origin to `git@github.com:egoisth777/ratmac.git`, and the local checkout directory from `E:/repos/projs/skill-dev/arca-scheduler` to `E:/repos/projs/skill-dev/ratmac`. Active links, badges, and repository metadata must follow the new slug. The `.git` metadata and checkout basename are part of the acceptance surface, not optional cleanup.

The work must begin with collision, authentication, access, and working-tree preflight checks; use a safe ordered cutover with a documented rollback; and finish with GitHub API/`gh` verification, Git remote/path checks, a clean Git state, and all existing project gates. Historical `.arca/log.md` and archived issue/ticket records remain unchanged. This artifact only records the incoming request: it performs no repository rename, filesystem move, source edit, push, deploy, or implementation.

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

Reverse requirement traces are carried in the goal specification and verification table as `EXT-001` through `EXT-006`. Residuals `res-039` through `res-044` and the single approved vertical ticket [t-038](../../../ticket/t-038.md) carry the P2/P3 execution mapping.
