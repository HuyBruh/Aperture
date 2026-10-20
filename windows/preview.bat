@echo off
cd /d "%~dp0"
if not exist "_preview" mkdir "_preview"
trofeo_lcd.exe --preview-png "_preview\preview.png" --preview-state music > "_preview\preview.log" 2>&1
start "" "_preview\preview.png"
