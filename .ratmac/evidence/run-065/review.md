# Independent review before full proof

Ticket [t-141](../../../.arca/ticket/archive/t-141.md), Run `run-065`, requirement SVE-004 (unique test scratch
folders). Every reviewer only read files and ran tests into its own git-ignored build folder; none edited a
file or ran a git command that changes state.

## Round 1 (2026-11-07)

Three reviewers split the work: one took the shared helper and the public checks, one the migration of every
scratch site in the public suites and the test library, and one the private lane.

- **Shared helper and public checks: accepted, no blocker or medium finding.** `TempTree::with_stamp` makes
  each folder with `fs::create_dir` only, takes a new counter value on every try, moves on from a taken
  name, and returns any other error; `TempTree::new` reads the clock once and calls it. The reviewer checked
  each public check against the red stub and against wrong helpers: a name without the counter fails
  `svev_010` at its 30-second deadline, adopting or emptying the planted folder fails `svev_011`, and a suite
  naming its own folder from the clock fails `svev_012`. All seven checks in the shared private test support
  suite passed. Four low findings:
  1. `svev_011` assumed, without checking, that the request's first try is the planted name. Fixed: the
     check now also asserts the request's counter is above the planted name's.
  2. The site matcher misses some shapes (a `push` onto a bound temporary path, a wrapper function, a join
     through a second variable, a formatted path). None exists today, and all fall outside the ticket's
     settled counting rule; not changed.
  3. Two wrong helpers pass every check: one that adopts a folder only when it loses a race
     (`if !path.exists() { create_dir_all }`), and a `new` with its own loop instead of calling
     `with_stamp`. Reading the code confirms neither is present; they are named here as limits, not
     claimed as caught.
  4. `write_project` in `test/qa/src/baseline.rs` kept a make-room delete that the accepted mechanics drop.
     Its folder is always new, so the line did nothing. Fixed: the line is removed.
- **Migration: rejected on one blocker.** The pre-split residue suite (`t077_presplit_residue`) named its
  linked worktree folder after a long test label, and the added counter pushed Git's own folder path inside
  it past the Windows path limit, so `every_public_entry_point_preflights_its_addressed_project` failed on
  this host every time. Everything else was accepted: no assertion was removed or loosened, every removed
  `Drop` only deleted the folder the tree now owns, no changed code keeps a folder past its test, and nothing
  in `test/qa` makes a folder in the system temporary folder outside the helper. Fixed: the linked folder now
  takes the short fixed label `t077-linked`; the suite passed 6 of 6.

- **Private lane: accepted, no blocker or medium finding.** Every row of `test-hidden/t-141/` proves its
  oracle cell and runs its valid control before its attack. Against the red stub all six rows fail (01, 04
  and 06 at the control, which the ticket pins; 02, 03 and 05 at the attack). The reviewer built nine
  wrong helpers in a scratch copy and each is caught by at least one row: a clock-only name, adopting,
  emptying or deleting a taken folder, a drop that does not delete or does not delete while unwinding, a
  drop that also deletes same-label folders, a `write` outside the owned folder, and a name off the
  documented format. The lane passed 14 of 14 twice, at one test thread, ten times in a row and in three
  rounds of six copies at once, leaving no folder behind. One low finding: four child notes used
  `println!`, so with `RUST_TEST_NOCAPTURE=1` set they broke the exact result line `support::isolated_test`
  reads. Fixed: they print to stderr; the lane passes 14 of 14 with and without that variable. A note, no
  action: the eight group cases always pass outside a row's child, so the sweep counts 14 lane tests for six
  rows, as t-120 and t-121 already do; the rows themselves cannot pass that way.

## Round 2 (2026-11-07)

- **Migration re-review of the three fixes: accepted, no blocker or medium finding.** The `t077` label fix,
  the removed make-room line, and the new `svev_011` assertion are correct; the assertion cannot fail
  spuriously, because the counter only goes up and nothing takes a counter value between the control and the
  real request. `svev_011` alone passed 20 of 20 repeated runs.
- **Private lane re-review of its fix: accepted, no finding.** Undoing the described change in memory
  restores exactly the reviewed bytes, so nothing else changed; the moved row 04 note now prints only after
  the control folder's name is checked.

## Round 3 (2026-11-07)

The first full proof found the private lane `t-140` red, alone as well as in the sweep: rows HT-140-02, 03, 04
and 06 found no freeze Engine record. `Pair::new` had moved its two project folders onto `TempTree`, which
removes them when the pair drops, and that lane reads the freeze Engine's `evidence.toml` inside them after the
child test exits. The folders now come from `TempTree::new` and are kept with `persist()`, as `main` always left
them.

- **Helper re-review of the `Pair::new` change: accepted, no blocker or medium finding.** Each folder is still
  made fresh by the shared helper; the struct has exactly `main`'s fields and leaves the same folders `main` did;
  no caller relied on removal, and the t-140 lane needs the opposite. Two low findings: the change departs from
  the accepted mechanics' "never calls `persist()`", recorded as a `conflict-resolved` log line; and the
  `svev_011` counter assertion could be exact (`== next + 1`) but is correct as written. The t126 suite was
  re-read at the digest below.

## Reviewed inputs

Every changed tracked file and every file of the private lane, at the bytes the full proof runs on.

| Reviewed input | Tracking | SHA-256 |
| :--- | :--- | :--- |
| `.arca/log.md` | tracked, modified | `b061b80ed94f08d8535f756d104105b93d9800faf3c6c0e199889a2a7af5e9b7` |
| `.arca/ticket/t-141.md` | tracked, modified | `1a81958009a4bdc84c24734d72a86d219c4c91a46177187afaa4e70d33bc0b29` |
| `.ratmac/evidence/lane-sweep/report.md` | tracked, modified | `9556ef6e0b5df27bb358548d3e6d6734f459ebcd927544619b1b3abfe74c1bd9` |
| `.ratmac/evidence/run-065/SVEV-010.toml` | tracked, new | `5e9d169262ab7cada25e2034be377290150fdec264857c8f862c4ee470fcb12b` |
| `.ratmac/evidence/run-065/SVEV-011.toml` | tracked, new | `9ca0eaa629fa2f1e657d9f53c0c93173eb0d27418f0a0549cddd2eaf4cc3c68d` |
| `.ratmac/evidence/run-065/SVEV-012.toml` | tracked, new | `9de018becf744295687b2bd44933d357ca8b0a5d3edfd654013b20cf50197063` |
| `.ratmac/lanes.toml` | tracked, modified | `e3b45b5ee44ac614fbc08ec0f673e6d5d767d09dea32ee18ee565eba90feab8a` |
| `test/qa/src/baseline.rs` | tracked, modified | `f016761d36dcc3cac58d9bfc6d9c024eda118dbbf024a58ede1db25123b023b1` |
| `test/qa/src/lib.rs` | tracked, modified | `b406ed8dd7cb6040ddc318925d41b89921b0732b548789b68d78d22f91c8abe5` |
| `test/qa/src/support.rs` | tracked, modified | `a73a965d385de6a89355603c16504072fe808927607c16273dd4e34300a8762f` |
| `test/qa/tests/t014_r024.rs` | tracked, modified | `cf77f085e962721a251869e077b2660f71102098c3f42a39071d8359799d68be` |
| `test/qa/tests/t015_r027.rs` | tracked, modified | `5e21d4f6b796756aaf345974000552afa5d5d2796c994ccc81427d79139b9206` |
| `test/qa/tests/t016_r005.rs` | tracked, modified | `9986dcb91be2658e546fab4c9201d7b929b32759e39d708002fce093d4af88ca` |
| `test/qa/tests/t017_r006.rs` | tracked, modified | `3b172b9a42634d539ad0417ca9693b49dba28ff7c0e9b9e87233d06577c3a147` |
| `test/qa/tests/t018_r017.rs` | tracked, modified | `d96cd8ad00ca2cbcd1450867bd25aa3eb07064f59a85558b7a5a093a5dc8453a` |
| `test/qa/tests/t019_r018.rs` | tracked, modified | `87197177a57888faca97eeb234d8fc61cfe9eb1eefa8eb9ac2abd1242120fd8f` |
| `test/qa/tests/t020_r019.rs` | tracked, modified | `9ce4d5d33ff2024a185abca7c60e8ec7579b6721b030f44a41a1444f3208cee5` |
| `test/qa/tests/t021_r020.rs` | tracked, modified | `a909ff65e60234ba6e1ccc479584cc4493f79bd59d05c3208b5f0da1196e2ebd` |
| `test/qa/tests/t022_r026.rs` | tracked, modified | `9eeff7e218193659663994fc3a726af4ab67ebd3564fff097d673f71a53eb0f0` |
| `test/qa/tests/t023_r015.rs` | tracked, modified | `8c5a3cbd2775f3c2f2e6c10c7235d20c716b87147f476868d9b61ebab5da2d0d` |
| `test/qa/tests/t024_r028.rs` | tracked, modified | `ee5eb87904fb260b413f31eef5a1a7ece12b1ecd4f72697317c13fc97825443d` |
| `test/qa/tests/t025_r029.rs` | tracked, modified | `6d5b17d4f0c3ba5196c22449831b32dc69a83176495d3c8f453d93837ad10573` |
| `test/qa/tests/t026_r030.rs` | tracked, modified | `0b6dc2d8c287dc79bc8b75842931eeeb145b0639b3e23e465846a60aa75108c8` |
| `test/qa/tests/t027_r012.rs` | tracked, modified | `641acf48e9673cda09bb4b9c4a47119b57f367a89b54c2f6f4a1aeb1a07b7e34` |
| `test/qa/tests/t028_r016.rs` | tracked, modified | `fe89e91a6e868fc0089f673f41d3019b26784bc0208dd06a3155f1c395d2da5c` |
| `test/qa/tests/t029_r007.rs` | tracked, modified | `44d2a4d8aa2ec5e6e652c37c36fea1ae5f599348a4cdaa2b03068ee75c5ab266` |
| `test/qa/tests/t030_r008.rs` | tracked, modified | `a047b866ec00dbce695a93974654b571b52b15cddb158bafefc937122671e009` |
| `test/qa/tests/t032_rat003.rs` | tracked, modified | `e8153a1b34de1d26b330e1ddd34516b19240c17dde255f8f43ea30376ba8eaf5` |
| `test/qa/tests/t036_rat007.rs` | tracked, modified | `100de6dc25d6ffad32ae145aa945c8ec28a90c728d1a0035790451bf79dc823c` |
| `test/qa/tests/t037_rat008.rs` | tracked, modified | `d0bfd90e1510fdb8d69ecdc39b4282fcb7b592fccc07d4377f634852dd28fec2` |
| `test/qa/tests/t041_guard_diagnostics.rs` | tracked, modified | `431612f1b3a9a100dcdade8721bb1411d9dadf2ded4e8f2b4a1fa220eabb3a23` |
| `test/qa/tests/t042_gate_pin.rs` | tracked, modified | `36ab863c9a3b03857087b5a5e113415d5758af481950ddcd440aaff07ea707f8` |
| `test/qa/tests/t043_goal_freeze.rs` | tracked, modified | `bb68533ddfd4c4a5f6364a0f4c1573b61ce22d964b11215eb45c2e96b3808006` |
| `test/qa/tests/t044_start_policy.rs` | tracked, modified | `71bd90ab3beb29d7fe0fe0fbeab20e08c76f15d0c11cff6d3fd85d872a373425` |
| `test/qa/tests/t045_bootstrap_doctor.rs` | tracked, modified | `11e37e18d47481489e807e6d34d50dd09c0ddcd40da5a0420b1098539a959d77` |
| `test/qa/tests/t047_receipts.rs` | tracked, modified | `7ab2944ad8add782b88d120f175c7696d3af6972c69ecb3a8f5062a6c60af193` |
| `test/qa/tests/t048_contract_gates.rs` | tracked, modified | `1c0a24afd1d37f87f03532d1780c5647d18bae8ce9ae6c45fe447b91326cd108` |
| `test/qa/tests/t049_completion_gate.rs` | tracked, modified | `c06d25c54018ddf56a49f7ed9a3e105d9015ebe1b216f876ea5535690a547477` |
| `test/qa/tests/t050_blocked_route.rs` | tracked, modified | `e2700789e46e779cb8f57a254f4b2ce101050f4567f331cdfbe5eefc17b669d0` |
| `test/qa/tests/t051_abandon.rs` | tracked, modified | `f921215cefd3053561028580c11962ff5bea7c356b2bc9396220c86443f85304` |
| `test/qa/tests/t056_typed_parser.rs` | tracked, modified | `3df77ff38cd1f20651976b737a604eb0af9a9ad62c66598af84533566f7b6b94` |
| `test/qa/tests/t057_deep_doctor.rs` | tracked, modified | `db1caaec9b5351eaef1cac35be42710baffb20cd6c75348cdbabe7f33a3a7723` |
| `test/qa/tests/t058_authoring_loop.rs` | tracked, modified | `ba3b31bb0ad3be4a344fbd4fa5ae0f364c7e668d8c30cfa57df2962e3878236c` |
| `test/qa/tests/t059_run_residency.rs` | tracked, modified | `ac287401dcb12fd0ed54d9f2ed5e09f598263d4528619872127598c339e58d36` |
| `test/qa/tests/t060_runbook_pin.rs` | tracked, modified | `c97209c715b8c1f230ec0a9c8b87a9e38a00242269ef1cff74f638ab36a8a071` |
| `test/qa/tests/t061_input_routing.rs` | tracked, modified | `8dfa7c150bf712c6fee9bb4be3d6b26cf2e344b0f6b29018f063651a4f869221` |
| `test/qa/tests/t061_uncapped_runs.rs` | tracked, modified | `f321b48a8defdfd3419498bac60669c10102eede33720829f82daac48e64359f` |
| `test/qa/tests/t062_verdict_delivery.rs` | tracked, modified | `0b99a39b3e532f1f3db3cac2a8af65e6c7671abaa968c9239844343fe8b56a18` |
| `test/qa/tests/t063_run_completion.rs` | tracked, modified | `f36d17992f4605eed4ca4b66898c00e6cb16519632d9099157832ac9a7daa437` |
| `test/qa/tests/t064_composition_format.rs` | tracked, modified | `ce259ed94f3bb93948125be5aa6139170749ac473745827a2ba38ec5275fb4e8` |
| `test/qa/tests/t065_motion_authorization.rs` | tracked, modified | `5dd110c14c84dca1ec60dbee4f27101da277f23afa711de70979fd4ce6c1cc39` |
| `test/qa/tests/t066_spawn_ledger.rs` | tracked, modified | `691bab3af8ae5730b6b2f90879c78a6b20ba16b636dcd2c1afba24bce0675ec4` |
| `test/qa/tests/t067_cycle_termination.rs` | tracked, modified | `75d55dfa012793f9c8afed6b881a72ef94fad083b90848ecf372c6947a3bad5f` |
| `test/qa/tests/t068_recursion_cap.rs` | tracked, modified | `5deea176d63a57ee4e40f6e8f5275e639ca665b4744e11abc85551a109c0642c` |
| `test/qa/tests/t069_child_reviewer.rs` | tracked, modified | `f3120878cfe86e0bc27b5a2409cf5b6047acf17a6b4240842f8623e0ef6fb9e5` |
| `test/qa/tests/t071_engine_root.rs` | tracked, modified | `33a1d122fb4f2cb18be01ab3978d4bb84459975b3fab599c3ede7fc71b31d10f` |
| `test/qa/tests/t072_repository_namespace.rs` | tracked, modified | `52a521499f1b36901168e1993b5ac024f5035d57b827d741cf5fcc0bd085a07e` |
| `test/qa/tests/t073_lock_split.rs` | tracked, modified | `06d35b82acf2e7128f735d5970b709cc8b65d8b0b95230a29ed63f3d3092a2cc` |
| `test/qa/tests/t074_child_workspace.rs` | tracked, modified | `5d3d84b02a9d6e9997196c1fcd447212d2333bb0530c821671378f1b57258022` |
| `test/qa/tests/t076_roots_table.rs` | tracked, modified | `0b69563fdac69da9fe61594575b1877815e7acf06f105e408cf4bc45b18129d7` |
| `test/qa/tests/t077_presplit_residue.rs` | tracked, modified | `a8106a07b6b7c5239c014ddbcbb13b751aa299c70c2f95d22e539426e0f70da6` |
| `test/qa/tests/t078_resolved_root_reporting.rs` | tracked, modified | `bc74f4bfd866b363b875c727ef72929fdefbdc47660ca85589686077279f7917` |
| `test/qa/tests/t079_root_spelling.rs` | tracked, modified | `70e68eaa3955761b64f869ab469cae8999f70d88d16e7906f09bebda657db788` |
| `test/qa/tests/t080_states_table.rs` | tracked, modified | `708e6c350b4d18b329859115deb10aada8ab4ed6d46fbda0caf7a1b70454de63` |
| `test/qa/tests/t081_run_record.rs` | tracked, modified | `5ad3f173b0c6e5e27617735d811bfa1baca61086035670932c03852b0c59e11b` |
| `test/qa/tests/t082_precutover_residue.rs` | tracked, modified | `e102a8f6a6806924bb5c68a84c13f5afc34735a4645eb0ddee064fa7fb2e776a` |
| `test/qa/tests/t083_state_surfaces.rs` | tracked, modified | `cad181a3b476addf175995a105a14a358c27b0581a88cfafe37ce149e29f2759` |
| `test/qa/tests/t084_state_audit.rs` | tracked, modified | `e958714e2ddf9ffd990e4f95a29d3c31641b43c6739397deadcf3d76611e16ac` |
| `test/qa/tests/t086_single_engine_binary.rs` | tracked, modified | `02a618ceed2083616f0f3e6b365c257c94dbf228af238aaeb451f1a54d515c36` |
| `test/qa/tests/t087_no_work_item.rs` | tracked, modified | `bc626b8dfd18cea802e26fb7919754f5e161a8345bb0198a8715ef0d6ee58476` |
| `test/qa/tests/t088_intake_authority.rs` | tracked, modified | `6c6105c878ea0bf51bbfb01a6959e0342d9658ba31d5ad568f80165c2bc25398` |
| `test/qa/tests/t089_bound_address.rs` | tracked, modified | `a25528308c73a9107a47baf7ad7fab03431a504af66fb80c24e0170ba0cb3739` |
| `test/qa/tests/t090_damage_checkpoint.rs` | tracked, modified | `5c62749e254a9d820a0fae7674f05d914866c63e094f44a236009c64618091f7` |
| `test/qa/tests/t091_open_work_item.rs` | tracked, modified | `a5269fae52a2a6f2fe36589677061aa48f79457b8ba20e2446cbf4982f07ef1b` |
| `test/qa/tests/t092_plan_build_runbook.rs` | tracked, modified | `1ee71873e382bc6cdd64f64d48555a603f08d9b6e8cb16ea7ea596f41ea8195a` |
| `test/qa/tests/t093_stage_answer.rs` | tracked, modified | `3c1c20609f880304f8d94ee44611c86043a919230f49dc49ae69e291ac27ff90` |
| `test/qa/tests/t094_edition_guard.rs` | tracked, modified | `f81f24601dbf2bdb8233aa00a1debeb7d1500a6740661c17bd27aac9d6074e6c` |
| `test/qa/tests/t103_self_describing_cli.rs` | tracked, modified | `34c7339f7c7072e77047eab5b42dfa3a1c33073593c2a6fe68919418f9ab3a8d` |
| `test/qa/tests/t104_operator_skill.rs` | tracked, modified | `473118b1de3b5a2dbd8e481a5ca696bedaf283376534be91e9b7e67cda161af8` |
| `test/qa/tests/t108_lane_sweep.rs` | tracked, modified | `a9f06d371397874d47f894e2255a92434d7cbdc2819dd7cfeeda1b6a5b583b0e` |
| `test/qa/tests/t110_post_split_ports.rs` | tracked, modified | `ecaeb3570d88196eaff99fc05d8c1c759133413dca27af5f4cc880fa7b017328` |
| `test/qa/tests/t112_completion_declared_data.rs` | tracked, modified | `57c57428497404f79798563c8fe456d9af8cb0b111c92f386704ffedc21256a4` |
| `test/qa/tests/t113_reopened_work_items.rs` | tracked, modified | `e5cff3547aadb2829927086975af02773b80e220c1d4d6a2e43b0284f0b869f8` |
| `test/qa/tests/t122_successor_ownership.rs` | tracked, modified | `12a3d73ca4d32e5099b3045d4a939b8d2a31fb3ba1ff15a3319bde7ba4f8f0c1` |
| `test/qa/tests/t123_portable_goal_fingerprints.rs` | tracked, modified | `dcce52fa893a56b1efd8e56f0715b25163c5021df63f1d1e6513035717a79d73` |
| `test/qa/tests/t126_shared_private_test_support.rs` | tracked, modified | `5d3c44a8b302eac2e043432c2d697ece3ef136863680c923da0cd747f46e37b5` |
| `test/qa/tests/t134_safe_nested_invocations.rs` | tracked, modified | `ead6b57e7128a855cb478818b657012ddae37fa2507cef255db9eac57ba822d1` |
| `test-hidden/t-141/Cargo.lock` | lane, ignored by Git, new | `61a9f1e04c2d0e640b2a66a1ef9a036f2f1f80ca421594a52c0ee837d9ebcfe3` |
| `test-hidden/t-141/Cargo.toml` | lane, ignored by Git, new | `68660c26768944523b88c550983971b608b112f6fe84bdaa2acc9d512a1f80cb` |
| `test-hidden/t-141/src/lib.rs` | lane, ignored by Git, new | `262fa2d976dc954e2482bed4801916aef024ff21d3fbf4577c57925af2bfbca9` |
| `test-hidden/t-141/tests/hidden.rs` | lane, ignored by Git, new | `751ac038239354448642fe9dff0e4dde599de4afec7b3c8c24899081498c7ac7` |
