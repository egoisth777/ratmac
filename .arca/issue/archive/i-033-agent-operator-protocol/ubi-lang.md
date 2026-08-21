# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Agent operator protocol (`AOP`) | The generic, project-independent instructions an agent needs to be driven by any runbook: orient, read the state prompt, work, place artifacts, step, branch on refusal codes, never write run state. This issue's stable requirement-ID prefix. |
| Self-describing CLI | The design where `rtm`'s own rendered output carries the operator protocol — prompts, guard expectations, and a truthful next-command hint — so the instructions are compiled into the running engine and can never drift from it. |
| Operator skill | A thin static skill folder (`SKILL.md` plus references) teaching the operating loop to skill-aware harnesses; it teaches invariant behavior, never enumerates flags, so it rarely goes stale. |
