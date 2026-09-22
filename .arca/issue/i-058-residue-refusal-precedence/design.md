# Issue design

## Proposed mechanics

[cli.rs](../../../src/cli.rs) routes doctor before the common residue preflight and parses doctor's options before resolving its target. Scaffold and skill have separate path-taking boundaries. [scheduler.rs](../../../src/scheduler.rs) owns refuse_flat_residue and several workspace-specific checks; [scaffold.rs](../../../src/scaffold.rs) already preflights before target existence.

Assumed: the precedence order is target discovery, root resolution, residue inspection, then option and operation validation. Exact pure help forms are no arguments, a sole `--help` or `-h`, or a recognized command followed only by `--help` or `-h`; the optional leading executable name accepted by the library wrapper does not change this rule. A help token mixed with other operational arguments is not a bypass and is validated only after residue preflight. Unknown commands remain write-free usage refusals.

For target-taking commands, perform a non-validating scan that identifies the first positional target without interpreting malformed options as paths. Check invoking and clearly addressed roots before reporting later argument defects; if there is no unambiguous target, check the invoking root and then report usage. Document that ambiguity rather than inventing an addressed root. Resolve each relevant project once using the context established by the single-root issue.

Centralize the preflight entry boundary and require an already checked context in command handlers. Public path-taking library wrappers construct that context before any handler-specific reads. Preserve the named legacy-folder ownership exception and all existing residue classes; this issue does not introduce migration or rewrite history.

Inventory the exported Engine operations and command dispatch, including initialization, recovery, review-intent or authorization operations, and the nested launch/receipt endpoint as they land. Drive one parameterized matrix over malformed options, missing addresses, occupied target paths, bad confirmations, and missing blockers, with each known residue shape independently present. Add an observer or injected operation boundary in tests to prove roster/read/parse/write handlers are not called; byte snapshots alone do not prove read ordering. The inventory must have a completeness check against the public route table so a new verb cannot omit the matrix silently.

Confirmed retirement currently intentionally bypasses some live-run checks. Integrate the ruling explicitly: it may still bypass damaged runbook pins to recover a Run, but retired-layout residue remains the universal preflight and never triggers an implicit repair.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
