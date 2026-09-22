# Independent review before full proof

Accepted on 2026-09-22, before executing the private lanes or full workspace proof.

- The independent public-proof author reviewed source, migrated consumers, inherited private fixture ports, and the new private oracle; the author's own public test file is excluded from that verdict.
- The independent private-proof author reviewed source, migrated consumers, inherited private ports, and the final public oracle; the author's own private crate is excluded from that verdict.
- Both reviewers accepted the shared source. Each new oracle has an independent reviewer. The coordinator re-derived the reviewed digests below before recording acceptance.
- Review corrections preserve selected bytes through callers, refuse linked build inputs by name, retain indexed membership in source-copy fixtures, preserve Unix filename identity, and replace recursive private-input discovery with exact named files.
- Four public checks pass against these bytes. At initial acceptance, private checks had compiled but had not executed. Any later source or oracle repair requires a renewed affected review and proof.

| Reviewed input | SHA-256 |
| :--- | :--- |
| `test/qa/src/audit_files.rs` | `010dfe97da0f3cd602dde85fcbd5073e9cb56c202a8be64fa9f75facba83cc5e` |
| `test/qa/src/lib.rs` | `163758b7a63b197e321fc7c36bbf6f174b33561928342f08d5aab2b4869c8875` |
| `test/qa/src/rebrand.rs` | `d0513d27bba7ab24d4ce1f702164cee26fbc26f008678c52fb1bf4a6837e85ca` |
| `test/qa/src/targets.rs` | `cb37824b81cba5c1b5d44979b88836e9e8ef50d1cb15bae6890108b7c530e803` |
| `test/qa/tests/t033_rat004.rs` | `e1c1aa52cc850997f48101e14f9bcadc5697fec93c3135dcbe7502f5a2d3cdb5` |
| `test/qa/tests/t047_receipts.rs` | `9e32bcd466b749efe557fd6c8f5817b001130912826b6dbc02c1165652342a5a` |
| `test/qa/tests/t056_typed_parser.rs` | `589737f2cc22bb7131e6158f742e3c162c631f1ac95de55b9cbd2dd2869a71ca` |
| `test/qa/tests/t076_roots_table.rs` | `65119c0920182706b398cf3774bd02d1cdcec7109fdef64b3fc4a0b5a9af2437` |
| `test/qa/tests/t079_root_spelling.rs` | `b78439ede22f03cf0c72989d0c05406cd0f10ad015869a2dac01ff948a1414ed` |
| `test/qa/tests/t084_state_audit.rs` | `fb3011f34981c695e0059a188733610faa9b2e9b28fea390b84c64526dbf091b` |
| `test/qa/tests/t086_single_engine_binary.rs` | `61d9d62757c67e25220c0eced19c9affab1aa7acb55eef87e686384ef1cd9cf3` |
| `test/qa/tests/t087_no_work_item.rs` | `412deae7383213313253984500d94308ed5ad61f30e36b3082376b1322b45da0` |
| `test/qa/tests/t118_tracked_repository_audits.rs` | `ee19f0098ab3049ea65f69e938b8da5e950edfcf3c1192704767d6211d1130bc` |
| `test-hidden/t-084/tests/hidden.rs` | `c1ee507ce9ba921b5600bd99670233fd4792a217eaadaacb3b6b3de95006c64c` |
| `test-hidden/t-086/tests/hidden.rs` | `3eb2d0efa1edb00b5ad65d05cc1fd93ed0a41819f762bf578aa3905181b32865` |
| `test-hidden/t-118/Cargo.toml` | `91615acb014f94c118abc2e8313898f252314fc6419c9771e33494aad7764dc6` |
| `test-hidden/t-118/src/lib.rs` | `b9202b1f5224bb4e16d5913923b7b4146f979b8843ad163eda40dee3d1be0283` |
| `test-hidden/t-118/tests/hidden.rs` | `08a3f241690024a81fb119fda93cd88e5008a944ea825c7ac7577240f03140af` |

## Renewed inherited-fixture review

After initial acceptance, all four new private checks passed. The inherited audit crate passed five checks; its sixth failed during Git indexing of the copied corpus because an existing receipt path exceeded the Windows Git path limit. The repair adds only command-local `core.longpaths=true` and `core.autocrlf=false` options to its fixture Git helper. It changes no assertions and persists no configuration. Both independent reviewers accepted the one-line delta before the retry; removing that line re-derives the initial reviewed digest `3e91f2f6e316917b0d263096f5362b7e3e6b5b6b2d4ad94eb0aed2f82379a9e4`. The table above records the renewed digest.

The inherited target-audit crate passed five checks; its shipped-command identity check initially lacked the built executable expected by that existing fixture. The ordinary `cargo build --offline -p ratmac --bin rtm` prerequisite was supplied and the exact failing check then passed. No source or oracle was changed for that prerequisite.
