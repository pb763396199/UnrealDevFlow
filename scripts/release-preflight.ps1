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
