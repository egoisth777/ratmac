# Issue design

## Proposed mechanics

Inventory the live roster and identify duplicated fixture/launch/snapshot/fault setup, then extract only repeated behavior into the existing test support crate. Keep per-lane assertions in their owning crates. Child process environments are built explicitly instead of changing shared parent environment. Keep unique temporary roots, restore guards, and deterministic captured output. The current runner remains the aggregate entry point; report crate count and elapsed/build reuse measurements as observations, not a brittle timing gate. Publish the migration ledger in its owning ticket/evidence and leave archived issue, ticket, and residual bytes intact.

The mechanics implement [WCP-002](spec.md#requirement-records) and are tested by the [verification plan](test-plan.md#verification).

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.

