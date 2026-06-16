#Requires -Version 5.1
param(
    [ValidateSet('status', 'capabilities', 'commands', 'schema')]
    [string]$Command = 'status'
)

$ErrorActionPreference = 'Stop'
$script:Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[Console]::OutputEncoding = $script:Utf8NoBom
$OutputEncoding = $script:Utf8NoBom

function Join-CodePointText {
    param([int[]]$CodePoints)
    -join ($CodePoints | ForEach-Object { [char]$_ })
}

$scriptRoot = Split-Path -Path $PSCommandPath -Parent
$repoRoot = Resolve-Path -LiteralPath (Join-Path $scriptRoot '..')
$displayName = Join-CodePointText @(0x5F00, 0x53D1, 0x6D41)

function Get-CapabilityList {
    @(
        [ordered]@{ name = 'workspaceStatus'; safety = 'readOnly'; summary = 'Read registered UE workspace and config state' },
        [ordered]@{ name = 'taskStatus'; safety = 'readOnly'; summary = 'Read active task and task inventory state' },
        [ordered]@{ name = 'junctionStatus'; safety = 'readOnly'; summary = 'Read managed Junction summary without changing links' },
        [ordered]@{ name = 'futureTaskCreate'; safety = 'boundedWrite'; summary = 'Future phase may create or switch UE tasks; disabled in this phase' }
    )
}

function Get-CommandList {
    @(
        [ordered]@{ name = 'status'; displayName = 'Status'; safety = 'readOnly'; resultKind = 'AgentWatcherModuleResult' },
        [ordered]@{ name = 'capabilities'; displayName = 'Capabilities'; safety = 'readOnly'; resultKind = 'AgentWatcherModuleResult' },
        [ordered]@{ name = 'commands'; displayName = 'Commands'; safety = 'readOnly'; resultKind = 'AgentWatcherModuleResult' },
        [ordered]@{ name = 'schema'; displayName = 'Schema'; safety = 'readOnly'; resultKind = 'AgentWatcherModuleResult' }
    )
}

function Get-StatusDetails {
    $configPath = Join-Path $env:USERPROFILE '.unrealdevflow\config.toml'
    [ordered]@{
        displayName = $displayName
        role = 'developmentFlow'
        sourceProject = 'UnrealDevFlow'
        repoRoot = $repoRoot.Path
        checks = [ordered]@{
            cargoToml = Test-Path -LiteralPath (Join-Path $repoRoot 'Cargo.toml') -PathType Leaf
            sourceDir = Test-Path -LiteralPath (Join-Path $repoRoot 'src') -PathType Container
            localConfig = Test-Path -LiteralPath $configPath -PathType Leaf
        }
        configPath = $configPath
        capabilities = Get-CapabilityList
    }
}

function Get-SchemaDetails {
    [ordered]@{
        displayName = $displayName
        role = 'developmentFlow'
        resultKind = 'AgentWatcherModuleResult'
        resultFields = @('schemaVersion', 'kind', 'moduleName', 'commandName', 'status', 'summary', 'exitCode', 'details')
        commands = Get-CommandList
        detailContracts = [ordered]@{
            status = @('displayName', 'role', 'sourceProject', 'repoRoot', 'checks', 'configPath', 'capabilities')
            capabilities = @('displayName', 'role', 'sourceProject', 'capabilities')
            commands = @('displayName', 'role', 'sourceProject', 'commands')
            schema = @('displayName', 'role', 'resultKind', 'resultFields', 'commands', 'detailContracts')
        }
    }
}

function New-ModuleResult {
    param(
        [string]$Name,
        $Details,
        [string]$Summary
    )
    [ordered]@{
        schemaVersion = 1
        kind = 'AgentWatcherModuleResult'
        moduleName = 'UnrealDevFlow'
        commandName = $Name
        status = 'success'
        summary = $Summary
        exitCode = 0
        details = $Details
    }
}

switch ($Command) {
    'status' {
        $result = New-ModuleResult -Name $Command -Summary ($displayName + ' status completed') -Details (Get-StatusDetails)
    }
    'capabilities' {
        $result = New-ModuleResult -Name $Command -Summary ($displayName + ' capabilities loaded') -Details ([ordered]@{
            displayName = $displayName
            role = 'developmentFlow'
            sourceProject = 'UnrealDevFlow'
            capabilities = Get-CapabilityList
        })
    }
    'commands' {
        $result = New-ModuleResult -Name $Command -Summary ($displayName + ' commands loaded') -Details ([ordered]@{
            displayName = $displayName
            role = 'developmentFlow'
            sourceProject = 'UnrealDevFlow'
            commands = Get-CommandList
        })
    }
    'schema' {
        $result = New-ModuleResult -Name $Command -Summary ($displayName + ' schema loaded') -Details (Get-SchemaDetails)
    }
}

$result | ConvertTo-Json -Depth 10
