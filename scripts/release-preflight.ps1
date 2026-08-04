#!/usr/bin/env pwsh
<#
.SYNOPSIS
Run the mandatory UnrealDevFlow release quality gate.
#>

param(
    [string]$Version,
    [switch]$Strict,
    [switch]$AllowDirty
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

function Invoke-CheckedStep {
    param(
        [string]$Name,
        [scriptblock]$Action
    )

    Write-Host ""
    Write-Host "==> $Name" -ForegroundColor Cyan
    $global:LASTEXITCODE = 0
    & $Action
    if ($global:LASTEXITCODE -ne 0) {
        throw "Step failed with exit code $global:LASTEXITCODE`: $Name"
    }
    Write-Host "OK: $Name" -ForegroundColor Green
}

function Require-Command {
    param([string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command not found: $Name"
    }
}

function Get-CargoPackageVersion {
    $inPackage = $false
    foreach ($line in Get-Content -LiteralPath "Cargo.toml") {
        if ($line -match '^\[package\]\s*$') {
            $inPackage = $true
            continue
        }
        if ($inPackage -and $line -match '^\[') {
            break
        }
        if ($inPackage -and $line -match '^version\s*=\s*"([^"]+)"') {
            return $Matches[1]
        }
    }
    throw "Could not read [package].version from Cargo.toml"
}

Invoke-CheckedStep "Check required tools" {
    Require-Command git
    Require-Command cargo
}

Invoke-CheckedStep "Check git working tree" {
    $status = git status --porcelain
    if ($status -and -not $AllowDirty) {
        throw "Working tree is dirty. Commit or stash changes before release, or pass -AllowDirty for local smoke checks."
    }
    if ($status) {
        Write-Host "Working tree is dirty, continuing because -AllowDirty was set." -ForegroundColor Yellow
    }
}

Invoke-CheckedStep "Check version metadata" {
    $cargoVersion = Get-CargoPackageVersion
    if ($Version) {
        $normalized = $Version.TrimStart("v")
        if ($normalized -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') {
            throw "Version must be semver-like, got: $Version"
        }
        if ($cargoVersion -ne $normalized) {
            throw "Cargo.toml version ($cargoVersion) does not match requested release version ($normalized)"
        }
    }
    Write-Host "Cargo.toml version: $cargoVersion"
}

Invoke-CheckedStep "Check release support files" {
    $required = @(
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        ".github/release.yml",
        "scripts/install.ps1",
        "scripts/package-release.ps1",
        "scripts/generate-release-notes.ps1",
        "docs/RELEASE.md",
        "skills/unrealdevflow/SKILL.md",
        "skills/unrealdevflow-release/SKILL.md"
    )

    foreach ($path in $required) {
        if (-not (Test-Path -LiteralPath $path)) {
            throw "Missing required release support file: $path"
        }
    }
}

Invoke-CheckedStep "Check PowerShell script encoding" {
    # Windows PowerShell 5.1 reads a .ps1 without a BOM using the console
    # codepage. One Chinese character inside a string literal is then enough to
    # eat the closing quote and take the whole script down with a syntax error,
    # which is exactly how the 0.2.0 release notes generator broke.
    foreach ($script in Get-ChildItem -Path "scripts" -Filter "*.ps1" -File) {
        $bytes = [System.IO.File]::ReadAllBytes($script.FullName)
        $hasBom = $bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF
        $text = [System.Text.Encoding]::UTF8.GetString($bytes)
        $hasNonAscii = [bool]($text.ToCharArray() | Where-Object { [int]$_ -gt 127 })

        # install.ps1 is fetched and piped straight into `iex`. A BOM survives
        # that trip as a real character, so `param()` is no longer the first
        # statement and the whole script fails to parse. It therefore has to
        # stay pure ASCII with no BOM -- adding one broke the documented
        # one-line install in 0.2.0.
        if ($script.Name -eq "install.ps1") {
            if ($hasBom) {
                throw "scripts/install.ps1 must not have a BOM; it is piped into iex and a BOM breaks its param() block"
            }
            if ($hasNonAscii) {
                throw "scripts/install.ps1 must stay pure ASCII; it cannot carry a BOM, so non-ASCII would be misread under Windows PowerShell 5.1"
            }
            continue
        }

        if ($hasBom) { continue }
        if ($hasNonAscii) {
            throw "scripts/$($script.Name) has non-ASCII characters but no UTF-8 BOM; Windows PowerShell 5.1 will misread it"
        }
    }
    Write-Host "All scripts/*.ps1 are safe to parse under Windows PowerShell 5.1"
}

Invoke-CheckedStep "Installer parses the way iex would see it" {
    # README tells people to run `irm <url> | iex`. That path never executes the
    # file from disk, it parses a downloaded string, so a byte-level problem at
    # the top of the file is invisible to every other check here.
    $bytes = [System.IO.File]::ReadAllBytes((Join-Path $RepoRoot "scripts\install.ps1"))
    $asDownloaded = [System.Text.Encoding]::UTF8.GetString($bytes)
    $errors = $null
    $null = [System.Management.Automation.Language.Parser]::ParseInput($asDownloaded, [ref]$null, [ref]$errors)
    if ($errors -and $errors.Count -gt 0) {
        throw "scripts/install.ps1 does not parse as a downloaded string: $($errors[0].Message)"
    }
    Write-Host "install.ps1 parses cleanly as a piped string"
}

Invoke-CheckedStep "Generate release notes (dry run)" {
    # Checking that the generator file exists proves nothing. It has to run,
    # against the curated notes for this very version, or a missing section or
    # a leftover placeholder only surfaces after the tag is already pushed.
    $probe = Join-Path ([System.IO.Path]::GetTempPath()) ("udf-release-notes-" + [System.Guid]::NewGuid().ToString("N") + ".md")
    try {
        & (Join-Path $PSScriptRoot "generate-release-notes.ps1") -Version (Get-CargoPackageVersion) -OutputPath $probe
        if ($LASTEXITCODE -ne 0) {
            throw "generate-release-notes.ps1 exited with $LASTEXITCODE"
        }
        $notes = Get-Content -Raw -Encoding utf8 -LiteralPath $probe
        if ($notes.Contains([char]0xFFFD)) {
            throw "Generated release notes contain replacement characters; something was decoded with the wrong codepage"
        }
        Write-Host "Release notes generate cleanly ($($notes.Split("`n").Count) lines)"
    } finally {
        if (Test-Path -LiteralPath $probe) { Remove-Item -LiteralPath $probe -Force }
    }
}

Invoke-CheckedStep "cargo fmt" {
    cargo fmt --all -- --check
}

Invoke-CheckedStep "cargo clippy strict" {
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
}

Invoke-CheckedStep "cargo test" {
    cargo test --workspace --all-targets --all-features --locked
}

Invoke-CheckedStep "cargo release build" {
    cargo build --release --locked
}

Invoke-CheckedStep "binary smoke test" {
    $exe = Join-Path $RepoRoot "target\release\udf.exe"
    if (-not (Test-Path -LiteralPath $exe)) {
        throw "Release binary not found: $exe"
    }
    & $exe --version
}

Write-Host ""
Write-Host "Release preflight passed." -ForegroundColor Green
