@echo off
cd /d "%~dp0..\"
net session >nul 2>&1
if %errorlevel% neq 0 (
  powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
  exit /b
)
taskkill /im aperture-display.exe /f >nul 2>&1
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0..\scripts\uninstall-autostart.ps1"
echo Autostart removed and aperture-display stopped.
timeout /t 8
