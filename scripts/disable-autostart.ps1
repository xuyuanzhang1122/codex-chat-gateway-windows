$ErrorActionPreference = 'Stop'
$shortcutPath = Join-Path ([Environment]::GetFolderPath('Startup')) 'Codex Chat Gateway.lnk'
if (Test-Path -LiteralPath $shortcutPath) { Remove-Item -LiteralPath $shortcutPath -Force; Write-Host 'Autostart disabled.' }
else { Write-Host 'Autostart was not enabled.' }
