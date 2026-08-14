# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| `ECP` | **Edition-Channel Pin** - this issue's stable requirement-ID prefix. |
| Channel | Which build lane an engine binary came from: `stable` or `nightly`. A property of a built binary, recorded as provenance; never a property of source. |
| Stable Engine | The engine built at the newest edition recorded in the editions ledger. Drives Runs that judge landings on this repository; the publish artifact. |
| Nightly Engine | The engine built at the latest green landing. Dogfooded in trials and ticket worktrees; never gates its own promotion. |
| Provenance | The recorded origin of a binary: full source commit plus channel or edition name, carried beside the existing identity (path + sha256) in the `[engine]` pin. |
| Promotion | Cutting an edition. Not a new ceremony - the edition's proof block is the promotion gate. |
