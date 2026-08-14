# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `ECPV-001` | `ECP-001` | An `[engine]` record carrying provenance round-trips through Run evidence; a running engine whose `source-commit` or `channel` differs from the recorded pin is refused with the same diagnostic class as an identity mismatch, and nothing is updated in place. |
| `ECPV-002` | `ECP-002` | On a fixture repository with an editions ledger, the bootstrap resolves `stable` to the newest ledger row's commit and `nightly` to the current landing, offline and deterministically; a ledger/tag disagreement refuses rather than picking a side. |
| `ECPV-003` | `ECP-003` | `rtm doctor` reports channel and provenance; a fixture with a live Run whose recorded engine is non-stable yields exactly one finding naming the mismatch, and a matching stable pin yields none. |
| `ECPV-004` | `ECP-001`..`ECP-003` | This repository's own cycle: a Run started by the stable engine records stable provenance, and the full suite, doctor, and link check stay green. |
