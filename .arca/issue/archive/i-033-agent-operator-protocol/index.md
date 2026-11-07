# The agent operator protocol travels with the engine

```yaml
issue-id: "i-033-agent-operator-protocol"
provenance: "Billy's 2026-08-18 alignment session: the executor and the runbook exist, but nothing generic teaches an agent how to operate them; four delivery routes were investigated by subagents (MCP, skills, context files, self-describing CLI) and Billy chose the CLI-plus-skill combination"
status: "integrated"
```

## Summary

A second project adopts ratmac by writing a runbook — but the agent on that
project has nothing telling it how to be driven by one: orient with
`rtm status`, read its state's prompt, do the work, place artifacts,
`rtm step`, branch on refusal codes, never write run state. Today that
knowledge lives only in this repository's own convention files and does not
travel. This issue makes the protocol travel two ways, per Billy's ruling:
the engine's own output teaches the loop completely (self-describing CLI),
and a thin `ratmac-operator` skill folder gives skill-aware harnesses the
same loop up front.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
