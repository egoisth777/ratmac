# Issue design

## Proposed correction

`R-024` predates plural Run residency. Narrow it to the three project-level files that remain flat: `.arca/ratmac.toml`, `.arca/log.md`, and `.arca/rtm.lock`.

`R-025` keeps the State File's ownership and field contract but changes its canonical address to `.arca/runs/<id>/state.toml`.

`FDC-004` explicitly supersedes the old path clauses in both rows. No file migration or Engine change follows from this issue: the landed Engine already uses the per-Run path.
