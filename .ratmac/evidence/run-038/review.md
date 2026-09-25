# Independent review before full proof

Accepted on 2026-09-23, before executing the full workspace proof.

- Two independent reviewers split the work: one reviewed the source change (`src/abandon.rs`, `src/cli.rs`, `src/scheduler.rs`), the live hints in `tools/rtm.ps1` and `.arca/schema.md`, the public oracle, and the adjusted `t-104` private assertion; the other reviewed the new private crate `t-115`, which a separate agent wrote without reading the public oracle or the source diff. Neither reviewer edited, built, or ran anything.
- The source review's first round asked for repairs: a missing or blank `--run` value did not teach the addressed usage, a project with no admitted Run and no leftover lock still taught the project-name phrase, and several refusals were not compared byte for byte. The second round accepted with no open finding.
- The private review took five rounds. Repairs made its checks judge only what WRS-007 fixes: presentation-free roster and example checks, a sole terminal-state admitted Run, diagnostics before any lock and after cleanup, help and documentation examples parsed as commands whose address and phrase must agree, either quote style, and every quoted hint collected whatever it says. The fifth round accepted with no open finding.
- Focused and private `t-115` checks pass against these bytes, in a normal run and with an inherited fault setting. The affected regressions `t051`, `t065`, `t066`, `t103`, and the private `t-065` and `t-104` crates pass. Any later source or oracle repair requires renewed review.

| Reviewed input | SHA-256 |
| :--- | :--- |
| `src/abandon.rs` | `63305214be3df2493f4f8a6a391c7dea1924f71361919e2614b1941c38fa37b5` |
| `src/cli.rs` | `04c4adb90971de371664d0289960e94357ae59c47e3927d88bd0e773362edcfe` |
| `src/scheduler.rs` | `071a3ecc58130bbdc5192173ec2c9710cd5bb88774578d604e1cfe0166b44a09` |
| `tools/rtm.ps1` | `440ae4b397799d69dd8792c41db6825a7cc9f0c5cd909d2af99c9f575bb5f57e` |
| `.arca/schema.md` | `b871e8a1050eacc76a6af0668431161a043e8554337525dd831b43918347a5d9` |
| `test/qa/tests/t115_abandon_confirmation_hints.rs` | `07535d234907e9613c7a7e3cc1d3cb2dc8d3abf1525638ac827ea2a09f5a3a02` |
| `test-hidden/t-104/tests/hidden.rs` | `ee85e06e04967a0b52957dbf16ca69ec44346c8229cf061af215212508989350` |
| `test-hidden/t-115/Cargo.toml` | `acb2718728dbcbaa022ed137a785b6ac90f3309597c9039c56b5b0fabed4d9a1` |
| `test-hidden/t-115/Cargo.lock` | `b8910acfed6a875242ff4af5979fed4c2c66bb7499d74003a01ea80f2f588b20` |
| `test-hidden/t-115/src/lib.rs` | `0e7c2fd74aade69ae2a96425ae27b9cc405ec7c151a68da5973d6649cb413579` |
| `test-hidden/t-115/tests/hidden.rs` | `37e71b4eb92e6d98234b83578576d8c63aa16b7265b14e7fd396001b61dd00af` |

## Renewed review after a surviving mutation

On 2026-09-25 the checkpoint damage stage deleted the pre-split residue refusal at the top of `src/abandon.rs::resolve_target` (probe d12); every public and private check stayed green, and so did every other suite that plans abandonment in process (`t051_abandon`, `t077_presplit_residue`, `t082_precutover_residue`, and private lanes t-063, t-065, t-066, t-068, t-069, t-073, t-077, t-082, t-109). The `rtm` binary refuses residue at dispatch before any abandon code runs, so only direct callers of `ratmac::abandon::plan_abandon` (respawn, in-process adapters) can observe the order. The oracle repair adds library-level cases to `wrsv_007_04`: with a leftover lock and no Run, and with one admitted Run, every direct plan refuses naming `state.toml`, teaches no phrase, and changes no fixture byte. The first renewed review asked for the admitted-Run cases: moving the check to `plan_abandon` right after the address resolves (probe d16) would otherwise show the roster and a Run phrase first. Both probes now fail `wrsv_007_04` and restore green. `src/` is unchanged. The independent reviewer accepted the repair with no findings.

| Reviewed input | SHA-256 |
| :--- | :--- |
| `test/qa/tests/t115_abandon_confirmation_hints.rs` | `c4959c265beb050dba3f9be984761224b83744a8ecfd91934bcd0c6a4fa9d785` |
