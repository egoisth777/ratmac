# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at
integration. `DEB` expands to **Duplicate engine binary** and is this issue's stable
requirement-ID prefix, defined in [ubi-lang.md](ubi-lang.md).

Both asks are repairs with an obvious shape, not forks a human must settle, so both are
proposed `accepted`.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `DEB-001` | One command proves the suite. The test package and the shipping package never write the same build output, no build of the repository prints an output filename collision, and every test that launches the engine launches the build it was compiled against — including the pause points that let a test hold the engine still mid-write. | accepted | Without this, a green suite is a claim about build order, not about the code. Measured at `4f78de5`: after a workspace build the binary carries none of the pause-point wiring and one blocked-route check fails; after a test-package build it carries the wiring and the same check passes. Recommended shape: the test package's copy gets its own target name and the tests name that target; the shipped command stays `rtm`. Turning the pause points on in the shipping build is refused — test-only wiring never ships. | |
| `DEB-002` | A collision cannot come back silently. The build refuses, or a check fails by name, if any two build targets in the repository ever resolve to the same output path again. | accepted | Cargo prints this as a warning today and says it may become a hard error later; the shop should not wait for the toolchain to decide when its evidence starts being honest. One check protects every green claim the shop makes. | |

## Out of scope

No engine behavior changes here: routing, guards, locking, receipts, and exit codes are
untouched. The state-vocabulary cutover is a separate sprint and depends on this issue only
for how its regression evidence is taken, which its gap record already names
([res-122](../../../residual/res-122.md)).
