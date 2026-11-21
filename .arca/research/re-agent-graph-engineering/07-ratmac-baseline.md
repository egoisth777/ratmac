# ratmac Baseline

**Date:** 2026-11-21
**Producing agent:** `ratmac-baseline`
**Method:** read from code at main 81d54d8, nothing run
**Known defects:** None known.

---

# ratmac (`rtm`): what the code does today

I read the code and specs at `main` 81d54d8. I changed nothing, and I ran no `rtm` command or test, so everything below comes from reading code. Items marked *(inferred)* follow from the code but were not tested. `rtm` has 16 commands (cli.rs:113-116). There is one strict-TOML runbook per project, at `.ratmac/ratmac.toml` (spec:9-14).

Correction to my earlier report: I have now read the build-command rule. It also blocks common test commands like `cargo test`, `npm test`, `go test` and `make` (details under §2).

**1. Who triggers a move, and is there anything in the background**
- A Run only moves when someone runs a command. `rtm step --run <id>` moves one Run by one step (cli.rs:580-622). `main` runs one command and exits (bin/rtm.rs:3-34).
- The Engine stores no caller identity. "Human confirmation" just means a phrase typed as an argument (blocked.rs:15-18, abandon.rs:17-19). *(Inferred: an agent with a shell can type the phrase too.)*
- There is no daemon and no timer. The only thread reads the nested receipt pipe during a single step (scheduler.rs:5497). Lock waits give up after 5 s (lock.rs:47). The nested endpoint checks for the child's end every 20 ms and never gives up (endpoint.rs:591-597).

**2. Every program the Engine launches** (all the `Command::new` calls in src/)

| What | Where | Pinned by hash? |
|---|---|---|
| `command_exit` program | scheduler.rs:6047 | Yes, from its first use. The SHA-256 is recorded in evidence.toml then; a later mismatch means it is not run (6082-6182). `exempt = true` skips the pin (6041). |
| `nested_run` child: the same `rtm` binary, run as `rtm endpoint` in the child checkout | scheduler.rs:5476 | Its hash is checked against `engine-pin` only if one is declared (5365-5391). The receipt's engine and class pins are also checked (5617, 5624). |
| `nested_run` `driver` | endpoint.rs:413 | No. It is resolved to a regular file (408) but never hashed or recorded. Its name is re-read from the parent's ratmac.toml on disk (224-262). The parent ignores it (scheduler.rs:5246). |
| Signature verifier (`-Y verify …`), governed Runs only | review.rs:1617-1634 | Yes. Its hash must match the enrollment and the runbook's `verifier-digest` (review.rs:943-1000). |
| `git` | root.rs:663 (every call); ignore_policy.rs:313-453 (init/start); scheduler.rs:1340 (spawn); channel.rs:184 (doctor) | No. It is found through PATH. |

**Build-command rule.** This check runs before the `exempt` check, so it applies to exempt commands too (scheduler.rs:6033 vs 6041). It matches on the program's file name, ignoring case (pin.rs:362-394).
- **Build drivers** `cargo make cmake ninja msbuild go npm` are refused with no arguments, or with any of `build run test check install bench fetch update publish` (pin.rs:337, 343-345). So `cargo test`, `npm test`, `go test` and plain `make` are all refused.
- **Compilers** `rustc cc gcc clang cl` are refused when given an output flag (`-o -c --emit --out-dir --crate-type --output`) or a source file ending in `.rs .c .cc .cpp .cxx .go .m` (pin.rs:340, 348-358).

**3. The nine guard kinds** (machine.rs:120-176; spec:143-153)
- **General:** `files_exact`, `file_contains`, `command_exit`, `join`, and `nested_run`. `command_exit` is subject to the build-command rule above. `nested_run` needs another ratmac checkout that declares a `program` key (endpoint.rs:500-502).
- **Assume this project's records:**
  - `sensitivity_receipts`: reads the ticket's `planned-test-refs:` list and receipts in `.ratmac/evidence/<run>/`. The test must have failed (non-zero exit), and the test file must contain `fn <name>(` (receipt.rs:108, 195, 212, 222-226).
  - `completion_gate`: tickets, focused/hidden-lane/quality checks and receipts (completion.rs:14, 61-74).
  - `intake_contract`: issue intake, five fixed issue files, and the dispositions accepted, rejected, duplicate, deferred (contract.rs:25-31, 1121).
  - `record_contract`: gap records (residuals), tickets with fixed headings (contract.rs:34-40).
- The goal-drift check is not a guard kind. Once the goal is frozen, every step refuses if the goal has drifted (scheduler.rs:4141-4163).

**4. Branching**
- Guards sit on States, not on edges (spec:47). Every guard is evaluated, and any failure refuses the step (scheduler.rs:4922-5101, 4165-4167).
- A State with more than one ordinary edge lists its `inputs`, and each edge carries one `input` (spec:46, 60).
- After the guards pass, the Engine reads `runs/<id>/verdict.toml`, which must hold exactly `state`, `input` and `rationale` (verdict.rs:12, 72-106; scheduler.rs:4170-4211). The route depends only on the State and that input (6333-6340). The verdict is then archived to `verdicts/NNNNNN.toml` (verdict.rs:203-211).
- A failed guard routes nowhere. The only side exit is `rtm hold`, along a `blocked-route` edge that `step` never takes (spec:62).

**5. Loops**
- RB214: every State on a cycle must carry a receipt or contract guard (the two receipt kinds or the two contract kinds). It checks the kind only and runs nothing (doctor.rs:549-610; spec:330). So a loop guarded only by general kinds fails this check.
- Only `rtm doctor` runs RB214; it has no other caller (doctor.rs:351). `start` and `step` use the parser (scheduler.rs:1034-1080), which has no RB214, so they do not refuse such a runbook.
- There is no visit counter and no attempt cap. The only limits are side effects: 999,999 archived verdicts per Run (verdict.rs:11, 243-246) and nesting depth 16 (lineage.rs:15).

**6. Data a Run carries**
- run.toml has seven fields (model.rs:211-221). `input_revision`, `output_revision` and `active_refs` are written empty (scheduler.rs:1808-1810), and I found no code that fills them. `evaluate_entry_prerequisites` (6216) has no caller.
- A top-level Run takes no input: `rtm start` accepts no arguments (cli.rs:471-475).
- A child Run gets `rtm spawn <name> --run <p> [--bind k=v] [--workspace]` (cli.rs:86).
  - Unknown binding names are refused (scheduler.rs:2240-2246). Missing values are not. `required` is only checked when the runbook is parsed (machine.rs:1025-1036).
  - Values are stored in the parent's spawn-ledger (2336-2353). Only `ticket-binding` guards read them (5747-5812). They are not in the State Prompt (6439-6462).
- Nothing in the Engine passes one State's output to a later State. The verdict holds only state, input and rationale. *(Inferred: anything carried forward goes through workspace files.)*

**7. Changing the runbook during a Run, and other pins**
- `rtm start` records the SHA-256 of ratmac.toml in evidence.toml (scheduler.rs:1826). Every motion checks it (717-721, 1080-1135). A plain edit is refused with "runbook pin mismatch".
- Gap: if no pin is recorded, the check passes (1108-1110), and an unparsable evidence.toml loads as empty (pin.rs:65-77). *(Inferred: deleting or editing evidence.toml removes the runbook pin.)*
- The signed route is `rtm recover`. It needs one `ratmac-recover-v1` signature from the Approver and appends to `amendments/` (recover.rs:1-17; cli.rs:80).
  - The new runbook must keep the Run's class, current State and workspace (recover.rs:1061-1090).
  - An ungoverned Run must pass `--enrollment` (recover.rs:1162-1174).
  - The head of the amendment chain becomes the expected pin (scheduler.rs:1119-1125).
- **Enforced:** the runbook pin, the frozen goal (4214-4235), gate pins after first use, the verifier pin, and the nested `engine-pin`/`class-pin` when declared.
- **Recorded only:**
  - The engine identity (1821-1822): `confirm_engine` (pin.rs:151) has no caller. Doctor only reports a live Run whose recorded channel or source commit differs from the stable edition (channel.rs:108-168).
  - The goal baseline (1824).
- **Not pinned:** `exempt` commands, the driver, and `git`.

**8. Child Runs**
- **spawn:** the parent must be in the State that declares the spawn (2215-2238), not passed or blocked (2204-2212), and not itself a child. Children cannot spawn their own children (2194). Spawning the same name twice is not refused, and ledger entries do not record the spawn name (ledger.rs:22-38).
- **respawn:** `--confirm "respawn <id>"` mints a new id and retires the old Run. A child successor keeps its class, bindings and workspace (2429-2438, 2632-2670).
- **join:** every child that has not been abandoned must have `passed` in its run.toml, and there must be at least `min` of them (default 1). Zero children refuses (5108-5187).
- **nested_run:** one child Run per invocation address (cli.rs:101). The parent's step blocks until the receipt arrives (5498). Depth is at most 16; `max-depth` can be 1-16 (5352-5363). A repeated identity or root is refused (5343-5351).
- **Running side by side:** there is no cap on active Runs (1719-1721). Each Run has its own lock.

**9. Pause, resume, abandon, and review**
- **hold:** `--blocker <ref> --confirm "hold <id>"`. The blocker must exist under a declared root, and the State must have a blocked route. The Run moves to the route target with status blocked (blocked.rs:15-23, 716-720).
- **resume:** the same blocker plus `--confirm "resume <id>"`. Status becomes executing and the State does not change. The Engine never judges whether the cause was fixed (blocked.rs:1204-1205; cli.rs:77).
- **abandon:** `--confirm "abandon <id>"`. It logs an event and deletes run.toml, evidence.toml and the root lock (abandon.rs:21-29). It needs no signature, even for a governed Run (abandon.rs has no review code).
- **Review policy** (spec:108-133): for every step, hold and intent of a governed Run, the Engine requires:
  - an enrollment the builder cannot write (review/protection.rs:1-13);
  - the declared reviewer signatures;
  - the Approver's signature over the canonical intent (spec:113-116).

  Resume needs a `ratmac-resume-v1` signature from the Approver (cli.rs:77).

**10. History and state files**
- Each Run's files are in `.ratmac/runs/<id>/`: run.toml, evidence.toml, spawn-ledger, verdicts, review.toml, pauses/, journal/ and amendments/ (model.rs:92-99; pause.rs:3-16).
- Shared files: log.md, mint.toml and locks/. Receipts are in `.ratmac/evidence/<run>/`. In a Git worktree, the main checkout's `.ratmac/` is used (root.rs:578-641).
- **Protection against hand edits:**
  - run.toml is strictly parsed for its seven fields (state.rs:17-25) but carries no hash or signature. `join` trusts a child's run.toml status (scheduler.rs:5145-5167).
  - log.md is opened append-only and symlinks are refused. The code says hand edits are outside its "cooperative threat model" (scheduler.rs:515-556).
  - Only amendments are hash-chained (recover.rs:860-875).
- **Absolute paths are stored** in:
  - evidence `resolved` entries (seen in run-076: `E:/…/rtm.exe`, `E:/dev/envs/anaconda3/python.exe`);
  - the ledger `workspace` (`\\?\E:\…`, seen in run-003);
  - the review `manifest` (spec:121).

**11. Exit codes and `--json`**
- **0:** success. A refused `rtm step` (failed guard, missing verdict, held or terminal Run) also exits 0 after printing `rtm: …` (cli.rs:606-621).
- **1:** errors and most refusals (cli.rs:19-24).
- **2:** usage refusals (cli.rs:26-33) and a bad `RATMAC_TRACE` (rtm.rs:14-19).
- **doctor:** 0 clean, 1 warnings only, 2 any error (spec:293-296).
- Only `rtm doctor` has `--json` (cli.rs:1806-1813).

**12. Terminal outcomes**
- **passed:** the Engine writes it when a step enters a State with no ordinary out-edge (scheduler.rs:4228-4230), or at start or spawn if the initial State has none (1669-1676, 2272-2276).
- **failed:** it is a valid status (model.rs:17) and the nested endpoint reads it (endpoint.rs:392), but no code ever writes it.
- **retired:** there is no such status. Abandon or respawn deletes the Run's record and logs an event. A nested parent sees "abandoned" when the child's run.toml is gone (endpoint.rs:366-377).
- Side note: `step` never sets status executing; only `resume` does (blocked.rs:1204).

**Not checked:**
- lock.rs, nested.rs (the invocation ledger), goal.rs, ownership.rs, profile.rs and skill.rs in full.
- Doctor checks other than RB214.
- The exact bytes of the canonical intent.
- The claims in `.arca/index.md`.
- Tests in test/qa.
