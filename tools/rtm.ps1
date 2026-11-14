#!/usr/bin/env pwsh
#Requires -Version 7

<#
.SYNOPSIS
    ORS-002: the project-local Stable Engine bootstrap.

.DESCRIPTION
    One command, run from the project root, that resolves the Engine binary,
    hashes it, compares it against the recorded pin when one exists, and
    reports the resolved path and identity.

    With no channel, or with -Channel nightly, the Engine resolves from the
    project-local build - building it there when absent - exactly as before:
    nightly stamps the current landing, and an existing binary is reused
    where SVE-002 below allows.

    With -Channel stable (ELR-002) the resolve and the build are split.
    Resolution belongs to the invoking checkout: its editions ledger and its
    tags must agree there, and only there - the tagged commit's own ledger is
    never consulted, because the row that names an edition lands after the
    commit it cites, so requiring agreement there would refuse every edition.
    The build belongs to a clean separate linked checkout standing at the
    tagged commit, whose tree must be identical to that commit - untracked
    paths beyond the declared build output count as differing - so no hand
    edit and no overlay is ever built from.

    SVE-002: the Engine is what Cargo answers. After a build it is the one
    executable Cargo reported for rtm. A binary is reused only from release,
    then debug, below the output folder Cargo resolves for the build root:
    with no channel or nightly only when that folder lies inside the project
    root - a folder outside it means building first - and with stable only
    from the build checkout's own target, which a stable build names as both
    of Cargo's folders. When Cargo reports or resolves no Engine, the
    bootstrap refuses by name, stating what it searched and the repair.

    It is deterministic and self-contained: nothing is installed, no PATH or
    global configuration is written, and no network is used. The build runs
    offline against the toolchain already on this machine, and the only paths
    it may write are the declared build output: the output and build folders
    Cargo resolves (target by default) and Cargo.lock.
#>

[CmdletBinding()]
param(
    # ECP-002: name a channel and the bootstrap resolves its commit, offline:
    # stable from the invoking checkout's editions ledger (refusing a
    # ledger/tag disagreement there), nightly from the current landing
    # (HEAD). ELR-002: stable builds in a clean separate checkout of the
    # tagged commit; nightly builds in place, as before.
    [ValidateSet('stable', 'nightly')]
    [string] $Channel
)

$ErrorActionPreference = 'Stop'

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
    [Console]::Error.WriteLine("bootstrap refused: $Reason")
    foreach ($line in $Guidance) { [Console]::Error.WriteLine("  $line") }
    exit 1
}

function Get-Sha256 {
    param([Parameter(Mandatory)][string] $Path)
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

# The recorded Engine pin, read with the field names the Engine writes.
function Get-EnginePin {
    param([Parameter(Mandatory)][string] $EvidencePath)
    if (-not (Test-Path -LiteralPath $EvidencePath)) { return $null }
    $lines = Get-Content -LiteralPath $EvidencePath
    $inEngine = $false
    $resolved = $null
    $sha256 = $null
    foreach ($line in $lines) {
        $trimmed = $line.Trim()
        if ($trimmed -match '^\[(.+)\]$') {
            $inEngine = ($Matches[1] -eq 'engine')
            continue
        }
        if (-not $inEngine) { continue }
        if ($trimmed -match '^resolved\s*=\s*"(.*)"$') { $resolved = $Matches[1] }
        if ($trimmed -match '^sha256\s*=\s*"(.*)"$') { $sha256 = $Matches[1] }
    }
    if ($null -eq $sha256) { return $null }
    [pscustomobject]@{ Resolved = $resolved; Sha256 = $sha256 }
}


# ECP-002: resolve the stable channel from the ledger, offline; a ledger/tag
# disagreement or malformed row refuses, never resolves.
function Resolve-StableChannel {
    param([Parameter(Mandatory)][string] $Root)
    $ledger = Join-Path $Root '.arca/editions.md'
    # An absent ledger and an unreadable one (locked, half-written, or
    # something else entirely at that path) are the same verdict: refuse.
    $rows = $null
    try {
        $rows = Get-Content -LiteralPath $ledger -ErrorAction Stop
    } catch {
        Deny -Reason "stable: cannot read $ledger"
    }
    $edition = $null
    $recorded = $null
    foreach ($line in @($rows)) {
        if ($line.Trim() -match '^\|\s*`(edition-[^`]+)`\s*\|\s*`([^`]*)`\s*\|') {
            $edition = $Matches[1]
            $recorded = $Matches[2]
        }
    }
    if ($null -eq $edition) {
        Deny -Reason 'stable: the editions ledger carries no edition row, so there is no stable to resolve'
    }
    if ($recorded -notmatch '^[0-9a-f]{40}$') {
        Deny -Reason "stable: ledger row $edition records '$recorded', which is not a whole 40-hex commit hash"
    }
    $tagged = & git -C $Root rev-parse --verify "refs/tags/$edition^{commit}" 2>$null
    if ($LASTEXITCODE -ne 0) {
        Deny -Reason "stable: ledger names $edition, but the tag does not resolve in the local repository"
    }
    if ($tagged -ne $recorded) {
        Deny -Reason "stable: the ledger records $edition at $recorded but the tag points at $tagged; a ledger/tag disagreement is refused, not resolved"
    }
    [pscustomobject]@{ Edition = $edition; Commit = $recorded }
}

# SVE-002: the output folder Cargo resolves for the build root, asked of
# Cargo itself and offline; no answer refuses by name.
function Get-CargoOutputFolder {
    param([Parameter(Mandatory)][string] $Directory)
    Push-Location -LiteralPath $Directory
    try {
        $answer = & cargo metadata --format-version 1 --no-deps --offline 2>&1
        $answerExit = $LASTEXITCODE
    } catch [System.Management.Automation.CommandNotFoundException] {
        Deny -Reason ("Cargo could not resolve the Engine's output folder in $Directory; " +
            "searched: cargo metadata --format-version 1 --no-deps --offline in $Directory; " +
            "repair: put cargo on PATH, then run pwsh -File tools/rtm.ps1")
    } finally {
        Pop-Location
    }
    $stdout = @($answer | Where-Object { $_ -isnot [System.Management.Automation.ErrorRecord] })
    $stderr = @($answer | Where-Object { $_ -is [System.Management.Automation.ErrorRecord] } | ForEach-Object { "$_" })
    $folder = $null
    if ($answerExit -eq 0) {
        try {
            $folder = (ConvertFrom-Json -InputObject ($stdout -join "`n") -ErrorAction Stop).target_directory
        } catch {
            $folder = $null
        }
    }
    if (-not $folder) {
        Deny -Reason ("Cargo could not resolve the Engine's output folder in $Directory; " +
            "searched: cargo metadata --format-version 1 --no-deps --offline in $Directory; " +
            "repair: fix the Cargo configuration or CARGO_TARGET_DIR it names, then run pwsh -File tools/rtm.ps1") -Guidance (
            @("Cargo's standard error:") + (@($stderr) | Select-Object -Last 10 | ForEach-Object { "  $_" }))
    }
    [string] $folder
}

# SVE-002: whether $Path is $Root or lies below it, compared as full paths,
# without case on Windows.
function Test-InsideRoot {
    param(
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][string] $Root
    )
    $separators = [char[]]@([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
    $full = [IO.Path]::GetFullPath($Path).TrimEnd($separators)
    $base = [IO.Path]::GetFullPath($Root).TrimEnd($separators)
    $comparison = if ($IsWindows) { [StringComparison]::OrdinalIgnoreCase } else { [StringComparison]::Ordinal }
    $full.Equals($base, $comparison) -or $full.StartsWith($base + [IO.Path]::DirectorySeparatorChar, $comparison)
}

# The two places an earlier build's Engine binary may sit, in the order the
# bootstrap prefers them, below the output folder Cargo resolves - inside the
# folder for the build target CARGO_BUILD_TARGET names, when it names one.
function Get-EngineCandidates {
    param([Parameter(Mandatory)][string] $Folder)
    $base = $Folder
    if ($env:CARGO_BUILD_TARGET) {
        $triple = $env:CARGO_BUILD_TARGET
        if ($triple.EndsWith('.json')) { $triple = [IO.Path]::GetFileNameWithoutExtension($triple) }
        $base = Join-Path $Folder $triple
    }
    @(
        (Join-Path $base 'release'),
        (Join-Path $base 'debug')
    ) | ForEach-Object { Join-Path $_ $engineName }
}

# ELR-002: the stable build checkout lives beside the invoking checkout, one
# per edition, so each edition's engine is built once and reused.
function Get-StableBuildRoot {
    param(
        [Parameter(Mandatory)][string] $Root,
        [Parameter(Mandatory)][string] $Edition
    )
    Join-Path (Split-Path -Parent $Root) "$((Split-Path -Leaf $Root))-stable-$Edition"
}

# ELR-002: locate or link the build checkout standing at the tagged commit.
# A registration whose directory is gone is pruned first; an existing
# directory must already stand at the tagged commit - the bootstrap never
# checks out over a checkout it did not leave in that state.
function Get-StableBuildCheckout {
    param(
        [Parameter(Mandatory)][string] $Root,
        [Parameter(Mandatory)][string] $Edition,
        [Parameter(Mandatory)][string] $Commit
    )
    $buildRoot = Get-StableBuildRoot -Root $Root -Edition $Edition
    if (Test-Path -LiteralPath $buildRoot) {
        $standing = & git -C $buildRoot rev-parse --verify 'HEAD^{commit}' 2>$null
        if ($LASTEXITCODE -ne 0) {
            Deny -Reason "stable: the build checkout at $buildRoot does not resolve as a checkout of this repository" -Guidance @(
                "inspect $buildRoot; remove it if it is not the linked worktree the bootstrap made",
                'pwsh -File tools/rtm.ps1 -Channel stable')
        }
        if ($standing -ne $Commit) {
            Deny -Reason "stable: the build checkout at $buildRoot stands at $standing, not at the tagged commit $Commit" -Guidance @(
                "git -C $buildRoot checkout --detach $Commit",
                'pwsh -File tools/rtm.ps1 -Channel stable')
        }
    } else {
        $null = & git -C $Root worktree prune
        $add = & git -C $Root worktree add --detach $buildRoot $Commit 2>&1
        if ($LASTEXITCODE -ne 0) {
            Deny -Reason "stable: the build checkout at $buildRoot could not be linked at $Commit" -Guidance (
                @('git reported:') + (@($add) | Select-Object -Last 5 | ForEach-Object { "  $_" }))
        }
    }
    $buildRoot
}

# ELR-002: the build checkout's tree must be identical to the tagged commit.
# git status must be silent apart from untracked declared build output
# (target and Cargo.lock): a modified tracked file, a staged difference, or
# an untracked path beyond the declared output each counts as differing.
function Assert-BuildTreeIsTagged {
    param(
        [Parameter(Mandatory)][string] $BuildRoot,
        [Parameter(Mandatory)][string] $Commit
    )
    $rows = @(& git -C $BuildRoot status --porcelain) | Where-Object { $null -ne $_ }
    if ($LASTEXITCODE -ne 0) {
        Deny -Reason "stable: the build checkout at $BuildRoot cannot be read for tree identity with $Commit" -Guidance @(
            "git -C $BuildRoot status --porcelain")
    }
    $differences = @()
    foreach ($line in $rows) {
        if ($line.Length -lt 4) {
            $differences += $line
            continue
        }
        $state = $line.Substring(0, 2)
        $path = $line.Substring(3).Trim('"')
        $declared = ($path -eq 'Cargo.lock') -or ($path -eq 'target') -or $path.StartsWith('target/')
        if ($state -eq '??' -and $declared) { continue }
        $differences += $line
    }
    if ($differences.Count -gt 0) {
        Deny -Reason "stable: the build checkout at $BuildRoot differs from the tagged commit $Commit, so its tree is not the tagged tree" -Guidance (
            @('a stable engine is built only from a checkout identical to the tagged commit; differences:') +
            (@($differences) | Select-Object -First 10 | ForEach-Object { "  $_" }) + @(
                "restore it: git -C $BuildRoot restore --staged --worktree .; git -C $BuildRoot clean -fd",
                "or remove $buildRoot and re-run the bootstrap to relink a clean checkout"))
    }
}

# Build the Engine inside $Directory after stamping provenance, and return
# the one executable Cargo reported for rtm (SVE-002), never a binary at a
# guessed path. An empty stamp pair builds without stamps. A -TargetDir is
# named to Cargo as both its output and its build folder, for this build only.
function Invoke-EngineBuild {
    param(
        [Parameter(Mandatory)][string] $Directory,
        [Parameter(Mandatory)][AllowEmptyString()][string] $StampChannel,
        [Parameter(Mandatory)][AllowEmptyString()][string] $StampCommit,
        [string] $TargetDir = ''
    )
    # ECP-002: stamp resolved provenance into the binary the build makes, so
    # a built engine can name the channel and commit it came from.
    if ($StampChannel) {
        $env:RTM_CHANNEL = $StampChannel
        $env:RTM_SOURCE_COMMIT = $StampCommit
    }
    $arguments = @('build', '--offline', '--bin', 'rtm', '--message-format=json')
    if ($TargetDir) { $arguments += @('--target-dir', $TargetDir) }
    $buildDir = $env:CARGO_BUILD_BUILD_DIR
    Push-Location -LiteralPath $Directory
    try {
        if ($TargetDir) { $env:CARGO_BUILD_BUILD_DIR = $TargetDir }
        $build = & cargo @arguments 2>&1
        $buildExit = $LASTEXITCODE
    } finally {
        if ($TargetDir) {
            if ($null -eq $buildDir) {
                Remove-Item Env:CARGO_BUILD_BUILD_DIR -ErrorAction SilentlyContinue
            } else {
                $env:CARGO_BUILD_BUILD_DIR = $buildDir
            }
        }
        Pop-Location
    }
    # Cargo's answers are its JSON records on standard output: the rendered
    # compiler messages, and each artifact's target name and executable.
    $rendered = @()
    $artifacts = @()
    $stderr = @()
    foreach ($line in @($build)) {
        if ($line -is [System.Management.Automation.ErrorRecord]) {
            $stderr += "$line"
            continue
        }
        try {
            $record = ConvertFrom-Json -InputObject "$line" -ErrorAction Stop
        } catch {
            continue
        }
        if ($record.reason -eq 'compiler-message' -and $record.message.rendered) {
            $rendered += @("$($record.message.rendered)".TrimEnd() -split "\r?\n")
        } elseif ($record.reason -eq 'compiler-artifact') {
            $artifacts += [pscustomobject]@{ Name = "$($record.target.name)"; Executable = "$($record.executable)" }
        }
    }
    if ($buildExit -ne 0) {
        Deny -Reason 'the project-local build did not produce an Engine' -Guidance (
            @('the build ran offline against the installed toolchain; it reported:') +
            (@($rendered) | ForEach-Object { "  $_" }) +
            @("Cargo's standard error:") +
            (@($stderr) | Select-Object -Last 10 | ForEach-Object { "  $_" }))
    }
    $executables = @($artifacts | Where-Object { $_.Name -eq 'rtm' -and $_.Executable } |
        ForEach-Object { $_.Executable } | Select-Object -Unique)
    if ($executables.Count -eq 1) {
        return $executables[0]
    }
    $reported = if ($artifacts.Count -eq 0) { 'nothing' } else {
        (@($artifacts) | ForEach-Object {
            if ($_.Executable) { "$($_.Name): $($_.Executable)" } else { "$($_.Name): no executable" }
        }) -join ', '
    }
    $verdict = if ($executables.Count -eq 0) {
        "Cargo reported no Engine executable for rtm in $Directory"
    } else {
        "Cargo reported $($executables.Count) distinct Engine executables for rtm in $Directory"
    }
    Deny -Reason ("$verdict; searched: Cargo's build report (cargo $($arguments -join ' ') in $Directory); " +
        "Cargo reported: $reported; repair: make the rtm binary target build in $Directory, then run pwsh -File tools/rtm.ps1")
}

$scriptRoot = Split-Path -Parent $PSCommandPath
$root = (Resolve-Path (Join-Path $scriptRoot '..')).Path
$here = (Get-Location).Path

if ($here -ne $root) {
    Deny -Reason "the bootstrap runs from the project root, not $here" -Guidance @(
        "cd $root",
        'pwsh -File tools/rtm.ps1')
}
if (-not (Test-Path -LiteralPath (Join-Path $root 'Cargo.toml'))) {
    Deny -Reason 'this directory has no Cargo.toml, so no Engine can be built here' -Guidance @(
        'run the bootstrap from the root of a ratmac project')
}

$engineName = if ($IsWindows) { 'rtm.exe' } else { 'rtm' }
$stable = $null

if ($Channel -eq 'stable') {
    # ELR-002: resolve from the invoking checkout's own ledger and tags,
    # then build the tagged commit in a clean separate checkout. The tagged
    # commit's own ledger is never judged against its tag.
    $stable = Resolve-StableChannel -Root $root
    $buildRoot = Get-StableBuildCheckout -Root $root -Edition $stable.Edition -Commit $stable.Commit
    Assert-BuildTreeIsTagged -BuildRoot $buildRoot -Commit $stable.Commit
    # SVE-002: the build checkout's own target is both of Cargo's folders, so
    # an inherited redirect never mixes editions or dirties the checkout.
    $own = Join-Path $buildRoot 'target'
    $engine = Get-EngineCandidates -Folder $own |
        Where-Object { Test-Path -LiteralPath $_ } |
        Select-Object -First 1
    if (-not $engine) {
        $engine = Invoke-EngineBuild -Directory $buildRoot -StampChannel 'stable' -StampCommit $stable.Commit -TargetDir $own
    }
} else {
    # SVE-002: reuse only from the output folder Cargo resolves, and only when
    # it lies inside the project root; a folder outside it means building.
    $out = Get-CargoOutputFolder -Directory $root
    $engine = $null
    if (Test-InsideRoot -Path $out -Root $root) {
        $engine = Get-EngineCandidates -Folder $out |
            Where-Object { Test-Path -LiteralPath $_ } |
            Select-Object -First 1
    }
    if (-not $engine) {
        $stampChannel = ''
        $stampCommit = ''
        if ($Channel) {
            $head = & git -C $root rev-parse --verify 'HEAD^{commit}'
            if ($LASTEXITCODE -ne 0) { Deny -Reason 'HEAD does not resolve in the local repository' }
            $stampChannel = $Channel
            $stampCommit = $head
        }
        $engine = Invoke-EngineBuild -Directory $root -StampChannel $stampChannel -StampCommit $stampCommit
    }
}

$engine = (Resolve-Path -LiteralPath $engine).Path
$observed = Get-Sha256 -Path $engine
$evidencePath = Join-Path $root '.ratmac/evidence.toml'
$pin = Get-EnginePin -EvidencePath $evidencePath

if ($null -ne $pin -and $pin.Sha256 -ne $observed) {
    Deny -Reason 'the resolved Engine is not the pinned Engine' -Guidance @(
        "resolved (observed) = $engine",
        "sha256 (observed) = $observed",
        "resolved (expected) = $($pin.Resolved)",
        "sha256 (expected) = $($pin.Sha256)",
        'the pin was recorded by the active Run in .ratmac/evidence.toml [engine]',
        'rebuild the pinned revision, or retire the Run with: rtm abandon --run <id> --confirm "abandon <run id>"')
}

Write-Report "Engine: $engine"
Write-Report "sha256: $observed"
if ($null -eq $pin) {
    Write-Report 'Pin: no pin recorded in .ratmac/evidence.toml'
} else {
    Write-Report 'Pin: matches .ratmac/evidence.toml [engine]'
}
if ($Channel) {
    if ($Channel -eq 'nightly') {
        $commit = & git -C $root rev-parse --verify 'HEAD^{commit}' 2>$null
        if ($LASTEXITCODE -ne 0) {
            Deny -Reason 'nightly: HEAD does not resolve in the local repository'
        }
        Write-Report "Channel: nightly is the current landing $commit"
    } else {
        Write-Report "Channel: stable is $($stable.Edition) at $($stable.Commit)"
    }
}
Write-Report "Diagnose: $engine doctor"
exit 0
