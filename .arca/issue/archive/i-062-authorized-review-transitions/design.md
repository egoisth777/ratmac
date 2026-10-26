# Issue design

## Proposed mechanics

Add a generic optional review-policy declaration to the single Machine Class
parser. It describes public identities and roles, the distinct-review count,
and the pinned signature verifier. It has no project, ticket, or issue names.
Strict parsing and doctor findings must reject malformed policies.

Those declarations select an externally enrolled policy; they do not establish
their own trust. Enrollment is controlled outside the builder's write and
signing authority and binds the builder's actual key identity, permitted review
keys and roles, policy digest, and verifier executable digest. Admission checks
the declaration against that enrollment. Relabeling the builder key, replacing
the verifier, or supplying a self-signed policy cannot pass admission. The
implementation must state the host mechanism that protects enrollment rather
than claim an agent-writable file or an unkeyed hash is an authority boundary.

The selected production verifier is the host's OpenSSH `ssh-keygen -Y verify`
with Ed25519 detached signatures, exact enrolled identities, and namespaces
`ratmac-review-v1`, `ratmac-approve-v1`, `ratmac-resume-v1`, and
`ratmac-recover-v1`. It receives canonical
length-framed payload bytes on standard input and is judged by exit status,
never diagnostic prose. Verification also checks its enrolled executable path
and digest; `check-novalidate` is not an authorization operation.

An operator-controlled protected trust origin supplies the enrollment manifest.
An external path alone is insufficient: the implementation must validate its
protected ownership/write boundary or an independently anchored enrollment
signature. The manifest binds project scope, builder actor and all its keys,
reviewers, Approver, minimum count, permitted role combination, and verifier.
Neither an environment override nor a runbook declaration may replace that
origin. No enrollment or signer private material is supplied by this dispatch;
live activation therefore remains an explicit external prerequisite while
implementation and isolated verification proceed. The Engine must refuse
missing enrollment rather than generate a substitute identity.

The selected protection mechanism is the host's ownership and access control,
not a new secret store. The explicitly addressed manifest and every ancestor
must be protected against the actual builder operating-system identity,
including replacement and permission changes. Unix enrollment is root-owned,
with no builder, group, or other write permission along the path. Windows
enrollment is owned by System or Administrators; effective builder-token access
must exclude writing, deletion/replacement, changing ownership, and changing
permissions along the path. Builder ownership, an elevated builder capable of
taking ownership, reparse/symlink paths, and unverifiable protection refuse.
The manifest explicitly binds that builder's operating-system identity and
project scope. Its allowed-signers file is protected by the same ownership and
access checks, pinned by digest, and passed directly to the verifier; an
agent-writable temporary signer file is never substituted for it. Identity/key
entries must agree exactly with the enrolled role map. Tests may inject a trust reader into an isolated verifier unit;
the shipped command has no test override, environment bypass, or self-enroll
command. Actual host-enrollment acceptance is never claimed from that injection.

Canonical intent bytes begin with `ratmac-authorization-v1` and one NUL byte.
Each following field is its unsigned 64-bit big-endian byte length followed by
its UTF-8 bytes, in this fixed order: purpose, project scope, Run id, owning
class, workspace, binding digest, next authorization sequence, hold occurrence,
held-record digest, current State, target State, input,
blocker, Engine identity, runbook identity, goal identity, policy digest,
enrollment digest, builder actor, current declared-work digest, previous
amendment digest, proposed-authority digest, change-set digest, review-set
digest. Inapplicable fields are empty, never omitted; integers use canonical
unsigned decimal without leading zeros. Authorization sequences are Engine-owned,
monotonic, never reused, and bound to the addressed Run; resume also binds the
specific committed hold occurrence. Review-set digests cover the sorted distinct
reviewer identities and exact accepted signature bytes using the same length
framing. Approval cannot reuse a review-purpose signature. The Engine recomputes
the complete intent under the Run lock immediately before consumption.

Expose the canonical transition intent for an external reviewer to inspect
and sign. The scheduler derives it from authoritative Run state, the pinned
policy, the selected transition, and explicitly declared evidence roots.
Reading an intent is not approval. The signer writes a detached signature
outside Engine-owned state; the Engine verifies it against declared public
material immediately before transition consumption and mutation.

Route normal and blocked movement through the same verification boundary.
Commit accepted authorization evidence with the transition's existing
durability discipline. Refusal, interruption, duplicate submission, and replay
must leave a reconstructible pre-transition or completed state.

Use a test verifier only in isolated fixtures, with an explicit non-production
identity; live adoption requires an independently controlled signing identity.
Positive and negative acceptance checks also exercise the actual production
signature verifier with isolated, distinct fixture keys; a fake verifier is
insufficient evidence. Enrollment/configuration tampering, same-key aliases,
and recovery-policy replacement receive explicit negative controls.
The workflow's selected-issue completion change consumes this authority
contract and does not invent a separate approval format.

This file is incoming evidence. Integrated mechanics remain authoritative only
in the accepted forward authority.
