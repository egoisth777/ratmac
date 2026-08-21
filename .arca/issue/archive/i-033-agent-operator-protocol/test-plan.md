# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| AOPV-001 | AOP-001 | On a Run whose state carries each declared guard kind, `rtm status` output names what that guard reads; a golden test pins the rendering, and the wording is asserted to come from the parsed declaration (renaming the guard's declared artifact changes the rendering with no render-code edit). |
| AOPV-002 | AOP-002 | Golden tests over every status/step rendering path: each ends in exactly one `next:` line naming a command the engine accepts in that state, or omits the line; a refusal's `next:` matches its stable code's repair. |
| AOPV-003 | AOP-003 | The skill subcommand at a fresh path writes one folder containing `SKILL.md` plus references; at an existing path it refuses and writes nothing; `SKILL.md` carries the engine identity stamp. |
| AOPV-004 | AOP-004 | A scan of the written skill finds no CLI flag tokens and no quoted command output; the loop steps and never-touch rules are present; driving a scaffolded runbook to terminal using only the skill and engine output succeeds. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | AOP-001..AOP-004 enter the goal at P1 |
| `.arca/goal/ubi-lang.md` | updated | Agent operator protocol, operator skill |
| `.arca/goal/spec.md` | updated | AOP-001..AOP-004 |
| `.arca/goal/design.md` | updated | renderer mechanics, skill subcommand |
| `.arca/goal/test-list.md` | updated | AOPV-001..AOPV-004 |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | - |
| `.arca/steering.md` | updated | Horizon: this issue advances the Generic engine and Authored-not-imitated Ideal-shape properties; ordering set at the next P1 |
