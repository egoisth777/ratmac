# Issue design

## Proposed mechanics

Keep the turn tool unaware of expiry markers. Add a closed, optional
`lanes-rerun-scope` declaration: `per-lane` preserves the current default;
`root-once` runs the declared command once from the primary checkout. Reject
unknown values before any mutation. The dry-run plan names where it runs.

This repository selects `root-once` and declares a fresh sweep followed by its
report check. Both must succeed: sweep alone reports but does not refuse a
stray crate, while check alone could accept an old report. Reuse the existing
sweep validator; add no marker parser or automatic expiry operation.
Both commands use `--report target/turn-close-lanes.md`; this ignored report
keeps routine turn verification from dirtying the tracked cycle-close report.

A final-verification failure happens after branch removal. A repeated close
must recognize a proven completed landing and retry only verification, not
require the deleted branch or repeat the stamp or log. Preserve the earlier
copy-back and only-copy refusal checks. Test this boundary explicitly.
The proof requires no item branch, worktree registration, or worktree folder,
the item stamp matching the current trunk tip, the supplied landing line
already present, and no unrelated tracked dirt. A missing or stale proof
still refuses. Retry reruns the entire verification; no new journal is added.

No changes to Engine source or the Machine Class are proposed. Keep existing
per-lane fixture declarations working instead of changing every caller.

This file is incoming evidence. Integrated mechanics remain authoritative
only in the accepted forward authority.
