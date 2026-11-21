# Agent Graph Frameworks

**Date:** 2026-11-21
**Producing agent:** `astra-agent-graph-frameworks`
**Method:** web research only, nothing installed or run
**Known defects:** These reports compared ratmac using an inaccurate summary: it said ratmac checks run nothing and a runbook cannot change during a Run. Both are wrong; see 07-ratmac-baseline.md. Every statement here about ratmac position or uniqueness is therefore unreliable. Also wrong in 04: "every model needs an explicit loop limit"; a state machine may run without end by design, and a bound is a choice per job.

---

## Comparison

Release labels indicate published maturity, not measured adoption. **A2** means Apache-2.0. In execution cells, **→** identifies what counts as completion.

**Q7 caveat:** memory-only history disappears on exit. Where unspecified, history editing and expiry guarantees were not verified.

**Q10, all rows:** local library execution is available; no mandatory orchestration cloud or single-model vendor. **Inference:** offline operation requires local models, tools and storage, with network exporters disabled. This was not tested.[1–17,19]

| Product, maker, licence, maturity (1) | Definition (2) | Advancement / work / completion (3–5) | Branches / loop bounds (6) | State, history / carried data (7–8) | Approval, resume, failures (9) |
|---|---|---|---|---|---|
| **LangGraph**; LangChain; MIT; 1.x.[1] | Python/TypeScript graph | Runtime; functions/agents → return | Code predicates; `recursion_limit` | Shared state; memory/database checkpoints; deletable history | Interrupt/resume; retries/timeouts |
| **Microsoft Agent Framework**; Microsoft; MIT; production 1.x.[2] | Code-defined typed graph | Runtime; executors/agents → messages | Conditional edges; `max_iterations` | Messages/shared state; memory, pickle files or database; checkpoints deletable | Human requests; checkpoint restore; application error policies |
| **AutoGen GraphFlow**; Microsoft; MIT; experimental, project archived.[3] | Python `DiGraphBuilder` | Runtime; agents → responses | Message predicates; `max_turns`/termination conditions | Messages; team state exportable as JSON; graph-specific recovery completeness unverified | Stop/save/restart; GraphFlow-specific approval details unverified |
| **CrewAI Flows**; CrewAI; MIT; 1.x.[4] | Python decorators | Runtime; methods/crews → return | Router labels; application counters | Dictionary/typed state; optional SQLite snapshots | Human feedback/resume; agent retries; flow exceptions |
| **Google ADK 2.0**; Google; A2; released graph runtime.[5] | Code graphs/dynamic workflows | Runtime; functions/agents → events | Event routes; explicit bounds; Python `LoopAgent` deprecated | Events/shared state; memory/database sessions; sessions deletable | Human input/resume; node retries/timeouts |
| **OpenAI Agents SDK**; OpenAI; MIT; actively released 0.x.[6] | Agents, tools, handoffs—not general graph DSL | Runtime; models/tools → final output | Model handoffs/tool choices; `max_turns` | Messages/context; database sessions; serializable paused `RunState` | Tool approval/resume; exceptions |
| **LlamaIndex Workflows**; LlamaIndex; MIT; 2.x, now in `llama-agents` repository.[7] | Python typed-event steps | Runtime; handlers → events | Emitted event types; counters/timeouts | Events/context; memory, serialized context or database-backed server | Human events; resume; retries/timeouts |
| **pydantic-graph**; Pydantic; MIT; V2.[8] | Python nodes/builders | Runtime or caller `.next()`; nodes → next node/End | Returned nodes/decisions; application bounds | Typed memory state; **V2 removed graph persistence** | Caller-controlled stepping; exceptions; durable recovery needs another layer |
| **Apache Burr**; ASF, formerly DAGWorks; A2; incubating.[9] | Python actions/transitions | Caller `.step()` or runtime; actions → return | State predicates; application bounds | State values; local JSON/JSONL tracking or database persistence | Halt/resume; exceptions; custom recovery |
| **Haystack pipelines**; deepset; A2; 3.x.[10] | Python/YAML pipelines | Runtime; components → outputs | Routers; `max_runs_per_component` | Named outputs; memory/file snapshots | Confirmation hooks; synchronous breakpoints; failure snapshots |
| **Mastra workflows**; Mastra; A2 core, separate enterprise licence; 1.x.[11] | TypeScript workflow builder | Runtime; steps → return | Predicates; `iterationCount` bounds | Outputs/shared state; database snapshots | Suspend/resume; configurable retries |
| **Semantic Kernel Process Framework**; Microsoft; MIT; experimental, successor is MAF.[12] | Code-defined steps/events | Runtime; kernel functions → events | Event routing; application bounds | Step state/event parameters; memory/files or Dapr storage | Human events; idle/resume; application error handling |
| **Strands graphs**; AWS; A2; production SDK, now `harness-sdk` repository.[13] | Code graph builder | Runtime; agents/custom nodes → return | Predicates; `max_node_executions` | Results/messages; memory/files/S3; sessions deletable | Interrupt/resume; timeouts; exceptions/failed results |
| **smolagents**; Hugging Face; A2; 1.x.[14] | Code/tools; ReAct loop, not graph DSL | Runtime; generated code/tools → `final_answer` plus optional checks | Model choices; `max_steps` | Editable in-memory action history | Approval callbacks; cooperative interruption; memory continuation |
| **DSPy**; Stanford; MIT; 3.x.[15] | Python modules/ReAct, not workflow engine | Runtime; models/tools → submission | Model tool choices; `max_iters` | Messages/history in memory; program saving excludes run history | Custom approvals; history continuation; tool errors returned to model |
| **Agno Workflows**; Agno; A2; 3.x.[16] | Python steps/containers | Runtime; functions/agents → return | Router/Condition; `max_iterations` | Outputs/session state; database-backed runs | Approval/`continue_run`; step retries/error policies |
| **XState**; Stately; MIT; stable v5, v6 beta.[17] | JavaScript/TypeScript statecharts | External/internal events; optional actors; guards/done events govern moves | Guarded transitions; application loop bounds | Context; caller-stored JSON snapshots; event log separately implemented | Approval states; snapshot restore; `onError`, timers |

## A. Three common design choices

* **Code defines control flow.** Prompts usually describe node work, not the authoritative graph.[1,4,7,11]
* **The runtime performs work and advances execution.** Manual stepping usually still executes the selected node.[8,9]
* **Persistence captures program state.** A saved checkpoint is not independent proof that an external task succeeded.[1,2,7]

## B. Publicly reported problems

First-hand reports; not reproduced here. Current fix status was not established.[18]

* **LangGraph:** resuming parallel interrupts reran an already-completed sibling, duplicating an external write.
* **CrewAI:** checkpoint resume skipped remaining loop iterations and the finishing method without reporting failure.
* **Mastra:** a crash after resume lost the saved suspension payload.

## C. Closest to ratmac

* **Burr:** closest stepping/storage combination. It already has local JSONL tracking. Unlike ratmac, `.step()` executes an action.[9]
* **XState:** closest external-event/guard model. File inspection, persistence and a command-line contract require application code.[17]
* **pydantic-graph:** explicit caller stepping, but nodes execute Python; current graph persistence is absent.[8]
* **MAF/Haystack:** file checkpoints exist, but restore executable workflows rather than approve independently performed work.[2,10]

## D. Unserved combination

**Inference, not an exclusivity claim:** none reviewed documents ratmac’s complete bundle: caller-owned work, checksum-fixed TOML runbook, disk-evidence gates, closed-word-plus-reason branches, repository-local growing history, and file-gated child-run joins.

Individual ingredients are not novel. **Positioning hypothesis:** an evidence gatekeeper for existing agents, rather than another agent runtime. Ratmac details here come from the supplied specification.

## E. What ratmac lacks

Most provide model/tool execution, automatic scheduling, richer in-memory data transfer, and tracing/streaming. Many add retries, timeouts and integrated human interaction. These trade simplicity for automation.[1,4,5,7,11,13]

**Scope:** added Agno and XState. All requested products were found. Excluded hosted-product pricing, exhaustive language parity, adoption counts and hands-on benchmarks.

## Sources

All sources below are official documentation/repositories, except **[18]**, which contains first-hand public issue reports.

1. **LangGraph:** [Repository/licence](https://github.com/langchain-ai/langgraph); [graph API](https://docs.langchain.com/oss/python/langgraph/graph-api); [checkpointers/history](https://docs.langchain.com/oss/python/langgraph/checkpointers); [interrupts](https://docs.langchain.com/oss/python/langgraph/interrupts); [fault tolerance](https://docs.langchain.com/oss/python/langgraph/fault-tolerance).

2. **Microsoft Agent Framework:** [Repository](https://github.com/microsoft/agent-framework); [workflow API](https://learn.microsoft.com/en-us/python/api/agent-framework-core/agent_framework.workflow?view=agent-framework-python-latest); [builder/limits](https://learn.microsoft.com/en-us/python/api/agent-framework-core/agent_framework.workflowbuilder?view=agent-framework-python-latest); [checkpoints](https://learn.microsoft.com/en-us/agent-framework/workflows/checkpoints); [checkpoint deletion](https://learn.microsoft.com/en-us/python/api/agent-framework-core/agent_framework.checkpointstorage?view=agent-framework-python-latest); [human input](https://learn.microsoft.com/en-us/agent-framework/workflows/human-in-the-loop); [error handling](https://learn.microsoft.com/en-us/agent-framework/agents/middleware/exception-handling).

3. **AutoGen:** [Repository/archive notice/licence](https://github.com/microsoft/autogen); [GraphFlow](https://microsoft.github.io/autogen/stable/user-guide/agentchat-user-guide/graph-flow.html); [teams API](https://microsoft.github.io/autogen/stable/reference/python/autogen_agentchat.teams.html); [state saving](https://microsoft.github.io/autogen/stable/user-guide/agentchat-user-guide/tutorial/state.html).

4. **CrewAI:** [Repository](https://github.com/crewAIInc/crewAI); [Flows](https://docs.crewai.com/en/concepts/flows); [human feedback](https://docs.crewai.com/en/learn/human-feedback-in-flows); [agent failure controls](https://docs.crewai.com/en/concepts/agents); [releases](https://github.com/crewAIInc/crewAI/releases).

5. **Google ADK:** [Repository/licence](https://github.com/google/adk-python); [ADK 2.0](https://adk.dev/2.0/); [graph routes](https://adk.dev/graphs/routes/); [dynamic workflows](https://adk.dev/graphs/dynamic/); [retries/timeouts](https://adk.dev/graphs/retry/); [sessions](https://adk.dev/sessions/session/); [Python API/deprecations](https://adk.dev/api-reference/python/google-adk.html).

6. **OpenAI Agents SDK:** [Official repository listing/licence](https://github.com/orgs/openai/repositories); [releases](https://github.com/openai/openai-agents-python/releases); [running agents](https://openai.github.io/openai-agents-python/running_agents/); [sessions](https://openai.github.io/openai-agents-python/sessions/); [approval and serialized state](https://openai.github.io/openai-agents-python/human_in_the_loop/).

7. **LlamaIndex Workflows:** [Repository/licence](https://github.com/run-llama/llama-agents); [2.x release](https://github.com/run-llama/llama-agents/releases/tag/llama-index-workflows%402.26.0); [workflow API](https://developers.llamaindex.ai/python/workflows-api-reference/workflow/); [context](https://developers.llamaindex.ai/python/workflows-api-reference/context/); [retry policies](https://developers.llamaindex.ai/python/llamaagents/workflows/retry_steps/); [server storage](https://developers.llamaindex.ai/python/workflows-api-reference/workflow_server/server/).

8. **Pydantic:** [Licence](https://github.com/pydantic/pydantic-ai/blob/main/LICENSE); [version policy](https://ai.pydantic.dev/version-policy); [graph builder/persistence removal](https://ai.pydantic.dev/graph/beta/); [nodes](https://ai.pydantic.dev/api/pydantic_graph/nodes/); [manual stepping](https://ai.pydantic.dev/api/pydantic_graph/graph/).

9. **Apache Burr:** [Repository/licence](https://github.com/apache/burr); [incubation status](https://incubator.apache.org/projects/burr.html); [application execution](https://burr.apache.org/docs/reference/application/); [persistence](https://burr.apache.org/docs/concepts/state-persistence/); [tracking file implementation](https://github.com/apache/burr/blob/main/burr/tracking/client.py); [human input](https://burr.apache.org/docs/concepts/human-in-the-loop/); [exception-management limitations](https://burr.apache.org/docs/concepts/planned-capabilities/).

10. **Haystack:** [Repository](https://github.com/deepset-ai/haystack); [pipeline API](https://docs.haystack.deepset.ai/reference/pipeline-api); [loops](https://docs.haystack.deepset.ai/docs/pipeline-loops); [breakpoints/snapshots](https://docs.haystack.deepset.ai/docs/pipeline-breakpoints); [human confirmation](https://docs.haystack.deepset.ai/docs/human-in-the-loop); [3.x migration](https://docs.haystack.deepset.ai/docs/migration).

11. **Mastra:** [Repository/licensing](https://github.com/mastra-ai/mastra); [control flow](https://mastra.ai/docs/workflows/control-flow); [workflow state](https://mastra.ai/docs/workflows/workflow-state); [snapshots](https://mastra.ai/docs/workflows/snapshots); [suspend/resume](https://mastra.ai/docs/workflows/suspend-and-resume); [errors/retries](https://mastra.ai/docs/workflows/error-handling).

12. **Semantic Kernel:** [Repository/successor notice](https://github.com/microsoft/semantic-kernel); [experimental Process Framework](https://learn.microsoft.com/en-us/semantic-kernel/frameworks/process/process-framework); [components](https://learn.microsoft.com/en-us/semantic-kernel/frameworks/process/process-core-components); [deployment/persistence](https://learn.microsoft.com/en-us/semantic-kernel/frameworks/process/process-deployment); [human-in-the-loop](https://learn.microsoft.com/en-us/semantic-kernel/frameworks/process/examples/example-human-in-loop); [best practices/errors](https://learn.microsoft.com/en-us/semantic-kernel/frameworks/process/process-best-practices).

13. **Strands:** [Current repository/licence](https://github.com/strands-agents/harness-sdk); [AWS production-release announcement](https://aws.amazon.com/blogs/opensource/introducing-strands-agents-1-0-production-ready-multi-agent-orchestration-made-simple/); [graph API](https://strandsagents.com/docs/api/python/strands.multiagent.graph/); [session storage/deletion](https://strandsagents.com/docs/user-guide/sdk/agents/session-management/); [multi-agent interrupts](https://strandsagents.com/docs/user-guide/sdk/interrupts-multi-agent/).

14. **smolagents:** [Repository](https://github.com/huggingface/smolagents); [agent API](https://huggingface.co/docs/smolagents/reference/agents); [guided tour](https://huggingface.co/docs/smolagents/guided_tour); [editable memory](https://huggingface.co/docs/smolagents/tutorials/memory); [human-review example](https://huggingface.co/docs/smolagents/en/examples/plan_customization).

15. **DSPy:** [Repository/licence](https://github.com/stanfordnlp/dspy); [current ReAct semantics](https://dspy.ai/current/diving-deeper/react/); [program saving versus history](https://dspy.ai/current/diving-deeper/saving-and-loading/); [local-model configuration](https://dspy.ai/current/getting-started/installation/).

16. **Agno:** [Repository/licence](https://github.com/agno-agi/agno); [workflow building blocks](https://docs.agno.com/workflows/building-workflows); [workflow state/resume API](https://docs.agno.com/reference/workflows/workflow); [loop limits](https://docs.agno.com/reference/workflows/loop-steps); [step retries](https://docs.agno.com/reference/workflows/step); [human-review configuration](https://docs.agno.com/workflows/hitl/human-review); [v3 migration](https://docs.agno.com/other/v3-migration).

17. **XState:** [Repository/licence](https://github.com/statelyai/xstate); [v6 beta announcement](https://stately.ai/blog/2026-11-05-xstate-v6-beta); [persistence](https://stately.ai/docs/persistence); [actors/completion/errors](https://stately.ai/docs/invoke); [guards](https://stately.ai/docs/states); [loop hazards](https://stately.ai/docs/eventless-transitions).

18. **Public user reports:** [LangGraph duplicate sibling execution #9106](https://github.com/langchain-ai/langgraph/issues/9106); [CrewAI prematurely terminated resumed loop #8927](https://github.com/crewAIInc/crewAI/issues/8927); [Mastra lost suspension data #27012](https://github.com/mastra-ai/mastra/issues/27012).

19. **Local execution and network caveats:** [MAF Ollama](https://learn.microsoft.com/en-us/agent-framework/agents/providers/ollama); [ADK model options](https://google.github.io/adk-docs/agents/models/); [Strands Ollama](https://strandsagents.com/docs/user-guide/sdk/model-providers/ollama/); [OpenAI custom endpoints](https://github.com/openai/openai-agents-python/blob/main/docs/config.md); [OpenAI default tracing/disable controls](https://github.com/openai/openai-agents-python/blob/main/docs/tracing.md); [CrewAI telemetry controls](https://docs.crewai.com/en/telemetry); [Agno telemetry controls](https://docs.agno.com/telemetry).
