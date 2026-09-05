[CmdletBinding()]
param(
    [ValidateSet('Host', 'Shells', 'All')]
    [string]$Suite = 'All',
    [Parameter(Mandatory = $true)]
    [string]$UdfPath,
    [Parameter(Mandatory = $true)]
    [string]$Workspace,
    [Parameter(Mandatory = $true)]
    [string]$Task,
    [Parameter(Mandatory = $true)]
    [string]$EvidenceRoot
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$udf = (Resolve-Path $UdfPath).Path
$evidence = [IO.Path]::GetFullPath($EvidenceRoot)
if (Test-Path -LiteralPath $evidence) {
    throw "EvidenceRoot 已存在，拒绝复用：$evidence"
}
New-Item -ItemType Directory -Force -Path $evidence | Out-Null
$configSource = Join-Path $env:USERPROFILE '.unrealdevflow\config.toml'
if (-not (Test-Path -LiteralPath $configSource)) {
    throw "找不到用户 UDF 配置：$configSource"
}
$isolatedConfig = Join-Path $evidence 'config'
New-Item -ItemType Directory -Force -Path $isolatedConfig | Out-Null
Copy-Item -LiteralPath $configSource -Destination (Join-Path $isolatedConfig 'config.toml')
$oldConfigEnv = $env:UNREALDEVFLOW_CONFIG_DIR
$env:UNREALDEVFLOW_CONFIG_DIR = $isolatedConfig

function Invoke-UdfJson {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [switch]$Async
    )
    $outPath = Join-Path $evidence "$Label.stdout.json"
    $errPath = Join-Path $evidence "$Label.stderr.log"
    # Keep arguments as an array.  Passing a bare `-test=UE.EditorBootTest`
    # token through PowerShell can split the dotted name; the native argv is
    # the contract this script is intended to prove.
    if ($Async) {
        # A started Game keeps its child process alive.  Start-Process lets
        # this wrapper wait only for the short-lived UDF parent and then use
        # the execution record/PID for the actual map probe.
        $udfProcess = Start-Process -FilePath $udf -ArgumentList $Arguments `
            -RedirectStandardOutput $outPath -RedirectStandardError $errPath `
            -PassThru -WindowStyle Hidden
        for ($attempt = 0; $attempt -lt 60; $attempt++) {
            $udfProcess.Refresh()
            if ($udfProcess.HasExited) { break }
            Start-Sleep -Milliseconds 500
        }
        $udfProcess.Refresh()
        if (-not $udfProcess.HasExited) {
            Stop-Process -Id $udfProcess.Id -Force -ErrorAction SilentlyContinue
            throw "$Label 的 UDF 父进程没有在 30 秒内返回"
        }
        $exitCode = $udfProcess.ExitCode
    } else {
        & $udf @Arguments 1> $outPath 2> $errPath
        $exitCode = $LASTEXITCODE
    }
    $raw = Get-Content -Raw -LiteralPath $outPath
    try {
        $json = $raw | ConvertFrom-Json
    } catch {
        throw "$Label 没有输出单一 JSON：$($_.Exception.Message)；详见 $outPath"
    }
    [pscustomobject]@{ label = $Label; exitCode = $exitCode; json = $json; stdout = $outPath; stderr = $errPath }
}

function Register-Profile {
    param([string]$Name, [string]$ScopeArg, [string]$File)
    $candidate = (Resolve-Path (Join-Path $repo $File)).Path
    $scopeValue = if ($ScopeArg -eq '--task') { $Task } else { $Workspace }
    $result = Invoke-UdfJson "configure-$Name" @('--format', 'json', 'run', 'configure', $Name, $ScopeArg, $scopeValue, '--file', $candidate)
    if (-not $result.json.ok) {
        throw "配置 $Name 失败；详见 $($result.stderr)"
    }
    return $result
}

function Stop-OwnedEditor {
    param([Parameter(Mandatory = $true)][int]$ProcessId)
    $children = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
        Where-Object { $_.ParentProcessId -eq $ProcessId } |
        Select-Object -ExpandProperty ProcessId)
    foreach ($child in $children) { Stop-OwnedEditor -ProcessId $child }
    Get-Process -Id $ProcessId -ErrorAction SilentlyContinue | Stop-Process -Force
    for ($attempt = 0; $attempt -lt 20; $attempt++) {
        if (-not (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)) { break }
        Start-Sleep -Milliseconds 250
    }
}

function Wait-ForGameMap {
    param(
        [Parameter(Mandatory = $true)]$Result,
        [Parameter(Mandatory = $true)][string]$Label
    )
    if ($Result.exitCode -ne 0 -or -not $Result.json.ok -or $Result.json.data.state -ne 'started') {
        throw "$Label 没有返回 started；详见 $($Result.stderr)"
    }
    $processId = [int]$Result.json.data.pid
    $logPath = [string]($Result.json.data.artifacts | Where-Object kind -eq 'nativeLog' | Select-Object -First 1 -ExpandProperty path)
    $map = '/Game/Maps/UGA_local/aes6_sh_sz_q1'
    $loaded = $false
    try {
        for ($attempt = 0; $attempt -lt 36; $attempt++) {
            $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
            if (-not $process) { throw "$Label 的 Editor 在地图确认前退出，PID=$processId" }
            if (Test-Path -LiteralPath $logPath) {
                $text = Get-Content -Raw -LiteralPath $logPath -ErrorAction SilentlyContinue
                if ($text -match [regex]::Escape("LogLoad: LoadMap: $map") -or
                    $text -match [regex]::Escape("LogWorld: Bringing World $map")) {
                    $loaded = $true
                    $evidence = Join-Path $evidence "$Label.map-loaded.txt"
                    "PID=$processId`nMap=$map`nLog=$logPath`n" | Set-Content -Encoding UTF8 -LiteralPath $evidence
                    break
                }
            }
            Start-Sleep -Seconds 5
        }
        if (-not $loaded) { throw "$Label 在 180 秒内没有从引擎日志确认地图 $map；日志：$logPath" }
        return [pscustomobject]@{ label = $Label; pid = $processId; log = $logPath; map = $map; evidence = (Join-Path $evidence "$Label.map-loaded.txt") }
    } finally {
        # Game smoke is an interaction probe, not a natural-exit assertion.
        # Terminate only the process UDF just reported after the map evidence
        # has been written; never use this result to mark a test passed.
        Stop-OwnedEditor -ProcessId $processId
    }
}

try {
    $hostResults = @()
    if ($Suite -in @('Host', 'All')) {
        $hostResults += Register-Profile 'editor-exit' '--task' 'workflow/ue-test-run-cli/assets/editor-exit-native-v3.json'
        $hostResults += Register-Profile 'editor-boot' '--task' 'tests/fixtures/run/editor-boot.json'
        $hostResults += Register-Profile 'automation-smoke' '--task' 'tests/fixtures/run/editor-automation.json'
        foreach ($name in @('editor-boot', 'editor-exit', 'automation-smoke')) {
            $result = Invoke-UdfJson "host-start-$name" @('--format', 'json', 'run', 'start', $name, '--task', $Task)
            if ($result.exitCode -ne 0 -or -not $result.json.ok) {
                throw "Host 运行 $name 失败；详见 $($result.stderr)"
            }
            $hostResults += $result
        }
    }

    $shellResults = @()
    if ($Suite -in @('Shells', 'All')) {
        $shellResults += Register-Profile 'perflab-game' '--workspace' 'workflow/ue-test-run-cli/assets/perflab-game-native-v3.json'
        $psResult = Invoke-UdfJson 'shell-powershell-plan' @('--format', 'json', 'run', 'plan', 'perflab-game', '--workspace', $Workspace)
        if ($psResult.exitCode -ne 0 -or -not $psResult.json.ok) { throw 'PowerShell plan 失败' }
        $shellResults += $psResult

        $cmdOut = Join-Path $evidence 'shell-cmd-plan.stdout.json'
        $cmdErr = Join-Path $evidence 'shell-cmd-plan.stderr.log'
        $cmdLine = "`"$udf`" --format json run plan perflab-game --workspace $Workspace 1>`"$cmdOut`" 2>`"$cmdErr`""
        cmd.exe /d /s /c $cmdLine
        $cmdCode = $LASTEXITCODE
        if ($cmdCode -ne 0) { throw "cmd.exe plan 失败，exit=$cmdCode" }
        $shellResults += [pscustomobject]@{ label = 'shell-cmd-plan'; exitCode = $cmdCode; stdout = $cmdOut; stderr = $cmdErr }

        $bash = Get-Command bash.exe -ErrorAction SilentlyContinue
        if (-not $bash) {
            $bashPath = @(
                (Join-Path ${env:ProgramFiles} 'Git\bin\bash.exe'),
                (Join-Path ${env:ProgramFiles} 'Git\usr\bin\bash.exe')
            ) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
            if ($bashPath) { $bash = Get-Item -LiteralPath $bashPath }
        }
        if (-not $bash) { throw '找不到 bash.exe，Shells 套件不能声称三种 shell 已验证' }
        $bashOut = Join-Path $evidence 'shell-git-bash-plan.stdout.json'
        $bashErr = Join-Path $evidence 'shell-git-bash-plan.stderr.log'
        $bashCommand = 'export MSYS_NO_PATHCONV=1; "' + $udf + '" --format json run plan perflab-game --workspace ' + $Workspace
        $bashExecutable = if ($bash.PSObject.Properties.Name -contains 'Source') { $bash.Source } else { $bash.FullName }
        & $bashExecutable -lc $bashCommand 1> $bashOut 2> $bashErr
        $bashCode = $LASTEXITCODE
        if ($bashCode -ne 0) { throw "Git Bash plan 失败，exit=$bashCode" }
        $shellResults += [pscustomobject]@{ label = 'shell-git-bash-plan'; exitCode = $bashCode; stdout = $bashOut; stderr = $bashErr }

        $psStart = Invoke-UdfJson 'shell-powershell-start' @('--format', 'json', 'run', 'start', 'perflab-game', '--workspace', $Workspace) -Async
        $shellResults += Wait-ForGameMap -Result $psStart -Label 'shell-powershell-start'

        $cmdStartOut = Join-Path $evidence 'shell-cmd-start.stdout.json'
        $cmdStartErr = Join-Path $evidence 'shell-cmd-start.stderr.log'
        $cmdStartLine = "start `"`" /b `"$udf`" --format json run start perflab-game --workspace $Workspace 1>`"$cmdStartOut`" 2>`"$cmdStartErr`""
        cmd.exe /d /s /c $cmdStartLine
        $cmdStartCode = $LASTEXITCODE
        if ($cmdStartCode -ne 0) { throw "cmd.exe start 失败，exit=$cmdStartCode" }
        for ($attempt = 0; $attempt -lt 60 -and (!(Test-Path -LiteralPath $cmdStartOut) -or (Get-Item -LiteralPath $cmdStartOut).Length -eq 0); $attempt++) { Start-Sleep -Milliseconds 500 }
        $cmdStartJson = Get-Content -Raw -LiteralPath $cmdStartOut | ConvertFrom-Json
        $shellResults += Wait-ForGameMap -Result ([pscustomobject]@{ exitCode = $cmdStartCode; json = $cmdStartJson; stderr = $cmdStartErr }) -Label 'shell-cmd-start'

        $bashStartOut = Join-Path $evidence 'shell-git-bash-start.stdout.json'
        $bashStartErr = Join-Path $evidence 'shell-git-bash-start.stderr.log'
        $bashStartBootstrapOut = Join-Path $evidence 'shell-git-bash-start.bootstrap.log'
        $bashStartBootstrapErr = Join-Path $evidence 'shell-git-bash-start.bootstrap.err'
        $bashStartCommand = 'export MSYS_NO_PATHCONV=1; ("' + $udf + '" --format json run start perflab-game --workspace ' + $Workspace + ' > "' + $bashStartOut + '" 2> "' + $bashStartErr + '" < /dev/null &)'
        & $bashExecutable -lc $bashStartCommand 1> $bashStartBootstrapOut 2> $bashStartBootstrapErr
        $bashStartCode = 0
        for ($attempt = 0; $attempt -lt 60 -and (!(Test-Path -LiteralPath $bashStartOut) -or (Get-Item -LiteralPath $bashStartOut).Length -eq 0); $attempt++) { Start-Sleep -Milliseconds 500 }
        $bashStartJson = Get-Content -Raw -LiteralPath $bashStartOut | ConvertFrom-Json
        $shellResults += Wait-ForGameMap -Result ([pscustomobject]@{ exitCode = $bashStartCode; json = $bashStartJson; stderr = $bashStartErr }) -Label 'shell-git-bash-start'

        foreach ($result in @($psResult, $cmdStartJson, $bashStartJson)) {
            foreach ($path in @($result.data.nativeArgv)) {
                if ([string]$path -match 'Git[\\/]Game') { throw "发现 Git Bash 路径转换污染：$path" }
            }
        }
    }

    [pscustomobject]@{
        suite = $Suite
        workspace = $Workspace
        task = $Task
        evidenceRoot = $evidence
        host = $hostResults | ForEach-Object { $_.label }
        shells = $shellResults | ForEach-Object { $_.label }
    } | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 (Join-Path $evidence 'summary.json')
    Write-Output "PASS: $Suite evidence=$evidence"
} finally {
    if ($null -eq $oldConfigEnv) { Remove-Item Env:UNREALDEVFLOW_CONFIG_DIR -ErrorAction SilentlyContinue }
    else { $env:UNREALDEVFLOW_CONFIG_DIR = $oldConfigEnv }
}
