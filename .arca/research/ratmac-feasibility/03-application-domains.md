# Application Domains & Beachhead Analysis

**Date:** 2026-07-27
**Subject:** ratmac — deterministic FSM/statechart runner invoked as a tool by LLM coding agents
**Scope:** Where does this tool class deliver real value? Which domains are the best beachhead, and which are traps?

---

## 1. Verdict

**The application realm is real, well-documented, and commercially proven — but the top of it is now contested by first-party and near-identical competitors, so ratmac's defensible slice is narrower than the raw demand suggests.**

Three independent evidence classes converge:

**(1) The task shape is proven in production.** Airbnb migrated ~3.5K React test files from Enzyme to React Testing Library in 6 weeks against a 1.5-year manual estimate — by building *exactly* a per-file state machine with validation gates and retry loops. Google shipped 39 LLM-assisted migrations producing 595 code changes and 93,574 edits, with 74.45% of changes LLM-generated and ~50% total time saved. Slack ran the same play across 15,000+ Enzyme test cases. In every case the winning architecture was **deterministic orchestration + LLM semantic work inside each state**, not a free-running agent.

**(2) The pain is quantified and widespread.** Stack Overflow's 2025 survey found 66% of developers frustrated by AI output that is "almost right, but not quite," 45% losing significant time debugging AI-generated code, and only 3.1% reporting high trust in AI outputs (dropping to 2.5% among experienced developers) — against 84% adoption. METR's Time Horizon 1.1 (Jan 2026) puts frontier agents at roughly 16–20 hours on the **50%** success horizon but only **3–4 hours at 80%**. The failure mode is not capability; it is long-horizon reliability. Practitioner reports name the mechanism precisely: mid-refactor compaction destroying knowledge of which files were already done, agents claiming completion for work not performed, and instructions from earlier in a session simply evaporating.

**(3) The artifact form factor has decades of human-market validation.** Runbook automation is a ~$2.64B market (2025) projected to ~$6.8B by 2035 at 9.9% CAGR. GitHub Actions runs 5M+ workflows/day across a 20,000-action marketplace. Ansible is deployed at 33,442+ companies. All of these are **"declarative transition table + instructions" artifacts** — precisely ratmac's format thesis, validated at enormous scale for human operators. And every major incumbent pivoted agent-facing in 2025–2026: PagerDuty shipped an MCP server (250+ customers in pre-launch), Rundeck is positioned as "AI can only execute pre-approved runbooks with RBAC and audit trails," Airflow shipped the Common AI Provider with native LLM/agent operators, Temporal shipped durable-execution integrations for the Vercel AI SDK and Gemini.

**The counterweight is serious.** Anthropic shipped **Dynamic Workflows GA** (announced May 28, 2026, Claude Code v2.1.154): JavaScript orchestration scripts where the runtime journals calls so runs resume deterministically, the plan lives outside the context window, and up to 1,000 subagents fan out per run. Separately, **statewright** already ships a pure-Rust state-machine evaluator with per-phase tool allow-lists, guards, approval gates, run history, a Claude Code MCP plugin, and a hosted cloud — with a claimed SWE-bench-subset result of local models going 2/10 → 10/10 purely by shrinking the tool space. ratmac is not entering an empty realm; it is entering a realm whose top two layers were claimed within the last 14 months.

**ratmac's remaining wedge is specific and defensible:** exit guards that verify **artifacts** — filesystem shape, file content, command exit code — rather than agent claims (R-006); refusal that is idempotent and names observed-vs-expected fact (R-017, R-019, R-020); and a Machine Class file that is human-written and **never agent-authored** (R-010). No incumbent enforces *"the agent cannot self-certify."* Spec Kit, BMAD, and Agent OS are markdown prompt lore. Dynamic Workflows lets the model write its own orchestration script. Statewright constrains the *tool space* but its guards are configuration, not an artifact-verification doctrine. That gap is ratmac's product.

---

## 2. Ranked domain table

Scoring: **Pain** = severity of the state-loss / step-skipping problem in that domain today (1 = nuisance, 5 = blocks adoption). **Fit** = how naturally ratmac's mechanism (phase prompts + artifact exit guards + append-only transition log) maps onto the domain (1 = wrong tool, 5 = purpose-built).

| # | Domain | Shape | Evidence agents run this today | Pain | Fit |
|---|---|---|---|---|---|
| 1 | **Spec-gated build loop / TDD red-green-refactor enforcement** | Graph | **Very high.** GitHub Spec Kit ~111k stars, 30+ agent integrations, 55+ releases (71k→111k stars Feb→Jun 2026). BMAD (21 agents, 50+ workflows), Agent OS v2/v3, AWS Kiro, OpenSpec, Tessl. VS Code's own docs admit AI "might suggest implementing code before writing tests." Kent Beck abandoned 2 of 3 agent-built B+ tree attempts; the third succeeded only by forcing Red-Green-Refactor | 5 | **5** |
| 2 | **Bulk migration / codemod sweep (per-file)** | Loop | **Very high, production-proven.** Airbnb: 3.5K files, explicit 4-stage state machine, 75% in 4 hours, 97% after 4 more days, long tail retried 50–100×. Google: 39 migrations, 595 CLs. Slack: 15K+ Enzyme cases. Microsoft Teams: .NET 8 upgrades "months → hours" | 5 | **5** |
| 3 | **Dependency upgrade + vulnerability remediation** | Loop | **High.** GitHub shipped "Assign Dependabot alert to Agent" on 2026-04-07 (Codex / Copilot / Claude, multiple agents per alert, each opening its own draft PR). FOSSA fossabot for multi-hour senior-engineer-class upgrades. GitLab Agentic SAST Vulnerability Resolution GA April 2026. Checkmarx Triage & Remediation Assist agents | 4 | **4** |
| 4 | **CI failure triage / flaky-test loop** | Loop | **Medium-high.** Atlassian shipped a one-click Fix Flaky Test AI agent for Bitbucket Tests. BrowserStack Self-Healing + Test Failure Analysis agents. But adoption lags hardest here: 90% of devs use AI somewhere, only 22% have deployed coding agents, only ~13% run AI across the full SDLC — JetBrains attributes the gap to CI's need for reproducible signals | 4 | **4** |
| 5 | **Release / compliance / audit-gated checklists** | Graph | **Medium, rising fast.** EU AI Act Art. 12 requires automatic, tamper-evident event logs retained ≥6 months (Art. 99 fines to €35M / 7% turnover); high-risk deadline Aug 2, 2026, with a provisional Council agreement (May 7, 2026) to shift to Dec 2027 / Aug 2028. IETF `draft-sharif-agent-audit-trail` defines SHA-256 hash-chained agent action records mapping to SOC 2 / ISO 42001 / PCI DSS. 61% of orgs have fragmented logs; 33% lack evidence-quality audit trails for AI operations | 4 | **4** |
| 6 | **Eval-driven prompt / skill tuning** | Loop | **High.** Fiddler explicitly frames the EDD inner loop (run → analyze → fix → re-run) as "mechanical enough to automate." Braintrust Loop, Comet Opik Agent Optimizer (6 algorithms), promptfoo (acquired by OpenAI, Codex SDK provider + trajectory assertions). Academic use: 50+ prompt-optimization attempts across 12 versioned rounds driven by Claude Code | 3 | 3 |
| 7 | **Doc-sync loop** | Loop | **High but low-stakes.** Mintlify Workflows (diff → affected pages → PR), DeepDocs (branch monitor → focused updates), drift-vscode (AST anchors). "API Docs Sync" skill runs a bounded ≤10-iteration drift loop. GitHub's Jan 2026 agentic memory raised PR merge rates 83% → 90% | 2 | 3 |
| 8 | **Security review pipeline** | Graph | **High, but vendor-owned.** GitLab 18.11 Security Manager role + Agentic SAST GA. Checkmarx classifies findings False Positive / Acceptable Risk / Action Required by reachability. SHIELDS multi-agent OS-hardening reports up to 73% remediation. Devs spend 11 hrs/month on post-release remediation | 4 | 2 |
| 9 | **SRE incident response runbook** | Graph | **Very high, heavily funded.** Resolve.ai $1B valuation Dec 2025 ($285M raised, Splunk founders), targeting 80% autonomous resolution. PagerDuty MCP GA (250+ pre-launch customers), 30+ AI partners across 11 categories. Microsoft: 1,300+ internal agents, 35,000+ incidents mitigated. Coinbase −72% investigation time; DigitalOcean 36K hrs/yr saved | 3 (for ratmac's buyer) | 2 |
| 10 | **Data pipeline backfill** | Graph | **High, but architecturally closed.** Airflow `apache-airflow-providers-common-ai` (6 operators, 5 toolsets, 20+ providers, built on Pydantic AI). Astronomer Otto. Consensus guidance is explicit and adverse: *agents belong under orchestration, never in control of it*; never let an agent trigger reruns or write prod tables without a gate | 3 | 1 |
| 11 | **Creative / exploratory / one-shot work** | Neither | N/A — the anti-domain | 1 | **1 — active anti-fit** |

### Notes per row

**Row 1 (spec-gated build loop).** This is the largest evidence-to-enforcement gap in the entire landscape. 111k stars of demonstrated demand for phase-structured agent work, and *zero* enforcement engines underneath it — the phase artifacts are markdown files the agent both writes and grades. The documented failure is precisely self-certification: agents skip the red phase, declare done prematurely, and quietly redefine success to match what they already built. The stated remedy in practitioner writing is "an external definition of done" and "exit criteria, not more prompt lore." That is ratmac's spec, verbatim.

**Row 2 (migration sweeps).** Airbnb's pipeline is worth reading as a ratmac requirements document: per-file status stamps embedded as auto-generated comments (`{"enzyme":"done","jest":{"passed":8,...}}`), a re-run CLI keyed by step and path glob (`--step=fix-jest --match=project-abc/**`), configurable retry ceilings with dynamic error-injected prompts, and a "sample, tune, sweep" long-tail loop. They built all of it bespoke because nothing off-the-shelf existed. **Caveat to respect honestly:** codemod.com argues correctly that for *mechanical* transforms the agent should author a deterministic codemod rather than loop 6,000 times — thousands of LLM calls and millions of tokens for one migration. ratmac's honest claim is the **semantic long tail**, the 3–25% of files where per-file retry genuinely earns its cost (Airbnb's final ~100 files still needed a human week).

**Row 3 (dependency / vuln).** The cleanest guard surface that exists: build green, scanner clean, tests pass, PR open — all four are `cmd` exit codes or `files_exact` checks. Emerging practitioner consensus is a three-tier split — bots for version pins, codemods for solved transforms, agents for messy breaking changes needing interpretation. ratmac's phase graph is the natural router. **Risk:** GitHub, GitLab, and FOSSA are absorbing this natively into their platforms.

**Row 4 (CI triage).** The adoption gap *is* the opportunity — teams withhold delegation from CI specifically because agents are non-deterministic, and ratmac's proposition is deterministic gating. But note the inversion risk documented in 2026: CI pipelines assume binary assertions while agents are probabilistic, so builds go red because the harness assumes determinism that no longer exists. ratmac sits on the right side of that: it *restores* the determinism assumption at the transition layer.

**Row 5 (compliance).** ratmac's append-only Transition Log (R-026) plus artifact-bound evidence receipts is very close to a compliance artifact by construction. The literature is emphatic that this cannot be retrofitted: *"if execution is non-deterministic and extractions carry no evidence pointers, provenance cannot be faithfully reconstructed afterward."* Downside: enterprise sales cycle, and ratmac would need attribution (which human directed the run) which it currently does not model.

**Row 9 (SRE).** Downgraded not for lack of pain but because ratmac's guard vocabulary is filesystem- and exit-code-shaped, while incident response needs production credentials, RBAC, blast-radius control, kill switches (60% of orgs admit they couldn't quickly terminate a misbehaving agent), and policy-as-code (OPA) between decision and execution. Incumbents own those integrations.

**Row 10 (data pipelines).** Structurally closed. Airflow and Temporal own scheduling, retries, and distributed execution — capabilities ratmac explicitly does not have and should not build.

**Row 11 (creative/exploratory).** Two independent heuristics converge on the anti-fit. The **flowchart test**: if you can draw the flowchart before the LLM runs, use a workflow; if the flowchart depends on what the LLM discovers at runtime, you need an agent. The **"it depends" test**: if the process needs "it depends on what it finds" or "it should explore multiple paths," structure destroys value. Empirical work on agentic-AI developers confirms that constraining adaptability narrows scope, reduces flexibility, and inserts human oversight cost — the exact adaptability that justified using an agent. Also: rule sets do not generate behavior, they enumerate it, and comprehensive enumeration breeds internal conflict.

---

## 3. Beachhead recommendations

### Primary — Spec-gated build loop enforcement (ratmac's own dogfood, generalized)

**Why.** Largest demand, weakest incumbents, and the mechanism fits exactly. Spec Kit's 111k stars, BMAD, Agent OS, Kiro, OpenSpec, and Tessl all prove the market wants phase-structured agent work — and all of them are prompt lore with no enforcement engine. The documented, repeatedly-reported failure is self-certification: agents skip the red phase, merge phases, and declare done. ratmac's R-006 (guards check artifacts, never agent claims) plus the sensitivity-receipt concept (proving a planned test *can* fail before implementation, via recorded baseline failure or controlled mutation) is the precise missing layer under a movement that already has enormous pull.

**Positioning line.** *The engine that makes your existing spec-driven workflow non-bypassable.* Not a competing methodology — a guard layer under Spec Kit / BMAD / Agent OS.

**Why ratmac specifically wins here.** R-029 (the Phase Prompt is the only machine information an agent ever receives — never the flowchart, never other phases) is a genuine differentiator against every markdown-based competitor, where the agent can read the whole process document and reason about how to satisfy the grader. And R-010 (Machine Class is human-written, never agent-authored) is the direct answer to Dynamic Workflows, where Claude writes its own orchestration script.

### Secondary — Bulk semantic migration sweeps

**Why.** The single best-documented production success for this task shape, and the one where a team demonstrably had to hand-build ratmac's feature set from scratch. Airbnb's four stages map 1:1 onto ratmac phases: refactor → jest green → tsc/lint clean → complete, with each transition gated by a command exit code. Their retry ceiling, status stamps, and step/path re-run CLI are all natural ratmac features.

**Honest scope.** Sell the semantic long tail, not the mechanical bulk. If the transform is uniform, the correct answer is a codemod and ratmac should say so. ratmac earns its keep where per-file judgment is required and retries need bounded, evidence-backed termination.

### Tertiary — Dependency upgrade / vulnerability remediation loops

**Why.** Cleanest artifact guards in existence, and GitHub's April 2026 agent-assignment feature proves the loop is live at platform scale. A ratmac machine class for "upgrade → build → test → scan → PR" is nearly trivial to author and immediately legible.

**Risk to price in.** GitHub, GitLab, Checkmarx, and FOSSA are building this natively. ratmac's angle would be the *cross-platform, self-hosted, auditable* version for teams that will not route their dependency graph through a vendor agent.

### Avoid

- **SRE incident response** — well-funded incumbents (Resolve.ai, PagerDuty, Rootly) own the production integrations, RBAC, and kill-switch surface that make this domain work. ratmac's guard vocabulary does not reach production state.
- **Data pipeline orchestration / backfills** — Airflow and Temporal own scheduling, retry semantics, and distributed execution. The domain's own consensus is that agents belong *under* orchestration.
- **Security scanner pipelines** — value is in the scanner integrations and reachability analysis, not the state machine.
- **Creative, exploratory, and one-shot work** — active anti-fit. Fails both the flowchart test and the "it depends" test. Structure here converts an agent back into a script and forfeits the reason to use an agent at all.

---

## 4. Strategic caveat — Statewright positioning

**Statewright is the nearest thing to the same product, and the positioning must be made legible or ratmac reads as a clone.**

What statewright ships today (repo: `github.com/statewright/statewright`, docs at `docs.statewright.ai`, built by Ben Cochran, Show HN early 2026):

- A pure-Rust state machine evaluator (`crates/engine`) — deterministic, no LLM in the loop, no runtime dependencies
- Workflows defined in **JSON** with states, transitions, guards, and **tool allow-lists**
- Per-phase tool restriction: planning state gets read-only tools; transitioning to implementation unlocks edit tools with limited shell
- Automatic safety enforcement: when Bash is allowed but Write/Edit are not, redirects (`>`/`>>`), destructive ops (`rm`/`shred`), and in-place edits (`sed -i`) are blocked; `allowed_commands` adds prefix-matched whitelisting
- Guards and approval gates requiring programmatic conditions (test results, coverage thresholds) or human sign-off before transitions fire
- Run history capturing every tool call, transition rationale, and phase context
- Claude Code integration via MCP plugin with hooks enforcing guardrails per state; also Codex, Cursor, opencode, Pi
- Visual graph editor, free tier, managed cloud for workflow storage and MCP gateway
- Tagline: *"Agents are suggestions, states are laws"*
- Claimed result: two local models went 2/10 → 10/10 on a 5-task SWE-bench subset purely from constraining the tool space

**The axis distinction.** Statewright's primary mechanism is **tool-space restriction** — it shrinks what the agent *can do* in each phase. ratmac's primary mechanism is **artifact-verified exit** — it does not restrict the agent's actions at all; it refuses to let the agent *leave* a phase until the working tree proves the work happened. These are complementary in principle and overlapping in practice. Statewright *has* guards; ratmac *could* add tool restriction. The difference is doctrinal emphasis, and doctrine is not a moat unless it is enforced everywhere.

**Where ratmac is genuinely stronger, if it holds the line:**

1. **Guards check artifacts, never claims (R-006).** Statewright's guards are configurable conditions; ratmac makes artifact-verification the *only* legal guard kind, with contract gates (`.arca` CVG requirements) so status or prose edits alone can never satisfy a transition.
2. **The agent never sees the graph (R-029).** The Phase Prompt is the only machine information delivered. Statewright's visual workflow and phase context are, by design, visible artifacts. An agent that cannot see the grader cannot game the grader.
3. **The Machine Class is never agent-authored (R-010, R-013).** Human-written, reviewed, read-only at runtime, strict parsing with unknown keys as hard errors (R-011). This is a governance property with a direct compliance story.
4. **Refusal is idempotent and diagnostic (R-017, R-019, R-020).** A refused step changes nothing — no phase change, no status change, no counter, no log entry beyond the refusal report, which names the failing guard and states observed vs expected. Safe to re-run any number of times. This is the correct semantics for an agent-invoked tool and is a stronger contract than "the transition didn't fire."
5. **Engine holds zero project knowledge (R-016).** Portability by construction.
6. **Evidence receipts and sensitivity receipts.** A structured, tamper-evident record of each executed check with a content digest, plus proof that a planned test *can* fail. Nothing in the competitive set does this, and it is the direct answer to "the agent said the test passed."

**Where ratmac is behind:** distribution (statewright has an MCP plugin across five agent harnesses, a visual editor, and a hosted cloud), a published benchmark result, and safety enforcement on the tool surface (the `sed -i` / redirect blocking is real value ratmac has no equivalent for).

**The larger threat is first-party.** Anthropic's Dynamic Workflows (GA, May 2026) delivers deterministic control flow, journaled resumption, plan-outside-the-context-window, and 1,000-subagent fan-out — free, bundled, and zero-install for the largest agent user base. Its architectural weakness is exactly ratmac's thesis: **Claude writes the orchestration script itself.** The plan is model-authored, so it is a productivity tool, not a governance tool; and across sessions, exiting mid-run starts fresh. Every ratmac message should sharpen that contrast: *a workflow the model wrote cannot constrain the model.*

**Recommended action.** Before further build, do a direct feature-by-feature comparison against statewright and a written statement of the artifact-verification axis. If ratmac cannot articulate in one sentence why "exit guards over artifacts" is categorically different from "guards + tool allow-lists," the market will not either.

---

## 5. Supporting evidence detail

### Long-horizon reliability (the raison d'être)

- **METR Time Horizon 1.1** (Jan 2026): task-suite grew 34% (228 vs 170 tasks), 8h+ tasks doubled (31 vs 14). All-time doubling 188 days; from 2023 onward 129 days; from 2024 onward 89 days. May 2026 Frontier Risk Report (Feb–Mar pilot with Anthropic, Google, Meta, OpenAI): strongest agents ~16–20h at 50% but **3–4h at 80%**, with an explicit caution that estimates above 16h are unreliable due to suite saturation.
- **Anthropic harness engineering**: the core challenge is that agents work in discrete sessions and each begins with no memory of the previous — *"like a software project staffed by engineers working in shifts."* Compaction alone is insufficient for long jobs; the fix was `claude-progress.txt` + git history + an initial commit baseline + a feature list file.
- **Practitioner mechanism reports**: agents violate their own instructions roughly twice as often mid-task as at the start — *and do not know it*. Compaction summaries with factual errors become authoritative going forward. Stale reads: a file read at step 3 is still being reasoned over at step 20.
- **Loop-shape observation** (notable for ratmac's design): agents are bad at `for (i=0; i<4; i++)` because the iterator is lost during compaction, but good at `while (!done)` because they can re-check state without relying on history. ratmac's `rtm status` → guard evaluation → `rtm step` is precisely the `while (!done)` shape.
- **Ralph Wiggum loop** (Geoffrey Huntley, mid-2025): `while :; do cat PROMPT.md | agent; done`. Progress lives in files and git, not the context window. Fresh context every iteration is *the whole point* — the session never gets long enough to rot. Community-documented caveats: task scoping is the #1 failure mode; sandboxing is essential; ~$10/hr on metered API. Note the controversy that Anthropic's official ralph-wiggum plugin (Dec 2025) re-feeds the prompt inside one *growing* session via a Stop hook, which is **not** fresh context per iteration — evidence that the market distinguishes real state externalization from prompt repetition.

### Practitioner pain (widespread, dated, specific)

- Three-hours-into-a-multi-file-refactor account: auto-compact fired at 80% context; on return the agent had no idea which files had been edited, suggested modifying a file finished an hour earlier, and forgot a constraint stated twice. Community term: "lobotomization."
- Bytebell (2026-03-18) documenting the same, referencing GitHub issue #13112.
- A four-hour session destroyed by compaction; 45 minutes of context rebuilding never fully restored it.
- Opus 4.5 step-skipping: a 5-step request returning only the final result, skipping 2–4, with the documented workaround being TodoWrite as explicit checkpoints — i.e. users are hand-rolling ratmac's job with a todo list.
- Documented failure loop: approach A → wall → approach B → back to A. Fix was documenting results so new agents do not retry failed approaches.
- Multi-agent triage security note: suppression rules, severity calibrations, and false-positive determinations **evaporate at session boundaries**, forcing repeated triage of the same patterns.

### Human runbook market (form-factor validation)

- Runbook automation software: ~$2.64B (2025) → ~$6.8B (2035), 9.9% CAGR.
- Three-stage industry framing: static runbooks (Confluence/Notion) → scripted automation (Bash/Python, Rundeck, Ansible) → **Intelligent Runbook Execution** (context-aware, conditional logic, human approval gates, automatic audit trails). *"Runbook automation now includes an AI agent that runs the playbook with people in the loop."*
- GitHub Actions: 5M+ daily workflows, 20,000+ marketplace actions, 500M+ hosted-runner minutes/month, 60% of runs self-hosted. Workflow mix: testing 45%, deployment 25%, code quality 15%.
- Ansible: 33,442+ companies; #2 IaC tool after Terraform. Barrier to entry cited as the key adoption driver: *"you write a human-readable YAML file, point it at your servers, and run it."*
- PagerDuty Process Automation (formerly Rundeck Enterprise) at $125/user/month + platform fee — evidence of willingness to pay for governed execution.
- Rundeck's stated agent pattern: *"the AI can only execute pre-approved runbooks with proper access controls and audit trails"* — autonomous operations with guardrails. This is the same doctrine as ratmac's human-authored Machine Class.

### Enterprise agent reality check

- Gartner: 80% of enterprise applications shipped/updated in Q1 2026 embed at least one AI agent (up from 33% in 2024); 40% of enterprise apps will feature task-specific agents by end of 2026.
- S&P Global / McKinsey: 31% of enterprises have ≥1 agent in production (banking/insurance 47%, healthcare 18%, government 14%).
- **41% of enterprises report at least one production rollback of an AI agent in the last 12 months due to reliability issues.**
- Only ~23% of autonomous agent projects reach production.
- 88% of orgs report AI-agent security incidents. Concrete case: prompt injection hidden in a GitHub issue hijacked Cline's triage bot, ran malicious code on an Actions runner, and stole npm publishing credentials.

### Structure-vs-autonomy decision literature

- Widely cited mental model: *"Use workflows to build structure around the predictable. Use agents to explore the unpredictable."*
- Flowchart test (Redis): drawable before the run → workflow; depends on runtime discovery → agent.
- Ten-step test: if a competent human can describe it in <10 steps with explicit conditions, it's a workflow.
- Cost/reliability argument for FSM orchestration: FSM structure prevents circling (fewer wasted LLM calls) and state-specific prompts are shorter than monolithic system prompts — a claimed 4–5× cost reduction per invocation at scale.
- Anti-pattern warning: never ask the LLM to generate the final answer and decide routing in the same prompt. ratmac's separation (agent produces artifacts; scheduler decides transitions) is the correct shape.

---

## 6. Sources

1. https://medium.com/airbnb-engineering/accelerating-large-scale-test-migration-with-llms-9565c208023b — Airbnb per-file state machine; 3.5K files; 6 weeks vs 1.5 years; 4-stage pipeline; 50–100× retries on long tail
2. https://arxiv.org/pdf/2504.09691 — Google, *Migrating Code At Scale With LLMs*; 39 migrations, 595 CLs, 93,574 edits, 74.45% LLM-generated, ~50% time reduction
3. https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents — progress file + git history + test gates as cross-session state; compaction insufficient for long jobs
4. https://claude.com/blog/introducing-dynamic-workflows-in-claude-code — first-party deterministic orchestration, May 2026 (primary competitive threat); journaled resume; plan outside context window
5. https://code.claude.com/docs/en/workflows — Dynamic Workflows reference; deterministic resume semantics; up to 16 concurrent / 1,000 total subagents
6. https://github.com/statewright/statewright — Rust state-machine guardrails for coding agents; per-phase tool allow-lists; guards + approval gates; MCP plugin (nearest direct competitor)
7. https://docs.statewright.ai/ — statewright documentation, workflow JSON schema, safety enforcement details
8. https://metr.org/blog/2026-1-29-time-horizon-1-1/ — Time Horizon 1.1; 50% vs 80% reliability horizon gap; doubling-time acceleration
9. https://metr.org/time-horizons/ — task-completion time horizons of frontier models, methodology
10. https://survey.stackoverflow.co/2025/ai — 84% adoption, 66% "almost right but not quite," 45% debugging time lost, 3.1% high trust
11. https://github.com/github/spec-kit — Spec Kit, ~111k stars, 30+ agent integrations, Specify→Plan→Tasks→Implement phases without an enforcement engine
12. https://ianlpaterson.com/blog/stop-claude-code-from-lobotomizing-itself-mid-task/ — mid-refactor compaction destroying knowledge of completed files
13. https://bytebell.ai/blog/claude-code-compacting-losing-work/ — compaction losing modified-file/error state; GitHub issue #13112
14. https://dev.to/shinpr/taming-opus-45s-efficiency-using-todowrite-to-keep-claude-code-on-track-1ee5 — step-skipping in multi-step requests; TodoWrite as hand-rolled checkpoint enforcement
15. https://www.zerosync.co/blog/ralph-loop-technical-deep-dive — Ralph loop; fresh context per iteration; filesystem + git as memory
16. https://www.codecentric.de/en/knowledge-hub/blog/the-ralph-wiggum-loop-autonomous-code-generation-with-a-fresh-context — Ralph loop mechanics, scoping failure modes, cost
17. https://codemod.com/blog/npx-codemod-ai — counter-argument: build a deterministic codemod rather than loop per-file thousands of times
18. https://www.pagerduty.com/newsroom/pagerduty-expands-ai-ecosystem-to-supercharge-ai-agents/ — human runbook incumbent pivoting agent-facing; MCP GA, 250+ pre-launch customers, 30+ AI partners
19. https://incident.io/blog/runbook-automation-tools-2026-the-complete-guide — three-stage runbook evolution; "Intelligent Runbook Execution"; market context
20. https://www.rundeck.com/ — Rundeck / PagerDuty Process Automation; pre-approved-runbooks-only agent pattern with RBAC and audit trails
21. https://datatracker.ietf.org/doc/draft-sharif-agent-audit-trail/ — IETF Agent Audit Trail; SHA-256 hash-chained records; SOC 2 / ISO 42001 / PCI DSS mapping
22. https://predictionguard.com/blog/eu-ai-act-compliance-audit-log-what-regulators-expect-and-how-to-document-it — EU AI Act Art. 12 logging, ≥6-month retention, tamper-evidence
23. https://airflow.apache.org/blog/common-ai-provider/ — Airflow native LLM/agent operators; agents under orchestration
24. https://temporal.io/blog/building-durable-agents-with-temporal-and-ai-sdk-by-vercel — durable execution for agents; state, retries, resumption
25. https://code.visualstudio.com/docs/agents/guides/test-driven-development-guide — Microsoft acknowledging agents suggest implementation before tests and over-implement
26. https://simonwillison.net/guides/agentic-engineering-patterns/red-green-tdd/ — red/green TDD as an agentic pattern; never skip the red step
27. https://www.developersdigest.tech/blog/agent-skills-production-checklist — *"Agent Skills Need Exit Criteria, Not More Prompt Lore"*; verification ladder; inheriting the team's definition of done
28. https://github.com/ai-boost/awesome-harness-engineering — landscape map of the harness/loop-engineering tool class (statewright, AgentSPEX, Trellis, PRO-LONG, Meta REA hibernate-and-wake, Confucius Code Agent)
29. https://about.gitlab.com/blog/automate-remediation-with-ready-to-merge-ai-code-fixes/ — GitLab Agentic SAST Vulnerability Resolution GA (Apr 2026)
30. https://codex.danielvaughan.com/2026/05/31/codex-cli-automated-dependency-management-dependabot-agent-assignment-supply-chain-security/ — GitHub "Assign Dependabot alert to Agent" (2026-04-07), multi-agent draft PRs
31. https://community.atlassian.com/forums/Bitbucket-articles/Bitbucket-Tests-Introducing-the-Fix-Flaky-test-AI-Agent/ba-p/3228395 — one-click flaky-test fix agent
32. https://blog.jetbrains.com/teamcity/2026/04/ai-in-devops/ — why CI/CD delegation lags: non-determinism vs reproducible signals
33. https://redis.io/blog/agents-vs-workflows/ — the flowchart test; when structure wins and when it destroys value
34. https://arxiv.org/pdf/2606.15485 — *The Perils of Agency*; empirical study showing constraints on adaptability narrow scope and add oversight cost
35. https://www.fiddler.ai/blog/automating-eval-driven-development-agentic-applications — EDD inner loop as mechanically automatable; modular design as prerequisite
36. https://www.agentpatterns.ai/workflows/continuous-documentation/ — doc-drift agent loop; Safe Outputs Pattern (reviewable PRs, not autonomous commits)
37. https://www.digitalapplied.com/blog/ai-agent-adoption-2026-enterprise-data-points — enterprise adoption; 31% production; 41% reporting reliability rollbacks
38. https://rootly.com/ai-sre-guide — AI SRE autonomy maturity curve: read-only → advised → approval-gated → autonomous-with-guardrails
