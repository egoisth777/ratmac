# Freeze-build measurements for the run-private freeze and ports folders ticket (`t-140`)

The run-private freeze and ports folders requirement (SVE-008) asks the
ticket to measure the time the run-private freeze build adds. These are
single-run observations, not a timing gate: two runs in each checkout, on
this Windows machine, on 2026-11-01, one after another from one Git Bash
script, right after the same script's full workspace test run and the
ticket's focused and private checks.

Every run is the rename suite (`test/qa/tests/t085_rename_is_behavior_free.rs`),
whose one test, `the_rename_changed_names_only`, builds the freeze Engine.
The command, run from the root of each checkout, was:

```
timeout 3600 cargo test --offline -p ratmac-qa --test t085_rename_is_behavior_free
```

- **Ticket worktree** (`E:/hive/packs/projs/skills/ratmac-t-140`), with the
  ticket's change in place but not yet committed: each test process removes
  the freeze tree an earlier run left, extracts it afresh, and builds it in a
  folder named after the suite inside its own `CARGO_TARGET_TMPDIR`.
- **Primary checkout** (`E:/hive/packs/projs/skills/ratmac`) on `main`, its
  `git status --short` empty after the runs: the freeze build lives in the
  shared `ratmac-freeze-<commit>` folder under the system temp folder, which
  a repeated run reuses.

The ticket records that these runs came before the fifth review round, which
did not change the freeze build.

## Time

| Run | Where | Cargo's build step | Test time | Wall time |
| :--- | :--- | ---: | ---: | ---: |
| `t085-wt-1` | ticket worktree, first run | 0.04 s, nothing compiled | 6.24 s | 7 s |
| `t085-wt-2` | ticket worktree, second run | 0.04 s, nothing compiled | 4.33 s | 5 s |
| `t085-main-1` | primary checkout on `main`, first run | 3.96 s, compiled `ratmac-qa` | 4.14 s | 8 s |
| `t085-main-2` | primary checkout on `main`, second run | 0.03 s, nothing compiled | 2.99 s | 3 s |

"Test time" is the test harness's own `finished in` figure for the one test.
"Wall time" is whole seconds around the whole command, Cargo's build step
included. All four runs exited 0 with 1 passed and 0 failed.

## What the freeze build adds

Pairing each worktree run with the run in the same place on `main`:

- first runs: 6.24 s - 4.14 s = 2.10 s
- second runs: 4.33 s - 2.99 s = 1.34 s

So the run-private freeze build adds about 1.3 to 2.1 seconds per
rename-suite run. The comparison uses test times, not wall times, because the
first run on `main` also compiled `ratmac-qa` (3.96 s), which the worktree
runs did not need. In both checkouts the second run was faster than the
first; two runs each cannot separate the change from this machine's own
variation, so the range is an estimate.

## Source

The numbers come from a scratch folder outside the repository, which is not
kept: `summary.txt` (exit status and wall seconds per run), the `.out` and
`.err` files of the four runs (the harness's and Cargo's output), and
`main-status-after.txt` (the primary checkout's status after its runs).
