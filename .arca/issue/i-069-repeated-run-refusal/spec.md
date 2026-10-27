# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WTC-001` | Every command that addresses a Run with `--run` (`status`, `step`, `intent`, `hold`, `resume`, `recover`, `spawn`, `respawn`, `abandon`) refuses `--run` given more than once, whether the values match or differ, and acts on none of them. It exits 2 with a message starting `<command>: --run given twice; address exactly one run; ` followed by the Run roster exactly as that command prints it when `--run` lacks a value; `abandon` keeps today's longer form. Before refusing it reads only what its roster output needs, after the retired-layout residue check that precedes every command; it checks no addressed Run Record, ledger, runbook, blocker, or confirmation phrase and writes nothing, so `rtm spawn rev --run run-001 --run run-002 --bind ticket=t-7` spawns nothing. | accepted | Accepted under Billy's 2026-10-26 wish selection after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-same-verdict-and-windows-timing-requirements) |

## Evidence and boundaries

- The shared addressing parser, `fn addressed_run_with_roots` in [src/cli.rs](../../../src/cli.rs) (line 183), refuses a second `--run` (line 193) with `"{command}: --run given twice; address exactly one run; {}{}\n{}"` (line 196): the roster summary, its rows, and a `next:` line. It refuses through `CliError::refusal`, whose exit code is 2 (lines 28-31); `CliError::new` exits 1 (lines 19-22).
- `rtm status` (line 645) and `rtm step` (line 573) use the shared parser. `rtm intent` collects its `--run` pairs (line 1168) and calls it at line 1273, after argument checks that read no file. `rtm abandon` refuses a repeat itself (line 1686) with `abandon: --run given twice; address exactly one run; {}; a Run is retired only by its own phrase: {}{}` (line 1689).
- Five handlers parse `--run` themselves and keep the last value with no repeat check: `fn hold` (arm at line 714, assignment at line 726), `fn resume` (lines 864 and 876), `fn recover` (lines 1022 and 1034), `fn spawn` (lines 1548 and 1560), and `fn respawn` (lines 1613 and 1625). Their own missing-value refusals print the roster summary and rows with no `next:` line and exit 2, for example `hold: --run needs a run id; {}{}` (line 722).
- The wish places these arms around lines 357, 483, and 542 and the shared parser around line 143; those line numbers are stale, and the lines above are current.
- Today's repeat refusal is pinned for `rtm status` with matching values in [the descriptive roster test](../../../test/qa/tests/t117_descriptive_run_roster.rs) (lines 765-768).
- No other command takes `--run`: `rtm endpoint` addresses its child with `--invocation`, and `init`, `start`, `doctor`, `scaffold`, `skill`, and `profile` take no Run address.
- Out of scope: other repeated options (`spawn: --workspace given twice` keeps its text and exit code 1), and the retired-layout residue refusal, which still comes before every command's own argument refusals.

## Assumptions

- A third or later `--run` refuses the same way, at the second occurrence.
- A repeat whose second `--run` has no value refuses as given twice, as in the shared parser. `rtm abandon` reads the value first (`src/cli.rs` lines 1671-1680, then 1686), so it keeps refusing that input with its `--run needs a run id` refusal.
- The five changed commands print the roster in the shape their own missing-value refusal already uses (summary and rows, no `next:` line), so each command's two addressing refusals stay one family. `status`, `step`, and `intent` keep today's shared message unchanged.
- Among pure argument errors, order follows the arguments, as in the shared parser: a malformed option written before the second `--run` still refuses first. Nothing that reads a file runs before the repeat is refused, apart from the residue check and the roster output.
- The exit code is 2, the code every existing addressing refusal already uses.

## Dependencies

- Run addressing (`FDC-004`): a missing, malformed, or unknown value still refuses with the roster.
- Human-facing Run roster output (`WRS-005`): the refusal prints the same descriptive rows.
- Abandon's address-first validation (`WRS-007`): `rtm abandon` keeps its own repeat refusal and phrase guidance.
- Retired-layout residue precedence (`WEB-004`): residue is still refused before any command's own argument refusal.
- No other selected issue is a prerequisite. The abandon release-window issue (`i-072-abandon-release-window`) changes the abandon module, not the abandon argument parser.

## Open decisions

Whether the five handlers check in place or share one small helper is left to the ticket, as long as each keeps its own roster shape. The disposition is set at integration; the linked forward authority then governs implementation.
