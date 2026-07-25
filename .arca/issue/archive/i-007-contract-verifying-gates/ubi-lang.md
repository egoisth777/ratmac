# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Contract gate | A mechanized phase gate that verifies the phase's artifact contract and evidence, so that status or prose edits alone can never satisfy it. |
| Evidence receipt | A structured, tamper-evident record of one executed check: the command or predicate, the exercised target, the observed result, and a content digest binding them. |
| Sensitivity receipt | An evidence receipt proving a planned test can fail: a recorded baseline failure before implementation or a controlled mutation that flips it. |
| Agent-writable evidence artifact | A file agents may author to carry notes and receipts, distinct from Scheduler-owned files; the append-only log remains Engine-owned. |
| Blocked route | A human-authorized Runbook route that moves a Run forward while an executing ticket is held with a linked blocker, preserving its honest partial evidence. |
| Blocker record | The concrete artifact a held ticket links to — a new five-file issue or a named residual — that states why the ticket cannot pass. |
| Run abandonment | The human-authorized terminal retirement of a broken active Run: RTM records a terminal abandoned event or equivalent evidence and safely retires the Run’s admission state so a fresh Run can start, without any agent deleting or editing Scheduler-owned files and without bypassing a stale lock. |
