# Issue design

## Proposed mechanics

The bootstrap stops guessing where the Engine is and asks Cargo instead.

In [rtm.ps1](../../../tools/rtm.ps1), `function Invoke-EngineBuild` adds `--message-format=json` to its existing build, reads each standard-output line with `ConvertFrom-Json`, keeps `compiler-artifact` records whose target name is `rtm` and whose `executable` is not empty, and returns the one distinct path; none or several refuse through `function Deny`, naming the build root, what Cargo reported, and the repair. A failed build's guidance prints each `compiler-message` record's `rendered` text, then Cargo's standard error. `function Get-EngineCandidates` takes the build root's Cargo-resolved output folder from `cargo metadata --format-version 1 --no-deps --offline` (`target_directory`), run inside the build root like the build, instead of `<root>/target`, keeping release before debug, and refuses by name when Cargo cannot answer. For the stable channel, the bootstrap names the build checkout's own `target` to Cargo for the build (for example with `--target-dir`) and reuses only from there; with no channel or nightly, the main flow skips reuse and builds when the resolved folder lies outside the project root. The description's declared build output (lines 27-30) becomes the resolved output folder and `Cargo.lock`. Stamping, the pin check, the reports, and the stable clean-tree check stay as they are. The public bootstrap suites gain redirected fixtures and planted stand-ins, and the doctor suite's planted-binary fixtures name their own `target` for the bootstrap they start. No new dependency, runner, or registry.

The mechanics implement [SVE-002](spec.md#requirement-records) and are tested by the [verification plan](test-plan.md#verification).

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
