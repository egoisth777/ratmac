# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| SDC | The requirement-ID prefix of this issue; expands to **Safe Deliberate-damage Checks**. IDs stay stable through the working authority and any future carrier. |
| Deliberate-damage check | Breaking the code on purpose, briefly, to prove a named test fails while the break is in - the mutation evidence every gap record cites. Temporary by definition; never lands. |
| Discard command | Any command that throws away uncommitted changes: `git checkout -- <path>`, `git restore`, `git clean`, `git reset --hard`, dropping a stash. Restoring saved bytes from a checkpoint is restoration, not a discard. |
| Unsaved completed work | Edits in the working tree that are wanted - not the deliberate damage itself - and are neither committed nor parked. What the `t-064` turn lost. |
| Checkpoint (safety commit) | The ephemeral commit made after a turn's tests are green and before any deliberate damage - subject exactly `t-<id>: checkpoint - not a landing`. Unpublished, unmerged, not a Landing, no log line; `git commit --amend` folds it into the green landing. |
| Park | Setting unsaved wanted work aside without landing it: `git stash push -m "t-<id>: <what>"`, dropped only after its content lands or is explicitly declared obsolete. |
| Working-authority requirement | An accepted ask that resolves to an explicit requirement-ID heading in `.arca/schema.md` instead of a product goal row. Binds contributors at integration; mints no goal row, no gap record, no ticket. |
