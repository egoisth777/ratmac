# Independent review before full proof

Accepted on 2026-10-04, before executing the final full workspace proof.

- Two independent reviewers split the work; neither edited, built, or ran anything in either checkout. One reviewed the source change in `tools/turn.ps1` (the new `review` and `proofs` verbs, the gate inside `checkpoint`, the new `after-proof-command` pause boundary, and the guidance and usage lines) and the new `.ratmac/review-proofs.diff`, together with the public suite `t131_review_before_proofs.rs` and the public regression adjustments in `t124` and `t125`, against the ticket. The other reviewed the new private crate `test-hidden/t-131` and the private regression adjustments in `t-124` and `t-125`.
- Before the contract went in, it was refreshed against the landed `main`: the draft had been written before `t-124`, `t-125`, and `t-127` through `t-130` landed. The refresh found no change in behavior; it re-pinned every cited line to the landed bytes and replaced the sentences that still called `t-124`, `t-125`, and `t-130` unlanded.
- After red, the implementer's runs exposed three test-side defects, each fixed by its owner rather than by bending the source. The private `t-124` and `t-125` unknown-verb controls kept the old five-verb row beside the two new usage lines. Private `ht_125_02`'s rename case changed bytes after its review without renewing it. Public `wcpv_011` closed with the turn's untracked records still in the worktree, which the landed `close` refuses (its only-copy rule) and `git worktree remove` refuses too. That last one was a contract error: the ticket said the records are destroyed with the worktree at close. It was amended so the operator deletes them before `close`.
- In the first round the private review accepted with no findings. The source review asked for one medium repair and two low ones: a receipt that could not be read let `proofs` run the declared command instead of refusing it as malformed, as the ticket's validation order requires; the stale row's path maps ignored letter case; and an item-records value ending in `/` let the verbs' own evidence into the snapshot. All three were repaired in `tools/turn.ps1`. The private crate had pinned the wrong behavior for the unreadable receipt, so its owner changed that case: a receipt held open with no sharing now refuses before the command, and a receipt held open readable but not replaceable still reaches the write-failure row.
- In the second round both reviews accepted with no findings.
- The deliberate-damage probe writer, working from the reviewed source, wrote 55 probes (49 in `tools/turn.ps1`, 6 in the diff) and named six it expected no test, or only one suite, to catch; their outcome is recorded by the damage run, not here.
- Deliberate damage over the first safety checkpoint (55 probes) found four promises no case pinned and one test-side race. A successful `proofs` now provably consumes a planted staging file for its receipt; the stale refusal provably lists added and removed work and lane paths, sorted; a record whose path repeats is provably refused as malformed; and the private marker wait reads the whole boundary line instead of a file the tool had created but not yet filled. Only the private crate `t-131` changed; the source stayed as reviewed. The private reviewer re-read the new cases against the ticket and accepted with no findings.
- Before review, the implementer reported green: public `t131` 3/3 (after its close repair), private `t-131` 6/6, private `t-124` 6/6, and private `t-125` 6/6 (after their repairs). After the review repairs, public `t131`, private `t-124`, `t-125`, and `t-131` stayed green.

| Reviewed input | SHA-256 |
| :--- | :--- |
| `.arca/ticket/t-131.md` | `45d4555f2278b513a304e2eca7352ea95592e6b16c8cf2f89f2807e056000b49` |
| `.arca/log.md` | `364e91ae369cdd4e0b8f76c57b6d48f59388c514ce6182a3cfd564176391b48b` |
| `.ratmac/lanes.toml` | `3a3e115478a91c16ad45e428d388579e59328eb7a6c3db88910e79a76d1bc277` |
| `tools/turn.ps1` | `7e8a5c6f5ffdc78456d0d7522f4fc3997c9ef51b7155d5d5d4fe97c64dda3905` |
| `.ratmac/review-proofs.diff` | `da2ba5a1e9a3aba356f16335fd75405eb4376a09f8ea6046af8b6f0be287ffca` |
| `test/qa/tests/t131_review_before_proofs.rs` | `aa7c70119d54c2c080e3fbda6ff71fcaf1c1819d3c5081abb2aa62c26fab9751` |
| `test/qa/tests/t124_byte_verified_restores.rs` | `2b2817aef58286c4069bec89281b2b514a1244ca6006ed22616712fa9726a7c7` |
| `test/qa/tests/t125_checkpoint_only_restoration.rs` | `e337f08d2e4cb92f06514626b34acacff7202f324213c2abdd2d29ecdee56229` |
| `test-hidden/t-131/Cargo.toml` | `3c0ccc24171e186ec7b67753c220a4bdf8c2db315ffac3415b49a31eed4c9137` |
| `test-hidden/t-131/Cargo.lock` | `e1874176e1d6f53cbfcde6caa52ab631369b1a41beb0d508b74cfbcac7bbe050` |
| `test-hidden/t-131/src/lib.rs` | `91c73f56ea29bdb0e85186f453d72c7503d0d320b91b99b42a54bf41611c879a` |
| `test-hidden/t-131/tests/hidden.rs` | `16ab2b14c6da7121b22ef798f917cbec8af48b7fb060fc8f3e183adf5bd82e6b` |
| `test-hidden/t-124/tests/hidden.rs` | `40a3b242950a18aff54977d5627c76b833a3e2545655932cfe6ed25dbe5d3a87` |
| `test-hidden/t-125/tests/hidden.rs` | `b4f0af08b4b47c39c8cbcdd69777d76c1f686ec7abdd992f4f6a6fa42acd105e` |
