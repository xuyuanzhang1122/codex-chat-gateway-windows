@echo off
setlocal
cd /d "%~dp0"
echo.
echo === Building STUDIO installer (Tauri + LobeHub) ===
echo NOT the legacy C#/WPF package.
echo.
powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-tauri-installer.ps1" %*
set ERR=%ERRORLEVEL%
if not "%ERR%"=="0" (
  echo.
  echo Build FAILED with exit code %ERR%.
  exit /b %ERR%
)
echo.
echo Open dist-installer\CodexChatGateway-Studio-Setup-v*.exe
endlocal
