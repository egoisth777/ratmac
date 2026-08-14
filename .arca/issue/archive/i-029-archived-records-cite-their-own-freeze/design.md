# Issue design

## Proposed mechanics

The gate already walks two sets of paths - the active folder and the archive - and then
applies one predicate to both. The change is to carry the origin of each path into the
predicate: an active path is checked against the Run's frozen revision, an archived path is
checked for a well-formed revision citation and nothing more.

The citation is still required, and still parsed: a record with no revision, or an
unparseable one, is a defect wherever it lives. What is dropped is only the equality against
today's freeze for records the archive rule has frozen.

## The fixture is the real deliverable

The reason this survived so long is that every fixture repository is born at the current
freeze. A fixture that carries an archived record citing an older revision, and expects a
pass, is what keeps the class dead. Without it the code change is one line and the defect
returns the next time somebody tightens the predicate.

## Rejected: re-stamp the archived records

Rewriting 127 archived records to today's revision would make the gate pass and destroy the
provenance the archive exists to keep, and it would have to be repeated at every future
freeze. It is the reading the archive rule exists to forbid.

## Rejected: stop counting archived records

Counting only live records would also make the gate pass, and would reintroduce
satisfaction by absence: a requirement whose record was archived would look unmeasured.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted
forward authority.
