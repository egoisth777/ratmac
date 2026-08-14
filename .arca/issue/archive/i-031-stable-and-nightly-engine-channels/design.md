# Issue design

## Proposed mechanics

This file is incoming evidence: integrated mechanics remain authoritative only in
the accepted forward authority.

**1. Provenance rides the existing `[engine]` record (`ECP-001`).** `Identity`
in `src/pin.rs` stays as it is (resolved path + sha256). The `[engine]` table
gains two recorded fields beside it: `source-commit` (full 40-hex) and
`channel` (`"edition-NNN"` or `"nightly"`). `Evidence::record_gate`'s rule is
reused verbatim: a differing recorded value is a refusal, never an update. The
binary learns its own provenance at build time (compile-time env, the same way
the QA harness already locates `CARGO_BIN_EXE_rtm`), so the running engine can
answer "what am I" without reading the tree it judges.

**2. The bootstrap resolves channels from the editions ledger (`ECP-002`).**
The `ORS-002` command gains a channel argument defaulting to `stable`. Stable
resolution: read `.arca/editions.md`, take the highest `edition-NNN` row, build
or locate the engine at that recorded commit (a cached binary under the Engine
root is a convenience, verified by sha256 before use; a cache miss builds from
the tag's commit). Nightly resolution: build at the current landing. The ledger,
not `git tag` alone, is the stable source - a moved tag already fails the
edition audit, and the ledger row is the record a citation resolves against.

**3. Stable judges, nightly is judged (`ECP-003`).** The Scheduler records the
driving engine's provenance into Run evidence at `rtm start` (it already records
identity). The doctor compares: live Run + non-stable provenance = finding.
Enforcement stays at the reporting surface in this issue; a hard refusal at
`start` is deliberately left to a later ask once the first stable binary has
driven a full cycle - the same staged route the record contract took.

**4. Day-one bootstrap.** `edition-002` predates provenance, so the first
stable binary is built by hand once from the `edition-002` ledger row and its
provenance recorded then; the loop closes from that point forward.

## Non-goals

- No second pin file (`toolchain.toml` was considered and rejected - two pin
  mechanisms can disagree).
- No publishing, release naming, or network distribution.
- No change to how editions are cut, audited, or recorded.
- No hard start-refusal on channel mismatch in this issue; report first.
