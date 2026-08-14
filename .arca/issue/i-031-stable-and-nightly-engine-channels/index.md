# Stable and nightly Engine channels

```yaml
issue-id: "i-031-stable-and-nightly-engine-channels"
provenance: "Billy's ask of 2026-08-13, filed as a wish the same day - self-hosting needs the driving engine pinned to a proven edition while the source moves"
status: "integrated"
```

## Summary

Since the i-015 sprint, this repository's own P1-P5 cycle is driven by `rtm` - built
from the very tree the Run is judging. A landing that breaks the engine is graded by
the broken engine: a self-clobbering loop with no floor. The bootstrap problem has a
known answer - the compiler that builds the compiler is a *previous, proven* release -
and this repository already owns every ingredient: editions are annotated, audited,
immutable stable markers (`EDN-001`..`EDN-003`); the Stable Engine pin exists
(`src/pin.rs`, `Evidence::engine`: resolved path + sha256, where a differing identity
is a refusal, not an update); and `ORS-002` mandates the deterministic bootstrap that
locates or builds the Stable Engine and verifies it against that pin.

What is missing is provenance and resolution: the pin records *which bytes* but not
*which edition or channel* those bytes were built from, so "stable" and "nightly"
cannot be told apart, resolved, or refused. This issue extends the existing pin -
never a second pin mechanism - with edition/channel provenance, and teaches the
existing bootstrap to resolve the two channels:

- **stable** - the engine built at the newest `edition-NNN` tag; drives Runs that
  judge landings on this repository; the publish artifact.
- **nightly** - the engine built at the latest green landing; dogfooded in trials
  and ticket worktrees; never gates its own promotion.

Promotion is not a new ceremony: cutting an edition already demands the proof block,
so stable is *defined* as the newest edition.

## History

- 2026-08-13: filed from Billy's ask; three requirement records proposed, all
  routed through the existing `Evidence::engine` pin and the `ORS-002` bootstrap.
- 2026-08-13: P1 integrated all three asks as accepted goal rows ECP-001..ECP-003; run-006 carries the sprint.
