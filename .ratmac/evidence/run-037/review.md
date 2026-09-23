# Independent review before full proof

Accepted on 2026-09-23, before executing the full workspace proof and the complete private-lane sweep.

- Three independent reviewers split the work: one reviewed the shared support module, the QA crate, the public oracle, and the new private crate `t-126`; one reviewed the migrated private crates `t-058` to `t-083`; one reviewed `t-084` to `t-118`. Each compared the worktree against the untouched primary checkout and none edited, built, or ran anything.
- The first round asked for repairs: snapshot helpers silently dropped links that the old walkers had followed or refused, several missing-folder rules had changed, the in-process reuse of lane runs did not cover every input, and the after inventory was not bound to current bytes. All repairs were applied, the after inventory was recaptured, and each reviewer re-reviewed the changed lines.
- Final verdicts: all three accept with no open finding. The shared reviewer independently re-derived all 464 file rows and 542 preserved rows of the recaptured after inventory against current bytes.
- Focused and private `t-126` checks pass against these bytes, in a normal run and with an inherited fault setting. Migrated crates were each run normally and with an inherited fault setting during migration and repair. Any later source or oracle repair requires a renewed affected review and proof.

| Reviewed input | SHA-256 |
| :--- | :--- |
| `test-hidden/t-058/src/lib.rs` | `c5dd5f529ee40ec79e5cc607e5fab83937fa43e7cd8808c5e3e072be4139c4eb` |
| `test-hidden/t-059/src/lib.rs` | `b7c673534baa28ea64d4470200baa7b11f9b154f5073d99f2348d2d2c16424aa` |
| `test-hidden/t-060/src/lib.rs` | `1f3a605617e5f0abc89ac5099a10bfe0f7a4fedc00e50da8acf3e7beaf4b7462` |
| `test-hidden/t-061/src/lib.rs` | `a00ce17bbf1209179e5ecaf136dfce8d9be4ddd7f0d653060bdafa72f2ede800` |
| `test-hidden/t-062/src/lib.rs` | `83784497452164b05c0f93a0a042d970e6d3683d13f1f152e1351ae3cd6a06a5` |
| `test-hidden/t-063/src/lib.rs` | `67d0b9461eb99f64c9d42fd9344785cb7be244da51db7a235364adf5bc0ffb32` |
| `test-hidden/t-064/src/lib.rs` | `d1e4ca35cc5d4127aca80fc0929ed4cd26bef4929099831bde4a0bcce6c691f9` |
| `test-hidden/t-065/src/lib.rs` | `9106019ec70675c8f44085f551f30f6cca95389aedc7099c3f4ca841e1846384` |
| `test-hidden/t-066/src/lib.rs` | `6be19aa5c7a5adcaf79f3dcb34a8eac7aad95871e7231a3a6a427695f9fe9b60` |
| `test-hidden/t-067/src/lib.rs` | `067d2c3f1af24af2b6c6a4799f2afef18a7f3c48cbb00958731287c50e426ca0` |
| `test-hidden/t-068/src/lib.rs` | `a8f56e040eb4e8ef4a825be5d60dde2601720518609c9cdd6cdda04ac378e23c` |
| `test-hidden/t-069/src/lib.rs` | `d8caf2829a2730333db6e5c9a262ccd68e37a02908e60210eab3d9f8fced45a9` |
| `test-hidden/t-070/src/lib.rs` | `4f8276070dd6c4fab7df09c4a3ef30b00fd66d99ccbecc29fe75b64808630ff7` |
| `test-hidden/t-071/src/lib.rs` | `31ed14c0797bd9315741458a1e43aa02bdea9a4f1ad75b57984e41c29c6fdc09` |
| `test-hidden/t-071/tests/hidden.rs` | `a5be5b18dac88c8d4c01704f6ca8aa80fe89e32d944f1f775c10f3fe4617afb8` |
| `test-hidden/t-072/src/lib.rs` | `515e897098ce3be07d7be672d2fd084bf4577dc9c099a31d01cb94b786064143` |
| `test-hidden/t-072/tests/hidden.rs` | `e64c14785cf5ca429e231505abb57d107807ede9a143d3a6aa9f444e937cdec9` |
| `test-hidden/t-073/src/lib.rs` | `c467c079c4a410e3189dff992fcc4b8896f19f70d0116e250ae01b2bc8301498` |
| `test-hidden/t-073/tests/hidden.rs` | `f00338e85cf1422749ce4df9f206bc769b46fe5ea814816cd231e146c5a50000` |
| `test-hidden/t-074/src/lib.rs` | `27fd642e77e4da838476a1e32753f372e75ef2be262b0a6bffcafb3ba33ca58c` |
| `test-hidden/t-074/tests/hidden.rs` | `f8f778d1a1fab767a9d89fb380e08f55bec1b21a3543314d80079b2c609a5bed` |
| `test-hidden/t-075/src/lib.rs` | `9ee678c687fc6717218abafab4546e078dd744a549cd994da45e114458dc9675` |
| `test-hidden/t-075/tests/hidden.rs` | `99e55cedc54a90c0a598579e0437e70e3c6aeb77de9e5c4764c058e1856306a0` |
| `test-hidden/t-076/src/lib.rs` | `e56c39bbf024bad9e909ea61c402dfdf6f4dd0f0e6dc02edb25b49050c1804a5` |
| `test-hidden/t-076/tests/hidden.rs` | `86f415c81babc310f8ffa74e749b7df248140d8f1596b2bdea12c40d6a9cbb44` |
| `test-hidden/t-077/src/lib.rs` | `a801f3a5349696636efdce8871a6463e525d9256681b20c229708ecbd5371f95` |
| `test-hidden/t-077/tests/hidden.rs` | `8e12db2ae2c396eb4bfa96f8b3f08bda9f6877ec4caaf28cbbe51997467126c2` |
| `test-hidden/t-080/src/lib.rs` | `1ef336e91c6cb3ffaaf0ccbcf83b1bbe2ea8b21074daff91619f104bb67c15fd` |
| `test-hidden/t-080/tests/hidden.rs` | `fb9b325ae5cfead9e844e1d9e982d614d624fe9706d842446c55e99f8dac2740` |
| `test-hidden/t-081/src/lib.rs` | `1ef336e91c6cb3ffaaf0ccbcf83b1bbe2ea8b21074daff91619f104bb67c15fd` |
| `test-hidden/t-081/tests/hidden.rs` | `e0c05236c4f6d1de00b2ad976f1f9955680952b80b3478b827696a5313cf8f15` |
| `test-hidden/t-082/src/lib.rs` | `1ef336e91c6cb3ffaaf0ccbcf83b1bbe2ea8b21074daff91619f104bb67c15fd` |
| `test-hidden/t-082/tests/hidden.rs` | `463fe628fb98b9244c55c0bfeeabfc16c7db90c777a97496eaa78a9f53addefa` |
| `test-hidden/t-083/src/lib.rs` | `f57a888e1cfeb40d927b9ec3ca0dd19dbe2c73980d7e1ce6471b79e394c13bb9` |
| `test-hidden/t-083/tests/hidden.rs` | `b9d0b5d6d18fa5d5d825759f02d48eb8a3b43b60e426084658e68d57a448bfb3` |
| `test-hidden/t-084/src/lib.rs` | `2f7049a8f7d16b342ffd12ebb4174f5a5b04cffc4ea7bc4eb857bbe50f10392b` |
| `test-hidden/t-084/tests/hidden.rs` | `7da761488e81d951d6c64a34ea4049db08d8abeca74683cf63dd3b38ea0fe1af` |
| `test-hidden/t-086/src/lib.rs` | `f4baa5585e373300d8c142c64ebc092a34ba99069b6f9b36ebd00da18ae6a21e` |
| `test-hidden/t-086/tests/hidden.rs` | `7d47126a799f510d4433785bea8df21b02cf6330edf23a651f54b0017050291c` |
| `test-hidden/t-087/src/lib.rs` | `9dff1ce19c308e5d714d44193085b538a768d2ce0db56f3715f0437497525f7b` |
| `test-hidden/t-087/tests/hidden.rs` | `8b2c82511809883f761f4be3667c34849f714367d787dd8282c78baa3337ba69` |
| `test-hidden/t-088/src/lib.rs` | `24900f02bbed519c6fe929a0e5afd11e48b39bd5f1ab759e515aa5fa14ab16be` |
| `test-hidden/t-088/tests/hidden.rs` | `a147f4546d8930096fb0ee64e2862c1b8181d32b3d187373bb0fa8b4fd6a1350` |
| `test-hidden/t-089/src/lib.rs` | `d83d414ce5407d662ea7cee516eb5a709fd813332f56c825398e394c5b1fd52d` |
| `test-hidden/t-089/tests/hidden.rs` | `7f6aa794fa9c421e2f8f31460287bcdc302ec8fb684009116eaba2832d328185` |
| `test-hidden/t-090/src/lib.rs` | `53647791f3b2a2299360cdbce4c4e14c259c65392e52cf023ada3d734125cbe7` |
| `test-hidden/t-090/tests/hidden.rs` | `aee4d7f7aa304545e2562676afa77c48f50ba5387de8c55ffe45a44792f7f57a` |
| `test-hidden/t-091/src/lib.rs` | `79dc8bf1ac57b2cb707e129357fd0ac930a007c6cc377cc135da76baf2501b42` |
| `test-hidden/t-091/tests/hidden.rs` | `ded3cf44bce3668dd50706199ed16f93c43bc1a53367d40ba3c51230d8e8a7c0` |
| `test-hidden/t-092/src/lib.rs` | `0b2b2a0e324496f30058a8e297a0a3d2d6a1efaca28d1fbafe00fcdf8b7749b2` |
| `test-hidden/t-092/tests/hidden.rs` | `e7c2b8976003c1de95dcc3d87421cba8381820d687fbdf278402319a8000c56b` |
| `test-hidden/t-093/src/lib.rs` | `65654ebd2608d69f4b7cf8ce37f078dac0f8a7ca569e9cd5e6d8d6f1c0875258` |
| `test-hidden/t-093/tests/hidden.rs` | `0407e724ad67b47416549a21fc7440c839bc4239c44d2b64dde85753fc79f8ec` |
| `test-hidden/t-094/src/lib.rs` | `65654ebd2608d69f4b7cf8ce37f078dac0f8a7ca569e9cd5e6d8d6f1c0875258` |
| `test-hidden/t-094/tests/hidden.rs` | `0f93e1f69a269301d5aab9f6d581e697a7df93cc0958d973fe9cc1f004c89198` |
| `test-hidden/t-095/src/lib.rs` | `76ecbd92705b4d0c8e32037ff8d2a43058eca0f58f9c4a3c567547f6079955d7` |
| `test-hidden/t-095/tests/hidden.rs` | `1f0512e7499af205e99712fc94b457473812df2455f93c971d3a7e5b9a00a519` |
| `test-hidden/t-096/src/lib.rs` | `76ecbd92705b4d0c8e32037ff8d2a43058eca0f58f9c4a3c567547f6079955d7` |
| `test-hidden/t-096/tests/hidden.rs` | `570d931dc12de77bd1c2ddb6062271bf8a9b00ab6369e594d3d7ba598056f58d` |
| `test-hidden/t-100/src/lib.rs` | `b1337ad25cde4904936c7a6314912710872317869e7bbf777eb83f768f3c573e` |
| `test-hidden/t-100/tests/hidden.rs` | `a28a0390c85470e5fe85c997f030d758566e335cd8be65bad85f9ccfeda0c691` |
| `test-hidden/t-101/src/lib.rs` | `b1337ad25cde4904936c7a6314912710872317869e7bbf777eb83f768f3c573e` |
| `test-hidden/t-101/tests/hidden.rs` | `5ee3b7f08e624393c7cf9bb0c0da0de171c32effde84af50225d3a1f9ac759f5` |
| `test-hidden/t-102/src/lib.rs` | `13d3cc4b413ca6ccec31951e5dc9dcb803d53baf13411fac1520517510a8ccdd` |
| `test-hidden/t-102/tests/hidden.rs` | `7166513acbd61b8d7784b43c26133c33e8f9cfdf5e7647df3dbf83280b0f760c` |
| `test-hidden/t-103/src/lib.rs` | `e3c8641ff60bc765989b1c5616416c424a18a952cb4a708a6b1dfc2dbd4638b7` |
| `test-hidden/t-103/tests/hidden.rs` | `cdad4287db47694efb239d491dc64bbb1d26569b78c74a2dd2945fa83e65b370` |
| `test-hidden/t-104/src/lib.rs` | `d0d224069ee165b9ef6ef835df7ea3f4a972205c3add1c4e7a6d8370f01e0fa9` |
| `test-hidden/t-104/tests/hidden.rs` | `fff5c035736e2236d3f2153fc15045569d019da9226c6693a77383655a283d36` |
| `test-hidden/t-105/src/lib.rs` | `d1742be95f7abdc4d283fe14d9de1add3b5c01470225cca5ab27eac411815fcb` |
| `test-hidden/t-105/tests/hidden.rs` | `0bc1d6b80eeb0adb0db42204cb002fd93de6a3f5c61e45e8a75c383978e20d4a` |
| `test-hidden/t-106/src/lib.rs` | `c0e3456abcbfaac8244eb182cb8831acbb616020338d96c345e595913c66116d` |
| `test-hidden/t-106/tests/hidden.rs` | `c5e547264ee16075e455eb2a0aee8c6bd788fa22accd180830e4f2e84dbdc888` |
| `test-hidden/t-107/src/lib.rs` | `e51b6d234714ee87a0c2ddd7b6405c3cf2d76a6d0a8a1b58b25775c7ccce87e5` |
| `test-hidden/t-107/tests/hidden.rs` | `35f3d7ede40834d985940845e88d6819eb97fa8a5eab11cfb16c97144c9f36e0` |
| `test-hidden/t-108/src/lib.rs` | `d5a597a41127631ae6ee29674ed2700301d837f30fdae856b1c1c02386628378` |
| `test-hidden/t-108/tests/hidden.rs` | `fa3d0023f91f196b7e08690f227c7be85ff7d8c94ac1df07d72f03bf08b084e7` |
| `test-hidden/t-109/src/lib.rs` | `cb7287616e09e08e6be8e0dd7f266a0222f41b0a6d41f9fe571d479fadbae4a1` |
| `test-hidden/t-109/tests/hidden.rs` | `423646e690adaf0affedde393be0321fbebcc23d254c3bef95790d5d1471b71e` |
| `test-hidden/t-110/src/lib.rs` | `f6f094571712be278779089fafc0f2b77f011403dfe70a58dcd47ce67eb789ae` |
| `test-hidden/t-110/tests/hidden.rs` | `bd43d1f65e22576d3d8d9d77c8c5e7778a0a1df4ecc2218d51b66aa1a29246ce` |
| `test-hidden/t-111/src/lib.rs` | `500be426b9571b9cea5bc76962d29b79925c7adc6ffbe791bda924501ffb2715` |
| `test-hidden/t-111/tests/hidden.rs` | `99c8acbe3b859d8d487c23628d2346a4449606715e1f321a193ebd3ef19d6e96` |
| `test-hidden/t-112/src/lib.rs` | `6c92fdbb8ecc6a74e36e19f3c1de564eb2551802ae0d3a7deb6bd69577be6267` |
| `test-hidden/t-112/tests/hidden.rs` | `99adaea46f1f02907f2c458ae764ca34d5442635cf70cc53907b412fac047f29` |
| `test-hidden/t-113/src/lib.rs` | `147104f2b7eb581b31b05d04968a26eac5b65459da486cc0c80a38e72d4d6db8` |
| `test-hidden/t-113/tests/hidden.rs` | `00d98081d754c9b9a30ba7c4a76bc608457c7ef72f04509524ed23986a3ad0e2` |
| `test-hidden/t-113/tests/reopened_work_items.rs` | `883af4a7d816189be8812b644395250bbb0e06e0b2795a0daa7d7c4dd84e1313` |
| `test-hidden/t-114/src/lib.rs` | `7fa76e6f9a1ec9818d76d52800dc5f31bc76e63f148311bba56c6b7741a2d7b3` |
| `test-hidden/t-118/src/lib.rs` | `805b7689be53fa6d30106477bdd6ab1caf2a7364d6c3e5677ea71162620634d9` |
| `test-hidden/t-118/tests/hidden.rs` | `3e4e1449f5305484c329b9fed78608fa72f349bde7cc13d1bfc414db5396bb81` |
| `test-hidden/t-126/Cargo.lock` | `66dc0306bdaed8d43c6ee93194a0888e5967e854e07f8963495f12bb08ff4a62` |
| `test-hidden/t-126/Cargo.toml` | `9803dcbaf393ab85c4cf0c90e979bffee6f3a6605eb49c0c599fd1ccac878fad` |
| `test-hidden/t-126/src/lib.rs` | `9450014d3928fd8fc058910cc0f4b4c6458a6926918e4bf8fec346c23d91f174` |
| `test-hidden/t-126/tests/hidden.rs` | `57bee28f6e75367906692ddeb848d16be7eb8826be9fa1f9f1566b465ac340ae` |
| `test/qa/Cargo.toml` | `498662fa45bf640b90c699aba2a60b1bb1f956c8951d68737292838faef42516` |
| `test/qa/src/aged.rs` | `d6e60255b32b482d47e2c6a530ab3060420e25a601a5ccea0e6fc1e220301db8` |
| `test/qa/src/archive.rs` | `b7238a6483978e45a6af40c8997b9daf2cdc85fa86013f013e8c297ce4397fd6` |
| `test/qa/src/audit_files.rs` | `37ac3421e8b16a6fb3b9d35cbf1ce27330a8e6f9e57aeb2377d312ef139eed4b` |
| `test/qa/src/baseline.rs` | `b1d859d45bbd731de42bbfbe89afec6407f668f7736b6ade399b23c8bc7b334d` |
| `test/qa/src/citations.rs` | `415cb4dad9a86a013e4c10e1b136f56d5a6cb5c58bae2cd76b335f18b4430395` |
| `test/qa/src/edition.rs` | `bef03b7cf4cf33dd8b7f924b302a51c80753c789c3bfc430202ee0c239dd5220` |
| `test/qa/src/lane_runs.rs` | `5542fd7b918fbdbd01f21bf333d5c393302fa2e8c8d2ce9f0b7ae287ed9106dc` |
| `test/qa/src/lib.rs` | `ffd6fd86aad81ac6fe54f41b39bda02404b6124d437849dc2c07d3f37e411bdf` |
| `test/qa/src/snapshot.rs` | `b3a3e9f30de93cf8501ecd7a992b702686c410b8f834b00346fb7e5eaca06ab4` |
| `test/qa/src/support.rs` | `b700e9729b015171aa731d640746964c2e5dcc92d1dde5c96082fb760a1d3d67` |
| `test/qa/src/tempgit.rs` | `85f22f44ac4fc69efb24fa8154eaf03b8c445adf319a6dc8d8ee50ec153391e2` |
| `test/qa/src/trial.rs` | `d4f82be2e303e69beebc726be65163b5b2a72f9ec49fbd8b71d8a538c081d546` |
| `test/qa/src/turn.rs` | `22a8e1971bbfb4ea3861eac4c6812f99a3d0459b5941c342779f0b858c535ec3` |
| `test/qa/tests/lane_sweeps.rs` | `484a1f4e4c5d5a1b798f126949a03976fa004206ba8373fdc53cdb6ad5e882eb` |
| `test/qa/tests/t108_lane_sweep.rs` | `fa4f13d1bf68877b5be6b44d1d197258e1d39535374a54e2bf26332179b5c48f` |
| `test/qa/tests/t109_pre_split_ports.rs` | `8159df336731871fd3205e09d8b0dd740a679afd1899a1c05b8db8f3a4f9cdf0` |
| `test/qa/tests/t110_post_split_ports.rs` | `fb62c975a762cfbfe34006650eeee1d6371b124bfcaa0e086b361cd078b7288a` |
| `test/qa/tests/t126_shared_private_test_support.rs` | `eebf2a77cff72e17a40fc42367ba319646358695327fedcab67c63a6e2b38907` |

## Renewed review after a surviving mutation

On 2026-09-23 the checkpoint damage stage replaced the moved-input guard in `test/qa/src/lane_runs.rs::Observations::observe` (`if inputs_digest(root) == before {` became `if true {`); every public and private check stayed green. The public reuse control left its mid-run edit in place, so the next observation saw a different digest and ran again whether or not the guard existed. The oracle repair writes the pre-run bytes back before observing again and requires a fresh run: with the guard the moved run was never kept; without it that run sits under the pre-run digest and would be reused. `test/qa/src/lane_runs.rs` is unchanged. The independent shared-source reviewer accepted the repair before the checks were rerun: the mutation is killed by reasoning over the code, no earlier assertion was removed, and no new defect was found. The table above keeps the initial digest; the renewed reviewed digest is:

| Reviewed input | SHA-256 |
| :--- | :--- |
| `test/qa/tests/t126_shared_private_test_support.rs` | `130341201c5b0c0b65a5a2c1061db9c13b41977773cc423b70fcc881060068a7` |

## Renewed review of the migration inventory exemption

On 2026-09-23 the final workspace proof failed in `t110_post_split_ports`: private lane t-084's rebrand audits flagged `.ratmac/evidence/run-037/migration-inventory.toml`, which had become indexed in the safety checkpoint. Its two hits (lines 104 and 182) are historical compiled test names that contain the retired position word, the same two already exempted for `baseline-inventory.toml`. The repair appends one exact-path row for that word to the rebrand allowlist. The shared-source reviewer accepted it with no findings: it is the narrowest supported token, the file has no other hits, renaming the identities would break WCP-002's preserved-identity guarantee, and no audit logic or other rule changed.

| Reviewed input | SHA-256 |
| :--- | :--- |
| `test/qa/fixtures/rebrand-audit/allowlist.tsv` | `64c7bace66e7abf35c34001ef4baa3ff55e575d8d8acbf6c8976a20ecc22a70c` |
