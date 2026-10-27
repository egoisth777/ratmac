# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WTC-003` | `rtm skill <folder>` finishes moving its partial folder into place when another program briefly holds a just-written file inside it open: on Windows a move refused with an access denial (raw code 5) is retried within the lock module's five-second wait bound and succeeds with today's output once the hold ends. A denial outlasting the bound, or any other move failure at once, refuses with today's message `rtm: skill: cannot write <path>: <error>` (exit code 2), tries to remove the partial folder as today, and leaves the requested path absent. Before every attempt it checks the requested path; if something exists there, it tries to remove the partial folder and refuses with today's `already exists` refusal instead of moving or waiting. | accepted | Accepted under Billy's 2026-10-26 wish selection after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-same-verdict-and-windows-timing-requirements) |

## Evidence and boundaries

- `fn write_skill_in` in [src/skill.rs](../../../src/skill.rs) (line 86) builds the whole folder at the hidden partial sibling (`fn write_partial`, line 159; suffix `rtm-skill-partial`, line 31). It moves the folder with one `std::fs::rename(&partial, path)` (line 130). On any error it tries to remove the partial folder, ignoring a failed removal (line 133), and returns the unwritable refusal (line 50), which prints `skill: cannot write <path>: <error>`. The command maps that refusal to exit code 2 ([src/cli.rs](../../../src/cli.rs) line 1898), and the binary adds the `rtm: ` prefix. Nothing in the skill write takes a lock or retries.
- The wish's evidence: in 25 runs of [the single root resolution test](../../../test/qa/tests/t119_single_root_resolution.rs) on the lock-wait ticket's branch (`t-136`), which does not touch the skill write, three failed. Two failed in an `rtm skill` call with `Access is denied. (os error 5)` (one call per route, `WEBV-017`, lines 884 and 906). A third failed once in the divergent resolver check (`WEBV-018`, line 1448); its message was not kept, so its route and cause are unknown. The likely cause of the two, not proven, is a brief hold by another program.
- The lock module ([src/lock.rs](../../../src/lock.rs)) holds the five-second wait bound (`WAIT_TIMEOUT`, line 47) and a private ten-millisecond retry interval (line 48). Its Windows open classifier (`fn classify_open_failure`, line 879) names access denied (code 5) and sharing violation (code 32). Its release-window test (`fn transient_release_window`, line 851) is written for lock files and treats a folder as a planted refusal, so it does not fit a folder move.
- The only skill test seam today aborts mid-write (`fn abort_mid_write_if_requested`, line 146 of the skill module, compiled with the `test-fault-injection` feature); no seam pauses before the move. The existing pause pattern is `PauseSpec` with `fn hold_pause_if_requested` in [src/nested.rs](../../../src/nested.rs) (line 1016).
- Out of scope: `rtm profile` moves its exported folder with the same single try ([src/profile.rs](../../../src/profile.rs) line 314). The wish records no failure there, so it is named only as an adjacent risk.
- Also out of scope: checking and then moving cannot exclude a path created between the two (on Linux a move replaces an empty folder). That narrow race exists today and stays.
- Also out of scope for the checks: the wish's "so a full test run never fails on it" rests on chance holds that no check can force. The ticket's proof instead repeats `cargo test --offline -p ratmac-qa --test t119_single_root_resolution` 25 times on this host, the count in the wish's evidence, and records zero `rtm skill` failures; any other failure is reported with its message.

## Assumptions

- The bound is measured from the first denial of the move, as lock waits measure theirs.
- Only an access denial (raw code 5) is retried; it is the denial the wish records. A sharing violation (raw code 32) refuses at once like any other failure.
- Other platforms keep the single try, because the recorded denial is a Windows behavior.
- If the hold also blocks deleting the partial folder, the folder stays behind exactly as today. The next `rtm skill` at that path names it with today's leftover refusal.
- The `already exists` refusal is today's occupied-path refusal, unchanged, with exit code 2: `rtm: skill: <path> already exists; the skill write never overwrites - choose a new path or delete that one yourself`. For a path that appears during the write, it replaces today's move error.

## Dependencies

- The operator skill folder (`AOP-003`): one folder, never overwriting.
- Single root resolution (`WEB-005`): the retry reuses the resolved roots and resolves nothing again, so one call per route (`WEBV-017`) and divergent resolver (`WEBV-018`) keep passing.
- Other selected issues: the abandon release-window issue (`i-072-abandon-release-window`) and the resumed child proof (`i-073-endpoint-release-window`) also lean on the lock module's wait bound. None of them is a prerequisite.
- The new pauses' tests must not break two selected test-isolation rules. The whole pause marker issue (`i-067-whole-pause-marker`, requirement `SVE-005`) has a test act only on a marker's whole line. The case-scoped hold barrier issue (`i-068-case-scoped-hold-barrier`, requirement `SVE-006`) keeps a hook armed through the environment inside its own case's commands. Neither is a prerequisite.

## Open decisions

Two choices are left to the ticket. One is where the small Windows denial classifier lives: the skill module, or a crate-visible helper beside the lock module's classifier. The other is how the test's hold is opened: the sharing mode that reproduces the recorded denial on the host. The disposition is set at integration; the linked forward authority then governs implementation.
