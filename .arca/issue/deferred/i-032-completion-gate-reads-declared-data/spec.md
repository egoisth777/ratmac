# Issue specification

Dispositions were revised and decided at the 2026-08-21 planning pass (P1)
per Billy's 2026-08-18 ruling recorded in the steering Horizon, signed by
Billy's sprint authorization of 2026-08-21. The requirement texts of CGD-001
and CGD-002 were rewritten at that pass to the workflow framing; the original
Engine-cutover asks survive in this bundle's design and acceptance notes as
incoming evidence for the deferred CGD-003.

`CGD` is this issue's stable requirement-ID prefix - **Completion from
Declared Data** - defined in [ubi-lang.md](ubi-lang.md).

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `CGD-001` | The ticket format the Plan-Build Runbook owns declares the checks a ticket must prove as three explicit front-matter tag-list fields - `focused-tests`, `hidden-lanes`, `quality-commands` - each entry an opaque check id taken verbatim. The ticket blank (`.arca/tpl/ticket.md`) and the working rules' ticket sections define the fields; a checker learns a ticket's checks from the tags, never from its prose. | accepted | Revised at P1 per Billy's 2026-08-18 ruling (steering Horizon): the declared-checks question is a workflow matter. The tags live in the ticket format the workflow owns; no Engine cutover is presumed. | [working authority](../../../schema.md#ticket-check-tags) |
| `CGD-002` | A tag list that is present but malformed - not a list of non-empty strings, or a duplicate id across the three fields - fails the ticket's shape check at creation, naming the field and the offending entry. A ticket declaring none of the three fields is not yet cut to this format and is judged by the rules it was cut under. | accepted | Revised at P1 to the workflow framing: malformed declarations refuse at the ticket shape check the workflow already runs, not in Engine code. Silent tolerance stays banned; the refusal moves to where the format is owned. | [working authority](../../../schema.md#ticket-check-tags) |
| `CGD-003` | Every guarantee `PGE-005` carries per receipt is unchanged: green, self-consistent, fresh, declared, no stray receipt beside the declared set, and the receipt's item field must match the addressed binding. Only the source of the declared-check list moves; the receipt format, the evidence path keyed by the Engine-minted Run id, and the refusal wording for receipt defects stay as they are. | deferred | Deferred at P1: the Engine-side cutover - the completion gate consuming the declared tags instead of parsing prose - follows once the tag format is proven on real tickets; Billy's ruling presumes no Engine cutover this sprint. | - |
## Acceptance criteria

- `src/completion.rs` contains no `## Merge Gate` split, no `HT-` shape
  match, and no backtick scan; `rg "Merge Gate|split|backtick"` over the gate
  finds only the typed declaration reader.
- A ticket whose front matter declares the three lists gates exactly as
  today's prose-parsed ticket did (the live tickets' declared sets are
  byte-equal before and after the cutover, proven by a fixture pair).
- A declaration that is present but malformed refuses naming the field and
  entry, and writes nothing.
- A work item that is not markdown but carries the declared fields passes the
  gate; the gate never requires the item to hold any heading.
- Every check `PGE-003` and `PGE-005` already carry per receipt still passes
  on this repository as it stands.
