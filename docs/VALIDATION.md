# Validation — Aperture 4.0

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
