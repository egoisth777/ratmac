# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WCPV-013` | [WCP-005](spec.md#requirement-records) | A fixture with line-ending conversion yields clean status but changed worktree bytes; verification refuses it by path. Use isolated Git configuration and LF/CRLF fixtures; no global configuration edits. |
| `WCPV-014` | [WCP-005](spec.md#requirement-records) | A byte-preserving checkpoint round-trip restores every declared file and produces matching recomputed digests; tampered manifests or missing targets refuse. Candidate: snapshot support tests and t090_damage_checkpoint.rs. |
| `WCPV-015` | [WCP-005](spec.md#requirement-records) | Unexpected clean/smudge filter behavior refuses before damage; supported restore failure never claims success or changes unrelated files. Fixture-local attributes/filter plus whole-tree snapshots. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../schema.md#wcp-005--byte-verified-restores). |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../schema.md#wcp-005--byte-verified-restores). |
| `.arca/goal/spec.md` | unaffected | [Accepted authority](../../schema.md#wcp-005--byte-verified-restores). |
| `.arca/goal/design.md` | updated | [Accepted authority](../../schema.md#wcp-005--byte-verified-restores). |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../schema.md#wcp-005--byte-verified-restores). |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | updated | [Accepted authority](../../schema.md#wcp-005--byte-verified-restores). |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
