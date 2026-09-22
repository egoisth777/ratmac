# Issue design

## Proposed mechanics

[scheduler.rs](../../../src/scheduler.rs) contains direct stderr diagnostics around recovery and lock/fault paths; [Engine entry point](../../../src/bin/rtm.rs) writes mandatory errors directly. [cli.rs](../../../src/cli.rs) writes normal reports through its provided output writer. Existing compiled fault points are used for deterministic concurrency and recovery testing.

Assumed: `RATMAC_TRACE=1` enables tracing; absent, empty, or `0` disables it, and another value returns a clear usage diagnostic without enabling it. Read this setting once while building invocation context. The small enabled flag is transient configuration, not persisted Engine state or a global mutable event log. The no-allocation requirement covers every disabled trace call; initialization must avoid an allocation when the variable is absent and account separately for reading an explicitly present setting.

Add one module for mandatory diagnostics and optional structured events. Route existing direct stderr writes through it rather than gating away errors users currently need. This distinguishes a required error from an optional decision record while keeping one writer. Preserve established mandatory-error text, including its actionable paths; the deterministic-byte and no-absolute-host-path restrictions apply to structured trace records only. Give those records an unambiguous format so tests can separate them from mandatory diagnostics without changing either meaning. Use a fixed record schema, deterministic field order, escaping, and a fixed event vocabulary; where a refusal has no stable code yet, assign one in its owning error contract and reuse it rather than manufacturing a trace-only guess.

Use root roles and repository-relative paths. Do not copy free-form error/child output, executable absolute paths, environment values, timestamps, process IDs, or lock-owner nonces into trace fields. Represent external paths by a declared role or redacted marker. Stable Run addresses and declared state/input names may appear only when the same input state determines them. Trace-write failure cannot alter a guard verdict or runtime transition; mandatory error handling retains its existing semantics.

Keep compiled pause points: they control timing for fault tests, whereas trace only observes. Hidden behavioral lanes must judge real state, receipts, exits, or outputs already in the product contract, never trace prose. Dedicated public trace-format and routing tests may assert the structured record contract, including an allocation counter around a disabled event; this is output verification, not gate evidence. Guards must not inherit an Engine trace channel as their proof input, and captured child trace cannot contaminate accepted receipt channels.

The implementation inventory must include every direct stderr write in src/, including binary entry points, so no parallel diagnostic owner survives.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
