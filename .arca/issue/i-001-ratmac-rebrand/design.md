# Issue design

## Proposed mechanics

This is an incoming change proposal; the mechanics below carry no authority until the issue is integrated into the frozen goal.

1. **Inventory and boundary.** Enumerate all tracked active references to `arca-scheduler` and `schd` across `.arca/current/`, `.arca/index.md`, Rust manifests/source, QA code/fixtures, checked-in docs/configuration, and generated metadata. Classify each hit as active, generated, append-only, or historical. Preserve the allowlisted append-only and archive records rather than mutating their provenance.
2. **SSOT first.** Update the active goal bundle to make `ratmac` the product name and `rtm` the executable name. Keep the established Machine/Run/Phase/Status semantics, `ratmac.toml` class filename, `.arca/state.toml`, and `.arca/log.md` layout unchanged unless the compatibility decision explicitly covers a path. Update examples, command tables, glossary entries, and design/test references together so the bundle has one vocabulary.
3. **Rust identity cutover.** Rename Cargo package/library/dependency/import-facing names to `ratmac`, rename the binary source/manifest entry to `rtm`, and update runtime diagnostics/help and all QA invocations. Use a symbol-aware rename where language-server support exists; otherwise perform a constrained source edit followed by compilation and the repository audit. Do not add an old-name re-export or second binary under the clean-cutover default.
4. **Runtime path decision.** Decide the transient lock spelling before changing code. The proposed clean-cutover path is `.arca/rtm.lock`; preserve lock acquisition, release, refusal, and crash-safety semantics. If a legacy `.arca/schd.lock` can be encountered, select a deterministic refusal/manual migration or explicitly tested one-time migration; never silently remove it or let a second invocation proceed.
5. **Tests and fixtures.** Rename test package/bin metadata, imports, helper command runners, fixture labels, expected diagnostics, and executable smoke invocations. Keep fixture data semantics and all existing behavior assertions unchanged. Add focused coverage for canonical `rtm` routing, old-name rejection/compatibility policy, generated metadata, stale-name allowlist, and chosen lock migration behavior without testing implementation details.
6. **Generated assets.** Regenerate `Cargo.lock` and any checked-in package/manifests or generated docs with Cargo/project tooling after source metadata is correct. Do not commit ignored `target/` outputs or treat a locally built binary as a source artifact.
7. **Verification and review.** Run the stale-name audit with its explicit historical allowlist, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, full `cargo test`, QA/hidden lanes, and real binary smoke commands for `rtm start`, `rtm status`, `rtm step`, help, and an error path. Review the diff for behavior-neutrality, accidental data-layout changes, missing callsites, and compatibility claims.

## Explicit exclusions

- No implementation, source edit, package publication, commit, push, deploy, or issue integration is performed by this incoming issue artifact.
- No scheduler behavior change: Machine state, Status semantics, guard evaluation, state ownership, transition logging, print-first behavior, and strict `ratmac.toml` parsing remain as currently specified.
- No new compatibility alias, deprecation shim, process-management mode, run-id feature, or unrelated cleanup is introduced.
- No rewriting of append-only `.arca/log.md`, archived ticket records, or ignored build outputs solely for branding.
