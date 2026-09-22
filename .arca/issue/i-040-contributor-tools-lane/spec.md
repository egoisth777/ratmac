# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WCP-001` | Every executable contributor-tool change has a declared owner, a carrier in the existing issue-to-ticket process, and durable evidence appropriate to its changed behavior. The working rules name the lane and apply it to existing tools and future tools regardless of implementation language. A behavior change requires a failing behavioral check before implementation and a passing check after it; a behavior-preserving change records the narrow proof that establishes preservation. Pure documentation keeps its existing shop lane. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../schema.md#wcp-001--contributor-tools-lane) |

## Evidence and boundaries

The lane list in [working rules](../../schema.md#units-and-git) names Engine source, tests, runbook, and documents but not executable tools. Existing [turn lifecycle](../../../tools/turn.ps1) and [trial lifecycle](../../../tools/trial.ps1) already demonstrate ticketed executable behavior.

Existing files are observed evidence. Proposed checks below have not been executed and are not satisfaction evidence.

## Assumptions

Use the existing Program lane for executable workflow behavior instead of introducing a third lane. Existing PowerShell and Python tools retain their languages; this ruling does not authorize new tools, dependencies, or rewrites.

## Dependencies

No new prerequisite. This working-authority ruling should precede executable deliverables in the other contributor-workflow issues.

## Open decisions

No unresolved disposition remains. The requirement was accepted on 2026-09-22; the linked forward authority governs implementation and its adopted assumptions.
