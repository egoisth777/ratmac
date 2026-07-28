# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Second parse | The independent toml walk in `scheduler.rs` that re-reads the runbook the parser already read — two readers, one file, drift by construction. This issue kills it. |
| Named refusal | An error that says what is wrong and where (typed, e.g. MachineClassParseError), as opposed to silently proceeding on a default value. |
