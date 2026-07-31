# Orchestration Landscape & Niche Validation

**Date:** 2026-07-27
**Subject:** ratmac — a Rust CLI acting as a deterministic FSM runner that an LLM coding agent calls as a tool
**Scope:** Competitive landscape of agent workflow/orchestration systems (2025–2026); identification of systems sharing ratmac's inverted-control model; assessment of niche occupancy.

---

## 1. Verdict

> **Partially occupied — and the strongest occupant landed three months ago with a patent.**

Ratmac's inverted-control model (deterministic state controller *outside* the agent, queried and requested-against by the agent) is a real, recognized, and independently-reinvented pattern. It is **not** empty space, and **not** crowded either: roughly three to four serious implementations exist, one of which has meaningful traction and a defensive patent position.

The pattern is also **not an anti-pattern** in the 2026 community. If anything, the discourse has flipped: the *prompt* is now widely treated as the anti-pattern for anything that must be enforced. But the community's actual center of gravity for enforcement is **hooks**, not a queryable FSM — which leaves a narrow but real seam.

### 1.1 Three strongest pieces of evidence

**1. Statewright is ratmac's model almost line-for-line, and it has traction.**
[github.com/statewright/statewright](https://github.com/statewright/statewright) — Rust deterministic engine, **418 stars**, created 2026-05-03, last pushed 2026-07-23 (actively developed). It exposes exactly the query/request tool pair ratmac is built around:

- `statewright_get_state` — returns "Current state, allowed tools, transitions, iteration count, model, thinking level"
- `statewright_transition` — "Emit an event to advance the state machine"

Workflows are defined as JSON data (states, transitions, guards, per-state tool allowlists, per-state model routing), against a published schema at `statewright.ai/workflow-schema.json`. It integrates with Claude Code, Codex, Cursor, opencode, and Pi. It also holds **a patent covering "the method of using state machines to constrain LLM agent tool access at the protocol layer."** The Hacker News reception was broadly positive *on the architecture*; the loudest criticism was directed at the patent and the FSL license, not the design.

**2. Theodosia implements the inversion explicitly, and its author is already converging on ratmac's exact thesis.**
[github.com/msradam/theodosia](https://github.com/msradam/theodosia) — Python, 16 stars, created 2026-05-19, pushed **2026-07-27 (today)**. It mounts an Apache Burr state machine as an MCP server. The agent reads `theodosia://state`, `theodosia://next`, and `theodosia://graph` resources to answer "where am I / what is legal," then calls a single gated `step(action, inputs)` tool to request a transition. Refusals return `valid_next_actions` so the agent self-corrects. The author has publicly stated they are building **skills→state-machine conversion, "since many popular agent skills are already written as phases for an AI model to follow"** — which is ratmac's founding thesis stated verbatim by someone else.

**3. The pattern is repeatedly reinvented from scratch, which means it is obvious-once-stated rather than novel.**
Independent arrivals at "the server holds the state, rejects out-of-order calls, and tells the agent what comes next" include: a Shopee engineer's `get_step(n)` gated MCP tool; Stratum's typed-YAML postcondition dispatch server; CodeLeash's TDD state machine over Claude Code hooks; `reactive-fsm` (TypeScript); MooreLLM (2024); and the Edit Approval State Machine (EASM). None of these cite each other. That convergence is the clearest signal that the *idea* carries no moat — only the execution does.

---

## 2. Systems table

Closeness score is to **ratmac's specific model**: a deterministic controller living outside the agent, holding a user-authored runbook as data, answering "what state am I in / what should I do next," and validating agent-requested transitions against guards.

| System | Loop controller | Runbook-as-data? | Closeness (1–5) |
|---|---|---|---|
| **Statewright** (Rust, MCP + hooks, 418★) | **Agent** — engine gates and validates; no LLM in the engine | **Yes** — JSON: states, transitions, guards, tool allowlists, per-state model/thinking level | **5** |
| **Theodosia** (Python, Burr→MCP, 16★) | **Agent** — calls `step`, reads `next`/`state` resources | **No** — Burr FSM authored in Python; companion project "Philip" lifts YAML/Mermaid/Excalidraw into Burr apps | **4.5** |
| **Stratum / stratum-mcp** (regression-io, ~3★, Feb 2026) | **Agent** — dispatch server with postconditions and gates | **Yes** — typed YAML specs | **4.5** |
| Stateful `get_step` MCP pattern (Medium write-up, no OSS release) | **Agent** — server rejects out-of-order calls | Partial — step list held as objects in server code | 3.5 |
| **TDD Guard** (2,283★, hooks) | **Agent** — PreToolUse hooks block violations | **No** — state derived by scanning a TDD log file | 3 |
| CodeLeash TDD Guard / Superpowers plugin | **Agent** — hook-enforced RED→GREEN→REFACTOR | **No** — phases hardcoded | 3 |
| **spec-workflow-mcp** (Pimzino) / **specs-workflow-mcp** (kingkongshot) | **Agent** — queries phase; server tracks progress across sessions | **No** — fixed Requirements→Design→Tasks pipeline | 2.5 |
| **GitHub Spec Kit / AWS Kiro / Tessl** | **Agent** (slash-command driven), with human approval gates between phases | Partial — spec artifacts as data, but phase pipeline is fixed | 2.5 |
| **Beads** (`bd` CLI, 25.7k★) | **Agent** — queries an external CLI-backed store | **No FSM**, no transition validation — it is a dependency-graph issue tracker | 2.5 |
| **workflows-mcp-server** (cyanheads) | **Agent** — fetches a YAML playbook; no state tracking | **Yes** (YAML playbooks) but no guards, no current-state concept | 2 |
| **statelyai/agent** (XState), **reactive-fsm**, **MooreLLM** | **Engine** — wraps and drives the LLM | Yes | 2 |
| **LangGraph** | **Engine** — the graph runs the model | Code-as-graph | 1.5 |
| **Burr** (Apache) | **Engine** | Code | 1.5 |
| **Pydantic AI graph** (`pydantic-graph`) | **Engine** by default; `Agent.iter` hands you manual node-by-node control | Code | 1.5 |
| **Mastra** | **Engine** in workflow mode, **model** in agent mode | Typed TypeScript | 1.5 |
| **CrewAI / AutoGen** | **Engine + model** jointly | Partial YAML | 1 |
| **OpenAI Agents SDK** | **SDK owns the while-loop**; the model steers | No | 1 |
| **Temporal / Microsoft Conductor / Inngest / Restate** | **Your workflow code** owns the loop; the platform makes it durable | Conductor: YAML | 1 |
| **Claude Code dynamic workflows** | **Script is the controller, agents are workers** — the exact inverse of ratmac | No — Claude *writes* the orchestration script at runtime | 1 |

### 2.1 Reading the table

There is a clean split at roughly score 2.5.

**Above the line (scores 2.5–5):** the agent drives, and an external deterministic component gates. Every system here is either MCP-based or hook-based. **None is a plain CLI.**

**Below the line (scores 1–2):** the engine drives and the LLM is a callee. This is the entire mainstream framework ecosystem — LangGraph, Burr, CrewAI, AutoGen, Mastra, Pydantic AI, OpenAI Agents SDK — plus the durable-execution layer (Temporal, Conductor). These are **not competitors to ratmac**; they solve a different problem for a different consumer (an application developer building an agent, not a coding agent operating in someone's repo).

Notably, **Claude Code's own dynamic workflows feature sits at the opposite pole**: Claude generates a JavaScript orchestration script that fans work out to subagents, and "the coordination happens outside the conversation." The plan is emergent and Claude-authored rather than a fixed deterministic graph. This is worth flagging because it is the feature most likely to be mistaken for overlap — it is not. It could in principle *compose* with an FSM (an FSM gating when a workflow runs), but Anthropic describes no such mechanism.

---

## 3. Direct-competitor profiles

### 3.1 Statewright — the one that matters

- **URLs:** [github.com/statewright/statewright](https://github.com/statewright/statewright) · [statewright.ai](https://statewright.ai/) · [docs.statewright.ai](https://docs.statewright.ai/) · Show HN: [news.ycombinator.com/item?id=48108778](https://news.ycombinator.com/item?id=48108778)
- **Language / metadata:** Rust (Cargo workspace: `crates/engine`, `crates/cli`, `crates/tui`, `crates/mcp-gateway`), plus TypeScript plugins for Pi and opencode. 418 stars, 15 forks, 227 commits. Created 2026-05-03; last pushed 2026-07-23.
- **License:** Apache 2.0 for the engine and agent crates. The MCP gateway is FSL-1.1-ALv2, converting to Apache 2.0 on 2029-05-03. A `PATENTS.md` pledge exists after community pushback.

**Architecture.** Three components:
1. **Engine** (`crates/engine`) — a pure Rust state machine evaluator handling states, transitions, guards, and tool restrictions. Deterministic, no LLM in the loop, no runtime dependencies. The author's own framing: *"JSON in => transition decisions out."*
2. **Agent binary** (`sw-agent`) — a direct-to-Ollama executor that loads a workflow, runs the LLM in a constrained loop, enforces tool access, and streams structured JSONL events. Also usable as a single-state executor via `--state` and `--config`. A `statewright` ratatui TUI wraps it.
3. **Plugin layer** — an MCP gateway that integrates with coding agents. On workflow activation, hooks enforce tool restrictions per state, so "the model sees 5 tools instead of 30."

**MCP tool surface (10 tools).** The status/transition pair is the core:
- `statewright_get_state` — current state, allowed tools, transitions, iteration count, model, thinking level
- `statewright_transition` — emit an event to advance the state machine
- Lifecycle: `statewright_load_workflow` (with resume), `statewright_pause`, `statewright_deactivate`, `statewright_get_status`
- Authoring/discovery: `statewright_create_workflow`, `statewright_list_workflows`
- Special: `statewright_run_agent` (spawns `sw-agent`), `statewright_force_state` (debug-only, gated behind `meta.debug`)
- Also exposes a slash command: `/statewright start bugfix`

**Workflow JSON shape.** Top level: `id`, `initial`, `meta` (`default_model`, `approval_mode`, `debug`), `states`, `guards`. Per state:
- `allowed_tools`, `allowed_commands` (prefix-matched, e.g. `pytest`)
- `max_edit_lines`, `max_files_per_state`, `max_iterations`
- `model` and `thinking_level` for per-state routing
- `on`: event → target transitions, optionally `{ "target": ..., "guard": ... }`
- `requires_approval` and `approval_message`
- `type: "final"` for terminal states

**Guards.** Comparisons on context-data fields only: `{ "field": "test_result", "op": "eq", "value": "pass" }`, `coverage gt 80`. The README describes these as *"programmatic guards on context data."* **Nothing indicates guards can inspect the filesystem or execute commands.** Related but separate mechanisms exist: interrupts (a file-glob edit trigger auto-transitions to a validation state), approval gates, Bash discernment (blocks `rm -rf`, write-via-redirect, `sed -i`, interpreters), env scoping (`blocked_env` / `env_overrides`), fork/join, and escalation detection.

**Claimed results.** On a 5-task SWE-bench subset, two local models (13.8 GB and 19.9 GB) improved from 2/10 to 10/10 with constraints enabled. **Caveat:** per the author's own HN comments, the experiment harness (task selection, patch scoring, control runs) has not been published. The engine, agent crate, and demo TUI are in the repo, and the simple-bugfix result is reproducible end-to-end with a 13B+ model on Ollama.

**Community reception (HN thread).** Broadly positive on the pattern:
- *tim-projects:* "I'm fully convinced that state machines are the key to getting low powered llm models to produce good quality code."
- *fizza_pizza* praised reducing the problem space rather than brute-forcing with bigger models.
- *addaon* described using a stricter variant — no tool calls at all, structured output driving state.
- *DeathArrow* independently built tool-denial extensions and confirmed that models rationalize away prompt-based rules.

Skeptical:
- *esafak:* "so what does the state engine buy you?" — arguing tests and review models suffice; and pressing on states that inherently need LLM judgment: *"Walk me through how you don't/can't hallucinate, given that you need an LLM to determine the state."*
- *esperent:* rule-breaking is usually a context problem — "whenever this happens, it's my fault"; tight prompts under 7k context mostly fix it.
- *DeathArrow* (limitation): blocking is only half the problem — denying a transition "doesn't mean the agent will do it." You can forbid, not compel.
- *prunrCloud:* worried about lost flexibility on "tasks that require more creative exploration."
- *redhale:* prompt-cache invalidation costs from changing tool lists between states.

The sharpest criticism was **licensing, not architecture**: *embedding-shape* said the patent "makes me want to run away and not look into it too deeply"; *striking* noted the repo license omitted the patent grant entirely. The author responded by adding a PATENTS.md exclusion and relicensing most crates as Apache 2.0.

---

### 3.2 Theodosia — the closest philosophical match

- **URL:** [github.com/msradam/theodosia](https://github.com/msradam/theodosia)
- **Language / metadata:** Python 3.11–3.13 (Burr does not yet support 3.14). Apache-2.0. 16 stars, 1 fork, 269 commits on `main`. Created 2026-05-19; last pushed 2026-07-27.
- **Tagline:** *"Put an AI agent on rails: mount a Burr state machine as an MCP server so the agent can only take the next allowed step, with every step recorded and replayable."*
- **Naming:** a play on Apache Burr (named for Aaron Burr); Theodosia Burr was his daughter.

**MCP tool surface (4 core tools):**
- `step(action, inputs)` — the single entry point for all FSM actions. The action namespace lives in the tool's argument schema, so *"FSM complexity changes the schema, not the tool count."*
- `reset_session`
- `fork_at(sequence_id)`
- `fork_from_past(app_id, sequence_id)`
- Plus `list_resources` / `read_resource` added via FastMCP's `ResourcesAsTools` transform, for clients lacking native resource support.

**"What state am I in / what's next?"** Answered via `theodosia://` MCP **resources** rather than tools: `state`, `next`, `graph` (with mermaid and dot renderings), `history`, `trace`, `session`. Additionally, every refusal from `step` includes `valid_next_actions`, so the agent learns the legal moves even when it errs.

**State machines as code, not config.** Defined in Python using Burr primitives: `@action` decorators with `reads`/`writes`, `ApplicationBuilder`, `with_transitions`, `with_entrypoint`. A factory is passed to `mount()` for per-session isolated state. The companion project **Philip** lifts declarative artifacts — Ansible YAML, Mermaid `stateDiagram`, Excalidraw — into Burr apps that Theodosia can then serve. This is the closest anyone has come to ratmac's runbook-as-data property, and it is a bolt-on rather than the core model.

**Guards and validation, layered:**
- Transition conditions gate edges in the graph, e.g. `Condition.expr("stage == 'ordered'")`
- Server-side reachability checks before each action runs; out-of-order calls receive structured refusals
- Five refusal codes: `invalid_transition`, `unknown_action`, `validation_failed`, `action_timeout`, `action_error`
- `theodosia doctor` statically validates the graph for CI
- `theodosia verify` recomputes the hash-chained ledger
- No built-in filesystem/artifact checks per se — but action bodies are arbitrary Python (examples include "real shellouts"), so such checks can be authored. The README is candid: *"the rails are only as tight as the graph you author."*

**Downstream projects (same author):**
- **Semley** — an SRE investigation agent. Burr is the state-machine engine for the investigation graph; Theodosia mounts that graph as a governed MCP server exposing a single `step` tool; a SQLite persister is the memory of record; a hash-chained trail is the audit ledger. The model advances the graph only via the governed step tool. An unreachable action is refused with the valid next actions, and a `conclude` action **must cite a read that actually ran or it is refused** — a genuine evidence guard, and the nearest thing in the field to ratmac's artifact guards.
- **An incident-triage agent** — read-only on-call triage reading metrics, logs, client load, and feature flags through MCP. Winner of the Crusoe challenge at the DevNetwork [AI+ML] Hackathon 2026. A variant is described as an SRE incident-investigation FSM served over MCP by Theodosia, where the agent keeps the full Grafana toolset while the FSM gates the procedure (triage → diagnose → verify → conclude) and the audit trail.

**Strategic note.** The author has stated they are working on **skills-to-state-machine conversions**, on the reasoning that many popular agent skills are already written as phases for a model to follow. If ratmac's pitch is "your SKILL.md phases are advisory; make them a real FSM," this project is already walking toward the same destination from the Python/MCP side.

---

### 3.3 Stratum — closest on runbook-as-data, but gone dark

- **URLs:** [pulsemcp.com/servers/regression-io-stratum](https://www.pulsemcp.com/servers/regression-io-stratum) · registry name `io.github.ruze00/stratum-mcp` · GitHub path previously `github.com/regression-io/stratum/tree/HEAD/stratum-mcp`
- **Status:** The GitHub repo returns **404 via the API** as of 2026-07-27 (private, renamed, or deleted). The PulseMCP listing survives. ~3 stars, ~582 estimated ecosystem visitors. Released 2026-02-23 — the earliest of the three direct competitors.
- **Description:** *"State machine dispatch server that gives AI coding agents structured execution with typed contracts, postcondition enforcement, and auditable traces."*
- **Components:** typed YAML specs, an MCP server (`stratum-mcp`), and a Python library (`stratum-py`) — with postconditions, retries, gates, and auditable execution traces, **explicitly "for Claude Code and Codex."**
- **Assessment:** On paper this is the closest match to ratmac's combination of *runbook-as-data* + *postcondition gates* + *coding-agent target*. In practice it appears abandoned or withdrawn, with negligible adoption. It is the best available evidence that (a) the idea has been tried before, and (b) having the idea is not sufficient — Statewright shipped four months later and took the space.

---

### 3.4 Runners-up worth knowing

- **The stateful `get_step` pattern** — [Medium write-up](https://medium.com/@akitek.mhh/enforcing-multi-step-agent-workflows-with-a-stateful-mcp-tool-5d11fa7c41ae). A single-tool MCP server holding an ordered list of step objects, exposing only `get_step(step_number)`. A temp state file initialized to `{"step_no": 0}`; each valid call checks that the last completed step is `N-1`, returns the instruction text, and advances. Out-of-order calls are deterministically rejected with a message naming the correct next call, and the agent self-corrects. The author frames it as *"an autonomous variation of the human-in-the-loop pattern,"* with an internal state tracker replacing human approval. Motivation: a 10-step workflow "looked clear on paper," but the model still skipped ahead, merged steps, and reordered. Design principle stated bluntly: *"If a workflow must be followed, encode that requirement in the system, not just in the instructions."* Trade-offs the author names: implementation complexity, tool-call quota consumption, and per-step latency. No open-source release.
- **TDD Guard** — [github.com/nizos/tdd-guard](https://github.com/nizos/tdd-guard), TypeScript, **2,283 stars**, created 2025-07-07, pushed 2026-07-06. Intercepts Write/Edit/MultiEdit before execution, validating against file path, intended modifications, current todo list, and latest test results. This is the traction benchmark for "deterministic discipline enforcement for Claude Code" — and it achieves it with **hooks and no user-authored state table at all**. Its author's own caveat is worth internalizing: enforcing test-first mechanically still produced code with "tight coupling, duplication, and poor design," because *"TDD's value comes from the mindset and discipline it instills, not from mechanical rule-following."*
- **Beads** — `bd` CLI, Go, **25,696 stars** (repo now `gastownhall/beads`), created 2025-10-12, pushed 2026-07-27. Steve Yegge's git-backed, SQLite+JSONL issue tracker as coding-agent memory, solving the "50 First Dates" problem. Relevant as **proof that a plain CLI holding external state for coding agents can achieve enormous adoption** — and as a reminder that it did so without any FSM or transition validation. It is a store, not a controller.
- **Microsoft Conductor** — open-source MIT CLI, May 2026. Multi-agent workflows in YAML, deterministic routing, Jinja2 conditions and branching, "the orchestration layer consumes zero tokens," structure fixed at definition time. Engine-drives-agents, so not a competitor — but it is the best example of *runbook-as-data in a CLI* and a useful reference for TOML/YAML schema design.
- **AgentSPEX** (UIUC/ScaleML) — [github.com/ScaleML/AgentSPEX](https://github.com/ScaleML/AgentSPEX). Declarative YAML workflow language with typed steps, branching, loops, and explicit state management, plus Docker sandbox, checkpointing, and trajectory logging. Version-controlled, reproducible workflow artifacts.
- **Edit Approval State Machine (EASM)** — a stateful gatekeeper between an AI coding agent and the filesystem. Small, but architecturally adjacent to ratmac's artifact-guard idea.
- **statelyai/agent** (XState), **reactive-fsm** (zero-dependency TypeScript, controls tool access per conversation state), **MooreLLM** (2024, [HN](https://news.ycombinator.com/item?id=41257561)) — all engine-wraps-LLM. Historical evidence that the FSM-for-agents idea is at least two years old.
- **AWS Step Functions Tool MCP Server** (awslabs) — bridges MCP clients to Step Functions state machines. Enterprise-flavored, IAM-scoped, but the state machines execute business processes rather than gate the agent's own procedure.

---

## 4. Differentiation and exposure analysis

### 4.1 Genuine differentiators

**(a) Artifact guards = filesystem ground truth. This is the strongest defensible edge.**

Compare the guard mechanisms across the field:

| System | What a guard actually checks |
|---|---|
| **Statewright** | Context-data field comparisons: `{"field": "test_result", "op": "eq", "value": "pass"}` — **trusts what the agent reported** |
| **Theodosia** | Burr `Condition.expr` over in-graph state — same trust problem, unless the action body shells out |
| **Stratum** | "Postconditions" — the only near-match, and the project is dark |
| **TDD Guard** | Real test results + file paths — genuine ground truth, but no state table |
| **ratmac** | Filesystem artifact checks gating transitions — **ground truth the agent cannot fabricate** |

Gating on observable filesystem state is qualitatively different from gating on self-reported context fields. An agent that has been told "set `test_result = pass` to advance" will, under pressure, set `test_result = pass`. An agent that must produce a file at a path cannot talk its way past `stat()`. This aligns directly with Anthropic's own long-running-agent guidance, which grounds progress in **git commits and a JSON feature list acting as a test gate** — and specifically notes JSON was chosen because *"the model is less likely to inappropriately change or overwrite JSON files compared to Markdown files."* Semley's `conclude`-must-cite-a-real-read rule is the only comparable evidence guard found in the entire survey.

**(b) CLI, not MCP.** All three direct competitors are MCP-first. The 2026 evidence strongly favors CLI for this class of tool:
- Benchmarked at 4–32× fewer tokens per operation, and 100% vs 72% success rate (ScaleKit, 75 runs, Claude Sonnet 4)
- Cost at 10,000 ops/month: ~$3.20 CLI vs ~$55.20 MCP
- MCP tool definitions alone can consume 16% of a context window for a single server; stacking a few can reach 72% of a 200K window
- Every major coding agent (Claude Code, Codex, Cursor, Aider, OpenHands) is built on bash — models have billions of bash examples in training data and zero MCP schemas
- Operationally: *"MCP servers crash, lose connections, and have startup latency, while a CLI binary is stateless and always available"*

Counter-argument to hold honestly: without structured schemas, agents can enter a "Help Loop," recursively calling `--help` and fragmenting context with untyped documentation. A ratmac CLI must return terse, structured, self-describing output to avoid this.

**(c) TOML runbook with per-phase prompt instructions in one file.** Statewright's JSON schema notably **lacks an explicit per-state prompt/instructions field** — the docs only say the model "gets clear instructions for the current phase." Co-locating the transition table, the per-phase instructions, and the guards in a single human-editable TOML file is a real ergonomic difference, and it is the property that makes "convert your SKILL.md phases into a runbook" a coherent pitch.

### 4.2 Exposures

**(a) A pure CLI cannot enforce anything. This is the most serious issue.**

Statewright pairs its MCP tools with `PreToolUse` hooks, so a violating agent is **blocked at execution time**. Ratmac, as a CLI the agent calls voluntarily, can be simply *not called*. The entire community draws the line at exactly this point, and the numbers are widely cited:

> With "never run `rm -rf`" in CLAUDE.md, Claude followed the instruction about **70%** of the time. With a hook, it is blocked **100%** of the time.

And from the Claude Code documentation ecosystem: *"skills are guidance, not enforcement. Claude tries to follow them, but there's no guarantee. If you need deterministic behavior, use hooks."* Further: *"a skill is model-judgment; a hook is harness-enforcement — this will happen regardless of what the model decides."*

**Implication for ratmac:** without a hook companion, ratmac is a *suggestion engine with good bookkeeping*, not a controller. The honest positioning is either (i) ship a `PreToolUse` hook that refuses edits when the FSM is in a state that disallows them, or (ii) drop the enforcement claim and compete on runbook ergonomics, auditability, and artifact-grounded truth.

**(b) Patent risk.** Statewright claims a method patent on *state-machine-constrained LLM agent tool access at the protocol layer*. Ratmac does not restrict tool access — it gates *transitions* on filesystem artifacts — which likely places it outside the claim. This should be verified by reading `PATENTS.md` and the claim text directly before any public launch. Note also that the community reaction to that patent was strongly negative, which is a positioning opportunity for a cleanly Apache/MIT-licensed alternative.

**(c) Known critiques of the pattern itself** (these apply to ratmac equally):
- **Fixed topology.** Academic critique: *"in all existing approaches, the state topology is determined before the episode begins and cannot change in response to evidence gathered during execution."*
- **Forbidding is not compelling.** DeathArrow's point stands: denying a transition does not make the agent do the right thing. An FSM can prevent a wrong move; it cannot produce a right one.
- **Per-step overhead.** Latency and tool-call budget consumed purely on workflow bookkeeping. The `get_step` author flagged this explicitly; *redhale* flagged prompt-cache invalidation as an additional cost when tool lists change per state.
- **Authoring burden.** *"Authoring and maintenance of explicit workflow blueprints or action schemas demand significant upfront effort, and do not naturally extend to highly flexible, dynamic scenarios."*
- **The unanswered question.** *esafak*'s challenge — "what does the state engine buy you that tests and a review model don't?" — has not been answered by anyone in this space, including Statewright. Ratmac should have an answer before launch. The artifact-guard property is probably that answer: tests tell you *whether* the code is right; an artifact-gated FSM tells you *whether the process was actually followed*, which is what matters for auditability, compliance, and multi-session continuity.
- **Mechanical compliance ≠ quality.** TDD Guard's author found that mechanically enforced test-first still produced tightly-coupled, duplicated, poorly-designed code.

**(d) Adjacent-platform encroachment.** Two things to watch:
- **MCP Tasks primitive** (SEP-1686, shipped experimental, 2026 roadmap published 2026-03-05) introduces protocol-level async task state with submit/poll/fetch semantics and durable `TaskExecution` records. Not an FSM, but it moves "stateful multi-step work" into the protocol layer, which erodes part of the DIY-state-server rationale.
- **Anthropic's own direction.** The official long-running-agent guidance recommends externalized state via files, git, and a JSON test gate — and explicitly does **not** prescribe a phase controller. Dynamic workflows push orchestration into Claude-generated scripts. If Anthropic ever ships a first-class declarative phase controller, this niche closes from above.

### 4.3 Is inversion-of-control recognized or an anti-pattern?

**Recognized, and increasingly the consensus direction.** The 2026 discourse has inverted the framing: the *prompt* is now treated as the anti-pattern for anything that must be enforced.

- Allen Chan's *AI Agent Anti-Patterns* series identifies the "Agent-as-Business-Process Fallacy" — replacing a controlled, auditable process graph with an agent that *approximates* rather than *executes*. His conclusion: *"models approximate rules; they don't execute them — any rule that must be enforced (access control, compliance checks, policy gates) belongs in code, not in a prompt."* And: prompt-based workflows appear to work in demos with well-formed inputs but break in production — *"the failure is not a crash, it's a silent wrong path."*
- The "Blueprint First, Model Second" paper (arXiv 2508.02721) fixes control flow as a deterministic, expert-authored program and invokes the model only at bounded nodes, reporting +10.1 pp average Pass¹ over SOTA LLM agent baselines with dramatically fewer tool calls.
- StateFlow and related work show structured workflows outperform unstructured prompting on multi-step tasks.
- A scheduler-theoretic survey (arXiv 2604.11378) formalizes agent-loop, event-driven, state-machine, and graph/flow execution patterns across 70 projects — the pattern space is now academically mapped.

**But note the crucial nuance:** the community's actual enforcement mechanism of choice is **hooks**, not a queryable FSM. Claude Code's documented architecture is explicit — *"there's no rigid state machine forcing it through phases; the model's reasoning drives the flow."* That gap between "skills describe phases" and "nothing enforces phase order" is real, is widely acknowledged, and is precisely the seam ratmac targets. Statewright targets the same seam and got there first with hooks *plus* an FSM.

---

## 5. Bottom line for ratmac

The niche is **partially occupied**. Building here is defensible, but only on a narrow and specific claim. In priority order:

1. **Artifact guards are the differentiator.** Filesystem/command-verified transitions are ground truth the agent cannot fabricate. No competitor has this as a first-class guard type. Lead with it.
2. **CLI-not-MCP is a real and well-evidenced advantage** on token cost, reliability, and model familiarity — provided output is terse and structured enough to avoid the `--help` loop.
3. **Ship a hook companion or drop the enforcement claim.** A voluntarily-called CLI cannot enforce. Statewright's hook layer is the reason it can make claims ratmac currently cannot.
4. **Check Statewright's patent claims before launch.** Ratmac gating transitions on artifacts, rather than restricting tool access, probably falls outside — but verify.
5. **Have an answer to "why not just tests and a review agent?"** Nobody in this space has one. The strongest available answer: tests verify the *artifact*; an artifact-gated FSM verifies the *process*, which is what survives across sessions and audits.
6. **Watch Theodosia.** It is small today, but its author is explicitly building skills→FSM conversion. That is the same destination from the Python/MCP direction.

---

## 6. Sources

**Direct competitors**
- [Statewright — GitHub](https://github.com/statewright/statewright)
- [Statewright — product site](https://statewright.ai/) · [docs](https://docs.statewright.ai/)
- [Show HN: Statewright — Visual state machines that make AI agents reliable](https://news.ycombinator.com/item?id=48108778)
- [Theodosia — GitHub](https://github.com/msradam/theodosia)
- [Semley (Theodosia-based SRE agent) — MCP Repository](https://mcprepository.com/msradam/semley)
- [Stratum MCP Server — PulseMCP](https://www.pulsemcp.com/servers/regression-io-stratum)

**The inverted-control pattern**
- [Enforcing Multi-Step Agent Workflows with a Stateful MCP Tool — Medium](https://medium.com/@akitek.mhh/enforcing-multi-step-agent-workflows-with-a-stateful-mcp-tool-5d11fa7c41ae)
- [My LLM kept calling tools it shouldn't, so I built a state machine to stop it — DEV](https://dev.to/roddcode/my-llm-kept-calling-tools-it-shouldnt-so-i-built-a-state-machine-to-stop-it-1i5k)
- [Show HN: Library for Finite State Machine Based LLM Agents (MooreLLM, 2024)](https://news.ycombinator.com/item?id=41257561)
- [Deterministic Orchestration: How State Machines Are Replacing Agent Loops in Regulated AI — HackerNoon](https://hackernoon.com/deterministic-orchestration-how-state-machines-are-replacing-agent-loops-in-regulated-ai)
- [Blueprint First, Model Second: A Framework for Deterministic LLM Workflow — arXiv 2508.02721](https://arxiv.org/pdf/2508.02721)

**Claude Code platform and the enforcement gap**
- [Steering Claude Code: CLAUDE.md, skills, hooks, subagents — Anthropic](https://claude.com/blog/steering-claude-code-skills-hooks-rules-subagents-and-more)
- [Effective Harnesses for Long-Running Agents — Anthropic](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)
- [Introducing dynamic workflows in Claude Code — Anthropic](https://claude.com/blog/introducing-dynamic-workflows-in-claude-code)
- [Automate actions with hooks — Claude Code Docs](https://code.claude.com/docs/en/hooks-guide)
- [Claude Code Workflows: Deterministic Multi-Agent Orchestration — alexop.dev](https://alexop.dev/posts/claude-code-workflows-deterministic-orchestration/)
- [Dive into Claude Code: The Design Space of Today's and Future AI Agent Systems — arXiv 2604.14228](https://arxiv.org/html/2604.14228v2)

**Adjacent tooling**
- [awesome-harness-engineering — GitHub](https://github.com/ai-boost/awesome-harness-engineering)
- [TDD Guard — GitHub](https://github.com/nizos/tdd-guard)
- [Beads — Steve Yegge's coding agent memory system](https://steve-yegge.medium.com/the-beads-revolution-how-i-built-the-todo-system-that-ai-agents-actually-want-to-use-228a5f9be2a9)
- [Conductor: Deterministic orchestration for multi-agent AI workflows — Microsoft Open Source](https://opensource.microsoft.com/blog/2026/05/14/conductor-deterministic-orchestration-for-multi-agent-ai-workflows/)
- [workflows-mcp-server — GitHub](https://github.com/cyanheads/workflows-mcp-server)
- [spec-workflow-mcp — GitHub](https://github.com/Pimzino/spec-workflow-mcp)
- [specs-workflow-mcp — GitHub](https://github.com/kingkongshot/specs-workflow-mcp)
- [AWS Step Functions Tool MCP Server — awslabs](https://awslabs.github.io/mcp/servers/stepfunctions-tool-mcp-server)

**Framework landscape / who controls the loop**
- [Managed Agents vs LangGraph vs Rolling Your Own: Who Should Run Your Agent Loop in 2026 — Developers Digest](https://www.developersdigest.tech/blog/managed-agents-vs-langgraph-vs-diy-2026)
- [Choosing an agent framework: LangChain vs LangGraph vs CrewAI vs PydanticAI vs Mastra vs Vercel AI SDK — Speakeasy](https://www.speakeasy.com/blog/ai-agent-framework-comparison/)
- [Agents — Pydantic AI docs](https://ai.pydantic.dev/agent/)
- [Apache Burr — GitHub](https://github.com/apache/burr/)
- [LangGraph overview — LangChain docs](https://docs.langchain.com/oss/python/langgraph/overview)
- [Agentic AI Workflows: Why Orchestration with Temporal is Key — IntuitionLabs](https://intuitionlabs.ai/articles/agentic-ai-temporal-orchestration)

**CLI vs MCP**
- [MCP vs. CLI for AI agents: When to Use Each (2026 decision framework)](https://manveerc.substack.com/p/mcp-vs-cli-ai-agents)
- [AI agents need two interfaces: CLI and MCP — RudderStack](https://www.rudderstack.com/blog/ai-agents-cli-mcp-design-pattern/)
- [Writing CLI Tools That AI Agents Actually Want to Use — DEV](https://dev.to/uenyioha/writing-cli-tools-that-ai-agents-actually-want-to-use-39no)

**Critique and counter-position**
- [AI Agent Anti-Patterns (Part 5): The Illusion of Control — Allen Chan](https://achan2013.medium.com/agent-anti-patterns-part-5-05da1c3c1828)
- [AI Agent Anti-Patterns (Part 1): Architectural Pitfalls — Allen Chan](https://achan2013.medium.com/ai-agent-anti-patterns-part-1-architectural-pitfalls-that-break-enterprise-agents-before-they-32d211dded43)
- [Why Your AI Agent Needs a State Machine, Not a Prompt Chain — Brightlume AI](https://brightlume.ai/blog/why-ai-agent-needs-state-machine-not-prompt-chain)
- [Agent-S: LLM Agentic workflow to automate SOPs — arXiv 2503.15520](https://arxiv.org/pdf/2503.15520)

**Protocol direction**
- [MCP Roadmap 2026 — analysis](https://tekkminds.com/blog/2026/03/the-mcp-roadmap-2026-whats-actually-changing/)
- [Architecting the Asynchronous Agent: A Guide to MCP Tasks — Medium](https://stn1slv.medium.com/architecting-the-asynchronous-agent-a-guide-to-mcp-tasks-7348c6527233)

---

## Appendix: Repository metadata snapshot (verified via GitHub API, 2026-07-27)

| Repo | Language | Stars | Created | Last push |
|---|---|---|---|---|
| `statewright/statewright` | Rust | 418 | 2026-05-03 | 2026-07-23 |
| `msradam/theodosia` | Python | 16 | 2026-05-19 | 2026-07-27 |
| `ruze00/stratum` | — | — | — | **404 Not Found** |
| `steveyegge/beads` → `gastownhall/beads` | Go | 25,696 | 2025-10-12 | 2026-07-27 |
| `nizos/tdd-guard` | TypeScript | 2,283 | 2025-07-07 | 2026-07-06 |
