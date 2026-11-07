# Issue specification

Dispositions were confirmed at the 2026-08-21 planning pass (P1), signed by
Billy's sprint authorization of the same day.

`AOP` is this issue's stable requirement-ID prefix — **Agent Operator
Protocol** — defined in [ubi-lang.md](ubi-lang.md).

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `AOP-001` | `rtm status` and `rtm step` render, for each pending guard, what the guard expects in plain words — the declared artifacts or check it will read — not only the guard's label. The rendering is derived from the parsed guard declaration, never hand-kept prose. | accepted | Accepted as filed: the guard declaration already holds the answer; rendering it removes the last guessing step in the operating loop. | [goal spec](../../../goal/spec.md#integrated-agent-operator-protocol-requirements) |
| `AOP-002` | Every `rtm status` and `rtm step` outcome — success or refusal — ends with one truthful `next:` line naming the single next legitimate command or act (e.g. `next: do the prompt's work, then rtm step --run <id>`). A refusal's `next:` names the repair for its stable code. A `next:` line the engine cannot stand behind is omitted, never guessed. | accepted | Accepted as filed: a truthful next line on every outcome makes the loop total; an unsupportable hint is omitted, never guessed. | [goal spec](../../../goal/spec.md#integrated-agent-operator-protocol-requirements) |
| `AOP-003` | `rtm` gains a subcommand that writes a thin `ratmac-operator` skill folder (`SKILL.md` plus reference files) at a caller-given path that does not exist yet — one folder, never overwrites, mirroring the scaffold's discipline. The skill teaches the operating loop and the never-touch rules, points at the CLI's own output for everything current, enumerates no flags, and is stamped with the writing engine's identity. | accepted | Accepted as filed: the engine writing its own operator skill keeps the manual paired with the binary, on the scaffold's never-overwrite discipline. | [goal spec](../../../goal/spec.md#integrated-agent-operator-protocol-requirements) |
| `AOP-004` | The skill teaches only invariant behavior: orient, read the prompt, place artifacts, step, branch on refusal codes, never write under the engine root. Anything version-shaped (verbs, flags, wording) is reached by telling the agent to run the command, never by quoting its output. | accepted | Accepted as filed: invariant behavior only; anything version-shaped is reached by running the command, so the skill cannot teach lies. | [goal spec](../../../goal/spec.md#integrated-agent-operator-protocol-requirements) |
## Acceptance criteria

- An agent given only a repository containing `.ratmac/ratmac.toml` and the
  operator skill can drive one Run from `rtm start` to a terminal state using
  nothing but the skill and the engine's own output — no `.arca/` convention
  file consulted.
- `rtm status` on a Run with a pending guard names what the guard will read,
  and the wording is generated from the guard's parsed declaration.
- Every status/step rendering path ends in a `next:` line or deliberately
  omits it; no rendering invents a command the engine would refuse.
- The skill-writing subcommand refuses an existing path, writes one folder,
  and the written `SKILL.md` carries the engine identity stamp and no flag
  enumeration.
- Investigated alternatives are on record: MCP was judged an adjunct (unreachable
  for plain agents, protocol churn), a scaffold-emitted AGENTS.md stub remains
  open for a later issue if skill activation proves unreliable in practice.
