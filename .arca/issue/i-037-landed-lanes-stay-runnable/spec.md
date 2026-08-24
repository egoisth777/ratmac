# Issue specification

Dispositions are the DESIGNER's proposal at filing, 2026-08-24; P1 confirms
or revises each at the next planning pass. None of the three asks presumes an
Engine feature: the sweep and its marker are shop-lane tooling and records,
and the close wiring is a runbook guard the existing vocabulary already
expresses.

`LNR` is this issue's stable requirement-ID prefix - **Landed-lane
runnability** - defined in [ubi-lang.md](ubi-lang.md).

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `LNR-001` | One sweep command runs every crate under `test-hidden/` from the primary checkout, outside any ticket turn, and reports exactly one verdict per crate - `pass`, `expired`, or `red` with the failing lane ids - plus a total, naming any crate the folder lacks rather than skipping it. When the sweep runs is a human decision recorded where the working rules keep it; this ask mints no schedule of its own. | pending | The lanes' only two runs are P5 inside the ticket worktree and the single post-merge confirmation from `main` (`.arca/schema.md:265`, `.arca/schema.md:133`); after that nothing names the folder - it is gitignored (`.gitignore:24`), outside the workspace (root `Cargo.toml` declares `members = ["test/qa"]` only), and referenced by no tool under `tools/`. Forty-eight crates, `t-058`..`t-105`, whose last run anywhere is their own landing. | - |
| `LNR-002` | A lane that no longer runs green against today's Engine is marked expired by an explicit act, and the marker names the edition the lane last passed at. The sweep counts an expired lane as neither `pass` nor `red`, so a `red` verdict always means a live regression and rot is a dated, visible fact. Renewing or removing a marker is the same explicit act the 2026-08-10 repair was. | pending | The wish's hand-taken sweep sorted the folder into three classes (`.arca/wishlist.md:26`): pre-split refusals (`t-058`..`t-070`), rename breakage repaired the same day (`t-071`..`t-083`), and by-design refusals against frozen source hashes (`t-078`, `t-079`). Without a marker, the second and third classes are indistinguishable from regressions, and the repair's knowledge survives only in the log. | - |
| `LNR-003` | The `close` State's Exit Guard may read the sweep's verdict, expressed in the existing guard vocabulary - the edition guard already proves the commit being left is tagged through one `command_exit` (`schema.md:518-528`); no new guard kind is minted and the Engine learns nothing about lanes. When wired, the close passes only on a verdict in which every landed lane reads `pass` or carries an expiry marker. | pending | Close is the one moment the whole folder is a sprint's responsibility at once; it is where `EDN-002` already proves the tree being left, and the same slot can prove the lanes being left behind are runnable or visibly expired. | - |

## Acceptance criteria

- One command produces the per-crate report over `t-058`..`t-105` on this
  repository, and the report names every crate the folder lacks rather than
  skipping it silently.
- The sweep's first run on this repository is itself evidence: `t-102`..
  `t-105` (landed 2026-08-21 with six lanes each, `.arca/log.md:433-436`)
  read `pass` or carry markers saying otherwise - no crate reads `red`
  without a live regression to point at.
- A crate whose lanes refuse with no marker reads `red` with lane ids;
  writing a marker naming an edition flips its verdict to `expired`;
  `t-078` and `t-079` read `expired` once marked, never `red`.
- A close whose sweep names a red, unexpired lane refuses; the same close
  passes once the lane is green or marked; the runbook diff that wires it
  adds a `command_exit`-class guard and nothing else.
- The sweep writes nothing but its report: a tree snapshot around a run is
  byte-identical apart from that artifact.
