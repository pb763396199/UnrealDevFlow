#!/usr/bin/env pwsh
# UnrealDevFlow 安装脚本
# 用法：.\install.ps1 [-InstallPath <path>] [-AddToPath]

param(
    [string]$InstallPath = "$env:USERPROFILE\.unrealdevflow\bin",
    [switch]$AddToPath
)

$ErrorActionPreference = "Stop"

Write-Host "UnrealDevFlow 安装脚本" -ForegroundColor Cyan
Write-Host "========================" -ForegroundColor Cyan
Write-Host ""

# 1. 检查 Rust 环境
Write-Host "[1/5] 检查 Rust 环境..." -ForegroundColor Yellow
try {
    $rustc = rustc --version
    Write-Host "  ✓ Rust 已安装：$rustc" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Rust 未安装" -ForegroundColor Red
    Write-Host "  请先安装 Rust: https://rustup.rs/" -ForegroundColor Yellow
    Write-Host "  或运行：winget install Rustlang.Rustup" -ForegroundColor Yellow
    exit 1
}

# 2. 检查 Git
Write-Host "[2/5] 检查 Git 环境..." -ForegroundColor Yellow
try {
    $git = git --version
    Write-Host "  ✓ Git 已安装：$git" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Git 未安装" -ForegroundColor Red
    Write-Host "  请先安装 Git: https://git-scm.com/" -ForegroundColor Yellow
    exit 1
}

# 3. 编译
Write-Host "[3/5] 编译 UnrealDevFlow..." -ForegroundColor Yellow
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $projectRoot

try {
    cargo build --release 2>&1 | Out-Null
    Write-Host "  ✓ 编译成功" -ForegroundColor Green
} catch {
    Write-Host "  ✗ 编译失败：$_" -ForegroundColor Red
    exit 1
}

# 4. 复制到安装目录
Write-Host "[4/5] 安装到 $InstallPath..." -ForegroundColor Yellow
if (-not (Test-Path $InstallPath)) {
    New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
}

$exeSource = "$projectRoot\target\release\unrealdevflow.exe"
$exeTarget = "$InstallPath\unrealdevflow.exe"

Copy-Item -Path $exeSource -Destination $exeTarget -Force
Write-Host "  ✓ 已复制 unrealdevflow.exe" -ForegroundColor Green

# 5. 添加到 PATH
if ($AddToPath) {
    Write-Host "[5/5] 添加到 PATH..." -ForegroundColor Yellow
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$InstallPath*") {
        [Environment]::SetEnvironmentVariable("Path", "$currentPath;$InstallPath", "User")
        Write-Host "  ✓ 已添加到用户 PATH" -ForegroundColor Green
        Write-Host "  ⚠ 需要重启终端才能生效" -ForegroundColor Yellow
    } else {
        Write-Host "  ✓ 已在 PATH 中" -ForegroundColor Green
    }
} else {
    Write-Host "[5/5] 跳过 PATH 添加（使用 -AddToPath 参数启用）" -ForegroundColor Yellow
}

# 验证安装
Write-Host ""
Write-Host "验证安装..." -ForegroundColor Cyan
try {
    & $exeTarget --version
    Write-Host "✓ 安装成功！" -ForegroundColor Green
} catch {
    Write-Host "✗ 验证失败：$_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "下一步：" -ForegroundColor Cyan
Write-Host "  1. 运行：unrealdevflow configure" -ForegroundColor White
Write-Host "  2. 或查看帮助：unrealdevflow --help" -ForegroundColor White
Write-Host ""
