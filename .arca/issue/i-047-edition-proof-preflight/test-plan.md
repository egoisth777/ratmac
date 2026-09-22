# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WCPV-022` | [WCP-008](spec.md#requirement-records) | Candidate text missing each required proof element is rejected before any tag exists; accepted candidate text passes the same validator used by the audit. Extend test/qa/src/edition.rs and t095_edition_audit.rs fixtures. |
| `WCPV-023` | [WCP-008](spec.md#requirement-records) | Dirty, stale-proof, failed-check, moved-target, duplicate-name, invalid-sequence, and unreachable-target fixtures leave tag refs and existing annotations unchanged. Independently plant pending intake, an open work item, or an unproven gap in an otherwise clean committed fixture whose quality commands all pass; each unfinished-cycle case still refuses before creating a tag. Use isolated repositories with before/after ref and tree snapshots. |
| `WCPV-024` | [WCP-008](spec.md#requirement-records) | A proven clean fixture creates exactly one annotated tag with exact candidate bytes and checked target. Its annotation and structural audit pass immediately; the full audit_sequence still reports the missing ledger row at that boundary. Only the later ledger-recording landing prescribed by ELR-001 makes the full sequence audit pass. Preserve that ordering and re-run cut to verify no overwrite. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../schema.md#wcp-008--edition-proof-preflight). |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../schema.md#wcp-008--edition-proof-preflight). |
| `.arca/goal/spec.md` | unaffected | [Accepted authority](../../schema.md#wcp-008--edition-proof-preflight). |
| `.arca/goal/design.md` | updated | [Accepted authority](../../schema.md#wcp-008--edition-proof-preflight). |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../schema.md#wcp-008--edition-proof-preflight). |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | updated | [Accepted authority](../../schema.md#wcp-008--edition-proof-preflight). |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
