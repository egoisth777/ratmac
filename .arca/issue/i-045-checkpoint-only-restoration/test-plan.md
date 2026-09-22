# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WCPV-016` | [WCP-006](spec.md#requirement-records) | Two source generations plus a misleading latest backup cannot cause the supported restore to select stale bytes; only the recorded checkpoint is used. Isolated ticket-turn fixture based on tools/turn.ps1 and t090_damage_checkpoint.rs. |
| `WCPV-017` | [WCP-006](spec.md#requirement-records) | Backup-source, implicit-source, arbitrary-revision, untracked-lane, escaping-path, stale-checkpoint, and unrelated-unsaved-work cases refuse before destructive writes. Snapshot refs, index, target bytes, and unrelated files around each refusal. |
| `WCPV-018` | [WCP-006](spec.md#requirement-records) | Interrupt a damage check; recovery restores the same checkpoint and re-derives all frozen hashes, then permits the next check. Compose with byte-verified restores and existing turn recovery tests. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../schema.md#wcp-006--checkpoint-only-restoration). |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../schema.md#wcp-006--checkpoint-only-restoration). |
| `.arca/goal/spec.md` | unaffected | [Accepted authority](../../schema.md#wcp-006--checkpoint-only-restoration). |
| `.arca/goal/design.md` | updated | [Accepted authority](../../schema.md#wcp-006--checkpoint-only-restoration). |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../schema.md#wcp-006--checkpoint-only-restoration). |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | updated | [Accepted authority](../../schema.md#wcp-006--checkpoint-only-restoration). |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
