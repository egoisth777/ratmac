# StateM Close Read

**Date:** 2026-11-21
**Producing agent:** `statem-close-read`
**Method:** web research only, nothing installed or run
**Known defects:** None known.

---

## Repository confirmation

**Confirmed.** [henryqin1997/statem](https://github.com/henryqin1997/statem) exists and is public. The owner’s profile names **Ziheng Qin**. GitHub reports **1,605 stars** and **Apache-2.0**. These match the reported identity, approximate popularity, and license. [Repository metadata](https://api.github.com/repos/henryqin1997/statem), [owner](https://api.github.com/users/henryqin1997).

The reported operating model also matches: an outside caller requests a target state, and executable checks permit or block that request. One qualification: its bundled parser supports a **YAML subset**, plus JSON—not unrestricted YAML. [Transition implementation](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L399-L488), [parser](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/miniyaml.py#L1-L33).

Source findings below refer to **`51e4ae52b552d7e7fdd4383bc7377e421ea19967`**, the inspected `main` revision, committed November 10. This matters: it includes fixes absent from older issue reports. [Revision](https://github.com/henryqin1997/statem/commit/51e4ae52b552d7e7fdd4383bc7377e421ea19967).

**Evidence boundary:** verified means source or public-record inspection, not execution. Inferences and third-party reports are identified below.

## 1. Who triggers a move?

- The caller invokes `statem goto TARGET`, or its alias `transfer TARGET`. `next` only lists outgoing edges; it does not choose or execute one. [CLI](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/cli.py#L88-L98), [runtime](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L390-L408).
- The inspected core has no daemon, scheduling command, or timer-driven transition mechanism. Its interface consists of individual CLI calls. [Complete command registration](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/cli.py#L46-L155).
- An optional Codex/Claude **Stop hook** asks the host to continue an unfinished agent turn. It calls `cur`, then returns a continuation prompt. It **does not call `goto`**. It allows stopping for its recursion guard, missing/unreadable runs, designated stop states, or no outgoing edges. [Hook implementation](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/hooks/statem_stop_hook.py#L20-L62).

## 2. Program and model execution; hash pinning

**Core execution mechanisms:**

- **`command`:** runs a command string with `subprocess.run(..., shell=True)`. It inherits the environment, adds StateM context, supports `cwd` and optional `timeout`, captures stdout/stderr, and passes on exit code zero. [Implementation](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L812-L839).
- **`llm_review`:** executes `run`, or `STATEM_LLM_REVIEW_CMD`, through the shell. It normally sends a generated review prompt on stdin. Success requires exit zero and any configured `accept_contains`, `reject_contains`, `accept_regex`, or `reject_regex` checks. The core delegates model access to that external command. **If no command is configured, it falls back to manual confirmation; `--yes` can therefore approve that fallback.** [Implementation](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L841-L890).
- These executable types can appear in node `in_hook`, `before_transfer`, `out_hook`, edge `condition` and `hook`, and current-entry dynamic checks. `start`, including resume, runs `in_hook`; `goto` runs the transition stages; `save` runs `out_hook` unless `--skip-hooks` is supplied. Check lists execute sequentially. [Startup](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L303-L324), [transition/save](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L399-L560), [dispatch](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L733-L762).

**Hash boundary:** the core pins the **runbook text**, not executable binaries, referenced scripts, model weights, or provider responses. Its command/reviewer schema contains no executable-hash field. Environment-selected reviewer commands are also outside the runbook hash. [Schema](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L70-L104), [execution](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L812-L890), [runbook comparison](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1052-L1086).

**Optional integrations are separate:**

- The Stop hook launches the StateM CLI using a shell-free argument list. [Source](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/hooks/statem_stop_hook.py#L77-L132).
- The Harbor adapter starts StateM, then delegates agent execution to Harbor’s Codex adapter; its DeepSeek variant supplies provider configuration. It pins Codex **by version**, `0.146.0`, and records a nine-file SHA-256 source manifest. [Version](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/harbor/statem_codex_git_webserver_deploy.py#L25-L64), [manifest](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/harbor/statem_codex_git_webserver_deploy.py#L193-L236), [agent launch](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/harbor/statem_codex_git_webserver_deploy.py#L412-L436), [DeepSeek](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/harbor/statem_codex_git_webserver_deploy.py#L516-L568).
- The example Git/webserver helper executes Git/SSH-related programs and HTTP probes. Its receipt checks its **own implementation hash** when validating reused evidence. That is example-specific, not universal executable pinning. [Processes](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/harbor/experimental/git_webserver_deploy_family.py#L418-L540), [receipt validation](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/harbor/experimental/git_webserver_deploy_family.py#L298-L387).

## 3. Full list of check kinds

Exactly six types are registered:

- `message`: displays text; always returns a passing result.
- `manual`: asks for yes/no confirmation.
- `checklist`: asks for confirmation of listed items.
- `command`: shell command judged by exit status.
- `predicate`: declarative filesystem/content checks.
- `llm_review`: external reviewer command, with the manual fallback described above.

These types are shared across hooks, conditions, and dynamic checks. [Registry/schema](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L29-L104), [dispatch](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L744-L804).

Predicate operations are `exists`, `non_empty`, `contains`, `not_contains`, `matches`, and `json_path`, optionally followed by `equals` or `one_of`. A bare `path` means `exists: true`. `equals`/`one_of` require `json_path`. [Execution](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L892-L934), [validation](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1452-L1483).

A plain string hook or condition becomes **`manual`**, not an automatically evaluated language-model instruction. [Normalization](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1404-L1427).

## 4. Branching and failed-check routing

- The caller supplies the target. StateM finds the corresponding outgoing edge. Duplicate source/target edges are rejected as ambiguous. It does not search for whichever condition passes. [Edge validation/resolution](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L167-L203).
- Checks exist **on both states and transitions**: source `before_transfer` and dynamic checks, followed by the selected edge’s `condition`. [Transition](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L399-L430).
- `on_failure` supports only **`block`** or **`continue`**. It cannot name another state. `continue` permits the originally requested transition; it does not select a repair branch. [Validation](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1479-L1483), [failure handling](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1690-L1729).
- Pre-leave failure retains the source. A failed target `in_hook` is different: the target pointer was already persisted. [Transition stages](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L430-L478).

**Inference:** a mandatory source-state gate also applies when requesting a repair edge. Merely adding another outgoing edge does not escape that gate. [Shared source checks](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L408-L430).

## 5. Loops and bounds

- Cycles are supported. The included coding runbook explicitly returns from `review` to `execute`, and through `session_refresh` to `plan`. [Example](https://github.com/henryqin1997/statem/blob/51e4ae5/examples/coding-agent.yaml#L94-L121).
- Optional edge `max_attempts` must be a positive integer. It counts attempts for **that edge and source-entry ID**, including blocked attempts. Exhaustion blocks before checks run. A fresh source entry has a fresh budget. [Counter](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L490-L539), [validation](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1220-L1226).
- Without `max_attempts`, retries are unbounded. There is no core run-wide iteration/deadline field in the supported schema. Command/reviewer timeouts are optional. [Documentation](https://github.com/henryqin1997/statem/blob/51e4ae5/README.md#L207-L218), [schema](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L34-L104).

**Inference:** an edge-entry ceiling does not bound the total number of repair cycles, because re-entry refreshes that ceiling. The Harbor example’s deadline handling is separate from the core. [Counter](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L500-L539), [adapter deadline](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/harbor/statem_codex_git_webserver_deploy.py#L275-L301).

## 6. Data carried by a run

Persisted fields include `version`, `run_id`, `spec_path`, `spec_hash`, `strict_spec`, `current`, `current_entry_id`, timestamps, `history`, agent identity, and configured edge-attempt counters. [Initialization](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L224-L245), [counters](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L500-L537).

Check results retain type, purpose, pass/fail, failure policy, output, and usually process exit code. Dynamic payloads additionally carry producer identity/role, `basis`, and checks. These are records, not declared state-to-state output ports. [Results](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1690-L1713), [dynamic payload](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1577-L1592).

Hooks receive inherited environment variables plus:

`STATEM_RUN_ID`, `STATEM_SPEC`, `STATEM_STATE_DIR`, `STATEM_CURRENT`, `STATEM_PURPOSE`. [Environment](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1660-L1668).

There are no first-class run parameter declarations, mutable run-variable fields, or output-binding declarations in the supported schema. The design explicitly lists **run variables as future work** and recommends files as external memory. [Schema](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L34-L104), [design](https://github.com/henryqin1997/statem/blob/51e4ae5/design.md#L20-L25), [backlog](https://github.com/henryqin1997/statem/blob/51e4ae5/design.md#L437-L450).

## 7. Definition changes during a run: spec rebinding

- Runtime loads compare the current runbook hash with the stored hash. Missing or changed hashes block runtime commands before hooks/checks execute. [Loader](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1052-L1086).
- `statem start SPEC --run-id ID` explicitly accepts the new binding. It records `spec_rebind` with previous/new paths and hashes, then a `resume` event. Events receive timestamps. [Rebinding](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L247-L302), [event recording](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1757-L1758).
- If the current node survives, it remains current. If removed, startup moves to `initial`, creates an entry, and records `migrate_current`. Startup reruns the resulting node’s `in_hook`. [Source](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L273-L324).
- No signature, reviewer approval, or authorization check guards this command. The rebind event contains no authenticated approving identity. Strict mode validates keywords; once enabled for a run, it remains enabled across rebinds. [Startup implementation](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L210-L324).

Dynamic checks can change separately without rebinding the runbook. Each attempt reloads them and records their producer metadata and check snapshot. [Dynamic execution](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L936-L1050).

## 8. Child runs, parallel runs, joins, nesting

The inspected schema and CLI have **no native child-run, fork, join, nested-state-machine, or parallel-state construct**. Each run has one `current` pointer. Separate run IDs provide independent records; one `active_run` file supplies the default selection. [Schema](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L34-L104), [run creation](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L210-L245), [selection/storage](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1162-L1197).

Multiple agents can contribute dynamic-check files. They are merged with `merge: all`; this is not a child-run join. Checks execute sequentially. [Merge restriction](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1511-L1530), [check loading](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1007-L1050), [execution](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L738-L742).

**Inference:** external processes can drive different run IDs concurrently. Same-run concurrent-write safety is not established; persistence rewrites the state file without a locking step in that implementation. [Persistence](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1191-L1197).

## 9. Pause, approval, resume, abandon

There are no dedicated `pause`, `approve`, or `abandon` commands or persisted lifecycle statuses. **Inference:** withholding further CLI calls effectively pauses progress, but creates no explicit pause record. Resume uses `start`, which reruns the current entry hook. [CLI](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/cli.py#L69-L155), [resume](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L291-L324).

Approval means `manual`/`checklist` confirmation. Non-TTY input or EOF fails confirmation; `--yes` auto-confirms. There is no authenticated reviewer role or signature requirement. Producer roles are caller-supplied metadata. [Confirmation](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L764-L804), [role registration](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L586-L624).

The Claude integration includes a `statem-runbook-reviewer` agent prompt. It asks for findings; it does not implement an authorization or signing mechanism. [Reviewer definition](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/claude/statem/agents/statem-runbook-reviewer.md#L1-L16).

## 10. State/history files, paths, and hand-edit protection

Default layout:

- `.statem/active_run`: plain-text run ID.
- `.statem/runs/<run-id>/state.json`: JSON state, including the `history` array.
- `.statem/runs/<run-id>/nodes/<node>/<entry-id>/dynamic_before_transfer/manifest.json`.
- Adjacent `checks.<agent-id>.json` files.

`--state-dir` or `STATEM_STATE_DIR` changes the root. [CLI default](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/cli.py#L58-L67), [dynamic layout](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1118-L1142), [state layout](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1175-L1197).

State is written through `state.json.tmp` followed by replacement. History entries are ordinary timestamped JSON objects. **There is no history hash chain or signature.** The documentation explicitly says someone able to rewrite both runbook and runtime state can bypass hash pinning. [Writer/events](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1191-L1197), [event format](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1757-L1764), [trust boundary](https://github.com/henryqin1997/statem/blob/51e4ae5/README.md#L289-L293).

The runtime root and stored spec path are resolved to absolute paths. Dynamic manifests also record resolved paths. Relative check paths use the spec directory, or a check’s `cwd`; absolute paths are accepted. [Resolution](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L133-L138), [runtime root](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L205-L208), [check paths](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L1730-L1743).

## 11. Exit codes and machine-readable output

- **`0`:** command handler completed successfully. Successful `goto --json` includes `run_id`, `from`, `to`, `current`, `current_entry_id`, results, and configured attempt-budget fields.
- **`2`:** `TransitionBlocked`, including failed gates or exhausted attempts.
- **`1`:** `StatemError`, including invalid transitions, invalid specs, or hash mismatches.

Handled errors under `--json` contain `{"ok": false, "error": ...}` and optional `details`; transition details include `stage` and `results`. [Exception codes](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L19-L27), [CLI envelope](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/cli.py#L12-L44), [transition output](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L470-L488).

**Important exception:** exit `2` does **not always mean “still at source.”** A blocking target `in_hook` fails after the target is persisted. Startup can likewise create/resume a run before its entry hook blocks. [Transition](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L449-L478), [startup](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L303-L324).

**Inference from exception handling:** JSON is not a universal error envelope. Argument parsing occurs before the handler’s `try`, and that handler catches only `StatemError`. Unhandled exceptions remain outside it. [CLI](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/cli.py#L12-L38).

## 12. Terminal outcomes

The core has no built-in success/failure/cancelled outcome enum or terminal-node flag. It retains a current node; a node without outgoing edges simply offers no next move. [Schema/state](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L34-L58), [next states](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L390-L407).

The optional Stop hook treats `handoff`, `done`, `complete`, `completed`, and `finished` as stop names, or stops when no outgoing edge exists. Names are configurable through `STATEM_AUTOLOOP_STOP_STATES`. These are **host-hook conventions**, not core terminal outcomes. [Hook](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/hooks/statem_stop_hook.py#L20-L75).

The Harbor adapter separately enforces a configurable final-state name, defaulting to `handoff`. [Configuration](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/harbor/statem_codex_git_webserver_deploy.py#L45-L55), [enforcement](https://github.com/henryqin1997/statem/blob/51e4ae5/integrations/harbor/statem_codex_git_webserver_deploy.py#L508-L514).

## 13. Installation, maintenance, contributors, issues, roadmap

- **Install:** documented method is cloning the repository and running `python3 -m pip install -e .`. [README](https://github.com/henryqin1997/statem/blob/51e4ae5/README.md#L72-L86).
- **Language/dependencies:** Python 3.11+, package version `0.1.0`, no declared runtime dependencies; build requires `setuptools>=68`. [Package metadata](https://github.com/henryqin1997/statem/blob/51e4ae5/pyproject.toml#L1-L24).
- **Offline use — inference:** installed core operations and local checks can work offline. Installation may need build dependencies; configured commands/models and network-oriented example gates can require connectivity. No offline execution was performed. [Package metadata](https://github.com/henryqin1997/statem/blob/51e4ae5/pyproject.toml#L1-L24), [external execution](https://github.com/henryqin1997/statem/blob/51e4ae5/statem/core.py#L812-L890).
- **Release cadence:** the public API lists one release and one tag: the August 18 DeepSeek policy-v9 artifact release. Current source changes extend through November 10. A regular release cadence cannot be established from this record. [Releases](https://api.github.com/repos/henryqin1997/statem/releases?per_page=100), [tags](https://api.github.com/repos/henryqin1997/statem/tags?per_page=100), [latest inspected commit](https://github.com/henryqin1997/statem/commit/51e4ae52b552d7e7fdd4383bc7377e421ea19967).
- **Contributors:** GitHub returns four linked accounts: `kaiwang960112`, `henryqin1997`, `lyx1590916-prog`, and `Yaxin-Lu`. With anonymous authors included, there are two additional author records, both named `henryqin1997`. **Distinct human count is not established.** [Contributor API](https://api.github.com/repos/henryqin1997/statem/contributors?per_page=100&anon=true).
- **Open issues:** zero ordinary open issues; the single open item is PR #1. Its themes are a construction-report example, attempt budgets tied to origin identity, strict validation, and dry-run inspection. Its analysis describes an older revision; current source already has edge-entry limits and persistent strict mode. [Open items](https://api.github.com/repos/henryqin1997/statem/issues?state=open&per_page=100), [PR #1](https://github.com/henryqin1997/statem/pull/1).
- **Closed issue themes:** Windows/non-interactive behavior, permissive predicate/schema handling, and unenforced spec hashes. PRs #3 and #6 were merged to address these. [#2](https://github.com/henryqin1997/statem/issues/2), [#4](https://github.com/henryqin1997/statem/issues/4), [#5](https://github.com/henryqin1997/statem/issues/5), [#3](https://github.com/henryqin1997/statem/pull/3), [#6](https://github.com/henryqin1997/statem/pull/6).
- **Stated roadmap:** `design.md` lists v2 suggestions for an evidence cache, first-class run variables, compact history summaries, and sandbox-friendly integration/export modes. These are backlog directions, not dated commitments. [Backlog](https://github.com/henryqin1997/statem/blob/51e4ae5/design.md#L437-L450).

## 14. Public evidence of use

- **Authors’ research use:** the paper reports StateM experiments on Terminal-Bench and BusinessBench. The repository publishes evaluation artifacts. This establishes an author-reported research use, not independent reproduction. [Paper](https://arxiv.org/abs/2608.15089), [artifact release](https://github.com/henryqin1997/statem/releases/tag/deepseek-policy9-tb21-artifacts-20260818).
- **`seong1995`:** reports actually driving StateM from an agent host on Windows and supplies reproductions and fixes. This is evidence of reported hands-on use. [Issue #2](https://github.com/henryqin1997/statem/issues/2).
- **`p-owerscripter`:** reports writing evaluation gates and testing mid-run edits with a shell-enabled agent. This is evidence of reported evaluation use. [Issue #4](https://github.com/henryqin1997/statem/issues/4), [issue #5](https://github.com/henryqin1997/statem/issues/5).
- **Lucas Araujo / `qualiobra`:** describes an existing Brazilian construction-report production pipeline and proposes translating it into a StateM example. The PR explicitly says the proposed runbook was produced by static analysis and had not been validated. **It does not establish a production StateM deployment.** [PR #1](https://github.com/henryqin1997/statem/pull/1).
- **`agents.nix`:** merged automated packaging of StateM’s Codex marketplace at a pinned revision. This establishes packaging, not actual deployment. [Packaging PR](https://github.com/sudosubin/agents.nix/pull/37609).

**Not checked:** private deployments, an exhaustive user census, execution of the inspected source, and the contents of the downloadable benchmark archives.
