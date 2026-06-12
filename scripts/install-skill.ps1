#!/usr/bin/env pwsh
<#
.SYNOPSIS
Developer helper for installing the bundled UnrealDevFlow AI skill.

.DESCRIPTION
End users should run scripts/install.ps1 or the GitHub Release installer. This
script is kept for repository development and delegates to the CLI's canonical
`skills` command.
#>

param(
    [switch]$Global,
    [switch]$Uninstall,
    [string]$Project
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$Exe = Join-Path $RepoRoot "target\release\unrealdevflow.exe"

if (-not (Test-Path -LiteralPath $Exe)) {
    Write-Host "Building unrealdevflow.exe for skill installation..." -ForegroundColor Cyan
    Push-Location $RepoRoot
    try {
        cargo build --release --locked
    } finally {
        Pop-Location
    }
}

$argsList = @("skills")
if ($Uninstall) {
    $argsList += "remove"
} else {
    $argsList += "install"
}

if ($Global) {
    $argsList += "--global"
}

if ($Project) {
    $argsList += @("--project", $Project)
}

& $Exe @argsList
