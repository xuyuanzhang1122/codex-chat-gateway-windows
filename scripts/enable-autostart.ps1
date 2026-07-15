$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'model-store.ps1')
try { Set-DefaultModelEnvironment -ProjectRoot $projectRoot | Out-Null }
catch { Write-Host $_.Exception.Message; Write-Host 'Configure a model before enabling autostart.'; exit 1 }
$startup = [Environment]::GetFolderPath('Startup')
$shortcutPath = Join-Path $startup 'Codex Chat Gateway.lnk'
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = (Join-Path $PSHOME 'powershell.exe')
$scriptPath = Join-Path $PSScriptRoot 'start-background.ps1'
$shortcut.Arguments = "-NoLogo -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$scriptPath`""
$shortcut.WorkingDirectory = $projectRoot
$shortcut.WindowStyle = 7
$shortcut.Description = 'Start Codex Chat Gateway in the background'
$shortcut.Save()
Write-Host "Autostart enabled: $shortcutPath"
