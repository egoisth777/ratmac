# Nothing marks a commit as a stable base for self-development

```yaml
issue-id: "i-028-stable-edition-tags"
provenance: "Billy's ruling of 2026-08-10, immediately after the i-015 sprint made the engine run its own cycle: with self-development live, the shop needs a marker that says a given commit is a stable point to develop the engine from, and the repository carries no such marker (two tags exist, both `trial-archive/*`)."
status: "pending"
```

## Summary

The engine now runs this repository's own P1-P5 cycle, so the engine is both the
tool and the thing being built. That makes one question urgent that was
harmless before: **which commit is a safe one to work from?**

Today nothing answers it. `git tag` lists two tags, both archived trials. Every
gap record cites a bare commit hash such as `git:4ac18a1`, and a bare hash on an
untagged, unmerged, or rewritten line can stop resolving - which is already a
live wish. A contributor asking "is the tree I am standing on a good one" has to
re-run every gate by hand and trust the answer.

Worse, the failure mode is silent. A sprint started from a mid-air commit - one
where a previous sprint was half-landed, or a gate was red - produces work that
looks green against a base that was never green. The engine cannot notice,
because the engine has no opinion about version control.

This issue asks for one named marker, and for the cycle itself to insist on it.
It proposes no engine behavior change: the enforcement is an Exit Guard in this
repository's own Machine Class, spelled in the guard vocabulary that already
exists.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
