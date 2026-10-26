@echo off
cd /d "%~dp0"
net session >nul 2>&1
if %errorlevel% neq 0 (
  powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
  exit /b
)
echo Installing Trofeo Pro Dashboard (autostart + launch)...
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\setup.ps1"
echo.
echo Done. The dashboard is running and will start at every Windows logon.
echo Opening HuyBruh Dash: pick a layout, colors and features there.
start "" "%~dp0huybruh_dash.exe"
timeout /t 10
