# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at
integration. `ENS` expands to **Engine namespace split** and is this issue's stable
requirement-ID prefix, defined in [ubi-lang.md](ubi-lang.md).

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `ENS-001` | The single Engine root is `.ratmac/` at the primary checkout root, holding the Machine Class `ratmac.toml`, runs, the mint record, locks, the Engine transition log, and receipts. No file under `.arca/` is written or mechanically read by the Engine except through a declared root name. | accepted | One folder per owner replaces the schema's per-file ownership ledger with a single structural rule, and a project can adopt `rtm` without a folder named after a methodology it does not run. | [goal spec](../../../goal/spec.md#requirement-records) |
| `ENS-002` | A linked worktree uses Git worktree metadata to resolve the primary checkout's `.ratmac/`; without Git, the Engine resolves `.ratmac/` at the current checkout root. All worktrees therefore share one roster, one id namespace, and one lock domain. | accepted | Resolving every linked worktree to the primary Engine root gives all worktrees one roster, one id namespace, and one lock domain. | [goal spec](../../../goal/spec.md#requirement-records) |
| `ENS-003` | Run ids are unique across every checkout and linked worktree of one repository, and a Run created in one workspace is addressable from any other. | accepted | `FDC-004`'s single id namespace is per-checkout today, which two worktrees break by minting the same ordinal; this restates the promise at repository scope. | [goal spec](../../../goal/spec.md#requirement-records) |
| `ENS-004` | Minting reads and advances a durable mint record under the store lock, so a Run id is never re-issued even after its Run directory is removed. | accepted | `FDC-006` never-reuse depends today on retired directories staying listed; a mint record makes it structural instead of roster-dependent. | [goal spec](../../../goal/spec.md#requirement-records) |
| `ENS-005` | Locking splits in two: a short store lock covers minting and roster or ledger mutation; a per-Run lock covers motion on one Run. Acquisition order is store before Run, and guard evaluation never holds the store lock. | accepted | One global invocation lock would serialize exactly the parallel child Runs the split exists to enable, because guards can run for minutes. | [goal spec](../../../goal/spec.md#requirement-records) |
| `ENS-006` | `rtm spawn` accepts `--workspace <path>`, canonicalizes it, and records it in the child ledger entry; absent means the parent's workspace. The child Run's guards and motion evaluate against the recorded workspace. | accepted | The ledger already reserves the field and always leaves it empty, so a child cannot be bound to the worktree its assigned agent works in. | [goal spec](../../../goal/spec.md#requirement-records) |
| `ENS-007` | The Engine transition log lives in the shared store; `.arca/log.md` becomes human-only with no Engine writer. | accepted | The transition log is the last two-writer file, and it would diverge per worktree the moment two child Runs move at once. | [goal spec](../../../goal/spec.md#requirement-records) |
| `ENS-008` | A top-level runbook `[roots]` table maps role names to repository-relative paths, guards address roots by name, an undeclared or missing root name is a static error with its own diagnostic code, and no `.arca` path literal remains in `src/`. | accepted | The contract guards hard-code methodology paths in Rust (`R-016` debt), which welds one workflow's folder names into a generic engine. | [goal spec](../../../goal/spec.md#requirement-records) |
| `ENS-009` | A pre-split live Engine artifact — `.arca/ratmac.toml`, `.arca/runs/`, `.arca/rtm.lock`, or a flat `.arca/state.toml` — makes every entry point refuse with instructions and move nothing. Archived receipts under `.arca/evidence/` are inert history, not residue. | accepted | `FDC-005` already rules that a stale layout refuses and instructs rather than auto-migrating; leaving an old runbook live would shadow the new one. | [goal spec](../../../goal/spec.md#requirement-records) |
| `ENS-010` | `rtm status` and `rtm doctor` report the resolved Engine root and the resolved store path. | accepted | The store sits outside the checkout listing, so an operator or agent must be able to read where it landed rather than guess. | [goal spec](../../../goal/spec.md#requirement-records) |
| `ENS-011` | The working rules stop naming pre-split Engine paths: `.arca/schema.md` and `.arca/index.md` name the Engine root, the store, and the per-Run state file, and no longer name `.arca/state.toml`. Archived and frozen records stay byte-for-byte. | accepted | The engine already refuses a flat `.arca/state.toml`, while the binding rules still mandate it in nine places, so the written rules contradict the running program. | [schema ENS-011](../../../schema.md#ens-011--current-engine-addresses) |
| `ENS-012` | Runtime files under `.ratmac/` are ignored by Git while the Machine Class and receipts stay tracked, so Run state can never enter a ticket branch or a merge. | accepted | Keeping volatile Run state out of commits prevents merge collisions while tracked, run-scoped receipts retain durable evidence. | [schema ENS-012](../../../schema.md#ens-012--engine-root-tracking-policy) |

## Out of scope

Whether a Subagent may operate its assigned child Run, and how deep a runbook may
declare spawning, are separate rulings with their own evidence. This issue only makes
cross-worktree operation mechanically possible. Worktree creation, merge, and cleanup
stay with the contributor tooling and the housekeeping wish; the Engine records a
workspace path and manages no worktree. Automatic ignore-rule writing at initialization is
a separate wish.
