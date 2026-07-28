# Issue design

## Proposed mechanics

Three parts. (1) Instructions doc: short, agent-facing, "how to write a runbook" — every schema fact is a
link into the runbook spec; the doc itself carries only procedure (start from scaffold, edit, run doctor,
repair by code). (2) Scaffold: `rtm` emits a minimal valid runbook (exact subcommand decided at P1 — likely
`rtm scaffold <path>`); its output is doctor-clean by test, permanently. (3) Repair loop: procedure in the
instructions doc keyed to doctor diagnostic codes — for each stable code, what it means and the usual fix;
the doctor's `--json` shape is the interface contract. Verification is end-to-end: from scaffold plus
instructions alone, a fresh agent (or scripted stand-in) reaches doctor-clean on a nontrivial machine
without reading `src/`.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
