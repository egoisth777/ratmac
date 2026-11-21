# Agent Graph Engineering — Index

**Date:** 2026-11-21
**Subject:** ratmac — how other tools let agents walk a graph of steps, and what ratmac itself does today
**Scope:** Thirteen finished reports, stored verbatim. Each file has a short header: date, producing agent, method, known defects.

---

## Files

- `01-agent-graph-frameworks.md` — 17 agent graph frameworks (LangGraph, Microsoft Agent Framework, Burr, XState and others) compared on ten questions.
- `02-workflow-engines.md` — 32 durable workflow engines and automation services (Temporal, Step Functions, Camunda, n8n and others) compared on the same questions.
- `03-coding-agent-process-tools.md` — 21 coding-agent process tools (Claude Code, Spec Kit, Beads, StateM, GateReeve and others); who advances, who works, who accepts completion.
- `04-standards-and-formal-models.md` — 20 standards, formal models and research patterns (statecharts, SCXML, BPMN, Petri nets, behavior trees, MCP Tasks and others).
- `05-experiment-pipeline-tools.md` — 26 experiment, pipeline and runbook tools (Snakemake, DVC, Luigi, AI Scientist, Ansible and others).
- `06-statem-close-read.md` — close source read of StateM at revision `51e4ae5`, the nearest competitor: 14 questions answered with line references.
- `07-ratmac-baseline.md` — what ratmac (`rtm`) does today, read from code at `main` 81d54d8: 12 questions answered with line references.
- `08-theory-models.md` — theory of state machines, graphs and related models: 40 findings from 22 sources on FSMs, statecharts, Petri and workflow nets, dataflow, Pregel, behaviour trees, BPMN and actors, and when "a process is a graph" holds or breaks.
- `09-engineering-practice.md` — how practitioners engineer agent graphs: 43 findings from 25 sources on fixed paths versus agent loops, common patterns, limits, state and recovery, testing and observation, and failures and custom implementations.
- `10-academic-research.md` — research papers on graph and state-machine structure for agents: 34 findings from 24 sources on surveys, foundational experiments, generated workflows, constraints and verification, StateM's results and limits, where imposed structure hurts, and the false-success study.
- `11-failure-evidence.md` — measured evidence of the pain: 24 findings from 14 sources on false completion, long-task reliability, multi-agent failures, token waste, review burden, and enforced workflows with counterexamples.
- `12-format-fluency.md` — process formats that models already know: 43 findings on 13 candidate formats (XState, SCXML, ASL, BPMN, Mermaid, DOT, PlantUML, Actions YAML, Makefile and others) covering popularity, expressiveness, parsers, licences and measured generation accuracy.
- `13-agent-native-runners.md` — agent-native runners and nearest products (StateM, IX Flow, Statewright, Spec Kit, Beads and others): storage, concurrency, complaints, and what "agent-native" means.

## Synthesis

- `synthesis.md` — a traced summary of the thirteen reports in ten sections; no recommendation; audited twice.

## Read first

Files 01–05 compared ratmac using an inaccurate summary. Their statements about ratmac's position or uniqueness are unreliable. `07-ratmac-baseline.md` is the correct account of ratmac. The full note is in each file's **Known defects** line.

## Pending

Nothing is pending.
