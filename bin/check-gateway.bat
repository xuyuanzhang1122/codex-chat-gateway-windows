@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0..\scripts\check.ps1"
if errorlevel 1 pause

