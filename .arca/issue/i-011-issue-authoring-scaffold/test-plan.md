# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `IASV-001` | `IAS-001` | A folder whose five files are the unmodified blanks, with `status` set to `integrated`, is refused by the intake gate naming the file and the first unfilled placeholder. The committed probe folder at trial commit `e77f679` is the fixture: it passes today. |
| `IASV-002` | `IAS-001` | A folder whose `index.md` states an `issue-id` other than its folder name is refused naming both values; a matching one passes. |
| `IASV-003` | `IAS-001` | A folder whose `provenance` is empty, whitespace, or still the placeholder is refused; a filled one passes. |
| `IASV-004` | `IAS-002` | The authoring check reports zero defects on this issue folder while its `status` is `pending`, and reports the same defect list as the intake gate for a folder that is defective for any non-status reason. |
| `IASV-005` | `IAS-002` | The authoring check and the intake gate disagree on exactly one input class: terminal status. Proven by running both over the same fixture set. |
| `IASV-006` | `IAS-003` | Scaffolding into a repository whose highest issue is `i-011` produces `i-012`, with `issue-id`, folder name, provenance, and `status: "pending"` already consistent, and the authoring check green on the untouched output except for author fields. |
| `IASV-007` | `IAS-003` | Scaffolding refuses when the target folder exists, when the derived number collides with an archived issue, and when the slug is malformed; after each refusal the tree is byte-identical. |
| `IASV-008` | `IAS-004` | The blank `.arca/tpl/issue/spec.md` carries the disposition note that five issues currently hand-repeat, and the scaffold leaves integrator columns unfilled without the authoring check objecting. |
| `IASV-009` | `IAS-005` | A requirement row whose disposition column is `rejected` but whose rationale contains the word accepted is not counted as accepted. This fails against the current parser. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/current/index.md` | updated | Link `i-011-issue-authoring-scaffold` and note that issue authoring is scaffolded and checkable before integration. |
| `.arca/current/ubi-lang.md` | updated | Define Authoring check, Issue scaffold, Requirement prefix, Author field, and Integrator field. |
| `.arca/current/spec.md` | updated | Integrate `IAS-001`–`IAS-005` with stable requirement IDs; record `IAS-006` as deferred. |
| `.arca/current/design.md` | updated | Record that the authoring check and the intake gate share one implementation with the terminal-status rule as a parameter, and that disposition is read by column. |
| `.arca/current/test-list.md` | updated | Add `IASV-001`–`IASV-009`, including the refusal negatives and the shared-implementation agreement check. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | Remains the one-line pointer to `.arca/index.md`. |
| `.arca/index.md` | updated | State the issue numbering rule, the scaffold entry point, and that the three front-matter rules are now mechanized rather than asserted. |
| `.arca/tpl/issue/spec.md` | updated | Absorb the disposition note currently hand-repeated in five issues; mark integrator columns. |
| `tools/check_links.py` | updated | Either grows the `pending` authoring mode or is retired in favour of the shared implementation; decided at integration. |
| `.arca/state.toml`, `.arca/log.md`, `.arca/rtm.lock` | unaffected | Authoring and checking touch no Scheduler-owned runtime artifact. |
