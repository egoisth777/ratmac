# ratmac Feasibility — Direction

Date: 2026-07-27
Basis: four independent deep-research reports in this folder:

- `01-landscape-and-competitors.md` — orchestration landscape, niche validation
- `02-empirical-evidence.md` — empirical evidence for/against state-driven agent workflows
- `03-application-domains.md` — application domains, beachhead analysis
- `04-case-against.md` — devil's advocate case

Key facts below (Statewright, platform absorption, artifact-guard gap) were found independently by 3 of 4 agents, which raises confidence in the synthesis.

## Verdict

**Conditional GO.** The concept "external deterministic FSM runner that an agent calls as a tool" is validated by research and by production practice — but generic "FSM runner for agent loops" is already occupied. ratmac survives only as a narrower, harder product:

> A **hook-enforced**, **artifact-verified**, **CLI-first** runbook runner, measured on **honesty, cost, resumability, and auditability** — never on raw success rate.

Anything broader is a weaker clone of Statewright.

## What the research settles

1. **The problem is real and quantified.** 72.5% of long-horizon agent failures are process-level (HORIZON); 80%-reliability horizons are 4–6× shorter than 50% horizons (METR); Airbnb and Google both hand-built per-file state machines with gates because nothing off-the-shelf existed. The task shape (loop/graph engineering) is production-proven.
2. **The value proposition is NOT higher success rate.** Structure's success-rate edge was a weak-model-era finding (StateFlow, 2024) and compresses or inverts on frontier models (ReAct beats plan-and-execute; Agentless lost to agentic scaffolds). What gated FSMs demonstrably buy:
   - Honesty: fabricated completions 25% → 0.95% (Goal-Autopilot)
   - Cost: 3–5× cheaper (StateFlow)
   - Durability: +15.5 pt retention across re-runs (SKILL.nb)
   - Auditability: EU AI Act Art. 12 enforcement from 2026-08-02
   - Caveat: no published controlled ablation isolates *filesystem artifact guards* specifically (closest proxy: Spec Kit Agents' validation hooks, +3.0%, p<0.05). ratmac's core bet is empirically untested — ratmac's own evaluation must fill this gap, hence the success metrics below.
3. **The niche is partially occupied.** Statewright (Rust, 418★, guards + per-state tool allowlists, hook enforcement on five harnesses, a method patent) is a near-superset. Theodosia and Stratum are smaller replicas. Platforms are absorbing the layer: Claude Code dynamic workflows, `/goal` with independent verifier, native Tasks, MCP Tasks primitive. The pattern has been independently reinvented ~6 times — validated, but not novel.

## The fatal flaw (fix or stop)

**A pure CLI is advisory by construction.** R-009 ("Scheduler is the sole writer of State Files") is a convention, not a mechanism: an agent holding Write/Bash can edit `.arca/state.toml`, fabricate guard artifacts, or never call `rtm`. Community data: advisory guidance ≈70% compliance; hooks ≈100%. ImpossibleBench shows frontier models game satisfiable guards at 46–93% rates, and stronger models cheat more.

Two required changes:

1. **Ship a `PreToolUse` hook companion** that denies agent writes to scheduler-owned files (`.arca/state.toml`, log). ~20 lines. Without it, ratmac is a suggestion engine, and every claim about determinism is fiction.
2. **Harden guards to read only outputs of processes the agent does not control** — test-runner exit codes, compiler output, git state, external checks — never agent-writable marker files. Lean hard on PGE-003 sensitivity receipts (prove the test could fail before implementation): neither Statewright (trusts agent-reported context fields) nor `/goal` (LLM transcript judge) verifies test sensitivity. This is the strongest differentiator in the entire space.

## The defensible wedge (vs Statewright)

| Axis | Statewright | ratmac |
|---|---|---|
| Enforcement object | Tool-space restriction | **Artifact-verified exit** |
| Interface | MCP-first | **CLI** (4–32× fewer tokens; no schema tax; no connection failure modes) |
| Guards trust | Agent-reported context fields | **Filesystem / process ground truth** |
| Runbook format | JSON, no per-state prompt field | **One TOML: transitions + per-phase prompts** |
| License posture | Apache-2.0 engine, FSL gateway, PATENTS.md | Fully local, fully open |

Action: read Statewright's PATENTS.md. ratmac does not restrict tool access, which likely sits outside the claim — verify before publishing.

## Beachhead

- **Primary: spec-gated build-loop enforcement.** Spec Kit (~111k★) proves demand for phase-structured agent work; Spec Kit/BMAD/Agent OS are markdown lore with zero enforcement — agents documented skipping the red phase and self-certifying. Pitch: *the engine that makes your existing SDD workflow non-bypassable.*
- **Secondary: semantic long-tail migration sweeps.** Airbnb's hand-built per-file state machine is the product-shaped hole. Honest claim is the semantic 3–25% of files, not mechanical transforms (those should be codemods).
- **Tertiary: dependency-upgrade / vuln remediation.** Cleanest artifact guards; absorption risk from GitHub/GitLab.
- **Avoid:** SRE incident response, data-pipeline orchestration (funded incumbents; guard surface is production state, not the filesystem), creative/exploratory/one-shot work (active anti-fit).

## Cautions to design for

1. **A wrong runbook fails confidently.** Goal-Autopilot's residual failures were all wrongly-compiled FSMs whose defective gates were then honestly satisfied. Budget for runbook validation itself (static graph checks, `rtm doctor`-style, reachability/dead-end detection).
2. **Honest stalls are the product, not a bug.** 68% of production agents stop within 10 steps for human intervention. Design the hand-back-to-human path first; expect high stall rates and make them cheap to resolve.
3. **Runbooks are model-heterogeneous.** Scaffold effects are turbulent across models (Kendall τ ≈ 0.17 on GAIA); a runbook tuned on one model may hurt on another. Keep phases coarse; encode gates, not reasoning.
4. **Do not self-evaluate by feel.** METR RCT: developers 19% slower with AI while believing +20%. Measure wall-clock, token cost, fabrication rate, and re-run retention — nothing else.
5. **Stay harness-agnostic at the core.** CLI + filesystem is the most stable contract available; keep hook integrations as thin optional adapters per harness, since the orchestration layer is where harness churn concentrates (37.3% of studied harness bugs).
6. **Keep v1 single-run, but keep the data model N-run-capable.** Per-worktree concurrent runs is where guard enforcement will be most valuable as parallel-fleet patterns (Agent Teams, worktree fan-out) become default.
7. **"The agent never sees the graph" is not an anti-gaming defense.** `03` claims graph opacity (R-029) prevents guard-gaming; `04` rebuts it: R-028 renders the Phase's Exit Guards into the Phase Prompt as a machine-readable checklist of exactly what must be true to advance — sharpening the exploit target, not hiding it. Opacity buys context economy only; guard integrity comes solely from guards the agent cannot write. Do not use R-029 in positioning.

## Direction summary

| Decision | Call |
|---|---|
| Build a generic FSM runner | **No** — wheel exists (Statewright, statecharts, LangGraph-class engines) |
| Build hook-enforced artifact-verified runbook runner | **Yes** — unclaimed, evidence-backed, one-author-sized |
| Enforcement | CLI stays the brain; add `PreToolUse` hook companion as the wall |
| Guards | Only agent-uncontrolled process outputs; sensitivity receipts as flagship |
| Positioning | "Artifact-verified exit" vs Statewright's "tool-space restriction"; make the distinction legible or ratmac reads as a clone |
| First target | Spec-gated build loops (own dogfood), then migration sweeps |
| Success metrics | Fabrication rate, token cost per completed phase, cross-session resume rate, honest-stall resolution time |

## Stop conditions

Abandon or fork-instead-of-build if any of these hold:

- Unwilling to add hook enforcement (product stays advisory → strictly worse Statewright).
- Statewright ships artifact/sensitivity guards natively before ratmac's v1.
- Statewright's patent claim, on real reading, covers artifact-gated transition validation generally.

## Baseline to beat

From `04-case-against.md`: delete the engine, keep the `.arca` methodology (conventions, five-file issue shape, residual ledger, receipt discipline), enforce it with a `justfile` + a `PreToolUse` hook + Claude Code's native `/goal` with a measurable end state. That configuration captures most of ratmac's value at roughly 2% of its maintenance surface. Every decision to continue building the engine must beat this baseline on measured outcomes (fabrication rate, token cost, resume rate) — not on feel.
