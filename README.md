<p align="center"><img src="docs/img/aperture/brand.png" alt="Aperture â€” a quieter view of your system" width="100%"></p>

<p align="center"><b>A native system display for the Thermalright Trofeo Vision 9.16â€³.</b><br>Large type. Complete readings. A little less noise.</p>

<p align="center"><a href="#the-collection">The collection</a> Â· <a href="#get-started">Get started</a> Â· <a href="docs/CONFIGURATION.md">Configuration</a> Â· <a href="docs/DEVELOPMENT.md">Build & contribute</a></p>

## New in 4.3

- **See the real panel in the app.** "Live display" under the preview shows exactly what the screen shows, once a second.
- **One-click updates.** The app tells you when a new release is out, downloads and verifies it, keeps your `trofeo.conf` and restarts the display by itself, with no administrator prompt.
- **Restart without UAC**, quick looks (six ready-made layout and palette pairs), a smooth crossfade between layouts, pump and case fan widgets, and a "Copy diagnostics" button for bug reports.

![Settings: layouts](docs/img/aperture/app-layouts.png)

## New in 4.2

| Layout | Idea |
| :-- | :-- |
| **Board** | An airport departure board: every number on split-flap tiles, one row per section. |
| **Studio** | Paper and ink: a light spec sheet with black numbers, hairlines and one orange accent. |
| **Pulse** | A live chart of the last two minutes for CPU, GPU and network, the readings under it. |
| **Cluster** | A car instrument cluster: three dials with a real scale, the album cover on the side. |

![Board](docs/img/aperture/board.png)
![Studio](docs/img/aperture/studio.png)
![Pulse](docs/img/aperture/pulse.png)
![Cluster](docs/img/aperture/cluster.png)

- **CPU fan without other software.** The fan speeds are read straight from the motherboard's Super I/O chip (Nuvoton, ITE, Fintek; MSI boards with their header names) through the PawnIO driver already used for the CPU temperature. HWiNFO or LibreHardwareMonitor are only needed on boards that cannot be read.
- **Accurate game FPS.** Rewritten the way PresentMon measures: frames are timed from Windows' own present events (DirectX 9-12, plus Vulkan / OpenGL through the graphics kernel), averaged over the last second of the game's frames. Browsers, launchers, Discord and the desktop are never shown as "the game"; the GPU driver's fullscreen counter is no longer used while real measurement works.
- **Sharp album covers.** The cover keeps its full resolution and is resized once per size with Lanczos / Catmull-Rom, instead of being shrunk to 160 px and stretched back.
- **A new settings app.** Side navigation, every layout (34) and every palette (12) visible with filters, a live Sensors page (temperatures, fans, FPS and where each comes from), a cleaner Customize page, a restart button that finds the real autostart task.

![Settings: sensors](docs/img/aperture/app-sensors.png)

## The collection

Seven compositions built for the panel's 1920 × 462 canvas share one information model, next to Focus, Grid and the Signature set (Stack, Mosaic, Mixer, Chart, Player, Tint, Glass, Gauge, Swiss) and the Classic set (Classic, Cockpit, Bento, Terminal, Chamfer, Rings, Dots). Five portrait layouts are available for a vertically mounted panel.

| Layout | Composition |
| :-- | :-- |
| **Panorama** | Three large instruments and a compact context footer. |
| **Cinema** | Large album artwork and a dense hardware console. |
| **Axis** | A shallow context bar above three equal hardware instruments. |
| **Ledger** | Three precise hardware rows with a separate context column. |
| **Duet** | CPU and GPU across two wide stages; memory has its own rail. |
| **Orbit** | Fine gauge arcs around the hardware numbers; a central clock and music column. |
| **Atelier** | Album art leads, with a complete hardware desk alongside it. |

![Axis](docs/img/aperture/axis.png)
![Orbit](docs/img/aperture/orbit.png)

*Images are rendered by the program using sample data, not measurements from a live PC.*

**Palettes.** Soft: Mineral, Frost, Blush, Sand, Iris, Lagoon (one accent on neutral surfaces). Vivid: NZXT, Graphite, Crimson, Arctic, Ember, Neon (a color per section on black). Or pick your own color: a whole matching palette is built from it.

## What you can see

- **CPU:** temperature, load, power, clock, fan RPM, uptime and network throughput.
- **GPU:** temperature, hotspot, load, power, fan, clock, VRAM and FPS with frame time.
- **Memory & storage:** memory use, free memory, SSD temperature, disk capacity use and read/write throughput.
- **Context:** clock, date, current weather, a three-day forecast, album art, track title, artist and elapsed/total time.

Readings depend on the hardware and sensor source. Unavailable sensors show `--`; they are never replaced with invented values. The display does not replace VRAM with FPS when a game starts.

## Get started

### Windows

1. Download **`aperture-windows.zip`** from [Aperture releases](https://github.com/HuyBruh/Aperture/releases/latest). The source ZIP is for developers and contains no executable.
2. Extract the entire package into a permanent folder. Close Thermalright TRCC so it releases the USB display.
3. Run **`Aperture.bat`** to open settings. Run **`scripts/install.bat`** if you want sensor-driver setup and automatic startup. Windows may ask for elevation.
4. Select a layout and palette. Settings are saved in `trofeo.conf` and picked up by the display program.

The package contains two executables: `aperture.exe` for settings, and `aperture-display.exe` for the USB display. The display service is now named `ApertureDisplay`. Existing `trofeo.conf` settings remain supported; the installer replaces the previous scheduled task.

**Upgrading from HuyBruh Dash:** back up your current folder and keep your existing `trofeo.conf`. The new default configuration is only for fresh installs. See [migration](docs/MIGRATION.md) before replacing a running installation.

### Sensors

| Reading | Source |
| :-- | :-- |
| CPU temperature / power | PawnIO on supported AMD/Intel CPUs; external sensor fallback |
| CPU fan / case fans | The motherboard's Super I/O chip through PawnIO; LibreHardwareMonitor or HWiNFO as a fallback |
| GPU telemetry | GPU interfaces and supported external sensor sources |
| Game FPS | ETW present events (DirectX 9-12, Vulkan, OpenGL); the display service runs elevated |
| Weather | Open-Meteo; a configured city or IP-based location |
| Now playing | Windows media controls; `playerctl` on Linux |

Open **Sensors** in the settings app to see live readings, their source, and to pick the CPU fan (or an AIO pump). Availability varies by machine. Weather needs an internet connection.

### Linux / build from source

See [development](docs/DEVELOPMENT.md) for prerequisites and commands. The new collection shares the existing software renderer and USB driver; this rework does not introduce a web runtime.

## Project structure

```text
src/dashboard/       Aperture collection renderer and generated geometry
design/              Canonical layout geometry and palette specifications
assets/brand/        Vector mark and application icons
windows/             Portable defaults, launchers and installation scripts
tools/               Design generation, checks, rendering and packaging
docs/                Configuration, migration, development and release evidence
```

Legacy renderers and upstream integrations remain in `src/` for compatibility. The new collection is isolated so a layout change does not require changes to sensor collection or the USB protocol.

## Performance & validation

The 3.x performance measurements were for older layouts on one PC at idle. They are [preserved as history](docs/history/v3.3-README.md), not presented as benchmarks of this rework. A fresh measurement is required before claiming a CPU or RAM budget for Aperture.

See [validation](docs/VALIDATION.md) for the exact checks performed and remaining hardware coverage. A successful preview render verifies the renderer, not every physical LCD or sensor configuration.

## Credits & license

By **Huy**, previously HuyBruh Dash. Built on [haven80/trofeo-lcd-rs](https://github.com/haven80/trofeo-lcd-rs), [sukualam/trofeo-lcd](https://github.com/sukualam/trofeo-lcd) and the protocol work in [Lexonight1/thermalright-trcc-linux](https://github.com/Lexonight1/thermalright-trcc-linux). AI-assisted implementation includes Claude and Codex. Full provenance is in [NOTICE.md](NOTICE.md).

Inter and Chakra Petch are distributed under the SIL Open Font License; their notices remain in `assets/`. **[GPL-3.0-or-later](LICENSE)**. Not affiliated with Thermalright.
