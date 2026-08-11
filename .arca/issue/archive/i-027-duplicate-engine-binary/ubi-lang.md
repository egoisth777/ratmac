# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| `DEB` | **Duplicate engine binary** — this issue's stable requirement-ID prefix. |
| Build target | One thing the build produces from source: a command a person can run, or a library other code links. Each target has a name and one output file. |
| Output collision | Two build targets that write the same output file. Whichever finishes last is the one that survives, so the result depends on build order rather than on what was asked for. |
| Pause point | A wiring seam, compiled in only for testing, that lets a test stop the engine at a named moment mid-operation, look at the tree, and let it continue. Absent from the shipped command by design. |
