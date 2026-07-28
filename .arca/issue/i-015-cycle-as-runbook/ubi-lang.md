# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Cycle runbook | The Machine Class at `.arca/ratmac.toml` that declares the P1-P5 working cycle itself, as opposed to a demonstration machine that merely shows the format. |
| Open ticket | A ticket still being worked. Today a human infers it; this issue requires a predicate a guard can evaluate. |
| Landing append | The act of adding one line to `.arca/log.md` for a landing. Scheduler-owned, so it must be performed by `rtm`, never by an agent instructed to edit the file. |
| Stage oracle | Whatever answers "where are we". Today the tree read through a lookup table in `.arca/index.md`; the endpoint is `rtm status`. |
