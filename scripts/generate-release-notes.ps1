#!/usr/bin/env pwsh
<#
.SYNOPSIS
Generate curated UnrealDevFlow release notes.

.DESCRIPTION
Formal release notes must be written as docs/releases/v<version>.md first.
This script expands a few placeholders and validates that the notes are not a
generic template. Use -AllowGeneratedDraft only when drafting a new notes file.
#>

param(
    [string]$Version,
    [string]$PreviousTag,
    [string]$OutputPath = "RELEASE_NOTES.md",
    [string]$Repo = "pb763396199/UnrealDevFlow",
    [switch]$AllowGeneratedDraft
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

function Try-GetRepoFromOrigin {
    try {
        $remote = git remote get-url origin 2>$null
        if ($remote -match 'github\.com[:/](?<owner>[^/]+)/(?<repo>[^/.]+)(\.git)?$') {
            return "$($Matches.owner)/$($Matches.repo)"
        }
    } catch {
        return $null
    }
    return $null
}

function Get-PreviousReleaseTag {
    param([string]$TargetVersion)

    $targetCore = $TargetVersion -replace '-.*$', ''
    $target = $null
    if (-not [version]::TryParse($targetCore, [ref]$target)) {
        return $null
    }

    $candidates = foreach ($tag in @(git tag --list "v[0-9]*" 2>$null)) {
        if ($tag -match '^v(?<version>\d+\.\d+\.\d+)$') {
            $parsed = $null
            if ([version]::TryParse($Matches.version, [ref]$parsed) -and $parsed -lt $target) {
                [pscustomobject]@{
                    Tag = $tag
                    Version = $parsed
                }
            }
        }
    }

    $previous = $candidates | Sort-Object Version -Descending | Select-Object -First 1
    if ($previous) {
        return $previous.Tag
    }
    return $null
}

function Test-ReleaseNotesQuality {
    param(
        [string]$Notes,
        [string]$SourcePath
    )

    $requiredSections = @(
        "一句话总结",
        "安装 / 升级",
        "重点变化",
        "新增",
        "修复",
        "破坏性变更",
        "AI Agent 变化",
        "校验",
        "变更列表"
    )

    foreach ($section in $requiredSections) {
        $escaped = [regex]::Escape($section)
        if ($Notes -notmatch "(?m)^##\s+$escaped\s*$") {
            throw "Release notes missing required section '$section': $SourcePath"
        }
    }

    $forbiddenPatterns = @(
        "TODO",
        "TBD",
        "待补",
        "__VERSION__",
        "__INSTALL_COMMAND__",
        "__RANGE__",
        "__COMMITS__",
        "这一版发布 UnrealDevFlow 的标准安装包和可复现 release 流程",
        "GitHub Release Windows 预编译安装链路。",
        "标准 release notes、checksum 和 release asset 打包流程。",
        '新增 `unrealdevflow-release` skill，发布任务必须按固定门禁执行。'
    )

    foreach ($pattern in $forbiddenPatterns) {
        if ($Notes.Contains($pattern)) {
            throw "Release notes still contain placeholder/boilerplate '$pattern': $SourcePath"
        }
    }
}

if (-not $Version) {
    $Version = Get-CargoPackageVersion
}
$Version = $Version.TrimStart("v")
$releaseTag = "v$Version"

$detectedRepo = Try-GetRepoFromOrigin
if ($detectedRepo) {
    $Repo = $detectedRepo
}

if (-not $PreviousTag) {
    $PreviousTag = Get-PreviousReleaseTag -TargetVersion $Version
}

$releaseEnd = "HEAD"
git rev-parse -q --verify "refs/tags/$releaseTag" *> $null
if ($LASTEXITCODE -eq 0) {
    $releaseEnd = $releaseTag
}

$range = if ($PreviousTag) { "$PreviousTag..$releaseEnd" } else { $releaseEnd }
# git writes UTF-8, but Windows PowerShell decodes a native command's stdout
# with the console codepage — on a Chinese Windows that turns every commit
# subject into mojibake in the published release notes.
$previousOutputEncoding = [Console]::OutputEncoding
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
try {
    $commits = @(git log $range --pretty=format:"- %s" 2>$null)
} finally {
    [Console]::OutputEncoding = $previousOutputEncoding
}
if (-not $commits) {
    $commits = @("- No commits found for $range.")
}
$commitText = $commits -join "`n"

$installCommand = 'powershell -ExecutionPolicy Bypass -c "irm https://github.com/' + $Repo + '/releases/latest/download/unrealdevflow-installer.ps1 | iex"'

$curatedPath = Join-Path $RepoRoot "docs\releases\$releaseTag.md"
if (Test-Path -LiteralPath $curatedPath) {
    # Windows PowerShell 5.1 reads files without a BOM as ANSI, which turns the
    # Chinese section headings into mojibake and makes every required-section
    # check fail. Say UTF-8 out loud rather than putting a BOM on the markdown.
    $notesTemplate = Get-Content -Raw -Encoding utf8 -LiteralPath $curatedPath
    $sourcePath = $curatedPath
} elseif ($AllowGeneratedDraft) {
    $notesTemplate = @'
# UnrealDevFlow v__VERSION__

## 一句话总结

TODO: 用一句话说明这个版本给用户带来的具体变化。

## 安装 / 升级

```powershell
__INSTALL_COMMAND__
```

## 重点变化

- TODO: 写用户能理解的价值，而不是内部提交名。

## 新增

- TODO: 列出本版本真正新增的能力。

## 修复

- TODO: 列出本版本真正修复的问题；没有就写“无”。

## 破坏性变更

- TODO: 写升级注意；没有就写“无”。

## AI Agent 变化

- TODO: 写 agent 使用方式、skill、提示词或流程变化；没有就写“无”。

## 校验

- TODO: 写本次实际跑过的门禁。

## 变更列表

Range: __RANGE__

__COMMITS__
'@
    $sourcePath = "generated draft"
} else {
    throw "Missing curated release notes source: docs/releases/$releaseTag.md. Create it first, or use -AllowGeneratedDraft only for drafting."
}

$notes = $notesTemplate.
    Replace("__VERSION__", $Version).
    Replace("__INSTALL_COMMAND__", $installCommand).
    Replace("__RANGE__", $range).
    Replace("__COMMITS__", $commitText)

if (-not $AllowGeneratedDraft) {
    Test-ReleaseNotesQuality -Notes $notes -SourcePath $sourcePath
}

$outputFull = if ([System.IO.Path]::IsPathRooted($OutputPath)) {
    [System.IO.Path]::GetFullPath($OutputPath)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $OutputPath))
}
$parent = Split-Path -Parent $outputFull
if ($parent) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}
$notes | Set-Content -LiteralPath $outputFull -Encoding utf8
Write-Host "Release notes written to $outputFull" -ForegroundColor Green
