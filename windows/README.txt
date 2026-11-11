APERTURE
A quieter view of your system.

GET STARTED
Open Aperture.bat to choose a layout, palette and display settings.
The settings window uses sample data for its layout previews.

MAINTENANCE
scripts\install.bat    Enable automatic startup and set up sensor drivers.
scripts\restart.bat    Restart the display service.
scripts\preview.bat    Save a sample display image.
scripts\uninstall.bat  Disable automatic startup.
Windows may ask for administrator permission for maintenance actions.

FOLDER GUIDE
assets\   Application icons and font licenses.
docs\     Configuration, migration and validation notes.
scripts\  Setup and maintenance tools.
trofeo.conf is the editable configuration. Keep the entire folder together.
aperture-display.exe and trofeo-status.txt are used by the display service.

SENSORS
Close Thermalright TRCC before starting Aperture.
Unavailable readings show --. Fan speeds are read from the motherboard through
the PawnIO driver (installed by scripts\install.bat); LibreHardwareMonitor or
HWiNFO are only needed on boards that cannot be read. Open Sensors in the
settings window for live readings and diagnostics.

LICENSE
See LICENSE and NOTICE.md. Source: https://github.com/HuyBruh/Aperture