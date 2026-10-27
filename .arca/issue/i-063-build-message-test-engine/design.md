# Issue design

## Proposed mechanics

The shared test support asks Cargo where it put a binary instead of guessing, and every helper that builds and then launches one uses that answer.

One function beside `today_engine()` in [the baseline support](../../../test/qa/src/baseline.rs) takes a build root and the caller's `cargo build` arguments, runs them in that root with `--message-format=json` added, reads standard output line by line with the test crate's existing reader ([json.rs](../../../test/qa/src/json.rs), so no new dependency), keeps `compiler-artifact` records whose target name is the requested binary and whose `executable` is not empty, and returns the one distinct path. None or several paths return an error naming the binary, the build command, and what Cargo reported. A failed build's refusal includes the `rendered` text of every `compiler-message` record and Cargo's standard error. `today_engine()` calls it with the repository root inside its existing once-per-process cache; the freeze `fn build` calls it with the freeze tree and its own output folder; `fn build_edition_cut` and the two building tests of the single-Engine-binary suite call it. Private lanes, the single-Engine-binary lane (`t-086`) included, replace their own readers, the refusal in the refusing lane (`t-116`), the fallback in the TOML-reading lane (`t-133`), and their fixed joins with calls into `ratmac_qa::baseline`, keeping each lane's identity and assertions. The general test support ([support.rs](../../../test/qa/src/support.rs)) stays standard-library-only, and the frozen before-source corpus is not touched. The helper neither sets nor clears the redirect variables; the existing scrub already lets them through.

The mechanics implement [SVE-001](spec.md#requirement-records) and are tested by the [verification plan](test-plan.md#verification).

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
