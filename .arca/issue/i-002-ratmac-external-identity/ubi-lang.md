# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| External repository identity | The GitHub slug, canonical `origin` URL, checkout directory, `.git` metadata, and active repository-facing links that identify this project outside its Rust code. |
| Canonical repository | The GitHub repository `egoisth777/ratmac`; the superseded `egoisth777/arca-scheduler` slug is not canonical after cutover. |
| Canonical origin | The exact SSH URL `git@github.com:egoisth777/ratmac.git` recorded for the local `origin` remote. |
| Checkout basename | The final local directory name `ratmac`, verified from the checkout's actual path and Git top-level rather than from a display-only label. |
| Historical record | Append-only `.arca/log.md` content and archived issue/ticket content retained as provenance and excluded from active-identity replacement. |
| Safe cutover | A preflighted, ordered identity migration with checkpoints, verification, and a reversible recovery path that never silently discards work or bypasses Git arbitration. |
