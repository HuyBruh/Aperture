# Aperture 4.1 beta 3

This prerelease adds the native Customize editor, image/GIF backgrounds, presets,
Panorama and Cinema layouts, Lagoon colors, and a personal-logo option in place of
music on the seven Aperture collection layouts.

Community fixes include device-qualified CPU fan/AIO pump and GPU fan selectors,
external GPU RPM fallback, AMD adapter preference by model name with automatic
VRAM-based selection, and DXGI FPS counting per swap chain using original event
timestamps. Test-only present calls are excluded. Rendering reuses the settings
snapshot across collection readings.

Download aperture-4.1-beta-windows.zip, extract the whole folder, and open
Aperture.bat. Preserve your existing trofeo.conf when upgrading. Run
scripts/install.bat to register automatic startup in the new folder.

234 local tests passed. Seven Lagoon layouts and the Cinema logo fixture rendered
successfully. RTX 4090/5070 Ti fan readings, hybrid-GPU selection and FPS accuracy
still require testing on affected machines. FPS is DXGI Present rate; generated or
displayed-frame counters may differ. The 6.86-inch LCD remains unsupported.

Settings: Layouts -> Customize selected theme. Sensor selection and FPS source are
under Sensors & help. AMD adapter changes require a display-process restart.
