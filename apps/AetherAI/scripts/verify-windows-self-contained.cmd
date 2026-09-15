@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0verify-windows-self-contained.ps1" %*
exit /b %ERRORLEVEL%
