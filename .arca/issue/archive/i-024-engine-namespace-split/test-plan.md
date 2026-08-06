# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `ENSV-001` | `ENS-001` | A fixture project with `.ratmac/ratmac.toml` starts, steps, and reports; no Engine write lands under `.arca/`. |
| `ENSV-002` | `ENS-002`, `ENS-010` | A primary-checkout and linked-worktree fixture proves Git worktree resolution reaches the primary checkout's `.ratmac/`, and `rtm status` and `rtm doctor` print that resolved Engine-root path. |
| `ENSV-003` | `ENS-002` | With Git absent, `.ratmac/` resolves at the current checkout root and is reported, rather than failing or guessing. |
| `ENSV-004` | `ENS-003` | Spawn in the primary checkout, step the child from a linked worktree, join in the primary checkout: one roster, one addressable Run, no duplicated id. |
| `ENSV-005` | `ENS-004` | Mint a Run, delete its directory, mint again: the new Run receives the next id, never the deleted one. |
| `ENSV-006` | `ENS-005` | Two Runs move concurrently without blocking each other, while two motions on one Run serialize; a long-running guard holds no Engine-root lock. |
| `ENSV-007` | `ENS-006` | `rtm spawn --workspace` records the canonical path in the ledger entry; the child's guards read that workspace; an absent flag inherits the parent's workspace; a wrong path is refused at guard evaluation with a named reason. |
| `ENSV-008` | `ENS-007` | Engine transitions append only to the log under the resolved `.ratmac/` root; `.arca/log.md` is byte-unchanged across a full Run. |
| `ENSV-009` | `ENS-008` | Guards resolve folders through `[roots]`; an undeclared root name, a missing root path, and a root overlapping the Engine root each fail static validation with their own codes; a source scan finds no `.arca` literal in `src/`. |
| `ENSV-010` | `ENS-009` | Each pre-split residue file present alone makes every entry point refuse, print instructions, and leave the tree byte-identical. |
| `ENSV-011` | `ENS-011` | The working rules name no pre-split Engine path outside archived history, and the documentation shape check passes. |
| `ENSV-012` | `ENS-012` | A Git fixture verifies that runtime files under `.ratmac/` are ignored, while `ratmac.toml` and receipts at `.ratmac/evidence/<run-id>/` are tracked; a ticket-branch commit contains no Run state. |
| `ENSV-013` | `ENS-006`, `ENS-012` | Two parallel sibling child Runs write receipts to distinct `.ratmac/evidence/<run-id>/` paths; their receipts merge without collision. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | Single Engine root, primary-checkout resolution, and tracking split enter the goal's orientation |
| `.arca/goal/ubi-lang.md` | updated | Engine root, Engine-root runtime, mint record, workspace binding, roots table, tracking split |
| `.arca/goal/spec.md` | updated | `ENS-001`..`ENS-012`; restates `FDC-004`, extends `FDC-005`, `FDC-006`, `FDC-011`; supersedes `R-016`, `R-024`..`R-026` path clauses |
| `.arca/goal/design.md` | updated | Supersedes `ADR-0008` state layout; records Engine-root resolution and lock ordering |
| `.arca/goal/test-list.md` | updated | `ENSV-001`..`ENSV-013`, including the two-worktree fixture |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | |
| `.arca/schema.md` | updated | Engine-owned file list, two-writer log clause, tracking split, and pre-split path spellings including `.arca/state.toml` |
| `.arca/index.md` | updated | Map table entries for the Engine root and the per-Run state file |
| `.arca/runbook-spec.md` | updated | Top-level `[roots]` table, its field types, and its new diagnostic codes |
| `.arca/steering.md` | updated | Horizon item 1 lands; ordering before the Plan-Build Runbook restated |
