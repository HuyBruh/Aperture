# Validation — Aperture 4.0

## 4.3.0 — 11 November 2026

Built on the same Windows PC (Rust 1.99.1, x86_64-pc-windows-gnu, WinLibs GCC 16.2) and
installed there. The status file reported `version=4.3.0`, `lcd=connected` and
`supervised=1`. A restart request from the app's folder restarted the display program in
under two seconds without an administrator prompt, and a live-view request produced
`trofeo-live.png` matching the panel (Orbit layout). Measured with Windows performance
counters over 20 s on a 16-thread Ryzen 7 5700X3D at the default 4 frames per second:
about 11 % of one thread (≈0.7 % of the whole CPU) and 72 MB private memory; turning
FPS capture off made no measurable difference. The update path was exercised against a
local release feed: download, size and SHA-256 check passed; the file replacement step is
Windows-only and is checked by the next real release. 184 display-program tests and the
settings-app tests (version comparison, base64) pass.

## 4.2.0 — 11 November 2026

Built on Windows (Rust stable, x86_64-pc-windows-gnu, WinLibs GCC) and installed on the
author's PC (AMD CPU and GPU, MSI MAG B550M MORTAR WIFI). After the
display service restarted with 4.2.0, its status file reported `lcd=connected`, the board
chip `NCT6687D` with CPU Fan 2226 RPM and three system fans around 1100 RPM read without
HWiNFO or LibreHardwareMonitor running, CPU temperature and GPU hotspot from built-in
sources, FPS capture running and no FPS reported on the desktop (previously the AMD driver
counter was shown there). The settings app ran on the same PC and showed those live values.
184 display-program tests pass (new: FPS rate maths, non-game filter, game tracking, fan
counter maths and CPU-fan choice). The four new layouts were inspected from native renders
of the Windows build. Not verified here: FPS inside a running game on this PC, ITE and
Fintek boards, NVIDIA cards.

Local validation on Windows, 29 October 2026. Source baseline: `4979ece4c33f38aed30221093dcde74ce95fe635` from HuyBruh/trofeo-lcd-rs. The local source archive's Rust files matched that baseline before editing.

## Completed

- Rust 1.99.1, x86_64-pc-windows-gnu; optimized builds of `aperture-display.exe` and `aperture.exe`.
- Cargo check of all binaries.
- 227 existing and new tests pass when run with `--test-threads=1` (46 library, 5 advanced editor, 176 device binary). The renderer's language/options state is global; parallel execution of the inherited tests is not reliable.
- Five layouts contain every required widget, stay inside the native canvas and have no intersecting panel rectangles.
- Five specified palettes meet 4.5:1 text/label contrast on their panel backgrounds.
- 49 native sample renders: seven collection layouts × five palettes with music, plus idle and missing-sensor/weather/media fixtures in Mineral. Outputs are 1920 × 462 and are not blank.
- Visual inspection of the five new compositions; spacing, ring/number clearance and cover sizing were refined using native renders.
- Native settings window launches with the Aperture title/mark, real renderer previews, the curated gallery and readable two-column cards. Its preview uses sample data and is labelled accordingly.
- Local installation was applied after the user accepted UAC. The installed display executable's SHA-256 matches the validated build, status reports `lcd=connected`, and the installed settings window reports **Display connected**. The active personal configuration is Axis + Blush with the previous orientation preserved. CPU temperature and GPU hotspot report built-in sources; CPU fan has no external sensor source and remains unavailable.
- A branded Aperture desktop shortcut was created. The old name's shortcut, launcher, executable and earlier documentation/source archive were retained in the pre-update backup rather than left among daily-use entry points.

## Not claimed

- The new collection has not been visually inspected on the physical LCD by this agent. Native PNG rendering and USB connection status are different checks.
- No new CPU/RAM benchmark is published. Historical 3.x values do not establish the cost of the new layouts.
- This local build uses GNU. The Windows MSVC workflow is supplied for independent build/validation on GitHub; its outcome must be checked after push.
- Other CPUs, GPUs, portrait mounting and Linux have not been retested on hardware during this rework. Existing integrations and portrait designs remain compatible code paths.

The final source ZIP excludes the local vendor cache, toolchains, credentials, personal configuration and build output. The portable Windows package includes the two required executables, clean defaults, launchers, installation scripts and concise help/licensing files.

## Aperture repository rename
The Rust package/crate is now aperture. The release display binary is aperture-display; screen and advanced settings targets are aperture-screen and aperture-config. After renaming, cargo check (all bins), 227 sequential release tests, both release executables, design generation/contrast checks and 49 native preview fixtures passed on the local GNU toolchain. The existing installed LCD was not replaced during the GitHub publication step. Windows MSVC validation is provided by GitHub Actions.
