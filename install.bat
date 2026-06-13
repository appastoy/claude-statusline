@echo off
where pwsh >nul 2>&1
if %errorlevel% equ 0 (
    pwsh -NoProfile -ExecutionPolicy Bypass -File "%~dp0script\install.ps1" %*
) else (
    powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0script\install.ps1" %*
)
if errorlevel 1 exit /b %errorlevel%
