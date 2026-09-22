# Issue design

## Proposed mechanics

[root.rs](../../../src/root.rs) already defines Roots with private paths but exports both Roots::resolve and resolve. [cli.rs](../../../src/cli.rs) mixes roots-taking and path-taking helpers. [scheduler.rs](../../../src/scheduler.rs) repeats resolution through public opens, residue wrappers, and workspace validation; status comments admit current tests cannot detect replacing the roots-taking path.

Assumed: promote the existing Roots value into the invocation's authority instead of adding a global cache. One owner at dispatch resolves it. Context-bound scheduling, diagnostic rendering, residue checking, and helpers receive a reference or owned clone; they never accept a raw root as a substitute for the context. Retain public path-taking compatibility wrappers as fresh entry points, but keep them out of the internal handler API.

Restrict the production resolver's visibility to entry construction. Separate pure formatting from discovery; reports derive their path only from the same context that opened the Run. For an addressed workspace, validate canonical membership against the context's repository identity rather than starting another root search. A genuinely different explicit target gets its own context, cached per invocation by canonical project identity.

Introduce a small injected resolver boundary for tests, with a call counter and scripted results. Production still uses the existing Git/fallback logic, without a dependency or runtime global. The test double returns a different root or errors if called twice; exercise full dispatch and library entry wrappers so any accidental duplicate is visible even when ordinary Git would answer identically.

Coordinate first with residue precedence. Nested independent invocations get separate contexts and do not inherit the parent's runtime root; they are separate Engine entries, not extra resolutions hidden in one handler.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.

