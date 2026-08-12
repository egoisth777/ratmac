# The Plan-Build Runbook

```yaml
issue-id: "i-015-cycle-as-runbook"
provenance: "Discovered while closing the i-011..i-014 sprint, 2026-07-28 - steering.md Current sprint endpoint is unmet and the frozen goal carries no requirement for it; schema.md, The only road back"
status: "integrated"
```

## Summary

The sprint route landed the Machine Class as first-class data - written specification, one typed reader,
deep doctor, authoring loop - but the endpoint the sprint was aimed at is not reached, and the frozen goal
never carried a requirement for it. `.arca/ratmac.toml` is still a demonstration machine (`build` ->
`build-review` -> `build-done`). The **Plan-Build Runbook** - the Machine Class for the P1-P5 cycle
this repository actually follows - still lives only as prose in `.arca/schema.md`, and "where are we" is
answered by a human reading a lookup table in `.arca/index.md` rather than by `rtm status`. Until RatMac
runs this runbook, the engine has not run the first real process it was built to support.

Closing that gap surfaced four sub-gaps that are not cosmetic - they are the reason it has not happened
yet. They are recorded here as requirements, not as intentions.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Disposition log

- 2026-07-29: deferred at the 2026-07-29 planning pass (status stays `pending`) — it needs the
  dependency strata beneath it: integrated Run residency (`i-017-run-residency`), input-routed
  transitions (`i-016-fsm-doctrine-convergence`), input delivery and durability
  (`i-019-input-delivery-durability`), Run completion (`i-020-run-completion`), and machine
  composition (`i-018-machine-composition`). The additions record Billy's 2026-07-30 atomic cut;
  requirement identifiers stayed unchanged. The ordering among the new concerns is an assumed
  dependency forecast, revocable at planning step 1.
- 2026-07-30: all six asks were dispositioned `deferred`; the issue closes this batch as `integrated` with zero accepted goal rows, satisfying the planning-step terminal status without pretending the asks landed. Steering retains the direction in Horizon; a later planning issue may carry the deferred asks when their dependencies land.

- 2026-07-31 correction: the preceding `integrated` conclusion is superseded. All six asks remain `deferred`, so the same five-file bundle stays live in the Deferred issue buffer with status `deferred`; no replacement issue carries them.

- 2026-08-10: selected by Billy and integrated. The bundle moved from the deferred buffer to the intake
  work area with status `pending`, and every ask was dispositioned: `PCR-001`, `PCR-002`, `PCR-003`,
  `PCR-005`, and `PCR-007` accepted; `PCR-004` rejected as superseded by the human-only history rule and
  the ruling that the Engine has no work-item concept; and the 2026-08-03 extension of `PCR-001` split
  into `PCR-008` and `PCR-009`, both accepted, because it carried two independently provable
  capabilities. Nothing is deferred, so the bundle archives at this pass. Two mechanics were settled
  against the author's proposals, both because something already in force decides them: one sprint is
  one Run, since a rest State routing back to intake would leave the machine with no initial State; and
  the per-item gates address their work item through a binding supplied at spawn rather than through the
  Run Record's active references, since composition landed after this issue was written and a value
  recorded once in an append-only ledger cannot go stale. Recorded in
  [ADR-0015](../../../goal/design.md#the-shops-own-cycle-as-a-runbook-adr-0015).
  This pass also renumbered [the test plan](test-plan.md) one for one against the accepted checks,
  dropping the rows for the rejected `PCR-004` and the earlier-dropped `PCR-006` and adding rows for
  the three newly accepted asks. Two entries in [the terms](ubi-lang.md) describe the problem as it
  stood when the issue was written and are kept as authored context: the active-references mechanism
  named there is not the one integrated, and the per-item gate's literal address form survives beside
  the bound form rather than being replaced.
