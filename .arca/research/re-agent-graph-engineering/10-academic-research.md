# Research papers on graph and state-machine structure for agents

**Date:** 2026-11-21
**Producing agent:** `astra-academic-research` (astra-planner)
**Method:** web research only; results are author-reported and no independent reproduction was located
**Known defects:** None known.

---

## Evidence key

**MEASURED** means author-reported experiments; **R?** means independent reproduction of the reported result was not located, not that reproduction failed.

## Scope and 2026 surveys

1. **DOCUMENTED:** The workflow-optimization survey distinguishes reusable templates, run-specific graphs and execution traces, rather than treating all graphs as state machines; it reports no new benchmark. [2026-03-23](https://arxiv.org/abs/2603.22386)

2. **DOCUMENTED:** *Graph Engineering* separates work organization, agent coordination and runtime-state management; it is a survey, not experimental proof that these are one engineering problem. [2026-08-26](https://arxiv.org/abs/2608.21156)

3. **DOCUMENTED:** *Graph-Mediated LLM Systems* additionally distinguishes reasoning, knowledge, retrieval, execution and coordination graphs; its contribution is taxonomy and evaluation guidance, not a new benchmark. [2026-11-04](https://arxiv.org/abs/2611.05953)

## Foundational experiments

4. **MEASURED; R?:** StateFlow’s explicit states report 13%/28% higher success than ReAct on InterCode-SQL/ALFWorld, with approximately fivefold/threefold lower cost using GPT-3.5. [2024-09-14; older](https://arxiv.org/abs/2403.11322)

5. **MEASURED; R?:** GPTSwarm optimizes prompts and graph connections, reporting Mini Crosswords accuracy of 0.800 versus Tree of Thoughts’ 0.675. [ICML 2024; older](https://ar5iv.labs.arxiv.org/html/2402.16823v2)

6. **MEASURED; R?:** Graph of Thoughts combines and revises intermediate answers, reporting “62%” better sorting quality and over 31% lower cost than Tree of Thoughts on synthetic sorting. [2023-08-18; older](https://arxiv.org/abs/2308.09687)

7. **MEASURED; R?:** MetaGPT’s standardized operating procedures and executable feedback report 85.9% HumanEval and 87.7% MBPP pass@1, testing coding rather than general workflow reliability. [ICLR 2024; older](https://arxiv.org/abs/2308.00352)

8. **MEASURED; R?:** LATS combines tree search, reflection and environment feedback, reporting 92.7% HumanEval pass@1 and a 75.9 WebShop score, not 75.9% task success. [ICML 2024; older](https://proceedings.mlr.press/v235/zhou24r.html)

## Generated and optimized workflows

9. **MEASURED; R?:** ADAS searches executable agent programs, reporting improvements of 13.6 DROP F1 points and 14.4 MGSM accuracy points over hand-designed baselines. [ICLR 2025](https://proceedings.iclr.cc/paper_files/paper/2025/file/36b7acf6f6010652b3f2a433774a66fe-Paper-Conference.pdf)

10. **MEASURED; R?:** AFlow searches code workflows, reporting 5.7% average improvement across HumanEval, MBPP, GSM8K, MATH, HotpotQA and DROP; its headline 4.55%-of-GPT-4o cost concerns inference, not total optimization expenditure. [2025-04-15](https://arxiv.org/abs/2410.10762)

11. **MEASURED; R?:** ScoreFlow trains workflow generators using score-based preferences, reporting 8.2% improvement across the same six benchmark families. [2025-02](https://arxiv.org/abs/2502.04306)

12. **MEASURED; R?:** MetaAgent generates FSM-controlled multi-agent systems, reporting 50% more passed checkpoints than its human-designed comparator in software-development tests. [2025-07-30](https://arxiv.org/html/2507.22606)

13. **MEASURED; R?:** EvoFSM separately modifies transition logic and state-specific skills, reporting 58.0% DeepSearch accuracy with Claude-4 while using slightly more reasoning steps than ReAct. [2026-02-04](https://arxiv.org/abs/2601.09465v2)

## Constraints, verification and monitoring

14. **MEASURED; R?:** SynCode’s grammar-controlled generation reduces Python/Go syntax errors by over 90% in most experiments, while functional-correctness gains are only slight. [2024-11-06; older](https://arxiv.org/html/2403.01632v4)

15. **MEASURED; R?:** AgentSpec’s runtime rules prevent over 90% of unsafe code executions and all tested hazardous embodied actions, across code, household and driving evaluations. [2025-03-24](https://arxiv.org/abs/2503.18666)

16. **MEASURED; R?:** Pro2Guard learns Markov chains and predicts violations, reporting 100% detection of tested driving violations and collisions up to 38.66 seconds ahead. [2025-08-01, v1](https://arxiv.org/html/2508.00500v1)

17. **MEASURED; R?:** Agent-C uses temporal specifications, constrained generation and SMT solving, raising customer-service policy conformance from 77.4% to 100% for Sonnet-4.5 and 83.7% to 100% for GPT-5. [2025-12-25](https://arxiv.org/abs/2512.23738)

18. **MEASURED; R?:** Soteria verifies plans before execution and enforces them afterward, reporting 1.7× average task success and 100% specification conformance across 207 τ²-Airline, τ²-Retail and Toolathlon tasks. [2026-11-13](https://arxiv.org/html/2611.17880)

19. **MEASURED; R?:** *Automata from Agent Traces* learns FSM-based failure monitors, achieving 100% precision but only 11.3% recall at its conservative SWE-agent operating point. [2026-11-12, v2](https://arxiv.org/html/2608.23670)

## StateM: results and boundaries

20. **MEASURED; R?:** StateM reports 424/445 successful Terminal-Bench-2.1 trials, or 95.28%, but explicitly labels this a raw, pre-adjudication submission. [2026-08-15, introduction](https://arxiv.org/pdf/2608.15089)

21. **DOCUMENTED:** Removing four rewarded trajectories the authors agree should fail yields 94.38%, while rejecting all nine flagged trajectories yields 93.26%. [2026-08-15, footnote 1](https://arxiv.org/pdf/2608.15089)

22. **DOCUMENTED:** StateM’s experiments combine a benchmark-adapted runbook with the runtime, and its GPT-5.5 reference is not an agent-version-matched A/B rerun. [2026-08-15, §3.1 and footnote 3](https://arxiv.org/pdf/2608.15089)

23. **MEASURED; R?:** Unchanged StateM transfer lowers DeepSeek accuracy from 82.7% to 82.0%, while BusinessBench’s frozen held-out gain is only 0.55 macro points, with regressions in two families. [2026-08-15, §4.3–4.5](https://arxiv.org/pdf/2608.15089)

24. **DOCUMENTED:** StateM’s $15.20 final-evaluation cost belongs to adapted DeepSeek, with $37.02 additional adaptation, whereas the 95.28% GPT run carries $1,062.95 submission-reported model cost. [2026-08-15, §4.4](https://arxiv.org/pdf/2608.15089)

## Where imposed structure hurts

25. **MEASURED; R?:** *How Strongly Should Task State Influence an LLM Agent?* raises τ²-airline success from 0.39 to 0.54 for one model, but incorrect gating hurts another model on PM-Bench. [2026-09-22](https://arxiv.org/abs/2609.25686)

26. **MEASURED; R?:** *The Verifier Tax* intercepts up to 94% of non-compliant actions yet reports below 5% safe completion in most τ-bench settings and 2.0–2.8× token overhead. [2026-03](https://arxiv.org/abs/2603.19328)

27. **MEASURED; R?:** *In-Context Prompting Obsoletes Agent Orchestration* reports fewer travel failures with full-procedure prompting, 11.5% versus 24%, but measures conversations using LLM judges rather than independent final-state verification. [2026-04-30](https://arxiv.org/abs/2604.27891)

## “From Confident Closing to Silent Failure”: exact findings

28. **DOCUMENTED:** Section 3.5 gives GPT-4o, Sonnet-4.5, Llama-3.3-70B, DeepSeek-R1 and o3-mini trajectories under five conditions: “full trajectory,” final-message removal, completion checklist, “only user requests, tool calls, and tool results,” or turn-by-turn analysis—not merely closing statements. [2026-06-01, §3.5 and Appendix D](https://arxiv.org/html/2606.09863#S3.SS5)

29. **DOCUMENTED:** Appendix D’s additional `strong-real` condition includes “user scenario, expected actions list, and evaluation criteria,” but Table 5 reports it for only three judges; AppWorld likewise uses three judges reading API-call sequences, optionally with task instructions and a checklist. [2026-06-01](https://arxiv.org/html/2606.09863#A4)

30. **MEASURED; R?:** Table 4’s “Task-disjoint (5 seeds)” rows give TF-IDF logistic-regression/XGBoost AUROC of 0.849/0.825±0.025 on τ²-bench and 0.915/0.953±0.020 on AppWorld, with different discrimination targets: false-versus-true success and false-success-versus-honest-failure, respectively. [2026-06-01, §3.5 and Table 4](https://arxiv.org/html/2606.09863#S4.T4)

31. **MEASURED; R?:** Table 5’s best τ² judge AUROC is 0.640, AppWorld’s best is 0.537, and Table 6 reports XGBoost inference at 1.19 ms versus approximately 4,000 ms for judges. [2026-06-01, §4.3–4.4](https://arxiv.org/html/2606.09863#S4.T5)

32. **CLAIMED:** Independent verification is a proposed structural defense, but §4.1 says “We treat this as an observation rather than a causal claim” because only one dual-control domain exists, containing 15 false-success cases. [2026-06-01](https://arxiv.org/html/2606.09863#S4.SS1)

33. **DOCUMENTED:** Section 5 says “The detector is a triage signal, not an autonomous monitor,” citing 50% precision at 10% flagging, domain-calibration needs, untested production transfer and preliminary adversarial rewrites flipping roughly 20–25% of decisions. [2026-06-01](https://arxiv.org/html/2606.09863#S5)

34. **OPINION—inference:** The evidence supports targeted structure and independently checked outcomes, not equivalence between graph representation, grammatical validity, policy compliance and successful execution. [StateM, 2026-08-15](https://arxiv.org/pdf/2608.15089); [false-success study, 2026-06-01](https://arxiv.org/html/2606.09863)

## Open questions

- Which gains survive equal prompts, retries, compute and verified outcomes?
- Which procedures transfer across models and tasks without adaptation?
- Does format familiarity independently improve execution?

## Could not verify

- Matched independent reproductions of the reported headline results; author-run cross-benchmark replication is not independent replication.
- A universally effective process format, arbitrary-agent compatibility or concurrent-task reliability.
- Whether Appendix C’s 200/300-character tool-argument/result truncation also applies to the false-success paper’s judge inputs.

## Source list

- **Surveys:** [Workflow Optimization](https://arxiv.org/abs/2603.22386); [Graph Engineering](https://arxiv.org/abs/2608.21156); [Graph-Mediated LLM Systems](https://arxiv.org/abs/2611.05953).
- **Peer-reviewed foundations:** [StateFlow—COLM 2024](https://arxiv.org/abs/2403.11322); [GPTSwarm—ICML 2024](https://arxiv.org/abs/2402.16823); [Graph of Thoughts—AAAI 2024](https://arxiv.org/abs/2308.09687); [MetaGPT—ICLR 2024](https://arxiv.org/abs/2308.00352); [LATS—ICML 2024](https://proceedings.mlr.press/v235/zhou24r.html).
- **Workflow generation:** [ADAS—ICLR 2025](https://arxiv.org/abs/2408.08435); [AFlow—ICLR 2025](https://arxiv.org/abs/2410.10762); [ScoreFlow](https://arxiv.org/abs/2502.04306); [MetaAgent—ICML 2025](https://arxiv.org/abs/2507.22606); [EvoFSM](https://arxiv.org/abs/2601.09465).
- **Constraints and verification:** [SynCode](https://arxiv.org/abs/2403.01632); [AgentSpec—ICSE 2026](https://doi.org/10.1145/3744916.3764546); [Pro2Guard, v1](https://arxiv.org/abs/2508.00500v1); [Agent-C](https://arxiv.org/abs/2512.23738); [Soteria](https://arxiv.org/abs/2611.17880); [Automata from Agent Traces](https://arxiv.org/abs/2608.23670).
- **Boundaries and counterexamples:** [StateM](https://arxiv.org/abs/2608.15089); [Task-State Influence](https://arxiv.org/abs/2609.25686); [Verifier Tax](https://arxiv.org/abs/2603.19328); [In-Context Prompting](https://arxiv.org/abs/2604.27891).
- **Requested audit:** [From Confident Closing to Silent Failure—FAGEN workshop, ICML 2026](https://arxiv.org/html/2606.09863).