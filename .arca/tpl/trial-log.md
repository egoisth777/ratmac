# Trial log: <trial-branch>

Fill every section. Keep feature behavior separate from RatMac's behavior:
the feature is the test load; RatMac's self-evaluation is the trial's primary
result. `tools/trial.ps1 finish` refuses a log with a missing section, an empty
section, an unfilled angle-bracket placeholder, or identity facts that
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

### Feature observations

<what happened to the feature - numbers, failures, surprises>

### RatMac observations

<what the workflow proved about RatMac - trustworthy gates, false claims, manual catches, and missing evidence>

## Verdict

<one feature-verdict line starting with adopt:, drop:, or inconclusive: - this line goes into the archive tag>

## Recommendations

### Feature route

<drop it, or describe the behavior that may re-enter normal P1-P5 development without copying trial bytes>

### RatMac route

<one actionable process recommendation per RatMac observation; these route to main before feature work>

## Artifacts and diffs

<paths, commits, and diffs a reader needs to reconstruct the trial>
