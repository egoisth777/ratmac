# Bootstrap the Engine Cargo actually built

```yaml
issue-id: "i-064-cargo-built-bootstrap-engine"
provenance: "Wishlist .arca/wishlist.md:25, \"The bootstrap finds, reports, and pin-checks the Engine binary Cargo actually built, wherever Cargo's build output goes\": Billy selected this wish for the next sprint on 2026-10-26."
status: "integrated"
```

## Summary

The project-local bootstrap (`tools/rtm.ps1`) builds the Engine with Cargo but then looks for it only in `target/release` and `target/debug` under the checkout. When Cargo's build output goes elsewhere, it either takes an older binary from one of those folders in place of the fresh build or refuses after a successful build. The bootstrap should hash, report, and pin-check the binary Cargo says it built, or, when it reuses without building, the one in the output folder Cargo resolves, wherever that is. A stable build keeps its output inside its own build checkout, and a project-root build whose output folder lies outside the project builds before taking a binary, so no other checkout's binary is ever taken.

Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.

Provenance: [selected wishlist](../../wishlist.md), line 25 at filing; the exact source entry stays unchanged until its desired end is proved.

Ideal-shape properties advanced: **Self-hosted; Every boundary machine-checked** in [steering](../../steering.md#ideal-shape).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](packs/repos/skills/ratmac/.arca/issue/i-064-cargo-built-bootstrap-engine/ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Planning decision

Accepted on 2026-10-27 under Billy's 2026-10-26 wish selection after independent
contract review. Advances the steering's Self-hosted and Every boundary
machine-checked properties through its stated outcome. Its open decisions are settled
and its accepted assumptions recorded in the forward authority. Integration makes
no implementation claim.
