# State-Machine Standards and Formal Models

**Date:** 2026-11-21
**Producing agent:** `astra-state-machine-standards`
**Method:** web research only, nothing installed or run
**Known defects:** These reports compared ratmac using an inaccurate summary: it said ratmac checks run nothing and a runbook cannot change during a Run. Both are wrong; see 07-ratmac-baseline.md. Every statement here about ratmac position or uniqueness is therefore unreliable. Also wrong in 04: "every model needs an explicit loop limit"; a state machine may run without end by design, and a bound is a choice per job.

---

## Comparison

**Reading key:** column 3 lists **mover / worker / completion decision**. **Host** means the application must supply the feature. Unless stated, history editing and retention are host-defined. **Local** assumes local models and dependencies. Standard-document terms are not software licences. Ratmac facts come from the supplied brief.

| Product, ownership, licence, maturity | Definition; contribution; agent use | Move / work / done | Branches and loops | State, history and carried data | Approval, recovery, deployment |
|---|---|---|---|---|---|---|
| **XState / Stately** — Stately; MIT core, commercial Studio; v6 beta [1] | JS/TS/canvas; executable statecharts; agents | Events/runtime / code actors / events and guards | Guards; cycles; authored bounds | Memory, host snapshots; context | Modeled approvals, errors, timers; local core; hosted/self-hosted Studio |
| **Stately Agent** — Stately; MIT; version 2 alpha [2] | TS/JSON; models choose legal events | Host/runtime / supplied executors / machine guards | Closed choices; model-call caps | Append-only host journal, snapshots; context | Resume, retries, errors; local/custom hosts; multiple model adapters |
| **Hierarchical state machines/statecharts** — Harel lineage; established formalism [3] | Diagrams/code; hierarchy, parallel regions, history; used by XState | Events/interpreter / host actions / transition semantics | Guards; cycles; no inherent bound | Active configuration, remembered substates; host data | Host policies; no inherent server/model requirement |
| **SCXML** — W3C; document terms; 2015 Recommendation [4] | XML statecharts; standardized semantics; indirect XState influence | Interpreter / actions/invocations / events | `cond` guards; unrestricted cycles | Configuration, queues, datamodel; persistence unspecified | Events, errors, delayed sends; implementation-dependent |
| **Amazon States Language** — Amazon; custom specification licence; production Step Functions [5] | JSON/canvas; Parallel/Map; Bedrock integration | Runtime / services or workers / return or callback | Choice expressions; cycles, counters/timeouts | Standard: service history, normally 90-day retention; JSON | Callbacks, Retry/Catch, timeouts; AWS reference runtime requires cloud |
| **CNCF Serverless Workflow → Open Workflow Specification** — Apache-2.0; 1.1 [6] | YAML/JSON; forks, events; MCP/A2A calls | Interpreter / services/scripts / responses/events | Switch, iteration, flow directives; authored bounds | Host storage; inputs, outputs, exported context | Events, suspension, retries/timeouts; runtime-dependent, vendor-neutral |
| **BPMN** — OMG; document terms; formal 2.0.2 [7] | Canvas/XML; gateways, compensation; Camunda agents | Engine / workers or humans / completion events | Conditions; loops/multi-instance; modeled bounds | Engine-defined storage/history; process variables | User tasks, timers, errors, compensation; local/server implementations |
| **DMN** — OMG; document terms; 1.6 formal, 1.7 beta [8] | Tables/XML/FEEL; explicit decision rules; agent adoption unverified | Caller/evaluator / expressions / returned decision | Hit policies; not a process-loop runner | Inputs/decision outputs; audit storage external | Approval/resume/retries external; embeddable evaluator |
| **Petri nets / workflow nets** — Petri/van der Aalst; established theory [9] | Places, transitions, tokens; concurrency and soundness analysis; mainstream agent adoption unverified | Enabled firing / abstract task / marking condition | Conflicting transitions; cycles; soundness is not unconditional termination | Marking/tokens; storage and audit external | Human work/failures require modeling; no deployment requirement |
| **BehaviorTree.CPP** — Faconti/contributors; MIT; released robotics library [10] | C++/XML; reactive sequences/fallbacks; game/robotics pattern | Caller ticks / leaf code / SUCCESS–FAILURE–RUNNING | Child status; repeat/retry decorators and limits | Memory/loggers; blackboard and ports | Custom approvals; halt/retry/timeout; local |
| **Dendron** — Richard Kelley; BSD-3-Clause; research implementation [11] | Python; behavior-tree LLM leaves | Caller ticks / models/code / leaf status | Conditions/fallbacks; repeat/retry limits | Memory blackboard/history; named outputs | Custom approvals; timeout/retry decorators; local Transformers |
| **Outlines** — .txt; Apache-2.0; released library [12] | Schemas/regex/grammars; structured model outputs | Decoder / model / grammar acceptance or stopping | Allowed-token masks; token limit | Decoder memory; generated text; host history | Approval/recovery external; local/cloud backends |
| **Guidance** — guidance-ai; MIT; released library [13] | Python plus regex/CFGs; interleaved generation/control | Python/decoder / model and code / return or stopping | `select`, Python loops; token/custom limits | Memory model objects, named captures; host history | Approval/recovery external; local/cloud backends |
| **StateFlow** — Wu et al.; repository licence not found; COLM 2024 [14] | Python/prompts; AutoGen state-based speaker selection | Selector / models/tools / code checks and model submission | Authored transitions; turn caps | Memory plus JSON logs; messages/results | Error-repair states; durable approvals/resume unspecified; OpenAI/Azure demonstrations |
| **AFlow** — FoundationAgents; MIT; ICLR 2025 [15] | Generated Python; searches workflow designs | Optimizer/runtime / models/code / evaluation and returns | Python branches/loops; optimization-round cap, not universal execution bound | Memory/editable workspace files; workflows/results | Durable approvals/resume unspecified; model-endpoint dependent |
| **LATS** — Zhou et al.; MIT; ICML 2024 [16] | Python search tree; Monte Carlo search/reflection | Search algorithm / models/tools / reward or terminal result | Scored selection; trajectory/iteration limits | Memory plus logs; observations, values, trajectories | Search retries; durable approvals/resume unspecified; OpenAI demonstration |
| **Plan-and-Execute** — pattern; LangChain reference, MIT LangGraph [17] | Code graph plus model-generated plan | Runtime / executor tools/models / replanner response | Replan loop; graph step limit | Host checkpoints; editable plan/results state | LangGraph interrupts/resume; local/cloud models |
| **A2A** — Google-origin, Linux Foundation; Apache-2.0; SDK ecosystem [18] | Protocol task lifecycle, not workflow definition | Serving agent / its implementation / reported status | Internal graph unspecified; terminal tasks cannot restart | Server tasks; messages/artifacts; retention unspecified | Input/auth pauses, cancellation/failure; server required, cloud optional |
| **MCP Tasks** — MCP maintainers; Apache-2.0 extension; stable 2026 schema [19] | Long-running operation lifecycle, not workflow graph | Server / tool implementation / server status | Internal branching unspecified; input/resume cycle | Durable task handle/results; TTL permits expiry | Poll/resume/input/cancel/failure; server required; client support varies |
| **Bounded Loops & Graphs** — Qualixar; Apache-2.0; small research project [20] | YAML; independent gates and conditional termination proofs; agent integrations | Engine / launched worker / independent gate | Joins/repairs; declared global budgets | Local append-only hash-chained ledger; artifacts/verdicts | Approvals, resume, retries, limits; local, multiple agent tools |

## A. Three recurring choices — assessment

- Runtime advances after events or returns, rather than an outside caller requesting one checked move.
- Data objects/messages carry progress; repository artifacts are rarely the primary interface.
- Graph structure constrains choices, but loops still need explicit bounds. [1–20]

**Important distinctions:** history states are not audit logs; workflow-net soundness does not guarantee every possible run terminates; constrained decoding guarantees permitted syntax, not successful work. Guidance’s CFG support is not merely finite-state decoding. [3,9,12,13]

## B. Publicly reported problems

- **XState:** restored v5 timers failed to fire. The open issue now distinguishes v6 wall-clock restoration from simulated-clock behavior; do not generalize the old failure. [21]
- **Outlines:** repetitive generation hit the token cap and left invalid JSON. Historical, closed report. [22]
- **MCP Tasks:** an October client-support request describes relay timeouts and model-driven polling consuming turns. This is reported integration friction, not a universal protocol limitation. [23]

## C. Closest to ratmac — assessment

- **Bounded Loops:** already has local file evidence, pinned graphs and append-only receipts. Unlike ratmac, it launches workers and enforces retries/budgets. [20]
- **Stately Agent:** supports external host control, legal-event choices and append-only replay. It is a programmable SDK requiring host integration, not a ready-made repository-file gate CLI. [2]

## D. Remaining opportunity — inference

No inspected offering documents ratmac’s exact combination: **caller-stepped, no work execution, pinned TOML, repository-file checks, closed-word branching with reasons**. That is a useful product boundary, not a novel formal model. File-based gates and append-only history alone are not unique. [2,20]

## E. Common capabilities ratmac lacks

Executable frameworks commonly provide tool/model execution, timers, retries, interactive inspection and richer data passing. Some add compensation or conditional formal verification. These are not universal features of standards themselves. [1,2,5–7,10,20]

**Skipped:** exhaustive engine inventories, performance reproduction and unsupported adoption estimates. Added Dendron and Bounded Loops. No requested name was missing.

## Sources

All primary sources; research results remain their authors’ claims.

1. **XState/Stately:** https://github.com/statelyai/xstate · https://stately.ai/docs/persistence · https://stately.ai/ · https://stately.ai/terms
2. **Stately Agent:** https://github.com/statelyai/agent · https://github.com/statelyai/agent/blob/next/docs/any-stack.md · https://github.com/statelyai/agent/blob/next/docs/event-log.md · https://github.com/statelyai/agent/blob/next/docs/usage-and-budgets.md
3. **Harel’s original statecharts paper, mirrored:** https://dubroy.com/refs/Statecharts_a_visual_formalism_for_complex_systems.pdf
4. **SCXML Recommendation:** https://www.w3.org/TR/scxml/
5. **ASL and AWS reference implementation:** https://states-language.net/ · https://docs.aws.amazon.com/step-functions/latest/dg/service-quotas.html · https://docs.aws.amazon.com/step-functions/latest/dg/connect-to-resource.html · https://docs.aws.amazon.com/bedrock-agentcore/latest/devguide/gateway-target-connector-step-functions.html
6. **Open Workflow Specification:** https://github.com/open-workflow-specification/specification · https://github.com/open-workflow-specification/specification/blob/main/dsl-reference.md · https://open-workflow-specification.org/blog/general/announcing-rename-to-open-workflow-specification/ · https://open-workflow-specification.org/blog/releases/release-110/
7. **BPMN and agent implementation:** https://www.omg.org/spec/BPMN · https://www.omg.org/spec/BPMN/2.0.2/PDF · https://docs.camunda.io/docs/components/connectors/out-of-the-box-connectors/agentic-ai-aiagent-tool-definitions/
8. **DMN:** https://www.omg.org/spec/DMN · https://www.omg.org/spec/DMN/1.5/PDF
9. **Workflow-net definitions and soundness:** https://www.vdaalst.com/publications/p44.pdf
10. **BehaviorTree.CPP:** https://github.com/BehaviorTree/BehaviorTree.CPP · https://www.behaviortree.dev/docs/learn-the-basics/BT_basics/ · https://www.behaviortree.dev/docs/tutorial-basics/tutorial_02_basic_ports/ · https://github.com/BehaviorTree/btcpp_website/blob/master/docs/nodes-library/DecoratorNode.md
11. **Dendron:** https://github.com/RichardKelley/dendron · https://github.com/RichardKelley/dendron/blob/main/docs/0_tutorial_single_node.md · https://arxiv.org/abs/2404.07439
12. **Outlines:** https://github.com/dottxt-ai/outlines · https://github.com/dottxt-ai/outlines-core
13. **Guidance:** https://github.com/guidance-ai/guidance
14. **StateFlow:** https://github.com/yiranwu0/StateFlow · https://github.com/yiranwu0/StateFlow/blob/main/InterCode/experiments/flow_bash.py · https://github.com/yiranwu0/StateFlow/blob/main/InterCode/experiments/eval_stateflow_bash.py
15. **AFlow:** https://github.com/FoundationAgents/AFlow · https://github.com/FoundationAgents/AFlow/blob/main/config/config2.example.yaml
16. **LATS:** https://github.com/andyz245/LanguageAgentTreeSearch · https://arxiv.org/abs/2310.04406
17. **Plan-and-Execute/LangGraph:** https://www.langchain.com/blog/planning-agents · https://github.com/langchain-ai/langgraph · https://docs.langchain.com/oss/python/langgraph/overview · https://docs.langchain.com/oss/python/langgraph/graph-api-control-flow
18. **A2A:** https://github.com/a2aproject/A2A · https://a2a-protocol.org/latest/topics/life-of-a-task/
19. **MCP Tasks:** https://github.com/modelcontextprotocol/ext-tasks · https://modelcontextprotocol.io/extensions/tasks · https://github.com/modelcontextprotocol/ext-tasks/blob/main/specification/draft/tasks.md
20. **Bounded Loops:** https://github.com/qualixar/bounded-loops · https://arxiv.org/abs/2609.27871
21. **XState timer report:** https://github.com/statelyai/xstate/issues/4860
22. **Outlines repetition report:** https://github.com/dottxt-ai/outlines/issues/1131
23. **MCP Tasks client-support request:** https://github.com/anthropics/claude-code/issues/102347
