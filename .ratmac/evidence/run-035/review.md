# Child-class hold: independent review

Reviewed on 2026-09-22 before expensive private proof. Source author:
`/root/boundary_planning`. Public-test author: `/root/issue_audit`.
Private-test author: `/root/wish_audit`.

The private-test author independently accepted the source and public tests.
The public-test author independently accepted the source and private tests,
then re-reviewed the final private bytes after the added unowned-parent
control. Neither reviewer claims independent review of its own tests.
The coordinator read the source diff and public controls and checked the
reviewed digests before starting private proof. These are recorded review
events under this Run's existing contract, not cryptographic enrollment.

| Reviewed file | SHA-256 |
| :--- | :--- |
| `src/blocked.rs` | `4c4c58f9c8656541dbf4e51148d853be64fe174fcfab6960f09f2007c1650520` |
| `src/scheduler.rs` | `52b71d74dc889affd14048014d3c1e5c63297125672cee471ea22a1a7fb4357b` |
| `test/qa/tests/t114_child_class_hold.rs` | `8d9afbff9c50f7aa4fbf13f2a5b34fed0cf3a8d9ee7ee6922ab967dfcea734f7` |
| `test-hidden/t-114/tests/hidden.rs` | `ccacc52869a1274f5f116410d34267f1844562fed2dd2e2326651b0538858cf2` |
| `test-hidden/t-114/src/lib.rs` | `19e901a0b8d6f6046fbbe47db844ba8896ce2bf4d77ef632bde6096da589da43` |

Both lookup sites resolve the Run's recorded owning class; an unowned Run
cannot borrow an inline child's route. Application still reopens and
revalidates while holding its mutation lock. Confirmation, blocker containment,
terminal and stale-plan checks, pin verification, and write ordering remain.
Wrong planner and wrong apply implementations are independently observable.
Public and private fixtures include same-name routes, ownership tampering,
workspace containment, unrelated-file preservation, and subsequent child motion.

Acceptance above approves the reviewed implementation and oracles. It does
not claim that private or full regression checks have already passed.

## Inherited fixture synchronization review

After the full regression run exposed the old interrupted-sweep test, the
coordinator recorded its fixture-drift classification in the ticket. The
private-proof reviewer compared its primary and worktree bytes and accepted
exactly one added `.arg("-u")` on the interrupted Python process. Every existing
assertion and the sweep implementation remain unchanged. This review precedes
the corrected fixture's fresh execution and renewed full proof.

Reviewed `test-hidden/t-108/tests/hidden.rs` SHA-256:
`6da027a924eae1514b48a5d1adf249bc7df204a4cb92f6733ac51c56dcc68606`.
