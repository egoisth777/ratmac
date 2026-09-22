# Issue test plan

## Verification

These are proposed checks, not executed results. Each check references the observable contract in [the specification](spec.md#requirement-records).

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Portable text (`WEBV-001`) | `WEB-001` | Two isolated trees with identical names and LF versus CRLF text yield exactly the same versioned revision, including nested paths and mixed CRLF/LF content. Assert the fixed serialized-byte and revision vectors below independently of the production serializer. Run the fixture on Windows and a Unix platform when available; the serialization's fixed vectors are platform-independent. |
| Meaning remains visible (`WEBV-002`) | `WEB-001` | Rename/add/remove files, edit a character, add a final newline, alter a binary byte, and change a lone CR; each changes the digest. File enumeration order does not. Length framing distinguishes adversarial path/content boundary combinations. |
| Forward compatibility (`WEBV-003`) | `WEB-001` | An old baseline/freeze fixture verifies only with the old algorithm, stays byte-identical, and remains parseable in an archived record. A new fixture carries exactly v2: plus 64 lowercase hexadecimal digits and is accepted inside the compound goal-sha256:v2:<digest> citation. Unknown versions, malformed markers, wrong digest lengths, uppercase new digests, and invalid path encodings refuse without writes. |
| Drift boundary (`WEBV-004`) | `WEB-001` | Start/freeze/step fixtures cover both versions, a legacy baseline frozen after upgrade, and a semantic edit after freeze. Line-ending-only edits affect only the legacy fixture. |

### Fixed vectors

These expected vectors define planned serializer checks; they are not claims that the Engine implementation has passed. File contents are specified as hexadecimal bytes. The text vector also applies when its input is `68656c6c6f0d0a` (hello followed by CRLF). The binary vector keeps its NUL, CRLF, and invalid UTF-8 byte unchanged. The two-file vector must match even if the fixture creates `b` before `a`.

| Fixture | Expected serialized bytes, hexadecimal | Expected revision |
| :--- | :--- | :--- |
| Existing empty directory | `7261746d61632d676f616c2d7632000000000000000000` | `v2:985cf08ca7fbde22915e18c240e90faec480d42cdcbb9c6a2574ab7e8aa88528` |
| One file `a.txt`, content `68656c6c6f0a` | `7261746d61632d676f616c2d76320000000000000000010000000000000005612e747874000000000000000668656c6c6f0a` | `v2:f971951e85b5db5335843e6d326699f7f1fd0583f9698c7e520631e308ffb22f` |
| One file `bin.dat`, content `000d0aff` | `7261746d61632d676f616c2d7632000000000000000001000000000000000762696e2e6461740000000000000004000d0aff` | `v2:391218dbef39b1bdd53dd95d6c43ce1fca611af864da532fa7ee541a111c29f6` |
| Empty file `a`, file `b` with content `0a` | `7261746d61632d676f616c2d7632000000000000000002000000000000000161000000000000000000000000000000016200000000000000010a` | `v2:476aa6a2b832fd23a0f717bbbb70c7a9e2efae43a887282c93b2d3097a588718` |

## Goal/Test File Traces

Integration was accepted on 2026-09-22; the table below records the authority changes. Product goal specification/design/test list and working citation grammar need integration; no forward authority has been written.

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/spec.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/design.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change in this intake bundle. |
| `.arca/schema.md` | unaffected | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/runbook-spec.md` | unaffected | Integration must update this authority if the accepted design adds or changes declared data. |
