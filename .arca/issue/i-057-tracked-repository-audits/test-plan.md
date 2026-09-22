# Issue test plan

## Verification

These are proposed checks, not executed results. Each check references the observable contract in [the specification](spec.md#requirement-records).

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Ignored scratch checkout (`WEBV-009`) | `WEB-003` | Create a real temporary Git repository with tracked clean content and an ignored nested checkout containing legacy tokens. Both named acceptance audit paths remain clean, and a read-failure trap under that ignored directory is never visited. |
| Tracked changes (`WEBV-010`) | `WEB-003` | Stage a new offending file, edit a tracked clean file to contain a forbidden token, and use a forbidden tracked filename. Each produces the named violation from working bytes. Staged deletion removes input; unstaged missing input refuses. |
| Explicit extras (`WEBV-011`) | `WEB-003` | The same untracked offending file is invisible by default and fails when explicitly listed. An absent file, directory, absolute path, parent traversal, escaping symlink, malformed listing, or unusable Git refuses without silently dropping coverage. |
| History and determinism (`WEBV-012`) | `WEB-003` | Historical allowlist rows still permit only their intended carriers, stale rows still fail, and unusual filenames including spaces/newlines survive NUL-delimited listing with stable report ordering. Invalid UTF-8 and NUL-containing fixtures take the declared binary byte-scan path: forbidden ASCII tokens still fail by byte offset, clean binary bytes pass, and unreadable files refuse. |

## Goal/Test File Traces

Integration was accepted on 2026-09-22; the table below records the authority changes. Working authority gets the requirement and executable proof; product goal remains unaffected unless implementation identifies a product contract that truly depends on this audit.

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../schema.md#web-003--tracked-repository-audits). |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../schema.md#web-003--tracked-repository-audits). |
| `.arca/goal/spec.md` | unaffected | [Accepted authority](../../schema.md#web-003--tracked-repository-audits). |
| `.arca/goal/design.md` | updated | [Accepted authority](../../schema.md#web-003--tracked-repository-audits). |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../schema.md#web-003--tracked-repository-audits). |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change in this intake bundle. |
| `.arca/schema.md` | updated | [Accepted authority](../../schema.md#web-003--tracked-repository-audits). |
| `.arca/runbook-spec.md` | unaffected | Integration must update this authority if the accepted design adds or changes declared data. |
