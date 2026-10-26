TROFEO PRO DASHBOARD - Windows package
======================================
1. Close the official TRCC software (two programs cannot drive the LCD at once).
2. Double-click  install.bat  (accept the UAC prompt). It checks the display, installs the PawnIO
   sensor driver if needed, starts the dashboard now and at every logon, then opens HuyBruh Dash.
3. HuyBruh Dash (huybruh_dash.exe): pick a layout, colors, flip 180, brightness, clock,
   game mode, night mode and heat alert, in English or Vietnamese. Changes show instantly.
   Its "Sensors & help" page explains anything that is missing (CPU temperature, fan, hotspot).

Other files
  trofeo.conf          Settings (HuyBruh Dash edits it for you).
  restart.bat          Restart the display.      uninstall.bat   Remove autostart and stop it.
  preview.bat          Render the current look to _preview\preview.png (no hardware needed).

Optional readings
  CPU temperature : PawnIO driver (installed by install.bat) - AMD Ryzen and Intel Core.
  CPU fan speed   : LibreHardwareMonitor (free) with Options -> Remote Web Server -> Run,
                    or HWiNFO with Settings -> Shared Memory Support.
  FPS             : needs Administrator (the autostart task takes care of it).

Source code and docs: https://github.com/HuyBruh/trofeo-lcd-rs      License: GPL-3.0-or-later
