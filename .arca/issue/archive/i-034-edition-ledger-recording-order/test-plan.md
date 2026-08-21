# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| ELRV-001 | ELR-001 | The working-authority Editions rules read the recording-landing order; a scan finds no live rule requiring a row and the commit it cites in one landing. |
| ELRV-002 | ELR-002 | On a fixture repository whose tagged edition commit carries a stale ledger row while the invoking checkout's row agrees with the tag, the stable bootstrap resolves, builds in a clean checkout, and stamps provenance; the same invocation refuses when the invoking checkout's row and tag disagree, and refuses when the build checkout's tree differs from the tagged commit. |

| ELRV-003 | ELR-003 | The working authority nowhere claims a sprint starts exactly at an edition and introduces no new start restriction; the close guard's probe is unchanged. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/spec.md` | updated | ELR-002 |
| `.arca/goal/test-list.md` | updated | ELRV-002 |
| `.arca/goal/ubi-lang.md` | updated | Recording landing |
| `.arca/goal/design.md` | updated | bootstrap resolve/build split |
| `.arca/goal/index.md` | updated | edition-ledger-recording section |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `.arca/schema.md` | updated | EDN-003 revised; ELR-001 heading added under Editions |
| `.arca/wishlist.md` | carrier | the 2026-08-21 stable-bootstrap wish is fulfilled by the ticket that lands ELR-002 and leaves the file in that landing |
