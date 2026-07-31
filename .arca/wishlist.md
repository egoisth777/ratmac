# Wishlist

Unordered ideas with zero commitment. Promotion is a human decision.

- **Use State, not Phase, for machine position** — Billy, 2026-07-30. Replace the public `Phase` term and the Runbook's `phases` vocabulary with `State`; “Phase” suggests a linear process and misstates a general state machine. Before promotion, resolve the existing collision with `State File` and its lifecycle `status` field so “state” does not ambiguously mean the graph position, the persisted artifact, and the whole runtime record. This is a product-language and schema cutover, not a local code rename.
