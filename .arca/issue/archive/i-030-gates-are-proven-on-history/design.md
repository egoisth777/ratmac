# Issue design

## Proposed mechanics

**1. A shared "aged fixture" builder in the QA harness.** The harness already has
`TempRepo` and the contract-gate fixture trees. The addition is one builder that advances a
fixture through a second freeze: write records at freeze A, archive them, move the goal,
freeze B, write live records. Every contract-gate check that needs a past composes this
instead of hand-rolling one, so the cost of `GPH-001` per gate is a few lines.

**2. The rule lands where Merge Gate contents are defined.** The working rules already
demand hidden-lane coverage per ticket; `GPH-002` adds one sentence to the same section, so
review reads both requirements from one place. No new document.

**3. This repository as the growing fixture (`GPH-003`).** The pattern is already proven:
`EDNV-004` audits this repository's own tags and keeps reporting as editions accumulate.
Each contract gate gets one such check with a stated expected verdict. When a gate's verdict
on this repository legitimately changes, the check changes in the same ticket that changes
the gate - the check is an oracle, not a snapshot.

## Rejected: a doctor rule that scans for fresh fixtures

Mechanically detecting "this gate has no fixture with a past" would need the doctor to
understand test code, which it cannot do honestly. The binding point is review at the Merge
Gate, where the other per-ticket guarantees already bind.

## Rejected: fold into i-029

i-029 fixes one gate's rule and proves that one gate on history (`ARF-003`). This issue is a
rule about all gates' checks and outlives i-029; folding them would couple a one-line gate
fix to a harness-wide sweep and stall the blocking fix behind the broad one.

This file is incoming evidence. Integrated mechanics remain authoritative only in the
accepted forward authority.
