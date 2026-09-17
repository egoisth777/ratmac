# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Records cite resolving commits (`RCR`) | This issue's stable requirement-ID prefix: a gap record's commit citation names a commit the repository still resolves, and a machine check holds that truth at closure. |
| Resolving citation | A `git:` hash in a record's `implementation-revision` that names a commit reachable from the repository's refs. A hash that exists only as a dangling object is not resolving: `git cat-file` answering is not resolution. |
| Stamp landing | The landing after the merged green landing where a satisfied record's `implementation-revision` and its ticket's `landed-commit` are written, the hash derived from the repository at that moment. The gap-record counterpart of ELR-001's recording landing. |
