@echo off
setlocal
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0..\scripts\restore-codex.ps1"
if errorlevel 1 pause
