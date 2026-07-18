@echo off
setlocal
cd /d "%~dp0desktop-tauri"
if not exist "node_modules\" (
  echo Installing npm dependencies...
  call npm install
  if errorlevel 1 exit /b 1
)
call npm run tauri dev
endlocal
