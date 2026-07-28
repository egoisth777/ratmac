# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| `PCR` | This issue's requirement-ID prefix, expanding to **P-Cycle Runbook**. Every requirement record here is `PCR-NNN`. |
| Cycle runbook | The Machine Class at `.arca/ratmac.toml` that declares the P1-P5 working cycle itself, as opposed to a demonstration machine that merely shows the format. |
| Open ticket | A ticket still being worked. Today a human infers it; this issue requires a predicate a guard can evaluate. |
| Landing append | The act of adding one line to `.arca/log.md` for a landing. Scheduler-owned while a Run is live, so it must then be performed by `rtm`, never by an agent instructed to edit the file. |
| Stage oracle | Whatever answers "where are we". Today the tree read through a lookup table in `.arca/index.md`; the endpoint is `rtm status`. |
| Active refs | The `active_refs` field of the State File (R-025): the Scheduler-written list of what the Run is currently working on - ticket and requirement ids. Present in the format and in fixtures, never yet populated by `rtm`. |
| Per-ticket gate | An Exit Guard whose verdict is about one named ticket - `sensitivity_receipts` and `completion_gate`. Both require a `ticket` field, which is why the cycle's build loop cannot name its ticket in a read-only runbook. |
