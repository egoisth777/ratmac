# Issue specification

Dispositions were decided at the 2026-08-21 planning pass (P1) that also
minted this issue; the batch was signed by Billy's sprint authorization of
the same day.

`ELR` is this issue's stable requirement-ID prefix - **Edition Ledger
Recording** - defined in [ubi-lang.md](ubi-lang.md).

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `ELR-001` | The edition tag is cut at the proven rest commit; its ledger row is appended in the next landing - the recording landing - citing the tagged commit's full hash, and is never edited afterwards. The old wording, one landing writing both the row and the commit it cites, is retired: a commit cannot contain its own hash, and the edition-002 and edition-003 corrections are the recorded proof. | accepted | A rule no landing can satisfy manufactures the exact disagreement its audit exists to catch. Recording after the fact is the only order in which the row's hash can be true on first write. | [working authority](../../../schema.md#elr-001---the-recording-landing-follows-the-tag) |
| `ELR-002` | The stable-channel bootstrap resolves the newest edition row and verifies ledger/tag agreement in the invoking project's current checkout, then locates or builds the engine from the tagged commit in a clean separate checkout whose tree is identical to that commit. It never requires the tagged commit's own ledger to agree with its tag, and it refuses when the build checkout's tree differs from the tagged commit. | accepted | At any tagged commit the ledger row citing it does not exist yet by ELR-001, so reading the ledger where the build runs can never succeed honestly; the invoking checkout's ledger is the current record the audit protects. | [goal spec](../../../goal/spec.md#integrated-edition-ledger-recording-requirements) |
| `ELR-003` | The claim that every sprint necessarily starts exactly at an edition is retired. The close guard `EDN-002` mandates is unchanged - it proves the commit being left is tagged - and `ECP-003` pins the driving engine, never the source tree under judgment. Between-sprint shop-lane landings, including the recording landing itself, legitimately follow the tag before the next start; no new start restriction is introduced. | accepted | `ELR-001` and the old consequence sentence cannot both hold: the ledger row the next stable bootstrap needs lands after the tag. The guarantee is restated as exactly what the guards prove, nothing more. | [working authority](../../../schema.md#elr-003---a-sprint-starts-after-the-tag-not-at-it) |

## Acceptance criteria

- The Editions rules in the working authority state the recording-landing
  order, and the old same-landing wording survives only in archived records
  and history.
- `pwsh -File tools/rtm.ps1 -Channel stable`, invoked from a healthy `main`
  whose newest ledger row agrees with its tag, produces a stamped stable
  engine without any hand edit to any checkout - including when the tagged
  commit's own ledger predates its row's correction.
- The bootstrap refuses when the stable build checkout's tree is not
  identical to the tagged commit.
- The edition audit's comparison of ledger rows against the tag database is
  unchanged: a missing, blank, or partial row is still a refusal.
- The working authority nowhere claims a sprint starts exactly at an edition,
  and it introduces no new start restriction in that claim's place.
