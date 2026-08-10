# Two packages build the same engine binary, so one test run cannot be trusted

```yaml
issue-id: "i-027-duplicate-engine-binary"
provenance: "Found while taking the P2 baseline for the state-vocabulary sprint on 2026-08-10: `cargo test --workspace` failed one blocked-route check that passes under `cargo test -p ratmac-qa`, and the difference is which package wrote target/debug/rtm.exe last (.arca/residual/res-122.md)."
status: "pending"
```

## Summary

The repository declares the engine command twice. The main package builds
`rtm` from `src/bin/rtm.rs`, and the test package builds a second `rtm` from
the very same source file with one extra build option turned on — the option
that compiles in the pause points the tests need to hold the engine still
mid-write. Both land at the same output path, so cargo prints an output
filename collision warning and the last writer wins.

The consequence is not a warning, it is a lie: a full-workspace test run can
report a red check that is green, or a green suite built from a binary the
tests did not intend. Measured at `4f78de5`, after a workspace build the
binary contains none of the pause-point wiring; after a test-package build it
contains it, and the affected check flips accordingly.

Nothing about the engine's behavior is wrong. What is wrong is that the shop
cannot state, in one command, that the suite is green — and that sentence is
the evidence half of every ticket the shop lands.

This issue asks for one arrangement in which the tests always run the binary
they built. It proposes no engine behavior change.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
