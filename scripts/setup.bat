@echo off
REM Felex Setup Script (Batch wrapper for PowerShell)
echo.
echo Felex Setup - Checking and installing dependencies...
echo.

REM Check if PowerShell is available
where powershell >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo PowerShell not found. Please install PowerShell or run setup.ps1 manually.
    pause
    exit /b 1
)

REM Run PowerShell setup script
powershell -ExecutionPolicy Bypass -File "%~dp0setup.ps1"

pause
