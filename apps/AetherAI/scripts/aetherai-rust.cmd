@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0aetherai-rust.ps1" %*
exit /b %ERRORLEVEL%
