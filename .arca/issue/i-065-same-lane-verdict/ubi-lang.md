# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Shared build output | A build output folder or file that more than one test, lane, or run can write: the repository's `target/` folder or wherever Cargo redirects it, `target/lanes`, the freeze build folder `ratmac-freeze-<commit>` under the system temp folder, a folder that a Cargo configuration file or an inherited variable makes several crates build into, and a lane crate's own build folder when two runs of that lane overlap. |
| Per-lane folder | A build output folder that belongs to one lane and that one run of that lane holds under a cross-process lock for the whole run, so a second run of the same lane waits instead of sharing it. |
| Run-private folder | A folder only one run uses, which no other run reads or rewrites; the freeze build folder and the pre-split ports sweep folder become run-private. |
| Cargo-running lane | A private lane whose tests run Cargo on the repository or on another lane's crate: to build and launch the Engine (`rtm` or `rtm-qa`) or another binary from this repository (`edition-cut`, `snapshot-manifest`), or to run that crate's tests. |
| Standalone verdict | The verdict a lane gives when its crate runs by itself, with no sweep, proof, or other lane running. |

Existing project terms follow [the dictionary](../../dict.md) and [goal language](../../goal/ubi-lang.md).
