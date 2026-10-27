# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WTC-002` | When `python tools/verify_issue_binding.py check` meets the item record the turn is bound to, or a gap record that item lists, and that record exists but cannot be opened or read, it refuses with exactly one line on standard error in its usual refusal form. That line holds the fixed `verify_issue_binding: ` prefix, the record's kind as its existing rows name it (`item record` or `residual record`), the record's path, and the read error. It exits 1, prints nothing on standard output, writes nothing, and never prints a Python traceback. The validator copy packaged in the plan-build profile behaves identically and stays byte-identical to the tool. | accepted | Accepted under Billy's 2026-10-26 wish selection after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-same-verdict-and-windows-timing-requirements) |

## Evidence and boundaries

- In [the validator](../../../tools/verify_issue_binding.py), `def check` (line 268) tests the item record with `os.path.isfile` and refuses `the item record <path> does not exist` if it is missing (line 289). It then reads the record through `def read_record` (line 290). Each gap record gets the same pair (lines 319 and 320). `def read_record` (lines 169-171) opens the file with no error handling, so an `OSError` escapes as a traceback.
- `def refuse` (lines 35-37) prints `verify_issue_binding: <message>` on standard error and exits 1. The declared-roots read already refuses its read error this way, with `cannot read the declared roots from ...` (line 227).
- The wish's evidence is [the selected issue completion review record](../../../.ratmac/evidence/run-055/review.md), line 9. It says an item or gap record that exists but cannot be opened prints a traceback, because the refusal table has no row for it. The wish places that table at `.arca/ticket/t-133.md`; it now sits in [the archived selected issue completion ticket](../../ticket/archive/t-133.md) (`t-133`), lines 354-367. An archived record keeps its bytes, so the new rows belong in the forward authority and the new ticket.
- The profile copy, [the packaged checker](../../../profiles/plan-build/checkers/verify_issue_binding.py), is byte-identical to the tool today. [The profile manifest](../../../profiles/plan-build/manifest.toml) records its content digest (lines 54-57). [src/profile.rs](../../../src/profile.rs) embeds both files (lines 110 and 155) and refuses an export whose artifact digest differs from the manifest (`fn validate`, line 208). The profile byte-identity check (`WCPV-019`) in [the profile test](../../../test/qa/tests/t135_distributable_plan_build_profile.rs) (lines 1499-1503) compares the two files. No live test, source file, or runbook pins the digest itself; archived records and evidence snapshots keep the old value as history.
- Refusal tests run the validator from [the selected issue completion test](../../../test/qa/tests/t133_selected_issue_completion.rs) (`fn run_validator`, line 1539).
- Out of scope: `def read_or_empty` (lines 174-179) reads the selected issue's `spec.md` and `test-plan.md`. It turns a read error into empty text, so an existing refusal row answers, not a traceback.
- Also out of scope: `def live_entries` (line 137) skips a spawn ledger it cannot open or parse (lines 149-152). That is a quiet skip, not a crash. Refusing on it would let one unrelated Run's damaged ledger block every bound turn, so it stays as is and is named as an adjacent risk.

## Assumptions

- The wording follows the declared-roots row: `verify_issue_binding: cannot read the item record <path>: <reason>` and `verify_issue_binding: cannot read the residual record <path>: <reason>`. `<path>` is shown as the existing rows show paths (forward slashes), and `<reason>` is the operating system's reason text.
- "Exists" keeps today's meaning, the validator's file-existence test. A record path that is absent or is a folder keeps today's `does not exist` refusal.
- The plan-build profile's copy and its recorded content digest change with the tool in the same change.

## Dependencies

- Selected issue completion (`WCP-003`): the validator's refusal family and its read-only, write-nothing contract stay intact.
- Distributable Plan-Build profile (`WCP-007`): the packaged checker stays byte-identical, the manifest's recorded digest matches it, and the profile byte-identity check (`WCPV-019`) keeps passing.
- No other selected issue touches the validator.

## Open decisions

The exact reason text (the error's short reason or its full text) is left to the ticket. So is how a test makes a record unreadable on each platform: holding it open with no sharing on Windows, or removing read permission elsewhere. The ticket confirms on the host that the existence test still passes while the open fails. The disposition is set at integration; the linked forward authority then governs implementation.
