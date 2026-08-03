# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| `FDC` | The requirement-ID prefix shared across the doctrine-convergence splits, expanding to **FSM Doctrine Convergence**. Coined in the evidence seed ([i-016 ubi-lang](../archive/i-016-fsm-doctrine-convergence/ubi-lang.md)); identifiers stay stable and are never renumbered. |
| `MCV` | This issue's verification-check prefix, expanding to **Machine Composition Verification**; every check in [test-plan.md](test-plan.md) is `MCV-NNN`. |
| Spawn ledger | The per-run record of spawned children. Its on-disk location (a name under the run's directory) is reserved by run residency ([i-017 spec.md](../archive/i-017-run-residency/spec.md), `FDC-004`); its contract - contents, when written, meaning - is carried here as `FDC-011` in [spec.md](spec.md) (home ruled 2026-08-03, extending the 2026-07-29 scope settlement), condensed from `.arca/research/re-ratmac-FSM/05-invocation-join.md` ("The spawn ledger"). |
| Child-as-reviewer | The first-increment judge-independence mechanism: a spawned child machine performs review; the sequencing is FDC-010's ask. |
| Witnessed verdict verb | The deferred judge-independence verb; it needs signer identity, which `ORS-001` deliberately keeps out of the Engine. |
| Recursion depth cap | One level: a spawned child Run may not itself spawn in this increment. Ruled 2026-08-03 (Billy, individual human ruling), carried as `FDC-012` in [spec.md](spec.md); previously the open question recorded at the 2026-07-29 split. |
