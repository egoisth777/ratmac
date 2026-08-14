#!/usr/bin/env pwsh
#Requires -Version 7

<#
.SYNOPSIS
    ORS-002: the project-local Stable Engine bootstrap.

.DESCRIPTION
    One command, run from the project root, that resolves the Engine binary
    from the project-local build - building it there when absent - hashes it,
    compares it against the recorded pin when one exists, and reports the
    resolved path and identity.

    It is deterministic and self-contained: nothing is installed, no PATH or
    global configuration is written, and no network is used. The build runs
    offline against the toolchain already on this machine, and the only paths
    it may write are the declared build output: target and Cargo.lock.
#>

[CmdletBinding()]
param(
    # ECP-002: name a channel and the bootstrap resolves its commit, offline:
    # stable from .arca/editions.md (refusing a ledger/tag disagreement),
    # nightly from the current landing (HEAD).
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
    if (-not (Test-Path -LiteralPath $ledger)) {
        Deny -Reason "stable: cannot read $ledger"
    }
    $edition = $null
    $recorded = $null
    foreach ($line in (Get-Content -LiteralPath $ledger)) {
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
$candidates = @(
    (Join-Path $root (Join-Path 'target/release' $engineName)),
    (Join-Path $root (Join-Path 'target/debug' $engineName)))
$engine = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

if (-not $engine) {
    if ($Channel) {
        # ECP-002: stamp resolved provenance into the binary it builds. A
        # build always builds HEAD, so `stable` is stamped only when HEAD is
        # the ledger's stable commit - anything else would be a binary from
        # the tree under judgment asserting proven provenance.
        $head = & git -C $root rev-parse --verify 'HEAD^{commit}'
        if ($LASTEXITCODE -ne 0) { Deny -Reason 'HEAD does not resolve in the local repository' }
        if ($Channel -eq 'stable') {
            $stable = Resolve-StableChannel -Root $root
            if ($head -ne $stable.Commit) {
                Deny -Reason "stable: HEAD is $head but stable is $($stable.Edition) at $($stable.Commit); a stable engine is built from the stable commit, never from the tree under judgment" -Guidance @(
                    "git -c advice.detachedHead=false checkout $($stable.Commit)",
                    'pwsh -File tools/rtm.ps1 -Channel stable')
            }
        }
        $env:RTM_CHANNEL = $Channel
        $env:RTM_SOURCE_COMMIT = $head
    }
    $build = & cargo build --offline --bin rtm 2>&1
    if ($LASTEXITCODE -ne 0) {
        Deny -Reason 'the project-local build did not produce an Engine' -Guidance (
            @('the build ran offline against the installed toolchain; it reported:') +
            (@($build) | Select-Object -Last 10 | ForEach-Object { "  $_" }))
    }
    $engine = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if (-not $engine) {
        Deny -Reason 'the build succeeded but no Engine binary is at the expected path' -Guidance @(
            "expected $($candidates[-1])")
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
        'rebuild the pinned revision, or retire the Run with: rtm abandon --confirm "abandon <project>"')
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
        $stable = Resolve-StableChannel -Root $root
        Write-Report "Channel: stable is $($stable.Edition) at $($stable.Commit)"
    }
}
Write-Report "Diagnose: $engine doctor"
exit 0
