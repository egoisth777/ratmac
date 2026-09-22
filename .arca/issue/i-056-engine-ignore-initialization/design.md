# Issue design

## Proposed mechanics

[scaffold.rs](../../../src/scaffold.rs) deliberately creates exactly one runbook file and no directories. [root.rs](../../../src/root.rs) distinguishes the invoking checkout from the shared Engine root. [schema Engine-root tracking policy](../../schema.md#ens-012--engine-root-tracking-policy) requires ignoring live runtime while tracking runbook and evidence.

Assumed: add a small `rtm init` command dedicated to Engine storage initialization. Keep `scaffold`'s one-file contract. Initialization creates only the necessary Engine directory and an Engine-local `.gitignore` containing anchored runtime entries; it does not create a Machine Class implicitly. Teach the next scaffold command after success.

For an existing local ignore file, preserve all bytes and append only missing narrowly scoped lines after validating the resulting effective policy. Do not add blanket `.ratmac/` exclusions. An operator's broader inherited rule or a negation that makes runtime visible is a conflict, not permission to rewrite the shared repository ignore file. A refusal names the effective conflicting path and asks the operator to repair that rule. A pre-existing runtime file already in Git's index must also refuse; an ignore pattern cannot untrack it.

In Git repositories, check effective ignoring for representative runtime and authoring paths, including root-level and local rules and the initialized ignore file itself. An absent Git executable or a directory established to be outside a Git repository allows local policy installation with a clear statement that repository-level conflicts cannot be verified; it must not claim to have verified Git behavior. A corrupt repository, an unexpected failed Git query, or inability to inspect its index or effective rule result refuses rather than being classified as no Git. For a linked checkout, install/check the shared root's runtime policy and ensure the invoking checkout's runbook/evidence are not excluded.

Make `start` perform the same idempotent policy preflight before admission, so the existing simple first-start flow cannot bypass protection; plan and validate before mutation and roll back new ignore changes if admission fails. Initialization must serialize concurrent attempts without overwriting another writer. Include this command in residue preflight and help coverage. The accepted goal must explicitly permit this Engine-local ignore file and directory creation; no workflow-root write is needed.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
