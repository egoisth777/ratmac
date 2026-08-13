# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `EDNV-001` | `EDN-002` | On a fixture repository whose runbook carries the edition guard, the step out of the closing stage refuses while no `edition-*` tag points at the commit being left, and the refusal names the command and the observed exit code. Tagging that same commit makes the identical step succeed, with nothing else changed. |
| `EDNV-002` | `EDN-002` | The guard reads version control, not the tree: a tag pointing at some other commit still refuses, and a tag whose name is not `edition-*` still refuses. |
| `EDNV-003` | `EDN-002` | The refusal writes nothing: State, Status, and the transition log are byte-identical after a refused step, and no tag is created or moved by the engine in any run. |
| `EDNV-004` | `EDN-001`, `EDN-003` | This repository's own history satisfies the convention: every `edition-*` tag is annotated, the numbers are sequential from `001` with no holes, each tag's commit is reachable from `main`, and each tag's message records the bar. Checked over the repository as it stands, so it keeps reporting as editions accumulate. |

## Integration traces

| Trace | Where it lands |
| :--- | :--- |
| The edition convention, its bar, and its immutability | A new requirement-ID section in the working rules, [schema.md](../../../schema.md) - the same authority that already carries the checkpoint rule the word must not collide with. |
| The guard itself | This repository's Machine Class, `.ratmac/ratmac.toml`, on the closing stage beside the record contract. |
| The term | The shop glossary, [dict.md](../../../dict.md) - corrected at P1 from the goal bundle's ubiquitous language, because `EDN` binds contributors, not the program, and `dict.md` is where working terms such as Park and Checkpoint already live. |
| The strengthened wish | The live wish about gap records citing commits that no longer resolve: an edition keeps its commit reachable, which is half that wish's desired end. |
