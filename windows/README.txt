TROFEO PRO DASHBOARD - Windows package
======================================
1. Close the official TRCC software (two programs cannot drive the LCD at once).
2. Double-click  install.bat   (accept the UAC prompt). The dashboard starts now and at every logon.
3. Double-click  choose-layout.bat  to pick one of 13 layouts, a color theme and a style.

Other files
  trofeo.conf          Settings. Edit + save = the display updates within ~1 second.
  trofeo_config.exe    Graphical settings editor.
  restart.bat          Restart the display.      uninstall.bat   Remove autostart and stop it.
  preview.bat          Render the current look to _preview\preview.png (no hardware needed).

Portrait layouts (lock, equalizer, receipt, synthwave, blueprint) are for a vertically mounted panel:
set  pro_rotate = cw  or  ccw  in trofeo.conf.  Picture upside-down in landscape?  flip = true.

Optional readings
  CPU fan RPM : run HWiNFO64 with  Settings -> Sensors -> Shared Memory Support  enabled.
  FPS         : needs Administrator (the autostart task takes care of it).

Source code and docs: https://github.com/HuyBruh/trofeo-lcd-rs      License: GPL-3.0-or-later
