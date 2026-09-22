# Issue design

## Proposed mechanics

[goal.rs](../../../src/goal.rs) sorts displayed relative paths but hashes raw bytes with newline delimiters. [pin.rs](../../../src/pin.rs) stores baseline and frozen revisions as strings; [contract.rs](../../../src/contract.rs) consumes the frozen citation. Goal freeze and drift remain the authority in [goal specification](../../goal/spec.md).

The new revision string is exactly `v2:` followed by 64 lowercase hexadecimal SHA-256 digits. The existing compound citation carries it as `goal-sha256:v2:<digest>`; its other fields retain their existing grammar. A bare 64-digit digest selects the legacy algorithm. All revision producers, comparison consumers, working-record parsers, and proof helpers adopt this grammar together before new stamps are minted. No mandatory field is added to historical records.

The hashed serialization begins with the ASCII bytes `ratmac-goal-v2` followed by one NUL byte, then an unsigned 64-bit big-endian file count. Each file entry contains, in order: an unsigned 64-bit big-endian path-byte length, the UTF-8 relative-path bytes with forward-slash separators, an unsigned 64-bit big-endian canonical-content-byte length, and those content bytes. No delimiter, terminator, padding, or other byte is added. Sort entries by their relative-path bytes in ascending bytewise order before serializing; reject unsupported path encoding instead of using lossy conversion. Preserve case and Unicode code points; do not promise equality for trees whose names differ. Reject any count or length that cannot fit the declared unsigned 64-bit field.

Content is text exactly when it is valid UTF-8 and contains no NUL byte. For that text, replace each CRLF byte pair with LF and change nothing else: preserve lone CR, byte-order marks, and final-newline presence. Every other file is binary and contributes its raw bytes unchanged, including any CRLF pairs. Empty directories contribute no entries; an existing empty goal directory has the zero-file serialization, while an absent goal retains the existing absent-goal behavior. A non-regular file or link whose contents cannot be represented safely refuses rather than following a host-specific target.

Use the recorded version at every drift comparison. A Run whose baseline predates the change keeps that version through its freeze; a newly started Run uses the new version. Historical goal stamps are provenance, not a request to recompute them with today's algorithm. Document how an operator starts a new portable Run without rewriting an old Run or archive.

Scope includes the fingerprint producer, comparison consumers, citation parsers, and their documentation. It does not change runbook byte pins, executable hashes, receipt-output hashes, or snapshot hashes: those attest exact bytes.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
