# Issue test plan

## Verification

All checks below are planned, not observed results.

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WRSV-007-01` | [`WRS-007`](spec.md#requirement-records) | With one and several admitted Runs, bare abandon requires an address and teaches the Run-id phrase rather than a project-name retirement command; bytes remain unchanged. |
| `WRSV-007-02` | [`WRS-007`](spec.md#requirement-records) | With an addressed Run, omit or truncate confirmation and reorder `--run`/`--confirm`; every phrase-specific hint matches the eventual accepted phrase. |
| `WRSV-007-03` | [`WRS-007`](spec.md#requirement-records) | Execute the displayed exact phrase in a disposable confirmed fixture and verify retirement; a project-name phrase for that Run still refuses unchanged. |
| `WRSV-007-04` | [`WRS-007`](spec.md#requirement-records) | With only a leftover lock, retain the project-name cleanup hint; check help, module examples, unknown-option diagnostics, and residue refusals for consistent guidance. |

## Existing test candidates

`test/qa/tests/t065_motion_authorization.rs`, `test/qa/tests/t051_abandon.rs`, `test/qa/tests/t077_presplit_residue.rs`, and existing abandonment command tests.

Select exact test ownership and independent private coverage when cutting tickets. Public contract assertions must detect the observed wrong behavior before implementation; private checks cover the changed boundary and its failure paths.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/spec.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/design.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change proposed. |
| `.arca/schema.md` | unaffected | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/runbook-spec.md` | unaffected | Any accepted format change must be specified before implementation. |
