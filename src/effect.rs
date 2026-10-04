//! Animated background effects (plasma, aurora, lava, waves) that cost almost no CPU.
//!
//! How it stays cheap:
//!
//! * The effect is a smooth scalar field, so it is only evaluated on a coarse grid
//!   (one sample every [`CELL`] pixels: ~6 000 samples on a 1920x462 panel).
//! * The grid is stretched to the full panel with plain linear interpolation in
//!   fixed point (integer adds in the inner loop, no trig, no floats per pixel).
//! * Each pixel is one lookup in a 1024-entry colour table (the palette, already
//!   multiplied by the brightness). Changing palette or brightness only rebuilds
//!   that table.
//!
//! The colour table is cyclic, so the field can be left "unwrapped" while it is
//! interpolated (no seams) and wrapped only at the lookup.
//!
//! The picture itself is the expensive part of the program (JPEG encode + USB), so
//! the frame rate of the effect is a setting of its own (`effect_fps`).

use trofeo_lcd::Framebuffer;

pub type Rgb = (u8, u8, u8);

/// Pixels between two samples of the field.
const CELL: usize = 12;
/// Colour table size (a power of two: the lookup wraps with a mask).
const LUT_N: usize = 1024;
/// Fractional bits of the field values (in units of one LUT entry).
const FRAC: u32 = 8;
/// Extra fractional bits kept while stepping along a row.
const STEP_FRAC: u32 = 4;
/// Length in pixels of "1.0" in the formulas below at `scale = 100`.
const UNIT_PX: f32 = 260.0;
/// Seconds a palette takes to cross-fade into the next one when rotating.
const FADE_SECS: f64 = 4.0;

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// The classic demoscene plasma: a sum of sines and a moving radial wave.
    Plasma,
    /// Slow undulating bands, like curtains of light.
    Aurora,
    /// A lava lamp: soft blobs drifting and merging.
    Lava,
    /// Smooth diagonal ripples, like silk.
    Waves,
}

impl Style {
    pub const NAMES: &'static str = "plasma | aurora | lava | waves";
    #[cfg(test)]
    pub const ALL: [Style; 4] = [Style::Plasma, Style::Aurora, Style::Lava, Style::Waves];

    pub fn parse(s: &str) -> Option<Style> {
        match s.trim().to_ascii_lowercase().as_str() {
            "plasma" => Some(Style::Plasma),
            "aurora" | "curtains" | "tende" => Some(Style::Aurora),
            "lava" | "blobs" | "lavalamp" => Some(Style::Lava),
            "waves" | "silk" | "onde" | "seta" => Some(Style::Waves),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Palettes
// ---------------------------------------------------------------------------

/// A looping colour gradient: the last stop blends back into the first.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub stops: Vec<Rgb>,
}

const PASTEL: [Rgb; 6] = [
    (0xFF, 0xB3, 0xBA), (0xFF, 0xDF, 0xBA), (0xFF, 0xFF, 0xBA), (0xBA, 0xFF, 0xC9), (0xBA, 0xE1, 0xFF), (0xDA, 0xC4, 0xFF),
];
const FLUO: [Rgb; 5] = [
    (0xFF, 0x00, 0x80), (0xFF, 0xF0, 0x00), (0x00, 0xFF, 0x78), (0x00, 0xC8, 0xFF), (0x96, 0x00, 0xFF),
];
const NEON: [Rgb; 4] = [(0xFF, 0x00, 0xC8), (0x6E, 0x00, 0xFF), (0x00, 0x8C, 0xFF), (0x00, 0xFF, 0xDC)];
const CANDY: [Rgb; 5] = [
    (0xFF, 0x8A, 0xC8), (0xFF, 0xC8, 0xE8), (0x9A, 0xE6, 0xFF), (0xB4, 0xFF, 0xDC), (0xD8, 0xA8, 0xFF),
];
const SUNSET: [Rgb; 5] = [
    (0x3A, 0x10, 0x70), (0xC8, 0x20, 0x80), (0xFF, 0x60, 0x30), (0xFF, 0xC0, 0x50), (0xB0, 0x20, 0x90),
];
const OCEAN: [Rgb; 4] = [(0x00, 0x20, 0x70), (0x00, 0x78, 0xD0), (0x00, 0xC8, 0xD0), (0x20, 0x50, 0xB0)];
const AURORA: [Rgb; 5] = [
    (0x00, 0x30, 0x40), (0x00, 0xC0, 0x80), (0x60, 0xFF, 0xB0), (0x70, 0x40, 0xD0), (0x10, 0x20, 0x70),
];
const FIRE: [Rgb; 5] = [
    (0x30, 0x00, 0x00), (0xC0, 0x20, 0x00), (0xFF, 0x80, 0x00), (0xFF, 0xD0, 0x40), (0x80, 0x10, 0x00),
];
const FOREST: [Rgb; 4] = [(0x00, 0x30, 0x18), (0x10, 0x80, 0x40), (0x90, 0xD0, 0x50), (0x00, 0x50, 0x40)];
const ICE: [Rgb; 4] = [(0x30, 0x50, 0xB0), (0x70, 0xB0, 0xFF), (0xD0, 0xF4, 0xFF), (0x60, 0x90, 0xE0)];
const MONO: [Rgb; 3] = [(0x20, 0x24, 0x30), (0xB8, 0xC0, 0xD0), (0x50, 0x58, 0x68)];

/// Names of the built-in presets, in the order `all` / `rotate` goes through them.
const PRESETS: [(&str, &[Rgb]); 12] = [
    ("pastel", &PASTEL),
    ("fluo", &FLUO),
    ("neon", &NEON),
    ("candy", &CANDY),
    ("sunset", &SUNSET),
    ("ocean", &OCEAN),
    ("aurora", &AURORA),
    ("fire", &FIRE),
    ("forest", &FOREST),
    ("ice", &ICE),
    ("mono", &MONO),
    ("rainbow", &[]), // generated (full hue circle)
];

/// The full hue circle as a looping palette.
fn rainbow_stops() -> Vec<Rgb> {
    (0..6)
        .map(|i| {
            let h = i as f32 * 60.0;
            hsv(h, 0.85, 1.0)
        })
        .collect()
}

fn hsv(h: f32, s: f32, v: f32) -> Rgb {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (((r + m) * 255.0).round() as u8, ((g + m) * 255.0).round() as u8, ((b + m) * 255.0).round() as u8)
}

impl Palette {
    pub const NAMES: &'static str =
        "pastel | fluo | neon | candy | sunset | ocean | aurora | fire | forest | ice | mono | rainbow";

    pub fn custom(stops: Vec<Rgb>) -> Palette {
        Palette { stops }
    }

    pub fn parse(s: &str) -> Option<Palette> {
        let n = match s.trim().to_ascii_lowercase().as_str() {
            "pastel" | "pastello" => "pastel",
            "fluo" | "fluorescent" | "fluorescente" => "fluo",
            "neon" => "neon",
            "candy" | "caramella" => "candy",
            "sunset" | "tramonto" => "sunset",
            "ocean" | "oceano" => "ocean",
            "aurora" => "aurora",
            "fire" | "fuoco" | "lava" => "fire",
            "forest" | "foresta" => "forest",
            "ice" | "ghiaccio" => "ice",
            "mono" | "grey" | "gray" | "grigio" => "mono",
            "rainbow" | "arcobaleno" => "rainbow",
            _ => return None,
        };
        PRESETS.iter().find(|(name, _)| *name == n).map(|(_, stops)| Palette {
            stops: if stops.is_empty() { rainbow_stops() } else { stops.to_vec() },
        })
    }

    /// Every preset (what `all` / `rotate` expands to).
    pub fn presets() -> Vec<Palette> {
        PRESETS
            .iter()
            .map(|(_, stops)| Palette { stops: if stops.is_empty() { rainbow_stops() } else { stops.to_vec() } })
            .collect()
    }

    /// Colour at `u` (0-1) of the closed loop.
    fn at(&self, u: f32) -> Rgb {
        match self.stops.len() {
            0 => (0, 0, 0),
            1 => self.stops[0],
            n => {
                let x = u.rem_euclid(1.0) * n as f32;
                let i = (x.floor() as usize).min(n - 1);
                lerp_rgb(self.stops[i], self.stops[(i + 1) % n], x - i as f32)
            }
        }
    }
}

/// Darken `c` to `k` (0-1) of its brightness. Light colours (pastels) would turn into
/// mud if they were simply multiplied, so the saturation rises a little as the
/// colour gets darker: a dim pastel pink becomes a dusty rose, not a grey-brown.
fn dim(c: Rgb, k: f32) -> Rgb {
    let (r, g, b) = (c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    if mx <= 0.0 || k <= 0.0 {
        return (0, 0, 0);
    }
    let s = (mx - mn) / mx;
    let v = mx * k;
    let s2 = (s + (1.0 - s) * (1.0 - k) * 0.45).min(1.0);
    // Position of each channel between the darkest (0) and brightest (1) one.
    let h = |x: f32| if mx > mn { (x - mn) / (mx - mn) } else { 0.0 };
    let ch = |x: f32| ((v * (1.0 - s2 * (1.0 - h(x)))) * 255.0).round().clamp(0.0, 255.0) as u8;
    (ch(r), ch(g), ch(b))
}

fn lerp_rgb(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    (l(a.0, b.0), l(a.1, b.1), l(a.2, b.2))
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// What the user configures (`effect_*` keys).
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// `None` = no effect (the normal background applies).
    pub style: Option<Style>,
    /// One palette is fixed; several rotate (cross-faded) every `palette_interval` s.
    pub palettes: Vec<Palette>,
    pub palette_interval: u32,
    /// 0 (frozen) - 100 (fast). 30 is a slow drift.
    pub speed: u32,
    /// Size of the features in % (100 = default, 200 = twice as large and calmer).
    pub scale: u32,
    /// 0-100: how bright the colours are (lower = easier to read the text on top).
    pub brightness: u32,
    /// Frames per second the effect is refreshed at while nothing else needs more.
    pub fps: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            style: None,
            palettes: vec![Palette::parse("neon").expect("preset")],
            palette_interval: 60,
            speed: 30,
            scale: 100,
            brightness: 40,
            fps: 8,
        }
    }
}

impl Settings {
    /// The style to draw: the configured one, or the plasma when the effect is
    /// wanted by another setting (`music_bg = effect`) without a style of its own.
    pub fn style_or_default(&self) -> Style {
        self.style.unwrap_or(Style::Plasma)
    }

    /// Does the picture change over time (so it needs a steady frame rate)?
    pub fn is_moving(&self) -> bool {
        self.speed > 0
    }
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

/// Identifies the colour table currently built (so it is rebuilt only when needed).
#[derive(Clone, Copy, PartialEq)]
struct LutKey {
    a: usize,
    b: usize,
    /// Cross-fade amount, quantised to 1/64.
    f: u8,
    brightness: u32,
    /// Hash of the palette contents (changes when the user edits them).
    sig: u64,
}

pub struct Effect {
    start: std::time::Instant,
    lut: Vec<[u8; 3]>,
    lut_key: Option<LutKey>,
    /// Field samples (fixed point, in LUT entries), `cols * rows`.
    cells: Vec<i32>,
    cols: usize,
    rows: usize,
    /// One interpolated row of samples.
    row: Vec<i32>,
    /// Pixel position of each column / row of samples, in formula units.
    xs: Vec<f32>,
    ys: Vec<f32>,
    grid_key: Option<(usize, usize, u32)>,
}

impl Effect {
    pub fn new() -> Self {
        Effect {
            start: std::time::Instant::now(),
            lut: vec![[0; 3]; LUT_N],
            lut_key: None,
            cells: Vec::new(),
            cols: 0,
            rows: 0,
            row: Vec::new(),
            xs: Vec::new(),
            ys: Vec::new(),
            grid_key: None,
        }
    }

    /// Paint the effect over all of `fb` at the current time.
    pub fn render(&mut self, fb: &mut Framebuffer, s: &Settings, style: Style, brightness: u32) {
        let t = self.start.elapsed().as_secs_f64();
        self.render_at(fb, s, style, brightness, t);
    }

    /// Same, at an explicit time in seconds (tests, previews).
    pub fn render_at(&mut self, fb: &mut Framebuffer, s: &Settings, style: Style, brightness: u32, secs: f64) {
        let (w, h) = (fb.width() as usize, fb.height() as usize);
        if w == 0 || h == 0 || s.palettes.is_empty() {
            return;
        }
        self.update_lut(s, brightness, secs);
        self.update_grid(w, h, s.scale);
        // Time that drives the motion: speed 100 is ~3x the default (30).
        let t = secs * (s.speed as f64 / 30.0);
        self.fill_cells(style, t);
        self.paint(fb, w, h);
    }

    // ----- colour table -----

    fn update_lut(&mut self, s: &Settings, brightness: u32, secs: f64) {
        let n = s.palettes.len();
        let (a, b, f) = if n == 1 {
            (0, 0, 0.0)
        } else {
            let interval = s.palette_interval.max(5) as f64;
            let idx = (secs / interval).floor() as usize;
            let phase = secs % interval;
            let cur = idx % n;
            if phase < FADE_SECS {
                ((cur + n - 1) % n, cur, (phase / FADE_SECS) as f32)
            } else {
                (cur, cur, 0.0)
            }
        };
        let fq = (f * 64.0).round() as u8;
        let sig = palette_sig(&s.palettes[a]) ^ palette_sig(&s.palettes[b]).rotate_left(17);
        let key = LutKey { a, b, f: fq, brightness, sig };
        if self.lut_key == Some(key) {
            return;
        }
        let k = brightness.min(100) as f32 / 100.0;
        let fade = fq as f32 / 64.0;
        for (i, e) in self.lut.iter_mut().enumerate() {
            let u = i as f32 / LUT_N as f32;
            let ca = s.palettes[a].at(u);
            let c = if a == b { ca } else { lerp_rgb(ca, s.palettes[b].at(u), fade) };
            let d = dim(c, k);
            *e = [d.0, d.1, d.2];
        }
        self.lut_key = Some(key);
    }

    // ----- field grid -----

    fn update_grid(&mut self, w: usize, h: usize, scale: u32) {
        let key = (w, h, scale);
        if self.grid_key == Some(key) {
            return;
        }
        self.cols = w / CELL + 2;
        self.rows = h / CELL + 2;
        let unit = UNIT_PX * scale.clamp(25, 400) as f32 / 100.0;
        // Centre the pattern on the panel so resizing doesn't make it jump around.
        let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
        self.xs = (0..self.cols).map(|c| ((c * CELL) as f32 - cx) / unit).collect();
        self.ys = (0..self.rows).map(|r| ((r * CELL) as f32 - cy) / unit).collect();
        self.cells = vec![0; self.cols * self.rows];
        self.row = vec![0; self.cols];
        self.grid_key = Some(key);
    }

    /// Evaluate the field at every sample (the only trigonometry of a frame).
    fn fill_cells(&mut self, style: Style, secs: f64) {
        let ph = |rate: f64| ((secs * rate) % std::f64::consts::TAU) as f32;
        // Palette position -> fixed point LUT units.
        let to_fx = |pos: f32| (pos * (LUT_N as f32) * (1u32 << FRAC) as f32) as i32;
        let (cols, rows) = (self.cols, self.rows);
        match style {
            Style::Plasma => {
                let (p1, p2, p3, p4) = (ph(0.31), ph(0.23), ph(0.17), ph(0.29));
                let (cx, cy) = (1.4 * ph(0.11).sin(), 0.5 * ph(0.07).cos());
                let col_term: Vec<f32> = self.xs.iter().map(|&x| (x * 1.05 + p1).sin()).collect();
                let row_term: Vec<f32> = self.ys.iter().map(|&y| (y * 2.1 - p2).sin()).collect();
                for r in 0..rows {
                    let y = self.ys[r];
                    for c in 0..cols {
                        let x = self.xs[c];
                        let diag = ((x * 0.8 + y * 1.3) + p3).sin();
                        let (dx, dy) = (x - cx, y - cy);
                        let rad = ((dx * dx + dy * dy).sqrt() * 1.6 + p4).sin();
                        let sum = col_term[c] + row_term[r] + diag + rad; // -4 .. 4
                        self.cells[r * cols + c] = to_fx(0.5 + sum * 0.2);
                    }
                }
            }
            Style::Aurora => {
                let (p1, p2, p3) = (ph(0.19), ph(0.27), ph(0.13));
                let col_wave: Vec<f32> = self.xs.iter().map(|&x| (x * 0.9 + p1).sin() * 0.9 + (x * 2.1 - p2).sin() * 0.35).collect();
                for r in 0..rows {
                    let y = self.ys[r];
                    for c in 0..cols {
                        let x = self.xs[c];
                        let warp = ((x * 0.55 + y * 0.9) + p3).sin() * 0.45;
                        let v = y * 1.1 + col_wave[c] + warp;
                        self.cells[r * cols + c] = to_fx(0.5 + v * 0.30);
                    }
                }
            }
            Style::Lava => {
                // Five soft blobs on slow Lissajous paths; the field is their summed influence.
                let blobs: [(f64, f64, f32, f32, f32, f32); 5] = [
                    (0.19, 0.13, 1.6, 0.55, 0.0, 0.70),
                    (0.11, 0.21, 2.1, 0.45, 1.3, 0.60),
                    (0.23, 0.17, 1.1, 0.50, 2.6, 0.55),
                    (0.15, 0.09, 2.6, 0.40, 3.9, 0.65),
                    (0.07, 0.25, 0.7, 0.45, 5.1, 0.50),
                ];
                let pos: Vec<(f32, f32, f32)> = blobs
                    .iter()
                    .map(|&(fx, fy, ax, ay, off, r)| ((ph(fx) + off).sin() * ax, (ph(fy) + off * 0.7).cos() * ay, r))
                    .collect();
                for r in 0..rows {
                    let y = self.ys[r];
                    for c in 0..cols {
                        let x = self.xs[c];
                        let mut f = 0.0f32;
                        for &(bx, by, rad) in &pos {
                            let (dx, dy) = (x - bx, y - by);
                            f += rad * rad / (dx * dx + dy * dy + 0.05);
                        }
                        // Squash to 0..~0.7: the blob centres reach the palette's bright colours and the
                        // cyclic palette never wraps through a seam.
                        let v = 1.0 - 1.0 / (1.0 + f * 0.5);
                        self.cells[r * cols + c] = to_fx(v * 0.7);
                    }
                }
            }
            Style::Waves => {
                let (p1, p2, p3) = (ph(0.21), ph(0.15), ph(0.25));
                for r in 0..rows {
                    let y = self.ys[r];
                    for c in 0..cols {
                        let x = self.xs[c];
                        let bend = (x * 0.5 - p2).sin() * 1.2 + (y * 0.9 + p3).sin() * 0.5;
                        let v = (x * 0.7 + y * 1.4 + bend + p1).sin() * 0.6 + (x * 1.5 - y * 0.6 - p1 * 0.7).sin() * 0.25;
                        self.cells[r * cols + c] = to_fx(0.5 + v * 0.4);
                    }
                }
            }
        }
    }

    // ----- upscale + colour lookup -----

    fn paint(&mut self, fb: &mut Framebuffer, w: usize, h: usize) {
        let mask = (LUT_N - 1) as i32;
        let shift = FRAC + STEP_FRAC;
        let cols = self.cols;
        let pixels = fb.as_bytes_mut();
        for y in 0..h {
            let (r, wy) = (y / CELL, (y % CELL) as i32);
            let top = &self.cells[r * cols..(r + 1) * cols];
            let bot = &self.cells[(r + 1) * cols..(r + 2) * cols];
            // Vertical blend of the two sample rows (cheap: `cols` values).
            for c in 0..cols {
                self.row[c] = top[c] + (bot[c] - top[c]) * wy / CELL as i32;
            }
            let out = &mut pixels[y * w * 3..(y + 1) * w * 3];
            let mut x = 0usize;
            for c in 0..cols - 1 {
                if x >= w {
                    break;
                }
                let run = CELL.min(w - x);
                let mut v = self.row[c] << STEP_FRAC;
                let dv = ((self.row[c + 1] - self.row[c]) << STEP_FRAC) / CELL as i32;
                for px in out[x * 3..(x + run) * 3].chunks_exact_mut(3) {
                    let col = self.lut[((v >> shift) & mask) as usize];
                    px[0] = col[0];
                    px[1] = col[1];
                    px[2] = col[2];
                    v += dv;
                }
                x += run;
            }
        }
    }
}

impl Default for Effect {
    fn default() -> Self {
        Self::new()
    }
}

fn palette_sig(p: &Palette) -> u64 {
    // FNV-1a over the stops: cheap, and only used to notice that a palette changed.
    let mut h = 0xcbf29ce484222325u64;
    for &(r, g, b) in &p.stops {
        for byte in [r, g, b] {
            h ^= byte as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h ^ p.stops.len() as u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use trofeo_lcd::Resolution;

    fn fb(w: u32, h: u32) -> Framebuffer {
        Framebuffer::new(Resolution { width: w, height: h })
    }

    fn settings(style: Style, pal: &str) -> Settings {
        Settings { style: Some(style), palettes: vec![Palette::parse(pal).unwrap()], ..Settings::default() }
    }

    fn avg_luma(f: &Framebuffer) -> f32 {
        let b = f.as_bytes();
        b.iter().map(|&v| v as u64).sum::<u64>() as f32 / b.len() as f32
    }

    #[test]
    fn dimming_keeps_colours_colourful() {
        // Identity at full brightness, black at zero, same hue in between.
        for c in [(255, 179, 186), (0, 255, 120), (30, 60, 200), (128, 128, 128)] {
            assert_eq!(dim(c, 1.0), c);
            assert_eq!(dim(c, 0.0), (0, 0, 0));
        }
        let (r, g, b) = dim((255, 179, 186), 0.5);
        assert!(r > g && r >= b, "still a pink: {r},{g},{b}");
        // A dim pastel is more saturated than the plain product (which would be (127,89,93)).
        let sat = |c: Rgb| (c.0.max(c.1).max(c.2) - c.0.min(c.1).min(c.2)) as f32 / c.0.max(c.1).max(c.2) as f32;
        assert!(sat((r, g, b)) > sat((127, 89, 93)) + 0.05);
        // Brighter input never gets darker as k grows.
        let mut last = 0;
        for k in 0..=20 {
            let (r, g, b) = dim((200, 150, 255), k as f32 / 20.0);
            let l = r as u32 + g as u32 + b as u32;
            assert!(l >= last);
            last = l;
        }
        // Greys stay grey.
        let (r, g, b) = dim((128, 128, 128), 0.4);
        assert!(r == g && g == b);
    }

    #[test]
    fn names_and_aliases_parse() {
        for n in ["plasma", "AURORA", "lava", "waves", "silk", "onde"] {
            assert!(Style::parse(n).is_some(), "{n}");
        }
        assert!(Style::parse("fire").is_none());
        for n in ["pastel", "pastello", "fluo", "neon", "candy", "sunset", "ocean", "aurora", "fire", "forest", "ice", "mono", "rainbow"] {
            assert!(Palette::parse(n).is_some(), "{n}");
        }
        assert!(Palette::parse("beige").is_none());
        assert_eq!(Palette::presets().len(), 12);
        assert!(Palette::presets().iter().all(|p| p.stops.len() >= 3));
    }

    #[test]
    fn every_style_fills_every_pixel_and_moves() {
        for style in Style::ALL {
            let s = settings(style, "fluo");
            let mut e = Effect::new();
            let (mut a, mut b) = (fb(1920, 462), fb(1920, 462));
            a.clear(1, 2, 3);
            e.render_at(&mut a, &s, style, 60, 10.0);
            e.render_at(&mut b, &s, style, 60, 22.0);
            // Nothing is left at the clear colour...
            assert!(a.as_bytes().chunks_exact(3).all(|p| p != [1, 2, 3]), "{style:?} left pixels untouched");
            // ...it uses a good range of colours, and it changes with time.
            let distinct: std::collections::HashSet<_> = a.as_bytes().chunks_exact(3).step_by(97).map(|p| (p[0], p[1], p[2])).collect();
            assert!(distinct.len() > 40, "{style:?} looks flat ({} colours)", distinct.len());
            assert_ne!(a.as_bytes(), b.as_bytes(), "{style:?} does not move");
        }
    }

    #[test]
    fn same_time_same_picture() {
        let s = settings(Style::Plasma, "neon");
        let (mut a, mut b) = (fb(640, 200), fb(640, 200));
        Effect::new().render_at(&mut a, &s, Style::Plasma, 50, 5.0);
        Effect::new().render_at(&mut b, &s, Style::Plasma, 50, 5.0);
        assert_eq!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn speed_zero_is_frozen_and_speed_scales_motion() {
        let mut s = settings(Style::Plasma, "ocean");
        s.speed = 0;
        let (mut a, mut b) = (fb(480, 160), fb(480, 160));
        let mut e = Effect::new();
        e.render_at(&mut a, &s, Style::Plasma, 50, 1.0);
        e.render_at(&mut b, &s, Style::Plasma, 50, 99.0);
        assert_eq!(a.as_bytes(), b.as_bytes(), "speed 0 must not move");
        assert!(!s.is_moving());
        s.speed = 30;
        assert!(s.is_moving());
        // Faster speed = same picture reached sooner.
        let (mut slow, mut fast) = (fb(480, 160), fb(480, 160));
        let mut s_slow = s.clone();
        s_slow.speed = 30;
        let mut s_fast = s.clone();
        s_fast.speed = 60;
        Effect::new().render_at(&mut slow, &s_slow, Style::Plasma, 50, 20.0);
        Effect::new().render_at(&mut fast, &s_fast, Style::Plasma, 50, 10.0);
        assert_eq!(slow.as_bytes(), fast.as_bytes());
    }

    #[test]
    fn brightness_controls_how_bright_it_is() {
        let s = settings(Style::Aurora, "pastel");
        let mut lum = Vec::new();
        for k in [0u32, 20, 40, 80, 100] {
            let mut f = fb(480, 160);
            Effect::new().render_at(&mut f, &s, Style::Aurora, k, 3.0);
            lum.push(avg_luma(&f));
        }
        assert_eq!(lum[0], 0.0, "brightness 0 is black");
        assert!(lum.windows(2).all(|w| w[0] < w[1]), "monotonic: {lum:?}");
        // The default keeps text readable: pastel at 40 stays well below mid-grey.
        assert!(lum[2] < 110.0, "{lum:?}");
    }

    #[test]
    fn pastel_is_softer_than_fluo_at_equal_brightness() {
        // Pastel colours are light (low saturation); fluo is saturated.
        let sat = |name: &str| {
            let s = settings(Style::Plasma, name);
            let mut f = fb(480, 160);
            Effect::new().render_at(&mut f, &s, Style::Plasma, 100, 7.0);
            let b = f.as_bytes();
            let n = (b.len() / 3) as f32;
            b.chunks_exact(3)
                .map(|p| {
                    let mx = p[0].max(p[1]).max(p[2]) as f32;
                    let mn = p[0].min(p[1]).min(p[2]) as f32;
                    (mx - mn) / mx.max(1.0)
                })
                .sum::<f32>()
                / n
        };
        assert!(sat("pastel") < sat("fluo") - 0.2, "{} vs {}", sat("pastel"), sat("fluo"));
    }

    #[test]
    fn scale_changes_feature_size() {
        // Bigger scale = smoother picture = smaller difference between neighbouring columns far apart.
        let var = |scale: u32| {
            let mut s = settings(Style::Plasma, "fluo");
            s.scale = scale;
            let mut f = fb(960, 240);
            Effect::new().render_at(&mut f, &s, Style::Plasma, 80, 4.0);
            let b = f.as_bytes();
            let mut acc = 0u64;
            for y in 0..240usize {
                for x in 0..959usize {
                    let (i, j) = ((y * 960 + x) * 3, (y * 960 + x + 1) * 3);
                    acc += (b[i + 1] as i32 - b[j + 1] as i32).unsigned_abs() as u64;
                }
            }
            acc
        };
        assert!(var(300) < var(100), "{} vs {}", var(300), var(100));
        assert!(var(100) < var(40), "{} vs {}", var(100), var(40));
    }

    #[test]
    fn works_at_any_size() {
        let s = settings(Style::Lava, "fire");
        for (w, h) in [(1, 1), (7, 3), (12, 12), (13, 25), (462, 1920), (1920, 462), (100, 462)] {
            let mut f = fb(w, h);
            Effect::new().render_at(&mut f, &s, Style::Lava, 50, 2.0);
            assert_eq!(f.as_bytes().len(), (w * h * 3) as usize);
        }
        // A buffer of a different size right after: the grid is rebuilt, not reused.
        let mut e = Effect::new();
        let mut a = fb(300, 100);
        e.render_at(&mut a, &s, Style::Lava, 50, 2.0);
        let mut b = fb(130, 400);
        e.render_at(&mut b, &s, Style::Lava, 50, 2.0);
    }

    #[test]
    fn rotating_palettes_cross_fade() {
        let mut s = settings(Style::Plasma, "pastel");
        s.palettes = vec![Palette::custom(vec![(255, 0, 0), (255, 0, 0)]), Palette::custom(vec![(0, 0, 255), (0, 0, 255)])];
        s.palette_interval = 10;
        let at = |secs: f64| {
            let mut f = fb(96, 48);
            Effect::new().render_at(&mut f, &s, Style::Plasma, 100, secs);
            let p = &f.as_bytes()[0..3];
            (p[0], p[2])
        };
        // Second 12: well after the fade into palette 1 (idx 1 -> blue).
        let (r, b) = at(18.0);
        assert!(r < 5 && b > 250, "blue expected: {r},{b}");
        // Second 20 = start of palette 0's turn: fade from blue (previous) into red begins.
        let (r0, b0) = at(20.0);
        assert!(b0 > 250 && r0 < 5, "fade starts from the previous palette: {r0},{b0}");
        let (rm, bm) = at(22.0);
        assert!(rm > 60 && bm > 60, "halfway through the fade both show: {rm},{bm}");
        let (r1, b1) = at(26.0);
        assert!(r1 > 250 && b1 < 5, "red after the fade: {r1},{b1}");
    }

    #[test]
    fn custom_palette_colors_are_used() {
        let mut s = settings(Style::Waves, "pastel");
        s.palettes = vec![Palette::custom(vec![(0, 200, 0), (0, 100, 0)])];
        let mut f = fb(240, 80);
        Effect::new().render_at(&mut f, &s, Style::Waves, 100, 1.0);
        assert!(f.as_bytes().chunks_exact(3).all(|p| p[0] == 0 && p[2] == 0 && p[1] >= 99));
    }

    #[test]
    fn lut_is_rebuilt_only_when_something_changed() {
        let s = settings(Style::Plasma, "neon");
        let mut e = Effect::new();
        let mut f = fb(120, 40);
        e.render_at(&mut f, &s, Style::Plasma, 40, 1.0);
        let key = e.lut_key;
        e.render_at(&mut f, &s, Style::Plasma, 40, 2.0);
        assert!(e.lut_key == key);
        e.render_at(&mut f, &s, Style::Plasma, 41, 2.0);
        assert!(e.lut_key != key);
    }

    #[test]
    fn no_wrap_seam_inside_one_picture() {
        // Neighbouring pixels never jump between the two ends of the palette
        // unless the palette itself does: with a smooth palette the biggest
        // horizontal step stays small.
        for style in Style::ALL {
            let s = settings(style, "ocean");
            let mut f = fb(1920, 462);
            Effect::new().render_at(&mut f, &s, style, 100, 13.0);
            let b = f.as_bytes();
            let mut worst = 0i32;
            for y in (0..462usize).step_by(7) {
                for x in 0..1919usize {
                    let (i, j) = ((y * 1920 + x) * 3, (y * 1920 + x + 1) * 3);
                    for k in 0..3 {
                        worst = worst.max((b[i + k] as i32 - b[j + k] as i32).abs());
                    }
                }
            }
            assert!(worst <= 12, "{style:?}: a seam of {worst} levels between two pixels");
        }
    }
}
