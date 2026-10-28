# Exercise only the portable channel-selection helper, without registry or installs.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$source = Join-Path $PSScriptRoot "windows_install_conpty_package_test.ps1"
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($source, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count -ne 0) {
    throw ($parseErrors | Out-String)
}
$definition = $ast.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $node.Name -eq "Test-AiraUpdateChannel"
}, $true) | Select-Object -First 1
if ($null -eq $definition) {
    throw "missing portable Aira channel-selection helper"
}
$calls = @($ast.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.CommandAst] -and
        $node.GetCommandName() -eq "Test-AiraUpdateChannel"
}, $true))
if ($calls.Count -ne 1) {
    throw "installer test must select its Aira branch through the channel helper"
}
Invoke-Expression $definition.Extent.Text

# Every fixture deliberately rejects --version so future identity changes cannot
# silently bypass the Aira-specific installer assertions.
function Invoke-FixtureHerdr {
    param([string]$Command, [string]$Subcommand)
    if ($Command -ne "channel" -or $Subcommand -ne "show") {
        throw "channel selection must query channel show, not the version label"
    }
    $global:LASTEXITCODE = $script:fixtureExitCode
    return $script:fixtureChannel
}

$script:fixtureExitCode = 0
foreach ($channel in @("aira (three4c/herdr-aira)", "aira", "stable", "preview", "not-aira")) {
    $script:fixtureChannel = $channel
    $expected = $channel -in @("aira (three4c/herdr-aira)", "aira")
    if ((Test-AiraUpdateChannel -ExePath Invoke-FixtureHerdr) -ne $expected) {
        throw "incorrect installer branch for channel '$channel'"
    }
}
$script:fixtureChannel = "aira (three4c/herdr-aira)"
$script:fixtureExitCode = 17
$rejected = $false
try {
    Test-AiraUpdateChannel -ExePath Invoke-FixtureHerdr | Out-Null
} catch {
    if ($_.Exception.Message -ne "could not read the installed fixture binary update channel") {
        throw
    }
    $rejected = $true
}
if (-not $rejected) {
    throw "channel-query failure did not stop the installer test"
}
Write-Output "PASS: Aira installer branch follows channel show independently of version syntax"
