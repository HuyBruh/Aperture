# Changelog

## 4.2.1

- The seven Aperture layouts redrawn on the Grid kit: equal 12 px gaps and equal columns, room between every value and the next label, heroes sized to the panel, hotspot and core count always shown.
- One weather block for every layout: one line when it fits, the next days as a list in tall panels, no overlapping text.
- One music block: the text always starts at the cover's own edge; large covers carry the title on a soft shade inside the cover.
- Pulse and Cluster show the GPU hotspot (and the core count / total memory).
- Chart's table reads label and value on one line; Dots' weather no longer overlaps.

## 4.2.0

- New layouts: Board (split-flap departure board), Studio (light paper and ink), Pulse (live two-minute charts) and Cluster (car instrument cluster).
- CPU and case fan speeds read directly from the motherboard (Nuvoton NCT67xx/NCT668x, ITE IT86xx/IT87xx, Fintek F718xx) through PawnIO; MSI boards show their header names. HWiNFO / LibreHardwareMonitor remain an optional fallback.
- Game FPS rewritten: QPC time stamps of DXGI, D3D9 and graphics-kernel present events, the busiest swap chain of the game, one-second window anchored on the newest frame; non-games are ignored (`fps_ignore` adds more); the AMD driver counter is only a fallback when ETW cannot start.
- Album covers kept at full resolution and resized once per drawn size with Lanczos / Catmull-Rom.
- Settings app redesigned: side navigation, every layout and palette visible with filters, live Sensors page, cleaner Customize page (English and Vietnamese), restart finds the real autostart task, encoded elevated commands.
- Custom colors build a full multi-hue palette again.
- The status file carries live readings for the settings app and the program version.

## 4.0.0 — Aperture

- Rebrand the settings app, icon, startup mark and portable launchers to Aperture.
- Add Axis, Ledger, Duet, Orbit and Atelier with a complete shared reading model.
- Add Mineral, Frost, Blush, Sand and Iris, using one coordinated accent and contrast-checked secondary text.
- Use Inter in Focus as well as the new collection and Grid.
- Put the curated collection first in settings, enlarge previews and group older landscape layouts separately.
- Make `all` rotate the seven-layout collection and the five new palettes. Explicit old layout and palette keys remain compatible.
- Replace the sprawling default configuration with a documented fresh-install file. Keep the device executable and task names for compatibility.
- Separate design geometry, the new renderer, brand sources, development tooling and migration documentation.
- Prepare a release workflow that builds and packages the actual Aperture settings binary and native preview evidence.

Earlier release notes are preserved in [history](docs/history/v3.3-CHANGELOG.md).
