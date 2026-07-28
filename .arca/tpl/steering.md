# Steering

<!-- Template for .arca/steering.md. Fill every {{...}}; keep every heading and this
order — hardness increases top to bottom: authored identity, then forecast, then the
derived record. Full rules: .arca/schema.md, "Steering layers". -->

Read me before the goal bundle. This file is direction: what ratmac is for,
the bets behind it, and the lines no goal, issue, or ticket may cross. When
direction changes, this file changes **first**; `.arca/goal/`,
`.arca/issue/`, and `.arca/ticket/` re-align to it, in that order. It binds
contributors (people and agents) choosing what to build next; it never
overrides `.arca/goal/spec.md` on what the running program does.

## What we are building

<!-- Layer: authored identity. Clock: pivot-driven — changes only on a pivot and
lands first, before any dependent change in goal/issue/ticket. -->

{{what-the-program-is-in-plain-terms}}

## Thesis

<!-- Layer: authored identity. Clock: pivot-driven, steering-first. -->

- {{bet-behind-the-build}}

## Invariants

<!-- Layer: authored identity. Clock: pivot-driven, steering-first. These survive
every goal change; a goal/issue/ticket that needs to break one IS a direction change
and starts here. -->

1. {{invariant}}

## Non-goals

<!-- Layer: authored identity. Clock: pivot-driven, steering-first. -->

- {{what-this-deliberately-is-not}}

## Horizon

<!-- Layer: forecast. Clock: free revision, any time; natural moment is right after
each P1 close, once landings have re-priced the pool. Purpose: aims triage (which
wishes wishwillow specifies next), weighs leverage (which pool issues matter),
orientation. Binds nothing; nothing here is chosen; no work is ever cut straight from
this list — an item is chosen only by going through P1 like any other issue, never by
promoting a horizon item in place. Speak only in direction/wish/issue terms: no ticket
terms, nothing executable. -->

{{anticipated-direction-order-beyond-the-current-sprint}}

## Current sprint

<!-- Layer: derived. Clock: written at exactly one moment — P1 close — and wholesale
only; incremental hand-edits are forbidden (a hand-patched derived record is authored
narration in costume). Never written during P4-P5. Carries no stored progress or
status — stage derivation reads the tree (open tickets, gap records, log lines), not
this file. No clearing at sprint end; the next P1 replaces it wholesale. -->

Freeze: `{{goal-git-head}}` @ `{{p1-close-date}}`.

{{sprint-one-line-aim}}

<!-- Route: ordered dependency list of the signed sprint only — what depends on what,
one why per edge; never dates or task breakdowns. Every entry traces to an issue
accepted at this stamped P1. -->

Route - order is dependency, not preference.

0. {{issue-ref}} - {{why-it-precedes-what-follows}}

## How direction flows

<!-- Fixed reference map, not a fill-in: the per-artifact table (which question each
artifact answers, and when it changes) followed by the pivot order — steering, then
the goal bundle, then issue triage, then tickets. Same clock as authored identity
(pivot-driven, steering-first). This blank does not restate it: ownership rules for
Scheduler-owned artifacts are stated once, in .arca/schema.md; the routing table is
carried by .arca/index.md. Copy the canonical wording from the live steering.md when
filling this in. -->

{{direction-flow-map-and-pivot-order}}
