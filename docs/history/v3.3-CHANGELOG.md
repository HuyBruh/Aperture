# Changelog


## 3.3 — New compositions

- New layouts: **stack** (three full-width rows), **mosaic** (eight one-number tiles + cover tile), **mixer** (level faders), **chart** (temperature chart + table), **player** (big album cover).

## 3.2 — Tile collection

- New layouts built on the Grid tiles: **tint** (tiles tinted per section, sky-colored weather, cover-colored music), **glass** (frosted tiles over a wallpaper from the theme colors), **gauge** (arc gauge per section with tiles around it).
- Removed studio, line, split, meter, glance, daylight, pulse, ambient, poster and horizon.

## 3.1 — Creative collection

- New layouts: **pulse** (temperature waves behind the numbers), **ambient** (album cover as the background, glass panel), **poster** (giant CPU temperature), **horizon** (sky by time of day, sun on a 24 h track).
- They are listed first in HuyBruh Dash and in `pro_layout = all`.

## 3.0 — HuyBruh Dash

- New name: **HuyBruh Dash**, with an HB logo and a 2-second start-up logo on the display.
- **Clean collection:** studio, grid, line, split, meter, glance, daylight (Inter, strict grid, Focus-size text).
- **HuyBruh Dash app** (`huybruh_dash.exe`): live preview, layout gallery, colors, display, features, sensors & help; English / Vietnamese. It replaces `choose-layout.bat`, `extras.bat` and `gallery.bat`.
- Intel CPU temperature and power through PawnIO (IntelMSR module); the installer installs PawnIO with winget.
- CPU temperature, CPU fan and GPU hotspot fall back to LibreHardwareMonitor (web server) or HWiNFO, read every second; `pro_fan_sensor` picks the fan.
- `trofeo-status.txt`: the display program reports the display and sensor sources (used by the app).
- Installer: checks that the display is connected, closes TRCC, explains the source-code-ZIP mistake, creates a desktop shortcut.
- Focus: clock, weather and music moved to the left.
- Removed the 2.0 novelty layouts (swiss stays).

## Pro Dashboard 2.0 — 2026-10-21

### 21 new landscape layouts (34 in total)
`aurora` frosted glass · `nixie` glowing tubes · `vfd` 7-segment vacuum display · `radar` sweeping scope ·
`scope` oscilloscope · `paper` newspaper · `swiss` typographic grid · `isometric` 3D towers ·
`ticker` trading terminal · `rpg` 8-bit HUD · `analog` hi-fi receiver · `hologram` · `flip` split-flap board ·
`neonsign` · `vinyl` turntable · `circuit` PCB · `pop` comic pop-art · `topo` map · `mercury` thermometers ·
`hud` fighter-jet HUD · `gamer` session dashboard (FPS graph, average / 1 % low / max, peak temperatures, session time).

### New options
- `pro_game_layout` / `pro_game_theme` — switch automatically while a game reports FPS (3 s to enter, 10 s to leave).
- `pro_night = 22:00-07:00`, `pro_night_dim`, `pro_night_theme` — night schedule with dimming.
- `pro_alert_cpu` / `pro_alert_gpu` — pulsing red frame and "HOT" pill above a temperature (3 °C hysteresis).
- `pro_theme = #RRGGBB` or `pro_accent = #RRGGBB` — a complete theme grown from one color.
- `pro_clock = 12h` — 12-hour clock with AM / PM.
- Windows helpers: `extras.bat` (guided setup of the above), `gallery.bat` (renders every layout), `choose-layout.bat` accepts names and lists all layouts.

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
