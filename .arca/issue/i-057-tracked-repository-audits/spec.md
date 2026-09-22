# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WEB-003` | Tree-scanning repository acceptance audits enumerate tracked and staged paths from the addressed repository index, read their current working-tree bytes, and never recurse into ignored or untracked directories merely because they exist. This applies to the shared retired-name audit and every acceptance check that claims to scan repository content. An intentionally untracked input is included only by a caller-supplied explicit relative file list, is reported as an extra input, and is subject to the same checks. Missing tracked inputs, invalid or escaping extra paths, unreadable inputs, and repository-listing failures are named failures, not silent omissions or a fallback directory walk. The audit declares how it handles binary inputs, reports any deliberate exclusions, and never disguises decoding failure as an unreadable-file omission. Existing historical allowlist matching and stale-entry detection remain enforced against the selected inputs. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../schema.md#web-003--tracked-repository-audits) |
