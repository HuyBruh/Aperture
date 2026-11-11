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
| `pro_layout` | `board`, `studio`, `pulse`, `cluster`, `panorama`, `cinema`, `axis`, `ledger`, `duet`, `orbit`, `atelier`, `focus`, `grid`, `stack`, `mosaic`, `mixer`, `chart`, `player`, `tint`, `glass`, `gauge`, `swiss`, `classic`, `cockpit`, `bento`, `terminal`, `chamfer`, `rings`, `dots` |
| `pro_layout = all` | Rotates the New and Aperture layouts plus Focus and Grid |
| `pro_layout = portrait` | Rotates the existing five portrait layouts |
| `pro_theme` | `mineral`, `frost`, `blush`, `sand`, `iris`, `#RRGGBB`; legacy palette names remain compatible |
| `pro_theme = all` | Rotates the five Aperture palettes |
| `pro_theme_interval` | Minutes between items in a rotation; `0` keeps the first |
| `pro_cover_color` | Whether album art changes accent colors |
| `pro_song_popup` | Whether a song change briefly overlays a cover/title card |
| `pro_style` | `cards`, `glass`, `flat`; used by the original Classic/Cockpit/Focus renderers |
| `pro_fan_sensor` | CPU fan: a motherboard header (`NCT6687D / CPU Fan`) or an external sensor label; omit for automatic |
| `fps_ignore` | Extra programs never measured as the game, comma separated; `none` disables the built-in list [restart] |
| `weather_city` | Optional fixed city; requires restart |

Use commas to build your own layout or palette rotation. Changing the displayed layout does not change the sensor source.

The complete upstream options reference is preserved in `reference/OPTIONS_GUIDE.md`. It includes features outside the Aperture dashboard and may describe legacy defaults.
