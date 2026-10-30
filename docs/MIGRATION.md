# From HuyBruh Dash to Aperture

1. Keep a backup of the old installation, including `trofeo.conf`.
2. Close the settings app. Stop the existing TrofeoLCD or ApertureDisplay scheduled task and display process before replacing its executable.
3. Extract the new portable package into a permanent folder. Copy your existing `trofeo.conf` into it instead of using fresh defaults.
4. Open `Aperture.bat`. Pick a collection layout and palette when ready. Device orientation, weather city and sensor choices stay in the copied configuration.
5. Run `scripts/install.bat` if you want the task and desktop shortcut pointed at the new folder. The new task is `ApertureDisplay` and the device program is `aperture-display.exe`. The installer removes the previous TrofeoLCD task and stops the old trofeo_lcd process before starting Aperture.

The settings program is now `aperture.exe`; a new `Aperture` shortcut replaces the product-facing name. Old shortcuts are not automatically deleted. Legacy layout keys still parse. `all` now rotates the reviewed collection; use an explicit comma-separated list to retain an old rotation.

Rollback: stop the new display process and restore the old folder/task path. A copied configuration is preferred to editing the only working copy. Do not start two display programs at once.
