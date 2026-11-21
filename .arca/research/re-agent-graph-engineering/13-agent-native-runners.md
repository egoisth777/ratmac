# Agent-native runners and nearest products

**Date:** 2026-11-21
**Producing agent:** `astra-agent-native-runners` (astra-planner)
**Method:** web research only, nothing installed or run
**Known defects:** None known.

---

## Evidence conventions

1. **DOCUMENTED:** Repository figures below are snapshots accessed 2026-11-21; “push” means latest GitHub push, and stars indicate attention—not deployed users.[S1]

## Requested candidates

2. **DOCUMENTED:** **StateM** uses YAML runbooks and isolated, per-run JSON state through an outside agent’s CLI calls (push 2026-11-10; 1,605 stars).[S2]

3. **DOCUMENTED:** **IX Flow** uses `def.yaml`, `SKILL.md`, and one JSON file per run, with explicit run IDs and structured next actions (push 2026-11-18; 1 star).[S3]

4. **DOCUMENTED:** **agency-runner** implements seven NEXUS phases through one `.agency/state.json` per workspace, without a server or model calls (push 2026-11-21; 0 stars).[S4]

5. **DOCUMENTED:** **GateReeve** combines Markdown policy with a versioned protocol and **SQLite workflow state**, so it is not text-only (push 2026-11-21; 0 stars).[S5]

6. **DOCUMENTED:** **FlowForge**, now `kagura-chen/flowforge`, uses YAML definitions but **SQLite persistence**, with multiple instances available (push 2026-08-27; 8 stars).[S6]

7. **DOCUMENTED:** **Qualixar Bounded Loops & Graphs** uses YAML manifests and hash-chained JSONL ledgers, but its engine executes workers rather than merely recording agent-selected transitions (push 2026-11-20; 5 stars).[S7]

8. **DOCUMENTED:** **Spec Kit’s shipped workflow engine** uses YAML, per-run JSON/JSONL files, and configurable `max_concurrency` fan-out—not just Markdown templates (push 2026-11-20; 150,857 stars).[S8]

9. **DOCUMENTED:** **OpenSpec** supports simultaneous change folders containing Markdown artifacts and YAML dependency schemas, explicitly describing dependencies as enablers rather than rigid gates (push 2026-11-20; 75,368 stars).[S9]

10. **DOCUMENTED:** **Beads**, now `gastownhall/beads`, defaults to single-writer embedded Dolt; concurrent writers require server mode, and JSONL is no longer authoritative (push 2026-11-21; 28,881 stars).[S10]

11. **DOCUMENTED:** **beads_rust** removes daemons and automatic Git operations but retains **SQLite plus JSONL export**, rather than text-only storage (push 2026-11-17; 1,161 stars).[S11]

12. **DOCUMENTED:** **Taskmaster** exposes CLI/MCP task management over tagged `tasks.json` lists, while its documented cluster executor defaults to five concurrent tasks (push 2026-04-28; 28,503 stars).[S12]

## Statewright: confirmed, but a different deployment contract

13. **DOCUMENTED:** **Statewright exists**, uses JSON state-machine definitions, and supports session isolation and fork/join branches (push 2026-11-18; 511 stars).[S13]

14. **DOCUMENTED:** Unlike StateM’s shell-accessible runbook, Statewright intercepts tools through supported host integrations; standalone Cursor MCP/rules mode remains advisory.[S13; undated, accessed 2026-11-21]

15. **DOCUMENTED:** Statewright’s complete self-hosted stack requires PocketBase, an MCP gateway, and an editor, although its Rust engine can be embedded separately.[S13; undated, accessed 2026-11-21]

16. **MEASURED:** Statewright’s author reports improvement from 2/10 to 10/10 attempts across two models on five SWE-bench tasks—not a throughput or general reliability benchmark.[S13; undated, accessed 2026-11-21]

## Additional matches and nearby tools

17. **DOCUMENTED:** **state-machine-skill** uses restricted Graphviz DOT, `state.json`, `history.jsonl`, and independently selected run directories, with dependency-free Python CLI execution (push 2026-11-16; 0 stars).[S14]

18. **DOCUMENTED:** **GraphKit** exposes `gk` commands over `graph.yaml`, durable run ledgers, and worktree-isolated parallel waves (push 2026-11-21; 5 stars).[S15]

19. **DOCUMENTED:** **wfctl** derives a fixed nine-step development pipeline from repository evidence across separate worktrees, without running the agents itself (push 2026-11-20; 0 stars).[S16]

20. **DOCUMENTED:** **Theodosia** exposes Python/Burr graphs through MCP `step(action, inputs)`, supports isolated sessions, and requires an opt-in tracker for disk records (push 2026-11-02; 16 stars).[S17]

21. **DOCUMENTED:** **Philip** already converts subsets of Ansible YAML and Mermaid state diagrams into Burr machines that outside agents can drive through Theodosia (push 2026-05-29; 0 stars).[S18]

22. **DOCUMENTED:** **TaskForge** implements eleven task states in Markdown files through a daemon-free Rust CLI intended for several agents (push 2026-11-11; 0 stars).[S19]

23. **DOCUMENTED:** Markdown-backed task alternatives include **Backlog.md** (push 2026-11-21; 7,456 stars), **hmans/beans** (2026-04-06; 968), and **wedow/ticket** (2026-11-09; 932).[S20]

24. **DOCUMENTED:** Further Markdown trackers include **radutopala/ticket** (push 2026-05-07; 8 stars), **tiquette** (2026-10-05; 0), and **Knot** (2026-09-28; 7).[S21]

25. **DOCUMENTED:** **TicketDeck** reserves completion for humans (push 2026-11-11; 0 stars), while **agilemarkdown** exposes a six-state lifecycle through CLI/MCP (2026-06-11; 8).[S22]

26. **DOCUMENTED:** **Shrimp Task Manager** exposes planning, dependencies, execution guidance, and verification through MCP with local JSON task storage (push 2025-08-21; 2,153 stars).[S23]

27. **DOCUMENTED:** **moul/workgraph** stores durable tasks and handoffs as Markdown plus JSON without a daemon (push 2026-10-04; 0 stars).[S24]

28. **DOCUMENTED:** **agent-control-tower** coordinates DSH role sessions using local files without a database or daemon (push 2026-11-20; 0 stars).[S25]

29. **DOCUMENTED:** **agentic-devtools** exposes workflow-advance commands with worktree-scoped JSON state, but its initiation commands launch Copilot sessions (push 2026-06-12; 7 stars).[S26]

30. **DOCUMENTED:** **Integration Assistant** supplies a domain-specific MCP state machine using JSON files, explicitly without cross-process locking (push 2026-11-21; 1 star).[S27]

## Complaints, limits, and speed evidence

31. **CLAIMED:** StateM users reported nonblocking predicates on 2026-10-04; a corrective pull request merged on 2026-11-10.[S28]

32. **CLAIMED:** FlowForge reports describe wrong-instance selection, unverified completion, YAML bloat, and one execution path per instance—not parallel branches.[S29; 2026-04-14–2026-06-10]

33. **DOCUMENTED:** IX Flow 0.3.3 corrected a published 0.3.2 startup failure.[S30; 2026-10-24]

34. **CLAIMED:** GateReeve’s maintainer reported 15–50-minute decisions/merges from repeated hashing of approximately 40,000 files, alongside oversized review prompts.[S31; 2026-11-07]

35. **CLAIMED:** Beads users reported Dolt migration disruption and recovery problems; the complaint thread closed on 2026-05-22.[S32; opened 2026-03-13]

36. **MEASURED:** A beads_rust issue reports 12.2 seconds and 16,494 SQL statements to import one changed row into a 6,000-issue tracker.[S33; 2026-10-21]

37. **CLAIMED:** A Taskmaster user estimated 45,000–50,000 context tokens for 59 MCP definitions; current documentation provides selective tool loading.[S34; 2025-10-06]

38. **CLAIMED:** OpenSpec users disputed generating design before specifications, illustrating disagreement about whether dependency freedom is desirable.[S35; 2026-02-11]

39. **CLAIMED:** Statewright’s launch discussion challenged licensing and patent-grant wording; the author subsequently reported correcting the license.[S36; 2026-05-12]

## What “agent-native” means

40. **OPINION:** Every defines agent-native architecture through user/agent capability parity, granular tools, composability, emergent capabilities, and improvement over time—not a particular file format.[S37; undated]

41. **OPINION:** Trevin Chow recommends consistent JSON output, clean stdout/stderr separation, stable exit codes, discoverable commands, and noninteractive operation.[S38; 2026-05-01]

42. **DOCUMENTED:** `agcli` implements structured errors, retryability, bounded output, and executable `next_actions` hints as an agent-facing CLI contract.[S39; undated]

43. **DOCUMENTED:** Anthropic recommends actionable model-readable errors, filtering/pagination, and selectable concise versus detailed responses instead of opaque codes or tracebacks.[S40; 2025-09-11]

44. **DOCUMENTED:** MCP’s stdio transport starts a client-owned subprocess, so MCP does not inherently require an independently running background service.[S41; 2025-11-25]

45. **MEASURED:** Scalekit’s 75-run study reports that adding an 800-token CLI skill reduced tool calls and latency by roughly one-third.[S42; 2026-03-11]

46. **MEASURED:** Smithery’s small vendor benchmark instead found native MCP more successful than CLI, with CLI using 2.9× more billed tokens and taking 2.4× longer.[S43; 2026-03-17]

47. **CLAIMED:** Anthropic illustrates reducing one MCP workflow from 150,000 to 2,000 tokens through code execution and filtering, not by abandoning MCP.[S44; 2025-11-04]

## Open questions

- Must the runner prevent unauthorized actions, or merely reject invalid recorded transitions?
- Does an established notation outperform a custom schema when models, tasks, instructions, and output sizes are controlled?

## Could not verify

- Exhaustiveness, actual active users, or safe maximum simultaneous-task counts.
- Comparable speed benchmarks for most candidates; isolated run IDs do not prove concurrent-write safety.
- Independent complaints for several tiny/new repositories, including agency-runner and Qualixar.
- Evidence that training-data familiarity alone makes DOT, Mermaid, Markdown, YAML, or SCXML superior.
- Durable execution-state storage in `slzcdhd/mcp-workflow-server`; its documented JSON file stores definitions.

## Source list

Repository documentation is undated unless stated; repository metadata and current documents were accessed 2026-11-21.

- **S1:** [GitHub repository metadata API](https://docs.github.com/en/rest/repos/repos#get-a-repository); counts and push dates retrieved from the corresponding repositories below.
- **S2:** [StateM README](https://github.com/henryqin1997/statem), [metadata](https://api.github.com/repos/henryqin1997/statem).
- **S3:** [IX Flow guide](https://github.com/agent-ix/ix-flow/blob/main/docs/guide.md), [metadata](https://api.github.com/repos/agent-ix/ix-flow).
- **S4:** [agency-runner releases](https://github.com/mahdyaralipor/agency-runner/releases), [metadata](https://api.github.com/repos/mahdyaralipor/agency-runner).
- **S5:** [GateReeve protocol/storage change](https://github.com/TrentBrown/gatereeve/pull/112), [metadata](https://api.github.com/repos/TrentBrown/gatereeve).
- **S6:** [FlowForge](https://github.com/kagura-chen/flowforge), [metadata](https://api.github.com/repos/kagura-chen/flowforge).
- **S7:** [Bounded Loops](https://github.com/qualixar/bounded-loops), [metadata](https://api.github.com/repos/qualixar/bounded-loops).
- **S8:** [Spec Kit workflow reference](https://github.com/github/spec-kit/blob/main/docs/reference/workflows.md), [June concurrency release notes](https://github.com/github/spec-kit/blob/main/newsletters/2026-June.md), [metadata](https://api.github.com/repos/github/spec-kit).
- **S9:** [OpenSpec concepts](https://github.com/Fission-AI/OpenSpec/blob/main/docs/concepts.md), [metadata](https://api.github.com/repos/Fission-AI/OpenSpec).
- **S10:** [Beads](https://github.com/gastownhall/beads), [metadata](https://api.github.com/repos/gastownhall/beads).
- **S11:** [beads_rust](https://github.com/Dicklesworthstone/beads_rust), [crate](https://crates.io/crates/beads_rust), [metadata](https://api.github.com/repos/Dicklesworthstone/beads_rust).
- **S12:** [Taskmaster cluster execution](https://docs.task-master.dev/capabilities/clusters), [task structure](https://docs.task-master.dev/capabilities/task-structure), [metadata](https://api.github.com/repos/eyaltoledano/claude-task-master).
- **S13:** [Statewright README](https://github.com/statewright/statewright), [metadata](https://api.github.com/repos/statewright/statewright).
- **S14:** [state-machine-skill](https://github.com/coldstar96/state-machine-skill), [metadata](https://api.github.com/repos/coldstar96/state-machine-skill).
- **S15:** [GraphKit](https://github.com/thanhNt16/graph-kit), [metadata](https://api.github.com/repos/thanhNt16/graph-kit).
- **S16:** [wfctl](https://github.com/aamarin/wfctl), [metadata](https://api.github.com/repos/aamarin/wfctl).
- **S17:** [Theodosia](https://github.com/msradam/theodosia), [metadata](https://api.github.com/repos/msradam/theodosia).
- **S18:** [Philip and supported subsets](https://github.com/msradam/philip), [metadata](https://api.github.com/repos/msradam/philip).
- **S19:** [TaskForge](https://github.com/aarontrank/taskforge), [metadata](https://api.github.com/repos/aarontrank/taskforge).
- **S20:** [Backlog.md](https://github.com/MrLesk/Backlog.md), [Beans](https://github.com/hmans/beans), [ticket](https://github.com/wedow/ticket).
- **S21:** [Go ticket](https://github.com/radutopala/ticket), [tiquette](https://github.com/a3lem/tiquette), [Knot](https://github.com/UniSoma/knot).
- **S22:** [TicketDeck](https://github.com/MuqiuPeng/TicketDeck), [agilemarkdown](https://github.com/mreider/agilemarkdown).
- **S23:** [Shrimp repository](https://github.com/cjo4m06/mcp-shrimp-task-manager), [MCP registry documentation](https://glama.ai/mcp/servers/cjo4m06/mcp-shrimp-task-manager).
- **S24:** [moul/workgraph](https://github.com/moul/workgraph).
- **S25:** [agent-control-tower](https://github.com/evolution301/agent-control-tower).
- **S26:** [agentic-devtools](https://github.com/ayaiayorg/agentic-devtools), [PyPI](https://pypi.org/project/agentic-devtools/).
- **S27:** [Integration Assistant](https://github.com/moein-71/Integration-Assistant-MCP-Based-Healthcare-Workflow-Simulator), [registry/storage limitations](https://glama.ai/mcp/servers/moein-71/Integration-Assistant-MCP-Based-Healthcare-Workflow-Simulator).
- **S28:** [StateM predicate report](https://github.com/henryqin1997/statem/issues/4), [merged correction](https://github.com/henryqin1997/statem/pull/6).
- **S29:** FlowForge [instance selection](https://github.com/kagura-chen/flowforge/issues/3), [completion checks](https://github.com/kagura-chen/flowforge/issues/13), [parallelism](https://github.com/kagura-chen/flowforge/issues/15), [YAML bloat](https://github.com/kagura-chen/flowforge/issues/17).
- **S30:** [IX Flow 0.3.3](https://github.com/agent-ix/ix-flow/releases/tag/v0.3.3).
- **S31:** [GateReeve performance report](https://github.com/TrentBrown/gatereeve/pull/99).
- **S32:** [Beads migration complaints](https://github.com/gastownhall/beads/issues/2573).
- **S33:** [beads_rust measured slowdown](https://github.com/Dicklesworthstone/beads_rust/issues/574).
- **S34:** [Taskmaster context complaint](https://github.com/eyaltoledano/claude-task-master/issues/1280), [selective loading](https://docs.task-master.dev/capabilities/mcp).
- **S35:** [OpenSpec dependency-order complaint](https://github.com/Fission-AI/OpenSpec/issues/695).
- **S36:** [Statewright Hacker News discussion](https://news.ycombinator.com/item?id=48108778).
- **S37:** [Every: Agent-native Architectures](https://every.to/guides/agent-native).
- **S38:** [10 Principles for Agent-Native CLIs](https://trevinsays.com/p/10-principles-for-agent-native-clis).
- **S39:** [agcli documentation](https://docs.rs/agcli/latest/agcli/), [crate listing](https://crates.io/crates/agcli).
- **S40:** [Anthropic: Writing effective tools](https://www.anthropic.com/engineering/writing-tools-for-agents).
- **S41:** [MCP stdio transport specification](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports).
- **S42:** [Scalekit benchmark](https://www.scalekit.com/blog/mcp-vs-cli-use), [benchmark code](https://github.com/scalekit-inc/mcp-vs-cli-benchmark).
- **S43:** [Smithery benchmark](https://smithery.ai/blog/mcp-vs-cli-is-the-wrong-fight), [benchmark code](https://github.com/smithery-ai/mcp-vs-cli-bench).
- **S44:** [Anthropic: Code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp).
- **Unverified lead:** [mcp-workflow-server](https://github.com/slzcdhd/mcp-workflow-server).