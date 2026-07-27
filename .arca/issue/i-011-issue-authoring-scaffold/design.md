# Issue design

## Proposed mechanics

Extend what exists; add no new subsystem.

1. `IAS-001` and `IAS-005` are changes inside `src/contract.rs`. Add a
   placeholder scan over all five files, an `issue-id` equals folder-name
   check, and a non-empty non-placeholder `provenance` check to the per-folder
   loop in `gate_intake`. Change `accepted_requirements` to split the row and
   read the disposition column by index, instead of testing whether the whole
   lowercased row contains the word.

2. `IAS-002` is a shared entry point rather than a second implementation.
   Split the per-folder body of `gate_intake` into a function taking the
   terminal-status rule as a parameter, so the authoring check and the intake
   gate run the same code with one rule toggled. A second implementation would
   drift from the first, which is how the current split between
   `tools/check_links.py` and `gate_intake` already behaves.

3. `IAS-003` follows the trial precedent: derive identity, collision-check,
   then write, with rollback leaving nothing behind. `tools/trial.ps1` start
   is the working model, including the printed recovery commands.

4. `IAS-004` is a template edit plus the note now hand-repeated in five
   issues. Move that sentence into `.arca/tpl/issue/spec.md` and mark the
   integrator columns so the scaffold can leave them alone.

Placeholder syntax: the issue blanks use double braces and the trial-log blank
uses angle brackets. The scan should accept the issue blanks' own syntax
rather than unify the two, since the trial-log validator already ships and
works.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
