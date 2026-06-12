#!/usr/bin/env pwsh
<#
.SYNOPSIS
Generate standardized UnrealDevFlow release notes.
#>

param(
    [string]$Version,
    [string]$PreviousTag,
    [string]$OutputPath = "RELEASE_NOTES.md",
    [string]$Repo = "pb763396199/UnrealDevFlow"
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

if (-not $Version) {
    $Version = Get-CargoPackageVersion
}
$Version = $Version.TrimStart("v")

$detectedRepo = Try-GetRepoFromOrigin
if ($detectedRepo) {
    $Repo = $detectedRepo
}

if (-not $PreviousTag) {
    $tags = @(git tag --sort=-v:refname 2>$null)
    $currentTag = "v$Version"
    $PreviousTag = ($tags | Where-Object { $_ -ne $currentTag } | Select-Object -First 1)
}

$range = if ($PreviousTag) { "$PreviousTag..HEAD" } else { "HEAD" }
$commits = @(git log $range --pretty=format:"- %s" 2>$null)
if (-not $commits) {
    $commits = @("- Initial public release assets and release process.")
}

$installCommand = 'powershell -ExecutionPolicy Bypass -c "irm https://github.com/' + $Repo + '/releases/latest/download/unrealdevflow-installer.ps1 | iex"'

$commitText = $commits -join "`n"

$notesTemplate = @'
# UnrealDevFlow v__VERSION__

## 一句话总结

这一版发布 UnrealDevFlow 的标准安装包和可复现 release 流程。

## 安装 / 升级

```powershell
__INSTALL_COMMAND__
```

## 新增

- GitHub Release Windows 预编译安装链路。
- PowerShell installer，自动安装 exe、写入 PATH、安装 AI skill。
- 标准 release preflight：fmt、clippy、test、release build、binary smoke test。
- 标准 release notes、checksum 和 release asset 打包流程。

## 修复

- 修复安装后当前终端找不到 `unrealdevflow` 的 PATH 刷新问题。
- 允许 `unrealdevflow skills install` 在工具未配置 UE 项目前运行。

## 破坏性变更

- 无。

## AI Agent 变化

- 新增 `unrealdevflow-release` skill，发布任务必须按固定门禁执行。
- installer 会把 `unrealdevflow` skill 安装到全局 agent 位置。

## 校验

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --all-targets --all-features --locked`
- `cargo build --release --locked`
- `target/release/unrealdevflow.exe --version`

## 变更列表

Range: __RANGE__

__COMMITS__
'@

$notes = $notesTemplate.
    Replace("__VERSION__", $Version).
    Replace("__INSTALL_COMMAND__", $installCommand).
    Replace("__RANGE__", $range).
    Replace("__COMMITS__", $commitText)

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
