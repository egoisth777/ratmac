# Issue design

## Proposed mechanics

Update working order, ticket template guidance, and the workflow states together when integrated. The order is: public checks green; independent source and test-oracle review accepted; full private-lane and quality checks green; safety checkpoint created from that reviewed green work; deliberate damage with checkpoint restoration and mandatory restored-green verification after each mutation; landing and merge; required post-merge verification on main. Resolve review findings and record the accepted snapshot before the private checks; a failed check returns to repair and invalidates affected review or proof. The safety checkpoint is never created before all required checks are green. Before accepting final evidence, compare it and the review against their declared source/test roots. Record command-order events in fixture execution so a wording change alone cannot certify behavioral ordering. The command trace distinguishes earning the reviewed private/damage proof from restored-green reruns after each mutation and the required post-merge verification on main. Those mandatory reruns remain intact and are never suppressed by the once-only claim.

The mechanics implement [WCP-004](spec.md#requirement-records) and are tested by the [verification plan](test-plan.md#verification).

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
