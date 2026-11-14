# Repeated single-root public suite

Ticket [t-148](../../../.arca/ticket/t-148.md), Run `run-068`, requirement WTC-003 (skill move retry).
The merge gate asks for `cargo test --offline -p ratmac-qa --test t119_single_root_resolution` 25 times on
this host with zero `rtm skill` failures. The 25 runs were made on 2026-11-14, one after another with
nothing else running, in the ticket worktree on the fix (`src/skill.rs`, SHA-256
`0f49a507de8dc1434f5896855ac6b3d7df0bf235e4c1aeec21859f148783843a`), after the full workspace check
and before the safety checkpoint.

All 25 exited 0 with `test result: ok. 4 passed; 0 failed`. No output holds a skill write refusal
(`cannot write`), a failed test, or a panic, so there is no other failure to report. The outputs
themselves are scratch files and are not kept.

This is a signal, not a proof. Before the fix the rename failure appeared in 2 of 40 runs of the private
lane `t-119`, a different and longer run than this suite, so 25 clean runs could happen by chance.

| Run | Exit | Seconds |
| ---: | ---: | ---: |
| 1 | 0 | 44 |
| 2 | 0 | 45 |
| 3 | 0 | 44 |
| 4 | 0 | 44 |
| 5 | 0 | 44 |
| 6 | 0 | 44 |
| 7 | 0 | 44 |
| 8 | 0 | 44 |
| 9 | 0 | 46 |
| 10 | 0 | 45 |
| 11 | 0 | 44 |
| 12 | 0 | 45 |
| 13 | 0 | 45 |
| 14 | 0 | 44 |
| 15 | 0 | 46 |
| 16 | 0 | 47 |
| 17 | 0 | 47 |
| 18 | 0 | 46 |
| 19 | 0 | 46 |
| 20 | 0 | 46 |
| 21 | 0 | 46 |
| 22 | 0 | 46 |
| 23 | 0 | 47 |
| 24 | 0 | 46 |
| 25 | 0 | 51 |
