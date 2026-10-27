# Issue design

## Proposed mechanics

No product behavior change is planned: both endpoint places already retry a release window. The only source change is a test-only pause compiled with the `test-fault-injection` feature.

A check needs a real release window on the child Run's lock at each place. It also needs to know the endpoint met the window before the window closes.

- The window comes from a peer that holds the child Run's lock, such as the child's own `rtm step`, started with the lock module's unlink hold.
- For the terminal read, the endpoint's existing after-admission pause (`after-admit`) lets the test open the window after the resumed claim and before the read.
- To show the endpoint met the window, add one test-only pause at the first retry within each call of each place in [src/endpoint.rs](../../../src/endpoint.rs) (`fn terminal_snapshot` at line 310, and the resumed claim at line 731), with a boundary value naming the place (`claim` or `terminal-read`), so the test can tell which place met the window. Give it its own selector so it can be combined with the after-admission pause. It follows the pause pattern in [src/nested.rs](../../../src/nested.rs) and is compiled only with the `test-fault-injection` feature.

The test waits for that pause, ends the peer's hold, releases the endpoint, and asserts that the claim or read completed.

If the single try reports the window as not acquired, the endpoint never reaches the pause. The claim refuses `the child Run lock for <id> is held; retry once it frees`. The terminal read reports no terminal fact, so the resumed child's declared driver is launched again.

The mechanics implement [WTC-005](spec.md#requirement-records) and are tested by the [verification plan](test-plan.md#verification).

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
