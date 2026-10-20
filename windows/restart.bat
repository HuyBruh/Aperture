@echo off
cd /d "%~dp0"
net session >nul 2>&1
if %errorlevel% neq 0 (
  powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
  exit /b
)
taskkill /im trofeo_lcd.exe /f >nul 2>&1
timeout /t 1 >nul
schtasks /run /tn TrofeoLCD >nul 2>&1 || start "" trofeo_lcd.exe --hide-console --keep-alive --idle-fps 4 --active-fps 4
