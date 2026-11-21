# Coding-Agent Process Tools

**Date:** 2026-11-21
**Producing agent:** `astra-coding-agent-process-tools`
**Method:** web research only, nothing installed or run
**Known defects:** These reports compared ratmac using an inaccurate summary: it said ratmac checks run nothing and a runbook cannot change during a Run. Both are wrong; see 07-ratmac-baseline.md. Every statement here about ratmac position or uniqueness is therefore unreliable. Also wrong in 04: "every model needs an explicit loop limit"; a state machine may run without end by design, and a bound is a choice per job.

---

## Comparison

Triples mean **who advances / who works / who accepts completion**.

**Pack** means instructions followed by an outside coding agent: user/model advances; agent works and judges completion. Reviews, repair loops and approvals are procedural, not an independent transition engine. Resume rereads artifacts; retries, timeouts and model access depend on the host.

**F** = editable local files; retention unspecified. **?** = not verified. **Local** = core works offline without a server. Stars measure attention, not users. Small-project activity shows **stars; last default-branch commit; latest GitHub release**, with dates in 2026.

| Product; maker; licence; maturity | Definition; advance / work / completion | Branches and loops | State, carried data, history | Approval, recovery, deployment |
|---|---|---|---|---|---|
| **Claude Code**; Anthropic; proprietary; shipping [1] | Markdown skills/subagents/plan, JSON hooks, JavaScript workflows; model/script / runtime / model-or-return plus hooks | Model or script; optional turn cap | Local/cloud chats, tasks, saved results; local transcripts default 30-day cleanup | Permissions, pause/replay; Claude online. Teams, `/loop`, scheduled tasks also exist |
| **Codex CLI**; OpenAI; Apache-2.0; 136k★ [2] | AGENTS.md, skills, hooks, plan; model / runtime / model plus hooks | Model; general loop cap ? | F transcripts plus SQLite runtime state; history configurable | Permissions/resume; optional offline local-model server |
| **Cursor**; Anysphere; proprietary; shipping [3] | Rules, hooks, automations; model / runtime / model plus hooks | Model/events; hook-loop limits | Local checkpoints/context; cloud conversations and artifacts; retention policies | Approval/resume; online, multiple models. Background Agents now Cloud Agents |
| **Gemini CLI**; Google; Apache-2.0; 109k★ [4] | Markdown, TOML commands, JSON hooks, plan; model / runtime / model plus hooks | Model; `maxSessionTurns` | Local chats/plans, shadow-Git checkpoints; configurable 30-day session retention | Permissions/restore/resume; Google online |
| **Kiro**; AWS; proprietary; IDE 1.x/CLI V3 [5] | Markdown specs, JSON hooks; model / runtime / human+model | Standard/Quick Spec; loop cap ? | F requirements/design/tasks; local chat database or cloud sessions; retention ? | Standard approval gates; Quick Spec skips them; resume; Kiro online |
| **Spec Kit**; GitHub; MIT; 151k★ [6] | **Now includes YAML execution engine**; runtime / agent+shell / return-or-gate | Expressions; bounded `while`; fan-out/in | F JSON inputs/results, frozen definition, append-only JSONL log | Approval, timeout, failed/paused resume; local engine, multiple hosts |
| **OpenSpec**; Fission AI; MIT; 75k★ [7] | YAML artifact schemas + skills; caller / agent / file existence | Artifact DAG; unrestricted revisions | F proposals/specs/tasks/archive | Review/validation/re-read; local, multiple hosts. OPSX replaced legacy workflow |
| **BMAD Method**; BMad Code; MIT; v6.12 stable, v7 alpha [8] | Markdown/YAML; Pack | Scope-selected paths; procedural loops | F briefs, architecture, stories/status | Pack; multiple hosts |
| **Taskmaster**; Hamster; MIT+Commons Clause; 28.5k★ [9] | JSON tasks; caller or loop / agent / status or completion marker | Dependencies/priorities; loop cap ? | F tasks and loop progress; optional team cloud | Manual status; blocked-loop stop; local-model option; execution loop uses Claude |
| **Superpowers**; Jesse Vincent/Prime Radiant; MIT; 323k★ [10] | Markdown skills/hooks; Pack | Design→plan→TDD→review; procedural loops | F designs/plans plus host transcripts | Pack; many hosts |
| **Beads**; Yegge/Gastownhall; MIT; 28.9k★ [11] | TOML/JSON formulas; caller / external agent / close+gate checks | Dependencies, reopen, workflow recipes; cap ? | Dolt issues/history; compaction and purge | Human/timer/CI gates; embedded offline, optional server |
| **beads_rust**; Jeffrey Emanuel; MIT+restrictive rider; 1,161★; 11-17; v0.7.6, 10-15 [12] | YAML status policy; caller / external / reported gate+policy | Status edges/reopen; cap ? | SQLite, JSONL exports, audit records; pruning | Gate reports, transactional rejection; local, agent-neutral |
| **Agent OS**; Builder Methods; MIT; v3, 5.6k★ [13] | Markdown standards/spec shaping; Pack | Host-controlled | F standards/specs | V3 removed installed subagents and delegates planning to host |
| **Conductor plugin**; gemini-cli-extensions; Apache-2.0; 3.8k★ [14] | Markdown skills; Pack | Tracks and correction loops | F `conductor/` specs/plans/status; Git | Approval/revert; now documents Claude and Antigravity support |
| **Conductor app**; Melty→Meta; proprietary [15] | GUI/workspaces/scripts; user+model / hosted agent / model | Independent workspaces; host loops | Local/cloud chats and worktrees; retention ? | Review/restart; online, multiple agents. Announced shutdown **reversed** |
| **StateM**; Ziheng Qin; Apache-2.0; 1,605★; 11-10; artifact release 08-18 [16] | YAML; caller / external / executable checks | Caller target; cycles; per-entry attempt ceilings | F state, evidence, history, spec hashes | Manual approval, blocked transitions, timeouts/resume; local core |
| **GateReeve**; Trent Brown; MIT; 0★; 11-21; v0.1.0-rc.31, 11-21 [17] | Pinned development protocol; caller / external / validators+reviews | Named passages; repair loops, cap ? | F artifacts and append-only journal; evidence fingerprints | Human authority, suspension/stale-proof rejection; local CLI, online full workflow |
| **agent-workflow / Shipwright**; Pulse; MIT; 0★; 10-09; v0.4.0, 10-09 [18] | JSON configuration+skills; Pack with reporting checks | Config/model; procedural loops | F `docs/features/` artifacts | Intake/plan/test decisions; Claude plugin; **explicitly non-blocking** |
| **IX Flow**; Agent IX; MIT; 1★; 11-18; v0.3.3, 10-24 [19] | YAML+skills; caller / external / invariants | Declared target; cycle bounds ? | F per-run JSON, typed items, event history/integrity check | Approval token, resume, invariant rejection; local, multiple hosts |
| **FlowForge**; kagura-chen; MIT; 8★; 08-27; no GitHub releases [20] | YAML; caller / external / caller assertion | Branch number; repair cycles; cap ? | SQLite tasks/history; resettable | Resume/reset; no documented evidence gate; local, agent-neutral |
| **agency-runner**; mahdyaralipor; MIT; 0★; 11-21; v0.2.0, 11-21 [21] | Fixed seven-phase NEXUS process; caller / external / evidence+decision | Pass/fail; three-failure escalation | F `.agency/state.json`: criteria, hashes, decisions | Gate approval, recorded force override, resume; local, agent-neutral |

## A. Three common choices — synthesis

- Put instructions around an existing agent instead of building a separate transition engine.[1–5,8,10,13–14,18]
- Preserve context in mutable artifacts, checklists or task records.[6–14,16–21]
- Combine model judgment with human approval or configurable checks; “done” rarely means independently verified correctness.[1–21]

## B. Publicly reported problems

Reports are evidence of failure modes, not independent reproductions:

- **Rules ignored:** Cursor users report skipped planning approvals despite workspace instructions.[22]
- **Runaway enforcement:** Claude users report Stop-hook/PreToolUse conflicts causing repeated blocked attempts.[23]
- **Checks silently passing:** StateM predicate/schema problems were addressed by merged PR #6.[24]
- **Evidence becomes expensive:** GateReeve PR #99 reports reviewer-history growth exceeding its prompt limit; it introduced bounded history.[25]

## C. Closest to ratmac

- **StateM:** substantially overlaps already. Differences: Python/YAML, shell and LLM checks, dynamic checks, and explicit spec rebinding rather than an unchangeable runbook.[16]
- **IX Flow:** generic caller-driven transitions, but typed records and executable invariants replace ratmac’s narrow file-check contract.[19]
- **GateReeve/agency-runner:** evidence-gated external work, but prescribed development processes rather than generic graphs.[17,21]
- **Spec Kit:** durable file-based workflows, but actively launches agents and commands.[6]

## D. Potential differentiation — inference

No reviewed documentation establishes ratmac’s **exact combination**: passive file checks, closed-word-plus-reason branching, immutable per-run runbook, and generic child-run joins. That is a positioning hypothesis, not proof of novelty. “State as files” alone is already occupied.[6,16–21; ratmac description supplied]

## E. Common capabilities beyond ratmac’s stated scope

Ready-made planning/TDD/review procedures, agent integration, and interactive approval are common. Execution-oriented products additionally supply model/tool invocation, timeouts, scheduling, recovery and interfaces.[1–14,17–19]

**Scope:** 21 concrete products. Added three close CLI competitors and beads_rust. Skipped exhaustive forks, unrelated FlowForge projects, Netflix/Orkes Conductor, and unverified benchmark/productivity claims. No local repository was accessed.

## Sources

All are primary sources; issue/forum entries are attributed user reports.

1. **Claude Code:** [features](https://code.claude.com/docs/en/features-overview), [hooks](https://code.claude.com/docs/en/hooks), [subagents](https://code.claude.com/docs/en/sub-agents), [planning/workflows](https://code.claude.com/docs/en/common-workflows), [dynamic workflows](https://code.claude.com/docs/en/workflows), [teams](https://code.claude.com/docs/en/agent-teams), [scheduled tasks](https://code.claude.com/docs/en/routines), [sessions](https://code.claude.com/docs/en/sessions), [settings](https://code.claude.com/docs/en/settings-reference), [CLI limits](https://code.claude.com/docs/en/cli-reference), [terms](https://code.claude.com/docs/en/legal-and-compliance).
2. **Codex:** [repository metadata/licence](https://api.github.com/repos/openai/codex), [customization](https://developers.openai.com/codex/concepts/customization), [hooks](https://developers.openai.com/codex/hooks), [commands](https://developers.openai.com/codex/cli/slash-commands), [configuration](https://developers.openai.com/codex/config-reference), [history](https://developers.openai.com/codex/config-advanced), [local providers](https://developers.openai.com/codex/cli/reference).
3. **Cursor:** [rules](https://cursor.com/docs/rules), [hooks](https://cursor.com/docs/hooks), [agent/checkpoints](https://cursor.com/docs/agent/overview), [cloud automations](https://cursor.com/docs/cloud-agent/automations), [cloud storage/security](https://prod.cursor.com/docs/cloud-agent/security), [retention](https://cursor.com/docs/enterprise/security-hardening), [terms](https://cursor.com/terms-of-service).
4. **Gemini CLI:** [repository metadata/licence](https://api.github.com/repos/google-gemini/gemini-cli), [documentation](https://geminicli.com/docs/), [configuration](https://geminicli.com/docs/reference/configuration/), [sessions and limits](https://geminicli.com/docs/cli/session-management/), [checkpointing](https://google-gemini.github.io/gemini-cli/docs/cli/checkpointing.html), [plan mode](https://geminicli.com/docs/cli/plan-mode/).
5. **Kiro:** [specs](https://kiro.dev/docs/specs/), [built-in workflows](https://kiro.dev/docs/custom-agents/built-in/), [harness/permissions](https://kiro.dev/docs/how-kiro-works/), [CLI V3](https://kiro.dev/docs/cli/v3/), [IDE migration](https://kiro.dev/docs/ide/0x-reference/), [sessions](https://kiro.dev/docs/cli/chat/session-management/), [licence](https://kiro.dev/license/).
6. **Spec Kit:** [README](https://github.com/github/spec-kit/blob/main/README.md), [metadata](https://api.github.com/repos/github/spec-kit), [workflow execution/state contract](https://github.com/github/spec-kit/blob/main/docs/workflows/api.md), [step types, loops, gates and failures](https://github.com/github/spec-kit/blob/main/docs/workflow-steps/types.md), [workflow overview](https://github.com/github/spec-kit/blob/main/docs/workflows/overview.md).
7. **OpenSpec:** [metadata](https://api.github.com/repos/Fission-AI/OpenSpec), [OPSX architecture and filesystem state](https://github.com/Fission-AI/OpenSpec/blob/main/docs/opsx.md), [licence](https://openspec.dev/legal/terms/).
8. **BMAD:** [README and release-line status](https://github.com/bmad-code-org/BMAD-METHOD/blob/main/README.md), [licence](https://github.com/bmad-code-org/BMAD-METHOD/blob/main/LICENSE).
9. **Taskmaster:** [README/licence](https://github.com/eyaltoledano/claude-task-master/blob/main/README.md), [metadata](https://api.github.com/repos/eyaltoledano/claude-task-master), [task structure](https://tryhamster.com/docs/taskmaster/capabilities/task-structure), [CLI/status/dependencies](https://tryhamster.com/docs/taskmaster/capabilities/cli-root-commands), [execution loop](https://tryhamster.com/docs/taskmaster/automation/loop), [offline providers](https://tryhamster.com/docs/taskmaster/getting-started/faq).
10. **Superpowers:** [README](https://github.com/obra/superpowers/blob/main/README.md), [metadata/licence](https://api.github.com/repos/obra/superpowers).
11. **Beads:** [README and storage migration](https://github.com/gastownhall/beads/blob/main/README.md), [metadata/licence](https://api.github.com/repos/gastownhall/beads), [formulas](https://beads.gascity.com/workflows/formulas), [gates](https://beads.gascity.com/workflows/gates).
12. **beads_rust:** [README: storage, workflow policy and pruning](https://github.com/Dicklesworthstone/beads_rust/blob/main/README.md), [restrictive licence rider](https://github.com/Dicklesworthstone/beads_rust/blob/main/LICENSE), [metadata](https://api.github.com/repos/Dicklesworthstone/beads_rust), [latest commit](https://api.github.com/repos/Dicklesworthstone/beads_rust/commits?per_page=1), [releases](https://api.github.com/repos/Dicklesworthstone/beads_rust/releases?per_page=1).
13. **Agent OS:** [README](https://github.com/buildermethods/agent-os/blob/main/README.md), [v3 migration](https://buildermethods.com/agent-os/migration), [metadata/licence](https://api.github.com/repos/buildermethods/agent-os).
14. **Conductor plugin:** [current README](https://github.com/gemini-cli-extensions/conductor/blob/main/README.md), [metadata/licence](https://api.github.com/repos/gemini-cli-extensions/conductor).
15. **Conductor app:** [product/Meta announcement](https://www.conductor.build/), [shutdown reversal](https://www.conductor.build/changelog/0.94.2-conductor-is-back), [privacy/storage](https://www.conductor.build/docs/reference/privacy), [cloud execution](https://www.conductor.build/docs/cloud/faq), [terms](https://www.conductor.build/terms).
16. **StateM:** [README, closely read](https://github.com/henryqin1997/statem/blob/main/README.md), [metadata/licence](https://api.github.com/repos/henryqin1997/statem), [latest commit](https://api.github.com/repos/henryqin1997/statem/commits?per_page=1), [artifact release](https://github.com/henryqin1997/statem/releases/tag/deepseek-policy9-tb21-artifacts-20260818).
17. **GateReeve:** [README, closely read](https://github.com/TrentBrown/gatereeve/blob/main/README.md), [user guide/protocol](https://github.com/TrentBrown/gatereeve/blob/main/USER-GUIDE.md), [metadata](https://api.github.com/repos/TrentBrown/gatereeve), [latest commit](https://api.github.com/repos/TrentBrown/gatereeve/commits?per_page=1), [release](https://github.com/TrentBrown/gatereeve/releases/tag/v0.1.0-rc.31).
18. **agent-workflow:** [README, closely read; explicit non-enforcement](https://github.com/pulse-amazing-team/agent-workflow/blob/main/README.md), [metadata](https://api.github.com/repos/pulse-amazing-team/agent-workflow), [latest commit](https://api.github.com/repos/pulse-amazing-team/agent-workflow/commits?per_page=1), [release](https://github.com/pulse-amazing-team/agent-workflow/releases/tag/v0.4.0).
19. **IX Flow:** [README](https://github.com/agent-ix/ix-flow/blob/main/README.md), [guide: invariants, approval, JSON persistence](https://github.com/agent-ix/ix-flow/blob/main/docs/guide.md), [metadata](https://api.github.com/repos/agent-ix/ix-flow), [latest commit](https://api.github.com/repos/agent-ix/ix-flow/commits?per_page=1), [releases](https://api.github.com/repos/agent-ix/ix-flow/releases?per_page=1).
20. **FlowForge:** [README](https://github.com/kagura-agent/flowforge), [current repository metadata](https://api.github.com/repos/kagura-chen/flowforge), [latest commit](https://api.github.com/repos/kagura-chen/flowforge/commits?per_page=1), [empty GitHub release listing](https://api.github.com/repos/kagura-chen/flowforge/releases?per_page=1).
21. **agency-runner:** [README](https://github.com/mahdyaralipor/agency-runner/blob/main/README.md), [metadata](https://api.github.com/repos/mahdyaralipor/agency-runner), [latest commit](https://api.github.com/repos/mahdyaralipor/agency-runner/commits?per_page=1), [releases](https://api.github.com/repos/mahdyaralipor/agency-runner/releases?per_page=1). README still labels itself 0.1.0; release metadata reports v0.2.0.
22. **Cursor report:** [planning workflow and workspace rules ignored](https://forum.cursor.com/t/agent-ignores-my-custom-planning-workflow-and-workspace-rules/170274).
23. **Claude report:** [Stop-hook blocking loop](https://github.com/anthropics/claude-code/issues/107256).
24. **StateM report/fix:** [predicate silently passes](https://github.com/henryqin1997/statem/issues/4), [merged fail-closed fix](https://github.com/henryqin1997/statem/pull/6).
25. **GateReeve maintainer report/fix:** [bounded reviewer history and fingerprint caching](https://github.com/TrentBrown/gatereeve/pull/99).
