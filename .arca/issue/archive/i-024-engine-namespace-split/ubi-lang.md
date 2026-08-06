# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Engine namespace split | This issue's requirement-ID prefix, `ENS`. |
| Engine root | The single `.ratmac/` folder at the primary checkout root, holding the Machine Class, runs, mint record, locks, Engine transition log, and receipts. A linked worktree uses Git worktree metadata to resolve the primary checkout's root; without Git it resolves the current checkout's root. |
| Engine-root runtime | The shared Git-ignored runtime entries inside the Engine root: `runs/`, `mint.toml`, `locks/`, and `log.md`. Every linked worktree resolves the same entries; they are not a separate store. |
| Mint record | The durable file naming the highest Run id ever issued, so removing a Run directory cannot re-issue its id. |
| Engine-root lock | The short lock held while minting a Run or mutating the roster or a ledger. Never held while a guard runs. |
| Run lock | The lock held while one Run moves. Two different Runs move at the same time; one Run never moves twice at once. |
| Workspace binding | The canonical filesystem path recorded on a child Run, naming the working folder its guards and evidence reads resolve against. |
| Roots table | The top-level runbook table mapping workflow role names to repository-relative paths, so guards address folders by name rather than by a path compiled into the Engine. |
| Tracked-versus-ignored split | Git tracks the Machine Class `ratmac.toml` and receipts at `.ratmac/evidence/<run-id>/`, while it ignores runtime files under `.ratmac/`. The split keeps Run state out of ticket branches and merges; run-scoped receipt paths prevent parallel child Runs from colliding. |
| Pre-split residue | A live Engine artifact still sitting at its old `.arca/` address. Its presence makes every entry point refuse and instruct; the Engine moves nothing. |
