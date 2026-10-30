# Aperture - install: checks, sensor driver, autostart, start now.
$ErrorActionPreference = 'Continue'
$dir = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$exe = Join-Path $dir 'aperture-display.exe'
$log = Join-Path $dir 'install_log.txt'

function Say($text, $color = 'Gray') { Write-Host $text -ForegroundColor $color }

# 0. Wrong download: the source code ZIP has no program inside.
if (-not (Test-Path $exe)) {
    Say ''
    Say '  aperture-display.exe was not found in this folder.' Red
    Say '  You probably downloaded the SOURCE CODE (Code > Download ZIP).' Yellow
    Say '  Download  aperture-windows.zip  from the Releases page instead:' Yellow
    Say '  https://github.com/HuyBruh/Aperture/releases/latest' Cyan
    Say ''
    Read-Host '  Press Enter to close'
    exit 1
}

Start-Transcript -Path $log -Force | Out-Null
Say "=== Aperture setup  $(Get-Date)  folder: $dir" Cyan

# 1. Remove the "downloaded from the Internet" mark so SmartScreen does not block autostart.
Get-ChildItem $dir -Recurse -File | Unblock-File -ErrorAction SilentlyContinue

# 2. Is the display connected? (Trofeo Vision 9.16" = USB 0416:5408 / 0416:5409)
$lcd = Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue | Where-Object { $_.InstanceId -match 'VID_0416&PID_540[89]' }
if ($lcd) {
    Say "Display found: $($lcd[0].FriendlyName)" Green
} else {
    Say 'WARNING: the Trofeo Vision 9.16" display was not found on USB.' Yellow
    Say '  - Check the USB cable (use a motherboard header / rear port).' Yellow
    Say '  - Only the 9.16" (1920x462) panel is supported; the 6.86" model is NOT.' Yellow
}

# 3. Thermalright's own software keeps the display busy: two programs cannot drive it.
$trcc = Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -match '^TRCC' }
if ($trcc) {
    Say 'Closing Thermalright TRCC (it blocks the display). Remove it from Windows startup too.' Yellow
    $trcc | Stop-Process -Force -ErrorAction SilentlyContinue
}

# 4. PawnIO: the signed driver that reads CPU temperature and power (AMD Ryzen and Intel Core).
$pawn = Get-Service -Name PawnIO -ErrorAction SilentlyContinue
if ($pawn) {
    Say 'PawnIO (CPU temperature driver): installed' Green
} else {
    Say 'Installing PawnIO (CPU temperature driver)...' Cyan
    $winget = Get-Command winget -ErrorAction SilentlyContinue
    if ($winget) {
        & winget install --id namazso.PawnIO -e --silent --accept-source-agreements --accept-package-agreements | Out-Host
    }
    if (Get-Service -Name PawnIO -ErrorAction SilentlyContinue) {
        Say 'PawnIO installed.' Green
    } else {
        Say 'Could not install PawnIO automatically. Install it from https://pawnio.eu then run scripts\install.bat again.' Yellow
        Say '(Without it the CPU temperature can still come from HWiNFO or LibreHardwareMonitor.)' Yellow
    }
}

# 5. Stop any running trofeo instance (two programs cannot share the USB panel).
Get-Process aperture-display, aperture-screen, trofeo_lcd, trofeo_screen -ErrorAction SilentlyContinue | ForEach-Object {
    Say "Stopping process $($_.Id): $($_.Path)"
    Stop-Process -Id $_.Id -Force
}
Start-Sleep -Seconds 1

# 6. Remove previous trofeo autostart tasks.
Get-ScheduledTask -ErrorAction SilentlyContinue | Where-Object {
    ((($_.Actions | ForEach-Object { "$($_.Execute) $($_.Arguments) $($_.WorkingDirectory)" }) -join ' ') -match 'aperture-display|trofeo_lcd')
} | ForEach-Object {
    Say "Removing old scheduled task: $($_.TaskPath)$($_.TaskName)"
    Unregister-ScheduledTask -TaskName $_.TaskName -TaskPath $_.TaskPath -Confirm:$false
}

# 7. Register the new autostart (elevated Scheduled Task at logon, hidden console, auto-restart).
Say "`n=== Installing autostart ===" Cyan
& (Join-Path $dir 'scripts\install-autostart.ps1') -Exe $exe -Arguments '--hide-console --keep-alive --idle-fps 4 --active-fps 4' -Method Task -TaskName 'ApertureDisplay'

# 8. Start now.
Start-ScheduledTask -TaskName 'ApertureDisplay'
Start-Sleep -Seconds 6
$p = Get-Process aperture-display -ErrorAction SilentlyContinue
if ($p) { Say "RUNNING: aperture-display PID $($p.Id -join ',')" Green } else { Say 'WARNING: aperture-display is NOT running (see trofeo-errors.txt)' Yellow }
$err = Join-Path $dir 'trofeo-errors.txt'
if (Test-Path $err) { Say "`n=== trofeo-errors.txt ==="; Get-Content $err | Select-Object -Last 20 | ForEach-Object { Say $_ } }
# 9. Desktop shortcut for the settings app.
$app = Join-Path $dir 'aperture.exe'
if (Test-Path $app) {
    $ws = New-Object -ComObject WScript.Shell
    $lnk = $ws.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Desktop')) 'Aperture.lnk'))
    $lnk.TargetPath = $app; $lnk.WorkingDirectory = $dir; $lnk.IconLocation = Join-Path $dir 'assets/brand/aperture.ico'; $lnk.Save()
    Say 'Desktop shortcut: Aperture' Green
}
Say '=== DONE ===' Green
Stop-Transcript | Out-Null
