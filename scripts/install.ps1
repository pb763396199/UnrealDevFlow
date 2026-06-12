#!/usr/bin/env pwsh
<#
.SYNOPSIS
Install or upgrade UnrealDevFlow from GitHub Releases.

.DESCRIPTION
Default mode downloads the latest release zip, extracts unrealdevflow.exe and
bundled skills/, installs them to ~/.unrealdevflow/bin, updates User PATH and
the current PowerShell session PATH, then installs the global AI skill.

Use -FromSource only when developing this repository locally.
#>

param(
    [string]$Repo = "pb763396199/UnrealDevFlow",
    [string]$Version = "latest",
    [string]$InstallPath = "$env:USERPROFILE\.unrealdevflow\bin",
    [switch]$NoPath,
    [switch]$NoSkill,
    [switch]$FromSource,
    [string]$SourceRoot,
    [switch]$AddToPath
)

$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host ""
    Write-Host $Message -ForegroundColor Cyan
}

function Write-Ok {
    param([string]$Message)
    Write-Host "  OK $Message" -ForegroundColor Green
}

function Get-ReleaseAssetUrl {
    param([string]$AssetName)

    if ($Version -eq "latest") {
        return "https://github.com/$Repo/releases/latest/download/$AssetName"
    }

    $tag = $Version
    if (-not $tag.StartsWith("v")) {
        $tag = "v$tag"
    }
    return "https://github.com/$Repo/releases/download/$tag/$AssetName"
}

function Add-UserPath {
    param([string]$PathToAdd)

    $normalized = [System.IO.Path]::GetFullPath($PathToAdd)
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @()
    if (-not [string]::IsNullOrWhiteSpace($userPath)) {
        $parts = $userPath.Split(";") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    }

    $alreadyInUserPath = $false
    foreach ($part in $parts) {
        if ([string]::Equals([System.IO.Path]::GetFullPath($part), $normalized, [StringComparison]::OrdinalIgnoreCase)) {
            $alreadyInUserPath = $true
            break
        }
    }

    if (-not $alreadyInUserPath) {
        $newUserPath = if ([string]::IsNullOrWhiteSpace($userPath)) { $normalized } else { "$userPath;$normalized" }
        [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
        Write-Ok "added to User PATH"
    } else {
        Write-Ok "already in User PATH"
    }

    $sessionParts = $env:Path.Split(";") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    $alreadyInSessionPath = $false
    foreach ($part in $sessionParts) {
        if ([string]::Equals([System.IO.Path]::GetFullPath($part), $normalized, [StringComparison]::OrdinalIgnoreCase)) {
            $alreadyInSessionPath = $true
            break
        }
    }

    if (-not $alreadyInSessionPath) {
        $env:Path = "$env:Path;$normalized"
        Write-Ok "added to current session PATH"
    } else {
        Write-Ok "already in current session PATH"
    }
}

function Install-FromRelease {
    param([string]$Destination)

    $assetName = "unrealdevflow-x86_64-pc-windows-msvc.zip"
    $assetUrl = Get-ReleaseAssetUrl -AssetName $assetName
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("unrealdevflow-install-" + [Guid]::NewGuid().ToString("N"))
    $zipPath = Join-Path $tempRoot $assetName
    $extractPath = Join-Path $tempRoot "extract"

    New-Item -ItemType Directory -Force -Path $tempRoot, $extractPath | Out-Null
    try {
        Write-Host "  Downloading $assetUrl"
        Invoke-WebRequest -Uri $assetUrl -OutFile $zipPath -UseBasicParsing
        Expand-Archive -LiteralPath $zipPath -DestinationPath $extractPath -Force

        $exe = Get-ChildItem -LiteralPath $extractPath -Recurse -Filter "unrealdevflow.exe" | Select-Object -First 1
        if (-not $exe) {
            throw "Release zip did not contain unrealdevflow.exe"
        }

        Copy-Item -LiteralPath $exe.FullName -Destination (Join-Path $Destination "unrealdevflow.exe") -Force

        $skillsSource = Get-ChildItem -LiteralPath $extractPath -Recurse -Directory |
            Where-Object { $_.FullName -match "\\skills\\unrealdevflow$" } |
            Select-Object -First 1
        if (-not $skillsSource) {
            throw "Release zip did not contain skills/unrealdevflow"
        }

        $skillsTarget = Join-Path $Destination "skills\unrealdevflow"
        if (Test-Path $skillsTarget) {
            Remove-Item -LiteralPath $skillsTarget -Recurse -Force
        }
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $skillsTarget) | Out-Null
        Copy-Item -LiteralPath $skillsSource.FullName -Destination $skillsTarget -Recurse -Force
    } finally {
        if (Test-Path $tempRoot) {
            Remove-Item -LiteralPath $tempRoot -Recurse -Force
        }
    }
}

function Install-FromSource {
    param(
        [string]$Root,
        [string]$Destination
    )

    if ([string]::IsNullOrWhiteSpace($Root)) {
        $Root = Split-Path -Parent $PSScriptRoot
    }
    $Root = [System.IO.Path]::GetFullPath($Root)
    if (-not (Test-Path (Join-Path $Root "Cargo.toml"))) {
        throw "SourceRoot is not an UnrealDevFlow repository: $Root"
    }

    Write-Host "  Building from source: $Root"
    Push-Location $Root
    try {
        cargo build --release --locked
    } finally {
        Pop-Location
    }

    Copy-Item -LiteralPath (Join-Path $Root "target\release\unrealdevflow.exe") -Destination (Join-Path $Destination "unrealdevflow.exe") -Force

    $skillsTarget = Join-Path $Destination "skills\unrealdevflow"
    if (Test-Path $skillsTarget) {
        Remove-Item -LiteralPath $skillsTarget -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $skillsTarget) | Out-Null
    Copy-Item -LiteralPath (Join-Path $Root "skills\unrealdevflow") -Destination $skillsTarget -Recurse -Force
}

Write-Host "UnrealDevFlow installer" -ForegroundColor Cyan
Write-Host "======================="

Write-Step "[1/5] Preparing install directory"
New-Item -ItemType Directory -Force -Path $InstallPath | Out-Null
$InstallPath = [System.IO.Path]::GetFullPath($InstallPath)
Write-Ok $InstallPath

Write-Step "[2/5] Installing binary and bundled skill source"
if ($FromSource) {
    Install-FromSource -Root $SourceRoot -Destination $InstallPath
} else {
    Install-FromRelease -Destination $InstallPath
}
Write-Ok "files installed"

Write-Step "[3/5] Updating PATH"
if ($NoPath) {
    Write-Host "  Skipped PATH update because -NoPath was set" -ForegroundColor Yellow
} else {
    Add-UserPath -PathToAdd $InstallPath
}

Write-Step "[4/5] Verifying command"
$exeTarget = Join-Path $InstallPath "unrealdevflow.exe"
& $exeTarget --version
if ($NoPath) {
    Write-Host "  Skipped PATH command lookup because -NoPath was set" -ForegroundColor Yellow
} else {
    $cmd = Get-Command unrealdevflow -ErrorAction SilentlyContinue
    if (-not $cmd) {
        throw "unrealdevflow.exe was installed but is still not visible via PATH in this session"
    }
    Write-Ok "PATH command resolves to $($cmd.Source)"
}

Write-Step "[5/5] Installing AI skill"
if ($NoSkill) {
    Write-Host "  Skipped skill install because -NoSkill was set" -ForegroundColor Yellow
} else {
    & $exeTarget skills install --global
    Write-Ok "global AI skill installed"
}

Write-Host ""
Write-Host "Installed successfully." -ForegroundColor Green
Write-Host ""
Write-Host "Next step:" -ForegroundColor Cyan
Write-Host "  unrealdevflow configure" -ForegroundColor White
