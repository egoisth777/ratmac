# Orchestration Prior Art — Per-Subtask State Models

**Date:** 2026-07-28
**Subject:** ratmac — how existing loop and graph orchestration systems model per-subtask state
**Scope:** One question, asked of every serious system in the field: when work fans out, what abstraction does each subtask get, who owns its state, and what happens when two subtasks write the same thing?
**Framing:** Written against `graph-loop.md` in this folder, which argues Loop → Graph → Recursive Self-Improvement are layers rather than replacements, with the human as the irreplaceable value function.

---

## 1. Verdict

> **Almost nobody has an answer. The field's dominant per-subtask state model is a shared mutable object with no merge rule, and the concurrency semantics are "last writer wins, silently."**

Of the nineteen systems and layers surveyed, exactly **two** arbitrate concurrent writes to a shared key: LangGraph refuses unless you declared a combiner in the schema, and AG2 1.0 serializes every write through a write-ahead log with per-channel locks. Two more sidestep the problem by having no shared key namespace at all — Temporal and Inngest's core step model. Three are safe only by accident, because they happen to run sequentially today (CrewAI's task layer, AutoGen, Inngest's AgentKit). Five hand you one mutable object and hope: Google's Agent Development Kit, Mastra, CrewAI Flows, the OpenAI Agents SDK, and Swarm. Claude Code is its own case — subagents share no state object, only the filesystem, and that substrate is unguarded unless you opt into a git worktree per agent.

A second finding, on the essay's own framing: the loop-engineering tooling it cites as an early prototype of Recursive Self-Improvement has **no per-subtask state model, and in the popular repository no state object at all** — its state is a markdown file the agent rewrites, and its "Loop Ready Score" is a deterministic formula summing regular-expression matches over prose (§3.3).

### 1.1 Three strongest pieces of evidence

**1. LangGraph is the only graph framework whose default is refusal.**
A state key with no declared reducer is backed by a `LastValue` channel. If two nodes in the same super-step write it, the channel raises `InvalidUpdateError` carrying error code `INVALID_CONCURRENT_GRAPH_UPDATE`, with the message *"At key '{key}': Can receive only one value per step. Use an Annotated key to handle multiple values."* — [`channels/last_value.py`](https://github.com/langchain-ai/langgraph/blob/main/libs/langgraph/langgraph/channels/last_value.py). Concurrency is not a runtime hazard you discover in production; it is a schema error you hit on the first parallel run. This is the single most transferable idea in the survey, and it is exactly ratmac's "refusal over guessing" thesis expressed as a data structure.

**2. Two widely-used frameworks advertise isolation of one thing while silently sharing another.**
Google's Agent Development Kit documents that parallel branches get "no automatic sharing" — but the source shows `_create_branch_ctx_for_sub_agent` performing a **shallow** Pydantic `model_copy()`, so every branch holds the same `Session` and therefore the same `State` object; `_update_session_state` merges deltas with a plain `session.state.update({key: value})`. What the branch actually isolates is *conversation-history filtering*, not state. Mastra is the same shape: per-step outputs are typed and isolated by step id, while `executionContext.state` is passed to every parallel branch **by reference** and merged with `Object.assign` in completion order. In both cases the documentation discusses isolation of the wrong object. This is the most dangerous pattern in the field, and the one ratmac must never imitate.

**3. The protocol layer has a better ownership model than any framework.**
The Model Context Protocol (MCP) Tasks extension gives a long-running subtask a durable server-owned handle: a `taskId`, a closed status set (`working`, `input_required`, `completed`, `failed`, `cancelled`), three terminal states that never change once reached, a time-to-live, and a suggested poll interval. The **client never writes the status** — it polls `tasks/get` and answers `inputRequests` via `tasks/update`. That is ratmac's one-writer invariant, arrived at independently, at the wire protocol. [modelcontextprotocol.io/extensions/tasks/overview](https://modelcontextprotocol.io/extensions/tasks/overview.md)

---

## 2. Comparison table — per-subtask state models

Read "abstraction" as: what a single fanned-out unit of work actually receives.

| System | Abstraction a subtask gets | Own state / slice / blackboard | Fan-out | Fan-in | Two children write the same key |
|---|---|---|---|---|---|
| **LangGraph** 1.2.10 | A node invocation, given either graph state or a bespoke `Send` payload | **Scoped read, shared write** — reads a payload, writes into shared typed channels | `Send(node, arg)` from a conditional edge; parallel edges | Reducer on the collecting channel (`Annotated[list, operator.add]`) | **Refuses** — `InvalidUpdateError` / `INVALID_CONCURRENT_GRAPH_UPDATE` unless a reducer is declared |
| **LangGraph functional API** | A `@task` future inside an `@entrypoint` | **Own state** — scoped to the function, no shared declaration | Call `@task`, collect futures | `f.result()` | No shared key namespace |
| **Claude Code subagents** | A fresh context window plus one prompt string | **Own state**, text-only return | `Agent` tool; `pipeline()` in dynamic workflows | Final message only | Nothing — the filesystem is the only shared substrate; isolation (worktrees) is opt-in |
| **Claude Code dynamic workflows** | A subagent whose result lands in a script variable | **Own state**; parent state is JavaScript variables | `agent()`, `pipeline()`, ≤16 concurrent | Script variables | Not applicable to variables; file collisions unguarded |
| **Google ADK** 2.5.0 | A sub-agent with a `branch` label and an `output_key` | **Shared blackboard** — shallow copy, same `Session.state` | `ParallelAgent` (now deprecated for Workflow) | Distinct `output_key` per child, merger agent downstream | **Silent last-write-wins**; docs tell you to add your own locks |
| **Mastra** 1.54.0 | A step with typed input/output schemas | **Both** — typed values isolated; `state` shared by reference | `.parallel()`, `.foreach({concurrency})`, `.branch()` | Object keyed by step id; array in input order | **Silent last-completer-wins** via `Object.assign` |
| **CrewAI Task** 1.15.8 | A `Task` producing its own `TaskOutput` | **Own state**; context arrives as a rendered string | `async_execution=True`, one daemon thread each | `CrewOutput` holding an ordered `list[TaskOutput]` | Cannot arise — concurrent tasks cannot see each other |
| **CrewAI Flow** | A `@listen` method on the flow object | **Shared mutable blackboard** (`self.state`) | `asyncio.gather` over listeners | `and_()` / `or_()` | **Races** — no lock guards `self.state` |
| **AutoGen** 0.7.5 (maintenance) | A turn in a shared append-only message thread | **Shared message list**; each agent owns its model context | `GraphFlow` returns a ready-set | Per-edge `activation_condition: "all"` / `"any"` | No key namespace; message delivery serialized by a hand-rolled `FIFOLock` |
| **AG2** 1.0 | A channel participant emitting envelopes | **Log-derived** — `WorkflowState.context_vars` rebuilt by replaying a write-ahead log | Channel envelopes | Log replay (`Hub.hydrate()`) | **Serialized** by a per-channel `asyncio.Lock` |
| **OpenAI Agents SDK** 0.19.0 | An `Agent` run sharing the parent's `RunContextWrapper` | **Shared blackboard** — `_fork_with_tool_input` copies references | No agent-level primitive; docs say use `asyncio.gather` | Agent-as-tool returns a **string**; tool results reassembled by an `order` index | **Races** — no lock in `run_context.py` or `tool_execution.py` |
| **Swarm** (dead) | A turn with a deep-copied `context_variables` dict | Shallow-merged dict, injected by reference into tools | None — sequential `for` loop | `context_variables.update(...)` | Last-write-wins |
| **Temporal** 1.30.0 | A child workflow or an activity, taking arguments | **Own state** — ordinary program variables made durable by replay | `asyncio.gather`, `start_child_workflow()` | Return values | **Impossible by construction**; torn reads across `await` need an `asyncio.Lock` |
| **Inngest core** 4.13.0 | A `step.run` in a separate execution | **Own state**, memoized result | Un-awaited handles + `Promise.all` | Platform aggregates step state and re-invokes | **Impossible** — separate executions, no shared memory |
| **Inngest AgentKit** 0.13.2 | An agent in a network sharing one `State` | **Shared blackboard** (`state.data` proxy, `state.kv` map) | None today — the loop is strictly sequential | `state._results` append | Does not arise yet; unguarded the moment parallelism lands |
| **loop-engineering** tools | Nothing — no subtask concept | **No state object**; `STATE.md` is freeform markdown the agent rewrites | None | None | Not applicable |
| **outerloop** | A run, not a subtask | Own state — evidence package keyed by run and loop id, append-only ledger | None — chain hardcoded to three steps | None | Not applicable |
| **Statewright** (nearest neighbour) | A named fork branch with its own current state and tool allowlist | Branch owns its position in the state machine; `context` is shared | `fork.branches` in the runbook JSON | `join: "all"` (AND gate) or `"any"` (race) | Not addressed — `context` merges from agent-supplied `data` |
| **MCP Tasks extension** | A durable server-owned handle | **Own state**, server-written, client-read | Not specified — one task per request | Poll `tasks/get` until terminal | Not applicable — one writer by protocol |

---

## 3. The loop layer

### 3.1 Anthropic's four loop types — real, dated, and silent on state

The post the essay refers to is **"Loop engineering: getting started with loops"**, [claude.com/blog/getting-started-with-loops](https://claude.com/blog/getting-started-with-loops), published **2026-06-30** by Delba de Oliveira and Michael Segner. It is on the product blog, not `anthropic.com/engineering` — the engineering index carries no loop taxonomy. The four types and their stated triggers and stop conditions:

| Type | Trigger | Stop condition |
|---|---|---|
| Turn-based | "A user prompt." | "Claude judges it has completed the task or needs additional context." |
| Goal-based (`/goal`) | "A manual prompt in real-time." | "Goal achieved OR maximum number of turns reached." |
| Time-based (`/loop`, `/schedule`) | "A specified time interval." | "You cancel it, or the work completes (the PR merges, the queue is empty)." |
| Proactive | "An event or schedule, with no human in real time." | "Each task exits when its goal is met. The routine itself runs until you turn it off." |

The organizing axis is *what you delegate*: the check, the stop condition, the trigger, the prompt. Goal-based loops use "an evaluator model [that] checks your condition and sends it back to work until the goal is met."

**On state between iterations, the post says nothing.** No context windows, no transcripts, no compaction, no files. The one adjacent sentence is about locality — "`/loop` runs on your computer, so if you turn it off, it stops." The closing advice is the line the essay quotes as proto-Recursive-Self-Improvement: *"When an individual result doesn't meet the standard, don't stop at fixing the individual issue, try to encode it to improve the system for all future iterations."*

The actual state model lives elsewhere in Anthropic's documentation:

- **Within a session**, the context window *is* the state. [How the agent loop works](https://code.claude.com/docs/en/agent-sdk/agent-loop.md): "It does not reset between turns within a session. Everything accumulates." At the limit, automatic compaction "summarizes older history to free space," emitting a `compact_boundary` event — and "specific instructions from early in the conversation may not be preserved. Persistent rules belong in CLAUDE.md… because CLAUDE.md content is re-injected on every request."
- **Across sessions**, `session_id` plus `resume`/`continue`/`fork` ([sessions](https://code.claude.com/docs/en/agent-sdk/sessions.md)), with a `session_store` adapter for stateless hosts.
- **Note-taking as escape hatch.** [Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) (2025-09-29): "the agent regularly writes notes persisted to memory outside of the context window," letting agents "maintain project state across sessions."

### 3.2 Claude Code subagents — isolated context, one string back

This is the cleanest per-subtask model in the field, and also the poorest. From [Subagents in the SDK](https://code.claude.com/docs/en/agent-sdk/subagents.md):

> "A subagent's context window starts fresh, with no parent conversation, but isn't empty. **The only content you pass from parent to subagent is the Agent tool's prompt string**, so include any file paths, error messages, or decisions the subagent needs directly in that prompt."

Return: "Intermediate tool calls and results stay inside the subagent; **only its final message returns to the parent**." Sibling communication is essentially absent — per [agent teams](https://code.claude.com/docs/en/agent-teams.md), subagents "only report back to the main agent" and "never talk to each other." The one exception is the `SendMessage` tool (v2.1.206+), which gives a holder a list of other named agents in the session.

**Two subagents editing the same file: the documentation never states what the runtime does.** It documents prevention instead. [Run agents in parallel](https://code.claude.com/docs/en/agents.md): "Do the tasks touch the same files? Isolate the work with worktrees." A subagent can declare `isolation: worktree` and get a temporary git worktree "so parallel edits don't conflict," locked while it runs. Without that, subagents share the working directory — same filesystem, no lock, no merge. For agent teams the guidance is blunt: "Two teammates editing the same file leads to overwrites. Break the work so each teammate owns a different set of files."

**Dynamic workflows** move intermediate state out of the context window entirely. From [workflows](https://code.claude.com/docs/en/workflows.md), the comparison table's "Where intermediate results live" row reads: subagents → "Claude's context window"; agent teams → "A shared task list"; workflows → **"Script variables."** The script "holds the loop, the branching, and the intermediate results itself, so Claude's context holds only the final answer." Limits: 16 concurrent agents, 1,000 per run, resume only within the same session, and resume is order-dependent — cached results "stop at the first agent that didn't finish, and every agent that started after that one runs again, even if it completed."

The most honest primary evidence on N-way conflict is Anthropic's own [Building a C compiler with a team of parallel Claudes](https://www.anthropic.com/engineering/building-c-compiler) (2026-02-05, Nicholas Carlini): no orchestrator agent, coordination via lock files in a `current_tasks/` directory, git as the only shared store, and the failure mode named outright — on an indivisible task, "Every agent would hit the same bug, fix that bug, and then overwrite each other's changes," making 16 agents no better than one.

**Bottom line for the loop layer:** isolated contexts, a single text return value, and the filesystem as the only genuinely shared substrate — whose isolation is opt-in rather than default.

### 3.3 The "loop engineering" tooling — one real tool, two architecture diagrams that compile

The essay credits Cobus Greyling's tooling with being "an early engineering prototype of Recursive Self-Improvement." All three repositories exist. Their substance is very unevenly distributed.

| Project | Stars | Created | Commits | What it is |
|---|---|---|---|---|
| [`cobusgreyling/loop-engineering`](https://github.com/cobusgreyling/loop-engineering) | **9,558** (1,307 forks) | 2026-06-09 | 328 | Real, used software wrapped around a prose-shaped metric |
| [`cobusgreyling/harness-foundry`](https://github.com/cobusgreyling/harness-foundry) | 3 | 2026-07-15 | **14** | A genuine skeleton, oversold as a "runtime" |
| [`cobusgreyling/outerloop`](https://github.com/cobusgreyling/outerloop) | 6 | 2026-07-09 | 37 | The best schemas of the three, authored in 39 minutes, essentially no users |

**loop-engineering is real.** Nearly ten thousand stars, 1,307 forks, 27 contributors, MIT licensed, 25 open issues, last pushed 2026-07-29. Fourteen tool packages under `tools/`, each with source, tests, and a package manifest: `loop-audit/src/auditor.ts` (32.6 KB), `loop-init/src/cli.ts` (34.0 KB), `loop-cost/src/estimator.ts` (10.8 KB), `loop-sync/src/sync.ts` (10.8 KB), plus `loop`, `loop-gate`, `loop-context`, `loop-worktree`, and a Model Context Protocol server. `loop-cost` in particular is a genuine model — cadence parsed into runs per day, prompt-caching fractions, and orchestration multipliers (maker-checker 2×, `parallel:N` → N+1, `debate:R` → 1+R). The packages publish under the `@cobusgreyling/` scope, not bare names, with modest but real weekly download counts (loop-cost ≈ 1,493, loop-audit ≈ 1,477).

**The Loop Ready Score is a real formula over unreal inputs, and that distinction is the finding.** The arithmetic is deterministic and auditable: a `SCORE_WEIGHTS` constant (`base: 7, stateFile: 18, triage: 14, verifier: 14, skillsTwoPlus: 14, …`), summed in `computeScore()`, clamped to 0–100, bucketed by `LEVEL_THRESHOLDS = { L1: 38, L2: 58, L3: 78 }`. But the signals it sums are file-existence checks and regular expressions run over concatenated markdown:

```ts
const escalation = ESCALATION_HINTS.some((re) => re.test(governanceCorpus));
const stallDetection = skillNames.includes('loop-context') || … STALL_HINTS.some((re) => re.test(governanceCorpus));
```

It scores whether you have a `STATE.md`, a `loop-budget.md`, a `gate.yaml`, and directories named from a hardcoded skill list. It never checks whether a loop ran, or worked. It is a documentation-completeness checker with a number attached — and predictably, the repository scores itself 100, level L3, in its own `STATE.md`.

This is precisely the distinction ratmac's schema already draws between **guidance-consistency evidence** (a check over document wording) and **behavioral evidence** (a recorded scenario of what was actually invoked). The most-starred artifact in loop engineering is a guidance-consistency checker presenting itself as a maturity measurement. Worth citing when positioning ratmac: the field's popular metric measures the presence of prose.

**Per-subtask state, across all three: absent.**

- **loop-engineering has no state object at all.** State is `STATE.md` — freeform markdown with `## High Priority` / `## Watch List` / `## Recent Noise` headings and a `Last run:` timestamp, rewritten by the agent. No schema, no store, no iteration record. `loop-run-log.md` is an append-only markdown log. The agent is both the writer and the reader of its own progress record, which is the exact failure mode ratmac exists to remove.
- **outerloop has real schemas at per-*run* granularity, and no subtask concept.** `packages/core/src/schemas.ts` defines an evidence package keyed by run and loop identifiers, with plan / implementation / verification / observability sections and a 0–10 risk score; a verdict schema over `ship|block|redirect|narrow|guardrail|reject` with `rationale: z.string().min(1, "Rationale is mandatory")`; and a ledger entry schema. Persistence is genuine — append-only JSONL plus a rebuilt index, with fixtures on disk. But `buildLedgerEntry` hardcodes the chain to exactly three steps (`evidence`, `risk-assessment`, `verdict`). There is no decomposition, no subtask identity, no iteration counter. The entire framework — 14 packages, 20 test files — landed on 2026-07-09 in **39 minutes** of commits, and tests are smoke assertions against nonexistent paths (`runOuterloopAudit("/tmp/nonexistent-…")`, asserting only that the grade is one of A–F).
- **harness-foundry is the closest to per-iteration state and the thinnest.** A trace-event schema with 13 event types including turn boundaries, and a session manifest with a turn count and a `running|completed|failed|recovered` status; the live `SessionRuntime` is a seven-field in-memory type. What it "versions" is YAML primitives composed into named stacks with a lockfile of truncated SHA-256 digests — but all 11 primitives are 119–197-byte stubs, and `primitiveDigest` assigns the payload's `version` field from `definition?.layer ?? "unknown"`, so the versioned runtime hashes a layer name into the version slot. Authored 2026-07-15 between 13:26 and 13:51.

**Disambiguation.** "outerloop" is a crowded name — 22 GitHub matches. The governance project is `cobusgreyling/outerloop`. Unrelated: `outergroup/outerloop` (Bayesian optimization, Python, 2022), `outerloop/outerloop.github.io` (a 2014 blog), `loopgain-ai/outerloop-bench`, and Red Hat devfile fixtures using "outer loop" in the Kubernetes deployment-cycle sense, which is a generic term rather than a project.

**Vapor flag.** loop-engineering clears the bar into real, used software. harness-foundry and outerloop are companion artifacts to Cobus Greyling's writing — architecture diagrams rendered as compiling TypeScript. Do not cite either as evidence that Recursive Self-Improvement has been engineered; cite them as evidence of what its schema would need to contain.

---

## 4. The graph layer

### 4.1 LangGraph — the reference implementation of state ownership

**Version and date:** `langgraph==1.2.10`, published **2026-07-28** (today). Release cadence is roughly weekly; 1.2.6 (2026-06-18) fixed a subgraph regression where a "nested subgraph inherits parent checkpoint_ns."

**Execution model.** Bulk-synchronous parallel, credited to Google's Pregel. "A super-step is a single iteration over the graph nodes." "Nodes that run in parallel are part of the same super-step, while nodes that run sequentially belong to separate super-steps." A node is `inactive` until it "receives a new message (state) on any of its incoming edges (or 'channels')"; execution ends "when all nodes are inactive and no messages are in transit." Super-steps are transactional: if any parallel branch raises, none of that super-step's updates land.

**State is typed channels with per-key merge rules.** Every reducer is "a binary function with two positional arguments" — current stored value on the left, node update on the right — applied as `new_value = reducer(left=current_state[key], right=node_update[key])`. Without one, "The default reducer ignores the left argument and replaces the state value with the right argument."

**And without one, parallel writes refuse.** This is the load-bearing fact. The `LastValue` channel's `update()` raises `InvalidUpdateError` when it receives more than one value in a step, carrying `ErrorCode.INVALID_CONCURRENT_GRAPH_UPDATE` and the message *"At key '{self.key}': Can receive only one value per step. Use an Annotated key to handle multiple values."* Declaring `Annotated[list[str], operator.add]` converts refusal into accumulation. `Overwrite` bypasses a reducer to write directly, and the same discipline applies — "only one node can use `Overwrite` on the same state key in a given super-step," or it is `InvalidUpdateError` again.

**Fan-out gives a subtask a bespoke payload, not a state slice.** `Send(node, arg)` is documented as taking "the name of the target node, and second is the state to pass to that node," and the class docstring says, of the payload, that "the sent state can differ from the core graph's state." The map-reduce example proves it: the worker reads `state['subject']` — singular, from the `Send` — never the graph-level `subjects` list. So the read view is isolated and per-worker; the **write** view is the shared channel set, arbitrated by reducers. That asymmetry is the whole design.

**Fan-in ordering is explicitly non-deterministic.** From the graph API guide: "updates from a parallel superstep may not be ordered consistently." The documented workaround is to "write the outputs to a separate field in the state together with a value with which to order them." For branches of unequal length, `builder.add_node(d, defer=True)` delays a fan-in node "until all other pending tasks are completed."

**Subgraphs.** Two patterns. Shared keys: pass the compiled subgraph straight to `add_node`, "no wrapper function needed." Different schema: write a wrapper that maps parent state in and subgraph output back out. Subgraph-private keys never reach the parent because the parent never declared the channel — isolation by schema, not by inheritance; in the nested example, "child or parent keys will not be accessible here." Writing a shared key from a subgraph via `Command.PARENT` **requires** a reducer on that key in the parent.

Subgraph persistence has three modes on `.compile(checkpointer=...)`: `None` (per-invocation — "Each call starts fresh and inherits the parent's checkpointer," each invocation getting its own checkpoint namespace), `True` (per-thread — "State accumulates across calls on the same thread"), `False` (stateless). Notably, per-thread subgraphs "do not support parallel tool calls," since "both calls write to the same namespace." Namespaces are `""` for the root, `"node_name:uuid"` for a subgraph, and nested namespaces are joined with a pipe.

**Durability is a dial, and partial failure is handled.** `durability="exit" | "async" | "sync"`, least to most durable; `async` is the default. When one node in a parallel super-step fails, LangGraph "stores pending checkpoint writes from any other nodes that completed successfully at that super-step" as task entries — so on resume "you don't re-run the successful nodes." Those per-task writes "are not full `StateSnapshot` checkpoints," so time travel still only resumes from super-step boundaries.

**A `StateSnapshot`** carries `values`, `next`, `config` (`thread_id`, `checkpoint_ns`, `checkpoint_id`), `metadata` (`source`, `writes`, `step`), `created_at`, `parent_config`, and `tasks` — each task with `id`, `name`, `error`, `interrupts`, and optionally a nested subgraph snapshot.

**Runtime context is separated from state.** A `context_schema` carries per-run configuration (model name, connection handles) reachable as `runtime.context`, deliberately kept out of the mutable state so it "does not clutter the graph's internal state." Immutable per-run input and mutable per-run state are different objects — the same split ratmac draws between the Machine Class and run state.

**The functional API is a different bargain.** `@task` and `@entrypoint` "manage state scoped to the function, without requiring explicit shared state management." Checkpointing differs: "the Graph API creates a new checkpoint after every superstep, while the Functional API saves task results to an existing checkpoint." On replay, LangGraph "restores results from completed tasks and subgraphs directly from the checkpointer, rather than recomputing them." You trade the declarative graph for ordinary control flow and lose the channel discipline — which is to say, you lose the refusal.

Docs: [graph API](https://docs.langchain.com/oss/python/langgraph/graph-api) · [use subgraphs](https://docs.langchain.com/oss/python/langgraph/use-subgraphs) · [checkpointers](https://docs.langchain.com/oss/python/langgraph/checkpointers) · [functional API](https://docs.langchain.com/oss/python/langgraph/functional-api)

### 4.2 Google Agent Development Kit — a blackboard advertised as isolation

**Version:** `google-adk` 2.5.0 (2026-07-16). Docs have moved to adk.dev.

`State` (`src/google/adk/sessions/state.py`) is dict-like over two dicts — committed `_value` and pending `_delta`, described in its own docstring as "A state dict that maintains the current value and the pending-commit delta." Prefixes `app:`, `user:`, `temp:` select **storage scope**, not per-subtask isolation: `temp:` is scoped to the current invocation, and sub-agents inherit the parent's invocation id, so `temp:` is shared all the way down.

`ParallelAgent` builds one generator per sub-agent and merges them through a queue with per-event resume signalling (an `asyncio.TaskGroup` on Python 3.11+). Each branch context comes from `_create_branch_ctx_for_sub_agent`, which does `invocation_context.model_copy()` — **shallow**. Every branch therefore holds the same `Session`, and the same `State`. What differs is `ctx.branch`.

The branch does real work, but on the wrong object for our question. The docstring of `contents.py::_is_event_belongs_to_branch` says it plainly: "This is for event context segregation between agents. E.g. agent A shouldn't see output of agent B." That is history filtering. State moves the other way, through `base_session_service._update_session_state`:

```python
for key, value in event.actions.state_delta.items():
  session.state.update({key: value})
```

Plain overwrite. No reducer, no compare-and-swap, no conflict detection. The documentation is careful about history and honest about ordering — "The order of results may not be deterministic" — and pushes the problem back to you: "you'd need to manage concurrent access to this shared context carefully (e.g., using locks) to avoid race conditions." There is **no warning specific to two sub-agents writing the same key**. The saving grace is accidental: the merge queue yields one event at a time, so `append_event` is serialized — you get interleaving, not a corrupted dict.

The sanctioned discipline is worth noting because ratmac already holds it: state "should **always** be updated as part of adding an `Event` to the session history using `session_service.append_event()`." Mutating a fetched `session.state` directly is called "problematic" — it "Bypasses Event History," "Breaks Persistence," and is "Not Thread-Safe." That is an append-only log with a derived view, described correctly and then not enforced.

`ParallelAgent` now carries `@deprecated` — "deprecated in favor of Workflow and will be removed in a future version."

### 4.3 Mastra — typed value-passing plus an unguarded side channel

**Version:** `@mastra/core` 1.54.0 (2026-07-28).

The primary channel is good: each step declares `inputSchema` and `outputSchema` (Zod, Valibot, or ArkType) and receives `inputData`. Fan-in is typed and explicit — "the output is an object where each key is the step's `id` and the value is that step's output." `.foreach` preserves input order. Composition verbs are `.parallel`, `.branch`, `.foreach({concurrency})`, `.dowhile`, `.dountil`. Streaming fan-in is ruled out: "Results can't be 'streamed' to the next step as they complete."

The secondary channel is the problem. `stateSchema` plus `{ state, setState }` "lets you share values across steps without passing them through every step's inputSchema and outputSchema." In `handlers/control-flow.ts`, `executeParallel` passes the **same reference** (`state: executionContext.state`) to every branch, and as each branch resolves, `applyMutableContext` runs `Object.assign(executionContext.state, mutableContext.state)`. Shallow assign, in completion order. `handlers/step.ts` buffers `setState` into a pending mutation rather than mutating live, which avoids mid-flight tearing but does nothing about the final merge. No reducer, no refusal, and the control-flow documentation never mentions shared state under `.parallel()` at all.

Suspend/resume is genuinely good: a suspended workflow's "current execution state is saved as a snapshot" to the configured storage provider, surviving restarts, and a single step can be resumed independently — `run.resume({ step: 'step-1', resumeData })`, with `suspendedStep.path` for nesting and `forEachIndex` for one `.foreach` iteration.

### 4.4 CrewAI — two disjoint models, one safe by accident

**Version:** 1.15.8 (2026-07-28). Source has moved to `lib/crewai/src/crewai/`.

**Crew/Task** has no shared mutable state. Each `Task` produces its own `TaskOutput`; upstream context arrives as a **rendered string** via `_get_context`. Fan-in is a `CrewOutput` holding an ordered `list[TaskOutput]`. `async_execution=True` spawns a real daemon thread per task — unbounded, not a pool — and the drain barrier resolves futures in **submission order**, not completion order. Concurrent tasks receive only `[last_sync_output]` as context, so they cannot see each other. Safety by avoidance, not design.

**Flow** is the modern model and it races. A single shared mutable `self.state` — dict or Pydantic model — is read and written by every `@start` and `@listen` method, and listeners run concurrently under `asyncio.gather`. Of the two locks in the 3,771-line flow runtime, `_or_listeners_lock` and `_usage_metrics_lock`, **neither guards `self.state`**. No reducer, no channel, no lock: last-write-wins, and any read-modify-write spanning an `await` loses updates. The [Flows documentation](https://docs.crewai.com/en/concepts/flows) says starts "often run in parallel" and then says nothing about write safety.

That CrewAI knows this is documented in its own tracker. [Issue #6125](https://github.com/crewAIInc/crewAI/issues/6125), "Critical: State loss in JsonProvider due to non-atomic checkpointing" (2026-06-11, closed), reports that *"5 concurrent agents performing 50 updates each resulted in a final count of 1 instead of 250."* Related: [#4822](https://github.com/crewAIInc/crewAI/issues/4822) (contextvar loss under `async_execution`, fixed by `copy_context()`) and [#5141](https://github.com/crewAIInc/crewAI/issues/5141) (shared model stop-words mutation polluting agents).

### 4.5 AutoGen and AG2 — the field's one deliberate answer

**AutoGen is in maintenance mode.** The README carries a `[!CAUTION]`: "AutoGen is now in maintenance mode. It will not receive new features or enhancements and is community managed going forward. New users should start with Microsoft Agent Framework." Last Python release `python-v0.7.5`, 2025-09-30.

Its state model is a shared append-only message list, with `TeamState` nesting per-agent containers plus the manager's thread, current turn, and next speaker index. There is no key namespace to race on, and message delivery is serialized by `SequentialRoutedAgent`'s hand-rolled `FIFOLock` — "A lock that ensures coroutines acquire the lock in the order they request it." Correct, and honest, but it is safety through the absence of shared keys. `GraphFlow` is the real fan-out: `select_speaker()` returns a whole ready-set, with per-edge `activation_condition: "all"` (join barrier) or `"any"` (first wins). `save_state()`/`load_state()` persist conversation history only — model clients, tools, system messages and termination conditions must be rebuilt.

**AG2 1.0, released 2026-07-27, is the only framework in this survey that treats concurrent state as a design problem.** It abandoned the old shared `ContextVariables` dict for an `ag2.network` architecture: context lives at `WorkflowState.context_vars` and is mutated only by emitting an envelope onto a write-ahead log. The documentation claims it outright — *"The per-channel write-ahead-log lock serialises concurrent writes — there's no race even with multiple tools racing on the same key."* — and the source backs it: `ag2/network/hub/core.py` holds `self._channel_locks: dict[str, asyncio.Lock]` and takes `async with self._wal_lock(envelope.channel_id)` before applying. State is a **derivation of the log**, rebuilt by `Hub.hydrate()` replaying envelopes, with delete ordered before set inside one envelope so overwrite is atomic.

This is one day old at the time of writing. Treat the claim as credible but unweathered.

### 4.6 OpenAI Agents SDK and Swarm — a shared object with no lock

**Swarm is dead** — OpenAI describes it as "educational," superseded by the Agents SDK; last push 2026-04-15. Its semantics are worth recording because they are the naive baseline: `context_variables` deep-copied at `run()` entry, then merged per turn with `context_variables.update(...)` — shallow, last-writer-wins. Tool execution is a plain sequential `for` loop; `parallel_tool_calls` only lets the *model* emit several calls. The dict is injected into tools **by reference**, so a tool can mutate the caller's live state outside the sanctioned return path.

**Agents SDK 0.19.0 (2026-07-27)** improved the typing and not the ownership. `RunContextWrapper[TContext]` holds your arbitrary object at `.context` plus usage, turn input, and approvals; it is never sent to the model. Sub-agent runs via `Agent.as_tool()` **share the same object** — `_fork_with_tool_input` copies references, not values.

There is no agent-level fan-out primitive. The multi-agent documentation's entire parallelism content is: "Running multiple agents in parallel, e.g. via Python primitives like `asyncio.gather`." Tool calls, however, are genuinely concurrent — a batch executor spawns an `asyncio` task per call, draining with `FIRST_COMPLETED`, bounded by `max_function_tool_concurrency` whose **default is `None`, meaning unbounded**. Fan-in is deterministic, since each task carries an `order` index.

**And it races.** N tools run as concurrent tasks against one shared `.context`, with no lock, no reducer, and no channel in `run_context.py` or `tool_execution.py`. The docs note derived wrappers "share the same underlying app context" and that nested runs "do not get an isolated copy of your app state by default" — then say nothing about thread safety. Under `asyncio` a single assignment will not tear, but `ctx.counter += 1` across an `await` loses updates. Confirmed races elsewhere in the SDK: [#3817](https://github.com/openai/openai-agents-python/issues/3817) (time-of-check-to-time-of-use in `AdvancedSQLiteSession`, fixed 2026-07-12) and [#1956](https://github.com/openai/openai-agents-python/pull/1956).

Sessions persist **conversation items only** — your `context` object is not persisted. Only the MongoDB backend documents an ordering guarantee under concurrent writers (a monotonic sequence counter); the others promise nothing.

---

## 5. The durable-execution layer

### 5.1 Temporal — ownership as a language property

**Version:** `temporalio` (Python) 1.30.0, 2026-07-02.

Temporal has **no state container at all**, and that is the point. State is ordinary instance variables inside a workflow class, made durable by deterministic replay of the event history. A shared dictionary is a data structure somebody must reconcile; Temporal state is a program's memory, reconstructed by re-running the same code over the same recorded events. Ownership is lexical scope. Two subtasks cannot write the same key because there is no key namespace — only variables one coroutine owns.

Fan-out is `asyncio.gather` over activity handles, or `start_child_workflow()` returning a handle (`execute_child_workflow()` being "a helper function for `start_child_workflow()` plus `await handle`"). Children share no memory with the parent — only lifecycle coupling through `ParentClosePolicy`. Results come back as typed return values, never merged.

The single-threaded deterministic event loop makes scheduling races structurally impossible, but **torn state across `await` points is real and documented**: "handler executions and your main Workflow method are all running concurrently, with switching occurring between them at `await` calls," and the canonical bad example warns "there may be times when the Workflow has self.x from one Activity execution and self.y from another." The prescribed fix is an `asyncio.Lock`. Query handlers are read-only by rule — "A Query handler returns a value: it can inspect but must not mutate the Workflow state" — enforced by requiring a synchronous `def`.

The cost is severe and honestly documented. No wall-clock branching, no random numbers, no inline I/O — all of that must live in activities, which "execute outside the replay path." The Python sandbox proxies modules to "prevent known non-deterministic library calls" and admits it "is not completely isolated." Changing code in a running workflow requires worker versioning ("the **recommended** way") or patching; adding, reordering, or removing command-producing calls without versioning breaks replay. Durability in exchange is total: the event history is "a complete and durable log of everything that has happened," and on crash "the Worker uses the Event History to replay the code and recreate the state."

`temporalio.contrib.openai_agents` ships in the SDK, turning model calls into activities and the agent loop into a workflow.

### 5.2 Inngest and AgentKit — safe core, unguarded agent layer

**Inngest core** (4.13.0, 2026-07-15) has no state object. `step.run` results are memoized in run state and replayed into a re-invoked function body. Fan-out is un-awaited handles collected with `Promise.all` — "This triggers all steps to run in parallel via separate executions" — and fan-in is platform-side: "When each step is finished, Inngest will aggregate each step's state and re-invoke the function with all state available." Concurrent same-key writes **cannot happen**, because parallel steps run in separate executions with no shared memory; the documentation is explicit that this is "true parallelism similar to multi-threading (without shared state)." Data moves through return values. Constraints: all step data under 4 MB, at most 1,000 steps, and racing steps are not cancelled — "The losing steps are *not* cancelled and continue to run."

**AgentKit** (0.13.2, 2025-11-13) reintroduces a blackboard. `State<T>` holds `state.data` (a plain object behind a `Proxy` whose setter trap currently just forwards to `Reflect.set` — a hook stub with no merge logic), a deprecated `state.kv` map, plus `_results` and `_messages`. Today the question does not arise, because the network loop is strictly sequential — and the source says why, at `network.ts:601`:

```ts
// XXX: It would be possible to parallel call these agents here by
// fetching the entire stack, parallel running, then awaiting the
// responses.   However, this confuses history and we'll take our time to
// introduce parallelisation after the foundations are set.
```

Safe by serialization, not by design. `state.data` has no reducer and no conflict rule, and will race the moment that comment is resolved. AgentKit state is also ephemeral — "only retained for a single `Network`'s run" and "not persisted across different Network `run()` calls."

---

## 6. The gate layer — ratmac's own neighbourhood

### 6.1 Statewright — a declarative fork/join in a runbook

The closest competitor by architecture (Rust, 418 stars, created 2026-05-03, last pushed 2026-07-23, 1 open issue, license NOASSERTION — see `01-landscape-and-competitors.md` in the feasibility folder) has the only **declarative** fan-out in this survey — expressed as data in the workflow file rather than as code.

From the [schema reference](https://docs.statewright.ai/workflows/schema-reference/): "Fork transitions spawn multiple branches, each with its own state and tool restrictions."

| Field | Required | Meaning |
|---|---|---|
| `fork.branches` | yes | "Named branches, each with `initial` and `terminal` state names" |
| `fork.join` | no | "Join strategy: `all` (default, AND gate) or `any` (race)" |
| `fork.on_complete` | yes | "State to transition to when join condition is met" |
| `fork.on_fail` | no | "State to transition to if any branch fails" |

```json
"BUILD_DONE": {
  "fork": {
    "branches": {
      "lint":  { "initial": "lint_run",  "terminal": "lint_done"  },
      "types": { "initial": "types_run", "terminal": "types_done" }
    },
    "join": "all",
    "on_complete": "deploying",
    "on_fail": "failed"
  }
}
```

One schema, two execution modes. Sequential: the agent finishes branches one at a time, signalling `statewright_transition(event='BRANCH_DONE:<branch>')`. Parallel (in Claude Code): sub-agents are spawned via the Agent tool, each calling `statewright_load_workflow(name=..., branch='lint')`, and "Each sub-agent gets independent tool enforcement," with the gateway performing the join. A sibling construct, `invoke`, delegates to a sub-machine with `on_complete`, `on_fail`, and `input`.

**But the shared object is unguarded, and it is agent-written.** Top-level `context` holds "Initial context values for guard evaluation," and "The agent writes to context via the `data` parameter of `statewright_transition`" — for example `statewright_transition(event='DEPLOY', data={test_result: 'pass', coverage: 92})`. Guards read it: `{"field": "coverage", "op": "gte", "value": 80}`, with operators `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `in`, `contains`, `exists`, `not_exists`.

Two facts matter for ratmac. First, the ordering rule is deliberate and good: data "merges into context after the transition completes," and guards are evaluated "against the context that existed **before** the current transition, not the `data` being passed with it" — so you cannot supply the value that authorizes your own transition in the same call. Second, and decisively: the branch owns its position in the state machine and its tool allowlist, **not its data**. Nothing in the schema scopes context per branch or arbitrates two branches writing the same field. Statewright has solved routing for fan-out and left state ownership open — which is precisely where ratmac's artifact-verified model has an answer that a context dictionary does not.

### 6.2 The MCP Tasks extension — one writer, at the protocol layer

Tasks are an **extension**, not core protocol: specified in SEP-1686 and SEP-2663, documented at [modelcontextprotocol.io/extensions/tasks/overview](https://modelcontextprotocol.io/extensions/tasks/overview.md), with the full text in the `experimental-ext-tasks` repository. The current specification revision is 2025-11-25 with a draft dated 2026-07-28; Tasks live outside `/specification/` because they are optional.

The model: when a request will be long-running, the server returns a `CreateTaskResult` (`resultType: "task"`) carrying a `taskId`, an initial status, a time-to-live, and a suggested poll interval — and "The task must be durably created before sending the response." The client polls `tasks/get`.

| Status | Meaning |
|---|---|
| `working` | in progress |
| `input_required` | server needs client input; see `inputRequests` |
| `completed` | finished; `result` holds the output |
| `failed` | a JSON-RPC error occurred; `error` holds details |
| `cancelled` | cancelled (not always honored) |

"`completed`, `failed`, and `cancelled` are terminal — once reached, the task's state does not change." Cancellation is cooperative: the server "acknowledges the intent but is not obligated to stop the work."

Three properties are worth ratmac's attention. **The client never writes the status** — it can only poll, answer input requests, and request cancellation. **The status set is closed and small**, with terminal states that are genuinely terminal. **`input_required` is a first-class state**, not an error — the protocol treats "waiting for a human" as a normal, resumable condition with a structured payload attached. That last one is the shape of ratmac's held ticket, and the MCP framing is better: it names what is needed rather than only that something is blocked.

---

## 7. Five honest answers to "two children wrote the same key"

Sorted by how much they cost to adopt.

1. **Refuse, unless a merge rule was declared up front.** LangGraph. Cost: a schema with per-key annotations, and a real error surface. Buys: the failure is a design-time error, not a production race. This is the only approach where "I did not think about concurrency" is caught by the machine.
2. **Serialize every write through an append-only log with per-channel locks, and derive state by replay.** AG2 1.0. Cost: an envelope protocol and a hydration path. Buys: race-freedom plus a full audit trail for free, because the log *is* the state.
3. **Delete the shared key namespace.** Temporal, Inngest core, LangGraph's functional API, and — for state, though not for files — Claude Code subagents. Cost: everything moves through arguments and return values, and (for Temporal) a determinism regime with a versioning tax. Buys: the question cannot be asked.
4. **Run sequentially and call it safe.** CrewAI's task layer, AutoGen's `FIFOLock`, Inngest's AgentKit. Cost: no parallelism. Buys: correctness today and a landmine tomorrow — AgentKit's own source says so, in a comment deferring parallelism "after the foundations are set," on a blackboard with no merge rule.
5. **Share one mutable object and document nothing.** Google ADK, Mastra, CrewAI Flows, OpenAI Agents SDK, Swarm. Cost: zero up front. Buys: a class of bug that reproduces once in fifty runs.

The trap worth naming, because two of the largest vendors fell into it: **isolating the wrong object and describing it as isolation.** Google ADK isolates conversation history per branch and says branches do not share; Mastra isolates step outputs by step id and says branches are independent. Both statements are true of the object named and false of the object that matters. A reader who trusts the summary sentence writes a racy workflow.

---

## 8. What ratmac could steal

ratmac today has exactly one concurrency mechanism — `R-015`, the Scheduler arbitrating access per Run via `.arca/rtm.lock`. There is no subtask, no child Run, no fan-out. That is currently correct (`steering.md`: "one repository, one Run at a time, local disk"), and the feasibility direction already flags the horizon: "Keep v1 single-run, but keep the data model N-run-capable." This survey is about what that data model should look like when the time comes.

**1. Refusal as the default merge rule — the highest-value idea in this document.** LangGraph's `INVALID_CONCURRENT_GRAPH_UPDATE` is the same principle as ratmac's "Refusal over guessing," expressed in a data structure rather than a doctrine. The transfer: if a runbook ever declares parallel phases, any artifact or record two branches may both write must carry a declared combiner in the runbook, or the Engine refuses the fork at **`rtm doctor`** time, before a Run exists. A concurrency bug becomes a runbook validation error with a stable code and a repair — which is exactly the shape ratmac already ships for every other schema fault.

**2. Statewright's fork schema is the right size, and its gap is ratmac's opening.** `branches` / `join: all|any` / `on_complete` / `on_fail` is the smallest declarative fan-out that fits in a plain TOML runbook, and it needs no new vocabulary. What Statewright leaves open is exactly what ratmac is good at: it scopes a branch's position in the state machine and its tool allowlist but not its data, and its `context` is agent-written. ratmac would scope a branch's **evidence root** instead — a directory a branch owns and no sibling may write. Ownership by path is checkable by the same guards that already exist.

**3. Borrow the pre-transition evaluation rule outright.** Statewright evaluates guards "against the context that existed **before** the current transition, not the `data` being passed with it." ratmac's completion gate already has the structural version of this — a receipt goes stale when the declared source roots change, so you cannot edit the work after the check. Say the rule in one sentence in the runbook specification: a gate never reads a value supplied by the act it is gating.

**4. Take `input_required` as a status, not `blocked` as an excuse.** The MCP Tasks lifecycle treats "waiting for a human" as a normal, resumable, structured state carrying an `inputRequests` payload — what is needed, not merely that something stopped. ratmac's `hold` marks a ticket held with a `blocker-ref` pointing at an issue folder or residual. Adopting the Tasks framing means a held ticket names the *input* that would release it, in a form the resuming caller can answer directly.

**5. Do not build a shared state dictionary. Ever.** Four of the twelve systems here shipped one and are still finding the bugs; CrewAI's own tracker records fifty updates from five agents collapsing to a final count of one. ratmac's existing shape — one writer for run state, append-only history, per-ticket evidence directories — is already model (3) from §7, the strongest answer available. The concrete rule: a fanned-out branch reads the frozen goal and its own ticket, and writes only under a path it owns. Combination happens at the join, by a gate that reads both children's artifacts — never by two writers touching one file.

**6. Make ordering explicit before it bites.** LangGraph, having built the most careful state model in the field, still warns that "updates from a parallel superstep may not be ordered consistently," and tells you to carry your own sort key. ratmac's `.arca/log.md` is append-only and single-writer today, so ordering is free. The moment more than one branch can append, every entry needs an ordering value that is not arrival order.

**7. Copy Anthropic's isolation posture, and fix its default.** Claude Code's answer to two subagents editing one file is a git worktree per agent — which ratmac already has, in `tools/trial.ps1`. The gap is that Anthropic's isolation is opt-in, and Anthropic's own C-compiler write-up shows the failure when it is skipped: agents "hit the same bug, fix that bug, and then overwrite each other's changes." If ratmac ever fans out, the worktree is not an option a runbook may decline.

**8. Name the measurement gap out loud.** The most-starred artifact in loop engineering — 9,558 stars — scores a project's loop maturity by checking that certain markdown files exist and that certain phrases appear in them, then prints a number from 0 to 100. ratmac's schema already forbids exactly this: a guidance-consistency check "can never satisfy a behavioral requirement," and every emitted check must name its kind first. That rule is currently an internal working rule. It is also the sharpest one-sentence positioning ratmac has against the loop-engineering ecosystem, and it should appear wherever ratmac is described to outsiders.

**9. What is not worth stealing.** Temporal's determinism regime buys total durability at the price of a sandbox, a versioning protocol, and a ban on wall-clock and randomness inside the loop — far too heavy for a repository-local command-line tool whose runs are minutes long, and in tension with the non-goal "Not a CI system, task queue, or scheduler-as-a-service." Take the *lesson* (ownership as lexical scope, state derived from an append-only log) and leave the machinery. Likewise the LangGraph functional API: it buys ergonomics by discarding the channel discipline, which is the one thing worth having.

---

## 9. Sources, dates, and vapor flags

**Publication dates matter here.** Three of the systems surveyed shipped material changes within 48 hours of this document: LangGraph 1.2.10 (2026-07-28), AG2 1.0 (2026-07-27), OpenAI Agents SDK 0.19.0 (2026-07-27). AG2's write-ahead-log claim in particular is one day old and unweathered — credible from source reading, but nobody has run it in anger.

**Deprecations to note:** AutoGen is in maintenance mode as of its own README, superseded by Microsoft Agent Framework. Google ADK's `ParallelAgent` carries `@deprecated`, "in favor of Workflow." OpenAI's Swarm is dead. AgentKit's `state.kv` is marked deprecated in its own source. Any comparison drawn from a 2025 blog post about these systems is describing something that no longer exists.

**Primary sources used**

Loop tooling — [`cobusgreyling/loop-engineering`](https://github.com/cobusgreyling/loop-engineering) · [`cobusgreyling/harness-foundry`](https://github.com/cobusgreyling/harness-foundry) · [`cobusgreyling/outerloop`](https://github.com/cobusgreyling/outerloop)

Loop layer — [Loop engineering: getting started with loops](https://claude.com/blog/getting-started-with-loops) (2026-06-30) · [How the agent loop works](https://code.claude.com/docs/en/agent-sdk/agent-loop.md) · [Subagents in the SDK](https://code.claude.com/docs/en/agent-sdk/subagents.md) · [Sessions](https://code.claude.com/docs/en/agent-sdk/sessions.md) · [Run agents in parallel](https://code.claude.com/docs/en/agents.md) · [Dynamic workflows](https://code.claude.com/docs/en/workflows.md) · [Agent teams](https://code.claude.com/docs/en/agent-teams.md) · [Worktrees](https://code.claude.com/docs/en/worktrees.md) · [Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) (2025-09-29) · [Building a C compiler with a team of parallel Claudes](https://www.anthropic.com/engineering/building-c-compiler) (2026-02-05)

Graph layer — [LangGraph graph API](https://docs.langchain.com/oss/python/langgraph/graph-api) · [use subgraphs](https://docs.langchain.com/oss/python/langgraph/use-subgraphs) · [checkpointers](https://docs.langchain.com/oss/python/langgraph/checkpointers) · [functional API](https://docs.langchain.com/oss/python/langgraph/functional-api) · [`langgraph/types.py`](https://github.com/langchain-ai/langgraph/blob/main/libs/langgraph/langgraph/types.py) · [`channels/last_value.py`](https://github.com/langchain-ai/langgraph/blob/main/libs/langgraph/langgraph/channels/last_value.py) · [LangGraph releases](https://github.com/langchain-ai/langgraph/releases) · [ADK docs](https://adk.dev/) and [`google/adk-python`](https://github.com/google/adk-python) · [Mastra docs](https://mastra.ai/docs) and [`mastra-ai/mastra`](https://github.com/mastra-ai/mastra) · [CrewAI docs](https://docs.crewai.com/) and [`crewAIInc/crewAI`](https://github.com/crewAIInc/crewAI) · [`microsoft/autogen`](https://github.com/microsoft/autogen) · [`ag2ai/ag2`](https://github.com/ag2ai/ag2) · [OpenAI Agents SDK docs](https://openai.github.io/openai-agents-python/) and [`openai/openai-agents-python`](https://github.com/openai/openai-agents-python) · [`openai/swarm`](https://github.com/openai/swarm)

Durable layer — [Temporal docs](https://docs.temporal.io/) and [`temporalio/sdk-python`](https://github.com/temporalio/sdk-python) · [Inngest docs](https://www.inngest.com/docs) and [`inngest/agent-kit`](https://github.com/inngest/agent-kit)

Gate layer — [Statewright schema reference](https://docs.statewright.ai/workflows/schema-reference/) · [`statewright/statewright`](https://github.com/statewright/statewright) · [MCP Tasks extension](https://modelcontextprotocol.io/extensions/tasks/overview.md) · [MCP versioning](https://modelcontextprotocol.io/specification/versioning)
