# Pin-check the Engine against the live Runs' pins

```yaml
issue-id: "i-074-live-run-pin-check"
provenance: "Manual plan-build observation, 2026-10-27 - while filing the cargo built bootstrap engine issue (i-064), `pwsh -NoProfile -File tools/rtm.ps1 -Channel stable` printed `Pin: no pin recorded in .ratmac/evidence.toml` and exited 0 while run-060 was live with its pin in `.ratmac/runs/run-060/evidence.toml`."
status: "integrated"
```

## Summary

The project-local bootstrap (`tools/rtm.ps1`) checks the Engine it resolves against a pin it reads from `.ratmac/evidence.toml`. The Engine no longer writes that file: each Run keeps its own pin in `.ratmac/runs/<id>/evidence.toml`. So the bootstrap always reports that no pin is recorded and never compares, even while a live Run has pinned the very Engine it resolves. The bootstrap should compare the Engine it resolves with the pin of every live Run in the one shared Engine root and report each. It should refuse only when the Engine it resolves is the file a live Run pinned and that file's bytes have changed since, so the default and nightly commands keep working while a sprint Run pins the stable Engine.

Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.

Provenance: manual observation during the 2026-10-27 planning pass; no wish carries it. The [cargo built bootstrap engine issue](../i-064-cargo-built-bootstrap-engine/spec.md) records the same finding as outside its scope.

Ideal-shape properties advanced: **Every boundary machine-checked; Self-hosted** in [steering](../../steering.md#ideal-shape).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](packs/repos/skills/ratmac/.arca/issue/i-074-live-run-pin-check/ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Planning decision

Accepted on 2026-10-27 under Billy's 2026-10-27 planning decision: refuse only when
the pinned file changed. Advances the steering's Every boundary machine-checked and
Self-hosted properties through its stated outcome. It has no open decision, and its
accepted assumptions are recorded in the forward authority. Integration makes no
implementation claim.
