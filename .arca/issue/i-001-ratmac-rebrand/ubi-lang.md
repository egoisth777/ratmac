# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| `ratmac` | The canonical product/project identity replacing `arca-scheduler` in active knowledge, package metadata, source-facing names, and user-facing text. |
| `rtm` | The canonical executable and command name replacing `schd`; examples use `rtm start`, `rtm step`, and `rtm status`. |
| Legacy identity | The superseded spellings `arca-scheduler` and `schd`, retained only where an append-only or historical artifact must preserve provenance, or where a compatibility decision explicitly allows them. |
| Clean cutover | The default migration policy for this issue: new installs, docs, tests, and commands use only `ratmac`/`rtm`; no compatibility executable, alias, package fallback, or silent legacy behavior is added. |
| Compatibility decision | The recorded choice for each externally observable legacy surface (command, package/crate, diagnostics, and transient lock path), including migration behavior and verification evidence if compatibility is retained. |
| Active knowledge | Current SSOT and contributor-facing material that controls or describes present behavior: `.arca/current/*`, `.arca/index.md`, live project docs/configuration, and executable tests/fixtures. |
| Historical artifact | An append-only log or archived record whose old spelling is part of its provenance and is not rewritten merely to make a new active identity look current. |
