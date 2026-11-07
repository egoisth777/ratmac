# Operable Run start with honest role evidence

```yaml
issue-id: "i-009-operable-run-start"
provenance: "Observed branch recovery evidence, 2026-07-24: the abandoned, uncommitted self-hosted Runbook experiment (formerly branch feat/ratmac-rombook-test at baseline e68bc51) and its independent advisor review. Observed there: a fresh session could not perform the documented first step because no project-local rtm bootstrap existed, requiring an ad-hoc ignored install directory and PATH change outside Run evidence; the human-approved Main-Agent start policy was accepted but only in uncommitted artifacts, while committed CLI help still prints the stale user-only rule; and role tests asserted wording in guidance files while their records claimed behavioral proof of invocation and non-invocation. The branch and its untracked artifacts are discarded, so the findings are restated here without links to them."
status: "integrated"
```

## Summary

The discarded run stalled at its very first documented step: the canonical path begins with bare `rtm status`, but the fresh session had only buildable sources, no deterministic way to locate or verify a Stable Engine binary, and no read-only diagnosis distinguishing the Runbook (`.arca/ratmac.toml`) from runtime state (`.arca/state.toml`). The caller policy the team retains — a human may invoke argument-free `rtm start`; the Main-Agent may invoke it only after explicit human Run-start sign-off for the target project; a Subagent never invokes `rtm` — exists on the fresh baseline only as discarded evidence, while committed help text still says start is user-only and agents must never start. Finally, the discarded role tests proved wording, not behavior, yet were recorded as behavioral proof. This issue re-establishes the policy across all active surfaces, adds a deterministic project-local bootstrap and read-only doctor, and requires behavioral claims to rest on behavioral evidence or be honestly recorded as guidance consistency. Dispositions in the specification record the author's proposed decision; P1 confirms or revises them at integration.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Integration

Folded into the goal on 2026-07-24 (P1). Every requirement disposition in [spec.md](spec.md) was confirmed `accepted`.

| Goal artifact | What it now carries |
| :--- | :--- |
| [Goal specification](../../../current/spec.md) | Requirement records `ORS-001`–`ORS-003` |
| [Goal design](../../../current/design.md) | The accepted mechanics for this issue |
| [Goal test list](../../../current/test-list.md) | Checks `ORSV-002`–`ORSV-009` |
| [Goal ubiquitous language](../../../current/ubi-lang.md) | This issue's terms |
| [Goal index](../../../current/index.md) | Reverse link to this issue |
