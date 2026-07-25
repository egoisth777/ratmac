# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `VR-001-issue-shape-and-boundary` | `EXT-006` | The issue directory contains exactly `index.md`, `ubi-lang.md`, `spec.md`, `design.md`, and `test-plan.md`; front matter has the matching issue ID, non-empty provenance, and `pending` status; all four route links resolve and no template marker remains. The creation diff contains no implementation or external mutation. |
| `VR-002-preflight-and-rollback` | `EXT-004` | Recorded preflight proves target-slug collision/availability, `gh` authentication and permission, remote/path/process safety, branch/worktree state, and clean status. The cutover order, checkpoints, captured old values, and reversible recovery path are reviewed; no force-push, silent deletion, or arbitration bypass is used. |
| `VR-003-github-identity` | `EXT-001`, `EXT-005` | `gh api repos/egoisth777/ratmac` and `gh repo view egoisth777/ratmac` both succeed and report `egoisth777/ratmac`; the old slug is not the canonical repository identity. |
| `VR-004-local-git-and-checkout-identity` | `EXT-002`, `EXT-005` | From the reopened checkout, `git remote get-url origin` is exactly `git@github.com:egoisth777/ratmac.git`; `.git/config` agrees; `git rev-parse --show-toplevel` resolves to `E:/repos/projs/skill-dev/ratmac`, whose actual basename is `ratmac`. |
| `VR-005-active-reference-and-history-audit` | `EXT-003`, `EXT-005` | A tracked-repository audit finds no unallowlisted active `egoisth777/arca-scheduler`, old canonical origin, or old checkout-path reference in links, badges, or repository metadata. `.arca/log.md` and archived issue/ticket records are byte-for-byte unchanged and explicitly listed as historical exceptions. |
| `VR-006-project-gates-and-clean-state` | `EXT-005` | Current behavior checks T-001–T-022, integrated rebrand checks VR-001–VR-008, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, full `cargo test` plus QA/hidden lanes, real `rtm` smoke/help/error checks, and `git diff --check` pass. Final `git status --porcelain` is empty. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/current/index.md` | updated | `EXT-001`, `EXT-006`; front door links the integrated external identity issue and records planning-only scope. |
| `.arca/current/ubi-lang.md` | updated | `EXT-003`; canonical repository, origin, basename, historical allowlist, and safe cutover are defined once. |
| `.arca/current/spec.md` | updated | `EXT-001`–`EXT-006`; accepted external identity requirements are authoritative. |
| `.arca/current/design.md` | updated | `EXT-004`, `EXT-005`; preparation evidence boundary, ordered checkpoints, rollback, and final gates are defined. |
| `.arca/current/test-list.md` | updated | `EXT-001`–`EXT-006`; `EVR-001`–`EVR-006` map each requirement to observable acceptance. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | `EXT-006`; routing remains valid. |
| `.arca/index.md` | unaffected | `EXT-006`; issue shape and historical-record rules remain authoritative. |
| `.arca/log.md` | unaffected | `EXT-003`, `EXT-005`; append-only history is preserved byte-for-byte. |
| `.arca/issue/i-001-ratmac-rebrand/` | unaffected | `EXT-001`, `EXT-006`; predecessor remains historical/integrated context. |
| `.arca/tpl/issue/` | unaffected | `EXT-006`; this issue is filled from the five required templates. |
| `.git/config` and GitHub repository metadata | updated | `EXT-001`, `EXT-002`, `EXT-005`; external identity is directly verified, not inferred from tracked content. |
