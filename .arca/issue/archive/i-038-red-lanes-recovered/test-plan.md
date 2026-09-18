# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `RLRV-001` | `RLR-002` | A fresh sweep report over this repository carries a `pass` row for every crate `t-058`..`t-070`, and each of those crates still declares exactly its six lane ids (`ht_NNN_01`..`ht_NNN_06` or the crate's original six names) - a port that drops or renames a lane is caught by the count and the names. |
| `RLRV-002` | `RLR-003` | The same report carries a `pass` row for each of `t-083`, `t-085`, `t-092`, `t-093`, `t-095`, `t-096`, `t-100`; a fixture listing containing `.ratmac/log.md` is found by the baseline helper's membership check, so the newline defect is dead. |
| `RLRV-003` | `RLR-004` | `.ratmac/lanes.toml` names `t-108`; `python tools/sweep_lanes.py check` exits `0` on this repository; on a fixture whose folder holds a crate the roster does not declare, the sweep's report names the stray and `check` exits `2` naming it. |
| `RLRV-004` | `RLR-004` | `python tools/sweep_lanes.py sweep --verify-expired` on this repository runs `t-078` and `t-079` and reports them red when run, while a plain sweep still reads them `expired` - the markers stay honest and stay in force. |
| `RLRV-005` | `RLR-001` | The working rules carry the triage rule as a requirement-ID heading; each recovery ticket's notes carry one row per crate naming its class and the landing that changed the contract; no crate `t-058`..`t-070` carries an expiry marker. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | unaffected | Every ask resolves to the working authority; the goal bundle gains no row, so the goal revision is unchanged by integration. |
| `.arca/goal/ubi-lang.md` | unaffected | The triage classes bind contributors, not the program; they live in the shop glossary. |
| `.arca/goal/spec.md` | unaffected | No product requirement row is minted (`PCR-008`). |
| `.arca/goal/design.md` | unaffected | No Engine mechanism changes; ADR-0020 already places lanes outside the Engine. |
| `.arca/goal/test-list.md` | unaffected | The checks above are cited by the tickets from this file, as `EDNV-*` and `GPHV-*` were. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | Still points at the schema. |
| `.arca/schema.md` | updated | New section "Landed-lane recovery" with headings `RLR-001`..`RLR-004`; each heading cites this issue's requirement records. |
| `.arca/dict.md` | updated | `RLR` registered under the Requirement ID entry; entries for triage class, retired contract, fixture drift, live regression, port, and roster. |
| `.arca/wishlist.md` | updated | The 2026-08-25 wish names this issue as its carrier. |
| `.ratmac/lanes.toml` | updated by ticket | `t-108` joins the roster (`RLR-004`). |
| `tools/sweep_lanes.py` | updated by ticket | `check` refuses a stray entry (`RLR-004`). |
