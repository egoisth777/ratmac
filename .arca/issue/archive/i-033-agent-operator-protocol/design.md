# Issue design

## Proposed mechanics

Two carriers, one protocol, chosen by Billy on 2026-08-18 from four
investigated routes (MCP server 6/10, context files 7/10, skill 8/10,
self-describing CLI 8/10 — subagent reports in the session record):

1. **The engine teaches at runtime (AOP-001, AOP-002).** The status/step
   renderers in `src/cli.rs` and `src/state.rs` already print the state
   prompt verbatim and name repairs on some refusals; close the two gaps —
   guard expectations rendered from the parsed guard declaration, and a
   truthful `next:` line on every outcome. Precedents: git's `hint:` lines,
   cargo's `help:` suggestions, `gh auth status` naming the fixing command.
   Estimated small: render code plus golden-output tests; every future
   feature that forgets the render fails its own test.
2. **A thin skill for skill-aware harnesses (AOP-003, AOP-004).** A new
   subcommand (working name `rtm skill <path>`) writes the
   `ratmac-operator/` folder: `SKILL.md` with trigger-bearing description
   ("use when `.ratmac/` exists or rtm/runbook/step is mentioned"), plus
   `references/` for the refusal-code table and runbook-reading notes. The
   skill format is the cross-vendor Agent Skills shape read natively by
   Claude Code, Cursor 2.4+, Codex, and VS Code. It stays thin by design:
   invariant loop only, no flags, engine-identity stamp for staleness
   detection.

Deliberately out: an MCP server (adjunct at best — unreachable for plain
CLI agents, spec churn) and a scaffold-emitted AGENTS.md stub (open for a
future issue if skill auto-activation proves unreliable; the failure mode is
"protocol silently absent", which the self-describing CLI base already
softens — the first `rtm` command an agent runs starts teaching it).

This file is incoming evidence. Integrated mechanics remain authoritative
only in the accepted forward authority.
