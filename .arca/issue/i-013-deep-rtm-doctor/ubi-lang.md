# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Diagnostic code | A stable, documented identifier for one defect class (e.g. unknown guard kind, unreachable state). Stable means: the same defect yields the same code across runs and releases. |
| Guard lint | Checks on guards beyond parseability: unknown kind, per-kind required/forbidden fields, unpinned non-exempt `command_exit`, agent-writable-guard warning. |
