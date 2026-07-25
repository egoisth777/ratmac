# Issue design

## Proposed mechanics

Suggested design only — the binding requirements live in [spec.md](spec.md); none of the mechanism choices below carry weight until folded into the goal.

- Snapshot audit: a QA-side helper (later callable from the pinned gate boundary of `i-006-engine-trust-boundary`) that runs `git status --porcelain` scoped to declared evidence roots, refuses on undeclared untracked or unstaged entries, and emits a manifest of sorted path, tracking state, and SHA-256 rows; store the manifest beside the evidence that cites it. Declared roots default to product sources, `test/`, and `.arca/` contributor artifacts.
- Archive-aware oracle: in `test/qa/tests/t038_ext_identity.rs`, replace the same-path `git diff --quiet HEAD -- <path>` loop with move resolution — for each HEAD path under the history roots, accept either an unchanged path or a complete authorized relocation to `.arca/issue/archive/<issue-id>/` whose file bytes match HEAD except relative-link rewrites, and require the whole five-file set to move together. Keep the mutation and partial-move failures loud.
- Opt-in lane: mark the external-identity acceptance test `#[ignore]` with an explicit runtime opt-in (for example an environment variable such as `RATMAC_RELEASE_ACCEPTANCE=1` checked at test start), and add one always-running reporter test that prints whether the lane ran or was skipped so the skip is visible in default output. Document the opt-in in the QA test list.
- Schema: add the archive authorization and the reviewable-snapshot rule to `.arca/index.md` as durable working rules, so the policy the oracles implement is committed rather than implied.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
