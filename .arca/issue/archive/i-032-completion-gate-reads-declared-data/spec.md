# Issue specification

`CGD-001` and `CGD-002` were revised and accepted at the 2026-08-21 planning
pass (P1) per Billy's 2026-08-18 workflow ruling, signed by his 2026-08-21
sprint authorization. On 2026-09-18, during run-030 intake, Billy explicitly
approved the remaining Engine cutover in `CGD-003` after the workflow tags had
been proved on real tickets. `CGD-003` is now accepted into the
[forward product goal](../../../goal/spec.md#integrated-completion-from-declared-data-requirements);
the earlier accepted dispositions and their working-authority ownership do not
change.

`CGD` is this issue's stable requirement-ID prefix - **Completion from
Declared Data** - defined in [ubi-lang.md](ubi-lang.md).

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `CGD-001` | The ticket format the Plan-Build Runbook owns declares the checks a ticket must prove as three explicit front-matter tag-list fields - `focused-tests`, `hidden-lanes`, `quality-commands` - each entry an opaque check id taken verbatim. The ticket blank (`.arca/tpl/ticket.md`) and the working rules' ticket sections define the fields; a checker learns a ticket's checks from the tags, never from its prose. | accepted | Revised at P1 per Billy's 2026-08-18 ruling (steering Horizon): the declared-checks question is a workflow matter. The tags live in the ticket format the workflow owns; no Engine cutover is presumed. | [working authority](../../../schema.md#ticket-check-tags) |
| `CGD-002` | A tag list that is present but malformed - not a list of non-empty strings, or a duplicate id across the three fields - fails the ticket's shape check at creation, naming the field and the offending entry. A ticket declaring none of the three fields is not yet cut to this format and is judged by the rules it was cut under. | accepted | Revised at P1 to the workflow framing: malformed declarations refuse at the ticket shape check the workflow already runs, not in Engine code. Silent tolerance stays banned; the refusal moves to where the format is owned. | [working authority](../../../schema.md#ticket-check-tags) |
| `CGD-003` | The implementation-completion gate derives its focused, hidden, and quality check set only from three explicit declared lists whose project-specific field names come from runbook data, not Rust. This workflow currently names them `focused-tests`, `hidden-lanes`, and `quality-commands`. The gate performs no Markdown-heading, `HT-`-shape, or backtick inference and accepts heading-free and non-Markdown declaration carriers. Absent selected fields keep the existing `declares no checks` refusal. Explicit empty lists are legal, but an entirely empty combined set keeps that refusal rather than passing vacuously. A non-list value, empty-string entry, or duplicate refuses naming the field and entry with no mutation. Receipt kinds and format, Run-keyed evidence paths, target binding, green/self-consistency/freshness checks, missing/stray/duplicate safeguards, declared-set matching, and receipt-defect wording stay unchanged. The sensitivity gate and archives stay unchanged. | accepted | Billy explicitly approved the remaining cutover during run-030 intake on 2026-09-18, after `CGD-001` and `CGD-002` had been proved by the landed tag-reader workflow. Exact Machine Class grammar and migration remain for the upcoming goal design decision; this acceptance fixes observable behavior, not that mechanism. | [goal `CGD-003`](../../goal/spec.md#integrated-completion-from-declared-data-requirements) |

## Acceptance criteria

These are planned acceptance conditions; this planning pass provides no
implementation proof.

- A fixture can rename the three fields through runbook data, remove or rename
  every heading, and still derive the same focused, hidden, and quality set.
- The completion path has no Markdown-heading split, `HT-` shape match, or
  backtick scan, and Rust contains no project-specific declaration field name.
- Malformed lists, empty-string entries, and duplicates refuse by field and
  entry with no mutation; an empty list for one kind is legal, while absent
  selected fields or an entirely empty combined set keep the existing
  `declares no checks` refusal.
- Receipt format and paths, target binding, receipt safeguards and wording,
  sensitivity behavior, and archived history remain unchanged.
- A non-Markdown declaration carrier with no possible heading passes when its
  declared receipts are green, self-consistent, fresh, and complete.
