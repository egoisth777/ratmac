# Issue design

## Proposed mechanics

Suggested design only — the binding requirements live in [spec.md](spec.md); none of the mechanism choices below carry weight until folded into the goal.

- Policy surfaces: update the `rtm start` help text in `src/cli.rs` (current checkout layout) to the human-approved Main-Agent policy; align `AGENTS.md` and any canonical skill shipped on this branch; implement the stale-policy audit as a QA test scanning active surfaces for the retired wording. Treat conversational sign-off as sufficient; optionally bind one sign-off to one start attempt in guidance to limit ambiguity.
- Bootstrap: a `rtm doctor`-style read-only subcommand plus one repo-local launcher (for example a `bin/` script or documented `cargo run` alias) that resolves the Engine binary from the project-local build or a recorded pin path, hashes it, compares against the pin record when present, and prints path, hash, Runbook validity, and state summary. Keep it offline and side-effect free; when no Run exists, print the distinction between `.arca/ratmac.toml` (human-authored Runbook) and `.arca/state.toml` (Scheduler-owned runtime state) with the legitimate next step.
- Behavioral harness: represent role scenarios as recorded transcripts of attempted commands or tool calls (produced by scripted agent sessions where available, otherwise curated fixture transcripts clearly labeled as such), with QA assertions over the transcript — presence of exactly one start invocation for the signed-off Main-Agent scenario, zero `rtm` invocations for unsigned and Subagent scenarios — plus one deliberately violating transcript that must fail. Record each check's evidence kind (behavioral versus guidance-consistency) in its output so residual classification cannot conflate them.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
