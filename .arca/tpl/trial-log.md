# Trial log: <trial-branch>

Fill every section. `tools/trial.ps1 finish` refuses a log with a missing
section, an empty section, an unfilled angle-bracket placeholder, or identity facts that
contradict the branch. Commit this file as `trial-log.md` at the root of the
trial worktree; finish copies it to `trials/<trial-branch>/trial-log.md` on the
experiment base, and that copy is the only trial content that outlives the trial.

## Identity

- trial: <trial-branch>
- base commit: <the commit the trial branched from>
- terminal commit: <the last commit of trial work>

## Hypothesis

<what this trial expected to be true, in one or two sentences>

## Procedure

<what was actually done, in order>

## Commands and tests

<the commands run and the tests that judged them>

## Observations

<what happened - numbers, failures, surprises>

## Verdict

<one line starting with adopt:, drop:, or inconclusive: - this line goes into the archive tag>

## Recommendations

<what the next loop should do with this result>

## Artifacts and diffs

<paths, commits, and diffs a reader needs to reconstruct the trial>
