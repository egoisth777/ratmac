# Full-sweep measurements for the lane-builds-stay-private ticket (`t-139`)

Shared private test support (WCP-002) asks the ticket to measure how much the
full lane sweep grows. These are observations, not a timing gate. Every sweep
is `python tools/sweep_lanes.py sweep` with its report written outside the
repository, on this Windows machine, one sweep at a time. Nothing else built
or ran tests during a sweep; a reviewer was reading files during the last
three.

## Time

| Sweep | Where | Crates | Verdicts | Wall time |
| :--- | :--- | ---: | :--- | ---: |
| before 1 | primary checkout, right after the sweep-crates-build-apart landing (`t-138`), 2026-10-29 | 81 | 79 pass, 2 expired | 1,337 s |
| before 2 | primary checkout, same code, 2026-10-30 02:34Z | 81 | 79 pass, 2 expired | 1,578 s |
| before 3 | primary checkout, same code, 2026-10-30 18:45Z, per-crate times | 81 | 79 pass, 2 expired | 2,004 s |
| after 1 | ticket worktree with the reviewed change, 2026-10-30 17:16Z | 82 | 80 pass, 2 expired | 2,676 s |
| after 2 | ticket worktree, same bytes, 2026-10-30 18:02Z, per-crate times | 82 | 80 pass, 2 expired | 2,584 s |

The same sweep of the same code varied from 1,337 s to 2,004 s on this
machine, so only the per-crate pair below says where the time goes. "after 2"
and "before 3" ran back to back, each printing a time per crate:

- The 81 crates both sweeps ran took 1,729 s after and 2,004 s before. The 25
  lanes that moved into their own folders took 1,018 s after and 1,135 s
  before; the other 56 took 711 s and 869 s. Moving a lane's build into its
  own folder did not slow a sweep whose folders were already built.
- The new lane crate for this ticket (`t-139`) took 855 s. Its six checks
  build nested crates and run lanes twice or alongside each other. It is the
  whole growth of a warm sweep.

## Build reuse

- Warm: both "after" sweeps reused every lane folder's build. No `rtm-qa` in
  a lane folder was rebuilt during them; only Cargo's dependency listings
  (`.d` files) were rewritten. The one fresh build is the test-Engine lane's
  (`t-137`) own fixture, which builds into a new folder on purpose.
- Cold: a checkout's first sweep builds `rtm-qa` once in each of the 25 lane
  folders, where before it built once for all of them. The cold time is
  measured by the primary checkout's sweep at this ticket's close.

## Disk

- Before: every lane used the repository's shared build output, one `rtm-qa`
  build in `target/debug`.
- After: 25 lane folders, 14,560 MB in total. Most hold one `rtm-qa` debug
  build of about 503 MB. The lane whose audit carries history by name
  (`t-084`) and the tracked-repository-audits lane (`t-118`) take 688 MB each,
  the single-Engine-binary lane
  (`t-086`) 754 MB because it also builds the shipped `rtm`, and the
  test-Engine lane (`t-137`) 2,028 MB because its fixtures build Engines in
  folders of their own.

## Cargo's lock files

No check, public or private, holds or relies on Cargo's own lock files: no
source under `test/` or `test-hidden/` names `.cargo-lock` or Cargo's package
cache lock. A lane's folder is held by the lane's own `lane.lock` inside it.
