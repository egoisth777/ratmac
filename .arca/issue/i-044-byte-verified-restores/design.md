# Issue design

## Proposed mechanics

The supported damage/restore operation stores a frozen manifest beside ticket evidence and reads it before every restore. Derive checkpoint target bytes and effective Git attributes/configuration, prove their checkout round-trip or refuse before permitting mutation, and verify actual worktree bytes afterward. Report mismatched paths explicitly and leave unrelated work untouched. Integrate this verifier into the checkpoint-only restore mechanism instead of creating a competing restore path.

The mechanics implement [WCP-005](spec.md#requirement-records) and are tested by the [verification plan](test-plan.md#verification).

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.

