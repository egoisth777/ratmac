# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WEB-001` | Goal baselines and freezes for newly started Runs use one versioned, deterministic serialization: relative paths use forward slashes, path ordering is bytewise and case-sensitive, records are length-delimited, and CRLF pairs in valid UTF-8 text without NUL bytes become LF while all other content bytes retain their meaning. Identical relative paths and canonical content produce identical revisions across platforms; added, removed, renamed, or semantically edited files change the revision. Binary content is hashed unchanged. Existing unversioned fingerprints keep their original algorithm and semantics for live Runs, including a legacy baseline first frozen after the upgrade; archived stamps stay byte-identical and parseable. An unknown version or an unrepresentable path refuses by name without updating evidence. No existing pin is silently upgraded or accepted merely because it matches either of two algorithms. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |
