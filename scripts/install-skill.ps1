# UnrealDevFlow Skill 安装脚本
# 将 skill 链接到 opencode 的 skills 目录

param(
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

# 获取脚本所在目录的父目录（项目根目录）
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$SkillSource = Join-Path $ProjectRoot "skill"

# opencode skills 目录
$OpencodeSkillsDir = Join-Path $env:USERPROFILE ".config\opencode\skills"
$SkillTarget = Join-Path $OpencodeSkillsDir "UnrealDevFlow"

if ($Uninstall) {
    Write-Host "Uninstalling UnrealDevFlow skill..." -ForegroundColor Yellow
    
    if (Test-Path $SkillTarget) {
        if ((Get-Item $SkillTarget).Attributes -band [IO.FileAttributes]::ReparsePoint) {
            # 是符号链接，直接删除
            cmd /c rmdir $SkillTarget
            Write-Host "Removed symlink: $SkillTarget" -ForegroundColor Green
        } else {
            # 是普通目录，递归删除
            Remove-Item -Path $SkillTarget -Recurse -Force
            Write-Host "Removed directory: $SkillTarget" -ForegroundColor Green
        }
    } else {
        Write-Host "Skill not found at: $SkillTarget" -ForegroundColor Yellow
    }
    
    Write-Host "UnrealDevFlow skill uninstalled." -ForegroundColor Green
    exit 0
}

Write-Host "Installing UnrealDevFlow skill..." -ForegroundColor Cyan
Write-Host "Project root: $ProjectRoot"
Write-Host "Skill source: $SkillSource"
Write-Host "Skill target: $SkillTarget"

# 检查源目录是否存在
if (-not (Test-Path $SkillSource)) {
    Write-Error "Skill source directory not found: $SkillSource"
    exit 1
}

# 确保 opencode skills 目录存在
if (-not (Test-Path $OpencodeSkillsDir)) {
    Write-Host "Creating opencode skills directory..." -ForegroundColor Yellow
    New-Item -ItemType Directory -Path $OpencodeSkillsDir -Force | Out-Null
}

# 如果目标已存在，先删除
if (Test-Path $SkillTarget) {
    Write-Host "Removing existing skill..." -ForegroundColor Yellow
    if ((Get-Item $SkillTarget).Attributes -band [IO.FileAttributes]::ReparsePoint) {
        cmd /c rmdir $SkillTarget
    } else {
        Remove-Item -Path $SkillTarget -Recurse -Force
    }
}

# 创建符号链接（需要管理员权限或开发者模式）
Write-Host "Creating symlink..." -ForegroundColor Yellow
try {
    cmd /c mklink /D $SkillTarget $SkillSource
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to create symlink"
    }
    Write-Host "Symlink created successfully!" -ForegroundColor Green
} catch {
    Write-Warning "Failed to create symlink. Falling back to copy..."
    Copy-Item -Path $SkillSource -Destination $SkillTarget -Recurse -Force
    Write-Host "Skill copied successfully!" -ForegroundColor Green
}

Write-Host ""
Write-Host "UnrealDevFlow skill installed successfully!" -ForegroundColor Green
Write-Host "Skill location: $SkillTarget" -ForegroundColor Cyan
Write-Host ""
Write-Host "To uninstall, run:" -ForegroundColor Yellow
Write-Host "  .\scripts\install-skill.ps1 -Uninstall" -ForegroundColor White
