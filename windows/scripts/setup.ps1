# Trofeo Pro Dashboard - install: stop old instances, register autostart, start now.
$ErrorActionPreference = 'Continue'
$dir = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$exe = Join-Path $dir 'trofeo_lcd.exe'
$log = Join-Path $dir 'install_log.txt'
Start-Transcript -Path $log -Force | Out-Null
Write-Host "=== Trofeo Pro Dashboard setup  $(Get-Date)  folder: $dir"

# 1. Remove the "downloaded from the Internet" mark so SmartScreen does not block autostart.
Get-ChildItem $dir -Recurse -File | Unblock-File -ErrorAction SilentlyContinue

# 2. Stop any running trofeo instance (two programs cannot share the USB panel).
Get-Process trofeo_lcd, trofeo_screen -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "Stopping process $($_.Id): $($_.Path)"
    Stop-Process -Id $_.Id -Force
}
Start-Sleep -Seconds 1

# 3. Remove previous trofeo autostart tasks.
Get-ScheduledTask -ErrorAction SilentlyContinue | Where-Object {
    ((($_.Actions | ForEach-Object { "$($_.Execute) $($_.Arguments) $($_.WorkingDirectory)" }) -join ' ') -match 'trofeo')
} | ForEach-Object {
    Write-Host "Removing old scheduled task: $($_.TaskPath)$($_.TaskName)"
    Unregister-ScheduledTask -TaskName $_.TaskName -TaskPath $_.TaskPath -Confirm:$false
}

# 4. Register the new autostart (elevated Scheduled Task at logon, hidden console, auto-restart).
Write-Host "`n=== Installing autostart ==="
& (Join-Path $dir 'scripts\install-autostart.ps1') -Exe $exe -Arguments '--hide-console --keep-alive --idle-fps 4 --active-fps 4' -Method Task -TaskName 'TrofeoLCD'

# 5. Start now.
Start-ScheduledTask -TaskName 'TrofeoLCD'
Start-Sleep -Seconds 6
$p = Get-Process trofeo_lcd -ErrorAction SilentlyContinue
if ($p) { Write-Host "RUNNING: trofeo_lcd PID $($p.Id -join ',')" } else { Write-Host "WARNING: trofeo_lcd is NOT running (see trofeo-errors.txt)" }
$err = Join-Path $dir 'trofeo-errors.txt'
if (Test-Path $err) { Write-Host "`n=== trofeo-errors.txt ==="; Get-Content $err | Select-Object -Last 20 | ForEach-Object { Write-Host $_ } }
Write-Host "=== DONE ==="
Stop-Transcript | Out-Null
