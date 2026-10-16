# Plan-Build profile terms

One line per term the profile's contract, schemas, and instructions use.

- Plan-Build profile - the distributable bundle defining the artifact shapes, checks, templates, words, and Machine Class needed to run the planning and build cycle.
- profile compatibility - the explicit relationship between a profile format and the Engine or checker versions that can interpret it.
- manifest - the bundle's `manifest.toml`, the strict-TOML inventory and compatibility record; it lists and hashes every artifact but is not self-listed, because it cannot hash itself.
- artifact - one manifest-listed file of the bundle; a bundle is exactly its manifest inventory.
- packaged checker - one of the two Python scripts under the bundle's `checkers/`, run through the runbook's `command_exit` guards.
- installed profile - a bundle placed inside a target repository at the path the runbook's guard rows name; as shipped that path is `profile/` at the repository root.
- dialect - the runbook vocabulary generation an Engine can read; this profile declares `states-v1`.
- clean repository - a Git repository with no `.arca/schema.md`, no `.ratmac/`, and no inherited agent prompt.
- initialization - the operator sequence `OPERATING.md` teaches: export or place the bundle, create the declared roots from the blanks, place the Machine Class at `.ratmac/ratmac.toml`, then run Engine commands.
- Machine Class - the plain-TOML file that declares roots, States, prompts, guards, transitions, and classes; the Engine instantiates it into a Run and is its only runtime writer of state.
- State - one position of the machine graph a Run occupies; prompts face the agent and guards face the disk.
- Run - one live instance of the Machine Class the Engine drives, with its own id, its own Run Record, and its own evidence.
- Run Record - the one file the Engine writes for one Run, `.ratmac/runs/<run-id>/run.toml`, holding its current State, status, and run-scoped fields; only the Engine writes it.
- Exit Guard - the machine's own reading of what is on disk at a State boundary - a file shape, a file content, or a command's exit code - never an agent's claim.
- goal bundle - the frozen requirement records beneath the runbook's declared goal root, one requirement row per accepted ask.
- issue bundle - the exact five-file issue folder (`design.md`, `index.md`, `spec.md`, `test-plan.md`, `ubi-lang.md`) whose asks intake dispositions.
- residual record - the gap record that answers one requirement as missing, partial, or satisfied, with its evidence pointers and rationale.
- work item - the self-contained unit a gap's owner cuts beneath the ticket root, declaring its own checks and merge gate.
- sensitivity receipt - the recorded proof that a planned test runs and fails for the stated reason before its implementation exists.
- completion receipt - the recorded proof that one declared check ran green and fresh against the tree as it stands.
- edition - the annotated Git tag (`edition-NNN`) that marks the commit a sprint closes on; sequential, never moved or overwritten.
- blocked route - the human-confirmed pause transition a held Run waits on, declared by the runbook with `blocked-route = true`.
- pause destination - the destination of a State's declared blocked route; this Machine Class declares each working State's pause destination as the State itself, so the graph position survives the hold.
