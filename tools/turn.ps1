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
        pwsh -File tools/turn.ps1 checkpoint -Item <string> -Target <path>...
        pwsh -File tools/turn.ps1 restore -Item <string>
        pwsh -File tools/turn.ps1 review -Item <string> -Verdict <accepted|rejected>
        pwsh -File tools/turn.ps1 proofs -Item <string>

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
    removal, branch deletion, and the final lanes verification - the declared
    lanes-rerun command run per the declared lanes-rerun-scope. Per-step
    completion is recorded in the repository state itself - the trunk
    containing the item, the lanes digests agreeing, the stamp field, the
    log line, the registration, the ref - and read back on resume, so an
    interrupted close resumes from the first uncompleted step and never
    repeats a landed mutation. A final verification that failed after the
    cleanup retries the same close branchlessly: once no item branch,
    worktree registration, or worktree folder remains, the stamp equals the
    current trunk tip, the landing line is already in the log, and no
    unrelated dirt exists, the repeated close proves those facts and runs
    only the final verification again, refusing by naming any missing proof.
    A removal refuses while the worktree holds
    the only copy of any artifact, naming the artifact and the copy-back
    that would release it; the one explicit release beside a verified
    copy-back is obsolete-by-name declaration. The stamp edits (item record
    and landing log) are left uncommitted in the trunk for the caller to
    land with the record flips they own.

    `checkpoint` and `restore` are the damage pair. Before deliberate
    damage, `checkpoint` proves the effective checkout settings round-trip
    every declared -Target path's bytes in both directions through Git's
    own conversion, then freezes one manifest recording each target's exact
    worktree digest, the checkpoint commit's blob id, the effective
    text/eol/filter attributes, and the effective core.autocrlf and core.eol
    settings. `restore` returns exactly the manifest's targets from the
    recorded commit through `git restore --staged --worktree`, then accepts
    only when every recomputed digest matches its frozen digest and the
    tracked tree is clean - a clean status alone never passes bytes that
    differ. Neither verb writes configuration, refs, or the index, and
    neither refuses from a linked worktree: the damage turn lives there,
    deliberately.

    `review` and `proofs` are the review-before-proofs pair (WCP-004).
    `review` records the independent source and test-oracle review over the
    exact bytes of the addressed checkout - every work row plus every lane
    row, its own evidence never a row - as one snapshot digest in the item's
    review record; `proofs` runs the declared lanes-rerun command exactly
    once behind an accepted, fresh review and records the proof receipt
    binding that snapshot. One accepted snapshot earns one proof set; the
    restored-green rerun after each mutation and close step 7's verification
    from the primary checkout stay mandatory and ungated. `checkpoint`
    freezes only reviewed, proven green work; `restore` and every lifecycle
    verb stay ungated. Neither verb consults turn state: the addressed
    checkout's bytes, the declared data, and the item-named records are
    their whole input.

    Ownership (the trial lifecycle's table): a human or the Main-Agent
    invokes these verbs from the primary checkout with the trunk checked
    out. An Advisor authors content only. A Subagent invokes no lifecycle
    verb. Every lifecycle verb - the dry run included - refuses from inside
    a linked worktree and prints the `cd` that fixes it: on Windows a shell
    standing in a worktree keeps a handle on it.

    `-Force` is accepted so that muscle memory meets the same refusal; it
    never changes a verdict. Nothing here reaches the network, installs
    anything, edits global Git configuration, kills a process, or deletes a
    directory by hand: plain Git and built-in cmdlets only.

    Declared data lives in .ratmac/turn.toml - the runbook's `[roots]`
    table shape (names to values), repo-local beside the runbook, because
    the runbook parser owns ratmac.toml's tables and refuses unknown keys.
    The tool reads: trunk (branch), lanes (untracked root copied per turn),
    item-records (root holding <item>.md), landing-log, stamp-field,
    skip (per-lane build-output directory names), lanes-rerun (the declared
    verification command), and the optional lanes-rerun-scope: `per-lane`
    (the default) runs the command once in each lane directory from the
    trunk, `root-once` runs it exactly once from the primary checkout, and
    any other value refuses before the first write.
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string] $Verb = 'status',

    [string] $Item = '',

    [string] $Line = '',

    [string[]] $Obsolete = @(),

    # One declared damage target spelling, or several comma-joined:
    # `pwsh -File` delivers `-Target a,b` as one string, so the damage
    # pair splits every value on commas and drops the empty pieces.
    [string[]] $Target = @(),

    # The independent review's recorded decision: exactly 'accepted' or
    # 'rejected' on `review`, and accepted-but-ignored on every verb that
    # does not read it - the same discipline as `-Force` (WCP-004).
    [string] $Verdict = '',

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

    # The closed optional scope for the final verification (ADR-0021):
    # omitted keeps the per-lane default; anything but the two known values
    # refuses here, before any verb writes anything.
    $rerunScope = 'per-lane'
    if ($values.ContainsKey('lanes-rerun-scope')) {
        $declaredScope = $values['lanes-rerun-scope']
        if ($declaredScope -isnot [string] -or [string]::IsNullOrEmpty($declaredScope)) {
            Deny -Reason "the declared lanes-rerun-scope must be one non-empty quoted string: 'per-lane' or 'root-once'" -Guidance @(
                'a list or an empty string is not a scope; omit the key to keep the per-lane default.'
            )
        }
        if ($declaredScope -cne 'per-lane' -and $declaredScope -cne 'root-once') {
            Deny -Reason "the declared lanes-rerun-scope '$declaredScope' is unknown" -Guidance @(
                "the scope is one of: 'per-lane', 'root-once'; omit the key to keep the per-lane default."
            )
        }
        $rerunScope = $declaredScope
    }

    [pscustomobject]@{
        Trunk       = [string]$values['trunk']
        Lanes       = [string]$values['lanes']
        ItemRecords = [string]$values['item-records']
        LandingLog  = [string]$values['landing-log']
        StampField  = [string]$values['stamp-field']
        Skip        = @([string[]]$values['skip'])
        LanesRerun  = [string]$values['lanes-rerun']
        RerunScope  = $rerunScope
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

    $expectedWorktreePath = "$($Facts.Parent)/$($Facts.RepoName)-$BranchItem"
    # Independent of the branch-based registration below: any registration
    # - detached, prunable, or on another branch - squatting on the expected
    # sibling path is a fact of its own, so a stale registration at the turn's
    # path is caught while unrelated worktrees are left alone.
    $registeredAtExpected = $false
    foreach ($registration in @($Facts.Registrations)) {
        if ($registration.StartsWith('worktree ') -and (Resolve-PathText $registration.Substring('worktree '.Length)) -eq $expectedWorktreePath) {
            $registeredAtExpected = $true
            break
        }
    }

    $worktreePath = Find-WorktreeFor -Facts $Facts -Branch $BranchItem
    if (-not $worktreePath) { $worktreePath = $expectedWorktreePath }
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
        ExpectedWorktreePath = $expectedWorktreePath
        RegisteredAtExpected = $registeredAtExpected
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

# Step 7, shared by the ordinary close and the branchless final-only retry:
# the final lanes verification, run per the declared lanes-rerun-scope.
# `per-lane` keeps the legacy behavior - the declared command once in each
# lane directory from the trunk; `root-once` runs it exactly once from the
# primary checkout, its output streamed so a failure names the crate or
# marker it died on. The command's meaning stays the command's own: this
# tool parses no expiry marker.
function Invoke-LanesVerification {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl,
        [Parameter(Mandatory)][AllowEmptyString()][string] $Resume
    )

    if ($Decl.RerunScope -eq 'root-once') {
        Push-Location $Facts.RepoRoot
        try {
            & pwsh -NoProfile -Command $Decl.LanesRerun
            $code = $LASTEXITCODE
        }
        finally {
            Pop-Location
        }
        if ($code -ne 0) {
            Deny -Reason "the final verification failed from the primary checkout (exit $code; scope root-once)" -Guidance @(
                "the verification command was: $($Decl.LanesRerun)",
                "it ran once from $($Facts.RepoRoot)",
                'the turn is closed and the landing stands; repair the reported failure, then repeat the same close:',
                "  $Resume"
            )
        }
        Write-Report 'step 7/7 lanes rerun: ran once from the primary checkout (scope root-once)'
        return
    }

    $lanesRoot = Join-Path $Facts.RepoRoot $Decl.Lanes
    $laneDirs = @()
    if (Test-Path -LiteralPath $lanesRoot -PathType Container) {
        $laneDirs = @(Get-ChildItem -LiteralPath $lanesRoot -Directory | Where-Object { $Decl.Skip -notcontains $_.Name })
    }
    if ($laneDirs.Count -eq 0) {
        Write-Report "step 7/7 lanes rerun: no lane directories under $($Decl.Lanes)"
        return
    }
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

# The close's tail report, shared by the ordinary close and the branchless
# final-only retry.
function Write-ClosedReport {
    param(
        [Parameter(Mandatory)][string] $BranchItem,
        [Parameter(Mandatory)] $Decl,
        [string[]] $ObsoleteNames = @()
    )
    Write-Report ''
    Write-Report "the turn for $BranchItem is closed"
    Write-Report "  the stamped item record and the appended landing line sit uncommitted on $($Decl.Trunk):"
    Write-Report '  commit them as the stamp landing - the record flips that ride it are yours.'
    if ($ObsoleteNames.Count -gt 0) {
        Write-Report "  declared obsolete this close: $($ObsoleteNames -join ', ')"
    }
}

# TCE-001's final-only retry: the item branch is gone, so every close
# mutation is behind; the close may run only the final verification, and
# only once every completed-landing fact is proven. Each refusal names the
# missing proof, not merely the absent branch. An item branch that still
# exists never reaches here - THK-002's ordinary interrupted-close resume
# owns that case.
function Invoke-FinalOnlyRetry {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl,
        [Parameter(Mandatory)] $State,
        [Parameter(Mandatory)][AllowEmptyString()][string] $Resume,
        [string[]] $Excused = @()
    )

    $missing = @()
    if ($State.RegisteredAtExpected) {
        $missing += "no worktree registration at the expected path - one is still registered at $($State.ExpectedWorktreePath) (detached or on another branch); remove it: git worktree remove $($State.ExpectedWorktreePath)"
    }
    if ($State.WorktreeExists) {
        $missing += "no worktree folder at the expected path - the directory $($State.ExpectedWorktreePath) still exists; inspect it and take it away yourself (the retry deletes no directory)"
    }
    if (-not $State.TrunkShort -or $State.StampValue -ne $State.TrunkShort) {
        $missing += "the $($Decl.StampField) stamp equal to the current $($Decl.Trunk) tip - it reads '$($State.StampValue)', the trunk tip is '$($State.TrunkShort)'"
    }
    if ([string]::IsNullOrEmpty($Line)) {
        $missing += "the landing's -Line - the retry proves the log line is already present, so it needs the words to look for"
    }
    elseif (-not $State.LogLinePresent) {
        $missing += "the landing line '$Line' already present in $($Decl.LandingLog) - the log does not carry it"
    }
    if ($missing.Count -gt 0) {
        Deny -Reason "no branch $Item exists, and the final-only retry of the final verification lacks its completed-landing proof" -Guidance (
            @('the branchless retry requires, each proven or refused by name:') +
            @($missing | ForEach-Object { "  missing: $_" }) +
            @('an item branch that still exists resumes the ordinary close order instead.', $Resume)
        )
    }

    Write-Report "closing $Item into $($Decl.Trunk)"
    Write-Report ''
    if ($Excused.Count -gt 0) {
        Write-Report 'note: the stamp edits from the landed close sit uncommitted in the trunk (the stamp landing is yours to commit):'
        foreach ($entry in $Excused) { Write-Report "  $entry" }
        Write-Report ''
    }
    Write-Report 'final-only retry: the landing already stands and every close mutation is proven complete,'
    Write-Report 'so only the final verification runs - no merge, copy-back, stamp, log line, removal, or branch deletion is repeated.'
    Invoke-LanesVerification -Facts $Facts -Decl $Decl -Resume $Resume
    Write-ClosedReport -BranchItem $Item -Decl $Decl -ObsoleteNames $Obsolete
    exit 0
}

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

    $state = Get-TurnState -Facts $Facts -Decl $Decl -BranchItem $Item
    $resume = "pwsh -File tools/turn.ps1 close -Item $Item$(if ($Line) { " -Line '$Line'" })"

    if (-not $state.BranchTip) {
        # No item branch: either nothing was ever opened, or the landing is
        # fully complete - the final-only retry sorts those two apart, naming
        # every missing proof.
        Invoke-FinalOnlyRetry -Facts $Facts -Decl $Decl -State $state -Resume $resume -Excused $excused
    }

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

    # Step 7: the final lanes verification, per the declared scope.
    Invoke-LanesVerification -Facts $Facts -Decl $Decl -Resume $resume

    Write-ClosedReport -BranchItem $Item -Decl $Decl -ObsoleteNames $Obsolete
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
    # The scope's execution location, stated where the declared data sits.
    $scopeWhere = if ($Decl.RerunScope -eq 'root-once') { "once from the primary checkout ($($Facts.RepoRoot))" } else { "once in each lane directory under $($Decl.Lanes)" }
    Write-Report "  rerun scope:   $($Decl.RerunScope) - the command runs $scopeWhere"
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
    Write-Report "  7. re-run the lanes (scope $($Decl.RerunScope)): $($Decl.LanesRerun), $scopeWhere"
    Write-Report '  recovery / resume (continues from the first uncompleted step):'
    Write-Report '    pwsh -File tools/turn.ps1 close -Item <item> -Line ''<words>'''
    Write-Report '  a failed final verification retries the same close once the landing proof holds'
    Write-Report '    (no item branch, registration, or worktree folder; the stamp equals the trunk tip;'
    Write-Report '    the landing line already in the log; no unrelated dirt) - the retry verifies only.'
    Write-Report '  a conflicted merge is left exactly as Git left it: resolve, git add <file> && git commit, then resume.'
    Write-Report '  the stamp edits the close leaves in the trunk are committed by you as the stamp landing.'
    Write-Report ''
    exit 0
}

# --- checkpoint and restore (WCP-005) ----------------------------------------

# The damage pair's one test seam, dormant unless the QA harness sets
# RATMAC_TEST_TURN_PAUSE explicitly on the child: validated after the item
# guard and before any write, so an invalid request refuses having written
# nothing. Each boundary belongs to exactly one verb.
function Get-TestPause {
    param([Parameter(Mandatory)][AllowEmptyCollection()][string[]] $Boundaries)

    $requested = [Environment]::GetEnvironmentVariable('RATMAC_TEST_TURN_PAUSE')
    if ([string]::IsNullOrEmpty($requested)) { return $null }

    if ($Boundaries -notcontains $requested) {
        $known = 'after-freeze', 'after-validate', 'after-restore', 'after-verify', 'after-proof-command'
        $fact = if ($known -contains $requested) { "$requested does not belong to this verb" }
        else { "unknown boundary '$requested'" }
        Deny -Reason "the test pause request is invalid: $fact" -Guidance @(
            'set RATMAC_TEST_TURN_PAUSE to a known boundary with its marker, release, and timeout companions, or unset it.'
        )
    }

    $marker = [Environment]::GetEnvironmentVariable('RATMAC_TEST_TURN_PAUSE_MARKER')
    $release = [Environment]::GetEnvironmentVariable('RATMAC_TEST_TURN_PAUSE_RELEASE')
    $timeoutText = [Environment]::GetEnvironmentVariable('RATMAC_TEST_TURN_PAUSE_TIMEOUT_MILLIS')
    if ([string]::IsNullOrEmpty($marker)) {
        Deny -Reason 'the test pause request is invalid: RATMAC_TEST_TURN_PAUSE_MARKER is missing' -Guidance @(
            'set RATMAC_TEST_TURN_PAUSE to a known boundary with its marker, release, and timeout companions, or unset it.'
        )
    }
    if ([string]::IsNullOrEmpty($release)) {
        Deny -Reason 'the test pause request is invalid: RATMAC_TEST_TURN_PAUSE_RELEASE is missing' -Guidance @(
            'set RATMAC_TEST_TURN_PAUSE to a known boundary with its marker, release, and timeout companions, or unset it.'
        )
    }
    $timeout = 0
    if ([string]::IsNullOrEmpty($timeoutText) -or -not [int]::TryParse($timeoutText, [ref]$timeout) -or $timeout -le 0) {
        Deny -Reason "the test pause request is invalid: RATMAC_TEST_TURN_PAUSE_TIMEOUT_MILLIS is not a positive integer: '$timeoutText'" -Guidance @(
            'set RATMAC_TEST_TURN_PAUSE to a known boundary with its marker, release, and timeout companions, or unset it.'
        )
    }

    [pscustomobject]@{ Boundary = $requested; Marker = $marker; Release = $release; TimeoutMillis = $timeout }
}

# Hold the verb at a proven boundary: the marker line lands in one step,
# then the verb waits for the release path, refusing on timeout with that
# boundary's disk state left exactly as it is.
function Invoke-TestPause {
    param([Parameter(Mandatory)] $Pause)
    try {
        [System.IO.File]::WriteAllText($Pause.Marker, "$($Pause.Boundary)`n", [System.Text.UTF8Encoding]::new($false))
    }
    catch {
        Deny -Reason "the test pause request is invalid: RATMAC_TEST_TURN_PAUSE_MARKER is not a writable file path: '$($Pause.Marker)'" -Guidance @(
            'set RATMAC_TEST_TURN_PAUSE to a known boundary with its marker, release, and timeout companions, or unset it.'
        )
    }
    $deadline = [DateTime]::UtcNow.AddMilliseconds($Pause.TimeoutMillis)
    while (-not (Test-Path -LiteralPath $Pause.Release -PathType Leaf)) {
        if ([DateTime]::UtcNow -ge $deadline) {
            Deny -Reason "injected turn pause at $($Pause.Boundary) expired"
        }
        Start-Sleep -Milliseconds 50
    }
}

# Git's raw stdout bytes into a file, so a content digest never passes
# through a decoded PowerShell string; stderr lands in a sibling file the
# caller's temporary directory owns and takes with it.
function Save-GitBytes {
    param(
        [Parameter(Mandatory)][string[]] $Arguments,
        [Parameter(Mandatory)][string] $Destination
    )
    $errorFile = "$Destination.err"
    & git @Arguments 1> $Destination 2> $errorFile
    $code = $LASTEXITCODE
    $text = ''
    if (Test-Path -LiteralPath $errorFile -PathType Leaf) {
        $text = [System.IO.File]::ReadAllText($errorFile).TrimEnd("`r", "`n")
        [System.IO.File]::Delete($errorFile)
    }
    [pscustomobject]@{ ExitCode = $code; Text = $text; Ok = ($code -eq 0) }
}

# The effective configuration value a manifest line records: the value Git
# answers, or `unset` when no scope configures it.
function Get-ConfigFact {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)][string] $Name
    )
    $result = Invoke-Git -Arguments @('-C', $Facts.RepoRoot, 'config', $Name)
    if ($result.Ok) { $result.Text.Trim() } else { 'unset' }
}

# The effective text, eol, and filter attribute values Git spells for a
# path, read through `-z`: its NUL-separated records carry the path
# verbatim whatever a host's quoting rule would escape, and the records
# come from git's raw bytes, so no console decoding touches them.
function Get-AttrFacts {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][string] $ProofDirectory
    )
    $attrFile = Join-Path $ProofDirectory 'attr-bytes'
    $result = Save-GitBytes -Arguments @('-C', $Facts.RepoRoot, 'check-attr', '-z', 'text', 'eol', 'filter', '--', $Path) -Destination $attrFile
    if (-not $result.Ok) {
        Deny -Reason "git -C $($Facts.RepoRoot) check-attr -z text eol filter -- $Path failed: $($result.Text)"
    }
    $raw = [System.IO.File]::ReadAllText($attrFile, [System.Text.UTF8Encoding]::new($false))
    $parts = @($raw -split "`0")
    $found = @{}
    for ($index = 0; $index + 2 -lt $parts.Count; $index += 3) {
        $found[$parts[$index + 1]] = $parts[$index + 2]
    }
    $values = [System.Collections.Generic.List[string]]::new()
    foreach ($attr in 'text', 'eol', 'filter') {
        if (-not $found.ContainsKey($attr)) {
            Deny -Reason "git -C $($Facts.RepoRoot) check-attr -z text eol filter -- $Path failed: unexpected output without a $attr row"
        }
        $values.Add([string]$found[$attr])
    }
    [string[]]$values.ToArray()
}

# Every -Target spelling: each value splits on commas, empty pieces drop,
# and exact duplicate spellings collapse to one.
function Get-CheckpointTargets {
    $seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    $targets = [System.Collections.Generic.List[string]]::new()
    foreach ($value in @($Target)) {
        foreach ($piece in ($value -split ',')) {
            if ($piece -ne '' -and $seen.Add($piece)) { $targets.Add($piece) }
        }
    }
    if ($targets.Count -eq 0) {
        Deny -Reason 'checkpoint needs -Target <path>...' -Guidance @(
            'declare every repository-relative path deliberate damage may mutate; at least one is required.'
        )
    }
    [string[]]$targets.ToArray()
}

# A repository-relative regular-file spelling: no drive, no leading '/', no
# backslash, no '..' component, no tab, no newline.
function Test-TargetSpelling {
    param([Parameter(Mandatory)][string] $Path)
    ($Path -cmatch '^[A-Za-z]:') -or
        $Path.StartsWith('/') -or
        $Path.Contains('\') -or
        (@($Path -split '/') -contains '..') -or
        $Path.Contains("`t") -or
        $Path.Contains("`n")
}

# The manifest's row order: paths sorted bytewise ascending - the UTF-8
# bytes of each spelling compared unit by unit, so the order holds beyond
# the basic multilingual plane and never depends on a culture or on UTF-16
# code-unit order.
function Get-BytewiseAscendingPaths {
    param([Parameter(Mandatory)][string[]] $Paths)
    Sort-Bytewise -Lines $Paths
}

# The round-trip proof for one target, through Git's own conversion both
# ways: checking the recorded blob out through the effective settings
# reproduces the current worktree bytes, and cleaning the current worktree
# bytes reproduces the recorded blob. The proof's own bytes live outside
# the repository, in the caller's temporary directory.
function Test-TargetRoundTrip {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)][string] $Head,
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][string] $ProofDirectory
    )
    $blob = (Get-GitText -Arguments @('-C', $Facts.RepoRoot, 'rev-parse', "$($Head):$Path")).Trim()
    $checkoutFile = Join-Path $ProofDirectory 'checkout-bytes'
    $checkout = Save-GitBytes -Arguments @('-C', $Facts.RepoRoot, 'cat-file', '--filters', "--path=$Path", "$($Head):$Path") -Destination $checkoutFile
    if (-not $checkout.Ok) {
        Deny -Reason "git -C $($Facts.RepoRoot) cat-file --filters --path=$Path $($Head):$Path failed: $($checkout.Text)"
    }
    $cleaned = (Get-GitText -Arguments @('-C', $Facts.RepoRoot, 'hash-object', "--path=$Path", '--', $Path)).Trim()
    $proven = $false
    try {
        $checkoutDigest = (Get-FileHash -LiteralPath $checkoutFile -Algorithm SHA256).Hash.ToLowerInvariant()
        $worktreeDigest = (Get-FileHash -LiteralPath (Join-Path $Facts.RepoRoot $Path) -Algorithm SHA256).Hash.ToLowerInvariant()
        $proven = ($checkoutDigest -eq $worktreeDigest) -and ($cleaned -eq $blob)
    }
    catch {
        $proven = $false
    }
    $proven
}

# The commit's subject as Git itself renders it: the first line of
# `git log -1 --format=%s <commit>`, whose rendering drops trailing
# whitespace. The safety comparison is byte-exact against the expected
# string, with no further trimming and no case folding; the refusal
# renders it verbatim, empty when the subject is empty.
function Get-CommitSubject {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)][string] $Commit
    )
    [string](@((Get-GitText -Arguments @('-C', $Facts.RepoRoot, 'log', '-1', '--format=%s', $Commit)) -split "`n")[0])
}

function Invoke-Checkpoint {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl,
        $Pause
    )

    # Validate, in the contract's order; every refusal writes nothing.
    # @() keeps a single spelling an array: a one-element [string[]] would
    # unroll to a scalar through the pipeline and lose .Count under StrictMode.
    $targets = @(Get-CheckpointTargets)
    foreach ($path in $targets) {
        if (Test-TargetSpelling -Path $path) {
            Deny -Reason "the declared target $path must stay repository-relative" -Guidance @(
                "a target is one repository-relative path: no drive, no leading '/', no backslash, no '..' component, no tab, no newline."
            )
        }
    }
    $recordsRoot = Join-Path $Facts.RepoRoot $Decl.ItemRecords
    if (-not (Test-Path -LiteralPath $recordsRoot -PathType Container)) {
        Deny -Reason "the declared item-records root $($Decl.ItemRecords) does not exist" -Guidance @(
            'create it, or fix item-records in .ratmac/turn.toml.'
        )
    }
    $head = (Get-GitText -Arguments @('-C', $Facts.RepoRoot, 'rev-parse', 'HEAD')).Trim()
    $subject = Get-CommitSubject -Facts $Facts -Commit $head
    if (-not [string]::Equals($subject, "${Item}: checkpoint - not a landing", [System.StringComparison]::Ordinal)) {
        Deny -Reason "HEAD $head is not the safety commit of ${Item}: its subject is $subject" -Guidance @(
            "the working rules fix the safety commit's subject exactly '${Item}: checkpoint - not a landing';",
            'commit the reviewed green work under that subject, then run checkpoint again.',
            'nothing was written.'
        )
    }
    foreach ($path in $targets) {
        $kind = Invoke-Git -Arguments @('-C', $Facts.RepoRoot, 'cat-file', '-t', "$($head):$path")
        if (-not $kind.Ok -or $kind.Text.Trim() -ne 'blob') {
            Deny -Reason "the declared target $path is not a tracked regular file at $head" -Guidance @(
                'damage may be planned only for targets the checkpoint commit tracks;',
                'commit the file first, or drop it from -Target.'
            )
        }
    }
    $trackedDirty = @($Facts.Porcelain | Where-Object { -not $_.StartsWith('??') })
    if ($trackedDirty.Count -gt 0) {
        Deny -Reason 'the working tree must be clean before a checkpoint freezes it; commit or put aside these entries first:' -Guidance (
            @($trackedDirty | ForEach-Object { "  $_" })
        )
    }

    # WCP-004: the freeze takes only reviewed, proven green work - the gate
    # sits after the clean-tracked-tree row and before the round-trip proof,
    # and every gate row refuses before any write.
    Invoke-ReviewGate -Facts $Facts -Decl $Decl

    # The manifest's rows, bytewise ascending by path.
    $ordered = @(Get-BytewiseAscendingPaths -Paths $targets)

    # The facts the manifest records.
    $autocrlf = Get-ConfigFact -Facts $Facts -Name 'core.autocrlf'
    $coreEol = Get-ConfigFact -Facts $Facts -Name 'core.eol'

    # Prove both directions before anything is written; the proof's
    # temporary bytes live outside the repository and are removed before
    # the verb ends, refusal or not.
    $proofDirectory = Join-Path ([System.IO.Path]::GetTempPath()) ("ratmac-turn-proof-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $proofDirectory -Force | Out-Null
    $rows = [System.Collections.Generic.List[string]]::new()
    try {
        foreach ($path in $ordered) {
            if (-not (Test-TargetRoundTrip -Facts $Facts -Head $head -Path $path -ProofDirectory $proofDirectory)) {
                Deny -Reason "the checkout round-trip for $path is not proven" -Guidance @(
                    'checking the recorded blob out through the effective settings must reproduce the current bytes, and cleaning the current bytes must reproduce the recorded blob;',
                    'the effective line-ending or checkout-filter settings cannot reproduce these bytes after a restore; fix the attributes, filters, or core.autocrlf and core.eol, or correct the file, then run checkpoint again.',
                    'nothing was written.'
                )
            }
            $digest = (Get-FileHash -LiteralPath (Join-Path $Facts.RepoRoot $path) -Algorithm SHA256).Hash.ToLowerInvariant()
            $blob = (Get-GitText -Arguments @('-C', $Facts.RepoRoot, 'rev-parse', "$($head):$path")).Trim()
            $attrs = Get-AttrFacts -Facts $Facts -Path $path -ProofDirectory $proofDirectory
            $rows.Add("target $path`t$digest`t$blob`t$($attrs[0])`t$($attrs[1])`t$($attrs[2])")
        }
    }
    finally {
        # The proof's own temporary bytes, outside the repository; a host
        # that cannot take them away never fails the verb on it.
        $null = try { [System.IO.Directory]::Delete($proofDirectory, $true) } catch { }
    }

    # Freeze: the whole manifest in one step, beside the item record.
    $manifestRelative = ("$($Decl.ItemRecords)/$Item-checkpoint-manifest.txt" -replace '\\', '/')
    $manifestPath = Join-Path $recordsRoot "$Item-checkpoint-manifest.txt"
    $content = "item $Item`ncommit $head`nsetting core.autocrlf $autocrlf`nsetting core.eol $coreEol`n"
    foreach ($row in $rows) { $content += "$row`n" }
    $temporaryPath = "$manifestPath.tmp"
    try {
        [System.IO.File]::WriteAllText($temporaryPath, $content, [System.Text.UTF8Encoding]::new($false))
        [System.IO.File]::Move($temporaryPath, $manifestPath, $true)
    }
    catch {
        Deny -Reason "writing the checkpoint manifest $manifestRelative failed: $($_.Exception.Message)" -Guidance @(
            'nothing was frozen; clear the path and run checkpoint again.'
        )
    }

    if ($null -ne $Pause) { Invoke-TestPause -Pause $Pause }

    $noun = if ($ordered.Count -eq 1) { 'target' } else { 'targets' }
    Write-Report 'turn checkpoint frozen'
    Write-Report "  item:      $Item"
    Write-Report "  commit:    $head"
    Write-Report "  manifest:  $manifestRelative ($($ordered.Count) $noun, round-trip proven)"
    Write-Report "  damage:    mutate only the declared targets, then run: pwsh -File tools/turn.ps1 restore -Item $Item"
    exit 0
}

# The manifest's malformed-refusal row, shared by every parse fact.
function Deny-Manifest {
    param(
        [Parameter(Mandatory)][string] $Relative,
        [Parameter(Mandatory)][string] $Fact
    )
    Deny -Reason "the checkpoint manifest $Relative is malformed: $Fact" -Guidance @(
        "repair the manifest, or re-run checkpoint -Item $Item from the green state."
    )
}

# The frozen manifest, parsed to its exact format: four header rows and one
# six-field tab-separated target row per declared path. Anything else
# refuses as malformed, the bytes left exactly as they read.
function Read-CheckpointManifest {
    param(
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][string] $Relative
    )
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        Deny-Manifest -Relative $Relative -Fact 'it carries a UTF-8 byte-order mark'
    }
    $text = $null
    try {
        $text = [System.Text.UTF8Encoding]::new($false, $true).GetString($bytes)
    }
    catch {
        Deny-Manifest -Relative $Relative -Fact 'it is not valid UTF-8'
    }
    if ($text.Contains("`r")) {
        Deny-Manifest -Relative $Relative -Fact 'a line carries a carriage return'
    }
    if (-not $text.EndsWith("`n")) {
        Deny-Manifest -Relative $Relative -Fact 'the last line is not LF-terminated'
    }
    $split = @($text -split "`n")
    $lines = @($split | Select-Object -First ($split.Count - 1))
    if ($lines.Count -lt 5) {
        Deny-Manifest -Relative $Relative -Fact "it carries $($lines.Count) lines, fewer than the four header rows and one target row"
    }
    foreach ($line in $lines) {
        if ($line -eq '') {
            Deny-Manifest -Relative $Relative -Fact 'it carries an empty line'
        }
    }
    if (-not ($lines[0] -cmatch '^item (.+)$')) {
        Deny-Manifest -Relative $Relative -Fact "the first line is not an item row: $($lines[0])"
    }
    $manifestItem = $Matches[1]
    if (-not ($lines[1] -cmatch '^commit ([0-9a-f]{40})$')) {
        Deny-Manifest -Relative $Relative -Fact "the commit row is not a 40-hex object id: $($lines[1])"
    }
    $commit = $Matches[1]
    if (-not ($lines[2] -cmatch '^setting core\.autocrlf (.*)$')) {
        Deny-Manifest -Relative $Relative -Fact "the core.autocrlf row is malformed: $($lines[2])"
    }
    $autocrlf = $Matches[1]
    if (-not ($lines[3] -cmatch '^setting core\.eol (.*)$')) {
        Deny-Manifest -Relative $Relative -Fact "the core.eol row is malformed: $($lines[3])"
    }
    $coreEol = $Matches[1]
    $rows = [System.Collections.Generic.List[object]]::new()
    for ($index = 4; $index -lt $lines.Count; $index++) {
        if (-not ($lines[$index] -cmatch '^target ([^\t]+)\t([0-9a-f]{64})\t([0-9a-f]{40})\t([^\t]+)\t([^\t]+)\t([^\t]+)$')) {
            Deny-Manifest -Relative $Relative -Fact "a target row is not the six-field tab-separated shape: $($lines[$index])"
        }
        $rows.Add([pscustomobject]@{
            Path   = $Matches[1]
            Digest = $Matches[2]
            Blob   = $Matches[3]
            Text   = $Matches[4]
            Eol    = $Matches[5]
            Filter = $Matches[6]
        })
    }
    if ($rows.Count -eq 0) {
        Deny-Manifest -Relative $Relative -Fact 'it carries no target row'
    }
    [pscustomobject]@{
        Item     = $manifestItem
        Commit   = $commit
        Autocrlf = $autocrlf
        CoreEol  = $coreEol
        Rows     = @($rows.ToArray())
    }
}

# A manifest spelling's plain repository form: empty and `.` components
# dropped, so `./a.txt` names `a.txt` - the form Git's own rows and index
# entries carry. The manifest itself keeps the spelling it was given.
function Get-PlainPath {
    param([Parameter(Mandatory)][string] $Path)
    (@($Path -split '/') | Where-Object { $_ -ne '' -and $_ -ne '.' }) -join '/'
}

# One porcelain row side as Git names it: a C-quoted side (it starts with
# `"`) is decoded - Git's `\a \b \t \n \v \f \r \" \\` and three-digit
# octal byte escapes - into bytes, then UTF-8. A side that cannot be read
# that way returns $null, and the caller counts its row as unrelated work:
# the guard fails closed on bytes it cannot name. An unquoted side is the
# path verbatim.
function ConvertFrom-GitRowPath {
    param([Parameter(Mandatory)][string] $Side)
    if (-not $Side.StartsWith('"')) { return $Side }
    if ($Side.Length -lt 2 -or -not $Side.EndsWith('"')) { return $null }
    $inner = $Side.Substring(1, $Side.Length - 2)
    $escapes = @{
        a = 7;  b = 8;  t = 9;  n = 10
        v = 11; f = 12; r = 13; '"' = 34; '\' = 92
    }
    $bytes = [System.Collections.Generic.List[byte]]::new()
    $index = 0
    while ($index -lt $inner.Length) {
        if ($inner[$index] -ne '\') {
            $bytes.Add([byte][char]$inner[$index])
            $index++
            continue
        }
        $index++
        if ($index -ge $inner.Length) { return $null }
        $escape = [string]$inner[$index]
        if ($escapes.ContainsKey($escape)) {
            $bytes.Add([byte]$escapes[$escape])
            $index++
            continue
        }
        $digits = ''
        while ($digits.Length -lt 3 -and $index -lt $inner.Length) {
            $octal = [int]$inner[$index]
            if ($octal -lt 48 -or $octal -gt 55) { break }
            $digits += $inner[$index]
            $index++
        }
        if ($digits.Length -eq 0) { return $null }
        $bytes.Add([byte][Convert]::ToInt32($digits, 8))
    }
    try {
        $text = [System.Text.UTF8Encoding]::new($false, $true).GetString([byte[]]$bytes.ToArray())
    }
    catch {
        return $null
    }
    if ($text.Contains([char]0)) { return $null }
    $text
}

# A rename or copy row's two sides, split around ` -> ` with quoted sides
# honored: the separator is the first ` -> ` outside every quoted span,
# and no other split is made.
function Split-GitRowSides {
    param([Parameter(Mandatory)][string] $Rest)
    $inQuotes = $false
    $escaped = $false
    for ($index = 0; $index -le $Rest.Length - 4; $index++) {
        $glyph = $Rest[$index]
        if ($escaped) { $escaped = $false; continue }
        if ($glyph -eq '\') { $escaped = $true; continue }
        if ($glyph -eq '"') { $inQuotes = -not $inQuotes; continue }
        if (-not $inQuotes -and $glyph -eq ' ' -and $Rest.Substring($index, 4) -ceq ' -> ') {
            return @($Rest.Substring(0, $index), $Rest.Substring($index + 4))
        }
    }
    @($Rest)
}

function Invoke-Restore {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl,
        $Pause
    )

    $manifestRelative = ("$($Decl.ItemRecords)/$Item-checkpoint-manifest.txt" -replace '\\', '/')
    $manifestPath = Join-Path $Facts.RepoRoot "$($Decl.ItemRecords)/$Item-checkpoint-manifest.txt"
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        Deny -Reason "the checkpoint manifest $manifestRelative does not exist" -Guidance @(
            "run checkpoint -Item $Item from the green state before restoring."
        )
    }
    $manifest = Read-CheckpointManifest -Path $manifestPath -Relative $manifestRelative
    if ($manifest.Item -cne $Item) {
        Deny -Reason "the checkpoint manifest $manifestRelative records item $($manifest.Item), not $Item" -Guidance @(
            "re-run checkpoint -Item $Item from the green state."
        )
    }
    $commit = $manifest.Commit
    if (-not (Test-RefExists -Ref $commit)) {
        Deny -Reason "the recorded checkpoint commit $commit does not resolve in this repository" -Guidance @(
            'the manifest was frozen in another repository, or the commit was removed;',
            'inspect the manifest before freezing again.'
        )
    }
    $headId = (Get-GitText -Arguments @('-C', $Facts.RepoRoot, 'rev-parse', 'HEAD')).Trim()
    if ($headId -cne $commit) {
        Deny -Reason "the recorded checkpoint $commit is stale: HEAD is $headId" -Guidance @(
            'the damage protocol never moves HEAD; this manifest does not name the commit the checkout stands on.',
            "re-run checkpoint -Item $Item from the current safety commit, or inspect the manifest before restoring.",
            'nothing was written.'
        )
    }
    $headSubject = Get-CommitSubject -Facts $Facts -Commit $headId
    if (-not [string]::Equals($headSubject, "${Item}: checkpoint - not a landing", [System.StringComparison]::Ordinal)) {
        Deny -Reason "HEAD $headId is not the safety commit of ${Item}: its subject is $headSubject" -Guidance @(
            "the working rules fix the safety commit's subject exactly '${Item}: checkpoint - not a landing';",
            "restore only from a manifest the tool froze from that commit; re-run checkpoint -Item $Item, or repair the manifest.",
            'nothing was written.'
        )
    }
    foreach ($row in $manifest.Rows) {
        $kind = Invoke-Git -Arguments @('-C', $Facts.RepoRoot, 'cat-file', '-t', "$($commit):$($row.Path)")
        if (-not $kind.Ok -or $kind.Text.Trim() -ne 'blob') {
            Deny -Reason "the manifest names $($row.Path), which the recorded checkpoint commit $commit does not carry" -Guidance @(
                "the manifest and the commit disagree; re-run checkpoint -Item $Item from the green state, or repair the manifest."
            )
        }
        $recomputed = (Get-GitText -Arguments @('-C', $Facts.RepoRoot, 'rev-parse', "$($commit):$($row.Path)")).Trim()
        if ($recomputed -cne $row.Blob) {
            Deny -Reason "the manifest row for $($row.Path) does not match the recorded checkpoint $($commit): blob $recomputed where the manifest carries $($row.Blob)" -Guidance @(
                'the manifest was edited, or it records a different commit;',
                "re-run checkpoint -Item $Item from the green state, or repair the manifest to the recorded facts."
            )
        }
    }

    if ($null -ne $Pause -and $Pause.Boundary -ceq 'after-validate') { Invoke-TestPause -Pause $Pause }

    # Guard the tree, after the pause and before the first mutation:
    # every non-`??` row must be a declared damage row - a row every path
    # it names is a manifest target, a rename row naming both its paths -
    # so target bytes mutated, staged, deleted, or renamed by the damage
    # protocol are all expected, while any unrelated unsaved work refuses
    # with everything exactly as found. The rows are read fresh here, so
    # dirt planted while the invocation was paused is caught. Each row's
    # paths are read as Git names them - a C-quoted side decoded, both
    # sides of a rename or copy row split around ` -> ` - and compared
    # against each manifest spelling's plain repository form; a side the
    # tool cannot read counts as unrelated, so the guard fails closed.
    # The refusal shows each row exactly as Git printed it.
    $targetSet = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    foreach ($row in $manifest.Rows) { $null = $targetSet.Add((Get-PlainPath -Path $row.Path)) }
    $unrelated = [System.Collections.Generic.List[string]]::new()
    foreach ($row in @(Get-GitLines -Arguments @('-C', $Facts.RepoRoot, 'status', '--porcelain'))) {
        if ($row.StartsWith('??')) { continue }
        $code = $row.Substring(0, 2)
        $rest = $row.Substring(3)
        $sides = if ($code -cmatch '[RC]') { @(Split-GitRowSides -Rest $rest) } else { @($rest) }
        $isDamage = $true
        foreach ($side in $sides) {
            $path = ConvertFrom-GitRowPath -Side $side
            if ($null -eq $path -or -not $targetSet.Contains($path)) { $isDamage = $false }
        }
        if (-not $isDamage) { $unrelated.Add($row) }
    }
    if ($unrelated.Count -gt 0) {
        Deny -Reason 'the working tree carries unsaved work unrelated to the declared targets; commit or put aside these entries first:' -Guidance (
            @($unrelated) +
            @('a restore may undo only the declared targets'' damage;', 'nothing was written.')
        )
    }

    $paths = @($manifest.Rows | ForEach-Object { Get-PlainPath -Path $_.Path })

    # Each target's index entry re-set to the checkpoint blob with no
    # cached file facts: an entry whose stat still matches bytes an earlier
    # setting wrote would make the restore skip that target, so fixing the
    # settings a digest-mismatch refusal named and re-running restore would
    # refuse the same way forever. One invocation for every target, in
    # manifest order, through the modes and blobs the recorded commit
    # carries; the worktree bytes are untouched.
    $proofDirectory = Join-Path ([System.IO.Path]::GetTempPath()) "ratmac-turn-restore-$([guid]::NewGuid().ToString('N'))"
    $null = New-Item -ItemType Directory -Path $proofDirectory -Force
    try {
        $treeFile = Join-Path $proofDirectory 'tree-bytes'
        $tree = Save-GitBytes -Arguments (@('-C', $Facts.RepoRoot, 'ls-tree', '-z', $commit, '--') + $paths) -Destination $treeFile
        if (-not $tree.Ok) {
            Deny -Reason "git -C $($Facts.RepoRoot) ls-tree -z $commit -- $($paths -join ' ') failed: $($tree.Text)"
        }
        $entries = @{}
        foreach ($entry in @([System.IO.File]::ReadAllText($treeFile, [System.Text.UTF8Encoding]::new($false)) -split "`0")) {
            if ($entry.Length -eq 0) { continue }
            $tab = $entry.IndexOf("`t")
            $head = $entry.Substring(0, $tab) -split ' '
            $entries[$entry.Substring($tab + 1)] = @{ Mode = $head[0]; Object = $head[2] }
        }
        $triples = [System.Collections.Generic.List[string]]::new()
        $resetArgs = [System.Collections.Generic.List[string]]::new([string[]]@('-C', $Facts.RepoRoot, 'update-index', '--add'))
        foreach ($row in $manifest.Rows) {
            $plain = Get-PlainPath -Path $row.Path
            $entry = $entries[$plain]
            if ($null -eq $entry -or $entry.Object -cne $row.Blob) {
                Deny -Reason "git -C $($Facts.RepoRoot) ls-tree -z $commit -- $($paths -join ' ') failed: unexpected output without $($row.Path) at its recorded blob"
            }
            $triples.Add("$($entry.Mode),$($row.Blob),$plain")
            $resetArgs.Add('--cacheinfo')
            $resetArgs.Add($triples[$triples.Count - 1])
        }
        $reset = Invoke-Git -Arguments $resetArgs.ToArray()
        if (-not $reset.Ok) {
            Deny -Reason "git update-index --add --cacheinfo $($triples -join ' ') failed: $($reset.Text)"
        }

    # Exactly the working rules' undo, one invocation for every target.
        $restore = Invoke-Git -Arguments (@('-C', $Facts.RepoRoot, 'restore', "--source=$commit", '--staged', '--worktree', '--') + $paths)
        if (-not $restore.Ok) {
            Deny -Reason "git restore --source=$commit --staged --worktree -- $($paths -join ' ') failed: $($restore.Text)"
        }
    }
    finally {
        # The verb's own temporary bytes, outside the repository; a host
        # that cannot take them away never fails the verb on it.
        $null = try { [System.IO.Directory]::Delete($proofDirectory, $true) } catch { }
    }

    if ($null -ne $Pause -and $Pause.Boundary -ceq 'after-restore') { Invoke-TestPause -Pause $Pause }

    # Verify: every recomputed digest against its frozen digest, then a
    # clean tracked tree. Both must hold; nothing is deleted on failure and
    # no unrelated file is touched.
    foreach ($row in $manifest.Rows) {
        $digest = ''
        try {
            $digest = (Get-FileHash -LiteralPath (Join-Path $Facts.RepoRoot $row.Path) -Algorithm SHA256).Hash.ToLowerInvariant()
        }
        catch {
            Deny -Reason "recomputing the digest of $($row.Path) failed: $($_.Exception.Message)" -Guidance @(
                'free or repair the path, then run restore again.'
            )
        }
        if ($digest -cne $row.Digest) {
            Deny -Reason "the restored bytes of $($row.Path) do not match its frozen digest" -Guidance @(
                'a clean Git status is not acceptance: the effective line-ending or checkout-filter settings changed since the checkpoint froze these bytes;',
                "restore the settings the manifest records for $($row.Path) (text $($row.Text), eol $($row.Eol), filter $($row.Filter); core.autocrlf $($manifest.Autocrlf), core.eol $($manifest.CoreEol)), then run restore again;",
                'no unrelated file was touched.'
            )
        }
    }
    $trackedDirty = @(Get-GitLines -Arguments @('-C', $Facts.RepoRoot, 'status', '--porcelain') | Where-Object { -not $_.StartsWith('??') })
    if ($trackedDirty.Count -gt 0) {
        Deny -Reason 'the tracked tree is not clean after the restore' -Guidance (
            @($trackedDirty) +
            @("the targets' digests matched, but the index or a tracked path still differs; repair the rows above, then run restore again.")
        )
    }

    if ($null -ne $Pause -and $Pause.Boundary -ceq 'after-verify') { Invoke-TestPause -Pause $Pause }

    Write-Report 'turn restore verified'
    Write-Report "  item:      $Item"
    Write-Report "  commit:    $commit"
    Write-Report "  manifest:  $manifestRelative"
    Write-Report "  targets:   $($manifest.Rows.Count) restored; every recomputed digest matches its frozen digest; index and tracked tree clean"
    exit 0
}

# --- review and proofs (WCP-004) ---------------------------------------------

# Strings sorted bytewise ascending: the UTF-8 bytes of each whole line
# compared unit by unit, so the order holds beyond the basic multilingual
# plane and never depends on a culture or on UTF-16 code-unit order - the
# snapshot's one sort over its rendered rows and the changed-input rows'
# order alike.
function Sort-Bytewise {
    param([Parameter(Mandatory)][AllowEmptyCollection()][string[]] $Lines)
    $list = [System.Collections.Generic.List[string]]::new([string[]]$Lines)
    $list.Sort([System.Comparison[string]]{
        param($Left, $Right)
        $leftBytes = [System.Text.UTF8Encoding]::new($false).GetBytes($Left)
        $rightBytes = [System.Text.UTF8Encoding]::new($false).GetBytes($Right)
        $shared = [Math]::Min($leftBytes.Length, $rightBytes.Length)
        for ($index = 0; $index -lt $shared; $index++) {
            if ($leftBytes[$index] -lt $rightBytes[$index]) { return -1 }
            if ($leftBytes[$index] -gt $rightBytes[$index]) { return 1 }
        }
        if ($leftBytes.Length -lt $rightBytes.Length) { return -1 }
        if ($leftBytes.Length -gt $rightBytes.Length) { return 1 }
        return 0
    })
    [string[]]$list.ToArray()
}

# The addressed item's own evidence under the declared item-records root,
# each name with its `.tmp` staging sibling: never rows of the reviewed
# inputs, whatever their tracked state, so no file a verb of this family
# writes can ever change a snapshot digest.
function Get-ExcludedEvidencePaths {
    param([Parameter(Mandatory)] $Decl)
    $excluded = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    $recordsPrefix = (("$($Decl.ItemRecords)" -replace '\\', '/')).TrimEnd('/')
    foreach ($name in "$Item-review-record.txt", "$Item-proof-receipt.txt", "$Item-checkpoint-manifest.txt") {
        $null = $excluded.Add("$recordsPrefix/$name")
        $null = $excluded.Add("$recordsPrefix/$name.tmp")
    }
    $excluded
}

# The lane half of the reviewed inputs: every file under the declared lanes
# root, skipping the declared build-output names at any depth and `.git`,
# rendered `lane <path>\t<sha256>` - the rows the tree walker's own walk
# emits, walked here one file at a time so an unreadable path refuses by
# name.
function Add-LaneRows {
    param(
        [Parameter(Mandatory)][string] $Directory,
        [AllowEmptyString()][string] $Prefix = '',
        [Parameter(Mandatory)][string[]] $SkipNames,
        [Parameter(Mandatory)][string] $VerbName,
        [Parameter(Mandatory)] $Rows
    )
    foreach ($entry in @(Get-ChildItem -LiteralPath $Directory -Force)) {
        if ($entry.PSIsContainer) {
            if ($SkipNames -contains $entry.Name) { continue }
            if ($entry.Name -eq '.git') { continue }
            Add-LaneRows -Directory $entry.FullName -Prefix ($Prefix + $entry.Name + '/') -SkipNames $SkipNames -VerbName $VerbName -Rows $Rows
        }
        else {
            $digest = ''
            try {
                $digest = (Get-FileHash -LiteralPath $entry.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            }
            catch {
                Deny -Reason "recomputing the digest of $($Prefix + $entry.Name) failed: $($_.Exception.Message)" -Guidance @(
                    "free or repair the path, then run $VerbName again."
                )
            }
            $Rows.Add("lane $($Prefix + $entry.Name)`t$digest")
        }
    }
}

# One `git ls-files` listing, read from Git's own raw stdout bytes as
# UTF-8 text, so a non-ASCII path never passes through a console decoding
# and the row spells the path as the working tree carries it.
function Get-GitListingLines {
    param(
        [Parameter(Mandatory)][string[]] $Arguments,
        [Parameter(Mandatory)][string] $Destination
    )
    $result = Save-GitBytes -Arguments $Arguments -Destination $Destination
    if (-not $result.Ok) {
        Deny -Reason "git $($Arguments -join ' ') failed: $($result.Text)"
    }
    $text = [System.IO.File]::ReadAllText($Destination, [System.Text.UTF8Encoding]::new($false))
    if ([string]::IsNullOrWhiteSpace($text)) { return @() }
    @($text -split "`r?`n" | Where-Object { $_ -ne '' })
}

# The reviewed inputs of the addressed checkout and their one snapshot
# digest: every work row (a path the index lists or one untracked
# non-ignored path, minus the addressed item's own evidence) plus every
# lane row, all rows together sorted bytewise ascending over the whole
# rendered line. A path the index lists but the working tree does not carry
# refuses the incomplete row; an unreadable path refuses the digest-failure
# row naming the invoking verb - the same bytes for both verbs that walk
# the rows.
function Get-ReviewedSnapshot {
    param(
        [Parameter(Mandatory)][string] $VerbName,
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl
    )
    $excluded = Get-ExcludedEvidencePaths -Decl $Decl
    $seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    $workPaths = [System.Collections.Generic.List[string]]::new()
    # The two listings, read from Git's raw bytes with quotepath off, live
    # in one temporary directory outside the repository, removed before
    # the verb ends, refusal or not.
    $listDirectory = Join-Path ([System.IO.Path]::GetTempPath()) ("ratmac-turn-snapshot-" + [guid]::NewGuid().ToString('N'))
    $null = New-Item -ItemType Directory -Path $listDirectory -Force
    try {
        $cached = @(Get-GitListingLines -Arguments @('-c', 'core.quotepath=false', '-C', $Facts.RepoRoot, 'ls-files', '--cached') -Destination (Join-Path $listDirectory 'cached'))
        foreach ($path in $cached) {
            if ($excluded.Contains($path)) { continue }
            if ($seen.Add($path)) { $workPaths.Add($path) }
        }
        foreach ($path in @(Get-GitListingLines -Arguments @('-c', 'core.quotepath=false', '-C', $Facts.RepoRoot, 'ls-files', '--others', '--exclude-standard') -Destination (Join-Path $listDirectory 'others'))) {
            if ($excluded.Contains($path)) { continue }
            if ($seen.Add($path)) { $workPaths.Add($path) }
        }
    }
    finally {
        $null = try { [System.IO.Directory]::Delete($listDirectory, $true) } catch { }
    }
    foreach ($path in $cached) {
        if ($excluded.Contains($path)) { continue }
        if (-not (Test-Path -LiteralPath (Join-Path $Facts.RepoRoot $path) -PathType Leaf)) {
            Deny -Reason "the reviewed snapshot is incomplete: $path is in the index but missing from the working tree" -Guidance @(
                'restore or remove the path, then record the review again.',
                'nothing was recorded.'
            )
        }
    }

    $rows = [System.Collections.Generic.List[string]]::new()
    foreach ($path in $workPaths) {
        $digest = ''
        try {
            $digest = (Get-FileHash -LiteralPath (Join-Path $Facts.RepoRoot $path) -Algorithm SHA256).Hash.ToLowerInvariant()
        }
        catch {
            Deny -Reason "recomputing the digest of $path failed: $($_.Exception.Message)" -Guidance @(
                "free or repair the path, then run $VerbName again."
            )
        }
        $rows.Add("work $path`t$digest")
    }
    $lanesRoot = Join-Path $Facts.RepoRoot $Decl.Lanes
    if (Test-Path -LiteralPath $lanesRoot -PathType Container) {
        Add-LaneRows -Directory $lanesRoot -Prefix '' -SkipNames $Decl.Skip -VerbName $VerbName -Rows $rows
    }

    $ordered = @(Sort-Bytewise -Lines @($rows.ToArray()))
    $payload = [System.Collections.Generic.List[byte]]::new()
    $utf8 = [System.Text.UTF8Encoding]::new($false)
    $payload.AddRange($utf8.GetBytes("ratmac-review-v1`n"))
    foreach ($row in $ordered) {
        $payload.AddRange($utf8.GetBytes($row))
        $payload.AddRange($utf8.GetBytes("`n"))
    }
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        $hashBytes = $sha256.ComputeHash($payload.ToArray())
    }
    finally {
        $sha256.Dispose()
    }
    [pscustomobject]@{
        Rows      = $ordered
        WorkCount = @($ordered | Where-Object { $_.StartsWith('work ') }).Count
        LaneCount = @($ordered | Where-Object { $_.StartsWith('lane ') }).Count
        Digest    = ([BitConverter]::ToString($hashBytes) -replace '-', '').ToLowerInvariant()
    }
}

# The review record's malformed-refusal row, shared by every parse fact.
function Deny-ReviewRecord {
    param(
        [Parameter(Mandatory)][string] $Relative,
        [Parameter(Mandatory)][string] $Fact
    )
    Deny -Reason "the review record $Relative is malformed: $Fact" -Guidance @(
        "repair the record, or re-run review -Item $Item -Verdict <accepted|rejected>."
    )
}

# The recorded review, parsed to its exact format: three header rows and
# one `work <path>\t<64-hex>` or `lane <path>\t<64-hex>` evidence row per
# reviewed input, every line LF-terminated. Anything else refuses as
# malformed, the bytes left exactly as they read.
function Read-ReviewRecord {
    param(
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][string] $Relative
    )
    $bytes = $null
    try {
        $bytes = [System.IO.File]::ReadAllBytes($Path)
    }
    catch {
        Deny-ReviewRecord -Relative $Relative -Fact "it cannot be read: $($_.Exception.Message)"
    }
    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        Deny-ReviewRecord -Relative $Relative -Fact 'it carries a UTF-8 byte-order mark'
    }
    $text = $null
    try {
        $text = [System.Text.UTF8Encoding]::new($false, $true).GetString($bytes)
    }
    catch {
        Deny-ReviewRecord -Relative $Relative -Fact 'it is not valid UTF-8'
    }
    if ($text.Contains("`r")) {
        Deny-ReviewRecord -Relative $Relative -Fact 'a line carries a carriage return'
    }
    if (-not $text.EndsWith("`n")) {
        Deny-ReviewRecord -Relative $Relative -Fact 'the last line is not LF-terminated'
    }
    $split = @($text -split "`n")
    $lines = @($split | Select-Object -First ($split.Count - 1))
    if ($lines.Count -lt 3) {
        Deny-ReviewRecord -Relative $Relative -Fact "it carries $($lines.Count) lines, fewer than the three header rows"
    }
    foreach ($line in $lines) {
        if ($line -eq '') {
            Deny-ReviewRecord -Relative $Relative -Fact 'it carries an empty line'
        }
    }
    if (-not ($lines[0] -cmatch '^item (.+)$')) {
        Deny-ReviewRecord -Relative $Relative -Fact "the first line is not an item row: $($lines[0])"
    }
    $recordItem = $Matches[1]
    if (-not ($lines[1] -cmatch '^verdict (accepted|rejected)$')) {
        Deny-ReviewRecord -Relative $Relative -Fact "the verdict row is neither 'accepted' nor 'rejected': $($lines[1])"
    }
    $verdict = $Matches[1]
    if (-not ($lines[2] -cmatch '^snapshot ([0-9a-f]{64})$')) {
        Deny-ReviewRecord -Relative $Relative -Fact "the snapshot row is not a 64-hex digest: $($lines[2])"
    }
    $snapshot = $Matches[1]
    $rows = [System.Collections.Generic.List[string]]::new()
    $paths = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    for ($index = 3; $index -lt $lines.Count; $index++) {
        if (-not ($lines[$index] -cmatch '^(work|lane) ([^\t]+)\t([0-9a-f]{64})$')) {
            Deny-ReviewRecord -Relative $Relative -Fact "an evidence row is not the '<work|lane> <path>\t<64-hex>' shape: $($lines[$index])"
        }
        if (-not $paths.Add($Matches[2])) {
            Deny-ReviewRecord -Relative $Relative -Fact "the path $($Matches[2]) repeats"
        }
        $rows.Add($lines[$index])
    }
    [pscustomobject]@{
        Item     = $recordItem
        Verdict  = $verdict
        Snapshot = $snapshot
        Rows     = @($rows.ToArray())
    }
}

# The proof receipt's malformed-refusal row, shared by every parse fact.
function Deny-ProofReceipt {
    param(
        [Parameter(Mandatory)][string] $Relative,
        [Parameter(Mandatory)][string] $Fact
    )
    Deny -Reason "the proof receipt $Relative is malformed: $Fact" -Guidance @(
        "repair the receipt, or re-earn it: run proofs -Item $Item behind a current review."
    )
}

# The earned proof receipt, parsed to its exact format: three
# LF-terminated lines only, the proof-set row carrying the declared
# command verbatim. Anything else refuses as malformed, the bytes left
# exactly as they read.
function Read-ProofReceipt {
    param(
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][string] $Relative
    )
    $bytes = $null
    try {
        $bytes = [System.IO.File]::ReadAllBytes($Path)
    }
    catch {
        # A receipt that cannot be read at all - one another process
        # holds open - is unparsable, and parsing is mandatory whenever
        # the receipt is present.
        Deny-ProofReceipt -Relative $Relative -Fact "it cannot be read: $($_.Exception.Message)"
    }
    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        Deny-ProofReceipt -Relative $Relative -Fact 'it carries a UTF-8 byte-order mark'
    }
    $text = $null
    try {
        $text = [System.Text.UTF8Encoding]::new($false, $true).GetString($bytes)
    }
    catch {
        Deny-ProofReceipt -Relative $Relative -Fact 'it is not valid UTF-8'
    }
    if ($text.Contains("`r")) {
        Deny-ProofReceipt -Relative $Relative -Fact 'a line carries a carriage return'
    }
    if (-not $text.EndsWith("`n")) {
        Deny-ProofReceipt -Relative $Relative -Fact 'the last line is not LF-terminated'
    }
    $split = @($text -split "`n")
    $lines = @($split | Select-Object -First ($split.Count - 1))
    if ($lines.Count -ne 3) {
        Deny-ProofReceipt -Relative $Relative -Fact "it carries $($lines.Count) lines, not exactly three"
    }
    foreach ($line in $lines) {
        if ($line -eq '') {
            Deny-ProofReceipt -Relative $Relative -Fact 'it carries an empty line'
        }
    }
    if (-not ($lines[0] -cmatch '^item (.+)$')) {
        Deny-ProofReceipt -Relative $Relative -Fact "the first line is not an item row: $($lines[0])"
    }
    $receiptItem = $Matches[1]
    if (-not ($lines[1] -cmatch '^snapshot ([0-9a-f]{64})$')) {
        Deny-ProofReceipt -Relative $Relative -Fact "the snapshot row is not a 64-hex digest: $($lines[1])"
    }
    $snapshot = $Matches[1]
    if (-not ($lines[2] -cmatch '^proof-set (.+)$')) {
        Deny-ProofReceipt -Relative $Relative -Fact "the proof-set row does not carry the command: $($lines[2])"
    }
    [pscustomobject]@{
        Item     = $receiptItem
        Snapshot = $snapshot
        Command  = $Matches[1]
    }
}

# The stale row's changed-input rows: one `work <path>` or `lane <path>`
# row per added, removed, or changed path - a path whose recorded row
# differs or that only one side carries - bytewise sorted.
function Get-SnapshotChanges {
    param(
        [Parameter(Mandatory)][AllowEmptyCollection()][string[]] $RecordedRows,
        [Parameter(Mandatory)][AllowEmptyCollection()][string[]] $CurrentRows
    )
    $recorded = [System.Collections.Generic.Dictionary[string,string]]::new([System.StringComparer]::Ordinal)
    foreach ($row in $RecordedRows) {
        if ($row -cmatch '^(work|lane) ([^\t]+)\t([0-9a-f]{64})$') { $recorded[$Matches[2]] = $row }
    }
    $current = [System.Collections.Generic.Dictionary[string,string]]::new([System.StringComparer]::Ordinal)
    foreach ($row in $CurrentRows) {
        if ($row -cmatch '^(work|lane) ([^\t]+)\t([0-9a-f]{64})$') { $current[$Matches[2]] = $row }
    }
    $changed = [System.Collections.Generic.List[string]]::new()
    foreach ($path in $current.Keys) {
        if (-not $recorded.ContainsKey($path) -or $recorded[$path] -cne $current[$path]) {
            $changed.Add(($current[$path] -replace '\t[0-9a-f]{64}$', ''))
        }
    }
    foreach ($path in $recorded.Keys) {
        if (-not $current.ContainsKey($path)) {
            $changed.Add(($recorded[$path] -replace '\t[0-9a-f]{64}$', ''))
        }
    }
    @(Sort-Bytewise -Lines @($changed.ToArray()))
}

# The stale-snapshot refusal, shared by `proofs` (which ran nothing) and
# the checkpoint gate (which wrote nothing): both digests, the sorted
# changed-input rows, the renewal line, and the invoking verb's own last
# line.
function Deny-StaleSnapshot {
    param(
        [Parameter(Mandatory)][string] $RecordRelative,
        [Parameter(Mandatory)][string] $RecordedDigest,
        [Parameter(Mandatory)][string] $CurrentDigest,
        [Parameter(Mandatory)][AllowEmptyCollection()][string[]] $Changes,
        [Parameter(Mandatory)][string] $LastLine
    )
    Deny -Reason 'the reviewed snapshot changed since the review recorded it' -Guidance (
        @("the review record $RecordRelative binds snapshot $RecordedDigest; the addressed checkout now hashes $CurrentDigest;",
            'changed inputs:') +
        @($Changes | ForEach-Object { "  $_" }) +
        @("renew the review over the repaired bytes: pwsh -File tools/turn.ps1 review -Item $Item -Verdict <accepted|rejected>",
            $LastLine)
    )
}

function Invoke-Review {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl
    )

    # Validate, in the contract's order; every refusal writes nothing.
    $recordsRoot = Join-Path $Facts.RepoRoot $Decl.ItemRecords
    if (-not (Test-Path -LiteralPath $recordsRoot -PathType Container)) {
        Deny -Reason "the declared item-records root $($Decl.ItemRecords) does not exist" -Guidance @(
            'create it, or fix item-records in .ratmac/turn.toml.'
        )
    }
    if ([string]::IsNullOrEmpty($Verdict)) {
        Deny -Reason 'review needs a verdict: -Verdict <accepted|rejected>' -Guidance @(
            'the verdict is the independent review''s decision over the exact snapshot this invocation records.'
        )
    }
    if ($Verdict -cne 'accepted' -and $Verdict -cne 'rejected') {
        Deny -Reason "the verdict '$Verdict' is unknown" -Guidance @(
            'a verdict is exactly ''accepted'' or ''rejected''.'
        )
    }
    $null = Get-TestPause -Boundaries @()

    $snapshot = Get-ReviewedSnapshot -VerbName 'review' -Facts $Facts -Decl $Decl

    # Record: the whole review record in one step, beside the item record.
    $recordRelative = ("$($Decl.ItemRecords)/$Item-review-record.txt" -replace '\\', '/')
    $recordPath = Join-Path $recordsRoot "$Item-review-record.txt"
    $content = "item $Item`nverdict $Verdict`nsnapshot $($snapshot.Digest)`n"
    foreach ($row in $snapshot.Rows) { $content += "$row`n" }
    $temporaryPath = "$recordPath.tmp"
    try {
        [System.IO.File]::WriteAllText($temporaryPath, $content, [System.Text.UTF8Encoding]::new($false))
        [System.IO.File]::Move($temporaryPath, $recordPath, $true)
    }
    catch {
        Deny -Reason "writing the review record $recordRelative failed: $($_.Exception.Message)" -Guidance @(
            'nothing was recorded; clear the path and run review again.'
        )
    }

    $workNoun = if ($snapshot.WorkCount -eq 1) { 'file' } else { 'files' }
    $laneNoun = if ($snapshot.LaneCount -eq 1) { 'file' } else { 'files' }
    Write-Report 'turn review recorded'
    Write-Report "  item:      $Item"
    Write-Report "  verdict:   $Verdict"
    Write-Report "  snapshot:  $($snapshot.Digest)"
    Write-Report "  inputs:    $($snapshot.WorkCount) work $workNoun, $($snapshot.LaneCount) lane $laneNoun"
    exit 0
}

function Invoke-Proofs {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl
    )

    # Validate, in the contract's order; every refusal writes nothing.
    $recordsRoot = Join-Path $Facts.RepoRoot $Decl.ItemRecords
    if (-not (Test-Path -LiteralPath $recordsRoot -PathType Container)) {
        Deny -Reason "the declared item-records root $($Decl.ItemRecords) does not exist" -Guidance @(
            'create it, or fix item-records in .ratmac/turn.toml.'
        )
    }
    $pause = Get-TestPause -Boundaries @('after-proof-command')

    $recordRelative = ("$($Decl.ItemRecords)/$Item-review-record.txt" -replace '\\', '/')
    $recordPath = Join-Path $recordsRoot "$Item-review-record.txt"
    $receiptRelative = ("$($Decl.ItemRecords)/$Item-proof-receipt.txt" -replace '\\', '/')
    $receiptPath = Join-Path $recordsRoot "$Item-proof-receipt.txt"

    if (-not (Test-Path -LiteralPath $recordPath -PathType Leaf)) {
        Deny -Reason "the review record $recordRelative does not exist" -Guidance @(
            'record the independent review over the exact bytes first:',
            "  pwsh -File tools/turn.ps1 review -Item $Item -Verdict <accepted|rejected>",
            'nothing ran.'
        )
    }
    $record = Read-ReviewRecord -Path $recordPath -Relative $recordRelative
    if ($record.Item -cne $Item) {
        Deny -Reason "the review record $recordRelative records item $($record.Item), not $Item" -Guidance @(
            "re-run review -Item $Item -Verdict <accepted|rejected>."
        )
    }
    if ($record.Verdict -cne 'accepted') {
        Deny -Reason "the review record $recordRelative records verdict rejected, not accepted" -Guidance @(
            'repair the review findings, then record the accepted review and earn the proof set;',
            'nothing ran.'
        )
    }
    $snapshot = Get-ReviewedSnapshot -VerbName 'proofs' -Facts $Facts -Decl $Decl
    if ($snapshot.Digest -cne $record.Snapshot) {
        Deny-StaleSnapshot -RecordRelative $recordRelative -RecordedDigest $record.Snapshot -CurrentDigest $snapshot.Digest `
            -Changes @(Get-SnapshotChanges -RecordedRows $record.Rows -CurrentRows $snapshot.Rows) -LastLine 'nothing ran.'
    }
    if (Test-Path -LiteralPath $receiptPath -PathType Leaf) {
        $receipt = Read-ProofReceipt -Path $receiptPath -Relative $receiptRelative
        if ($receipt.Item -cne $Item) {
            Deny -Reason "the proof receipt $receiptRelative records item $($receipt.Item), not $Item" -Guidance @(
                "re-run proofs -Item $Item."
            )
        }
        if ($receipt.Snapshot -ceq $snapshot.Digest) {
            Deny -Reason "the proof set for $Item is already earned at snapshot $($receipt.Snapshot)" -Guidance @(
                'the once-only rule earns one complete proof set per accepted review;',
                'a renewed proof set needs a renewed review over changed bytes.',
                'nothing ran.'
            )
        }
    }

    # Earn: the declared command exactly once from the addressed checkout,
    # through the same invocation close step 7 uses, judged by exit code.
    Push-Location $Facts.RepoRoot
    try {
        & pwsh -NoProfile -Command $Decl.LanesRerun
        $code = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }
    if ($code -ne 0) {
        Deny -Reason "the proof-set command failed in $($Facts.RepoRoot) (exit $code)" -Guidance @(
            "the verification command was: $($Decl.LanesRerun)",
            "it ran once from $($Facts.RepoRoot)",
            'no receipt was written; repair the reported failure, then run proofs again.'
        )
    }

    if ($null -ne $pause) { Invoke-TestPause -Pause $pause }

    # Record: the whole proof receipt in one step, beside the item record.
    $content = "item $Item`nsnapshot $($snapshot.Digest)`nproof-set $($Decl.LanesRerun)`n"
    $temporaryPath = "$receiptPath.tmp"
    try {
        [System.IO.File]::WriteAllText($temporaryPath, $content, [System.Text.UTF8Encoding]::new($false))
        [System.IO.File]::Move($temporaryPath, $receiptPath, $true)
    }
    catch {
        Deny -Reason "writing the proof receipt $receiptRelative failed: $($_.Exception.Message)" -Guidance @(
            'the command already ran green; clear the path and run proofs again.'
        )
    }

    Write-Report 'turn proofs earned'
    Write-Report "  item:      $Item"
    Write-Report "  snapshot:  $($snapshot.Digest)"
    Write-Report "  command:   $($Decl.LanesRerun)"
    Write-Report "  ran:       once from $($Facts.RepoRoot)"
    Write-Report "  next:      commit the green work as '${Item}: checkpoint - not a landing', then run: pwsh -File tools/turn.ps1 checkpoint -Item $Item -Target <path>..."
    exit 0
}

# The checkpoint gate (WCP-004): the freeze takes only reviewed, proven
# green work. Inserted after t-125's clean-tracked-tree row and before the
# round-trip proof, every row refuses before any write, so the manifest
# never freezes unreviewed or unproven work.
function Invoke-ReviewGate {
    param(
        [Parameter(Mandatory)] $Facts,
        [Parameter(Mandatory)] $Decl
    )

    $recordsRoot = Join-Path $Facts.RepoRoot $Decl.ItemRecords
    $recordRelative = ("$($Decl.ItemRecords)/$Item-review-record.txt" -replace '\\', '/')
    $recordPath = Join-Path $recordsRoot "$Item-review-record.txt"
    $receiptRelative = ("$($Decl.ItemRecords)/$Item-proof-receipt.txt" -replace '\\', '/')
    $receiptPath = Join-Path $recordsRoot "$Item-proof-receipt.txt"

    if (-not (Test-Path -LiteralPath $recordPath -PathType Leaf)) {
        Deny -Reason "the review record $recordRelative does not exist" -Guidance @(
            'the checkpoint freezes only reviewed, proven green work;',
            "run review -Item $Item -Verdict <accepted|rejected>, then proofs -Item $Item, then checkpoint again.",
            'nothing was written.'
        )
    }
    $record = Read-ReviewRecord -Path $recordPath -Relative $recordRelative
    if ($record.Item -cne $Item) {
        Deny -Reason "the review record $recordRelative records item $($record.Item), not $Item" -Guidance @(
            "re-run review -Item $Item -Verdict <accepted|rejected>."
        )
    }
    if ($record.Verdict -cne 'accepted') {
        Deny -Reason "the review record $recordRelative records verdict rejected, not accepted" -Guidance @(
            'repair the review findings, then record the accepted review and earn the proof set;',
            'nothing was written.'
        )
    }
    $snapshot = Get-ReviewedSnapshot -VerbName 'checkpoint' -Facts $Facts -Decl $Decl
    if ($snapshot.Digest -cne $record.Snapshot) {
        Deny-StaleSnapshot -RecordRelative $recordRelative -RecordedDigest $record.Snapshot -CurrentDigest $snapshot.Digest `
            -Changes @(Get-SnapshotChanges -RecordedRows $record.Rows -CurrentRows $snapshot.Rows) -LastLine 'nothing was written.'
    }
    if (-not (Test-Path -LiteralPath $receiptPath -PathType Leaf)) {
        Deny -Reason "the proof receipt $receiptRelative does not exist" -Guidance @(
            'the checkpoint freezes only reviewed, proven green work;',
            "run proofs -Item $Item, then checkpoint again.",
            'nothing was written.'
        )
    }
    $receipt = Read-ProofReceipt -Path $receiptPath -Relative $receiptRelative
    if ($receipt.Item -cne $Item) {
        Deny -Reason "the proof receipt $receiptRelative records item $($receipt.Item), not $Item" -Guidance @(
            "re-run proofs -Item $Item."
        )
    }
    if ($receipt.Snapshot -cne $snapshot.Digest) {
        Deny -Reason "the proof receipt $receiptRelative binds snapshot $($receipt.Snapshot), not the reviewed snapshot $($snapshot.Digest)" -Guidance @(
            "renewed proof follows the renewed review; run proofs -Item $Item again.",
            'nothing was written.'
        )
    }
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
    'checkpoint' {
        Guard-Item
        $pause = Get-TestPause -Boundaries @('after-freeze')
        $declaration = Get-Declaration -RepoRoot $facts.RepoRoot
        Invoke-Checkpoint -Facts $facts -Decl $declaration -Pause $pause
    }
    'restore' {
        Guard-Item
        $pause = Get-TestPause -Boundaries @('after-validate', 'after-restore', 'after-verify')
        $declaration = Get-Declaration -RepoRoot $facts.RepoRoot
        Invoke-Restore -Facts $facts -Decl $declaration -Pause $pause
    }
    'review' {
        Guard-Item
        $declaration = Get-Declaration -RepoRoot $facts.RepoRoot
        Invoke-Review -Facts $facts -Decl $declaration
    }
    'proofs' {
        Guard-Item
        $declaration = Get-Declaration -RepoRoot $facts.RepoRoot
        Invoke-Proofs -Facts $facts -Decl $declaration
    }
    default {
        Deny -Reason "unknown verb '$Verb'" -Guidance @(
            'available verbs: status, open, close, checkpoint, restore, review, proofs',
            'pwsh -File tools/turn.ps1 status [-Item <string>]',
            'pwsh -File tools/turn.ps1 open -Item <string>',
            "pwsh -File tools/turn.ps1 close -Item <string> [-Line <words>] [-Obsolete <artifact>...]",
            "pwsh -File tools/turn.ps1 checkpoint -Item <string> -Target <path>...",
            'pwsh -File tools/turn.ps1 restore -Item <string>',
            'pwsh -File tools/turn.ps1 review -Item <string> -Verdict <accepted|rejected>',
            'pwsh -File tools/turn.ps1 proofs -Item <string>'
        )
    }
}
