# Development

## Windows

Install the stable Rust MSVC toolchain and Visual Studio C++ Build Tools. The GUI uses egui/eframe. Sensor and USB integrations are native.

```powershell
python tools/generate-design.py --check
python tools/check-design.py
cargo check --locked --bins
cargo test --locked --bins --lib -- --test-threads=1
cargo build --release --locked --bin aperture-display --bin aperture
python tools/render-collection.py
powershell -NoProfile -File tools/package.ps1
```

Python 3.11+ is needed for design tooling. Pillow is only needed to regenerate brand raster assets and preview contact sheets. The runtime itself does not need Python.

## Linux

Install Rust, a C compiler, `pkg-config`, libudev development headers and PulseAudio development headers. `playerctl` supplies media titles. Build with the same Cargo commands. Install the provided udev rule for non-root USB access. The settings binary is `aperture`.

## Layout ownership

- `src/pro.rs`: configuration, data sources, rotation and top-level composition.
- `src/pro_designs.rs`: formatted view model shared with earlier designs.
- `src/pro_minimal.rs`: shared reading helpers and the existing clean layouts.
- `src/dashboard/collection.rs`: the five Aperture compositions, bounded text and missing-data states.
- `src/dashboard/geometry.rs`: generated widget coordinates. Do not edit by hand.
- `src/bin/studio.rs`: the native settings app.

Changing a widget's position in `design/collection.json` and regenerating geometry changes the real renderer. The gallery uses `aperture-display --preview-png`; it does not approximate the LCD in a separate web implementation.

Use sample renders for visual checks, then test on the physical panel before claiming hardware validation. Keep sensor/USB changes separate from presentation changes whenever possible.
