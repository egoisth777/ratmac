# Issue design

## Proposed mechanics

The move into place waits out a brief hold the same way lock waits do. The written bytes do not change.

In `fn write_skill_in` in [src/skill.rs](../../../src/skill.rs), the single move at line 130 becomes a short loop:

- Just before each attempt, including the first, the loop checks that nothing exists at the requested path. If something does, it tries to remove the partial folder and returns today's occupied refusal.
- On Windows, a move error with raw code 5 (access denied) is retried after a short sleep, until the lock module's wait bound (`WAIT_TIMEOUT`, five seconds, [src/lock.rs](../../../src/lock.rs) line 47) has passed since the first denial.
- Any other error, or a denial at the deadline, tries to remove the partial folder as today and returns today's unwritable refusal with the last error.
- Other platforms keep the single try.

No lock is taken and no dependency is added.

Tests need two test-only pauses, compiled only with the `test-fault-injection` feature like `fn abort_mid_write_if_requested`. The first comes before the first attempt's check of the requested path, so the test can open a file inside the partial folder or create something at the requested path before that check runs. The second comes after the first denied move, so the test ends its hold only after the denial has happened. Each follows the existing pause pattern in [src/nested.rs](../../../src/nested.rs): a selector, a marker file, a release file, and a timeout. Production builds compile them away.

The mechanics implement [WTC-003](spec.md#requirement-records) and are tested by the [verification plan](test-plan.md#verification).

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
