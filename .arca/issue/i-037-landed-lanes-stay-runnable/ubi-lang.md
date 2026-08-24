# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Landed-lane runnability (`LNR`) | This issue's stable requirement-ID prefix: a landed ticket's private lanes either still run green against today's Engine or carry a visible expiry marker - never silently rotting. |
| Sweep | The one shop-lane command that runs every crate under `test-hidden/` from the primary checkout, outside any ticket turn, and reports one verdict per crate: `pass`, `expired`, or `red` with lane ids. |
| Expiry marker | The explicit record a lane carries when it can no longer run green, naming the edition it last passed at, the date, and the reason; written by an explicit act, never by silence. |
| Last-good edition | The edition an expiry marker names: the newest edition against which the crate's lanes ran green. |
