# Red landed lanes are recovered, so the wired close guard can pass

```yaml
issue-id: "i-038-red-lanes-recovered"
provenance: "Wishlist, `The twenty-two red landed lanes are repaired or explicitly expired so the wired close guard can pass` - the op, filed 2026-08-25; promoted 2026-09-17 on Billy's next-issue authorization, re-evidenced by a per-crate triage of every red verdict in the first sweep report"
status: "pending"
```

## Summary

The first lane sweep (`.ratmac/evidence/lane-sweep/report.md`, swept
2026-08-25) read 28 crates `pass`, 2 `expired`, and 20 `red`, and the close
guard wired at that cycle boundary (`.ratmac/ratmac.toml`, the second
`command_exit` guard on the `close` State) now refuses every sprint close
until each red crate reads `pass` or carries a dated expiry marker. The
guard working is the point of `i-037`; the red set it names is debt that
issue deliberately left visible instead of silently expiring.

A per-crate triage on 2026-09-17 found no live regression among the twenty.
Thirteen crates (`t-058`..`t-070`) predate the engine-namespace split and
are refused at their first `rtm start` by the pre-split residue guard
(`src/scheduler.rs`, `refuse_flat_residue_at`); five of them also fail to
build against renames the shop authorized later - the Run Record field
`phase` -> `state` (t-081), `HoldRequest` losing its ticket field (t-087),
and the public `doctor::render_json` retired for the `engine_root`-bearing
`Diagnosis::render_json` (t-078). Four post-split crates drifted behind
later landings: `t-083` and `t-085` pin the pre-edition-004 command
surface (no teach line, no `skill` verb), and `t-092`/`t-093` cite the goal
freeze before the Engine mints it, so their gap-record fixtures carry an
empty citation the record contract rightly refuses; `t-085` also trips a
newline defect in the qa baseline helper's path comparison. Three crates
(`t-095`, `t-096`, `t-100`) are green today - their sweep-time red was the
edition-004 tag lagging its ledger row mid-repair - and only the recorded
verdict is stale. Separately, `t-108`'s own crate sits in the folder
unrostered (`.ratmac/lanes.toml` ends at `t-107`), and the `check` verb
treats a stray crate as information, so the close guard would pass without
that crate ever being swept.

This issue asks for four things: a triage rule that fixes the act each
class of red licenses, so expiry can never silence a red verdict
(`RLR-001`); the thirteen pre-split crates ported and green (`RLR-002`);
the seven post-split crates green - four ported, three re-swept
(`RLR-003`); and the roster kept complete by the landing that adds a crate,
a stray crate refused by the check, and the close guard exiting `0` on this
repository with every marker still honest under `--verify-expired`
(`RLR-004`).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## History

- 2026-09-17: filed from the wishlist by the op on Billy's next-issue
  authorization; dispositions are the author's proposal, P1 confirms or
  revises at integration.
