# Changelog

## Pro Dashboard 1.0 — 2026-10-20

First release of the **pro** dashboard on top of `trofeo-lcd-rs` 1.1.0.

### New layouts
- **Landscape:** `bento`, `terminal`, `chamfer`, `rings`, `dots` (join `classic`, `cockpit`, `focus`).
- **Portrait** (vertically mounted panel, `pro_rotate = cw|ccw`): `lock`, `equalizer`, `receipt`, `synthwave`, `blueprint`.
- `pro_layout = all` cycles every landscape layout; `pro_layout = portrait` cycles the portrait ones.

### Features
- Elapsed / total time next to the progress bar in the now-playing area.
- Volume / mute pop-up and "new song" pop-up (rotated correctly in portrait).
- Accent colors that follow the album art (`pro_cover_color`).
- CPU fan, GPU fan and hotspot, VRAM, FPS + frame-time, SSD temperature and read / write throughput on every layout.
- Weather with a 3-day forecast.
- Bundled **Chakra Petch** font (Light / Regular / Medium) with full Vietnamese diacritics.
- Windows helper package: `install.bat`, `choose-layout.bat`, `restart.bat`, `uninstall.bat`, `preview.bat`.

### Under the hood
- New software-rendered, anti-aliased painter (`tiles.rs`, `smooth_font.rs`); redraws only when the displayed data changes.
- `Layer::rotate_into` for portrait output; marquee disabled in portrait (ellipsis instead).
- Preview CLI: `--preview-png <file> --preview-state idle|music --config <file>`.
