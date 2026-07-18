@echo off
setlocal
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0..\scripts\model-manager.ps1"
if errorlevel 1 pause
