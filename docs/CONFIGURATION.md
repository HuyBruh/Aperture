# Configuration

Open Aperture for the normal workflow. `trofeo.conf` remains the compatibility file, next to `aperture-display.exe`.

```ini
layout = pro
pro_layout = axis
pro_theme = mineral
pro_clock = 24h
pro_cover_color = false
pro_song_popup = false
flip = false
```

| Setting | Values |
| :-- | :-- |
| `pro_layout` | `axis`, `ledger`, `duet`, `orbit`, `atelier`, `focus`, `grid`; existing legacy names also work |
| `pro_layout = all` | Rotates the seven-layout Aperture collection |
| `pro_layout = portrait` | Rotates the existing five portrait layouts |
| `pro_theme` | `mineral`, `frost`, `blush`, `sand`, `iris`, `#RRGGBB`; legacy palette names remain compatible |
| `pro_theme = all` | Rotates the five Aperture palettes |
| `pro_theme_interval` | Minutes between items in a rotation; `0` keeps the first |
| `pro_cover_color` | Whether album art changes accent colors |
| `pro_song_popup` | Whether a song change briefly overlays a cover/title card |
| `pro_style` | `cards`, `glass`, `flat`; used by the original Classic/Cockpit/Focus renderers |
| `pro_fan_sensor` | External CPU fan label; omit for automatic selection |
| `weather_city` | Optional fixed city; requires restart |

Use commas to build your own layout or palette rotation. Changing the displayed layout does not change the sensor source.

The complete upstream options reference is preserved in `reference/OPTIONS_GUIDE.md`. It includes features outside the Aperture dashboard and may describe legacy defaults.
