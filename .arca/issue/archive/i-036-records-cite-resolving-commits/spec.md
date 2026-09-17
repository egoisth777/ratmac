# Issue specification

Dispositions were confirmed at the 2026-08-24 planning pass (P1), signed by
Billy. The open carrier and reachability decisions were settled in the same
pass and are recorded in the goal design
([ADR-0019](../../../goal/design.md#the-citation-check-ships-in-the-qa-crate-reachability-is-any-ref-adr-0019)).

`RCR` is this issue's stable requirement-ID prefix — **Records Cite Resolving
commits** — defined in [ubi-lang.md](ubi-lang.md).

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `RCR-001` | A gap record's `implementation-revision` is written only in a landing that follows the green landing it cites: the green landing merges to `main` first, then a stamp landing flips the record `satisfied` and derives the hash from the repository at that moment — the commit the landed tip resolves to — never from memory or prediction. The P5 amend order is unchanged; the recording rule mirrors [ELR-001](../../../schema.md#elr-001---the-recording-landing-follows-the-tag): a commit cannot contain its own hash, so no rule may ask a landing to carry the citation of the commit it is. | accepted | `res-151` already cited `git:7fa79f6` inside the merged green landing `6cee4bb` itself, and `.arca/schema.md:265` mandates writing the record's bytes before the `git commit --amend` that finalizes the green landing — the prediction is structural, not a slip. | [goal spec](../../../goal/spec.md#integrated-records-cite-resolving-commits-requirements) |
| `RCR-002` | A machine check resolves every `git:` citation in every gap record's `implementation-revision` — active folder and archive counted as one namespace — and refuses a record whose cited commit is unreachable from the repository's refs; a hash that answers `git cat-file` only as a dangling object is exactly this refusal, not a pass. Archived records are byte-frozen, so citations judged under the pre-rule order are carried as an enumerated allowlist naming each record, its unresolvable hash, and the reason (the SVC-008 pattern); an allowlist row that matches nothing fails as stale. The refusal names the record and the hash. | accepted | The engine validates the `frozen-goal-bundle-revision` citation (`src/contract.rs:680-695`) and never reads `implementation-revision` anywhere in `src/`; the wish's own close — "the stamp is proved at closure instead of remembered" — is a check, and a scan at filing finds 11 of 153 records citing unreachable commits. | [goal spec](../../../goal/spec.md#integrated-records-cite-resolving-commits-requirements) |
| `RCR-003` | When the commit a citation names is rewritten or removed after the citation exists — amend, reset, checkpoint fold, branch delete — the citation is re-derived from the repository in the same change, together with the owning ticket's `landed-commit`; a re-stamp is a stamp step, never a hand fix from memory. The residual archive move refuses a record citing a commit that does not resolve and is not covered by the historical allowlist, so the pending re-stamp obligation (`.arca/schema.md:400-401`) gains a verifiable discharge. | accepted | Four verified hand corrections — `.arca/log.md:300` (t-071, four records plus the ticket), `:342` (t-080, `res-118`), `:344` (t-081, `res-119`), `:410` (`res-138`'s stale hash) — and two rot events no hand pass caught in two weeks (`git:0231def`, `git:0770778`): past the line where a repeated fix becomes a check. | [goal spec](../../../goal/spec.md#integrated-records-cite-resolving-commits-requirements) |

## Acceptance criteria

- The working rules state the stamp-landing order for gap records, and no
  live rule instructs a record to carry the hash of a commit it lands inside.
- The check refuses a record citing an unreachable commit, naming the record
  and the hash, on a fixture where the cited commit was orphaned by an amend.
- On this repository at integration, the check names exactly the rotted
  citations (11 at filing) before the allowlist and passes with the allowlist
  in place; every allowlist row matches a real record, and removing a
  matched row's record fails the row as stale.
- A `satisfied` record's citation and its ticket's `landed-commit` are
  re-derived by one stamp step after a landing is amended; the archive move
  refuses while either cites an unresolvable commit outside the allowlist.
