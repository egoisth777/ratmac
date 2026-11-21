# Experiment and Pipeline Tools

**Date:** 2026-11-21
**Producing agent:** `astra-experiment-pipeline-tools`
**Method:** web research only, nothing installed or run
**Known defects:** These reports compared ratmac using an inaccurate summary: it said ratmac checks run nothing and a runbook cannot change during a Run. Both are wrong; see 07-ratmac-baseline.md. Every statement here about ratmac position or uniqueness is therefore unreliable. Also wrong in 04: "every model needs an explicit loop limit"; a state machine may run without end by design, and a bound is a choice per job.

---

## Comparison

All requested products were located. † marks research/recent systems; other products are established implementations. These maturity labels are **judgments**, not adoption measurements.

**Key:** R = runtime advances and executes code; A = model directs tools; H = outside caller advances and performs work. O = offline/local; S = self-hosted services; C = cloud. Offline assumes installed dependencies. “Scripted” loops need author-written stopping rules. Unlisted approval, timeout, retention, and concurrency limits remain unverified. Ordinary local files remain owner-editable/deletable—an inference, not an application guarantee.

| Product, maker, licence (1) | Definition; parameters; mover/worker (2–4) | Completion; branches/loop bounds (5–6) | State/history; handoff; concurrent runs (7–8) | Approval/recovery/deployment (9–10) |
|---|---|---|---|---|
| **Snakemake**, team; MIT [1] | Snakefile/config; R | Outputs plus `ensure` checksums/nonempty; checkpoint branches; scripted loops | `.snakemake`/files; paths; separate directories | Retry/rerun; O |
| **Nextflow**, Seqera; Apache-2.0 [2] | DSL/params; R | Exit+outputs; data branches; preview recursion `times`/`until` | LevelDB/files; channels; session IDs | Retry/timeout/resume; O |
| **DVC pipelines**, lakeFS; Apache-2.0 [3] | YAML/parameter overrides; R | Exit+outputs; hashes determine reuse; DAG/matrix, scripted quality loops | Lock/cache/Git; files; isolated queue `-j` | Reproduce after failure; O |
| **MLflow tracking**, Databricks/community; Apache-2.0 [4] | No graph; logged parameters; H | Caller ends run; caller branches/loops | SQL/artifacts, deletable; metrics/files; concurrent IDs | Caller-managed status/recovery; O |
| **W&B runs/sweeps**, CoreWeave; MIT SDK/commercial service [5] | YAML/config; controller→worker | Exit/logged metrics; search algorithm, `run_cap`, early termination | Server/local files, deletable; config/artifacts; multiple agents | Sweep pause/requeue; tracking O, sweeps S/C |
| **Kedro**, LF AI & Data; Apache-2.0 [6] | Python DAG/parameters; R | Return/save; scripted branching/quality loops | Session memory/configurable catalog; named datasets; separate sessions | Failure/rerun; O |
| **Metaflow**, Netflix/community; Apache-2.0 [7] | Python/Parameters; R | Return; conditions/self-recursion, author-bounded | Local/cloud immutable artifacts; editable tags; concurrent IDs | Retry/timeout/resume; O |
| **Luigi**, Spotify/community; Apache-2.0 [8] | Python task parameters; R; `ExternalTask` uses outside work | `complete()` usually checks targets; dynamic dependencies/scripted loops | Scheduler memory/file, optional SQL history; targets; parameterized task identities | Retry/rerun; O/optional server |
| **GNU Make**, GNU; GPL-3.0+ [9] | Makefile/variables; R | Timestamps/exit—not scientific validation; scripted branches/loops | Memory/files; files/variables; caller isolates runs | Failure/rerun; O |
| **just**, Casey Rodarmor; CC0 [10] | justfile/arguments; R | Exit; shell branches/loops | No durable run ledger; files/stdout; caller isolates runs | Confirmation/failure/rerun; O |
| **Task**, go-task; MIT [11] | YAML/variables; R | Exit; `status`/hashes are preflight checks; conditions/list loops | `.task` fingerprints/files; variables; caller isolates runs | Confirmation/failure/rerun; O |
| **AiiDA**, core team; MIT [12] | Python WorkChain/input ports; R | Validators/exit; `if_`/`while_`; restart-chain `max_iterations` | SQL/repository; immutable provenance but deletable nodes; ports; concurrent processes | Pause/play/restart limits; O/services |
| **AI Scientist v1/v2†**, Sakana; custom AI Scientist licence [13] | Code/prompts/config; A | Execution/evaluation/model judgment; run/stage caps | Local workspaces/logs; results; v2 workers | Repair budgets/timeouts; model APIs |
| **AI Scientist v3†**, Sakana; Apache-2.0 [14] | Phase code/Hydra overrides; runtime→coding agents | Evaluations/model decisions; idea-cycle caps | Run directories; artifacts; parallel workers | Phase resume; drafting exception; multiple agent tools |
| **Agent Laboratory†**, Schmidgall/team; MIT [15] | Python/YAML/topic; A | Code/model/reviewer; `max_steps` | Local checkpoints/files; phase context; parallel option | Copilot feedback/reload; model APIs |
| **Coscientist†**, Gomes group; Apache-2.0+Commons Clause [16] | Planner prompts/tools; A | Tool/experimental results; loop cap? | Published trajectories; live persistence/concurrency? | Recovery?; published system GPT-4/cloud |
| **AI co-scientist†**, Google; public code licence? [17] | Research goal; specialist-agent process | Model tournaments/refinement; hard cap? | Context memory; hypotheses; asynchronous agents | Human feedback; recovery?; Gemini/cloud |
| **Robin†**, FutureHouse; Apache-2.0 [18] | Python configuration; model→tools/human experiments | Analysis/experimental feedback; loop cap? | Timestamped local outputs; reports/data; run limit? | Human laboratory handoff; recovery?; multiple API providers |
| **protocols.io**, Springer Nature; commercial platform/CC-BY public protocols [19] | Structured prose/quantities; H | Checked steps; human branches/repetition | Cloud run records/notes; concurrent records; retention? | Manual continuation; offline? |
| **Rundeck / PagerDuty Runbook Automation**, PagerDuty; Apache-2.0/commercial [20] | UI/YAML/XML/options; R | Exit/plugin checks; scripts, paid conditionals; retry caps | DB/logs, purgeable; captured variables; overlapping runs opt-in | Retry/timeout; resume commercial; S/C |
| **StackStorm/Orquesta**, community/LF; Apache-2.0 [21] | YAML/inputs; R | Action result/expressions; transitions/retry counts | MongoDB, configurable purging; context/outputs; concurrent executions | `core.ask`, pause/resume/retry; S |
| **SSM Automation**, AWS; commercial service [22] | YAML/JSON/visual/parameters; R | Action/API assertions; branches; loops ≤100 | AWS history expires after 30 days; named outputs; quota-controlled runs | Approval/retry/timeout/failure; C/AWS |
| **Ansible**, Red Hat/community; GPL-3.0+ [23] | YAML/extra-vars; R | Module result/`assert`/file hashes; `when`, bounded `until` | Memory/optional logs; registered variables; caller isolates runs | Pause/step/retry/start-at-task; O |
| **Process Street**, Process Street; commercial [24] | Visual forms; H/automations | Required tasks/approval; conditions/rejection loops, cap? | Cloud records, deletable; fields/files; separate runs | Approval/rework; C |
| **Manifestly**, Manifestly LLC; commercial [25] | Visual checklist/fields; H/API | Checked steps/signoff; conditional visibility; loop cap? | Cloud history, deletable; fields/files; separate runs | Approval/manual recovery; C |
| **Strands Agent SOPs†**, AWS/community; Apache-2.0 [26] | Markdown/parameters; outside agent | Agent follows instructions; prose branches/bounds | Host-defined state/history/concurrency; prompts/files | Approval/resume by instruction; host/model-dependent |

**Late-2026 caveats:** DVC changed stewardship; MLflow’s filesystem tracking backend is deprecated; W&B’s local sweep controller is deprecated; StackStorm no longer supports Mistral. These are not product shutdowns.[3–5,21]

## A. Three common choices

- Runtime owns execution and advancement.
- Parameters enter once; named outputs, files, or shared context connect steps.
- Retries and finite fan-out are clearer than “repeat until scientifically good.”

These are synthesis judgments from the comparison.

## B. Publicly reported problems

First-hand reports—not reproduced, and not assumed still unfixed:[27]

- DVC: shared-workspace locks obstruct concurrent experiments.
- Orquesta: retrying a fan-out task repeats successful items.
- AI Scientist v3: failed drafting cannot resume.
- protocols.io: Edinburgh reported a subscription increase exceeding 700%.

## C. Closest to ratmac

- **Luigi `ExternalTask`**: outside work plus independent completion checks; still a scheduler/task system.[8]
- **Snakemake `ensure`, Task `status`, DVC hashes**: file evidence, but execution/caching rather than caller-driven state transitions. Task’s checks are preflight.[1,3,11]
- **Strands SOPs**: agent-followed procedures, but instruction compliance replaces enforced gates.[26]
- **AI Scientist v3**: run directories and resumption, but it launches agents and specializes in research.[14]

## D. Unfilled combination

**Inference:** none inspected documents ratmac’s entire supplied combination: caller-only advancement, file checks, closed branch words plus reasons, checksum-fixed runbook, repository append-only history, and child-run joins. Individual ingredients are not unique.

## E. What ratmac lacks

Most engines execute work and manage parallelism. Many add retries, scheduling, remote workers, dashboards, or authenticated approvals.[1–12,20–25] Ratmac’s narrower opportunity is **evidence-gated progress without owning execution**, not another experiment scheduler.

**Skipped:** exhaustive ELNs, robotics drivers, Calkit, additional science-agent forks, paid-feature testing, adoption counts, and token-saving benchmarks.

## Sources

Sources below are official documentation/repositories, original research, or explicitly identified first-hand reports.

1. **Snakemake:** [Repository/licence](https://github.com/snakemake/snakemake); [rules, checks, checkpoints, retries](https://snakemake.readthedocs.io/en/stable/snakefiles/rules.html); [CLI/configuration](https://snakemake.readthedocs.io/en/stable/executing/cli.html); [provenance](https://snakemake.readthedocs.io/en/latest/executing/provenance.html); [locking](https://snakemake.readthedocs.io/en/stable/project_info/faq.html).
2. **Nextflow:** [Project/licence](https://docs.seqera.io/nextflow); [maintainer](https://www.nextflow.io/about-us.html); [workflows/recursion](https://nextflow.io/docs/latest/workflow.html); [processes/retries](https://www.nextflow.io/docs/latest/process.html); [cache and resume](https://www.nextflow.io/docs/latest/cache-and-resume.html).
3. **DVC:** [Repository](https://github.com/treeverse/dvc); [stewardship change](https://dvc.org/blog/dvc-joins-lakefs-your-questions-answered/); [pipeline/lock schema](https://dvc.org/doc/user-guide/project-structure/pipelines-files); [experiment parameters](https://doc.dvc.org/command-reference/exp/run); [parallel queue](https://dvc.org/doc/command-reference/queue/start); [experiment isolation](https://doc.dvc.org/user-guide/experiment-management/running-experiments).
4. **MLflow:** [Repository/licence](https://github.com/mlflow/mlflow); [origin/governance](https://www.linuxfoundation.org/press/press-release/the-mlflow-project-joins-linux-foundation); [tracking API](https://mlflow.org/docs/latest/ml/tracking/tracking-api/); [local database](https://mlflow.org/docs/latest/ml/tracking/tutorials/local-database/); [backend stores](https://mlflow.org/docs/latest/self-hosting/architecture/backend-store/); [deprecation notice](https://github.com/mlflow/mlflow/issues/18534); [deletion API](https://mlflow.org/docs/latest/rest-api.html).
5. **W&B:** [SDK/licence](https://github.com/wandb/wandb); [CoreWeave acquisition](https://investors.coreweave.com/news/news-details/2025/CoreWeave-Completes-Acquisition-of-Weights--Biases/default.aspx); [sweep configuration](https://docs.wandb.ai/models/sweeps/sweep-config-keys); [agents](https://docs.wandb.ai/models/sweeps/start-sweep-agents); [pause/resume](https://docs.wandb.ai/models/sweeps/pause-resume-and-cancel-sweeps); [preemption](https://docs.wandb.ai/models/sweeps/signal-handling-sweep-runs); [delete runs](https://docs.wandb.ai/models/runs/delete-runs); [local-controller deprecation](https://docs.coreweave.com/products/wandb/sweeps/local-controller); [offline logging](https://docs.coreweave.com/products/wandb/integrations/add-wandb-to-any-library).
6. **Kedro:** [Governance](https://lfaidata.foundation/projects/kedro/); [licence](https://github.com/kedro-org/kedro/blob/main/LICENSE.md); [parameters](https://docs.kedro.org/en/stable/configuration/parameters.html); [execution](https://docs.kedro.org/en/stable/build/run_a_pipeline/); [sessions](https://docs.kedro.org/en/stable/extend/session/); [ephemeral session-store implementation](https://docs.kedro.org/en/0.19.3.post1/_modules/kedro/framework/session/store.html).
7. **Metaflow:** [Origin/licence](https://docs.metaflow.org/introduction/what-is-metaflow); [flows/conditions/recursion](https://docs.metaflow.org/metaflow/basics); [parameters](https://docs.metaflow.org/api/flowspec); [failure handling](https://docs.metaflow.org/scaling/failures); [resume](https://docs.metaflow.org/metaflow/debugging); [tags](https://docs.metaflow.org/metaflow/client); [artifact immutability](https://outerbounds.com/blog/five-ways-to-use-the-new-metaflow-tags); [datastore implementation](https://github.com/Netflix/metaflow/blob/master/docs/datastore.md).
8. **Luigi:** [Tasks/completion/dynamic dependencies](https://luigi.readthedocs.io/en/stable/tasks.html); [`ExternalTask`](https://luigi.readthedocs.io/en/stable/api/luigi.task.html); [configuration/retries/state](https://luigi.readthedocs.io/en/latest/configuration.html); [scheduler/history](https://luigi.readthedocs.io/en/stable/central_scheduler.html); [source/licence](https://luigi.readthedocs.io/en/stable/_modules/luigi/worker.html).
9. **GNU Make:** [Official manual](https://www.gnu.org/software/make/manual/make.html).
10. **just:** [Repository/licence/manual](https://github.com/casey/just); [confirmation](https://just.systems/man/en/attributes.html); [parallelism](https://just.systems/man/en/parallelism.html).
11. **Task:** [Repository/licence](https://github.com/go-task/task); [up-to-date checks](https://taskfile.dev/docs/guide/up-to-date); [loops](https://taskfile.dev/docs/guide/loops); [dependencies/failure](https://taskfile.dev/docs/guide/dependencies); [variables/conditions/confirmation](https://taskfile.dev/docs/guide/).
12. **AiiDA:** [Repository/licence](https://github.com/aiidateam/aiida-core); [WorkChain semantics](https://aiida.readthedocs.io/projects/aiida-core/en/latest/topics/workflows/usage.html); [restart bounds](https://aiida.readthedocs.io/projects/aiida-core/en/stable/howto/workchains_restart.html); [process control](https://aiida.readthedocs.io/projects/aiida-core/en/stable/topics/processes/usage.html); [storage installation](https://aiida.readthedocs.io/projects/aiida-core/en/stable/installation/guide_quick.html); [immutability](https://aiida.readthedocs.io/projects/aiida-core/en/latest/topics/processes/concepts.html); [deletion](https://aiida.readthedocs.io/projects/aiida-core/en/latest/howto/data.html).
13. **AI Scientist v1/v2:** [v1 experiment limits](https://github.com/SakanaAI/AI-Scientist/blob/main/ai_scientist/perform_experiments.py); [v2 repository/licence](https://github.com/SakanaAI/AI-Scientist-v2); [v2 configuration](https://github.com/SakanaAI/AI-Scientist-v2/blob/main/bfts_config.yaml).
14. **AI Scientist v3:** [Official repository, configuration, security, resumption](https://github.com/SakanaAI/AI-Scientist-v3).
15. **Agent Laboratory:** [Repository/licence/checkpoints/copilot](https://github.com/SamuelSchmidgall/AgentLaboratory); [implementation/limits](https://github.com/SamuelSchmidgall/AgentLaboratory/blob/main/ai_lab_repo.py); [original paper](https://arxiv.org/abs/2501.04227).
16. **Coscientist:** [Repository/licence](https://github.com/gomesgroup/coscientist); [original research and implementation disclosure](https://www.nature.com/articles/s41586-023-06792-0).
17. **Google AI co-scientist:** [Official introduction](https://research.google/blog/accelerating-scientific-breakthroughs-with-an-ai-co-scientist/); [2026 original research](https://www.nature.com/articles/s41586-026-10644-y).
18. **Robin:** [Repository/licence/configuration/providers](https://github.com/Future-House/robin); [FutureHouse announcement](https://www.futurehouse.org/research-announcements/demonstrating-end-to-end-scientific-discovery-with-robin-a-multi-agent-system); [original paper](https://arxiv.org/pdf/2505.13400).
19. **protocols.io:** [Ownership](https://group.springernature.com/gp/group/media/press-releases/publisher-acquires-protocolsio-driving-open-research/25844774); [run records/scaling](https://www.nature.com/articles/s41596-024-01012-z); [original platform paper](https://journals.plos.org/plosbiology/article?id=10.1371/journal.pbio.1002538); [public-content licence](https://www.protocols.io/we-enter-protocols).
20. **Rundeck/PagerDuty:** [Editions/licensing](https://docs.rundeck.com/docs/about/introduction.html); [job schema](https://docs.rundeck.com/docs/manual/document-format-reference/job-yaml-v12.html); [options/concurrency/retries](https://docs.rundeck.com/docs/manual/jobs/creating-jobs.html); [data handoff](https://docs.rundeck.com/docs/manual/jobs/job-variables.html); [commercial conditionals](https://docs.rundeck.com/docs/manual/jobs/conditional-logic.html); [commercial resume](https://docs.rundeck.com/docs/manual/jobs/job-resume.html); [history cleanup](https://docs.rundeck.com/docs/manual/projects/execution-history-cleaner.html); [cloud](https://docs.rundeck.com/docs/about/cloud/).
21. **StackStorm:** [Repository/licence](https://github.com/StackStorm/st2); [Orquesta language](https://docs.stackstorm.com/orquesta/languages/orquesta.html); [pause/resume](https://docs.stackstorm.com/orquesta/operations.html); [human inquiries](https://docs.stackstorm.com/inquiries.html); [MongoDB deployment](https://docs.stackstorm.com/latest/reference/ha.html); [history purging](https://docs.stackstorm.com/troubleshooting/purging_old_data.html); [Mistral retirement](https://docs.stackstorm.com/latest/workflows.html).
22. **AWS SSM:** [Actions/retries/timeouts](https://docs.aws.amazon.com/systems-manager/latest/userguide/automation-actions.html); [loops](https://docs.aws.amazon.com/systems-manager/latest/userguide/automation-action-loop.html); [approval](https://docs.aws.amazon.com/systems-manager/latest/userguide/automation-action-approve.html); [inputs/outputs](https://docs.aws.amazon.com/systems-manager/latest/userguide/visual-designer-action-inputs-outputs.html); [quotas/history retention](https://docs.aws.amazon.com/general/latest/gr/ssm.html).
23. **Ansible:** [Repository/licence](https://github.com/ansible/ansible); [variables](https://docs.ansible.com/projects/ansible/latest/playbook_guide/playbooks_variables.html); [loops/retries](https://docs.ansible.com/projects/ansible/latest/playbook_guide/playbooks_loops.html); [file hashes](https://docs.ansible.com/projects/ansible/latest/collections/ansible/builtin/stat_module.html); [assertions](https://docs.ansible.com/projects/ansible/latest/collections/ansible/builtin/assert_module.html); [pause](https://docs.ansible.com/projects/ansible/latest/collections/ansible/builtin/pause_module.html); [CLI](https://docs.ansible.com/projects/ansible/latest/cli/ansible-playbook.html).
24. **Process Street:** [Terms](https://www.process.st/terms/); [conditional logic/stop tasks](https://www.process.st/help/docs/getting-started-optimize-your-processes/); [approval/rejection](https://www.process.st/help/docs/approvals/); [API](https://www.process.st/help/docs/process-street-api/); [run deletion](https://www.process.st/help/docs/delete-workflow-runs/); [restoration](https://www.process.st/help/docs/recover-deleted-workflows-and-runs-2/).
25. **Manifestly:** [Operator/terms](https://www.manifest.ly/tos); [workflow/run model](https://help.manifest.ly/article/43-manifestly-checklists-101); [conditions](https://help.manifest.ly/article/66-conditional-logic-in-workflows); [approvals](https://www.manifest.ly/features/workflow-approvals); [API/webhooks](https://www.manifest.ly/integrations/using-webhooks-api-integrate-checklists); [delete/archive](https://help.manifest.ly/article/107-how-do-i-delete-or-archive-a-checklist-run).
26. **Strands SOPs:** [Repository/licence/instruction-based progress](https://github.com/strands-agents/agent-sop); [AWS introduction](https://aws.amazon.com/blogs/opensource/introducing-strands-agent-sops-natural-language-workflows-for-ai-agents/).
27. **First-hand problem reports:** [DVC workspace locking](https://discuss.dvc.org/t/true-parallel-dvc-repro-of-independent-branches-on-shared-workspace-databricks-rwlock-prevents-concurrent-execution/2706); [Orquesta retry scope](https://github.com/StackStorm/orquesta/issues/201); [AI Scientist v3 drafting resume](https://github.com/SakanaAI/AI-Scientist-v3/issues/2); [University of Edinburgh protocols.io subscription notice](https://library.ed.ac.uk/research-support/research-data-service/during/open-research-tools/protocols).
