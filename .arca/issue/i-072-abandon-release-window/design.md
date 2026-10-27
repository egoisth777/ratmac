# Issue design

## Proposed mechanics

Abandon's lock check treats a release in progress the way the waiting lock acquirers already do.

`fn lock_path_present` in [src/abandon.rs](../../../src/abandon.rs) keeps its attribute read. Sometimes the read fails with an error the lock module classifies as a release window (`fn transient_release_window`, [src/lock.rs](../../../src/lock.rs) line 851). Then the check sleeps the lock module's retry interval and reads again, until the wait bound (`WAIT_TIMEOUT`, five seconds) has passed since the first denial, and returns the settled answer. Not-found still means absent. Any other error, or a denial at the deadline, returns today's refusal with the last error.

The smallest shape is one crate-visible helper in the lock module, so the classifier, interval, and bound stay in one place; `fn run_lock_is_contended` keeps its own deadline. Abandon still holds no lock while it reads. Other platforms keep the single read.

A held release window cannot make this read fail (see the specification's evidence). So the checks need one test-only fault, compiled only with the `test-fault-injection` feature like `fn inject_lock_fault` (line 1179 of the lock module). It is an environment selector that makes the first few presence reads of a named lock, or every read, fail with a chosen raw error code (5 for the release-window cases, 32 for the other-code control) and records each failure it injects in a file outside the project. The classification, the wait, and the settled read then run unchanged.

The mechanics implement [WTC-004](spec.md#requirement-records) and are tested by the [verification plan](test-plan.md#verification).

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
