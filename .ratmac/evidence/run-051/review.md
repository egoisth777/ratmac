# Independent review before full proof

Accepted on 2026-10-03, before executing the final full workspace proof.

- Two independent reviewers split the work; neither edited, built, or ran anything in either checkout. One reviewed the source change (the new `src/diag.rs` and the nine changed source files) and the public suite `t128_deterministic_trace_channel.rs` against the ticket. The other reviewed the new private crate `test-hidden/t-128`.
- Before the contract went in, it was refreshed against the landed `main`: the inventory of mandatory stderr lines and their source lines, the two standing child-pipe exceptions, and the refusal codes. Before red, the test writers' questions were settled in the ticket, never by the implementer: a Git-initialised standalone fixture resolves `repository-primary`; the edge record's `input` follows the Run's current State; a lock file left by a killed process compares by name.
- The implementer's first run exposed a private-crate defect, fixed by its owner rather than by bending the source: the record helper appended a line feed that the line splitter strips, so the exact-sequence assertions could match no implementation. The helper now compares records without the line feed, and a separate check proves every record in raw stderr ends in one.
- Each review took two rounds. In the first, the source review asked for three medium repairs: the edge record fired before a governed step's authorization check, so a step refused there emitted both an edge record and a refusal record; refusal records for `goal drift`, `intake_contract`, and `record_contract` copied a field that holds absolute host paths, leaking the host layout and differing across fixture locations; and two lock sites formatted a string on every call even with tracing off. The private review asked for two: no lane fed a carriage return or byte 0x7F through the escaping rule, and the library-entry lane could not see records a wrongly placed latch would write to the process's own stderr. The ticket was amended so those three refusal kinds always omit `path`, and its refusal-code list now names the six `authorization: <kind>` codes today's governed step spells; the implementer and the private owner made each repair.
- The deliberate-damage probe writer, working ahead of the final suites, flagged faults no lane would catch; the private owner added a case for each reachable one: a refused step after entry (a goal-drift refusal) with its exact record sequence and no edge record, the same refusal at two fixture locations with identical records and no path, and a traced lock contender that waits and then acquires once, or whose wait expires and records nothing.
- In the second round both reviews accepted. The private review added one low: the crate's refusal-code table still held the pre-amendment list; its owner added the six codes.
- Deliberate damage from the first safety commit found one gap no lane covered: moving the `mint_read` record to after the durable mint write survived every check, because every lane minted successfully. The private owner added an exhausted-mint case to lane 04 - a mint record already at the largest integer, where the refused start still emits `mint_read` before the root release and no `mint_write`, with an untraced control first. A third, narrow private review accepted it with no findings; the mutation applied from the safety commit failed lane 04, and the full proof and every damage check were then rerun from a new safety commit.
- Behavior no lane can reach, judged by reading only: the edge record's place after a governed authorization refusal, since a spawned binary cannot pass enrollment in a test (it needs an operator-protected manifest, and the in-process trust seam never traces); the current-directory failure line; parent-directory sync-failure warnings; and the removal of the toggle from Git and `ssh-keygen` children, which ignore it.
- Before review, the implementer reported green: public `t128` 4/4, private `t-128` 6/6, the `diag::` unit tests 2/2, public `t022`, `t044`, `t073`, `t075`, `t086`, `t103`, `t119`, `t120`, `t121`, `t122`, and `t129`, private `t-073`, `t-119`, `t-120`, `t-121`, and `t-129`, `cargo clippy` (one existing warning in an unchanged file), and `cargo fmt --check`. After the repairs, the same set and the extended private crate stayed green.

| Reviewed input | SHA-256 |
| :--- | :--- |
| `.arca/ticket/t-128.md` | `73085c15a42403c7afe36142e94aa3ac0899450cdd0a9ca776b080ac729bba03` |
| `.arca/log.md` | `6e937aefd0f6c61468a4a57d8d8d3adc99d8c7ba7d40fa6a92744f20371076bd` |
| `.ratmac/lanes.toml` | `7052c1b8db4cb2a5ac243cdb0e12ec2ebc3e3f4a753a995ed7b9f23fb015e775` |
| `src/diag.rs` | `028d5df6c0ad622f7f0de0df0b5b902736c85b552f31e3f4b0ac85f3e4d594a6` |
| `src/bin/rtm.rs` | `335248714cf138926edb4e53a163b5b7a151008b6ef107766491ec7361f76e60` |
| `src/blocked.rs` | `48276e08175a737816500ff5c0f1b3da9063c772100388fd4f8e62686cc28967` |
| `src/channel.rs` | `89ca74248288672abf05a9ba6ef3b593d68b341ef101b48d1918bb748c49667b` |
| `src/lib.rs` | `d0ad9441645da45272dba3150d1485007ce484b2eeee98f5e6646692dd4602bd` |
| `src/lock.rs` | `0715e092a6d8fcbce8249f9162cff05311b400edd0e84eba5f5997276ead8b1a` |
| `src/mint.rs` | `e971c8a683919730a88a8d7445c65d2ae47f639380cafe6b1d73fb6f8bfd93e5` |
| `src/review.rs` | `09bbbeb09e1b5d6948afc3e4a8517b14c2139986ee8057d7099be6f81b8139fe` |
| `src/root.rs` | `b9c043526ba538ddae93d3257fd1972ef2902ddc66058a7ad338eb264979ee1a` |
| `src/scheduler.rs` | `d27bb3e264ca065c5f609cbba38c38151d61f328c37005f10dfe4f8b5f8058fb` |
| `test/qa/tests/t128_deterministic_trace_channel.rs` | `620a3af300de2560085841a7b151bde02e3803e2e5feb65c10442658982f486e` |
| `test-hidden/t-128/Cargo.toml` | `da3f88bde4e34598371400d0aa9d1daaec667cea2b37fe44a7be0bc11e3ee616` |
| `test-hidden/t-128/Cargo.lock` | `626b32e0e23e08e4374e5a7ffe48f3774dd051da9b0f0f0668c5d5f0c24166b3` |
| `test-hidden/t-128/src/lib.rs` | `3b65d1c1b1689b985e6fed82f9c8c7bdf934c7048cf4552af5dfe50f5cfdadb9` |
| `test-hidden/t-128/tests/hidden.rs` | `1eebd7743418bf216a7288a9b139e86d476fced07ac9c43349eae136fe2953d1` |
