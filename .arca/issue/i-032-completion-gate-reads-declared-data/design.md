# Issue design

## Settled mechanism

This file records incoming design evidence. The accepted behavior is
authoritative in [goal `CGD-003`](../../goal/spec.md#integrated-completion-from-declared-data-requirements).
The mechanism is settled by [ADR-0022](../../goal/design.md#completion-fields-are-selected-by-typed-runbook-data-adr-0022),
while the exact grammar and diagnostics belong only to the
[runbook specification](../../runbook-spec.md#completion-declaration-mapping).
This summary does not redefine that schema or claim implementation proof.

**1. The declaration has three runbook-selected lists.** The current workflow
names its fields:

- `focused-tests` - focused receipt ids.
- `hidden-lanes` - hidden receipt ids.
- `quality-commands` - quality receipt commands.

Those are project data, not Rust vocabulary. The generic completion gate knows
the three receipt kinds - focused, hidden, and quality - and receives the
project-specific field selection from runbook data. Entries stay opaque and
match receipts verbatim. The gate consumes the resulting lists, not raw prose,
and preserves the focused-then-hidden-then-quality order. A malformed list, an
empty-string entry, or a duplicate refuses naming the field and entry before
any mutation.

The mapping reuses the guard's existing optional root and exact opaque
`ticket` or `ticket-binding` address: that address names the declaration
carrier itself, with no new path or suffix. The older `planned-test-refs`
field is deliberately not this project's focused selection. It remains part
of its earlier planning and sensitivity identity; archived records are not
rewritten, and equal planned/focused ids in a new fixture are deliberate rather
than inferred.

**2. Human-readable prose may stay but is not load-bearing.** Review sections
may continue to explain intended coverage. The gate does not split a Markdown
heading, harvest an `HT-`-shaped token, or scan backticks. Renaming or removing
headings therefore changes nothing, and a non-Markdown declaration carrier can
use the same completion contract.

**3. One narrow reader replaces the prose boundary.** The prose-discovery helpers
`hidden_lane_ids()`, `merge_gate_commands()`, and `backticked()` leave the
completion path, along with any raw-Markdown input used only by them. The
replacement is one generic selected-string-list reader extracted from the
proved workflow QA reader. QA supplies its local field names and the Engine
supplies the typed runbook mapping; no project field name lives in Engine code
and no YAML dependency is added. Its first-line/closing-fence region, selected
top-level fields, quoted block entries, explicit `[]`, and refusal rules are
the narrow subset defined by the runbook specification, not general YAML.
Unrelated fields and body prose carry no completion meaning.

**4. What is deliberately unchanged.**

- Receipt kinds and format, Run-keyed evidence paths, target binding,
  freshness and self-consistency checks, green/missing/stray/duplicate
  safeguards, and receipt-defect wording.
- The sensitivity gate (`PGE-003`).
- Archived tickets and other historical bytes.

## Settled empty-list behavior

The landed `CGD-001`/`CGD-002` workflow already proves the compatible meaning:
an explicit empty list is legal and means no checks of that kind. The Engine
cutover keeps that meaning. When the selected declaration fields are absent,
the existing `declares no checks` refusal remains; when every explicit list is
empty, the same refusal prevents a vacuous pass. This is the safest compatible
default because one explicitly empty check kind stays valid without allowing
completion to prove nothing.

## Settled self-host boundary

An omitted mapping remains parse-compatible, but after the existing
paused-Run check a new Engine refuses it before reading the declaration and
never falls back to prose. It therefore cannot drive the current unmapped
runbook through completion.

The capability landing tracks an exact `.ratmac/completion-guard.diff` for
the two completion guards and proves apply/reverse byte identity while leaving
the real Machine Class unmapped. Its temporary traversal uses the patched copy
and calls itself the prepared cutover, not the exact shipped runbook. The
fixture ticket gains its focused, hidden-lane, and quality tags in that same
landing. Separate proofs show the candidate fails closed on the tracked
unmapped guard before reading its artifact, while doctor adds no
missing-mapping lint. The `RB113` Diagnostics and authoring-repair rows travel
with the source that first emits `RB113`.

Stable `edition-007` drives the unchanged Machine Class and `run-030` to
rest without changing its pin evidence. The post-rest recording/activation
landing applies the tested patch, restores the exact-shipped-byte traversal,
and removes the temporary patch path and unmapped assertion. Roots, States,
transitions, edition and sweep guards stay unchanged, and no new Engine
behavior lands there. Its edition ledger claims only the capability at the
tag, not repository wiring; the new stable is bootstrapped before another Run.
The activation and test-source switch remain in the implementing ticket's
scope, with any required direct-primary conflict resolution recorded in the
log. ADR-0022 owns the full rollout contract.
