# Operating the Plan-Build profile

This folder is the Plan-Build profile: one distributable bundle holding the
Machine Class template, machine-readable record schemas, packaged checkers,
issue/gap/work-item blanks, the required terms, and these instructions. A
clean repository - a Git repository with no `.arca/schema.md`, no `.ratmac/`,
and no inherited agent prompt - initializes from this file alone and drives
a complete issue-to-proven-completion sprint using only the bundle's data
and the Engine's own output.

Initialization is file placement the operator owns: the Engine has no verb
for it, installs no global configuration, and installs no dependencies.
Everything current is reached by running the Engine and reading its output,
never by copying what it printed.

## Prerequisites

Two external programs, nothing else: `git`, and `python` 3.11 or newer (for
the `tomllib` module the packaged checkers read). No network access, no
installs, no fetches: the profile is deterministic and offline.

## Initialization

1. Export the bundle into the target repository, or place it there:

   `rtm profile plan-build <path>`

   The export writes exactly one folder at a path that does not exist yet,
   validates the packaged manifest before the first byte, never overwrites,
   and creates no directories outside the one target folder. As shipped the
   bundle installs at `profile/` at the repository root - the path the
   guard rows below name.

2. Choose the roots. Open `profile/machine-class.toml` and edit its
   `[roots]` table to the paths your repository wants for the goal, issue,
   residual, and ticket roots, and the working authority. Then copy the
   edited file to `.ratmac/ratmac.toml`; if a runbook already lives there,
   moving it aside is your call - the Engine never overwrites it. If you
   install the bundle somewhere other than `profile/`, also edit the guard
   rows that name `profile/checkers/verify_profile.py` and
   `profile/checkers/verify_issue_binding.py` to point at the installed
   bundle path.

3. Create the declared roots from the blanks. Copy
   `profile/blanks/residual.md` into the residual root, `profile/blanks/ticket.md`
   into the ticket root, and the five files under `profile/blanks/issue/`
   into one folder per issue beneath the issue root, filling each
   `{{placeholder}}`. The record shapes are declared data: read the
   matching file under `profile/schemas/`, and the words in
   `profile/terms.md`.

4. Self-check the installed bundle before the first Run:

   `python profile/checkers/verify_profile.py check`

   One line on success - `profile verified: plan-build (format 1, 16
   artifacts)` - and one actionable refusal line on any missing, malformed,
   or incompatible artifact.

5. Orient, then run the cycle:

   `rtm status`

   The roster answers. `rtm start` admits the first Run at `intake`;
   `rtm step --run <id>` advances it through the cycle; `rtm spawn turn
   --run <parent> --bind item=<item> --bind issue=<issue>` begins each work
   item's turn. Every refusal names its repair.

## The closing mark

A sprint closes on an edition: an annotated Git tag named `edition-NNN`,
cut with plain `git tag -a edition-NNN -m ...` at the commit the sprint
leaves. The close guard proves it with `git describe --exact-match --match
edition-*`. An edition is never moved or overwritten: sequential, and each
sprint starts from the one the last sprint left.

## Ownership boundaries

The Engine writes the Engine root (`.ratmac/runs/`, `.ratmac/locks/`,
`.ratmac/mint.toml`, `.ratmac/log.md`) and nothing else; you own every
other file, this bundle included. The packaged checkers are read-only: they
write nothing anywhere. The bundle stays ordinary tracked shop content.
