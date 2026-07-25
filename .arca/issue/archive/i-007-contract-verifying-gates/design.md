# Issue design

## Proposed mechanics

Suggested design only — the binding requirements live in [spec.md](spec.md); none of the mechanism choices below carry weight until folded into the goal.

- Gate implementation: implement each phase predicate inside the pinned gate boundary defined by `i-006-engine-trust-boundary` (preferably in-process in the pinned Engine), parsing issue, residual, and ticket records with the same schema code paths the shape check uses, so contract and shape cannot drift apart.
- Receipts: an agent-writable `.arca/evidence/` directory holding one small structured file per executed check — command, working directory, target refs (planned-test ID, residual ID), exit status, and a SHA-256 over the captured output — plus an index per ticket. The P4 gate resolves each planned-test ID to a receipt with a failing baseline (or mutation kill); the P5 gate either re-executes the ticket's declared commands directly or verifies fresh receipts whose digests match re-hashed outputs. Receipts are evidence inputs, not Scheduler state; the Engine may copy a one-line summary into the log itself.
- Ownership: rewrite the P4-equivalent prompt to direct notes into the ticket file or the evidence directory, or provide an explicit `rtm` append command that writes the Scheduler-owned log on the agent's behalf; add an executable prompt-audit test that scans the active Runbook prompts for Scheduler-owned paths.
- Blocked route: reuse the existing human `hold t-<id>` convention as the authorization; require a `blocked-by` reference on the held ticket pointing at a new five-file issue (preferred — it feeds the next planning pass) or a named residual; add a Route Guard predicate `p5-blocked` that verifies held-plus-linked state and routes to intake, leaving ticket status `held` and residuals untouched.
- Abandonment: an explicit `rtm abandon`-style command gated on human confirmation (for example a typed confirmation phrase only a human session supplies) that appends the terminal abandoned event to the Scheduler-owned log itself, marks the State File terminal, and retires the lock — all inside the Engine, so no agent ever deletes or edits `.arca/state.toml`, `.arca/log.md`, or `.arca/rtm.lock` directly. Perform the authorization check before the first write so an unauthorized request refuses with zero bytes changed, and route stale-lock recovery through this same authorized path instead of any bypass flag.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
