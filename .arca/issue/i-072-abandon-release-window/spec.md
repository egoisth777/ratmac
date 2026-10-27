# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WTC-004` | While holding no lock, `rtm abandon` checks whether the project's root lock and the addressed Run's lock exist. That check can meet another command's release of one of those locks; on Windows a read taken at the moment the releasing command closes its handle can answer `Access is denied. (os error 5)`. Then abandon retries within the lock module's five-second wait bound and acts on the settled answer, present or absent, exactly as without the release. Only a read failure that outlasts the bound, or any other read error, refuses with today's text `abandon cannot inspect lock <path>: <error>` inside today's message for that route (on the planning route, `rtm: abandon refused; abandon cannot inspect lock <path>: <error>`), exit code 1. | accepted | Accepted under Billy's 2026-10-26 wish selection after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-same-verdict-and-windows-timing-requirements) |

## Evidence and boundaries

- `fn lock_path_present` in [src/abandon.rs](../../../src/abandon.rs) (line 462) reads the lock path's attributes with `fs::symlink_metadata` (line 463). Found means present, not-found means absent, and any other error refuses `abandon cannot inspect lock <path>: <error>` (line 467).
- That read runs with no lock held, in two places. An unaddressed abandon uses it to look for a leftover root lock (line 177, in `fn resolve_target_in`, line 143). A planned retirement uses it for the root and Run locks (lines 336-338, in `fn plan_abandon_in`, line 258). The command adds `abandon refused; ` and exit code 1 to planning refusals ([src/cli.rs](../../../src/cli.rs) lines 1744 and 1746). The same inner text also appears inside abandon's `--confirm` guidance refusals, which also exit 1.
- The waiting lock acquirers already treat this window as passing. The availability probe, `fn run_lock_is_contended` ([src/lock.rs](../../../src/lock.rs) line 705), classifies an access denial through `fn transient_release_window` (line 851): Windows code 5 while the path is not a folder, and never on other platforms. It keeps waiting until the deadline set by the wait bound (`WAIT_TIMEOUT`, line 47).
- The lock-wait ticket's (`t-136`) [review record](../../../.ratmac/evidence/run-059/review.md) records the same read failing inside lock waits (line 17). The private lock-contention case (`HT-136-02`) refused `cannot inspect lock <run lock> while waiting: Access is denied. (os error 5)` twice in 35 re-verifications. A 120-run stress run failed twice the same way, once in `rtm abandon`'s own lock wait and once in `rtm hold`. The repair went into `src/lock.rs` only. The source reviewer ruled abandon's lock-free read outside that ticket, as a known adjacent risk (line 18). No failure has been observed from `fn lock_path_present` itself.
- A held release window does not make this read fail. [The lock-wait ticket's gap record](../../residual/archive/res-110.md) (`res-110`, lines 61-62) says the attribute read meets the window only when the peer's handle closes between the two calls inside one standard-library attribute read: its open and its directory-listing fallback. No seam can pause that gap. So the existing lock hold seams (`RATMAC_TEST_LOCK_HOLD` set to `release` or `unlink`, line 1228 of the lock module) cannot force it, and the checks need an injected denial.
- Out of scope: the apply step's lock acquisitions, which already wait through windows, and every non-Windows platform, where nothing changes.

## Assumptions

- "Settled" means the first read after the window that succeeds or answers not-found. The answer at that moment decides, even if another command claims the lock right after, exactly as without the release.
- The bound is measured from the first denial, as the waiting acquirers measure theirs. The wait holds no lock, so no other command waits on abandon.
- A refusal at the bound carries the last error.

## Dependencies

- The lock split (`ENS-005`): abandon still reads without holding the root or Run lock, and takes root before Run when it applies.
- Abandon's address-first validation (`WRS-007`): the address and phrase checks keep their order and texts.
- Other selected issues: the resumed child proof (`i-073-endpoint-release-window`) uses the same meaning of release window, and the skill move retry (`i-071-skill-move-retry`) the same wait bound. Neither is a prerequisite.
- The test-only fault is armed only for the case's own commands, as the case-scoped hold barrier issue (`i-068-case-scoped-hold-barrier`, requirement `SVE-006`) requires. That is an edge this issue must not break, not a prerequisite.

## Open decisions

Whether the settled read lives in the lock module, as one crate-visible helper beside `fn transient_release_window`, or in the abandon module is left to the ticket. So is how the test-only injection is spelled. The disposition is set at integration; the linked forward authority then governs implementation.
