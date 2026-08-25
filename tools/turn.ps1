#!/usr/bin/env pwsh
#Requires -Version 7

<#
.SYNOPSIS
    THK-001..THK-004: the repo-local turn lifecycle - the trial lifecycle's
    sibling under tools/.

.DESCRIPTION
    A work item's turn (a ticket's build turn in this shop, but the item is
    an opaque string to this tool) lives on its own branch and linked sibling
    worktree. This entry point opens and closes it in the working rules'
    fixed order, so the order is a mechanism and not discipline alone.

    Verbs available today:

        pwsh -File tools/turn.ps1 status [-Item <string>]
        pwsh -File tools/turn.ps1 open   -Item <string>
        pwsh -File tools/turn.ps1 close  -Item <string> [-Line <words>]
                                         [-Obsolete <artifact>...]

    `status` is the dry run: it prints the declared data, the live turns,
    and each mutating verb's planned mutations and recovery commands. It
    changes nothing.

    `open` checks every precondition - clean trunk, no colliding branch or
    worktree registration, lanes root present - before the first Git write;
    a failed precondition refuses with a named reason and zero mutation.
    Then it creates the item-named branch with its registered sibling
    worktree and copies the declared untracked lanes root in, skipping each
    lane's declared build output.

    `close` runs the fixed order, each step refusing on its own failure and
    blocking every later step: merge into the trunk (fast-forward when it
    has not moved, otherwise one merge commit), verified copy-back of the
    lanes root, stamp of the landed tip into the item record's declared
    field, the landing's line appended to the declared landing log, worktree
    removal, branch deletion, and the lanes rerun from the trunk. Per-step
    completion is recorded in the repository state itself - the trunk
    containing the item, the lanes digests agreeing, the stamp field, the
    log line, the registration, the ref - and read back on resume, so an
    interrupted close resumes from the first uncompleted step and never
    repeats a landed mutation. A removal refuses while the worktree holds
    the only copy of any artifact, naming the artifact and the copy-back
    that would release it; the one explicit release beside a verified
    copy-back is obsolete-by-name declaration. The stamp edits (item record
    and landing log) are left uncommitted in the trunk for the caller to
    land with the record flips they own.

    Ownership (the trial lifecycle's table): a human or the Main-Agent
    invokes these verbs from the primary checkout with the trunk checked
    out. An Advisor authors content only. A Subagent invokes no lifecycle
    verb. Every verb - the dry run included - refuses from inside a linked
    worktree and prints the `cd` that fixes it: on Windows a shell standing
    in a worktree keeps a handle on it.

    `-Force` is accepted so that muscle memory meets the same refusal; it
    never changes a verdict. Nothing here reaches the network, installs
    anything, edits global Git configuration, kills a process, or deletes a
    directory by hand: plain Git and built-in cmdlets only.

    Declared data lives in .ratmac/turn.toml - the runbook's `[roots]`
    table shape (names to values), repo-local beside the runbook, because
    the runbook parser owns ratmac.toml's tables and refuses unknown keys.
    The tool reads: trunk (branch), lanes (untracked root copied per turn),
    item-records (root holding <item>.md), landing-log, stamp-field,
    skip (per-lane build-output directory names), lanes-rerun (the command
    run once in each lane directory from the trunk).
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string] $Verb = 'status',

    [string] $Item = '',

    [string] $Line = '',

    [string[]] $Obsolete = @(),

    # Accepted and never honored: no refusal in this lifecycle has a force
    # variant, so the flag exists only to meet the same refusal.
    [switch] $Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:ItemPattern = '^[A-Za-z0-9][A-Za-z0-9._-]*$'

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
    [Console]::Error.WriteLine("turn refused; $Reason")
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

# --- declared data -----------------------------------------------------------

# The one quoted string a value may be, or the list of them.
function Convert-ToValue {
    param([Parameter(Mandatory)][string] $Raw, [Parameter(Mandatory)][string] $Key)

    if ($Raw -cmatch '^\[(.*)\]$') {
        $found = @([regex]::Matches($Matches[1], '"([^"]*)"') | ForEach-Object { $_.Groups[1].Value })
        return , $found
    }
    if ($Raw -cmatch '^"([^"]*)"$') {
        return , $Matches[1]
    }
    Deny -Reason "the declared key $Key in .ratmac/turn.toml is malformed: $Raw" -Guidance @(
        'a value is one quoted string or a list of quoted strings.'
    )
}

# The turn declaration, read from .ratmac/turn.toml's [roots] table.
function Get-Declaration {
    param([Parameter(Mandatory)][string] $RepoRoot)

    $path = Join-Path $RepoRoot '.ratmac/turn.toml'
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        Deny -Reason "the turn declaration $path does not exist" -Guidance @(
            'declare the turn data in .ratmac/turn.toml under a [roots] table:',
            'trunk, lanes, item-records, landing-log, stamp-field, skip, lanes-rerun.'
        )
    }

    $values = @{}
    $inRoots = $false
    foreach ($rawLine in [System.IO.File]::ReadAllLines($path)) {
        $line = $rawLine.Trim()
        if ($line -eq '' -or $line.StartsWith('#')) { continue }
        if ($line -ceq '[roots]') { $inRoots = $true; continue }
        if ($line.StartsWith('[')) { $inRoots = $false; continue }
        if (-not $inRoots) { continue }
        if ($line -cmatch '^([A-Za-z0-9-]+)\s*=\s*(.+)$') {
            $key = $Matches[1]
            $value = $Matches[2].Trim()
            # A trailing comment sits outside the closing quote or bracket.
            if ($value -cmatch '^(("[^"]*")|(\[[^\]]*\]))\s*#') { $value = $Matches[1].Trim() }
            $values[$key] = Convert-ToValue -Raw $value -Key $key
        }
        else {
            Deny -Reason "the turn declaration $path has a malformed line: $line" -Guidance @(
                'a line is `key = "value"` or `key = ["value", ...]`.'
            )
        }
    }

    $required = 'trunk', 'lanes', 'item-records', 'landing-log', 'stamp-field', 'skip', 'lanes-rerun'
    foreach ($key in $required) {
        if (-not $values.ContainsKey($key)) {
            Deny -Reason "the turn declaration $path declares no $key" -Guidance @(
                "add $key to the [roots] table in .ratmac/turn.toml."
            )
        }
    }
    foreach ($key in 'trunk', 'lanes', 'item-records', 'landing-log', 'stamp-field', 'lanes-rerun') {
        if ($values[$key] -isnot [string] -or [string]::IsNullOrEmpty($values[$key])) {
            Deny -Reason "the declared $key must be one non-empty quoted string" -Guidance @(
                'lists are allowed only for skip.'
            )
        }
    }
    foreach ($key in 'lanes', 'item-records', 'landing-log') {
        $value = [string]$values[$key]
        if ([System.IO.Path]::IsPathRooted($value) -or $value -like '*..*' -or $value -match '^[A-Za-z]:') {
            Deny -Reason "the declared $key path $value must stay repository-relative"
        }
    }

    [pscustomobject]@{
        Trunk       = [string]$values['trunk']
        Lanes       = [string]$values['lanes']
        ItemRecords = [string]$values['item-records']
        LandingLog  = [string]$values['landing-log']
        StampField  = [string]$values['stamp-field']
        Skip        = @([string[]]$values['skip'])
        LanesRerun  = [string]$values['lanes-rerun']
    }
}

# --- facts -------------------------------------------------------------------

# Everything the verbs decide from, read once, mutating nothing.
function Get-Facts {
    $inside = Invoke-Git -Arguments @('rev-parse', '--is-inside-work-tree')
    if (-not $inside.Ok) {
        Deny -Reason 'this directory is not a Git repository' -Guidance @(
            'cd to the primary checkout and run the verb again.'
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

    [pscustomobject]@{
        RepoRoot      = $repoRoot
        RepoName      = Split-Path -Leaf $repoRoot
        Parent        = Resolve-PathText (Split-Path -Parent $repoRoot)
        Primary       = $primary
        InLinkedTree  = ($gitDir -ne $commonDir)
        CurrentBranch = (Get-GitText -Arguments @('rev-parse', '--abbrev-ref', 'HEAD')).Trim()
        Porcelain     = @(Get-GitLines -Arguments @('status', '--porcelain'))
        Registrations = $registrations
    }
}

# THK-004's ownership, on every verb: the primary checkout only.
function Guard-PrimaryInvocation {
    param(
        [Parameter(Mandatory)][string] $VerbName,
        [Parameter(Mandatory)] $Facts
    )
    if (-not $Facts.InLinkedTree) { return }
    Deny -Reason "$VerbName runs from the primary checkout, not from inside the linked worktree $($Facts.RepoRoot)" -Guidance @(
        "cd $($Facts.Primary)",
        'then run the same command again; nothing has been touched.'
    )
}

function Test-RefExists {
    param([Parameter(Mandatory)][string] $Ref)
    (Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', $Ref)).Ok
}

# The item is an opaque binding string; the pattern only keeps it a safe
# branch name and path segment.
function Guard-Item {
    if ([string]::IsNullOrEmpty($Item)) {
        Deny -Reason "$Verb needs an item: -Item <string>" -Guidance @(
            'the item is the opaque branch name the turn runs on.'
        )
    }
    if ($Item -cnotmatch $script:ItemPattern) {
        Deny -Reason "the item '$Item' is malformed" -Guidance @(
            'an item is letters, digits, dots, dashes, and underscores, starting alphanumeric: [A-Za-z0-9][A-Za-z0-9._-]*'
        )
    }
}

# The worktree path registered for a branch, from the registrations.
function Find-WorktreeFor {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)][string] $Branch
    )
    $current = ''
    foreach ($line in @($Facts.Registrations)) {
        if ($line.StartsWith('worktree ')) {
            $current = Resolve-PathText $line.Substring('worktree '.Length)
        }
        elseif ($line -eq "branch refs/heads/$Branch") {
            return $current
        }
    }
    ''
}

# --- tree walking ------------------------------------------------------------

# `relative path sha256` rows for every file under a root, skipping the
# declared build-output directory names at any depth.
function Get-TreeRows {
    param(
        [Parameter(Mandatory)][string] $Root,
        [Parameter(Mandatory)][string[]] $SkipNames
    )
    $rows = [System.Collections.Generic.List[string]]::new()
    if (Test-Path -LiteralPath $Root -PathType Container) {
        Add-TreeRows -Directory $Root -Prefix '' -SkipNames $SkipNames -Rows $rows
    }
    @($rows.ToArray() | Sort-Object)
}

# The recursive half of Get-TreeRows: appends into the shared list, so no
# child scope ever shadows the accumulation.
function Add-TreeRows {
    param(
        [Parameter(Mandatory)][string] $Directory,
        [AllowEmptyString()][string] $Prefix = '',
        [Parameter(Mandatory)][string[]] $SkipNames,
        [Parameter(Mandatory)] $Rows
    )
    foreach ($entry in @(Get-ChildItem -LiteralPath $Directory -Force)) {
        if ($entry.PSIsContainer) {
            if ($SkipNames -contains $entry.Name) { continue }
            if ($entry.Name -eq '.git') { continue }
            Add-TreeRows -Directory $entry.FullName -Prefix ($Prefix + $entry.Name + '/') -SkipNames $SkipNames -Rows $Rows
        }
        else {
            $digest = (Get-FileHash -LiteralPath $entry.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            $Rows.Add("$($Prefix + $entry.Name) $digest")
        }
    }
}

# The lanes rows under a checkout's declared lanes root.
function Get-LanesRows {
    param(
        [Parameter(Mandatory)][string] $CheckoutRoot,
        [Parameter(Mandatory)] $Decl
    )
    Get-TreeRows -Root (Join-Path $CheckoutRoot $Decl.Lanes) -SkipNames $Decl.Skip
}

# Copy a directory tree into a destination, skipping the declared
# build-output names at any depth and writing nothing else.
function Copy-LanesTree {
    param(
        [Parameter(Mandatory)][string] $Source,
        [Parameter(Mandatory)][string] $Destination,
        [Parameter(Mandatory)][string[]] $SkipNames
    )
    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    foreach ($entry in @(Get-ChildItem -LiteralPath $Source -Force)) {
        if ($entry.PSIsContainer) {
            if ($SkipNames -contains $entry.Name) { continue }
            if ($entry.Name -eq '.git') { continue }
            Copy-LanesTree -Source $entry.FullName -Destination (Join-Path $Destination $entry.Name) -SkipNames $SkipNames
        }
        else {
            Copy-Item -LiteralPath $entry.FullName -Destination (Join-Path $Destination $entry.Name)
        }
    }
}

# The first `field: "value"` line's value, or '' when the record carries no
# such field.
function Get-FieldValue {
    param(
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][string] $Field
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    $pattern = '^\s*' + [regex]::Escape($Field) + ':\s*"([^"]*)"'
    foreach ($line in [System.IO.File]::ReadAllLines($Path)) {
        if ($line -cmatch $pattern) { return $Matches[1] }
    }
    $null
}

# --- open --------------------------------------------------------------------

# THK-001: every collision is named before anything is written.
function Get-OpenCollisions {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl,
        [Parameter(Mandatory)][string] $BranchItem,
        [Parameter(Mandatory)][string] $WorktreePath
    )
    $collisions = @()
    $branchExists = Test-RefExists -Ref "refs/heads/$BranchItem"
    $registeredBranch = ''
    $registeredThere = $false
    $lines = @($Facts.Registrations)
    for ($index = 0; $index -lt $lines.Count; $index++) {
        if (-not $lines[$index].StartsWith('worktree ')) { continue }
        $registeredPath = Resolve-PathText $lines[$index].Substring('worktree '.Length)
        if ($registeredPath -ne $WorktreePath) { continue }
        $registeredThere = $true
        for ($scan = $index + 1; $scan -lt $lines.Count; $scan++) {
            if ($lines[$scan].StartsWith('worktree ')) { break }
            if ($lines[$scan].StartsWith('branch ')) {
                $registeredBranch = $lines[$scan].Substring('branch refs/heads/'.Length)
                break
            }
        }
        break
    }

    if ($branchExists -and $registeredBranch -eq $BranchItem) {
        # The turn is already open exactly as planned: the copy resumes.
        return [pscustomobject]@{ Collisions = @(); Resume = $true; RegisteredBranch = $registeredBranch }
    }
    if ($branchExists) {
        $collisions += "branch $BranchItem already exists"
    }
    if ($registeredThere) {
        if ($registeredBranch -ne '') {
            $collisions += "a worktree is already registered at $WorktreePath (branch $registeredBranch)"
        }
        else {
            $collisions += "a worktree is already registered at $WorktreePath"
        }
    }
    elseif (Test-Path -LiteralPath $WorktreePath) {
        $collisions += "the sibling path $WorktreePath already exists"
    }
    [pscustomobject]@{ Collisions = $collisions; Resume = $false; RegisteredBranch = $registeredBranch }
}

# Roll a failed open back, or say exactly how to finish it by hand.
function Undo-Open {
    param(
        [Parameter(Mandatory)][string] $BranchItem,
        [Parameter(Mandatory)][string] $WorktreePath,
        [Parameter(Mandatory)][string] $TrunkTip,
        [Parameter(Mandatory)][string] $Failure
    )
    $unrecovered = @()

    # Read the registrations as they are now: a half-finished creation may
    # have left state the caller's facts never saw.
    $fresh = Get-GitLines -Arguments @('worktree', 'list', '--porcelain')
    $registered = $false
    foreach ($line in $fresh) {
        if ($line -eq "branch refs/heads/$BranchItem") { $registered = $true }
    }
    if ($registered) {
        $removal = Invoke-Git -Arguments @('worktree', 'remove', $WorktreePath)
        if (-not $removal.Ok) {
            $unrecovered += "git worktree remove $WorktreePath"
        }
    }

    # Compare-and-delete: the ref goes only while it still points at the
    # trunk tip this open recorded, so concurrent work is never discarded.
    $current = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "refs/heads/$BranchItem")
    if ($current.Ok) {
        if ($current.Text.Trim() -eq $TrunkTip) {
            $deletion = Invoke-Git -Arguments @('update-ref', '-d', "refs/heads/$BranchItem", $TrunkTip)
            if (-not $deletion.Ok) {
                $unrecovered += "git update-ref -d refs/heads/$BranchItem $TrunkTip"
            }
        }
        else {
            $unrecovered += "refs/heads/$BranchItem moved to $($current.Text.Trim()); it was left alone - inspect it before deleting"
        }
    }

    if (Test-Path -LiteralPath $WorktreePath) {
        $unrecovered += "the sibling path $WorktreePath still exists; inspect what is in it before taking it away"
    }

    if ($unrecovered.Count -eq 0) {
        Deny -Reason "opening the turn failed: $Failure" -Guidance @(
            'nothing persists: the branch ref was removed and no worktree was registered.',
            'fix the reported cause and run open again.'
        )
    }
    Deny -Reason "opening the turn failed: $Failure" -Guidance (
        @('rollback could not finish; run these by hand, in order:') + $unrecovered
    )
}

function Invoke-Open {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl
    )

    $trunkTipResult = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "refs/heads/$($Decl.Trunk)")
    if (-not $trunkTipResult.Ok) {
        Deny -Reason "the declared trunk $($Decl.Trunk) does not exist in this repository"
    }
    $trunkTip = $trunkTipResult.Text.Trim()

    if ($Facts.CurrentBranch -ne $Decl.Trunk) {
        Deny -Reason "open starts from the declared trunk $($Decl.Trunk), but $($Facts.CurrentBranch) is checked out" -Guidance @(
            "git checkout $($Decl.Trunk)"
        )
    }
    if ($Facts.Porcelain.Count -gt 0) {
        $entries = @('the trunk must be clean; commit or put aside these entries first:') +
            @($Facts.Porcelain | ForEach-Object { "  $_" })
        Deny -Reason "the trunk $($Decl.Trunk) is not clean" -Guidance $entries
    }

    $lanesRoot = Join-Path $Facts.RepoRoot $Decl.Lanes
    if (-not (Test-Path -LiteralPath $lanesRoot -PathType Container)) {
        Deny -Reason "the declared lanes root $($Decl.Lanes) is absent from the primary checkout" -Guidance @(
            "create $($Decl.Lanes) (it is the untracked root the turn carries) or declare another lanes path in .ratmac/turn.toml."
        )
    }

    $worktreePath = "$($Facts.Parent)/$($Facts.RepoName)-$Item"
    $collisions = Get-OpenCollisions -Facts $Facts -Decl $Decl -BranchItem $Item -WorktreePath $worktreePath
    if ($collisions.Collisions.Count -gt 0) {
        Deny -Reason "the turn for $Item collides with existing work" -Guidance (
            @($collisions.Collisions) + @('choose another item, or finish the colliding turn first.')
        )
    }
    if (-not $collisions.Resume) {
        $creation = Invoke-Git -Arguments @('worktree', 'add', '-b', $Item, $worktreePath, $trunkTip)
        if (-not $creation.Ok) {
            Undo-Open -BranchItem $Item -WorktreePath $worktreePath -TrunkTip $trunkTip -Failure $creation.Text
        }
        # Verify the creation against the plan as it stands now, not against
        # the caller's pre-creation facts.
        $created = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "refs/heads/$Item")
        $registeredNow = $false
        foreach ($line in Get-GitLines -Arguments @('worktree', 'list', '--porcelain')) {
            if ($line -eq "branch refs/heads/$Item") { $registeredNow = $true }
        }
        if (-not $created.Ok -or $created.Text.Trim() -ne $trunkTip -or -not $registeredNow) {
            Undo-Open -BranchItem $Item -WorktreePath $worktreePath -TrunkTip $trunkTip -Failure 'the created turn does not match the plan'
        }
    }

    # The lanes copy: files in, declared build output skipped. A failure on
    # a fresh open undoes the whole turn; a failure on a resumed open leaves
    # the open turn exactly as it was, to resume again.
    $copied = 0
    $copyFailure = ''
    try {
        $before = @(Get-LanesRows -CheckoutRoot $Facts.RepoRoot -Decl $Decl)
        Copy-LanesTree -Source $lanesRoot -Destination (Join-Path $worktreePath $Decl.Lanes) -SkipNames $Decl.Skip
        $copied = $before.Count
    }
    catch {
        $copyFailure = "the lanes copy failed: $($_.Exception.Message)"
    }
    if (-not $copyFailure) {
        $sourceRows = @(Get-LanesRows -CheckoutRoot $Facts.RepoRoot -Decl $Decl)
        $targetRows = @(Get-LanesRows -CheckoutRoot $worktreePath -Decl $Decl)
        if (Compare-Object -ReferenceObject $sourceRows -DifferenceObject $targetRows) {
            $copyFailure = 'the lanes copy did not verify against the source'
        }
    }
    if ($copyFailure) {
        if ($collisions.Resume) {
            Deny -Reason "opening the turn failed: $copyFailure" -Guidance @(
                'the already-open turn was left exactly as it was; nothing was undone.',
                'fix the reported cause and run open again to redo the copy.'
            )
        }
        Undo-Open -BranchItem $Item -WorktreePath $worktreePath -TrunkTip $trunkTip -Failure $copyFailure
    }

    if ($collisions.Resume) {
        Write-Report "turn already open; the lanes copy completed again (skipped: $($Decl.Skip -join ', '))"
    }
    else {
        Write-Report "turn opened"
        Write-Report "  branch:     $Item at $trunkTip"
        Write-Report "  worktree:   $worktreePath"
        Write-Report "  lanes root: $($Decl.Lanes) -> $($worktreePath)/$($Decl.Lanes) ($copied files, skipped: $($Decl.Skip -join ', '))"
        Write-Report ""
        Write-Report "next: cd $worktreePath"
        Write-Report "undo: git worktree remove $worktreePath && git update-ref -d refs/heads/$Item $trunkTip"
    }
    exit 0
}

# --- the turn state ----------------------------------------------------------

# Everything close and status decide from, read without mutating anything.
function Get-TurnState {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl,
        [Parameter(Mandatory)][string] $BranchItem
    )
    $trunkTip = ''
    $tipResult = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "refs/heads/$($Decl.Trunk)")
    if ($tipResult.Ok) { $trunkTip = $tipResult.Text.Trim() }

    $branchTip = ''
    $branchResult = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "refs/heads/$BranchItem")
    if ($branchResult.Ok) { $branchTip = $branchResult.Text.Trim() }

    $merged = $false
    if ($branchTip -and $trunkTip) {
        $merged = (Invoke-Git -Arguments @('merge-base', '--is-ancestor', "refs/heads/$BranchItem", "refs/heads/$($Decl.Trunk)")).Ok
    }

    $worktreePath = Find-WorktreeFor -Facts $Facts -Branch $BranchItem
    if (-not $worktreePath) { $worktreePath = "$($Facts.Parent)/$($Facts.RepoName)-$BranchItem" }
    $worktreeExists = Test-Path -LiteralPath $worktreePath -PathType Container
    $worktreeDirty = @()
    if ($worktreeExists) {
        $status = Invoke-Git -Arguments @('-C', $worktreePath, 'status', '--porcelain')
        if ($status.Ok -and -not [string]::IsNullOrWhiteSpace($status.Text)) {
            $worktreeDirty = @($status.Text -split "`r?`n" | Where-Object { $_ -ne '' -and -not $_.StartsWith('??') })
        }
    }

    $lanesEqual = $false
    $lanesNote = ''
    if ($worktreeExists) {
        try {
            $lanesEqual = -not (Compare-Object -ReferenceObject @(Get-LanesRows -CheckoutRoot $worktreePath -Decl $Decl) `
                    -DifferenceObject @(Get-LanesRows -CheckoutRoot $Facts.RepoRoot -Decl $Decl))
        }
        catch {
            # A held or locked lanes file is a fact to report, not a crash:
            # the copy-back step refuses by name when it reaches it.
            $lanesNote = $_.Exception.Message
        }
    }

    $itemRecordPath = Join-Path (Join-Path $Facts.RepoRoot $Decl.ItemRecords) "$BranchItem.md"
    $landingLogPath = Join-Path $Facts.RepoRoot $Decl.LandingLog
    $trunkShort = ''
    if ($trunkTip) {
        $short = Invoke-Git -Arguments @('rev-parse', '--short', "refs/heads/$($Decl.Trunk)")
        if ($short.Ok) { $trunkShort = $short.Text.Trim() }
    }
    $stampValue = Get-FieldValue -Path $itemRecordPath -Field $Decl.StampField

    $logLinePresent = $false
    if ($Line -and (Test-Path -LiteralPath $landingLogPath -PathType Leaf)) {
        foreach ($entry in [System.IO.File]::ReadAllLines($landingLogPath)) {
            if ($entry.Contains($Line)) { $logLinePresent = $true; break }
        }
    }

    [pscustomobject]@{
        TrunkTip        = $trunkTip
        TrunkShort      = $trunkShort
        LanesEqual      = $lanesEqual
        LanesNote       = $lanesNote
        BranchTip       = $branchTip
        Merged          = $merged
        WorktreePath    = $worktreePath
        WorktreeExists  = $worktreeExists
        WorktreeDirty   = $worktreeDirty
        Registered      = ((Find-WorktreeFor -Facts $Facts -Branch $BranchItem) -ne '')
        ItemRecordPath  = $itemRecordPath
        LandingLogPath  = $landingLogPath
        StampValue      = [string]$stampValue
        LogLinePresent  = $logLinePresent
    }
}

# THK-003: the artifacts whose only copy lives in the worktree - untracked
# or ignored entries, declared build outputs aside - with the rows that
# would be lost.
function Get-OnlyCopyArtifacts {
    param(
        [Parameter(Mandatory)][string] $WorktreeRoot,
        [Parameter(Mandatory)][string] $PrimaryRoot,
        [Parameter(Mandatory)] $Decl
    )
    $artifacts = @()
    $entries = @()
    $status = Invoke-Git -Arguments @('-C', $WorktreeRoot, 'status', '--porcelain', '--ignored=matching', '--untracked-files=normal')
    if ($status.Ok -and -not [string]::IsNullOrWhiteSpace($status.Text)) {
        $entries = @($status.Text -split "`r?`n" | Where-Object {
                $_.StartsWith('??') -or $_.StartsWith('!!')
            } | ForEach-Object { $_.Substring(3).TrimEnd('/') })
    }
    foreach ($entry in $entries) {
        $parts = @($entry -split '/')
        $skipped = $false
        foreach ($part in $parts) {
            if ($Decl.Skip -contains $part) { $skipped = $true; break }
        }
        if ($skipped) { continue }

        # Only the rows whose only copy is here would be lost: a row the
        # primary already carries is released, a row absent here is already
        # preserved. A row that cannot be read is treated as held, and a
        # held artifact blocks the removal rather than crashing it.
        try {
            $inWorktree = @(Get-TreeRows -Root (Join-Path $WorktreeRoot $entry) -SkipNames $Decl.Skip)
            $inPrimary = @(Get-TreeRows -Root (Join-Path $PrimaryRoot $entry) -SkipNames $Decl.Skip)
        }
        catch {
            $artifacts += "$entry (unreadable: held or locked by another process)"
            continue
        }
        $preserved = [System.Collections.Generic.HashSet[string]]::new([string[]]$inPrimary)
        foreach ($row in $inWorktree) {
            if (-not $preserved.Contains($row)) {
                $artifacts += $entry
                break
            }
        }
    }
    $artifacts
}

# --- close -------------------------------------------------------------------

function Invoke-Close {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl
    )

    if (-not (Test-RefExists -Ref "refs/heads/$($Decl.Trunk)")) {
        Deny -Reason "the declared trunk $($Decl.Trunk) does not exist in this repository"
    }
    if ($Facts.CurrentBranch -ne $Decl.Trunk) {
        Deny -Reason "close merges into the declared trunk $($Decl.Trunk), but $($Facts.CurrentBranch) is checked out" -Guidance @(
            "git checkout $($Decl.Trunk)",
            'then run close again; nothing has been touched.'
        )
    }

    # A merge left in progress by an earlier refused close is named as such,
    # not as generic dirt.
    if ((Invoke-Git -Arguments @('rev-parse', '-q', '--verify', 'MERGE_HEAD')).Ok) {
        $conflicted = @(Get-GitLines -Arguments @('diff', '--name-only', '--diff-filter=U'))
        Deny -Reason "a merge is already in progress on $($Decl.Trunk)" -Guidance (
            @('resolve the conflicted files, then: git add <file> && git commit') +
            @($conflicted | ForEach-Object { "  $_" }) +
            @('the copy-back, stamp, log line, removal, and branch deletion did not run.')
        )
    }

    # The trunk must be clean - except the close's own declared writes from
    # an earlier interrupted close, which belong to the stamp landing the
    # caller commits.
    $itemRecordRel = "$($Decl.ItemRecords)/$Item.md"
    $others = @()
    $excused = @()
    foreach ($entry in @($Facts.Porcelain)) {
        $path = $entry.Substring(3).Trim('"')
        if ($path -eq $itemRecordRel -or $path -eq $Decl.LandingLog) { $excused += $entry }
        else { $others += $entry }
    }
    if ($others.Count -gt 0) {
        $entries = @('the trunk must be clean; commit or put aside these entries first:') +
            @($others | ForEach-Object { "  $_" })
        Deny -Reason "the trunk $($Decl.Trunk) is not clean" -Guidance $entries
    }

    if (-not (Test-RefExists -Ref "refs/heads/$Item")) {
        Deny -Reason "no branch $Item exists to close"
    }

    $state = Get-TurnState -Facts $Facts -Decl $Decl -BranchItem $Item
    $resume = "pwsh -File tools/turn.ps1 close -Item $Item$(if ($Line) { " -Line '$Line'" })"

    if (-not $state.Registered -and -not $state.WorktreeExists -and -not $state.Merged) {
        # Nothing was ever opened and nothing ever landed: there is no turn
        # to close. (An unregistered, deleted worktree after a landed merge
        # is a resume, handled at the removal step.)
        Deny -Reason "the turn worktree for $Item is not registered" -Guidance @(
            "git worktree add $($state.WorktreePath) $Item",
            'then run close again.'
        )
    }
    if ($state.WorktreeDirty.Count -gt 0) {
        $entries = @("the turn worktree $($state.WorktreePath) must have its work committed; commit or put aside:") +
            @($state.WorktreeDirty | ForEach-Object { "  $_" })
        Deny -Reason 'the turn worktree holds uncommitted work' -Guidance $entries
    }

    Write-Report "closing $Item into $($Decl.Trunk)"
    Write-Report ''
    if ($excused.Count -gt 0) {
        Write-Report "note: the stamp edits from an earlier close sit uncommitted in the trunk (the stamp landing is yours to commit):"
        foreach ($entry in $excused) { Write-Report "  $entry" }
        Write-Report ''
    }

    # Step 1: the merge - fast-forward when the trunk has not moved, else
    # one merge commit that is itself a landing.
    if ($state.Merged) {
        Write-Report "step 1/7 merge: already landed (resumed)"
    }
    else {
        $merge = Invoke-Git -Arguments @('merge', $Item)
        if (-not $merge.Ok) {
            $conflicted = @(Get-GitLines -Arguments @('diff', '--name-only', '--diff-filter=U'))
            Deny -Reason "merging $Item into $($Decl.Trunk) stopped with conflicts" -Guidance (
                @('the merge is left exactly as Git left it - not aborted, not rewound, not rebased:') +
                @($conflicted | ForEach-Object { "  $_" }) +
                @(
                    'resolve those files, then: git add <file> && git commit',
                    'the copy-back, stamp, log line, removal, and branch deletion did not run.',
                    $resume
                )
            )
        }
        $landed = (Invoke-Git -Arguments @('merge-base', '--is-ancestor', "refs/heads/$Item", "refs/heads/$($Decl.Trunk)")).Ok
        if (-not $landed) {
            Deny -Reason "the merge reported success but $($Decl.Trunk) does not contain $Item" -Guidance @(
                'inspect the repository; nothing was copied, stamped, appended, or removed.',
                $resume
            )
        }
        $moved = (Invoke-Git -Arguments @('merge-base', '--is-ancestor', $state.TrunkTip, "refs/heads/$Item")).Ok
        $shape = if ($moved) { 'fast-forward' } else { 'one merge commit' }
        Write-Report "step 1/7 merge: $Item landed on $($Decl.Trunk) ($shape)"
    }

    # Step 2: the verified copy-back of the lanes root.
    if ($state.LanesEqual -or (-not $state.WorktreeExists -and $state.Merged)) {
        # A vanished worktree behind a landed merge is a resume: the fixed
        # order ran the verified copy-back before any removal.
        Write-Report "step 2/7 copy-back: $($Decl.Lanes) already verified (resumed)"
    }
    else {
        $fromLanes = Join-Path $state.WorktreePath $Decl.Lanes
        $toLanes = Join-Path $Facts.RepoRoot $Decl.Lanes
        if (Test-Path -LiteralPath $fromLanes -PathType Container) {
            try {
                Copy-LanesTree -Source $fromLanes -Destination $toLanes -SkipNames $Decl.Skip
            }
            catch {
                Deny -Reason "the copy-back of $($Decl.Lanes) could not be verified" -Guidance (
                    @("the copy failed: $($_.Exception.Message)") +
                    @('the stamp, log line, removal, and branch deletion did not run.') +
                    @('fix the reported cause and run close again; the merge stands.', $resume)
                )
            }
        }
        $verifyFailure = ''
        try {
            $after = @(Get-LanesRows -CheckoutRoot $state.WorktreePath -Decl $Decl)
            $here = @(Get-LanesRows -CheckoutRoot $Facts.RepoRoot -Decl $Decl)
        }
        catch {
            $verifyFailure = $_.Exception.Message
        }
        if ($verifyFailure) {
            Deny -Reason "the copy-back of $($Decl.Lanes) could not be verified: a lanes file is held or locked" -Guidance (
                @("the unreadable file: $verifyFailure") +
                @('close whatever holds it, then run close again; the merge stands.', $resume)
            )
        }
        $diff = @(Compare-Object -ReferenceObject $after -DifferenceObject $here | Select-Object -First 5)
        if ($diff.Count -gt 0) {
            $entries = @('the copy-back did not verify; these lanes files differ between the worktree and the primary checkout:') +
                @($diff | ForEach-Object { "  $($_.InputObject)" })
            Deny -Reason "the copy-back of $($Decl.Lanes) could not be verified" -Guidance (
                $entries +
                @('the stamp, log line, removal, and branch deletion did not run.') +
                @('fix the reported cause and run close again; the merge stands.', $resume)
            )
        }
        Write-Report "step 2/7 copy-back: $($Decl.Lanes) copied back and verified (skipped: $($Decl.Skip -join ', '))"
    }

    # Step 3: the stamp - the landed tip resolved from the trunk now, after
    # the merge, never predicted from memory.
    $landedShort = (Get-GitText -Arguments @('rev-parse', '--short', "refs/heads/$($Decl.Trunk)")).Trim()
    if (-not (Test-Path -LiteralPath $state.ItemRecordPath -PathType Leaf)) {
        Deny -Reason "the item record $($Decl.ItemRecords)/$Item.md does not exist" -Guidance @(
            "the stamp step writes the $($Decl.StampField) field there; create the record or fix item-records in .ratmac/turn.toml.",
            'the log line, removal, and branch deletion did not run.',
            $resume
        )
    }
    if ($state.StampValue -eq $landedShort) {
        Write-Report "step 3/7 stamp: $($Decl.StampField) already carries the landed tip $landedShort (resumed)"
    }
    else {
        $recordText = [System.IO.File]::ReadAllText($state.ItemRecordPath)
        $pattern = '(?m)^(\s*' + [regex]::Escape($Decl.StampField) + ':\s*")[^"]*(")'
        if (-not [regex]::IsMatch($recordText, $pattern)) {
            Deny -Reason "the item record $($Decl.ItemRecords)/$Item.md carries no $($Decl.StampField) field" -Guidance @(
                'the stamp step writes the declared field; fix the record or stamp-field in .ratmac/turn.toml.',
                'the log line, removal, and branch deletion did not run.',
                $resume
            )
        }
        $written = [regex]::Replace($recordText, $pattern, "`${1}$landedShort`${2}", 1)
        [System.IO.File]::WriteAllText($state.ItemRecordPath, $written, [System.Text.UTF8Encoding]::new($false))
        $check = Get-FieldValue -Path $state.ItemRecordPath -Field $Decl.StampField
        if ($check -ne $landedShort) {
            Deny -Reason "the stamp did not verify: $($Decl.StampField) reads '$check', expected '$landedShort'" -Guidance @(
                'the log line, removal, and branch deletion did not run.',
                $resume
            )
        }
        Write-Report "step 3/7 stamp: $($Decl.StampField) = $landedShort (the landed tip, resolved after the merge)"
    }

    # Step 4: the landing's line, appended to the declared log.
    if ($state.LogLinePresent) {
        Write-Report "step 4/7 log line: already appended (resumed)"
    }
    elseif ([string]::IsNullOrEmpty($Line)) {
        Deny -Reason "close needs -Line <words> to append the landing's line to $($Decl.LandingLog)" -Guidance @(
            "pwsh -File tools/turn.ps1 close -Item $Item -Line '<the landing's words>'",
            'the merge, copy-back, and stamp stand; the removal and branch deletion did not run.',
            "resume the close from the log-line step: $resume"
        )
    }
    else {
        if (-not (Test-Path -LiteralPath $state.LandingLogPath -PathType Leaf)) {
            Deny -Reason "the declared landing log $($Decl.LandingLog) does not exist" -Guidance @(
                'the log step appends the landing line there; create it or fix landing-log in .ratmac/turn.toml.',
                'the removal and branch deletion did not run.',
                $resume
            )
        }
        $logText = [System.IO.File]::ReadAllText($state.LandingLogPath)
        $newline = if ($logText.Contains("`r`n")) { "`r`n" } else { "`n" }
        $date = (Get-Date).ToString('yyyy-MM-dd')
        $body = $logText
        if ($body.Length -gt 0 -and -not $body.EndsWith($newline)) { $body += $newline }
        $body += "- ${date}: $Line$newline"
        [System.IO.File]::WriteAllText($state.LandingLogPath, $body, [System.Text.UTF8Encoding]::new($false))
        $found = $false
        foreach ($entry in [System.IO.File]::ReadAllLines($state.LandingLogPath)) {
            if ($entry.Contains($Line)) { $found = $true; break }
        }
        if (-not $found) {
            Deny -Reason "the landing line did not verify in $($Decl.LandingLog)" -Guidance @(
                'the removal and branch deletion did not run.',
                $resume
            )
        }
        Write-Report "step 4/7 log line: appended to $($Decl.LandingLog)"
    }

    # Step 5: the removal - behind the only-copy refusal.
    if (-not $state.Registered) {
        if ($state.WorktreeExists) {
            # Git drops the registration even when the deletion failed, so
            # an unregistered directory is an incomplete removal, not a
            # completed one.
            Deny -Reason "the turn worktree $($state.WorktreePath) is no longer registered but its directory remains" -Guidance @(
                'close every process standing inside it - nothing here kills a process or deletes a directory by hand,',
                "then take the directory away yourself: rmdir /s /q `"$($state.WorktreePath)`"",
                'inspect it first if it holds anything you did not copy out.',
                'the merge, copy-back, stamp, and log line stand; the branch deletion and lanes rerun did not run.',
                $resume
            )
        }
        Write-Report "step 5/7 worktree: already removed (resumed)"
    }
    else {
        $artifacts = @(Get-OnlyCopyArtifacts -WorktreeRoot $state.WorktreePath -PrimaryRoot $Facts.RepoRoot -Decl $Decl)
        $unreleased = @()
        foreach ($artifact in $artifacts) {
            if ($Obsolete -contains $artifact) {
                Write-Report "  declared obsolete by its owner: $artifact"
            }
            else {
                $unreleased += $artifact
            }
        }
        if ($unreleased.Count -gt 0) {
            $entries = @('the worktree holds the only copy of these artifacts; a removal would destroy them:') +
                @($unreleased | ForEach-Object { "  $_" }) +
                @(
                    'the copy-back that would release each one: copy it into the primary checkout and verify the bytes,',
                    "or declare it obsolete by name: add -Obsolete <artifact> to the close.",
                    'no force variant exists: -Force is accepted and changes nothing.'
                )
            Deny -Reason 'the removal is refused: the worktree holds the only copy' -Guidance (
                $entries +
                @('the merge, copy-back, stamp, and log line stand; the worktree and branch were left alone.', $resume)
            )
        }
        $removal = Invoke-Git -Arguments @('worktree', 'remove', $state.WorktreePath)
        if (-not $removal.Ok) {
            Deny -Reason "removing the turn worktree $($state.WorktreePath) failed: $($removal.Text)" -Guidance @(
                'close every shell, editor, and process rooted in that directory - nothing here forces a removal and no process is killed.',
                'the merge, copy-back, stamp, and log line stand; the branch was not deleted.',
                $resume
            )
        }
        Write-Report "step 5/7 worktree: $($state.WorktreePath) removed"
    }

    # Step 6: the branch, deleted only once the trunk holds it.
    $current = Invoke-Git -Arguments @('rev-parse', '--verify', '--quiet', "refs/heads/$Item")
    if (-not $current.Ok) {
        Write-Report "step 6/7 branch: $Item already deleted (resumed)"
    }
    else {
        $tip = $current.Text.Trim()
        $preserved = (Invoke-Git -Arguments @('merge-base', '--is-ancestor', "refs/heads/$Item", "refs/heads/$($Decl.Trunk)")).Ok
        if (-not $preserved) {
            Deny -Reason "the branch $Item is not contained in $($Decl.Trunk), so it was kept" -Guidance @(
                "branch: $tip",
                'inspect it before deleting anything by hand.'
            )
        }
        $deletion = Invoke-Git -Arguments @('update-ref', '-d', "refs/heads/$Item", $tip)
        if (-not $deletion.Ok) {
            Deny -Reason "deleting $Item failed: $($deletion.Text)" -Guidance @(
                "everything else is done; the branch still points at $tip.",
                $resume
            )
        }
        Write-Report "step 6/7 branch: $Item deleted (preserved by $($Decl.Trunk))"
    }

    # Step 7: the lanes rerun, once, from the trunk.
    $lanesRoot = Join-Path $Facts.RepoRoot $Decl.Lanes
    $laneDirs = @()
    if (Test-Path -LiteralPath $lanesRoot -PathType Container) {
        $laneDirs = @(Get-ChildItem -LiteralPath $lanesRoot -Directory | Where-Object { $Decl.Skip -notcontains $_.Name })
    }
    if ($laneDirs.Count -eq 0) {
        Write-Report "step 7/7 lanes rerun: no lane directories under $($Decl.Lanes)"
    }
    else {
        foreach ($lane in $laneDirs) {
            Push-Location $lane.FullName
            try {
                & pwsh -NoProfile -Command $Decl.LanesRerun | Out-Null
                $code = $LASTEXITCODE
            }
            finally {
                Pop-Location
            }
            if ($code -ne 0) {
                Deny -Reason "the lanes rerun failed in $($lane.FullName) (exit $code)" -Guidance @(
                    "the rerun command was: $($Decl.LanesRerun)",
                    'everything else is done: the turn is closed and the landing stands; rerun the lanes yourself and judge the failure.'
                )
            }
        }
        Write-Report "step 7/7 lanes rerun: ran in $($laneDirs.Count) lane director$(if ($laneDirs.Count -eq 1) { 'y' } else { 'ies' }) under $($Decl.Lanes)"
    }

    Write-Report ''
    Write-Report "the turn for $Item is closed"
    Write-Report "  the stamped item record and the appended landing line sit uncommitted on $($Decl.Trunk):"
    Write-Report "  commit them as the stamp landing - the record flips that ride it are yours."
    if ($Obsolete.Count -gt 0) {
        Write-Report "  declared obsolete this close: $($Obsolete -join ', ')"
    }
    exit 0
}

# --- status ------------------------------------------------------------------

function Invoke-Status {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl
    )

    Write-Report "turn status (read-only; nothing below has been applied)"
    Write-Report ''
    Write-Report "declared data (.ratmac/turn.toml):"
    Write-Report "  trunk:         $($Decl.Trunk)"
    Write-Report "  lanes root:    $($Decl.Lanes)"
    Write-Report "  item records:  $($Decl.ItemRecords)/<item>.md"
    Write-Report "  landing log:   $($Decl.LandingLog)"
    Write-Report "  stamp field:   $($Decl.StampField)"
    Write-Report "  skip list:     $($Decl.Skip -join ', ')"
    Write-Report "  lanes rerun:   $($Decl.LanesRerun)"
    $clean = if ($Facts.Porcelain.Count -eq 0) { 'clean' } else { "dirty ($($Facts.Porcelain.Count) entries)" }
    Write-Report ''
    Write-Report "checked out:      $($Facts.CurrentBranch)"
    Write-Report "working tree:     $clean"
    Write-Report "primary checkout: $($Facts.Primary)"
    Write-Report ''

    Write-Report 'live turns (registered sibling worktrees):'
    $live = @()
    $current = ''
    foreach ($line in @($Facts.Registrations)) {
        if ($line.StartsWith('worktree ')) { $current = Resolve-PathText $line.Substring('worktree '.Length) }
        elseif ($line.StartsWith('branch ') -and $current -and $current -ne $Facts.Primary) {
            $branch = $line.Substring('branch refs/heads/'.Length)
            if ((Resolve-PathText (Split-Path -Parent $current)) -eq $Facts.Parent) {
                $live += "  $branch at $current"
            }
        }
    }
    if ($live.Count -eq 0) { Write-Report '  (none)' }
    foreach ($entry in $live) { Write-Report $entry }
    Write-Report ''

    if (-not [string]::IsNullOrEmpty($Item)) {
        $state = Get-TurnState -Facts $Facts -Decl $Decl -BranchItem $Item
        Write-Report "turn ${Item}:"
        Write-Report "  branch:    $(if ($state.BranchTip) { $state.BranchTip } else { 'missing' })"
        Write-Report "  worktree:  $(if ($state.Registered) { $state.WorktreePath } else { 'not registered' })"
        Write-Report "  merge:     $(if ($state.Merged) { 'landed' } else { 'pending' })"
        Write-Report "  copy-back: $(if ($state.LanesEqual) { 'verified' } else { 'pending' })$(if ($state.LanesNote) { " - unreadable: $($state.LanesNote)" })"
        Write-Report "  stamp:     $($Decl.StampField) = '$($state.StampValue)'$(if ($state.StampValue -eq $state.TrunkShort -and $state.TrunkShort) { ' (the landed tip)' })"
        Write-Report "  log line:  $(if ($state.LogLinePresent) { 'appended' } else { 'absent' })"
        if ($state.Registered) {
            $only = @(Get-OnlyCopyArtifacts -WorktreeRoot $state.WorktreePath -PrimaryRoot $Facts.RepoRoot -Decl $Decl)
            if ($only.Count -gt 0) {
                Write-Report '  only-copy artifacts (a removal would refuse; copy them out or declare them obsolete):'
                foreach ($artifact in $only) { Write-Report "    $artifact" }
            }
            else {
                Write-Report '  only-copy artifacts: none (a removal would proceed)'
            }
        }
        Write-Report ''
    }

    $itemLabel = if ([string]::IsNullOrEmpty($Item)) { '<item>' } else { $Item }
    $worktreePath = "$($Facts.Parent)/$($Facts.RepoName)-$itemLabel"
    Write-Report 'open plan:'
    Write-Report "  git worktree add -b $itemLabel $worktreePath $($Decl.Trunk)"
    Write-Report "  copy $($Decl.Lanes) -> $worktreePath/$($Decl.Lanes), skipping: $($Decl.Skip -join ', ')"
    Write-Report '  recovery if it fails midway:'
    Write-Report "    git worktree remove $worktreePath"
    Write-Report '    git update-ref -d refs/heads/<item> <trunk-tip>'
    $blocked = @()
    if ($Facts.Porcelain.Count -gt 0) { $blocked += 'the working tree is not clean' }
    if (-not (Test-Path -LiteralPath (Join-Path $Facts.RepoRoot $Decl.Lanes) -PathType Container)) {
        $blocked += "the declared lanes root $($Decl.Lanes) is absent from the primary checkout"
    }
    if ($blocked.Count -gt 0) {
        Write-Report '  blocked by:'
        foreach ($entry in $blocked) { Write-Report "    $entry" }
    }
    Write-Report ''

    Write-Report 'close plan (in this order; a failing step refuses it and every later step):'
    Write-Report "  1. merge $itemLabel into $($Decl.Trunk) (fast-forward when the trunk has not moved, else one merge commit)"
    Write-Report "  2. copy-back: $worktreePath/$($Decl.Lanes) -> $($Facts.Primary)/$($Decl.Lanes), verified byte for byte"
    Write-Report "  3. stamp: write the landed $($Decl.Trunk) tip into $($Decl.ItemRecords)/<item>.md field $($Decl.StampField)"
    Write-Report "  4. log line: append '- <date>: <words>' to $($Decl.LandingLog) (pass -Line '<words>')"
    Write-Report "  5. remove the worktree $worktreePath (refuses while it holds the only copy of any artifact)"
    Write-Report "  6. delete the branch $itemLabel (only once $($Decl.Trunk) holds it)"
    Write-Report "  7. re-run the lanes once from the trunk: $($Decl.LanesRerun), in each directory under $($Decl.Lanes)"
    Write-Report '  recovery / resume (continues from the first uncompleted step):'
    Write-Report '    pwsh -File tools/turn.ps1 close -Item <item> -Line ''<words>'''
    Write-Report '  a conflicted merge is left exactly as Git left it: resolve, git add <file> && git commit, then resume.'
    Write-Report '  the stamp edits the close leaves in the trunk are committed by you as the stamp landing.'
    Write-Report ''
    exit 0
}

# --- dispatch ----------------------------------------------------------------

$facts = Get-Facts
switch ($Verb) {
    'status' {
        Guard-PrimaryInvocation -VerbName 'status' -Facts $facts
        if (-not [string]::IsNullOrEmpty($Item) -and $Item -cnotmatch $script:ItemPattern) {
            Deny -Reason "the item '$Item' is malformed" -Guidance @(
                'an item is letters, digits, dots, dashes, and underscores, starting alphanumeric.'
            )
        }
        $declaration = Get-Declaration -RepoRoot $facts.RepoRoot
        Invoke-Status -Facts $facts -Decl $declaration
    }
    'open' {
        Guard-PrimaryInvocation -VerbName 'open' -Facts $facts
        Guard-Item
        $declaration = Get-Declaration -RepoRoot $facts.RepoRoot
        Invoke-Open -Facts $facts -Decl $declaration
    }
    'close' {
        Guard-PrimaryInvocation -VerbName 'close' -Facts $facts
        Guard-Item
        $declaration = Get-Declaration -RepoRoot $facts.RepoRoot
        Invoke-Close -Facts $facts -Decl $declaration
    }
    default {
        Deny -Reason "unknown verb '$Verb'" -Guidance @(
            'available verbs: status, open, close',
            'pwsh -File tools/turn.ps1 status [-Item <string>]',
            'pwsh -File tools/turn.ps1 open -Item <string>',
            "pwsh -File tools/turn.ps1 close -Item <string> [-Line <words>] [-Obsolete <artifact>...]"
        )
    }
}
