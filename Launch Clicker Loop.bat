@echo off
cd /d "%~dp0"
if exist "%~dp0target\release\clicker-loop.exe" (
    start "" "%~dp0target\release\clicker-loop.exe"
) else (
    cargo build --release
    if errorlevel 1 (
        pause
        exit /b 1
    )
    start "" "%~dp0target\release\clicker-loop.exe"
)
