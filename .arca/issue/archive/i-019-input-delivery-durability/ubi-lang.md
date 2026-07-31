# Ubiquitous language

`FDC` expands to **FSM Doctrine Convergence**. The prefix was coined in the doctrine-convergence seed and remains unchanged across its split issues so requirement identifiers stay stable.

`FDCV` expands to **FSM Doctrine Convergence Verification**. The check prefix was coined in the evidence seed and remains stable across the split test plans.

## Terms

| Term | Meaning |
| :--- | :--- |
| Legal input list | The closed set of transition input values the Machine Class permits at one state. It is class data, not a Run's judgment. |
| Transition input value | The one value extracted from a live verdict record and used to select an outgoing edge. |
| Live verdict record | The external judge's current on-disk judgment for one addressed Run and state: one transition input value plus rationale, awaiting Engine consumption. |
| Archived verdict | A consumed live verdict record held immutably in Run evidence. It records what selected the transition and can never become live input again. |
| Verdict slot | Reused from the accepted goal: the per-Run location where a typed verdict lands. This issue does not silently rename the term or the current `verdict.toml` reservation. |
