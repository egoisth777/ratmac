# Issue design

## Proposed mechanics

Serde derive on MachineClass and its parts; GuardKind as a tagged enum so unknown kinds fail at
deserialization with a field-and-line error (extend the existing MachineClassParseError rather than invent a
parallel error type). Per-kind required/forbidden field checks run immediately after deserialization, against
the guard-kind table from the runbook spec (i-011). `scheduler.rs` takes the typed MachineClass as input and
loses its own toml walk entirely. The load path returns `Result<MachineClass, MachineClassParseError>` all
the way up; call sites that today fall back to `MachineGraph::default()` propagate the refusal instead.
Existing R-002/R-003/R-011 tests stand as the behavioral net; new tests cover unknown kind, per-kind field
violations, and the missing-file refusal.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
