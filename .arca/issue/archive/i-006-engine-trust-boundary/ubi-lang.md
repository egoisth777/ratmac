# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Stable Engine pin | The recorded identity (resolved path plus content hash) of the `rtm` binary that owns the active Run. |
| Pinned gate artifact | A prebuilt executable whose resolved path and content hash are recorded alongside the Stable Engine pin; the only project-derived code a command guard may execute during a Run, with any byte change detectable afterward. |
| Refusal diagnostic | The bounded observed-versus-expected text a refused transition prints, naming the concrete artifact or predicate the agent must repair. |
| Start baseline revision | The content revision of `.arca/current/` recorded when the Run is created, before any intake integration. |
| Frozen goal revision | The content revision of `.arca/current/` computed after intake integration completes; the only revision gap analysis and residual records may cite as the freeze. |
| Goal drift | Any change to `.arca/current/` observed after the frozen goal revision is recorded and before the build batch closes. |
