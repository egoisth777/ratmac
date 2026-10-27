# Issue design

## Proposed mechanics

The five handlers that keep the last `--run` get the repeat check the shared parser already has. Nothing else changes.

In [src/cli.rs](../../../src/cli.rs), the `"--run"` arm of `fn hold`, `fn resume`, `fn recover`, `fn spawn`, and `fn respawn` first tests `run.is_some()`, before it reads the value, as `fn addressed_run_with_roots` does at line 193, so `--run run-001 --run` also refuses as given twice. A repeat returns `CliError::refusal` (exit code 2) with `"<command>: --run given twice; address exactly one run; {}{}"`, filled from the same `roster_line(roots)` value that command's missing-value refusal uses. The check sits inside the existing argument loop, so it runs before every lookup, read, and write that follows the loop. Routing the five through the shared parser, as `fn intent` does, would add its `next:` line to them, so the in-place check is the smaller change. `fn status`, `fn step`, `fn intent`, and `fn abandon` do not change.

The mechanics implement [WTC-001](spec.md#requirement-records) and are tested by the [verification plan](test-plan.md#verification).

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
