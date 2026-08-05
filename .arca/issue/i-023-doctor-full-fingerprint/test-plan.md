# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `DFPV-001` | `DFP-001` | On `main`, add and run a fresh red-first regression before the production change. It independently hashes the exact test-built `rtm` executable, observes the current abbreviated report fail, then passes only when argument-free `rtm doctor` reports that exact 64-character lowercase hexadecimal digest and the before-and-after filesystem snapshot is identical. |
| `DFPV-002` | `DFP-001`, `ORS-002`, `DRD-005` | The existing doctor suites pass: `cargo test -p ratmac-qa --test t045_bootstrap_doctor` and `cargo test -p ratmac-qa --test t057_deep_doctor`. Existing write-free, state, Runbook, arbitrary-path, findings, exit-code, and JSON behavior remains green. |
| `DFPV-003` | `DFP-001` | `cargo test --workspace` passes from the public workspace after the focused regression is green. |
| `DFPV-004` | `DFP-001` | `cargo fmt --all -- --check` passes. |
| `DFPV-005` | `DFP-001` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes. |
| `DFPV-006` | `DFP-001` | The later ticket assesses every hidden lane: `Regression`, `Input/Routing`, `Lifecycle/Model`, `Durability/Recovery`, `Output/Filesystem`, and `Cross-Feature`. `Regression` and `Output/Filesystem` require coverage; every other lane records either coverage or a specific public not-applicable rationale. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | Add the reverse route for `i-023-doctor-full-fingerprint`. |
| `.arca/goal/ubi-lang.md` | unaffected | No new durable terms are introduced. |
| `.arca/goal/spec.md` | updated | Add accepted requirement `DFP-001` while retaining `ORS-002` and `DRD-005`. |
| `.arca/goal/design.md` | updated | Record the accepted full-digest rendering and unchanged-behavior boundary. |
| `.arca/goal/test-list.md` | updated | Add `DFPV-001` through `DFPV-006`. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No agent-policy change is proposed. |
| `.arca/schema.md` | unaffected | Existing issue-shape, P1, and evidence rules govern this bundle. |
| `.arca/dict.md` | unaffected | No new durable terms are needed; `ubi-lang.md` uses the template no-term sentence. |
