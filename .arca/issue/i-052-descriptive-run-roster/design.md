# Issue design

## Proposed mechanics

- Keep the canonical identifier listing used internally separate from a human-readable roster renderer. Read the per-Run records and parent ledgers once for the report and use their stored facts, without interpreting an opaque binding as a ticket title.

- Use consistent rendering in missing/invalid-address diagnostics and the roster printed by status. Show unknown or unreadable facts by path and reason; an unreadable ledger must not license describing an uncertain child as top-level.

- Preserve stable ordering and escape control characters in rendered opaque values. No new persistent display name, database, external lookup, or lifecycle command is needed.

## Scope and assumptions

These are minimal proposed mechanics under Billy's 2026-09-22 instruction to proceed without questions. They authorize no source edit, runtime mutation, historical rewrite, or completed-work claim in this issue-creation stage.

No new hard dependency. Shares read-only historical-record handling with terminal-history status; successor publication must not expose an ownership-free row.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.

