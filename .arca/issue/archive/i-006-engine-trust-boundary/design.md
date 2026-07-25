# Issue design

## Proposed mechanics

Suggested design only — the binding requirements live in [spec.md](spec.md); none of the mechanism choices below carry weight until folded into the goal.

- Pinning: at `rtm start` (or first guard use), resolve each project-derived guard program to a prebuilt executable in a Run-scoped location, hash it with SHA-256, and record engine, gate, and Runbook identities together in Run evidence. At every guard evaluation, re-hash the artifact and refuse on mismatch. A simpler alternative that removes the second artifact entirely: fold the gate predicates into the pinned `rtm` binary itself (for example `rtm gate <predicate>` or in-process predicates), so the existing Stable Engine pin covers all routing logic; this trades binary size for a single trust surface and is the preferred direction if predicates and Engine can co-version.
- Exemption: mark non-project probe commands explicitly in the Runbook (or detect an allowlist such as `rustc --version`) so the pin rule stays enforceable without forbidding toolchain checks.
- Diagnostics: replace the null stderr wiring in the command-guard evaluator (the Engine's `command_exit` path in `src/scheduler.rs` at the current checkout layout) with a bounded capture — for example the last 4 KiB of stderr — embedded in the `GuardFailure` observed text, with an explicit `…truncated` marker and a fixed `no diagnostic emitted` placeholder for silent failures. Keep the bound deterministic so refusal output is reproducible.
- Freeze: compute the goal content hash inside the transition that closes intake integration, store both `baseline_revision` (from Run creation) and `goal_revision` (post-integration) as distinct Run-evidence fields, and verify the frozen hash at each subsequent transition request until batch closure, refusing on drift. Do not reuse one field for both meanings.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
