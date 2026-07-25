#!/usr/bin/env pwsh
#Requires -Version 7

<#
.SYNOPSIS
    TWL-001..TWL-003, TWL-006, TWL-009: the repo-local trial lifecycle.

.DESCRIPTION
    A trial is an experiment that is free to fail: it lives on its own branch
    and linked worktree opened from the experiment base, and it is contained,
    numbered, and reversible.

    Verbs available today:

        pwsh -File tools/trial.ps1 status [-Slug <slug>]
        pwsh -File tools/trial.ps1 start  -Slug <slug> [-Number <n>]

    `status` is the dry-run: it prints the base and its tip, cleanliness, live
    and archived trials, the next inferred identity, and the planned mutations
    and recovery commands of every mutating verb. It changes nothing.

    `start` opens a trial or refuses. Every precondition is checked before the
    first Git write; if creation fails midway the new branch ref is
    compare-and-deleted, and a rollback that cannot finish prints the exact
    manual recovery commands rather than guessing.

    Ownership: a human or the Main-Agent runs these verbs from the primary
    checkout with the experiment base checked out. Subagents run neither these
    verbs nor `rtm`. On Windows a shell whose working directory is inside a
    trial worktree keeps a handle on it, so mutating verbs refuse from there
    and print the `cd` that fixes it.

    The experiment base is fixed at `exp/ratmac-deterministic` (TWL-001). It is
    deliberately not a parameter: there is no way to open a trial from another
    branch.

    Nothing here reaches the network, installs anything, or edits global Git
    configuration: plain Git and built-in cmdlets only.
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string] $Verb = 'status',

    [string] $Slug = '',

    [int] $Number = 0
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:SlugPattern = '^[a-z0-9]+(-[a-z0-9]+)*$'

# TWL-001: the one base a trial may start from.
$Base = 'exp/ratmac-deterministic'

# A number counts as given only when the caller typed it: 0 is the unset default.
$script:NumberGiven = $PSBoundParameters.ContainsKey('Number')

function Write-Report {
    param([string] $Text = '')
    [Console]::Out.WriteLine($Text)
}

# One named reason plus guidance, on stderr, with a non-zero exit.
function Deny {
    param(
        [Parameter(Mandatory)][string] $Reason,
        [string[]] $Guidance = @()
    )
    [Console]::Error.WriteLine("trial refused; $Reason")
    foreach ($line in $Guidance) {
        [Console]::Error.WriteLine("  $line")
    }
    exit 1
}

function Invoke-Git {
    param([Parameter(Mandatory)][string[]] $Arguments)
    $standardOutput = & git @Arguments 2>&1
    $exitCode = $LASTEXITCODE
    $text = if ($null -eq $standardOutput) { '' } else { ($standardOutput | Out-String) }
    [pscustomobject]@{
        ExitCode = $exitCode
        Text     = $text.TrimEnd("`r", "`n")
        Ok       = ($exitCode -eq 0)
    }
}

function Get-GitText {
    param([Parameter(Mandatory)][string[]] $Arguments)
    $result = Invoke-Git -Arguments $Arguments
    if (-not $result.Ok) {
        Deny -Reason "git $($Arguments -join ' ') failed: $($result.Text)"
    }
    $result.Text
}

function Get-GitLines {
    param([Parameter(Mandatory)][string[]] $Arguments)
    $text = Get-GitText -Arguments $Arguments
    if ([string]::IsNullOrWhiteSpace($text)) { return @() }
    @($text -split "`r?`n" | Where-Object { $_ -ne '' })
}

function Resolve-PathText {
    param([Parameter(Mandatory)][string] $Path)
    ($Path -replace '\\', '/').TrimEnd('/')
}

# Everything the verbs decide from, read once, mutating nothing.
function Get-Facts {
    $inside = Invoke-Git -Arguments @('rev-parse', '--is-inside-work-tree')
    if (-not $inside.Ok) {
        Deny -Reason 'this directory is not a Git repository' -Guidance @(
            'cd to the repository root and run the verb again.'
        )
    }

    $repoRoot = Resolve-PathText (Get-GitText -Arguments @('rev-parse', '--show-toplevel'))
    $gitDir = (Resolve-Path (Get-GitText -Arguments @('rev-parse', '--absolute-git-dir'))).Path
    $commonDirText = Get-GitText -Arguments @('rev-parse', '--git-common-dir')
    if (-not [System.IO.Path]::IsPathRooted($commonDirText)) {
        $commonDirText = Join-Path $repoRoot $commonDirText
    }
    $commonDir = (Resolve-Path $commonDirText).Path

    $registrations = Get-GitLines -Arguments @('worktree', 'list', '--porcelain')
    $primary = $repoRoot
    foreach ($line in $registrations) {
        if ($line.StartsWith('worktree ')) {
            $primary = Resolve-PathText $line.Substring('worktree '.Length)
            break
        }
    }

    $liveTrials = @(Get-GitLines -Arguments @(
            'for-each-ref', '--format=%(refname:short)', 'refs/heads/trial-*'))
    $archivedTrials = @(Get-GitLines -Arguments @(
            'for-each-ref', '--format=%(refname:short)', 'refs/tags/trial-archive/*'))

    $durableRoot = Join-Path $repoRoot 'trials'
    $durableTrials = @()
    if (Test-Path -LiteralPath $durableRoot -PathType Container) {
        $durableTrials = @(Get-ChildItem -LiteralPath $durableRoot -Directory |
                Where-Object { $_.Name -like 'trial-*' } |
                ForEach-Object { $_.Name })
    }

    $baseRef = "refs/heads/$Base"
    $baseTipResult = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', $baseRef)
    $baseTip = if ($baseTipResult.Ok) { $baseTipResult.Text.Trim() } else { '' }

    [pscustomobject]@{
        RepoRoot        = $repoRoot
        RepoName        = Split-Path -Leaf $repoRoot
        Parent          = Resolve-PathText (Split-Path -Parent $repoRoot)
        Primary         = $primary
        InLinkedTree    = ($gitDir -ne $commonDir)
        CurrentBranch   = (Get-GitText -Arguments @('rev-parse', '--abbrev-ref', 'HEAD')).Trim()
        Porcelain       = @(Get-GitLines -Arguments @('status', '--porcelain'))
        BaseRef         = $baseRef
        BaseTip         = $baseTip
        LiveTrials      = $liveTrials
        ArchivedTrials  = $archivedTrials
        DurableTrials   = $durableTrials
        Registrations   = $registrations
    }
}

function Get-TrialNumbers {
    param([Parameter(Mandatory)] $Facts)
    $numbers = @()
    foreach ($name in @($Facts.LiveTrials) + @($Facts.DurableTrials)) {
        if ($name -match '^trial-(\d+)-') { $numbers += [int]$Matches[1] }
    }
    foreach ($tag in @($Facts.ArchivedTrials)) {
        if ($tag -match '^trial-archive/trial-(\d+)-') { $numbers += [int]$Matches[1] }
    }
    $numbers
}

# TWL-002: identity is computed before any mutation.
function Get-Identity {
    param(
        [Parameter(Mandatory)] $Facts,
        [string] $SlugText,
        [int] $Explicit
    )
    $numbers = @(Get-TrialNumbers -Facts $Facts)
    $next = if ($numbers.Count -eq 0) { 1 } else { ([int]($numbers | Measure-Object -Maximum).Maximum) + 1 }
    $chosen = if ($Explicit -gt 0) { $Explicit } else { $next }
    $padded = '{0:d3}' -f $chosen
    $slugText = if ([string]::IsNullOrEmpty($SlugText)) { '<slug>' } else { $SlugText }
    $branch = "trial-$padded-$slugText"

    [pscustomobject]@{
        Number        = $chosen
        Padded        = $padded
        Slug          = $slugText
        Branch        = $branch
        WorktreePath  = "$($Facts.Parent)/$($Facts.RepoName)-$branch"
        ArchiveTag    = "trial-archive/$branch"
        DurableLog    = "trials/$branch/trial-log.md"
        Inferred      = $next
    }
}

function Test-RefExists {
    param([Parameter(Mandatory)][string] $Ref)
    (Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', $Ref)).Ok
}

# TWL-001: every collision is named before anything is written.
function Get-Collisions {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Identity
    )
    $collisions = @()
    if (Test-RefExists -Ref "refs/heads/$($Identity.Branch)") {
        $collisions += "branch $($Identity.Branch) already exists"
    }
    foreach ($name in @($Facts.LiveTrials)) {
        if ($name -match "^trial-$($Identity.Padded)-" -and $name -ne $Identity.Branch) {
            $collisions += "trial number $($Identity.Padded) is taken by branch $name"
        }
    }
    foreach ($tag in @($Facts.ArchivedTrials)) {
        if ($tag -match "^trial-archive/trial-$($Identity.Padded)-") {
            $collisions += "trial number $($Identity.Padded) is taken by archive tag $tag"
        }
    }
    foreach ($durable in @($Facts.DurableTrials)) {
        if ($durable -match "^trial-$($Identity.Padded)-") {
            $collisions += "trial number $($Identity.Padded) is taken by durable log trials/$durable"
        }
    }
    if (Test-Path -LiteralPath $Identity.WorktreePath) {
        $collisions += "the sibling path $($Identity.WorktreePath) already exists"
    }
    foreach ($line in @($Facts.Registrations)) {
        if ($line.StartsWith('worktree ')) {
            $registered = Resolve-PathText $line.Substring('worktree '.Length)
            if ($registered -eq $Identity.WorktreePath) {
                $collisions += "a worktree is already registered at $($Identity.WorktreePath)"
            }
        }
    }
    $collisions
}

function Get-StartPlan {
    param([Parameter(Mandatory)] $Identity, [Parameter(Mandatory)] $Facts)
    @(
        "git worktree add -b $($Identity.Branch) $($Identity.WorktreePath) $Base"
    )
}

function Get-StartRecovery {
    param([Parameter(Mandatory)] $Identity)
    @(
        "git worktree remove $($Identity.WorktreePath)",
        "git update-ref -d refs/heads/$($Identity.Branch)"
    )
}

function Get-FinishPlan {
    param([Parameter(Mandatory)] $Identity)
    @(
        "git tag -a $($Identity.ArchiveTag) <terminal-commit> -m '<verdict>'",
        "git add $($Identity.DurableLog) && git commit -m 'trial($($Identity.Branch)): archive durable log'",
        "git worktree remove $($Identity.WorktreePath)",
        "git update-ref -d refs/heads/$($Identity.Branch) <terminal-commit>"
    )
}

function Get-FinishRecovery {
    param([Parameter(Mandatory)] $Identity)
    @(
        "git branch $($Identity.Branch) $($Identity.ArchiveTag)",
        "git worktree add $($Identity.WorktreePath) $($Identity.Branch)"
    )
}

function Get-SyncPlan {
    @('git merge main')
}

function Get-SyncRecovery {
    @(
        'resolve the conflicted files listed by git status, then: git commit',
        'a conflicted merge is left visible; nothing is auto-resolved'
    )
}

function Invoke-Status {
    param([Parameter(Mandatory)] $Facts)

    if ([string]::IsNullOrEmpty($Facts.BaseTip)) {
        Deny -Reason "the experiment base $Base does not exist in this repository" -Guidance @(
            "create the branch $Base from the commit the experiments start at."
        )
    }
    if ($script:NumberGiven -and $Number -lt 1) {
        Deny -Reason "the trial number '$Number' is malformed" -Guidance @(
            'a trial number is a positive integer; omit -Number to take the next free one.'
        )
    }
    if (-not [string]::IsNullOrEmpty($Slug) -and $Slug -cnotmatch $script:SlugPattern) {
        Deny -Reason "the topic slug '$Slug' is malformed" -Guidance @(
            'a slug is lowercase words joined by single dashes: [a-z0-9]+(-[a-z0-9]+)*'
        )
    }

    $identity = Get-Identity -Facts $Facts -SlugText $Slug -Explicit $Number
    $clean = if ($Facts.Porcelain.Count -eq 0) { 'clean' } else { "dirty ($($Facts.Porcelain.Count) entries)" }

    Write-Report "trial status (read-only; nothing below has been applied)"
    Write-Report ""
    Write-Report "experiment base:  $Base"
    Write-Report "base tip:         $($Facts.BaseTip)"
    Write-Report "checked out:      $($Facts.CurrentBranch)"
    Write-Report "working tree:     $clean"
    Write-Report "primary checkout: $($Facts.Primary)"
    Write-Report ""

    Write-Report "live trials:"
    if ($Facts.LiveTrials.Count -eq 0) { Write-Report "  (none)" }
    foreach ($name in @($Facts.LiveTrials)) { Write-Report "  $name" }
    Write-Report "archived trials:"
    if ($Facts.ArchivedTrials.Count -eq 0) { Write-Report "  (none)" }
    foreach ($tag in @($Facts.ArchivedTrials)) { Write-Report "  $tag" }
    Write-Report "durable logs:"
    if ($Facts.DurableTrials.Count -eq 0) { Write-Report "  (none)" }
    foreach ($durable in @($Facts.DurableTrials)) { Write-Report "  trials/$durable/trial-log.md" }
    Write-Report ""

    # TWL-006: what finish would do with each live trial, and where it stands.
    foreach ($name in @($Facts.LiveTrials)) {
        $live = Get-LiveIdentity -Facts $Facts -BranchName $name
        $liveState = Get-TrialState -Facts $Facts -Identity $live
        $liveDefects = @(Get-LogDefects -Identity $live -State $liveState)
        Write-Report "trial $($name):"
        if (-not $liveState.LogPresent) {
            Write-Report "  log: missing - commit trial-log.md on $name (template: .arca/tpl/trial-log.md)"
        }
        elseif ($liveDefects.Count -eq 0) {
            Write-Report "  log: valid"
        }
        else {
            Write-Report "  log: invalid - $($liveDefects[0])"
            foreach ($defect in @($liveDefects | Select-Object -Skip 1)) { Write-Report "        $defect" }
        }
        $worktreeState = if (-not $liveState.WorktreeExists) { 'missing' }
            elseif ($liveState.WorktreePorcelain.Count -gt 0) { "dirty ($($liveState.WorktreePorcelain.Count) entries)" }
            else { 'clean' }
        Write-Report "  worktree: $worktreeState at $($liveState.WorktreePath)"
        if ($liveState.TagExists) {
            Write-Report "  archive tag: done (at $($liveState.TagTarget))"
        }
        else {
            Write-Report "  archive tag: pending"
        }
        if ($liveState.DurableCommitted) {
            Write-Report "  durable log: done ($($live.DurableLog))"
        }
        else {
            Write-Report "  durable log: pending ($($live.DurableLog))"
        }
        if ($liveState.TagExists -or $liveState.DurableCommitted) {
            Write-Report "  a finish stopped partway; resume it - the steps already done are skipped:"
        }
        Write-Report "  resume: pwsh -File tools/trial.ps1 finish -Slug $($live.Slug)"
        Write-Report ""
    }

    Write-Report "next identity (number $($identity.Padded), inferred next is $('{0:d3}' -f $identity.Inferred)):"
    Write-Report "  branch:          $($identity.Branch)"
    Write-Report "  worktree path:   $($identity.WorktreePath)"
    Write-Report "  archive tag:     $($identity.ArchiveTag)"
    Write-Report "  durable log:     $($identity.DurableLog)"
    if ($identity.Slug -eq '<slug>') {
        Write-Report "  (pass -Slug <topic> to see the concrete names)"
    }
    Write-Report ""

    $collisions = @(Get-Collisions -Facts $Facts -Identity $identity)
    Write-Report "start plan:"
    foreach ($line in Get-StartPlan -Identity $identity -Facts $Facts) { Write-Report "  $line" }
    Write-Report "  recovery if it fails midway:"
    foreach ($line in Get-StartRecovery -Identity $identity) { Write-Report "    $line" }
    if ($collisions.Count -gt 0) {
        Write-Report "  blocked by:"
        foreach ($line in $collisions) { Write-Report "    $line" }
    }
    if ($Facts.Porcelain.Count -gt 0) {
        Write-Report "  blocked by: the working tree is not clean"
    }
    Write-Report ""

    Write-Report "finish plan (in this order; a failing step refuses it and every later step):"
    foreach ($line in Get-FinishPlan -Identity $identity) { Write-Report "  $line" }
    Write-Report "  recovery from the archive tag:"
    foreach ($line in Get-FinishRecovery -Identity $identity) { Write-Report "    $line" }
    Write-Report ""

    Write-Report "sync plan (base receives main, merge only):"
    foreach ($line in Get-SyncPlan) { Write-Report "  $line" }
    Write-Report "  recovery:"
    foreach ($line in Get-SyncRecovery) { Write-Report "    $line" }
    Write-Report ""
    exit 0
}

# TWL-003: create the trial or leave the repository exactly as it was.
function Invoke-Start {
    param([Parameter(Mandatory)] $Facts)

    if ($Facts.InLinkedTree) {
        Deny -Reason "start must run from the primary checkout, not from inside a linked trial worktree" -Guidance @(
            "cd $($Facts.Primary)",
            'then run the same command again.'
        )
    }
    if ([string]::IsNullOrEmpty($Slug)) {
        Deny -Reason 'start needs a topic slug' -Guidance @(
            'pwsh -File tools/trial.ps1 start -Slug <topic>',
            'a slug is lowercase words joined by single dashes: [a-z0-9]+(-[a-z0-9]+)*'
        )
    }
    if ($script:NumberGiven -and $Number -lt 1) {
        Deny -Reason "the trial number '$Number' is malformed" -Guidance @(
            'a trial number is a positive integer; omit -Number to take the next free one.'
        )
    }
    if ($Slug -cnotmatch $script:SlugPattern) {
        Deny -Reason "the topic slug '$Slug' is malformed" -Guidance @(
            'a slug is lowercase words joined by single dashes: [a-z0-9]+(-[a-z0-9]+)*'
        )
    }
    if ([string]::IsNullOrEmpty($Facts.BaseTip)) {
        Deny -Reason "the experiment base $Base does not exist in this repository" -Guidance @(
            "create the branch $Base from the commit the experiments start at."
        )
    }
    if ($Facts.CurrentBranch -ne $Base) {
        Deny -Reason "a trial starts only from the experiment base $Base, but $($Facts.CurrentBranch) is checked out" -Guidance @(
            "git checkout $Base"
        )
    }
    if ($Facts.Porcelain.Count -gt 0) {
        $entries = @('commit or put aside these entries first:') +
            @($Facts.Porcelain | ForEach-Object { "  $_" })
        Deny -Reason "the experiment base is not clean: a trial starts only from a clean committed tip" -Guidance $entries
    }

    $identity = Get-Identity -Facts $Facts -SlugText $Slug -Explicit $Number
    $collisions = @(Get-Collisions -Facts $Facts -Identity $identity)
    if ($collisions.Count -gt 0) {
        Deny -Reason "trial identity $($identity.Branch) collides with existing work" -Guidance (
            @($collisions) + @('pass -Number <n> for a free number, or choose another slug.')
        )
    }

    $baseTip = $Facts.BaseTip
    $creation = Invoke-Git -Arguments @(
        'worktree', 'add', '-b', $identity.Branch, $identity.WorktreePath, $Base)

    if (-not $creation.Ok) {
        Undo-Start -Identity $identity -BaseTip $baseTip -Failure $creation.Text
    }

    # Post-verification: the trial exists exactly as planned, or it is undone.
    $created = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "refs/heads/$($identity.Branch)")
    $registered = @(@(Get-GitLines -Arguments @('worktree', 'list', '--porcelain')) |
        Where-Object { $_ -eq "branch refs/heads/$($identity.Branch)" })
    if (-not $created.Ok -or $created.Text.Trim() -ne $baseTip -or $registered.Count -eq 0) {
        Undo-Start -Identity $identity -BaseTip $baseTip -Failure 'the created trial does not match the plan'
    }

    Write-Report "trial started"
    Write-Report "  branch:        $($identity.Branch) at $baseTip"
    Write-Report "  worktree:      $($identity.WorktreePath)"
    Write-Report "  archive tag:   $($identity.ArchiveTag) (created by finish)"
    Write-Report "  durable log:   $($identity.DurableLog) (the only file a finish adds to $Base)"
    Write-Report ""
    Write-Report "next: cd $($identity.WorktreePath)"
    Write-Report "undo: git worktree remove $($identity.WorktreePath) && git update-ref -d refs/heads/$($identity.Branch) $baseTip"
    exit 0
}

# Roll the failed creation back, or say exactly how to finish it by hand.
function Undo-Start {
    param(
        [Parameter(Mandatory)] $Identity,
        [Parameter(Mandatory)][string] $BaseTip,
        [Parameter(Mandatory)][string] $Failure
    )
    $unrecovered = @()

    $registered = @(@(Get-GitLines -Arguments @('worktree', 'list', '--porcelain')) |
        Where-Object { $_ -eq "worktree $($Identity.WorktreePath)" })
    if ($registered.Count -gt 0) {
        $removal = Invoke-Git -Arguments @('worktree', 'remove', $Identity.WorktreePath)
        if (-not $removal.Ok) {
            $unrecovered += "git worktree remove $($Identity.WorktreePath)"
        }
    }

    # Compare-and-delete: the ref goes only while it still points at the base
    # tip this start recorded, so concurrent work is never discarded.
    $current = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "refs/heads/$($Identity.Branch)")
    if ($current.Ok) {
        if ($current.Text.Trim() -eq $BaseTip) {
            $deletion = Invoke-Git -Arguments @(
                'update-ref', '-d', "refs/heads/$($Identity.Branch)", $BaseTip)
            if (-not $deletion.Ok) {
                $unrecovered += "git update-ref -d refs/heads/$($Identity.Branch) $BaseTip"
            }
        }
        else {
            $unrecovered += "refs/heads/$($Identity.Branch) moved to $($current.Text.Trim()); it was left alone - inspect it before deleting"
        }
    }

    if (Test-Path -LiteralPath $Identity.WorktreePath) {
        $unrecovered += "the sibling path $($Identity.WorktreePath) still exists; remove it by hand after checking what is in it"
    }

    if ($unrecovered.Count -eq 0) {
        Deny -Reason "creating the trial worktree failed: $Failure" -Guidance @(
            'nothing persists: the new branch ref was removed and no worktree was registered.',
            'fix the reported cause and run start again.'
        )
    }

    Deny -Reason "creating the trial worktree failed: $Failure" -Guidance (
        @('rollback could not finish; run these by hand, in order:') + $unrecovered
    )
}

# TWL-004: the sections a trial log must carry, each non-empty.
$script:RequiredSections = @(
    'Identity',
    'Hypothesis',
    'Procedure',
    'Commands and tests',
    'Observations',
    'Verdict',
    'Recommendations',
    'Artifacts and diffs'
)

# The identity of a trial that already exists, read back from its branch name.
function Get-LiveIdentity {
    param([Parameter(Mandatory)] $Facts, [Parameter(Mandatory)][string] $BranchName)
    if ($BranchName -cnotmatch '^trial-(\d{3,})-(.+)$') {
        Deny -Reason "the branch $BranchName is not a trial branch" -Guidance @(
            'a trial branch is trial-<nnn>-<slug>.'
        )
    }
    [pscustomobject]@{
        Number       = [int]$Matches[1]
        Padded       = $Matches[1]
        Slug         = $Matches[2]
        Branch       = $BranchName
        WorktreePath = "$($Facts.Parent)/$($Facts.RepoName)-$BranchName"
        ArchiveTag   = "trial-archive/$BranchName"
        DurableLog   = "trials/$BranchName/trial-log.md"
        Inferred     = 0
    }
}

# TWL-005: which trial a mutating verb acts on is never guessed.
function Resolve-Trial {
    param([Parameter(Mandatory)] $Facts)
    $candidates = @($Facts.LiveTrials)
    if (-not [string]::IsNullOrEmpty($Slug)) {
        $candidates = @($candidates | Where-Object { $_ -cmatch "^trial-\d{3,}-$([regex]::Escape($Slug))$" })
    }
    if ($script:NumberGiven) {
        $wanted = '{0:d3}' -f $Number
        $candidates = @($candidates | Where-Object { $_ -match "^trial-0*$wanted-" })
    }
    if ($candidates.Count -eq 0) {
        Deny -Reason 'no live trial matches this request' -Guidance (
            @('live trials:') + @(if ($Facts.LiveTrials.Count -eq 0) { '  (none)' } else { $Facts.LiveTrials | ForEach-Object { "  $_" } })
        )
    }
    if ($candidates.Count -gt 1) {
        Deny -Reason 'more than one live trial matches this request' -Guidance (
            @('name one with -Slug <topic> or -Number <n>:') + @($candidates | ForEach-Object { "  $_" })
        )
    }
    Get-LiveIdentity -Facts $Facts -BranchName $candidates[0]
}

# Everything finish decides from, read without mutating anything.
function Get-TrialState {
    param([Parameter(Mandatory)] $Facts, [Parameter(Mandatory)] $Identity)

    $terminal = ''
    $tip = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "refs/heads/$($Identity.Branch)")
    if ($tip.Ok) { $terminal = $tip.Text.Trim() }

    $forkPoint = ''
    if ($terminal -and $Facts.BaseTip) {
        $merged = Invoke-Git -Arguments @('merge-base', $Base, $Identity.Branch)
        if ($merged.Ok) { $forkPoint = $merged.Text.Trim() }
    }

    $registered = $false
    $currentWorktree = ''
    for ($index = 0; $index -lt $Facts.Registrations.Count; $index++) {
        $line = $Facts.Registrations[$index]
        if ($line -eq "branch refs/heads/$($Identity.Branch)") {
            $registered = $true
            for ($back = $index; $back -ge 0; $back--) {
                if ($Facts.Registrations[$back].StartsWith('worktree ')) {
                    $currentWorktree = Resolve-PathText $Facts.Registrations[$back].Substring('worktree '.Length)
                    break
                }
            }
            break
        }
    }
    $worktreePath = if ($currentWorktree) { $currentWorktree } else { $Identity.WorktreePath }
    $worktreeExists = Test-Path -LiteralPath $worktreePath -PathType Container
    $worktreePorcelain = @()
    if ($worktreeExists) {
        $status = Invoke-Git -Arguments @('-C', $worktreePath, 'status', '--porcelain')
        if ($status.Ok -and -not [string]::IsNullOrWhiteSpace($status.Text)) {
            $worktreePorcelain = @($status.Text -split "`r?`n" | Where-Object { $_ -ne '' })
        }
    }

    $logResult = Invoke-Git -Arguments @('show', "$($Identity.Branch):trial-log.md")
    $logText = if ($logResult.Ok) { ($logResult.Text -replace "`r`n", "`n") } else { '' }

    $tagExists = (Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "refs/tags/$($Identity.ArchiveTag)")).Ok
    $tagTarget = ''
    $tagAnnotated = $false
    if ($tagExists) {
        $target = Invoke-Git -Arguments @('rev-parse', "$($Identity.ArchiveTag)^{commit}")
        if ($target.Ok) { $tagTarget = $target.Text.Trim() }
        $kind = Invoke-Git -Arguments @('cat-file', '-t', $Identity.ArchiveTag)
        $tagAnnotated = ($kind.Ok -and $kind.Text.Trim() -eq 'tag')
    }

    $durablePath = Join-Path $Facts.RepoRoot $Identity.DurableLog
    $durableCommitted = $false
    if (Test-Path -LiteralPath $durablePath -PathType Leaf) {
        $recorded = Invoke-Git -Arguments @('log', '-1', '--format=%H', '--', $Identity.DurableLog)
        $durableCommitted = ($recorded.Ok -and -not [string]::IsNullOrWhiteSpace($recorded.Text))
    }

    [pscustomobject]@{
        Terminal          = $terminal
        ForkPoint         = $forkPoint
        Registered        = $registered
        WorktreePath      = $worktreePath
        WorktreeExists    = $worktreeExists
        WorktreePorcelain = $worktreePorcelain
        LogPresent        = $logResult.Ok
        LogText           = $logText
        TagExists         = $tagExists
        TagTarget         = $tagTarget
        TagAnnotated      = $tagAnnotated
        DurablePath       = $durablePath
        DurableCommitted  = $durableCommitted
    }
}

# TWL-004: validity is mechanical - sections present and filled, identity true.
function Get-LogDefects {
    param([Parameter(Mandatory)] $Identity, [Parameter(Mandatory)] $State)
    $defects = @()
    if (-not $State.LogPresent) {
        return @("trial-log.md is not committed on $($Identity.Branch)")
    }

    $lines = @($State.LogText -split "`n")
    $bodies = @{}
    $current = ''
    foreach ($line in $lines) {
        if ($line -cmatch '^##\s+(.+?)\s*$') {
            $current = $Matches[1]
            if (-not $bodies.ContainsKey($current)) { $bodies[$current] = @() }
            continue
        }
        if ($current) { $bodies[$current] += $line }
    }

    foreach ($section in $script:RequiredSections) {
        if (-not $bodies.ContainsKey($section)) {
            $defects += "missing section: ## $section"
            continue
        }
        $filled = @($bodies[$section] | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
        if ($filled.Count -eq 0) {
            $defects += "empty section: ## $section"
            continue
        }
        # An untouched template line is not an observation.
        $placeholders = @($filled | Where-Object { $_ -match '<[^>]+>' })
        if ($placeholders.Count -gt 0) {
            $defects += "unfilled placeholder in section ## $($section): $($placeholders[0].Trim())"
        }
    }

    if ($bodies.ContainsKey('Identity')) {
        $identityText = ($bodies['Identity'] -join "`n")
        $declaredBranch = if ($identityText -cmatch '(?m)^\s*-\s*trial:\s*(\S+)\s*$') { $Matches[1] } else { '' }
        $declaredBase = if ($identityText -cmatch '(?m)^\s*-\s*base commit:\s*([0-9a-f]{7,40})\s*$') { $Matches[1] } else { '' }
        $declaredTerminal = if ($identityText -cmatch '(?m)^\s*-\s*terminal commit:\s*([0-9a-f]{7,40})\s*$') { $Matches[1] } else { '' }

        if ($declaredBranch -ne $Identity.Branch) {
            $defects += "identity mismatch: the log says trial: $declaredBranch but this is $($Identity.Branch)"
        }
        if (-not $declaredBase) {
            $defects += 'identity mismatch: the log states no base commit'
        }
        elseif (-not $State.ForkPoint.StartsWith($declaredBase)) {
            $defects += "identity mismatch: the log says base commit: $declaredBase but $($Identity.Branch) forked at $($State.ForkPoint)"
        }
        if (-not $declaredTerminal) {
            $defects += 'identity mismatch: the log states no terminal commit'
        }
        else {
            $resolved = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "$declaredTerminal^{commit}")
            if (-not $resolved.Ok) {
                $defects += "identity mismatch: terminal commit $declaredTerminal is not a commit in this repository"
            }
            else {
                $sha = $resolved.Text.Trim()
                $onTrial = (Invoke-Git -Arguments @('merge-base', '--is-ancestor', $sha, $Identity.Branch)).Ok
                $onBase = (Invoke-Git -Arguments @('merge-base', '--is-ancestor', $sha, $Base)).Ok
                if (-not $onTrial -or $onBase) {
                    $defects += "identity mismatch: terminal commit $declaredTerminal is not work done on $($Identity.Branch)"
                }
            }
        }
    }

    $verdict = ''
    if ($bodies.ContainsKey('Verdict')) {
        $verdict = @($bodies['Verdict'] | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -First 1)
    }
    if ($verdict) { $State | Add-Member -NotePropertyName Verdict -NotePropertyValue ([string]$verdict).Trim() -Force }
    $defects
}

# TWL-005: the ordered, resumable finish.
function Invoke-Finish {
    param([Parameter(Mandatory)] $Facts)

    if ([string]::IsNullOrEmpty($Facts.BaseTip)) {
        Deny -Reason "the experiment base $Base does not exist in this repository" -Guidance @(
            "create the branch $Base from the commit the experiments start at."
        )
    }
    if ($Facts.InLinkedTree) {
        Deny -Reason 'finish runs from the primary checkout, not from inside a trial worktree' -Guidance @(
            "cd $($Facts.Primary)",
            'then run finish again; nothing has been touched.'
        )
    }
    if ($Facts.CurrentBranch -ne $Base) {
        Deny -Reason "finish runs with the experiment base $Base checked out, but $($Facts.CurrentBranch) is" -Guidance @(
            "git checkout $Base"
        )
    }
    if ($Facts.Porcelain.Count -gt 0) {
        $entries = @('the base checkout must be clean; commit or put aside these entries first:') +
            @($Facts.Porcelain | ForEach-Object { "  $_" })
        Deny -Reason 'the experiment base is not clean' -Guidance $entries
    }

    $identity = Resolve-Trial -Facts $Facts
    $state = Get-TrialState -Facts $Facts -Identity $identity
    $resume = "pwsh -File tools/trial.ps1 finish -Slug $($identity.Slug)"

    if (-not $state.Registered -and -not $state.WorktreeExists) {
        Deny -Reason "the trial worktree for $($identity.Branch) is not registered" -Guidance @(
            "git worktree add $($identity.WorktreePath) $($identity.Branch)",
            'then run finish again.'
        )
    }
    if ($state.WorktreePorcelain.Count -gt 0) {
        $entries = @("the trial worktree $($state.WorktreePath) must be clean; commit or put aside:") +
            @($state.WorktreePorcelain | ForEach-Object { "  $_" })
        Deny -Reason 'the trial worktree is not clean' -Guidance $entries
    }

    $defects = @(Get-LogDefects -Identity $identity -State $state)
    if ($defects.Count -gt 0) {
        $entries = @("commit a valid trial-log.md on $($identity.Branch) - the template is .arca/tpl/trial-log.md:") +
            @($defects | ForEach-Object { "  $_" })
        Deny -Reason "the trial log is invalid, so no tag was created and nothing was deleted" -Guidance $entries
    }
    $verdict = if ($state.PSObject.Properties.Name -contains 'Verdict') { $state.Verdict } else { 'no verdict' }

    Write-Report "finishing $($identity.Branch) at $($state.Terminal)"
    Write-Report ''

    # Step 1: the archive tag, before any deletion.
    if ($state.TagExists) {
        if ($state.TagTarget -ne $state.Terminal) {
            Deny -Reason "the archive tag $($identity.ArchiveTag) already points at $($state.TagTarget), not at the terminal commit $($state.Terminal)" -Guidance @(
                'inspect that tag; nothing was created and nothing was deleted.',
                "the later steps (durable log, worktree removal, branch deletion) did not run."
            )
        }
        if (-not $state.TagAnnotated) {
            Deny -Reason "the archive tag $($identity.ArchiveTag) exists but is not annotated" -Guidance @(
                "git tag -d $($identity.ArchiveTag)",
                'then run finish again; nothing was deleted.'
            )
        }
        Write-Report "step 1/4 archive tag $($identity.ArchiveTag): already at $($state.Terminal) (resumed)"
    }
    else {
        $message = "trial $($identity.Branch)`nbase $($state.ForkPoint)`nterminal $($state.Terminal)`nverdict $verdict"
        $tagging = Invoke-Git -Arguments @('tag', '-a', $identity.ArchiveTag, $state.Terminal, '-m', $message)
        if (-not $tagging.Ok) {
            Deny -Reason "creating the archive tag failed: $($tagging.Text)" -Guidance @(
                'nothing was deleted; the durable log, worktree removal, and branch deletion did not run.'
            )
        }
        $verify = Invoke-Git -Arguments @('rev-parse', "$($identity.ArchiveTag)^{commit}")
        $kind = Invoke-Git -Arguments @('cat-file', '-t', $identity.ArchiveTag)
        if (-not $verify.Ok -or $verify.Text.Trim() -ne $state.Terminal -or -not $kind.Ok -or $kind.Text.Trim() -ne 'tag') {
            Deny -Reason 'the archive tag did not verify at the terminal commit' -Guidance @(
                "git tag -d $($identity.ArchiveTag)",
                'nothing was deleted; run finish again after checking the repository.'
            )
        }
        Write-Report "step 1/4 archive tag $($identity.ArchiveTag): created at $($state.Terminal)"
    }

    # Step 2: the durable log, alone, on the base.
    if ($state.DurableCommitted) {
        Write-Report "step 2/4 durable log $($identity.DurableLog): already committed (resumed)"
    }
    else {
        $directory = Split-Path -Parent $state.DurablePath
        if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
            New-Item -ItemType Directory -Path $directory -Force | Out-Null
        }
        $body = if ($state.LogText.EndsWith("`n")) { $state.LogText } else { $state.LogText + "`n" }
        [System.IO.File]::WriteAllText($state.DurablePath, $body, [System.Text.UTF8Encoding]::new($false))
        $staged = Invoke-Git -Arguments @('add', '--', $identity.DurableLog)
        $stagedNames = Invoke-Git -Arguments @('diff', '--cached', '--name-only')
        if (-not $staged.Ok -or $stagedNames.Text.Trim() -ne $identity.DurableLog) {
            Deny -Reason "staging the durable log would have committed more than the log: $($stagedNames.Text)" -Guidance @(
                "git reset -- $($identity.DurableLog)",
                "the archive tag $($identity.ArchiveTag) stands; the worktree and branch were left alone.",
                $resume
            )
        }
        $commit = Invoke-Git -Arguments @('commit', '-m', "trial($($identity.Branch)): archive durable log")
        if (-not $commit.Ok) {
            Deny -Reason "committing the durable log failed: $($commit.Text)" -Guidance @(
                "the archive tag $($identity.ArchiveTag) stands; the worktree and branch were left alone.",
                $resume
            )
        }
        $touched = Invoke-Git -Arguments @('diff-tree', '--no-commit-id', '--name-only', '-r', 'HEAD')
        if ($touched.Text.Trim() -ne $identity.DurableLog) {
            Deny -Reason "the base commit carries more than the durable log: $($touched.Text)" -Guidance @(
                'inspect HEAD on the base; the worktree and branch were left alone.'
            )
        }
        Write-Report "step 2/4 durable log $($identity.DurableLog): committed on $Base"
    }

    # Step 3: the linked worktree, removed without force.
    $stillRegistered = @(@(Get-GitLines -Arguments @('worktree', 'list', '--porcelain')) |
        Where-Object { $_ -eq "branch refs/heads/$($identity.Branch)" })
    if ($stillRegistered.Count -eq 0) {
        Write-Report "step 3/4 worktree $($state.WorktreePath): already removed (resumed)"
    }
    else {
        $removal = Invoke-Git -Arguments @('worktree', 'remove', $state.WorktreePath)
        if (-not $removal.Ok) {
            Deny -Reason "removing the trial worktree $($state.WorktreePath) failed: $($removal.Text)" -Guidance @(
                'close every shell, editor, and process rooted in that directory - nothing here removes it by force and no process is killed.',
                "the archive tag $($identity.ArchiveTag) and the durable log stand; the branch was not deleted.",
                $resume
            )
        }
        Write-Report "step 3/4 worktree $($state.WorktreePath): removed"
    }

    # Step 4: the branch, deleted only while the tag still preserves it.
    $current = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "refs/heads/$($identity.Branch)")
    if (-not $current.Ok) {
        Write-Report "step 4/4 branch $($identity.Branch): already deleted (resumed)"
    }
    else {
        $preserved = Invoke-Git -Arguments @('rev-parse', "$($identity.ArchiveTag)^{commit}")
        if (-not $preserved.Ok -or $preserved.Text.Trim() -ne $current.Text.Trim()) {
            Deny -Reason "the archive tag no longer preserves $($identity.Branch), so the branch was kept" -Guidance @(
                "branch: $($current.Text.Trim())",
                "tag:    $(if ($preserved.Ok) { $preserved.Text.Trim() } else { 'unreadable' })",
                'inspect both before deleting anything by hand.'
            )
        }
        $deletion = Invoke-Git -Arguments @(
            'update-ref', '-d', "refs/heads/$($identity.Branch)", $current.Text.Trim())
        if (-not $deletion.Ok) {
            Deny -Reason "deleting $($identity.Branch) failed: $($deletion.Text)" -Guidance @(
                "everything else is done; the branch still points at $($current.Text.Trim()).",
                $resume
            )
        }
        Write-Report "step 4/4 branch $($identity.Branch): deleted (preserved by $($identity.ArchiveTag))"
    }

    Write-Report ''
    Write-Report 'recovery (recreates exactly what the deletion removed):'
    foreach ($line in Get-FinishRecovery -Identity $identity) { Write-Report "  $line" }
    exit 0
}

# TWL-007: the base receives main by merging, and by nothing else.
function Invoke-Sync {
    param([Parameter(Mandatory)] $Facts)

    if ([string]::IsNullOrEmpty($Facts.BaseTip)) {
        Deny -Reason "the experiment base $Base does not exist in this repository" -Guidance @(
            "create the branch $Base from the commit the experiments start at."
        )
    }
    if ($Facts.InLinkedTree) {
        Deny -Reason 'sync runs from the primary checkout, not from inside a trial worktree' -Guidance @(
            "cd $($Facts.Primary)",
            "git checkout $Base",
            'then run sync again; nothing has been touched.'
        )
    }
    if ($Facts.CurrentBranch -ne $Base) {
        Deny -Reason "sync merges main into $Base, but $($Facts.CurrentBranch) is checked out" -Guidance @(
            "git checkout $Base",
            'fixes are authored on main and reach the base only here.'
        )
    }
    if ($Facts.Porcelain.Count -gt 0) {
        $entries = @('the base checkout must be clean before a merge; commit or put aside:') +
            @($Facts.Porcelain | ForEach-Object { "  $_" })
        Deny -Reason 'the experiment base is not clean' -Guidance $entries
    }
    if (-not (Test-RefExists -Ref 'refs/heads/main')) {
        Deny -Reason 'there is no local main to merge from' -Guidance @(
            'create or check out main first; sync never fetches.'
        )
    }

    $before = (Get-GitText -Arguments @('rev-parse', "refs/heads/$Base")).Trim()
    $merge = Invoke-Git -Arguments @('merge', 'main')
    if (-not $merge.Ok) {
        $conflicted = @(Get-GitLines -Arguments @('diff', '--name-only', '--diff-filter=U'))
        $entries = @('the merge is left exactly as Git left it - not aborted, not reset, not rebased:') +
            @($conflicted | ForEach-Object { "  $_" }) +
            @(
                'resolve those files, then: git add <file> && git commit',
                'to walk away instead, decide yourself: git merge --quit or git reset are yours to run, never this script''s.'
            )
        Deny -Reason "merging main into $Base stopped with conflicts" -Guidance $entries
    }

    $after = (Get-GitText -Arguments @('rev-parse', "refs/heads/$Base")).Trim()
    Write-Report "sync: merged main into $Base"
    Write-Report "  before: $before"
    Write-Report "  after:  $after"
    if ($before -eq $after) {
        Write-Report '  the base was already up to date; nothing changed.'
    }
    Write-Report ''
    Write-Report 'live trials keep their own branches and worktrees; this merge touched only the base checkout.'
    exit 0
}

$facts = Get-Facts
switch ($Verb) {
    'status' { Invoke-Status -Facts $facts }
    'start' { Invoke-Start -Facts $facts }
    'finish' { Invoke-Finish -Facts $facts }
    'sync' { Invoke-Sync -Facts $facts }
    default {
        Deny -Reason "unknown verb '$Verb'" -Guidance @(
            'available verbs: status, start, finish, sync',
            'pwsh -File tools/trial.ps1 status',
            'pwsh -File tools/trial.ps1 start -Slug <topic> [-Number <n>]',
            'pwsh -File tools/trial.ps1 finish [-Slug <topic>] [-Number <n>]',
            'pwsh -File tools/trial.ps1 sync'
        )
    }
}
