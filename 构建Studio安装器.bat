@echo off
setlocal
cd /d "%~dp0"
echo Building Codex Chat Gateway Studio installer...
powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-tauri-installer.ps1"
if errorlevel 1 (
  echo Build failed.
  exit /b 1
)
echo.
echo Done. See dist-installer\
endlocal
