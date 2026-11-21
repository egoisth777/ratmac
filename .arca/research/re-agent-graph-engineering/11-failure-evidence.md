# Measured evidence of the pain

**Date:** 2026-11-21
**Producing agent:** `astra-failure-evidence` (astra-planner)
**Method:** web research only; papers were read, no experiment was reproduced
**Known defects:** None known. Most 2026 sources are preprints without independent replication.

---

## Procedures and false completion

1. **MEASURED —** On SOPBench’s 903 cases across seven domains, GPT-4o passed 62.13% when success required both the correct outcome and complete procedural checks. [S1, 2025-06-17](https://arxiv.org/html/2503.08669v2)

2. **MEASURED —** Across 9,876 τ²-bench runs from eight model families, false completion represented 45–48% of failures in airline/retail domains, versus 3% in telecom where users could independently inspect state. [S2, 2026-06-01](https://arxiv.org/abs/2606.09863)

3. **MEASURED —** A filtered AppWorld sample contained 1,425 false successes and 454 honest failures, making its 75.8% false-success rate conditional on selected self-reporting architectures—not all agent runs. [S2, 2026-06-01](https://arxiv.org/html/2606.09863)

4. **MEASURED —** Across five judge models and five prompting strategies, no LLM judge exceeded 0.65 detection AUROC on τ²-bench, where 0.5 means chance discrimination. [S2, 2026-06-01](https://arxiv.org/abs/2606.09863)

## Long-horizon reliability

5. **MEASURED —** METR’s original 170-task study found nearly 100% success on tasks taking humans under four minutes but below 10% on tasks taking over approximately four hours. [S3, 2025-03-19](https://metr.org/blog/2025-03-19-measuring-ai-ability-to-complete-long-tasks/)

6. **DOCUMENTED —** METR’s horizon measures human-equivalent task duration at a specified success probability, not uninterrupted agent runtime or a controlled experiment extending the same task. [S3, 2025-03-19](https://metr.org/blog/2025-03-19-measuring-ai-ability-to-complete-long-tasks/)

7. **MEASURED —** METR’s newer results report horizons up to 27 hours on TH2’s verifiable subset, within a 36-task, 12-family suite, showing that early-2025 failure rates are not current capability ceilings. [S4, 2026-11-20](https://metr.org/blog/2026-11-20-q4-2026-capability-update/)

## Multi-agent failures

8. **MEASURED —** MAST analyzed 1,642 annotated execution traces across seven multi-agent frameworks and reported task-failure rates ranging from 41% to 86.7%. [S5, 2025-10-26, v3](https://arxiv.org/pdf/2503.13657v3)

9. **MEASURED —** MAST identified 14 failure modes, including disobeying task requirements, repeating completed steps, losing context, premature termination, and inadequate verification. [S5, 2025-10-26, v3](https://arxiv.org/pdf/2503.13657v3)

## Repetition, replanning, and polling costs

10. **MEASURED —** A six-model audit classified approximately 18% of output tokens as removable waste using an LLM judge, rather than treating all expensive computation as waste. [S6, 2026-11-04, preprint](https://arxiv.org/html/2611.07238)

11. **MEASURED —** For over 45% of tasks in that audit, one agent’s identified wasted output exceeded another successful agent’s entire output on the same task. [S6, 2026-11-04, preprint](https://arxiv.org/html/2611.07238)

12. **MEASURED —** Experiments using Llama-3.3-70B across 100 seeds in each of two game environments found excessive replanning increased token use and backtracking rather than reliably improving progress. [S7, 2025-09-03](https://arxiv.org/html/2509.03581v1)

13. **MEASURED —** A one-model simulated waiting experiment reported 152,025 tokens for naive polling over 24 simulated hours versus approximately 920 for event-driven coordination, with 100 coordination-arm runs. [S8, 2026-11-03, preprint](https://arxiv.org/html/2611.04100v1)

14. **MEASURED —** The same simulation reduced polling cost to 11,881 tokens by discarding accumulated context, while a plain external watcher was cheaper than the proposed coordination mechanism. [S8, 2026-11-03, preprint](https://arxiv.org/html/2611.04100v1)

## Human review burden

15. **MEASURED —** METR’s randomized study of 16 experienced developers completing 246 real repository tasks found AI availability increased completion time by 19%, with a 95% confidence interval of approximately 2–39%. [S9, 2025-07-10](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/)

16. **MEASURED —** Within that study’s 44-task screen-recording subset, developers spent approximately 9% of AI-assisted working time reviewing and cleaning generated output. [S9, 2025-07](https://arxiv.org/html/2507.09089)

17. **MEASURED —** Four maintainers reviewing 296 AI-generated PRs across three repositories judged roughly half of benchmark-test-passing patches unsuitable for merging. [S10, 2026-03-10](https://metr.org/notes/2026-03-10-many-swe-bench-passing-prs-would-not-be-merged-into-main/)

18. **DOCUMENTED —** METR subsequently warned that its newer productivity experiment suffered participation and task-selection biases, preventing a reliable estimate of current productivity effects. [S11, 2026-02-24](https://metr.org/blog/2026-02-24-uplift-update/)

## Enforced workflows—and counterexamples

19. **MEASURED —** Across eight seeds of 200-account batches, an executable FSM with Haiku 4.5 averaged 139.2 procedurally faithful, correct decisions versus approximately 48 for the strongest non-FSM baseline. [S12, 2026-11-07, preprint](https://arxiv.org/html/2611.10884)

20. **MEASURED —** In that experiment, plain prompting averaged 7.9 faithful, correct decisions in the first 50 accounts and zero in the final 100, whereas FSM performance remained 31.9–37.6 per 50-account window. [S12, 2026-11-07, preprint](https://arxiv.org/html/2611.10884)

21. **MEASURED —** On 125 screened banking tasks, DeepSeek-V4-Flash passed 70.4% with prose instructions, 86.4% with compiled instructions alone, and 92.8% with a program-guided execution mechanism. [S13, 2026-07-23, v3 preprint](https://arxiv.org/html/2607.11346v3)

22. **DOCUMENTED —** That comparison was not purely prose versus syntax because the official prose omitted alternative ordering and early-exit details retained by the compiled representation. [S13, 2026-07-23](https://arxiv.org/html/2607.11346v3)

23. **MEASURED —** On the same 125 tasks, Qwen2.5-7B fell from 68.8% with freely interpreted compiled instructions to 54.4% under program-guided execution, a statistically significant 14.4-point loss. [S13, 2026-07-23](https://arxiv.org/html/2607.11346v3)

24. **MEASURED —** Plan-and-Act’s 165-task WebArena-Lite ablation improved from 43.63% with its best static planner to 53.94% with dynamic replanning, providing evidence against freezing plans in changing environments. [S14, 2025-04-22, v3](https://arxiv.org/html/2503.09572v3)

## Open questions

- How much benefit comes from enforced transitions versus better specifications, smaller context, additional retries, or stronger outcome checks?
- Do savings survive the human cost of writing, reviewing, and maintaining workflows?
- Can enforcement preserve adaptability when tools fail, requirements change, or unexpected branches become necessary?
- Which failures require workflow control, and which require better reasoning or independently verifiable outputs?

## Could not verify

- Representative production-wide rates for skipped steps, false completion, or wasted polling expenditure.
- The total trajectory count underlying the 18% token-waste estimate.
- Consistent denominators for widely repeated MAST per-mode percentages; versions and secondary summaries disagree.
- Independent replications establishing that the recent FSM results generalize across arbitrary agents and domains.
- A universal exponential success-decay law: METR compares heterogeneous tasks, and changing task length also changes difficulty.
- Evidence that static-plan failures condemn FSMs generally: an FSM can permit branching, recovery, and replanning.

## Sources

- **S1:** [SOPBench](https://arxiv.org/html/2503.08669v2), 2025-06-17.
- **S2:** [From Confident Closing to Silent Failure](https://arxiv.org/abs/2606.09863), 2026-06-01.
- **S3:** [METR: Measuring AI Ability to Complete Long Software Tasks](https://metr.org/blog/2025-03-19-measuring-ai-ability-to-complete-long-tasks/), 2025-03-19.
- **S4:** [METR Q4 capability update](https://metr.org/blog/2026-11-20-q4-2026-capability-update/), 2026-11-20.
- **S5:** [Why Do Multi-Agent LLM Systems Fail?](https://arxiv.org/abs/2503.13657v3), 2025-10-26.
- **S6:** [How Do AI Agents Waste Your Money?](https://arxiv.org/abs/2611.07238), 2026-11-04.
- **S7:** [Learning When to Plan](https://arxiv.org/html/2509.03581v1), 2025-09-03.
- **S8:** [The Agent Board](https://arxiv.org/abs/2611.04100), 2026-11-03.
- **S9:** [METR developer-productivity study](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/), 2025-07-10.
- **S10:** [Many SWE-bench-Passing PRs Would Not Be Merged](https://metr.org/notes/2026-03-10-many-swe-bench-passing-prs-would-not-be-merged-into-main/), 2026-03-10.
- **S11:** [METR productivity-experiment update](https://metr.org/blog/2026-02-24-uplift-update/), 2026-02-24.
- **S12:** [Keeping LLM Agents on Procedure](https://arxiv.org/abs/2611.10884), 2026-11-07.
- **S13:** [Compile, Then Page](https://arxiv.org/abs/2607.11346), 2026-07-23.
- **S14:** [Plan-and-Act](https://arxiv.org/abs/2503.09572), 2025-04-22.