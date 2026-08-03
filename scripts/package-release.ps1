#!/usr/bin/env pwsh
<#
.SYNOPSIS
Package UnrealDevFlow release assets for GitHub Releases.
#>

param(
    [string]$Version,
    [string]$OutputDir = "dist",
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

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

function Remove-DirectoryInsideRepo {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }

    $repoFull = [System.IO.Path]::GetFullPath($RepoRoot)
    $targetFull = [System.IO.Path]::GetFullPath($Path)
    if (-not $targetFull.StartsWith($repoFull, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove path outside repository: $targetFull"
    }

    Remove-Item -LiteralPath $targetFull -Recurse -Force
}

if (-not $Version) {
    $Version = Get-CargoPackageVersion
}
$Version = $Version.TrimStart("v")

if (-not $SkipBuild) {
    cargo build --release --locked
}

$OutputDir = if ([System.IO.Path]::IsPathRooted($OutputDir)) {
    [System.IO.Path]::GetFullPath($OutputDir)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $OutputDir))
}
$staging = Join-Path $OutputDir "package-root"
$zipPath = Join-Path $OutputDir "unrealdevflow-$Target.zip"
$rawExePath = Join-Path $OutputDir "udf.exe"
$installerPath = Join-Path $OutputDir "unrealdevflow-installer.ps1"
$notesPath = Join-Path $OutputDir "RELEASE_NOTES.md"
$checksumPath = Join-Path $OutputDir "SHA256SUMS.txt"

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
Remove-DirectoryInsideRepo -Path $staging
if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
if (Test-Path -LiteralPath $checksumPath) { Remove-Item -LiteralPath $checksumPath -Force }

New-Item -ItemType Directory -Force -Path $staging | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $staging "skills") | Out-Null

Copy-Item -LiteralPath "target\release\udf.exe" -Destination (Join-Path $staging "udf.exe") -Force
Copy-Item -LiteralPath "skills\unrealdevflow" -Destination (Join-Path $staging "skills\unrealdevflow") -Recurse -Force
Copy-Item -LiteralPath "README.md" -Destination (Join-Path $staging "README.md") -Force
Copy-Item -LiteralPath "docs\RELEASE.md" -Destination (Join-Path $staging "RELEASE.md") -Force

Copy-Item -LiteralPath "target\release\udf.exe" -Destination $rawExePath -Force
Copy-Item -LiteralPath "scripts\install.ps1" -Destination $installerPath -Force

if (-not (Test-Path -LiteralPath $notesPath)) {
    & (Join-Path $PSScriptRoot "generate-release-notes.ps1") -Version $Version -OutputPath $notesPath
}

Compress-Archive -Path (Join-Path $staging "*") -DestinationPath $zipPath -Force

$assets = @($rawExePath, $zipPath, $installerPath, $notesPath)
$checksumLines = foreach ($asset in $assets) {
    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $asset
    "{0}  {1}" -f $hash.Hash.ToLowerInvariant(), (Split-Path -Leaf $asset)
}
$checksumLines | Set-Content -LiteralPath $checksumPath -Encoding utf8

Write-Host "Packaged UnrealDevFlow v$Version" -ForegroundColor Green
Write-Host "  $rawExePath"
Write-Host "  $zipPath"
Write-Host "  $installerPath"
Write-Host "  $notesPath"
Write-Host "  $checksumPath"
