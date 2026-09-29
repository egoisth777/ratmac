# Independent review before full proof

Accepted on 2026-09-28, before executing the full workspace proof.

- Two independent reviewers split the work; neither edited, built, or ran anything. One reviewed the source change (`src/goal.rs`, `src/contract.rs`, `src/scheduler.rs`) and the public suite `test/qa/tests/t123_portable_goal_fingerprints.rs` against the ticket. The other reviewed the new private crate `test-hidden/t-123`, written without reading the public suite or the source, judged from the ticket and the red engine bytes alone, and read the public suite the same way.
- The contract was re-pinned after `t-122` landed, moving only line citations. Before red, two test-writer questions were settled in the ticket, never by the implementer: `<goal root>` is today's text at each site (the drift line prints the goal address, the read-refusal wrapper prints the goal directory), and a refusal with several unrepresentable entries names the first one in ascending bytewise order of displayed relative-path bytes, the same order the fault seam uses.
- After red, the implementer's disputes were settled against the ticket and today's engine: one nested-tree oracle in the private crate dropped the subfolder from each relative path, a private-crate defect its owner fixed; and the ticket's "today's literal absent" for a goal root removed after the freeze was unreachable, because root validation refuses first. An engine change to `src/roots.rs` that made that case reachable was reverted; the ticket and the HT-123-05 oracle now say a removed goal root refuses at root validation with today's RB603 text, updating nothing.
- The ticket's one regression adjustment was made by the public owner: `t048_contract_gates.rs`'s repository self-check now takes the expected freeze from the live gap records' own citations instead of recomputing it with today's algorithm, and keeps its final green assertion unchanged.
- The source review took two rounds. The first asked for one medium repair: the v2 entries were sorted by their displayed spelling, which turns a backslash into `/`, while the hash used the path bytes, so a Unix name containing a backslash could serialize out of order. The implementer now sorts by the hashed path bytes and keeps the displayed order only for refusal naming and the fault seam. The review also found an example commit id in the ticket two characters short, which was corrected. The second round accepted with no finding.
- The private review took two rounds. The first asked for one medium repair, operation-log reads that fell back to empty so the no-lock and no-new-kind checks could pass with no marks; and five low ones: doc-comment casing against the ticket's oracle cells, one RB603 path spelling, the rollback text's two-line shape, a public twin that never mixed line endings inside one file, and a public helper that passed inherited test variables to the engine. The second round accepted with no finding.
- `.ratmac/lanes.toml` gains only the `t-123` roster entry in sorted order, which the schema requires in the same change as the crate; the lane sweep proves it mechanically.
- The first full proof of the landing commit failed one landed check, `t110_post_split_ports`: both of its nested full sweeps, run at the same time, reported the `t-086` lane red on `ht_086_04_build_order_stops_deciding_what_a_test_launches`, a test that gives a paused engine 10 seconds to reach its barrier. The source was byte-identical to the checkpoint proof, which passed; only record files had changed, and that lane reads none of them. Run alone in the worktree, the `t-086` lane passed 6/6. The proof was discarded and rerun with nothing changed.

| Reviewed input | SHA-256 |
| :--- | :--- |
| `.arca/ticket/t-123.md` | `f55e791c57e30f97afb155110cbe3f4e4dd92122bb2ffd0b9ed1157df7a2ce2f` |
| `.ratmac/lanes.toml` | `86f4b005c033b358273149230ab43b44175e0c57a01c161391b816a38ff5027c` |
| `src/contract.rs` | `6595e9a0073f671b50a27dfd911a93fa37fcd73c6145cf90c6579018bfb71c26` |
| `src/goal.rs` | `498406a19765b06c6e275e635322169236a30ec8fc1dcf40dc721d64bb6bbe0e` |
| `src/scheduler.rs` | `3a8c739f05068dad088dc51ae16afe9e6636d4553ea809b12553037a39a8f2e5` |
| `test/qa/tests/t048_contract_gates.rs` | `4e5261f0abc8b6c2045b4196bfc3c0b666481ee5ff5591986769c238d11097c1` |
| `test/qa/tests/t123_portable_goal_fingerprints.rs` | `295621f75d9b7cfacd209e28a1f88c6b0cedef3c50b039e5a7630fd615668447` |
| `test-hidden/t-123/Cargo.toml` | `e768f0b317253a208ad27e42023a59a1d02eb02b7e9aa7c14f9e3510095b250f` |
| `test-hidden/t-123/Cargo.lock` | `f5f8936846d32c2d4d1579dfaf7d6372dbf03c9325123ba9dd5365d90208baa3` |
| `test-hidden/t-123/src/lib.rs` | `601bc9b9d91e1ae9be5a4d9a9e240c9a70f19ba11eecc4c6b4633141f7f557c4` |
| `test-hidden/t-123/tests/hidden.rs` | `b150ae56cc090b0884b5aece33acf9e480bac9e5f48383e36fe559498eba9c7e` |
