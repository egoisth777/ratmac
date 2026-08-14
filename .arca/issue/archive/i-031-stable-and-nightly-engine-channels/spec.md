# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises
them at integration.

`ECP` is this issue's stable requirement-ID prefix - **Edition-Channel Pin**,
defined in [ubi-lang.md](ubi-lang.md). Every record extends the existing Stable
Engine pin (`Evidence::engine`) and the `ORS-002` bootstrap; none introduces a
second pin mechanism.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `ECP-001` | The Stable Engine pin carries build provenance beside its identity: the source commit (full hash) and either the edition name the binary was built at or the channel marker `nightly`. A recorded pin whose provenance differs from the running engine's is a refusal, not an update - the same rule the identity already carries. Provenance lives in the existing `[engine]` record; no second pin file exists. | accepted | The pin already proves *which bytes*; without *which edition* the channels cannot be told apart, and a stale stable binary is indistinguishable from a current one. | [goal spec](../../goal/spec.md#integrated-edition-channel-pin-requirements) |
| `ECP-002` | The `ORS-002` bootstrap resolves a requested channel: `stable` locates or builds the engine at the newest `edition-NNN` tag recorded in the editions ledger; `nightly` locates or builds it at the current landing. It verifies the result against the recorded pin when present, records provenance on first resolution, and reports channel, edition (or commit), path, and identity - still with no global installation, PATH mutation, or network fetch. | accepted | Resolution must be deterministic and local, and the ledger (not the tag database alone) is the record a moved tag cannot silently satisfy. | [goal spec](../../goal/spec.md#integrated-edition-channel-pin-requirements) |
| `ECP-003` | A Run that judges a landing on this repository is driven by the stable-channel engine; the nightly channel may drive trials and ticket-worktree work but never gates its own promotion. Argument-free `rtm doctor` reports the running engine's channel and provenance, and a running engine that does not match the recorded stable pin while a Run is live is a doctor finding, not a guess. | accepted | Evidence-not-claim at the binary level: the engine under change must not grade the change. The doctor is the existing `ORS-002`/`DRD-005` report surface, extended by one row. | [goal spec](../../goal/spec.md#integrated-edition-channel-pin-requirements) |

## Acceptance criteria

- The `[engine]` record in Run evidence carries provenance, and a provenance
  mismatch refuses exactly as an identity mismatch does today.
- One bootstrap command resolves either channel deterministically and offline,
  and its stable answer is derived from the editions ledger.
- `rtm doctor` names the running engine's channel; a live Run driven by a
  non-stable engine is a reported finding.
- No second pin file, no PATH mutation, no network fetch, and `DEB-001`/`DEB-002`
  (one engine binary per build target) still hold.
