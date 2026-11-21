# Agent graph engineering - synthesis of the thirteen reports

**Date:** 2026-11-21
**Producing agents:** document-writer (draft and fixes), reviewer (two independent audits)
**Method:** built only from the thirteen stored reports; every statement cites a report and finding; audited twice against the stored reports, 23 fixes then 8 fixes applied; the final text after the last 8 fixes was not audited a third time
**Known defects:** No recommendation by design. MEASURED results are the sources' own; nothing was run or reproduced.

---

Citation convention: This synthesis uses numbered rows, sources, findings or answers, and named sections where needed. `[NN header]` means report metadata; `[index]` means the index.

## 1. What was studied and how

DOCUMENTED: The index dated 2026-11-21 lists thirteen reports. [index] Files 01–05 catalogue frameworks, engines, coding tools, standards and pipelines; 06 reads StateM at 51e4ae5; 07 reads ratmac at 81d54d8; 08–13 cover theory, practice, papers, failures, formats and runners. [index] [06 header] [07 header] The report headers describe web/source reading without executing or reproducing the systems or experiments. [01–13 headers] Sources include older work and recent preprints. [08 header] [11 header] Files 01–05 used a faulty ratmac brief; their positioning is discarded in favour of 06 (StateM) and 07 (ratmac). [index]

MEASURED means source-reported results, not our experiments. Here, NR is a synthesis-only flag: the stored reports do not document an independent repeat. Separately, report 10 says its web research located no independent reproduction; its R? label does not mean reproduction failed. [10 header] [10 section "Evidence key"]

## 2. Are state machines and graphs the same thing?

- DOCUMENTED: a finite-state machine can be drawn as a graph. Researcher inference: the equivalence holds when the graph describes a finite, closed, fully specified global transition system, including data and concurrent configurations—not merely one active step. [08 items 37, 39–40] CLAIMED; NR: bounded parallel work can become one global machine, but combinations multiply its size. Unbounded queues or stacks change the answer. [08 item 1] [08 item 10] [08 item 34] [08 item 36]
- DOCUMENTED: A directed acyclic graph admits a topological ordering that respects prerequisites; that ordering need not form a connected execution path. [08 item 21] Parallel statecharts activate several regions. A complete state must account for them together. [08 item 7]
- Researcher inference: possible completion is not inevitable completion. A task diagram cannot prove that its tools return. [08 item 16] [08 item 38]

## 3. How large is the pain?

- MEASURED; NR: SOPBench’s 903 cases across seven domains gave GPT-4o 62.13% success requiring both outcome and procedure. Across 9,876 τ² runs across eight model families, false completion was 45–48% of airline/retail failures versus 3% in telecom. AppWorld’s selected self-reporting sample contained 1,425 false successes and 454 honest failures: 75.8%, not all runs. [11 item 1] [11 item 2] [11 item 3]
- DOCUMENTED: false-completion judges were GPT-4o, Sonnet-4.5, Llama-3.3-70B, DeepSeek-R1 and o3-mini. Five conditions supplied full trajectories, removed final messages, added checklists, supplied only requests/tool calls/results, or requested turn-by-turn analysis—not just closing statements. Additional “strong-real” supplied user scenarios, expected actions and evaluation criteria to three judges; AppWorld used three judges reading API-call sequences, optionally with instructions and a checklist. [10 item 28] [10 item 29]
- MEASURED; NR: trace classifiers tested on held-out tasks (“Task-disjoint,” five seeds) scored AUROC 0.849/0.825±0.025 for TF-IDF logistic regression/XGBoost on τ² false-versus-true success; AppWorld false-success-versus-honest-failure scores were 0.915/0.953±0.020. Best judge AUROCs were 0.640/0.537 respectively. XGBoost inference took 1.19 ms versus approximately 4,000 ms for judges. AUROC measures discrimination, not accuracy. [10 item 30] [10 item 31]
- CLAIMED: independent verification is a proposed defense, but the authors call the domain comparison “an observation rather than a causal claim”: only one dual-control domain contained 15 false-success cases. DOCUMENTED limits: detector is triage, not an autonomous monitor; 50% precision at 10% flagging; domain calibration needed; production transfer untested; preliminary adversarial rewrites flipped roughly 20–25% of decisions. [10 item 32] [10 item 33]
- MEASURED; NR: MAST’s 1,642 annotated traces across seven frameworks had 41–86.7% task failure. A six-model audit used an LLM judge to classify approximately 18% of output tokens removable waste. [11 item 8] [11 item 10] Report 11 could not verify the total trajectory count underlying this estimate. [11 section "Could not verify"] MEASURED; NR: METR’s 2025 randomized study of 16 experienced developers completing 246 real repository tasks found that AI availability increased completion time by 19%, with a 95% confidence interval of approximately 2–39%. [11 item 15] Its 44-task recording subset spent approximately 9% of AI-assisted working time reviewing and cleaning output. These are not production-wide rates. [11 item 16]
- MEASURED; NR: one model’s waiting simulation used 152,025 tokens for naive polling over 24 simulated hours versus approximately 920 for event-driven coordination, with 100 coordination-arm runs. Discarding context reduced polling to 11,881 tokens; a plain external watcher was cheaper still. [11 item 13] [11 item 14]
- MEASURED; NR: Four maintainers reviewing 296 AI-generated PRs across three repositories judged roughly half of benchmark-test-passing patches unsuitable for merging. [11 item 17]
- MEASURED; NR: METR’s 2025 170-task sample fell below 10% success beyond roughly four human-hours; November 2026 TH2 results reached 27-hour horizons on a verifiable subset within 36 tasks across 12 families. DOCUMENTED: horizons mean human-equivalent task duration at a specified success probability (the stored finding does not give the TH2 cutoff), not agent runtime or a fixed capability ceiling. [11 item 5] [11 item 6] [11 item 7]

## 4. Does structure help or hurt?

### Results favouring structure

| Evidence | Sample and measure | Limit/source |
|---|---|---|
| MEASURED; NR | Across eight seeds of 200-account batches, Haiku 4.5 with an executable FSM averaged 139.2 faithful, correct decisions per batch, versus approximately 48 for the strongest non-FSM baseline. | One task setting. [11 item 19] |
| MEASURED; NR | DeepSeek-V4-Flash, 125 banking tasks: prose 70.4%, compiled instructions 86.4%, guided execution 92.8% passed. | DOCUMENTED: prose omitted ordering/early-exit details; not syntax alone. [11 item 21] [11 item 22] |
| MEASURED; NR | Soteria, 207 τ²-Airline, τ²-Retail and Toolathlon tasks: 1.7× average task success and 100% specification conformance. | These are different measures. [10 item 18] |
| MEASURED; NR | StateM, Terminal-Bench-2.1: 424/445, 95.28% raw success. | DOCUMENTED: pre-adjudication. Counting four author-acknowledged failures as failures gives 94.38%; rejecting all nine flagged trajectories gives 93.26%. Adapted runbook; comparator not version-matched. [10 item 20] [10 item 21] [10 item 22] |

### Costs, limits and counterexamples

| Evidence | Sample and measure | Limit/source |
|---|---|---|
| MEASURED; NR | Qwen2.5-7B, same 125 banking tasks: compiled instructions 68.8% versus guided execution 54.4%. | Significant 14.4-point loss. [11 item 23] |
| MEASURED; NR | Verifier Tax, τ-bench settings: up to 94% non-compliant actions intercepted; below 5% safe completion in most settings; 2.0–2.8× tokens. | Task count not stored. [10 item 26] |
| MEASURED; NR | Travel conversations: full-procedure prompts 11.5% failures versus orchestration 24%. | Sample count not stored; LLM judges, not independent final-state checks. [10 item 27] |
| MEASURED; NR | Plan-and-Act, 165 WebArena-Lite tasks: static planner 43.63%, dynamic replanning 53.94% success. | Static plans, not FSMs generally. [11 item 24] |
| MEASURED; NR | StateM unchanged transfer: DeepSeek accuracy 82.7%→82.0%; held-out BusinessBench gain 0.55 macro points, two families regressed. | Sample sizes not stored here. [10 item 23] |

- MEASURED; NR: τ²-airline success rose 0.39→0.54 for one model; incorrect gates hurt another on PM-Bench. Model names/task counts not stored. [10 item 25]
- MEASURED; NR: an FSM failure monitor’s conservative SWE-agent setting achieved 100% precision but 11.3% recall. Sample count not stored; separate from false-completion classifiers. [10 item 19]
- MEASURED; NR: SynCode reduced Python/Go syntax errors over 90% in most experiments, with only slight functional-correctness gains; sample counts not stored. JSONSchemaBench found decoder-coverage differences across approximately 10,000 schemas. [10 item 14] [12 item 43]
- CLAIMED: GateReeve’s maintainer reported 15–50-minute decisions/merges from repeated hashing of approximately 40,000 files, alongside oversized review prompts. The stored report does not establish current performance. [13 item 34] Claude users reported repeated blocked attempts from conflicting Stop/PreToolUse hooks; sample counts not stored. [13 item 34] [03 section "B. Publicly reported problems"]
- CLAIMED: Vercel simplified its deployed SQL agent because maintaining orchestration scaffolding outweighed improving the agent. Octomind reported inaccessible state and obstructive abstractions in historical LangChain—not current LangGraph. MEASURED; NR: Vercel’s perfect success covered five questions. These cases do not establish an industry-wide retreat. [09 items 39, 42–43] [09 section "Could not verify"]
- DOCUMENTED: LangGraph’s Functional API supplies its runtime through ordinary functions without an explicitly declared graph. Inference: this challenges requiring an explicitly declared graph; it does not show that runtime structure or checks are unnecessary. [09 item 6]
- MEASURED; NR: Llama-3.3-70B, 100 seeds in each of two game environments: excessive replanning increased tokens and backtracking. Different experiment from Plan-and-Act, not a contradiction. [11 items 12, 24]
- DOCUMENTED, ratmac code read: terminal graph position determines `passed`; no code writes `failed`; joins trust unsigned child status. Inference: editing/deleting pin evidence removes the runbook pin. Inference: recorded completion and checked transitions are not uniformly independent proof of successful work. DOCUMENTED: StateM permits `--yes` approval of unconfigured `llm_review`. [07 items 7, 10, 12] [06 item 2]
- DOCUMENTED: METR’s newer productivity experiment had participation/task-selection biases, preventing a reliable current-effect estimate; sample size not stored. [11 item 18]
- DOCUMENTED: StateM’s 95.28% GPT run: $1,062.95 submission-reported model cost for 445 Terminal-Bench-2.1 trials—not an estimate of structure’s overhead. [10 items 20, 24]

## 5. How practitioners build

- MEASURED; NR: MAP’s filtered survey of 86 deployed-system practitioners reported 68% allowing at most ten steps before intervention. [09 item 5] MAP also reported that 74% of its deployed-system respondents primarily depended on human evaluation and that 61% of survey respondents used frameworks; the stored framework-use finding does not specify its denominator. [09 items 28, 35] Separately, 17/20 production case studies used custom implementations. These are different samples, not contradictory adoption estimates. [09 item 34]
- DOCUMENTED patterns: routers, parallel workers/gathering, evaluator/refiner loops, saved intermediate results and checkpoints. Replay can repeat external writes. [09 item 7] [09 item 9] [09 item 10] [09 item 17] [09 item 18] [09 item 19]

- CLAIMED: Manus restricts available tools by state and keeps recoverable information in files. Anthropic’s long-running coding harness uses a checklist, progress file, Git history and incremental tests. The stored findings provide no isolated effect estimates for these practices. [09 items 21, 22, 24]

## 6. Process formats: training exposure, semantics and measured performance

- DOCUMENTED: StarCoder2 training included JSON/XML/YAML, each subsampled to 8GB, and DOT to 1GB. That does not establish current proprietary-model coverage. Downloads/questions measure activity, not learned competence. [12 item 1] [12 item 5] [12 item 19]
- MEASURED; NR: Llama-3.3-70B on 55 descriptions scored 0.43 BPMN element similarity and omitted 7.96 DOT nodes on average. A six-model CI study reported up to 69% reference similarity, only 3% perfect matches; sample count not stored. Similarity is not execution correctness. [12 item 14] [12 item 21] [12 item 27]
- DOCUMENTED: Mermaid/DOT describe diagrams, not executable guards/data passing. PlantUML also depicts rather than executes states; GNU Make drops dependency cycles, leaving runtime loops and data exchange to recipes, variables and files. [12 item 29] Agent-YAML dialects are not interchangeable. XState, SCXML, ASL and Open Workflow have execution rules; JSON Schema alone has none. [12 item 6] [12 item 8] [12 item 10] [12 item 16] [12 item 20] [12 item 23] [12 item 31] [12 item 34] [12 item 35] [12 item 42]

- MEASURED; NR: GPT-4o solved roughly 60% of 1,140 BigCodeBench Python completion tasks, fewer than half of instruction-only variants. Its EnvBench Bash Agent configured 196/665 JVM repositories; compilation checks, not complete runtime tests, judged success. [12 item 39] [12 item 40]

## 7. Documented runner contracts

- DOCUMENTED contracts: StateM uses caller-requested moves checked at states and transitions. [06 items 1, 4] IX Flow adds typed records/invariants. [03 row 19] Statewright supports fork/join and intercepts tools through supported host integrations; standalone Cursor MCP/rules mode remains advisory. [13 items 13–14] Spec Kit executes agents and shell commands. [03 row 6]
- Stars (DOCUMENTED, 2026-11-21): StateM 1,605; IX Flow 1; Statewright 511; Spec Kit 150,857. Attention, not users. [13 item 1] [13 item 2] [13 item 3] [13 item 8] [13 item 13]
- Reported use (CLAIMED): StateM’s authors and two outside testers. Verified production use: not found by our search of stored StateM evidence; the construction-pipeline proposal was unvalidated. [06 item 14]

- DOCUMENTED: GateReeve uses SQLite workflow state. Beads uses single-writer embedded Dolt; concurrent writers need server mode. Philip already converts subsets of Ansible YAML and Mermaid states into Burr machines. [13 item 5] [13 item 10] [13 item 21]

## 8. Ratmac today: documented contracts and inferred limits

- DOCUMENTED: caller-stepped engine using one strict-TOML runbook per project, with nine guard kinds, including command execution and nested runs. [07 section "what the code does today", opening paragraph] Branches need an allowed word plus rationale; failed guards refuse rather than choose repair. Signed recovery permits runbook changes. Some guards assume project-specific ticket/receipt formats. [07 items 1–3] [07 item 4] [07 item 7]
- DOCUMENTED: child joins exist; children cannot spawn children. Nested runs block waiting for receipts, with depth capped at 16. Top-level start accepts no parameters; state outputs have no built-in handoff. A refused step can exit zero; only doctor offers JSON. Ordinary run records remain editable. [07 item 6] [07 item 8] [07 item 10] [07 item 11]
- DOCUMENTED: command checks normally pin executable bytes from first use, but nested drivers and Git are not pinned. Governed step, hold and intent operations require the declared reviewer signatures and the Approver’s signature. Resume requires an Approver signature; abandon remains unsigned. [07 item 9] Ordinary run status is unsigned; only amendments have a hash chain. Inference: the history is not uniformly tamper-proof evidence. [07 item 2] [07 item 7] [07 item 10]
- Inference: arbitrary-task support is constrained by those data, nesting, checking and caller-interface contracts—not merely graph notation. DOCUMENTED: command checks also reject common build/test commands; only doctor requires project-specific receipt/contract guards on cycles; start/step do not enforce that rule. Execution has no attempt cap. [07 item 2] [07 item 5] [07 item 6] [07 item 8] [07 item 11]

## 9. What the evidence does not answer

- Production-wide failure rates and workflow-maintenance costs. [11 section "Could not verify"] [11 section "Open questions"]
- The stored evidence does not isolate enforced transitions from better specifications, smaller context, additional retries or stronger outcome checks. [11 section "Open questions"]
- Report 12 did not verify a matched benchmark covering all candidate process formats. [12 section "Could not verify"]
- Whether tool-argument/result truncation applied to false-success judge inputs; production transfer. [10 section "Could not verify"] [10 item 33]
- Safe simultaneous-task limits or verified adoption for tiny runners. [13 section "Could not verify"]

## 10. Contradictions between reports

- DOCUMENTED correction: 01–05 assume passive checks/immutable runbooks. The code read records executable checks and signed recovery; their ratmac comparisons are unreliable. [01 header] [02 header] [03 header] [04 header] [05 header] [07 item 2] [07 item 7]
- Storage-description mismatch, not proven incompatibility: 03 presents GateReeve files/journal; 13 documents SQLite workflow state. Files may coexist with a database. [03 row 17] [13 item 5]
- 04’s summary says loops “need explicit bounds”; 02’s comparison key says “authors must add bounds”; 05’s key says scripted loops “need author-written stopping rules.” Their Known-defects headers say a bound is “a choice per job.” Report 08 separates possible completion from guaranteed termination. [02 section "Comparison"] [04 section "A. Three recurring choices — assessment"] [05 section "Comparison"] [02 header] [04 header] [05 header] [08 item 16]
