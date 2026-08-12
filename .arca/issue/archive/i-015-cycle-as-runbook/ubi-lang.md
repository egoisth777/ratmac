# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Plan-Build Runbook | The formal name of the Machine Class for the P1-P5 working cycle and the first real runbook intended to run on the RatMac engine. The workflow owns its rules; RatMac only executes the declared Machine Class. |
| `PCR` | This issue's permanent requirement-ID prefix, coined from the earlier working name **P-Cycle Runbook**. Every requirement record here remains `PCR-NNN`; naming the runbook does not rename stable requirement IDs. |
| Open ticket | A ticket still being worked. Today a human infers it; this issue requires a predicate a guard can evaluate. |
| Landing append | The act of adding one line to `.arca/log.md` for a landing. Scheduler-owned while a Run is live, so it must then be performed by `rtm`, never by an agent instructed to edit the file. |
| Stage oracle | Whatever answers "where are we". Today the tree read through a lookup table in `.arca/index.md`; the endpoint is `rtm status`. |
| Active refs | The `active_refs` field of the Run Record (R-025): the Scheduler-written list of what the Run is currently working on - ticket and requirement ids. Present in the format and in fixtures, never yet populated by `rtm`. |
| Per-ticket gate | An Exit Guard whose verdict is about one named ticket - `sensitivity_receipts` and `completion_gate`. Both require a `ticket` field, which is why the cycle's build loop cannot name its ticket in a read-only runbook. |
