# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| TCEV-001 | TCE-001 | A real fixture turn closes using the repository's declared verifier: a passing crate and a valid expired refusing crate succeed with separate report counts; an unexpired failure, malformed marker, missing crate, and stray crate each refuse by name. The verifier creates a fresh report, and marker bytes never change. |
| TCEV-002 | TCE-001; THK-002 | Force the final verification to fail after cleanup, repair its input, then repeat close. Only verification runs again; landing, copy-back, stamp, log, worktree removal, and branch deletion are not repeated. Invalid or absent landing evidence cannot enter this final-only retry. |
| TCEV-003 | TCE-001; THK-004 | A generic root command runs exactly once from the primary checkout; omitted scope retains per-lane invocation. Unknown scope refuses before writes. Dry-run describes the chosen scope and changes nothing. No fixture verifier depends on this repository's marker filename. |

## Goal/Test File Traces

Integrated at the 2026-09-18 planning pass. The accepted forward authority
is [TCE-001](../../../goal/spec.md#integrated-turn-close-expiry-requirements);
the mechanism is [ADR-0021](../../../goal/design.md#turn-close-verification-delegates-to-the-declared-sweep-adr-0021).

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | TCE-001; this issue's index |
| `.arca/goal/ubi-lang.md` | unaffected | Existing terms suffice. |
| `.arca/goal/spec.md` | updated | TCE-001; this issue's requirement record |
| `.arca/goal/design.md` | updated | TCE-001; this issue's proposed mechanics |
| `.arca/goal/test-list.md` | updated | TCEV-001..003; this verification table |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | Existing routing remains valid. |
| `.arca/schema.md` | updated | Final turn confirmation uses the declared verification policy; links to TCE-001. Fixed close order and removal safety remain binding. |
| `.arca/dict.md` | updated | Requirement ID entry registers TCE. |
| `.arca/steering.md` | updated | Regenerate Current sprint at planning close; Self-hosted and Every boundary machine-checked are the admission properties. |
| `.arca/wishlist.md` | updated | Mark the promoted wish as carried; remove it only when fulfilled. |
