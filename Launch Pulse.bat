@echo off
cd /d "%~dp0"
if exist "%~dp0target\release\pulse.exe" (
    start "" "%~dp0target\release\pulse.exe"
) else (
    cargo build --release
    if errorlevel 1 (
        pause
        exit /b 1
    )
    start "" "%~dp0target\release\pulse.exe"
)
