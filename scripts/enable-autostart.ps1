$ErrorActionPreference = 'Stop'
if (-not $PSScriptRoot) {
    if ($MyInvocation.MyCommand.Path) {
        $PSScriptRoot = Split-Path -Parent -LiteralPath $MyInvocation.MyCommand.Path
    } elseif ($env:CODEX_CHAT_GATEWAY_ROOT) {
        $PSScriptRoot = Join-Path $env:CODEX_CHAT_GATEWAY_ROOT 'scripts'
    } else {
        throw 'PSScriptRoot is empty; cannot locate scripts directory.'
    }
}
$projectRoot = if ($env:CODEX_CHAT_GATEWAY_ROOT) { $env:CODEX_CHAT_GATEWAY_ROOT } else { Split-Path -Parent $PSScriptRoot }
$modelsPath = Join-Path $projectRoot '.gateway\models.json'
if (-not (Test-Path -LiteralPath $modelsPath)) {
    Write-Host 'Configure a model in Studio before enabling autostart.'
    exit 1
}
$startup = [Environment]::GetFolderPath('Startup')
$shortcutPath = Join-Path $startup 'Codex Chat Gateway.lnk'
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = (Join-Path $PSHOME 'powershell.exe')
$scriptPath = Join-Path $PSScriptRoot 'start-background.ps1'
$shortcut.Arguments = "-NoLogo -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$scriptPath`" -NonInteractive"
$shortcut.WorkingDirectory = $projectRoot
$shortcut.WindowStyle = 7
$shortcut.Description = 'Start Codex Chat Gateway in the background'
$shortcut.Save()
Write-Host "Autostart enabled: $shortcutPath"
