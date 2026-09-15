@echo off
setlocal
for %%I in ("%~dp0..") do set "ROOT=%%~fI"
set "TOOLCHAIN=%ROOT%\tools\rust\x86_64-pc-windows-gnu"
"%TOOLCHAIN%\bin\rustdoc.exe" --sysroot "%TOOLCHAIN%" %*
exit /b %ERRORLEVEL%
