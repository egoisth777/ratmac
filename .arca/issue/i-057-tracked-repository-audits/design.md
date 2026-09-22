# Issue design

## Proposed mechanics

[rebrand.rs](../../../test/qa/src/rebrand.rs) owns collect_files and audit; its SKIPPED names exclude only .git, test-hidden, and target, while I/O failures are silently skipped. Both [active reference audit](../../../test/qa/tests/t034_rat005.rs) and [full rebrand acceptance](../../../test/qa/tests/t037_rat008.rs) call this shared audit.

Assumed: use `git ls-files --cached -z` under the supplied audit root, with Git environment overrides cleared, then sort and deduplicate paths. Read working files, not committed blobs, so changes being reviewed are visible. Reject conflicts and missing indexed files unless the index itself stages their removal. Gitlinks are repository boundaries, not permission to descend into another worktree; a symlink is inspected as a tracked link and never followed outside the root.

Provide an explicit extra-path slice in the audit helper; existing callers use an empty slice. Tests that intentionally build untracked fixture trees supply their exact file list or initialize a small temporary index. Extra inputs require safe repository-relative regular-file paths, no glob or directory expansion, and no link escape; duplicates collapse deterministically and missing entries refuse. Surface this list in the report so it cannot silently weaken the claimed coverage.

Update all repository-tree audit callers found in the implementation inventory to use the same listing, including vocabulary and path-name checks; focused fixture walks remain valid when their scope is explicitly a fixture. Preserve the old historical-token allowlist as a separate content decision: an extra input is not an exemption from forbidden-name checks.

Assumed binary policy: read each selected regular file as bytes. Valid UTF-8 without NUL bytes is checked as text with line locations; all other readable content is checked for the same forbidden ASCII token byte sequences, reporting byte offsets rather than lossy text. Path-name checks still apply to every selected path. Thus binary classification is explicit and causes no content exclusion; unreadable bytes remain a named failure. If a future audit needs a true binary exclusion, that exclusion must be explicitly declared and reported rather than inferred from a decoding error.

This is contributor-test behavior, not a new Engine guard. Integrate it into the working authority with an executable deliverable and link its public test proof; do not make the generic Engine know this repository's legacy spellings.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
