# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Reviewable snapshot | Candidate content whose every exercised path is visible to git review (tracked or staged), so the tested tree can be reconstructed and audited from the recorded change. |
| Snapshot manifest | The recorded enumeration binding an evidence claim to its snapshot: per declared root, the git tracking state and a content digest. |
| Declared evidence root | A directory the acceptance evidence claims to have exercised (product sources, QA suites, contributor artifacts). |
| Authorized archive move | Relocating a completed issue folder to `.arca/issue/archive/<issue-id>/` with identity and five-file shape preserved and content unchanged except required relative-link updates. |
| Release acceptance lane | The environment-coupled checks (live GitHub identity, exact origin, branch, clean worktree) that prove an operator cutover, runnable only by explicit opt-in. |
| Default suite | What plain `cargo test --workspace` runs with no opt-in environment configured. |
