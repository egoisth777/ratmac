# How practitioners engineer agent graphs

**Date:** 2026-11-21
**Producing agent:** `astra-engineering-practice` (astra-planner)
**Method:** web research only, nothing installed or run
**Known defects:** None known. Much vendor guidance is opinion, labelled as such.

---

## Choosing fixed paths or agent loops

1. **DOCUMENTED** — Anthropic distinguishes workflows with predefined code paths from agents that choose their next actions dynamically ([S1](https://www.anthropic.com/engineering/building-effective-agents), 2024-12-19; **older**).

2. **OPINION** — Anthropic recommends workflows for predictable tasks and autonomous agents when the required steps cannot be specified beforehand ([S1](https://www.anthropic.com/engineering/building-effective-agents), 2024-12-19; **older**).

3. **OPINION** — OpenAI recommends exhausting a single agent’s capabilities before adding agents to address instruction complexity or tool confusion ([S2](https://cdn.openai.com/business-guides-and-resources/a-practical-guide-to-building-agents.pdf), 2025-04).

4. **CLAIMED** — Google’s ADK 2.0 rationale separates deterministic routing from the individual nodes that need probabilistic model reasoning ([S3](https://developers.googleblog.com/why-we-built-adk-20/), 2026-07-01).

5. **MEASURED** — MAP’s filtered survey of 86 deployed-system practitioners found that 68% allowed at most ten steps before human intervention ([S4](https://arxiv.org/html/2512.04123v4), 2026-06-04).

6. **DOCUMENTED** — LangGraph’s Functional API provides its runtime through ordinary functions without requiring an explicitly declared graph ([S5](https://www.langchain.com/blog/introducing-the-langgraph-functional-api), 2025-01-29).

## Common patterns

7. **DOCUMENTED** — Google’s coordinator/router pattern sends requests to specialists using their descriptions and the request’s content ([S6](https://developers.googleblog.com/developers-guide-to-multi-agent-patterns-in-adk/), 2025-12-16).

8. **DOCUMENTED** — Anthropic distinguishes orchestrator–worker decomposition, decided at runtime, from parallel tasks defined in advance ([S1](https://www.anthropic.com/engineering/building-effective-agents), 2024-12-19; **older**).

9. **DOCUMENTED** — Google’s map-reduce-style fan-out/gather pattern runs workers concurrently and passes their saved results to a final synthesizer ([S6](https://developers.googleblog.com/developers-guide-to-multi-agent-patterns-in-adk/), 2025-12-16).

10. **DOCUMENTED** — Google implements evaluator–optimizer loops with separate generator, critic, and refiner agents ([S6](https://developers.googleblog.com/developers-guide-to-multi-agent-patterns-in-adk/), 2025-12-16).

11. **DOCUMENTED** — Microsoft’s Magentic manager creates a plan, assigns specialists, checks progress, and replans when execution stalls ([S7](https://devblogs.microsoft.com/agent-framework/agent-frameworks-orchestration-patterns-reach-1-0/), 2026-07-07).

12. **OPINION** — OpenAI recommends human gates after repeated failures and before sensitive or irreversible actions ([S2](https://cdn.openai.com/business-guides-and-resources/a-practical-guide-to-building-agents.pdf), 2025-04).

13. **DOCUMENTED** — AWS describes explicit graphs as suitable for processes requiring prescribed steps, approvals, or quality gates ([S8](https://aws.amazon.com/blogs/opensource/introducing-strands-agents-1-0-production-ready-multi-agent-orchestration-made-simple/), 2025-07-15).

## Limits, state, and recovery

14. **DOCUMENTED** — Google’s refinement example combines `max_iterations=3` with an early-completion signal ([S6](https://developers.googleblog.com/developers-guide-to-multi-agent-patterns-in-adk/), 2025-12-16).

15. **OPINION** — Microsoft recommends iteration caps with explicit fallback behavior rather than assuming the evaluator will eventually approve ([S9](https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns), source metadata 2026-02-12).

16. **OPINION** — HumanLayer recommends feeding compact tool errors back into context so the model can repair its next action ([S10](https://www.humanlayer.dev/blog/12-factor-agents), 2025-04-03).

17. **DOCUMENTED** — Google passes intermediate results through named `session.state` entries written using `output_key` ([S6](https://developers.googleblog.com/developers-guide-to-multi-agent-patterns-in-adk/), 2025-12-16).

18. **DOCUMENTED** — LangChain uses intermediate checkpoints both for failure recovery and for planned interruptions such as human review ([S11](https://www.langchain.com/blog/building-langgraph), 2025-09-04).

19. **DOCUMENTED** — LangGraph warns that external writes must remain safe to repeat because replay can re-execute unfinished operations ([S12](https://docs.langchain.com/oss/python/langgraph/functional-api), undated; retrieved 2026-11-21).

20. **OPINION** — HumanLayer recommends unifying execution state with business state instead of maintaining competing representations ([S10](https://www.humanlayer.dev/blog/12-factor-agents), 2025-04-03).

21. **CLAIMED** — Manus uses a context-aware state machine to restrict available tools without removing their definitions from context ([S13](https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus), 2025-07-18).

22. **CLAIMED** — Manus stores recoverable information in files so context can shrink without permanently discarding that information ([S13](https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus), 2025-07-18).

23. **DOCUMENTED** — Anthropic treats compaction, persistent notes, and isolated subagent contexts as separate controls for long-running work ([S14](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents), 2025-09-29).

24. **CLAIMED** — Anthropic’s long-running coding harness combines an initializer, feature checklist, progress file, Git history, and incremental testing ([S15](https://www.anthropic.com/research/effective-harnesses-for-long-running-agents), 2025-11-26).

25. **CLAIMED** — Anthropic chose JSON for its feature checklist because agents altered it inappropriately less often than Markdown during internal experimentation ([S15](https://www.anthropic.com/research/effective-harnesses-for-long-running-agents), 2025-11-26).

26. **CLAIMED** — OpenAI runs an isolated application instance per Git worktree so simultaneous coding tasks can exercise their own changes ([S16](https://openai.com/index/harness-engineering/), 2026-02-11).

## Testing and observation

27. **DOCUMENTED** — Google evaluates both final responses and tool-use trajectories against predefined cases ([S17](https://developers.googleblog.com/en/agent-development-kit-easy-to-build-multi-agent-applications/), 2025-04-09).

28. **MEASURED** — MAP reports that 74% of its deployed-system respondents primarily depend on human evaluation ([S4](https://arxiv.org/html/2512.04123v4), 2026-06-04).

29. **MEASURED** — LangChain’s 1,340-response survey found detailed step-and-tool tracing in 62% of responding organizations ([S18](https://www.langchain.com/state-of-agent-engineering), publication undated; survey 2025-11-18–12-02).

30. **CLAIMED** — OpenAI exposes task-local logs, metrics, and traces directly to Codex through an ephemeral observability stack ([S16](https://openai.com/index/harness-engineering/), 2026-02-11).

31. **CLAIMED** — Orbital’s AI Engineer World’s Fair talk describes model upgrades breaking prompts in its deployed real-estate legal agent ([S19](https://tech.orbitalwitness.com/posts/2025-06-11-ai-engineer-worlds-fair-talk/), write-up 2025-06-11).

## Failures and custom implementations

32. **MEASURED** — LangChain’s survey ranked quality as the leading production barrier, selected by 32% of respondents ([S18](https://www.langchain.com/state-of-agent-engineering), publication undated; survey 2025-11-18–12-02).

33. **CLAIMED** — Anthropic reports that vague research delegation caused subagents to misunderstand assignments or duplicate searches ([S20](https://www.anthropic.com/engineering/multi-agent-research-system), 2025-06-13; mirror cross-checked).

34. **MEASURED** — Seventeen of MAP’s twenty production case studies used custom implementations, citing flexibility, simplicity, and control ([S4](https://arxiv.org/html/2512.04123v4), 2026-06-04).

35. **MEASURED** — MAP’s survey nevertheless reported framework use by 61% of respondents ([S4](https://arxiv.org/html/2512.04123v4), 2026-06-04).

36. **CLAIMED** — MAP documents two teams moving from frameworks to custom production implementations ([S4](https://arxiv.org/html/2512.04123v4), 2026-06-04).

37. **OPINION** — Dex Horthy’s AI Engineer talk advocates owning control flow instead of hiding retry, pause, and termination decisions inside framework abstractions ([S21](https://ai.engineer/talks/12-factor-agents), recording 2025-07-03).

38. **CLAIMED** — Manus rebuilt its agent framework four times as its team discovered better context-management approaches ([S13](https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus), 2025-07-18).

39. **CLAIMED** — Octomind left historical LangChain because nested abstractions and inaccessible agent state obstructed its testing workflow ([S22](https://octomind.dev/blog/why-we-no-longer-use-langchain-for-building-our-ai-agents), 2024-06-21; **older, not current LangGraph**).

40. **OPINION** — Cognition’s 2025 guidance favors shared decision context because independently acting agents can make incompatible choices ([S23](https://cognition.com/blog/dont-build-multi-agents), 2025-06-12).

41. **OPINION** — Cognition’s 2026 follow-up favors parallel analysis while keeping writes single-threaded ([S24](https://cognition.com/blog/multi-agents-working), 2026-04-22).

42. **CLAIMED** — Vercel simplified its deployed SQL agent after maintaining orchestration scaffolding became more work than improving the agent ([S25](https://vercel.com/blog/we-removed-80-percent-of-our-agents-tools), 2025-12-22).

43. **MEASURED** — Vercel’s advertised perfect success result covered only five evaluation questions ([S25](https://vercel.com/blog/we-removed-80-percent-of-our-agents-tools), 2025-12-22).

## Open questions

- Which transitions require guarantees, and which benefit from model judgment?
- How are context, durable state, working files, and external side effects separated?
- Which parallel tasks can operate without conflicting writes?
- Does familiar process syntax improve task success, or merely ease authoring?

## Could not verify

- No retrieved controlled study established a universally superior textual process format.
- The evidence does not establish an industry-wide retreat from graph frameworks.
- Anthropic’s research-system details required quotations and a mirror because its full primary page was not retrievable.
- Checklist-format observations and five-question evaluations do not establish broad production reliability.

## Sources

- **S1:** Anthropic, *Building effective agents*.
- **S2:** OpenAI, *A practical guide to building agents*.
- **S3:** Google, *Why we built ADK 2.0*.
- **S4:** Pan et al., *Measuring Agents in Production*, v4.
- **S5:** LangChain, *Introducing the LangGraph Functional API*.
- **S6:** Google, *Developer’s guide to multi-agent patterns in ADK*.
- **S7:** Microsoft, *Agent Framework’s Orchestration Patterns Reach 1.0*.
- **S8:** AWS, *Introducing Strands Agents 1.0*.
- **S9:** Microsoft Azure Architecture Center, *AI agent orchestration patterns*.
- **S10:** HumanLayer, *12 Factor Agents*.
- **S11:** Nuno Campos/LangChain, *Building LangGraph*.
- **S12:** LangChain, *Functional API overview*.
- **S13:** Yichao Ji/Manus, *Context Engineering for AI Agents*.
- **S14:** Anthropic, *Effective context engineering for AI agents*.
- **S15:** Anthropic, *Effective harnesses for long-running agents*.
- **S16:** OpenAI, *Harness engineering*.
- **S17:** Google, *Agent Development Kit: Making it easy to build multi-agent applications*.
- **S18:** LangChain, *State of Agent Engineering*.
- **S19:** Andrew Thompson/Orbital, AI Engineer World’s Fair talk write-up.
- **S20:** Anthropic, *How we built our multi-agent research system*; [checked mirror](https://github.com/newton20/harness-engineering-kb/blob/master/raw/anthropic-com-engineering-multi-agent-research-system.md).
- **S21:** Dex Horthy, *12-Factor Agents*, AI Engineer World’s Fair recording.
- **S22:** Octomind, *Why we no longer use LangChain*.
- **S23:** Cognition, *Don’t Build Multi-Agents*.
- **S24:** Cognition, *Multi-Agents: What’s Actually Working*.
- **S25:** Vercel, *We removed 80% of our agent’s tools*.
