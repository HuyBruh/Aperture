//! The `pro` layout: a dark "control center" dashboard in the style of premium
//! hardware monitors — one strip of panels, each with a header, a ring, the
//! detailed values and a short history graph:
//!
//! `TIME + WEATHER | PROCESSOR | GRAPHICS | MEMORY / STORAGE | NOW PLAYING / NETWORK`
//!
//! * Processor: load ring, temperature, power, clock, cores, model name, load history.
//! * Graphics: load ring, temperature, hotspot, power, fan RPM, clock, VRAM used/total,
//!   adapter name, FPS badge while a game reports it, load history.
//! * Memory: used / total GB + ring. Storage: system drive usage ring, drive temperature,
//!   read / write speed.
//! * Now playing: album cover, title, artist, progress (accent color from the cover).
//! * Network: download / upload speed with a small history.
//!
//! Everything is painted on a transparent [`Layer`] (shared with `tiles.rs`) that is
//! re-drawn only when something visible changes.

use super::*;
use crate::gpu_amd::GpuAmdData;
use crate::media::TrackInfo;
use crate::sysextra::DiskInfo;
use crate::tiles::{gradient_at, lerp_rgb, rrect_cov, FontStyle, Layer, Painter, W};
use crate::weather::WeatherSnapshot;
use crate::weather_icon::{self, WeatherIcon};
use chrono::Timelike;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

type Rgb = (u8, u8, u8);
type R = (f32, f32, f32, f32);

const WHITE: Rgb = (255, 255, 255);

// ------------------------------------------------------------------- themes

/// Colors of one theme. Every theme is dark; they differ in tint and accents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    bg: Rgb,
    panel: Rgb,
    text: Rgb,
    muted: Rgb,
    dim: Rgb,
    cpu: [Rgb; 2],
    gpu: [Rgb; 2],
    ram: [Rgb; 2],
    ssd: [Rgb; 2],
    net: [Rgb; 2],
    up: Rgb,
    fps: Rgb,
    /// Seconds line under the clock and the clock panel's accent.
    time: Rgb,
}

pub const THEMES: &[Theme] = &[
    // violet / cyan / pink on near-black (NZXT CAM-like)
    Theme {
        name: "nzxt", bg: (0, 0, 0), panel: (12, 13, 17), text: (242, 243, 246), muted: (168, 172, 184), dim: (118, 122, 136),
        cpu: [(124, 92, 255), (196, 140, 255)], gpu: [(14, 165, 233), (103, 232, 249)], ram: [(236, 72, 153), (251, 164, 206)],
        ssd: [(245, 158, 11), (253, 214, 99)], net: [(16, 185, 129), (110, 231, 183)], up: (96, 165, 250), fps: (163, 230, 53), time: (255, 255, 255),
    },
    // monochrome silver: no colors except warnings (Apple / studio gear look)
    Theme {
        name: "graphite", bg: (0, 0, 0), panel: (15, 15, 16), text: (246, 246, 247), muted: (165, 165, 170), dim: (112, 112, 118),
        cpu: [(150, 150, 158), (255, 255, 255)], gpu: [(150, 150, 158), (255, 255, 255)], ram: [(150, 150, 158), (255, 255, 255)],
        ssd: [(150, 150, 158), (255, 255, 255)], net: [(150, 150, 158), (255, 255, 255)], up: (200, 200, 206), fps: (255, 255, 255), time: (255, 255, 255),
    },
    // red / orange on warm black (gaming brands)
    Theme {
        name: "crimson", bg: (0, 0, 0), panel: (17, 10, 11), text: (247, 240, 240), muted: (188, 165, 167), dim: (138, 114, 116),
        cpu: [(220, 20, 50), (255, 99, 99)], gpu: [(255, 78, 42), (255, 160, 80)], ram: [(190, 24, 93), (251, 113, 133)],
        ssd: [(249, 115, 22), (253, 186, 116)], net: [(239, 68, 68), (252, 165, 165)], up: (253, 186, 116), fps: (250, 204, 21), time: (255, 120, 120),
    },
    // ice blue / teal on deep navy
    Theme {
        name: "arctic", bg: (2, 5, 10), panel: (10, 17, 27), text: (236, 244, 252), muted: (152, 172, 194), dim: (104, 124, 146),
        cpu: [(56, 189, 248), (186, 230, 253)], gpu: [(45, 212, 191), (153, 246, 228)], ram: [(129, 140, 248), (199, 210, 254)],
        ssd: [(96, 165, 250), (191, 219, 254)], net: [(34, 211, 238), (165, 243, 252)], up: (199, 210, 254), fps: (110, 231, 183), time: (186, 230, 253),
    },
    // amber / gold on warm black (instrument cluster)
    Theme {
        name: "ember", bg: (0, 0, 0), panel: (16, 13, 9), text: (250, 245, 236), muted: (190, 172, 150), dim: (136, 120, 100),
        cpu: [(251, 146, 60), (254, 205, 140)], gpu: [(234, 179, 8), (254, 240, 138)], ram: [(248, 113, 113), (254, 178, 160)],
        ssd: [(217, 119, 6), (252, 211, 77)], net: [(163, 230, 53), (217, 249, 157)], up: (254, 205, 140), fps: (250, 204, 21), time: (254, 205, 140),
    },
    // magenta / cyan / yellow on black (cyberpunk)
    Theme {
        name: "neon", bg: (0, 0, 0), panel: (11, 8, 19), text: (242, 240, 255), muted: (176, 164, 206), dim: (122, 110, 156),
        cpu: [(255, 0, 153), (255, 110, 199)], gpu: [(0, 229, 255), (140, 250, 255)], ram: [(190, 60, 255), (226, 150, 255)],
        ssd: [(255, 214, 0), (255, 240, 140)], net: [(0, 255, 140), (140, 255, 200)], up: (0, 229, 255), fps: (255, 230, 0), time: (255, 110, 199),
    },
];

/// How the panels are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Solid rounded panels with a thin border.
    Cards,
    /// Panels with a soft glow of their accent color at the top.
    Glass,
    /// No panels: hairline dividers between the sections.
    Flat,
}

/// Arrangement of the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Panels in a row (the first design).
    Classic,
    /// Car instrument cluster: two gauges and the clock in the middle.
    Cockpit,
    /// A few huge numbers.
    Focus,
}

impl Layout {
    pub const ALL: [Layout; 3] = [Layout::Classic, Layout::Cockpit, Layout::Focus];
    pub fn name(self) -> &'static str {
        match self {
            Layout::Classic => "classic",
            Layout::Cockpit => "cockpit",
            Layout::Focus => "focus",
        }
    }
}

/// `pro_layout`, `pro_theme`, `pro_style`, `pro_theme_interval` from trofeo.conf.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub layouts: Vec<Layout>,
    pub themes: Vec<Theme>,
    pub style: Style,
    /// Minutes between two themes when several are listed (0 = only the first).
    pub interval_min: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { layouts: vec![Layout::Classic], themes: vec![THEMES[0]], style: Style::Cards, interval_min: 30 }
    }
}

pub fn theme_names() -> String {
    THEMES.iter().map(|t| t.name).collect::<Vec<_>>().join(", ")
}

pub fn parse(file: &ConfigFile) -> anyhow::Result<Settings> {
    let mut s = Settings::default();
    if let Some(v) = file.get("pro_theme") {
        let mut list = Vec::new();
        for name in v.split(',').map(|n| n.trim().to_ascii_lowercase()).filter(|n| !n.is_empty()) {
            if name == "all" {
                list.extend_from_slice(THEMES);
                continue;
            }
            let t = THEMES
                .iter()
                .find(|t| t.name == name)
                .ok_or_else(|| anyhow::anyhow!("pro_theme: '{name}' is not a theme ({}, all)", theme_names()))?;
            list.push(*t);
        }
        if !list.is_empty() {
            s.themes = list;
        }
    }
    if let Some(v) = file.get("pro_layout") {
        let mut list = Vec::new();
        for name in v.split(',').map(|n| n.trim().to_ascii_lowercase()).filter(|n| !n.is_empty()) {
            if name == "all" {
                list.extend_from_slice(&Layout::ALL);
                continue;
            }
            let l = Layout::ALL
                .iter()
                .find(|l| l.name() == name)
                .ok_or_else(|| anyhow::anyhow!("pro_layout: '{name}' is not valid (classic, cockpit, focus, all)"))?;
            list.push(*l);
        }
        if !list.is_empty() {
            s.layouts = list;
        }
    }
    if let Some(v) = file.get("pro_style") {
        s.style = match v.trim().to_ascii_lowercase().as_str() {
            "cards" | "card" | "solid" => Style::Cards,
            "glass" | "glow" => Style::Glass,
            "flat" | "minimal" | "lines" => Style::Flat,
            _ => anyhow::bail!("pro_style: '{v}' not valid (cards | glass | flat)"),
        };
    }
    if let Some(v) = file.get_u32("pro_theme_interval").map_err(|e| anyhow::anyhow!(e))? {
        if v > 1440 {
            anyhow::bail!("pro_theme_interval: 0-1440 minutes");
        }
        s.interval_min = v;
    }
    Ok(s)
}

thread_local! {
    static CURRENT: std::cell::Cell<(Theme, Style)> = std::cell::Cell::new((THEMES[0], Style::Cards));
}

fn th() -> Theme {
    CURRENT.with(|c| c.get().0)
}

fn style() -> Style {
    CURRENT.with(|c| c.get().1)
}

/// The theme and layout in use now: the only ones, or those of this time slot when
/// several rotate (every `interval_min` minutes).
fn pick(s: &Settings) -> (Theme, Layout) {
    let slot = if s.interval_min == 0 { 0 } else { (Local::now().timestamp().max(0) as u64) / 60 / s.interval_min as u64 };
    let t = if s.themes.is_empty() { THEMES[0] } else { s.themes[(slot % s.themes.len() as u64) as usize] };
    let l = if s.layouts.is_empty() { Layout::Classic } else { s.layouts[(slot % s.layouts.len() as u64) as usize] };
    (t, l)
}

/// Color right under the content (to "cut" shapes, e.g. the moon).
fn under() -> Rgb {
    if style() == Style::Flat { th().bg } else { th().panel }
}

const WARN: Rgb = (251, 146, 60);
const HOT: Rgb = (248, 82, 82);

/// Seconds of history in the graphs (one sample per second).
const HIST: usize = 120;

/// Everything the screen shows (filled by the main loop or the preview).
pub struct Data<'a> {
    pub cpu_name: &'a str,
    pub cores: (usize, usize),
    pub cpu_pct: f32,
    pub cpu_temp: Option<f32>,
    pub cpu_power: Option<f32>,
    pub cpu_mhz: Option<u32>,
    pub gpu_pct: Option<f32>,
    pub gpu: &'a GpuAmdData,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub disk: Option<&'a DiskInfo>,
    /// CPU fan (HWiNFO shared memory), when available.
    pub cpu_fan_rpm: Option<u32>,
    /// Read / write, MB/s.
    pub disk_rate: (f64, f64),
    /// Down / up, KB/s.
    pub net_kb: (f64, f64),
    pub weather: Option<&'a WeatherSnapshot>,
    pub fahrenheit: bool,
    pub uptime_s: u64,
    pub track: Option<&'a Arc<TrackInfo>>,
}

struct Cover {
    key: usize,
    size: u32,
    rgb: Vec<u8>,
    accent: Rgb,
}

pub struct ProScreen {
    layer: Layer,
    sig: String,
    cpu: VecDeque<f32>,
    gpu: VecDeque<f32>,
    net: VecDeque<f32>,
    last_push: Option<Instant>,
    hist_ver: u64,
    cover: Option<Cover>,
    /// Titles too long for their box, scrolled on every frame, and since when.
    marquees: Vec<MarqueeSpec>,
    marquee_since: Instant,
    marquee_k: f32,
}

impl ProScreen {
    pub fn new() -> Self {
        ProScreen {
            layer: Layer::new(1, 1),
            sig: String::new(),
            cpu: VecDeque::new(),
            gpu: VecDeque::new(),
            net: VecDeque::new(),
            last_push: None,
            hist_ver: 0,
            cover: None,
            marquees: Vec::new(),
            marquee_since: Instant::now(),
            marquee_k: 1.0,
        }
    }

    /// Fills the graphs with plausible values (preview only).
    pub fn seed_demo_history(&mut self) {
        for i in 0..HIST {
            let t = i as f32;
            self.cpu.push_back((22.0 + 14.0 * (t / 9.0).sin() + 9.0 * (t / 3.7).cos()).clamp(2.0, 100.0));
            self.gpu.push_back((40.0 + 25.0 * (t / 13.0).sin() + 8.0 * (t / 2.9).sin()).clamp(2.0, 100.0));
            self.net.push_back((700.0 + 600.0 * (t / 6.0).sin().max(0.0) + 150.0 * (t / 2.3).cos()).max(0.0));
        }
        self.hist_ver += 1;
        self.last_push = Some(Instant::now());
    }

    fn push_history(&mut self, d: &Data) {
        if self.last_push.is_some_and(|t| t.elapsed().as_millis() < 1000) {
            return;
        }
        self.last_push = Some(Instant::now());
        for (q, v) in [(&mut self.cpu, d.cpu_pct), (&mut self.gpu, d.gpu_pct.unwrap_or(0.0)), (&mut self.net, d.net_kb.0 as f32)] {
            q.push_back(v);
            while q.len() > HIST {
                q.pop_front();
            }
        }
        self.hist_ver += 1;
    }

    fn update_cover(&mut self, d: &Data) {
        let Some(cover) = d.track.and_then(|t| t.cover.as_ref()) else {
            self.cover = None;
            return;
        };
        let key = Arc::as_ptr(cover) as usize;
        if self.cover.as_ref().is_some_and(|c| c.key == key) {
            return;
        }
        let size = 160u32;
        let rgb = scale_square(&cover.rgb, cover.width, cover.height, size);
        let accent = cover_accent(&rgb);
        self.cover = Some(Cover { key, size, rgb, accent });
    }

    /// Paints the screen on `fb`; `area` = the part of `fb` the interface may use.
    pub fn draw(&mut self, fb: &mut Framebuffer, area: (u32, u32, u32, u32), d: &Data) {
        let settings = opts().pro;
        let (theme, layout) = pick(&settings);
        CURRENT.with(|c| c.set((theme, settings.style)));
        fb.clear(theme.bg.0, theme.bg.1, theme.bg.2);
        self.push_history(d);
        self.update_cover(d);
        let now = Local::now();
        let progress = d.track.and_then(|t| t.timeline).map(|tl| (tl.position_now_ms() / 1000, tl.duration_ms / 1000));
        let sig = format!(
            "{}{:?}{:?}|{area:?}|{:?}|{}|{}|{:.0}|{:?}|{:?}|{:?}|{:?}|{:?}|{}|{}|{:?}|{:.1}{:.1}|{:.0}{:.0}|{:?}|{}|{:?}|{:?}|{}",
            theme.name,
            settings.style,
            layout,
            d.cpu_fan_rpm,
            now.format("%H:%M:%S %F"),
            d.cpu_name,
            d.cpu_pct,
            d.cpu_temp.map(|v| v.round()),
            d.cpu_power.map(|v| v.round()),
            d.cpu_mhz.map(|m| m / 10),
            d.gpu_pct.map(|v| v.round()),
            d.gpu,
            d.ram_used_mb / 100,
            d.ram_total_mb,
            d.disk,
            d.disk_rate.0,
            d.disk_rate.1,
            d.net_kb.0,
            d.net_kb.1,
            d.weather.map(|w| (w.temp_c.round() as i32, w.code, w.humidity, w.city.clone())),
            d.uptime_s / 60,
            d.track.map(|t| (t.title.clone(), t.artist.clone())),
            progress,
            self.hist_ver,
        );
        if sig != self.sig {
            let (w, h) = (area.2, area.3);
            if self.layer.w != w as usize || self.layer.h != h as usize {
                self.layer = Layer::new(w, h);
            } else {
                self.layer.clear();
            }
            let cover = self.cover.as_ref();
            let hist = (&self.cpu, &self.gpu, &self.net);
            let mut p = Painter { layer: &mut self.layer, font: FontStyle::Smooth };
            MARQ.with(|m| m.borrow_mut().clear());
            let (wf, hf) = (w as f32, h as f32);
            let k = (hf / 462.0).min(wf / 1920.0 * 1.15).clamp(0.5, 2.0);
            let sc = Scale::new(k);
            match layout {
                Layout::Classic => paint(&mut p, wf, hf, d, &now, hist, cover),
                Layout::Cockpit => paint_cockpit(&mut p, wf, hf, &sc, d, &now, cover),
                Layout::Focus => paint_focus(&mut p, wf, hf, &sc, d, &now, hist, cover),
            }
            let found = MARQ.with(|m| std::mem::take(&mut *m.borrow_mut()));
            let same_text = found.iter().map(|m| &m.text).eq(self.marquees.iter().map(|m| &m.text));
            if !same_text {
                self.marquee_since = Instant::now();
            }
            self.marquees = found;
            self.marquee_k = k;
            self.sig = sig;
        }
        self.layer.composite_onto(fb, area.0, area.1);
        let t = self.marquee_since.elapsed().as_secs_f32();
        for m in &self.marquees {
            paint_marquee(fb, area.0, area.1, m, self.marquee_k, t);
        }
    }

    /// True while a title is scrolling (the main loop then raises the frame rate).
    pub fn animating(&self) -> bool {
        let t = self.marquee_since.elapsed().as_secs_f32();
        self.marquees.iter().any(|m| {
            let tw = crate::smooth_font::text_width(&m.text, crate::smooth_font::px_for_cap(m.cap), sfw(m.weight));
            marquee_offset(tw, 70.0 * self.marquee_k, MARQ_SPEED * self.marquee_k, t).1
        })
    }
}

// ------------------------------------------------------------------- layout
//
// One type scale for the whole screen (cap heights at 462 px panel height):
//   XL 60  the clock and the ring percentages
//   L  34  every value (temperatures, watts, clocks, RPM, VRAM, FPS, RAM, net, ...)
//   M  17  names, sub-lines, date, weather text
//   S  13.5 labels / headers (spaced capitals) and units
// Spacing is on a 2 px grid; panels share the same header, subtitle and graph rows.

#[derive(Clone, Copy)]
struct Scale {
    k: f32,
    xl: f32,
    l: f32,
    m: f32,
    s: f32,
}

impl Scale {
    fn new(k: f32) -> Self {
        Scale { k, xl: 60.0 * k, l: 34.0 * k, m: 17.0 * k, s: 13.5 * k }
    }
}

fn paint(
    p: &mut Painter,
    w: f32,
    h: f32,
    d: &Data,
    now: &chrono::DateTime<Local>,
    hist: (&VecDeque<f32>, &VecDeque<f32>, &VecDeque<f32>),
    cover: Option<&Cover>,
) {
    let k = (h / 462.0).min(w / 1920.0 * 1.15).clamp(0.5, 2.0);
    let sc = Scale::new(k);
    let m = (10.0 * k).round();
    let gap = (12.0 * k).round();
    let ph = h - 2.0 * m;
    let weights = [268.0f32, 408.0, 584.0, 288.0, 304.0];
    let avail = w - 2.0 * m - gap * (weights.len() as f32 - 1.0);
    let sum: f32 = weights.iter().sum();
    let mut xs = Vec::new();
    let mut x = m;
    for wt in weights {
        let cw = (avail * wt / sum).floor();
        xs.push((x, cw));
        x += cw + gap;
    }
    let top = m;
    let half = ((ph - gap) / 2.0).floor();

    draw_time(p, (xs[0].0, top, xs[0].1, ph), &sc, d, now);
    draw_cpu(p, (xs[1].0, top, xs[1].1, ph), &sc, d, hist.0);
    draw_gpu(p, (xs[2].0, top, xs[2].1, ph), &sc, d, hist.1);
    draw_ram(p, (xs[3].0, top, xs[3].1, half), &sc, d);
    draw_disk(p, (xs[3].0, top + half + gap, xs[3].1, ph - half - gap), &sc, d);
    let media_h = ((ph - gap) * 0.60).floor();
    draw_media(p, (xs[4].0, top, xs[4].1, media_h), &sc, d, cover);
    draw_net(p, (xs[4].0, top + media_h + gap, xs[4].1, ph - media_h - gap), &sc, d, hist.2);
}

// --------------------------------------------------------------- primitives

fn panel(p: &mut Painter, r: R, k: f32, accent: Rgb) {
    let r = (r.0.round(), r.1.round(), r.2.round(), r.3.round());
    let rad = 18.0 * k;
    match style() {
        Style::Cards => {
            p.layer.fill_rrect_solid(r, rad, th().panel, 255);
            p.layer.stroke_rrect(r, rad, 1.0, WHITE, 20);
        }
        Style::Glass => {
            p.layer.fill_rrect_solid(r, rad, th().panel, 255);
            // a soft wash of the accent color fading out downwards
            let (x, y, w, h) = r;
            let glow_h = (h * 0.55).max(1.0);
            for py in (y as i32)..((y + glow_h) as i32) {
                let t = (py as f32 - y) / glow_h;
                let a = (34.0 * (1.0 - t) * (1.0 - t)) as u8;
                if a == 0 {
                    continue;
                }
                for px in (x as i32)..((x + w) as i32) {
                    let cov = rrect_cov(px as f32 + 0.5, py as f32 + 0.5, x, y, w, h, rad);
                    if cov > 0.0 {
                        p.layer.over(px, py, accent, (a as f32 * cov) as u8);
                    }
                }
            }
            p.layer.stroke_rrect(r, rad, 1.0, WHITE, 26);
            // bright top edge
            capsule(p, (x + rad, y + 1.0), (x + w - rad, y + 1.0), 0.6, accent, 120);
        }
        Style::Flat => {
            // hairline dividers between the sections (none on the outer edges)
            let (x, y, w, h) = r;
            if x > 24.0 * k {
                capsule(p, (x - 6.0 * k, y + 14.0 * k), (x - 6.0 * k, y + h - 14.0 * k), 0.5, WHITE, 40);
            }
            if y > 24.0 * k {
                capsule(p, (x + 14.0 * k, y - 6.0 * k), (x + w - 14.0 * k, y - 6.0 * k), 0.5, WHITE, 40);
            }
        }
    }
}

fn pad(k: f32) -> f32 {
    (18.0 * k).round()
}

fn circle(p: &mut Painter, cx: f32, cy: f32, r: f32, c: Rgb, a: u8) {
    p.layer.fill_rrect((cx - r, cy - r, 2.0 * r, 2.0 * r), r, &move |_| c, a);
}

/// A line with round ends, `r` = half the thickness.
fn capsule(p: &mut Painter, a: (f32, f32), b: (f32, f32), r: f32, c: Rgb, alpha: u8) {
    let (x0, x1) = ((a.0.min(b.0) - r - 1.0).floor() as i32, (a.0.max(b.0) + r + 1.0).ceil() as i32);
    let (y0, y1) = ((a.1.min(b.1) - r - 1.0).floor() as i32, (a.1.max(b.1) + r + 1.0).ceil() as i32);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = (dx * dx + dy * dy).max(1e-6);
    for py in y0..y1 {
        for px in x0..x1 {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let t = (((fx - a.0) * dx + (fy - a.1) * dy) / len2).clamp(0.0, 1.0);
            let (qx, qy) = (a.0 + dx * t, a.1 + dy * t);
            let dist = (fx - qx).hypot(fy - qy);
            let cov = (r + 0.5 - dist).clamp(0.0, 1.0);
            if cov > 0.0 {
                p.layer.over(px, py, c, (alpha as f32 * cov).round() as u8);
            }
        }
    }
}

/// Text with extra space between the letters (small capitals labels).
fn spaced(p: &mut Painter, x: f32, base: f32, text: &str, cap: f32, wt: W, c: Rgb, sp: f32) -> f32 {
    let mut pen = x;
    let mut buf = [0u8; 4];
    for ch in text.chars() {
        pen += p.text(pen, base, ch.encode_utf8(&mut buf), cap, wt, c, 255) + sp;
    }
    (pen - x - sp).max(0.0)
}

fn spaced_width(p: &Painter, text: &str, cap: f32, wt: W, sp: f32) -> f32 {
    let n = text.chars().count() as f32;
    p.width(text, cap, wt) + sp * (n - 1.0).max(0.0)
}

/// Panel header: accent dot + spaced title on the left, `right` (index) on the right.
/// Returns the baseline of the header.
fn header(p: &mut Painter, r: R, k: f32, accent: Rgb, title: &str, right: &str) -> f32 {
    let pd = pad(k);
    let cap = 13.5 * k;
    let base = r.1 + pd + cap;
    circle(p, r.0 + pd + 4.5 * k, base - cap / 2.0, 4.5 * k, accent, 255);
    spaced(p, r.0 + pd + 17.0 * k, base, title, cap, W::Medium, th().muted, 2.0 * k);
    if !right.is_empty() {
        let iw = spaced_width(p, right, cap, W::Medium, 1.2 * k);
        spaced(p, r.0 + r.2 - pd - iw, base, right, cap, W::Medium, th().dim, 1.2 * k);
    }
    base
}

/// Value + small unit, left-aligned at `x`; returns the width.
fn value_unit(p: &mut Painter, x: f32, base: f32, value: &str, unit: &str, cap: f32, wt: W, vc: Rgb) -> f32 {
    let vw = p.text(x, base, value, cap, wt, vc, 255);
    if unit.is_empty() {
        return vw;
    }
    let g = (cap * 0.12).max(2.0);
    vw + g + p.text(x + vw + g, base, unit, (cap * 0.40).max(7.0), W::Regular, th().muted, 255)
}

fn value_unit_width(p: &Painter, value: &str, unit: &str, cap: f32, wt: W) -> f32 {
    let mut t = p.width(value, cap, wt);
    if !unit.is_empty() {
        t += (cap * 0.12).max(2.0) + p.width(unit, (cap * 0.40).max(7.0), W::Regular);
    }
    t
}

/// Largest cap (<= `cap`) at which value + unit fit in `max_w`.
fn fit_vu(p: &Painter, value: &str, unit: &str, cap: f32, max_w: f32, wt: W) -> f32 {
    let mut c = cap;
    for _ in 0..30 {
        if value_unit_width(p, value, unit, c, wt) <= max_w || c < 8.0 {
            break;
        }
        c *= 0.93;
    }
    c
}

/// Cuts `text` with "..." so it is at most `max_w` wide.
fn ellipsize(p: &Painter, text: &str, cap: f32, wt: W, max_w: f32) -> String {
    if p.width(text, cap, wt) <= max_w {
        return text.to_string();
    }
    let mut s: String = text.to_string();
    while s.chars().count() > 1 && p.width(&format!("{}...", s.trim_end()), cap, wt) > max_w {
        s.pop();
    }
    format!("{}...", s.trim_end())
}

/// A ring with a gradient arc, the value in the middle and a small label under it.
fn ring(p: &mut Painter, cx: f32, cy: f32, d: f32, frac: Option<f32>, colors: [Rgb; 2], value: &str, unit: &str, label: &str, k: f32) {
    let thick = (d * 0.072).max(4.0);
    let r = d / 2.0;
    p.layer.arc(cx, cy, r, thick, 1.0, &|_| WHITE, 22);
    if let Some(f) = frac {
        let f = f.clamp(0.0, 1.0);
        if f > 0.004 {
            let span = f.max(0.02);
            p.layer.arc(cx, cy, r, thick, f, &move |t| gradient_at(&colors, t / span), 255);
        }
    }
    let inner = d - 2.0 * thick - 18.0 * k;
    let cap = fit_vu(p, value, unit, d * 0.31, inner, W::Medium);
    let tw = value_unit_width(p, value, unit, cap, W::Medium);
    let lcap = (d * 0.07).max(12.0 * k);
    let block = cap + if label.is_empty() { 0.0 } else { lcap + 9.0 * k };
    let base = cy - block / 2.0 + cap;
    value_unit(p, cx - tw / 2.0, base, value, unit, cap, W::Medium, th().text);
    if !label.is_empty() {
        let lw = spaced_width(p, label, lcap, W::Medium, 1.5 * k);
        spaced(p, cx - lw / 2.0, base + 9.0 * k + lcap, label, lcap, W::Medium, th().dim, 1.5 * k);
    }
}

/// Area graph of `hist` (newest on the right), scaled to `max`.
fn sparkline(p: &mut Painter, r: R, hist: &VecDeque<f32>, max: f32, colors: [Rgb; 2], k: f32) {
    let (x, y, w, h) = r;
    // baseline + a faint mid line
    capsule(p, (x, y + h), (x + w, y + h), 0.5, WHITE, 26);
    let mut dy = y + h / 2.0;
    while dy > y {
        let mut dx = x;
        while dx < x + w {
            capsule(p, (dx, dy), ((dx + 3.0 * k).min(x + w), dy), 0.5, WHITE, 14);
            dx += 9.0 * k;
        }
        dy -= h / 2.0;
    }
    if hist.len() < 2 || max <= 0.0 {
        return;
    }
    let step = w / (HIST as f32 - 1.0);
    let x_start = x + w - (hist.len() as f32 - 1.0) * step;
    let val = |px: f32| -> f32 {
        let f = ((px - x_start) / step).clamp(0.0, hist.len() as f32 - 1.0);
        let i = (f.floor() as usize).min(hist.len() - 2);
        let t = f - i as f32;
        hist[i] * (1.0 - t) + hist[i + 1] * t
    };
    let bottom = y + h;
    // filled area: vertical gradient, fading out downwards
    let col_x0 = x_start.max(x).floor() as i32;
    let col_x1 = (x + w).ceil() as i32;
    for px in col_x0..col_x1 {
        let fx = px as f32 + 0.5;
        let v = (val(fx) / max).clamp(0.0, 1.0);
        let ty = bottom - v * h;
        let c = gradient_at(&colors, (fx - x) / w);
        let mut py = ty.floor() as i32;
        while (py as f32) < bottom {
            let fy = py as f32 + 0.5;
            let cov = (fy - ty + 0.5).clamp(0.0, 1.0);
            let fade = 1.0 - ((fy - ty) / (bottom - ty + 1.0)).clamp(0.0, 1.0);
            let a = (70.0 * fade * fade * cov).round() as u8;
            if a > 0 {
                p.layer.over(px, py, c, a);
            }
            py += 1;
        }
    }
    // the line
    let n = hist.len();
    let pts: Vec<(f32, f32)> = (0..n)
        .map(|i| (x_start + i as f32 * step, bottom - (hist[i] / max).clamp(0.0, 1.0) * h))
        .collect();
    for wnd in pts.windows(2) {
        let c = gradient_at(&colors, ((wnd[1].0 - x) / w).clamp(0.0, 1.0));
        capsule(p, wnd[0], wnd[1], 1.5 * k, c, 255);
    }
    // the newest point
    if let Some(&last) = pts.last() {
        circle(p, last.0, last.1, 3.2 * k, colors[1], 255);
    }
}

fn temp_color(t: f32, warn: f32, hot: f32) -> Rgb {
    if t >= hot {
        HOT
    } else if t >= warn {
        WARN
    } else {
        th().text
    }
}

fn rate(kb: f64) -> (String, &'static str) {
    if kb >= 1024.0 * 100.0 {
        (format!("{:.0}", kb / 1024.0), "MB/s")
    } else if kb >= 1024.0 {
        (format!("{:.1}", kb / 1024.0), "MB/s")
    } else {
        (format!("{kb:.0}"), "KB/s")
    }
}

fn gb(mb: u64) -> String {
    let g = mb as f64 / 1024.0;
    if g >= 100.0 { format!("{g:.0}") } else { format!("{g:.1}") }
}

fn mmss(s: u64) -> String {
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

// ------------------------------------------------------------------- panels

/// Label (S, spaced capitals) with a value (L) + unit under it. Height: [`stat_h`].
fn stat(p: &mut Painter, sc: &Scale, x: f32, top: f32, w: f32, label: &str, value: &str, unit: &str, vc: Rgb) {
    let k = sc.k;
    spaced(p, x, top + sc.s, label, sc.s, W::Medium, th().dim, 1.6 * k);
    let cap = fit_vu(p, value, unit, sc.l, w - 4.0 * k, W::Medium);
    value_unit(p, x, top + sc.s + 7.0 * k + sc.l, value, unit, cap, W::Medium, vc);
}

fn stat_h(sc: &Scale) -> f32 {
    sc.s + 7.0 * sc.k + sc.l
}

fn draw_time(p: &mut Painter, r: R, sc: &Scale, d: &Data, now: &chrono::DateTime<Local>) {
    let k = sc.k;
    panel(p, r, k, th().time);
    let (x, y, w, h) = r;
    let pd = pad(k);
    let inner = w - 2.0 * pd;
    // weekday + date
    let wd = i18n::weekday(now).to_uppercase();
    let wb = y + pd + sc.s;
    spaced(p, x + pd, wb, &wd, sc.s, W::Medium, th().muted, 2.2 * k);
    let date = crate::tiles::nice_case(&i18n::day_month_year(now));
    let dcap = p.fit(&date, sc.m, inner, W::Regular);
    let date_base = wb + 14.0 * k + sc.m;
    p.text(x + pd, date_base, &date, dcap, W::Regular, th().text, 255);
    // the clock
    let t = now.format("%H:%M").to_string();
    let tcap = p.fit(&t, sc.xl * 1.25, inner, W::Light);
    let tw = p.width(&t, tcap, W::Light);
    let tbase = date_base + 28.0 * k + tcap;
    let tx = x + (w - tw) / 2.0;
    p.text(tx, tbase, &t, tcap, W::Light, th().text, 255);
    // seconds: a thin line under the time
    let sy = tbase + 18.0 * k;
    let frac = now.second() as f32 / 59.0;
    p.layer.fill_rrect((x + pd, sy, inner, 4.0 * k), 2.0 * k, &|_| WHITE, 24);
    if frac > 0.0 {
        let fw = (inner * frac).max(4.0 * k);
        p.layer.fill_rrect((x + pd, sy, fw, 4.0 * k), 2.0 * k, &move |t| gradient_at(&[lerp_rgb(th().time, th().bg, 0.45), th().time], t), 255);
    }
    // weather
    let sep = (sy + 24.0 * k).round();
    capsule(p, (x + pd, sep), (x + w - pd, sep), 0.5, WHITE, 30);
    let lb = sep + 18.0 * k + sc.s;
    let Some(wt) = d.weather else {
        spaced(p, x + pd, lb, "WEATHER", sc.s, W::Medium, th().dim, 1.6 * k);
        p.text(x + pd, lb + 20.0 * k + sc.l, "--", sc.l, W::Medium, th().dim, 255);
        return;
    };
    let city = wt.city.clone().unwrap_or_else(|| "WEATHER".into()).to_uppercase();
    let city = ellipsize(p, &city, sc.s, W::Medium, inner - 30.0 * k);
    spaced(p, x + pd, lb, &city, sc.s, W::Medium, th().dim, 1.6 * k);
    let isz = 52.0 * k;
    let wy = lb + 14.0 * k;
    let night = !(6..19).contains(&now.hour());
    weather_icon(p, x + pd - 3.0 * k, wy, isz, wt.icon, night);
    let temp = format!("{:.0}\u{b0}", wt.temp_in(d.fahrenheit).round() + 0.0);
    let ttx = x + pd + isz + 12.0 * k;
    p.text(ttx, wy + isz / 2.0 + sc.l / 2.0, &temp, sc.l, W::Medium, th().text, 255);
    let cond = crate::tiles::nice_case(weather_icon::condition_name(wt.icon));
    let cond = match wt.humidity {
        Some(hm) => format!("{cond} \u{b7} {hm}%"),
        None => cond,
    };
    let cond = ellipsize(p, &cond, sc.m, W::Regular, inner);
    p.text(x + pd, wy + isz + 12.0 * k + sc.m, &cond, sc.m, W::Regular, th().muted, 255);
    // the next three days, at the bottom
    let days: Vec<_> = wt.forecast.iter().skip(1).take(3).collect();
    if !days.is_empty() {
        let cw = inner / days.len() as f32;
        let tb = y + h - pd;
        let db = tb - sc.m - 12.0 * k;
        for (i, f) in days.iter().enumerate() {
            let cx = x + pd + i as f32 * cw;
            let dn = i18n::weekday_short(f.weekday).to_uppercase();
            let dw = spaced(p, cx, db, &dn, sc.s, W::Medium, th().dim, 1.2 * k);
            let is = 22.0 * k;
            weather_icon(p, cx + dw + 4.0 * k, db - sc.s / 2.0 - is / 2.0, is, f.icon, false);
            let hi = format!("{:.0}\u{b0}", f.temp_max_in(d.fahrenheit).round() + 0.0);
            let lo = format!("{:.0}\u{b0}", f.temp_min_in(d.fahrenheit).round() + 0.0);
            let hw = p.text(cx, tb, &hi, sc.m, W::Medium, th().text, 255);
            p.text(cx + hw + 3.0 * k, tb, &lo, sc.s, W::Regular, th().dim, 255);
        }
    }
}

/// Geometry shared by the processor and graphics panels: header, name, ring on the
/// left, a 4-row grid of stats on the right, a thin graph along the bottom.
struct BigRows {
    stats_x: f32,
    row_tops: [f32; 4],
}

fn big_panel(p: &mut Painter, r: R, sc: &Scale, colors: [Rgb; 2], title: &str, index: &str, name: &str, frac: Option<f32>, value: &str, unit: &str, hist: &VecDeque<f32>) -> BigRows {
    let k = sc.k;
    panel(p, r, k, colors[0]);
    let (x, y, w, h) = r;
    let pd = pad(k);
    let hb = header(p, r, k, colors[0], title, index);
    let nb = hb + 14.0 * k + sc.m;
    let name = ellipsize(p, name, sc.m, W::Regular, w - 2.0 * pd);
    p.text(x + pd, nb, &name, sc.m, W::Regular, th().text, 255);
    // graph along the bottom
    let gh = 46.0 * k;
    let gb = y + h - pd;
    sparkline(p, (x + pd, gb - gh, w - 2.0 * pd, gh), hist, 100.0, colors, k);
    // ring + rows between the name and the graph
    let top = nb + 16.0 * k;
    let bottom = gb - gh - 14.0 * k;
    let area = bottom - top;
    let rd = 196.0 * k;
    ring(p, x + pd + rd / 2.0, top + area / 2.0, rd.min(area), frac, colors, value, unit, "LOAD", k);
    let row = area / 4.0;
    let off = ((row - stat_h(sc)) / 2.0).max(0.0);
    let mut row_tops = [0.0; 4];
    for (i, t) in row_tops.iter_mut().enumerate() {
        *t = top + i as f32 * row + off;
    }
    BigRows { stats_x: x + pd + rd + 22.0 * k, row_tops }
}

/// Uptime as ("H:MM", "h") or ("Dd H", "h").
fn uptime_str(s: u64) -> (String, String) {
    let (dd, hh, mm) = (s / 86_400, (s % 86_400) / 3600, (s % 3600) / 60);
    if dd > 0 { (format!("{dd}d {hh}"), "h".into()) } else { (format!("{hh}:{mm:02}"), "h".into()) }
}

fn draw_cpu(p: &mut Painter, r: R, sc: &Scale, d: &Data, hist: &VecDeque<f32>) {
    let pct = format!("{:.0}", d.cpu_pct);
    let b = big_panel(p, r, sc, th().cpu, "PROCESSOR", "01", d.cpu_name, Some(d.cpu_pct / 100.0), &pct, "%", hist);
    let cw = r.0 + r.2 - pad(sc.k) - b.stats_x;
    let (tv, tc) = match d.cpu_temp {
        Some(t) => (format!("{t:.0}"), temp_color(t, 80.0, 90.0)),
        None => ("--".into(), th().dim),
    };
    stat(p, sc, b.stats_x, b.row_tops[0], cw, "TEMP", &tv, "\u{b0}C", tc);
    let pw = d.cpu_power.map_or("--".into(), |v| format!("{v:.0}"));
    stat(p, sc, b.stats_x, b.row_tops[1], cw, "POWER", &pw, "W", th().text);
    let clk = d.cpu_mhz.map_or("--".into(), |m| format!("{:.2}", m as f32 / 1000.0));
    stat(p, sc, b.stats_x, b.row_tops[2], cw, "CLOCK", &clk, "GHz", th().text);
    match d.cpu_fan_rpm {
        Some(rpm) => stat(p, sc, b.stats_x, b.row_tops[3], cw, "FAN", &rpm.to_string(), "RPM", th().text),
        None => {
            let (uv, uu) = uptime_str(d.uptime_s);
            stat(p, sc, b.stats_x, b.row_tops[3], cw, "UPTIME", &uv, &uu, th().text)
        }
    }
}

fn draw_gpu(p: &mut Painter, r: R, sc: &Scale, d: &Data, hist: &VecDeque<f32>) {
    let g = d.gpu;
    let name = g.name.clone().unwrap_or_else(|| "GPU".into());
    let pct = d.gpu_pct.map_or("--".into(), |v| format!("{v:.0}"));
    let unit = if d.gpu_pct.is_some() { "%" } else { "" };
    let b = big_panel(p, r, sc, th().gpu, "GRAPHICS", "02", &name, d.gpu_pct.map(|v| v / 100.0), &pct, unit, hist);
    let cw = ((r.0 + r.2 - pad(sc.k) - b.stats_x) / 2.0).floor();
    let (x0, x1) = (b.stats_x, b.stats_x + cw);
    let t = |v: Option<i32>, warn: f32, hot: f32| match v {
        Some(t) => (t.to_string(), temp_color(t as f32, warn, hot)),
        None => ("--".to_string(), th().dim),
    };
    let (tv, tc) = t(g.temp_edge_c, 80.0, 90.0);
    stat(p, sc, x0, b.row_tops[0], cw, "TEMP", &tv, "\u{b0}C", tc);
    let (hv, hc) = t(g.temp_hotspot_c, 95.0, 105.0);
    stat(p, sc, x1, b.row_tops[0], cw, "HOTSPOT", &hv, "\u{b0}C", hc);
    let pw = g.power_w.map_or("--".into(), |v| v.to_string());
    stat(p, sc, x0, b.row_tops[1], cw, "POWER", &pw, "W", th().text);
    let fan = g.fan_rpm.map_or("--".into(), |v| v.max(0).to_string());
    stat(p, sc, x1, b.row_tops[1], cw, "FAN", &fan, "RPM", th().text);
    let clk = g.clock_mhz.filter(|c| *c > 0).map_or("--".into(), |v| v.to_string());
    stat(p, sc, x0, b.row_tops[2], cw, "CLOCK", &clk, "MHz", th().text);
    let (vv, vu) = match (g.vram_used_mb, g.vram_total_mb) {
        (Some(u), Some(t)) if t > 0 => (format!("{:.1}", u as f32 / 1024.0), format!("/{:.0} GB", t as f32 / 1024.0)),
        (Some(u), _) => (format!("{:.1}", u as f32 / 1024.0), "GB".to_string()),
        _ => ("--".into(), String::new()),
    };
    stat(p, sc, x1, b.row_tops[2], cw, "VRAM", &vv, &vu, th().text);
    match g.fps.filter(|f| *f > 0) {
        Some(f) => {
            stat(p, sc, x0, b.row_tops[3], cw, "FPS", &f.to_string(), "", th().fps);
            stat(p, sc, x1, b.row_tops[3], cw, "FRAME TIME", &format!("{:.1}", 1000.0 / f as f32), "ms", th().fps);
        }
        None => {
            stat(p, sc, x0, b.row_tops[3], cw, "FPS", "--", "", th().dim);
            stat(p, sc, x1, b.row_tops[3], cw, "FRAME TIME", "--", "", th().dim);
        }
    }
}

/// Small card: header, a ring on the right, a value (L) and two lines (M) on the left.
fn card(p: &mut Painter, r: R, sc: &Scale, colors: [Rgb; 2], title: &str, index: &str, frac: Option<f32>, ring_txt: &str, value: (&str, &str, Rgb), line1: &str, line2: &str) {
    let k = sc.k;
    panel(p, r, k, colors[0]);
    let (x, y, w, h) = r;
    let pd = pad(k);
    let hb = header(p, r, k, colors[0], title, index);
    let body_top = hb + 12.0 * k;
    let body_h = y + h - pd - body_top;
    let rd = body_h.min(w * 0.36).max(30.0);
    let cx = x + w - pd - rd / 2.0;
    let cy = body_top + body_h / 2.0;
    ring(p, cx, cy, rd, frac, colors, ring_txt, if frac.is_some() { "%" } else { "" }, "", k);
    let lw = cx - rd / 2.0 - 12.0 * k - (x + pd);
    let vcap = fit_vu(p, value.0, value.1, sc.l, lw, W::Medium);
    let l2cap = if line2.is_empty() { 0.0 } else { p.fit(line2, sc.m, lw, W::Regular) };
    let block = sc.l + 14.0 * k + sc.m + if line2.is_empty() { 0.0 } else { 10.0 * k + sc.m };
    let vb = cy - block / 2.0 + sc.l;
    value_unit(p, x + pd, vb, value.0, value.1, vcap, W::Medium, value.2);
    let s1 = ellipsize(p, line1, sc.m, W::Regular, lw);
    p.text(x + pd, vb + 14.0 * k + sc.m, &s1, sc.m, W::Regular, th().muted, 255);
    if !line2.is_empty() {
        let s2 = ellipsize(p, line2, l2cap, W::Regular, lw);
        p.text(x + pd, vb + 14.0 * k + 2.0 * sc.m + 10.0 * k, &s2, l2cap, W::Regular, th().dim, 255);
    }
}

fn draw_ram(p: &mut Painter, r: R, sc: &Scale, d: &Data) {
    let frac = if d.ram_total_mb > 0 { d.ram_used_mb as f32 / d.ram_total_mb as f32 } else { 0.0 };
    let pct = format!("{:.0}", frac * 100.0);
    let used = gb(d.ram_used_mb);
    let total = format!("of {} GB", gb(d.ram_total_mb));
    let free = format!("{} GB free", gb(d.ram_total_mb.saturating_sub(d.ram_used_mb)));
    card(p, r, sc, th().ram, "MEMORY", "03", Some(frac), &pct, (used.as_str(), "GB", th().text), &total, &free);
}

fn draw_disk(p: &mut Painter, r: R, sc: &Scale, d: &Data) {
    let (rd, wr) = d.disk_rate;
    let io = format!("R {:.1}  W {:.1} MB/s", rd, wr);
    match d.disk {
        Some(di) => {
            let pct = format!("{:.0}", di.used_frac() * 100.0);
            let (tv, tc) = match di.temp_c {
                Some(t) => (t.to_string(), temp_color(t as f32, 60.0, 70.0)),
                None => ("--".to_string(), th().dim),
            };
            let space = format!("{:.0}/{:.0} GB", di.used_gb, di.total_gb);
            card(p, r, sc, th().ssd, "STORAGE", "04", Some(di.used_frac()), &pct, (tv.as_str(), "\u{b0}C", tc), &space, &io);
        }
        None => card(p, r, sc, th().ssd, "STORAGE", "04", None, "--", ("--", "", th().dim), &io, ""),
    }
}

fn draw_media(p: &mut Painter, r: R, sc: &Scale, d: &Data, cover: Option<&Cover>) {
    let k = sc.k;
    let accent = cover.map_or(th().muted, |c| c.accent);
    panel(p, r, k, accent);
    let (x, y, w, h) = r;
    let pd = pad(k);
    let hb = header(p, r, k, accent, "NOW PLAYING", "05");
    let Some(t) = d.track else {
        let cx = x + w / 2.0;
        let cy = hb + (y + h - hb) / 2.0 - 10.0 * k;
        music_note(p, cx, cy - 8.0 * k, 34.0 * k, th().dim);
        let s = "Nothing playing";
        let sw = p.width(s, sc.m, W::Regular);
        p.text(cx - sw / 2.0, cy + 40.0 * k, s, sc.m, W::Regular, th().dim, 255);
        return;
    };
    let body_top = hb + 16.0 * k;
    let bar_zone = 44.0 * k;
    let cs = (y + h - pd - bar_zone - body_top).min(w * 0.36).max(24.0).floor();
    let (cx0, cy0) = (x + pd, body_top);
    match cover {
        Some(c) => blit_cover(p, c, cx0, cy0, cs, 12.0 * k),
        None => {
            p.layer.fill_rrect((cx0, cy0, cs, cs), 12.0 * k, &|t| lerp_rgb((40, 42, 52), (24, 25, 32), t), 255);
            music_note(p, cx0 + cs / 2.0, cy0 + cs / 2.0, cs * 0.32, th().muted);
        }
    }
    // title (one line, scrolls when too long) + artist + album
    let tx = cx0 + cs + 14.0 * k;
    let tw = x + w - pd - tx;
    let title = clean_title(&t.title);
    let tcap = sc.m * 1.12;
    let base = cy0 + tcap + 4.0 * k;
    title_line(p, tx, base, tw, &title, tcap, W::Medium, th().text);
    let artist = if t.artist.trim().is_empty() { "Unknown artist".to_string() } else { t.artist.clone() };
    let artist = ellipsize(p, &artist, sc.m, W::Regular, tw);
    p.text(tx, base + 14.0 * k + sc.m, &artist, sc.m, W::Regular, th().muted, 255);
    if !t.album.trim().is_empty() && t.album.trim() != t.title.trim() {
        let album = ellipsize(p, &t.album, sc.s, W::Regular, tw);
        p.text(tx, base + 14.0 * k + sc.m + 12.0 * k + sc.s, &album, sc.s, W::Regular, th().dim, 255);
    }
    // progress
    let bh = 5.0 * k;
    let by = y + h - pd - bh;
    let bw = w - 2.0 * pd;
    if let Some(tl) = t.timeline.filter(|tl| tl.duration_ms > 0) {
        let pos = tl.position_now_ms();
        let f = (pos as f32 / tl.duration_ms as f32).clamp(0.0, 1.0);
        p.layer.fill_rrect((x + pd, by, bw, bh), bh / 2.0, &|_| WHITE, 28);
        if f > 0.0 {
            let fw = (bw * f).max(bh);
            let (a0, a1) = (accent, lerp_rgb(accent, (255, 255, 255), 0.45));
            p.layer.fill_rrect((x + pd, by, fw, bh), bh / 2.0, &move |t| lerp_rgb(a0, a1, t), 255);
            circle(p, x + pd + fw, by + bh / 2.0, 6.5 * k, WHITE, 255);
        }
        let l = mmss(pos / 1000);
        let rr = mmss(tl.duration_ms / 1000);
        p.text(x + pd, by - 12.0 * k, &l, sc.s, W::Medium, th().muted, 255);
        let rw = p.width(&rr, sc.s, W::Medium);
        p.text(x + w - pd - rw, by - 12.0 * k, &rr, sc.s, W::Medium, th().dim, 255);
    } else {
        p.layer.fill_rrect((x + pd, by, bw, bh), bh / 2.0, &|_| WHITE, 20);
    }
}

fn draw_net(p: &mut Painter, r: R, sc: &Scale, d: &Data, hist: &VecDeque<f32>) {
    let k = sc.k;
    panel(p, r, k, th().net[0]);
    let (x, y, w, h) = r;
    let pd = pad(k);
    let hb = header(p, r, k, th().net[0], "NETWORK", "06");
    let top = hb + 10.0 * k;
    let bottom = y + h - pd;
    let row = (bottom - top) / 2.0;
    let vw = (w - 2.0 * pd) * 0.60;
    for (i, (kb, c, down)) in [(d.net_kb.0, th().net[1], true), (d.net_kb.1, th().up, false)].into_iter().enumerate() {
        let base = top + i as f32 * row + (row + sc.l) / 2.0;
        arrow(p, x + pd + 9.0 * k, base - sc.l / 2.0, 9.0 * k, down, c);
        let (v, u) = rate(kb);
        let vc = fit_vu(p, &v, u, sc.l, vw - 28.0 * k, W::Medium);
        value_unit(p, x + pd + 28.0 * k, base, &v, u, vc, W::Medium, th().text);
    }
    let peak = hist.iter().cloned().fold(64.0f32, f32::max) * 1.15;
    let gx = x + pd + vw + 8.0 * k;
    let gw = x + w - pd - gx;
    if gw > 30.0 * k {
        sparkline(p, (gx, top + 14.0 * k, gw, bottom - top - 14.0 * k), hist, peak, th().net, k);
    }
}

// -------------------------------------------------------------------- icons

fn arrow(p: &mut Painter, cx: f32, cy: f32, s: f32, down: bool, c: Rgb) {
    let t = 1.3 * s / 7.0 * 1.0;
    let sign = if down { 1.0 } else { -1.0 };
    let tip = (cx, cy + sign * s);
    capsule(p, (cx, cy - sign * s), tip, t, c, 255);
    capsule(p, (cx - s * 0.62, cy + sign * s * 0.38), tip, t, c, 255);
    capsule(p, (cx + s * 0.62, cy + sign * s * 0.38), tip, t, c, 255);
}

fn music_note(p: &mut Painter, cx: f32, cy: f32, s: f32, c: Rgb) {
    let r = s * 0.22;
    circle(p, cx - s * 0.28, cy + s * 0.32, r, c, 255);
    circle(p, cx + s * 0.32, cy + s * 0.2, r, c, 255);
    let t = s * 0.055;
    capsule(p, (cx - s * 0.28 + r - t, cy + s * 0.32), (cx - s * 0.28 + r - t, cy - s * 0.42), t, c, 255);
    capsule(p, (cx + s * 0.32 + r - t, cy + s * 0.2), (cx + s * 0.32 + r - t, cy - s * 0.54), t, c, 255);
    capsule(p, (cx - s * 0.28 + r - t, cy - s * 0.42), (cx + s * 0.32 + r - t, cy - s * 0.54), t * 1.6, c, 255);
}

fn cloud(p: &mut Painter, x: f32, y: f32, s: f32, c: Rgb) {
    // x, y = top-left of an s*s box; the cloud sits in its lower part
    circle(p, x + s * 0.36, y + s * 0.50, s * 0.20, c, 255);
    circle(p, x + s * 0.58, y + s * 0.44, s * 0.24, c, 255);
    p.layer.fill_rrect((x + s * 0.12, y + s * 0.50, s * 0.78, s * 0.24), s * 0.12, &move |_| c, 255);
}

fn sun(p: &mut Painter, cx: f32, cy: f32, r: f32, c: Rgb) {
    circle(p, cx, cy, r, c, 255);
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::FRAC_PI_4;
        let (s, co) = a.sin_cos();
        capsule(p, (cx + co * r * 1.45, cy + s * r * 1.45), (cx + co * r * 1.85, cy + s * r * 1.85), r * 0.13, c, 255);
    }
}

fn moon(p: &mut Painter, cx: f32, cy: f32, r: f32, c: Rgb) {
    circle(p, cx, cy, r, c, 255);
    circle(p, cx + r * 0.45, cy - r * 0.35, r * 0.85, under(), 255);
}

fn weather_icon(p: &mut Painter, x: f32, y: f32, s: f32, icon: WeatherIcon, night: bool) {
    let sun_c = (253, 200, 72);
    let moon_c = (226, 232, 240);
    let cloud_c = (214, 220, 230);
    let dark_cloud = (150, 158, 172);
    let rain_c = (96, 165, 250);
    match icon {
        WeatherIcon::Clear => {
            if night {
                moon(p, x + s * 0.5, y + s * 0.5, s * 0.30, moon_c)
            } else {
                sun(p, x + s * 0.5, y + s * 0.5, s * 0.22, sun_c)
            }
        }
        WeatherIcon::PartlyCloudy => {
            if night {
                moon(p, x + s * 0.66, y + s * 0.30, s * 0.19, moon_c);
            } else {
                sun(p, x + s * 0.66, y + s * 0.30, s * 0.15, sun_c);
            }
            cloud(p, x - s * 0.08, y + s * 0.20, s * 0.82, cloud_c);
        }
        WeatherIcon::Cloudy => {
            cloud(p, x - s * 0.06, y - s * 0.12, s * 0.8, dark_cloud);
            cloud(p, x + s * 0.08, y + s * 0.06, s * 0.92, cloud_c);
        }
        WeatherIcon::Fog => {
            for (i, wf) in [0.7f32, 0.85, 0.6].iter().enumerate() {
                let yy = y + s * (0.32 + i as f32 * 0.18);
                capsule(p, (x + s * 0.12, yy), (x + s * (0.12 + wf * 0.76), yy), s * 0.045, cloud_c, 255);
            }
        }
        WeatherIcon::Drizzle | WeatherIcon::Rain => {
            cloud(p, x, y - s * 0.12, s, cloud_c);
            let n = if icon == WeatherIcon::Rain { 3 } else { 2 };
            for i in 0..n {
                let xx = x + s * (0.32 + i as f32 * 0.18);
                capsule(p, (xx, y + s * 0.70), (xx - s * 0.06, y + s * 0.90), s * 0.035, rain_c, 255);
            }
        }
        WeatherIcon::Snow => {
            cloud(p, x, y - s * 0.12, s, cloud_c);
            for i in 0..3 {
                let xx = x + s * (0.32 + i as f32 * 0.18);
                circle(p, xx, y + s * (0.76 + (i % 2) as f32 * 0.1), s * 0.045, WHITE, 255);
            }
        }
        WeatherIcon::Thunder => {
            cloud(p, x, y - s * 0.12, s, dark_cloud);
            let c = (250, 204, 21);
            let t = s * 0.04;
            capsule(p, (x + s * 0.54, y + s * 0.62), (x + s * 0.44, y + s * 0.80), t, c, 255);
            capsule(p, (x + s * 0.44, y + s * 0.80), (x + s * 0.56, y + s * 0.80), t, c, 255);
            capsule(p, (x + s * 0.56, y + s * 0.80), (x + s * 0.46, y + s * 0.98), t, c, 255);
        }
    }
}

// -------------------------------------------------------------------- cover

/// Box-filter resize of a square-ish RGB image to `n` x `n`.
fn scale_square(rgb: &[u8], w: u32, h: u32, n: u32) -> Vec<u8> {
    let mut out = vec![0u8; (n * n * 3) as usize];
    if w == 0 || h == 0 || rgb.len() < (w * h * 3) as usize {
        return out;
    }
    for oy in 0..n {
        let y0 = oy * h / n;
        let y1 = ((oy + 1) * h / n).max(y0 + 1).min(h);
        for ox in 0..n {
            let x0 = ox * w / n;
            let x1 = ((ox + 1) * w / n).max(x0 + 1).min(w);
            let (mut r, mut g, mut b, mut c) = (0u32, 0u32, 0u32, 0u32);
            for yy in y0..y1 {
                for xx in x0..x1 {
                    let i = ((yy * w + xx) * 3) as usize;
                    r += rgb[i] as u32;
                    g += rgb[i + 1] as u32;
                    b += rgb[i + 2] as u32;
                    c += 1;
                }
            }
            let o = ((oy * n + ox) * 3) as usize;
            out[o] = (r / c.max(1)) as u8;
            out[o + 1] = (g / c.max(1)) as u8;
            out[o + 2] = (b / c.max(1)) as u8;
        }
    }
    out
}

/// A vivid color taken from the cover (weighted by saturation), brightened enough
/// to read on the dark panels.
fn cover_accent(rgb: &[u8]) -> Rgb {
    let (mut r, mut g, mut b, mut wsum) = (0f32, 0f32, 0f32, 0f32);
    for px in rgb.chunks_exact(3).step_by(3) {
        let (pr, pg, pb) = (px[0] as f32, px[1] as f32, px[2] as f32);
        let mx = pr.max(pg).max(pb);
        let mn = pr.min(pg).min(pb);
        let sat = if mx > 0.0 { (mx - mn) / mx } else { 0.0 };
        let wgt = sat * sat * (mx / 255.0) + 0.002;
        r += pr * wgt;
        g += pg * wgt;
        b += pb * wgt;
        wsum += wgt;
    }
    if wsum <= 0.0 {
        return (200, 204, 214);
    }
    let (mut r, mut g, mut b) = (r / wsum, g / wsum, b / wsum);
    let mx = r.max(g).max(b).max(1.0);
    if mx < 200.0 {
        let f = 200.0 / mx;
        r = (r * f).min(255.0);
        g = (g * f).min(255.0);
        b = (b * f).min(255.0);
    }
    (r as u8, g as u8, b as u8)
}

fn blit_cover(p: &mut Painter, c: &Cover, x: f32, y: f32, size: f32, radius: f32) {
    let n = size.round() as i32;
    if n <= 0 {
        return;
    }
    let (xi, yi) = (x.round() as i32, y.round() as i32);
    let src = c.size as f32;
    for py in 0..n {
        for px in 0..n {
            let cov = rrect_cov(px as f32 + 0.5, py as f32 + 0.5, 0.0, 0.0, n as f32, n as f32, radius);
            if cov <= 0.0 {
                continue;
            }
            // bilinear sample
            let fx = ((px as f32 + 0.5) * src / n as f32 - 0.5).clamp(0.0, src - 1.0);
            let fy = ((py as f32 + 0.5) * src / n as f32 - 0.5).clamp(0.0, src - 1.0);
            let (ix, iy) = (fx.floor() as u32, fy.floor() as u32);
            let (ix1, iy1) = ((ix + 1).min(c.size - 1), (iy + 1).min(c.size - 1));
            let (tx, ty) = (fx - ix as f32, fy - iy as f32);
            let at = |xx: u32, yy: u32| -> (f32, f32, f32) {
                let i = ((yy * c.size + xx) * 3) as usize;
                (c.rgb[i] as f32, c.rgb[i + 1] as f32, c.rgb[i + 2] as f32)
            };
            let (a, b, cc, dd) = (at(ix, iy), at(ix1, iy), at(ix, iy1), at(ix1, iy1));
            let mix = |a: f32, b: f32, c: f32, d: f32| (a * (1.0 - tx) + b * tx) * (1.0 - ty) + (c * (1.0 - tx) + d * tx) * ty;
            let col = (mix(a.0, b.0, cc.0, dd.0) as u8, mix(a.1, b.1, cc.1, dd.1) as u8, mix(a.2, b.2, cc.2, dd.2) as u8);
            p.layer.over(xi + px, yi + py, col, (255.0 * cov).round() as u8);
        }
    }
    p.layer.stroke_rrect((xi as f32, yi as f32, n as f32, n as f32), radius, 1.0, WHITE, 26);
}

/// Splits `text` into at most two lines that fit `max_w` (the second one cut with "...").
fn wrap2(p: &Painter, text: &str, cap: f32, wt: W, max_w: f32) -> Vec<String> {
    if p.width(text, cap, wt) <= max_w {
        return vec![text.to_string()];
    }
    let mut first = String::new();
    let mut rest_start = 0;
    for (i, word) in text.split_inclusive(' ').enumerate() {
        let cand = format!("{first}{word}");
        if p.width(cand.trim_end(), cap, wt) > max_w {
            if i == 0 {
                // one very long word: cut it
                return vec![ellipsize(p, text, cap, wt, max_w)];
            }
            break;
        }
        first = cand;
        rest_start += word.len();
    }
    let rest = text[rest_start..].trim();
    if rest.is_empty() {
        return vec![first.trim_end().to_string()];
    }
    vec![first.trim_end().to_string(), ellipsize(p, rest, cap, wt, max_w)]
}

// ------------------------------------------------------------- scrolling title
//
// A title too long for its box is not painted on the cached layer: its position is
// recorded here and `ProScreen::draw` scrolls it directly on the frame (hold 3 s,
// scroll, hold, ...), with soft fades at the edges. Only the scrolling phase asks
// for a higher frame rate, so a short title costs nothing extra.

#[derive(Clone, PartialEq)]
struct MarqueeSpec {
    x: f32,
    w: f32,
    base: f32,
    cap: f32,
    weight: W,
    color: Rgb,
    text: String,
}

thread_local! {
    static MARQ: std::cell::RefCell<Vec<MarqueeSpec>> = const { std::cell::RefCell::new(Vec::new()) };
}

const MARQ_HOLD: f32 = 3.0;
const MARQ_SPEED: f32 = 42.0; // px/s at k = 1

fn sfw(w: W) -> crate::smooth_font::Weight {
    match w {
        W::Light => crate::smooth_font::Weight::Light,
        W::Regular => crate::smooth_font::Weight::Regular,
        W::Medium => crate::smooth_font::Weight::Medium,
    }
}

/// One line of text in a box `w` wide: drawn as-is when it fits, scrolled otherwise.
fn title_line(p: &mut Painter, x: f32, base: f32, w: f32, text: &str, cap: f32, wt: W, color: Rgb) {
    if p.width(text, cap, wt) <= w {
        p.text(x, base, text, cap, wt, color, 255);
    } else {
        MARQ.with(|m| m.borrow_mut().push(MarqueeSpec { x, w, base, cap, weight: wt, color, text: text.to_string() }));
    }
}

/// (offset in px, scrolling now) for a text `tw` wide, `t` seconds after it appeared.
fn marquee_offset(tw: f32, gap: f32, speed: f32, t: f32) -> (f32, bool) {
    let run = (tw + gap) / speed;
    let phase = t % (MARQ_HOLD + run);
    if phase < MARQ_HOLD { (0.0, false) } else { ((phase - MARQ_HOLD) * speed, true) }
}

/// Draws a scrolling title straight onto the frame (`ox`, `oy` = where the layer sits).
fn paint_marquee(fb: &mut Framebuffer, ox: u32, oy: u32, m: &MarqueeSpec, k: f32, t: f32) {
    let px = crate::smooth_font::px_for_cap(m.cap);
    let wt = sfw(m.weight);
    let tw = crate::smooth_font::text_width(&m.text, px, wt);
    let gap = 70.0 * k;
    let (off, _) = marquee_offset(tw, gap, MARQ_SPEED * k, t);
    let fade = 22.0 * k;
    let (x0, x1) = (m.x, m.x + m.w);
    let fw = fb.width() as i32;
    let fh = fb.height() as i32;
    let (cr, cg, cb) = (m.color.0 as u32, m.color.1 as u32, m.color.2 as u32);
    let bytes = fb.as_bytes_mut();
    for copy in 0..2 {
        let start = m.x - off + copy as f32 * (tw + gap);
        if start > x1 || start + tw < x0 {
            continue;
        }
        crate::smooth_font::draw_with(&m.text, start, m.base, px, wt, |gx, gy, cov| {
            let fx = gx as f32 + 0.5;
            if fx < x0 || fx >= x1 {
                return;
            }
            let mut a = cov as f32 / 255.0;
            if off > 0.0 {
                a *= ((fx - x0) / fade).clamp(0.0, 1.0);
            }
            a *= ((x1 - fx) / fade).clamp(0.0, 1.0);
            if a <= 0.0 {
                return;
            }
            let (fx2, fy2) = (gx + ox as i32, gy + oy as i32);
            if fx2 < 0 || fy2 < 0 || fx2 >= fw || fy2 >= fh {
                return;
            }
            let i = ((fy2 * fw + fx2) * 3) as usize;
            let a8 = (a * 255.0) as u32;
            let inv = 255 - a8;
            bytes[i] = ((cr * a8 + bytes[i] as u32 * inv) / 255) as u8;
            bytes[i + 1] = ((cg * a8 + bytes[i + 1] as u32 * inv) / 255) as u8;
            bytes[i + 2] = ((cb * a8 + bytes[i + 2] as u32 * inv) / 255) as u8;
        });
    }
}

/// "Song (Official Music Video) [4K]" -> "Song".
fn clean_title(t: &str) -> String {
    const JUNK: &[&str] = &[
        "official", "video", "audio", "lyric", "mv", "m/v", "4k", "hd", "hq", "visualizer", "remaster", "explicit", "clip", "music",
    ];
    let mut out = String::new();
    let mut depth = 0;
    let mut chunk = String::new();
    for c in t.chars() {
        match c {
            '(' | '[' | '【' => {
                if depth == 0 {
                    chunk.clear();
                }
                depth += 1;
                chunk.push(c);
            }
            ')' | ']' | '】' if depth > 0 => {
                chunk.push(c);
                depth -= 1;
                if depth == 0 {
                    let lc = chunk.to_lowercase();
                    if !JUNK.iter().any(|j| lc.contains(j)) {
                        out.push_str(&chunk);
                    }
                    chunk.clear();
                }
            }
            _ if depth > 0 => chunk.push(c),
            _ => out.push(c),
        }
    }
    out.push_str(&chunk);
    let mut s = out.split_whitespace().collect::<Vec<_>>().join(" ");
    for tail in [" - Topic", " | Official", " |"] {
        if let Some(i) = s.find(tail) {
            if i > 0 {
                s.truncate(i);
            }
        }
    }
    let s = s.trim_matches(|c: char| c == '-' || c == '|' || c.is_whitespace()).to_string();
    if s.is_empty() { t.trim().to_string() } else { s }
}

// ------------------------------------------------------------------ shapes

/// Arc from `start` degrees (0 = 12 o'clock, clockwise) over `sweep` degrees, round ends.
/// `color(t)`: t = 0..1 along the drawn arc.
fn arc2(p: &mut Painter, cx: f32, cy: f32, r_out: f32, thick: f32, start: f32, sweep: f32, color: &dyn Fn(f32) -> Rgb, alpha: u8) {
    if sweep <= 0.1 || alpha == 0 {
        return;
    }
    let r_in = r_out - thick;
    let r_mid = r_out - thick / 2.0;
    let cap = thick / 2.0;
    let rad = |deg: f32| deg.to_radians();
    let end = start + sweep;
    let (sx, sy) = (cx + r_mid * rad(start).sin(), cy - r_mid * rad(start).cos());
    let (ex, ey) = (cx + r_mid * rad(end).sin(), cy - r_mid * rad(end).cos());
    let x0 = (cx - r_out - 1.0).floor() as i32;
    let x1 = (cx + r_out + 1.0).ceil() as i32;
    let y0 = (cy - r_out - 1.0).floor() as i32;
    let y1 = (cy + r_out + 1.0).ceil() as i32;
    for py in y0..y1 {
        for px in x0..x1 {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let (dx, dy) = (fx - cx, fy - cy);
            let d = dx.hypot(dy);
            if d > r_out + 1.0 || d < r_in - 1.0 {
                continue;
            }
            let mut ang = dx.atan2(-dy).to_degrees();
            if ang < 0.0 {
                ang += 360.0;
            }
            let mut rel = ang - start;
            let centre = sweep / 2.0;
            while rel <= centre - 180.0 {
                rel += 360.0;
            }
            while rel > centre + 180.0 {
                rel -= 360.0;
            }
            let radial = (r_out + 0.5 - d).clamp(0.0, 1.0) * (d - r_in + 0.5).clamp(0.0, 1.0);
            let arc_px = d * std::f32::consts::PI / 180.0;
            let inside = ((rel * arc_px) + 0.5).clamp(0.0, 1.0) * (((sweep - rel) * arc_px) + 0.5).clamp(0.0, 1.0);
            let mut cov = radial * inside;
            for (qx, qy) in [(sx, sy), (ex, ey)] {
                cov = cov.max((cap + 0.5 - (fx - qx).hypot(fy - qy)).clamp(0.0, 1.0));
            }
            if cov > 0.0 {
                let t = (rel / sweep).clamp(0.0, 1.0);
                p.layer.over(px, py, color(t), (alpha as f32 * cov).round() as u8);
            }
        }
    }
}

/// Thin horizontal bar: track + gradient fill.
fn hbar(p: &mut Painter, x: f32, y: f32, w: f32, h: f32, frac: Option<f32>, colors: [Rgb; 2]) {
    p.layer.fill_rrect((x, y, w, h), h / 2.0, &|_| WHITE, 22);
    if let Some(f) = frac {
        let f = f.clamp(0.0, 1.0);
        if f > 0.0 {
            let fw = (w * f).max(h);
            p.layer.fill_rrect((x, y, fw, h), h / 2.0, &move |t| gradient_at(&colors, t * f.max(0.05)), 255);
        }
    }
}

/// Cover + title (scrolls) + artist + progress, in the box (x, y, w, h).
fn now_playing_compact(p: &mut Painter, r: R, sc: &Scale, d: &Data, cover: Option<&Cover>, show_idle: bool) {
    let k = sc.k;
    let (x, y, w, h) = r;
    let accent = cover.map_or(th().muted, |c| c.accent);
    let Some(t) = d.track else {
        if show_idle {
            music_note(p, x + h * 0.45, y + h / 2.0, h * 0.32, th().dim);
            p.text(x + h + 6.0 * k, y + h / 2.0 + sc.m / 2.0, "Nothing playing", sc.m, W::Regular, th().dim, 255);
        }
        return;
    };
    let cs = h;
    match cover {
        Some(c) => blit_cover(p, c, x, y, cs, 10.0 * k),
        None => {
            p.layer.fill_rrect((x, y, cs, cs), 10.0 * k, &|t| lerp_rgb((40, 42, 52), (24, 25, 32), t), 255);
            music_note(p, x + cs / 2.0, y + cs / 2.0, cs * 0.34, th().muted);
        }
    }
    let tx = x + cs + 14.0 * k;
    let tw = x + w - tx;
    let title = clean_title(&t.title);
    title_line(p, tx, y + sc.m + 2.0 * k, tw, &title, sc.m, W::Medium, th().text);
    let artist = ellipsize(p, &t.artist, sc.s, W::Regular, tw);
    p.text(tx, y + sc.m + 2.0 * k + 10.0 * k + sc.s, &artist, sc.s, W::Regular, th().muted, 255);
    if let Some(tl) = t.timeline.filter(|tl| tl.duration_ms > 0) {
        let f = (tl.position_now_ms() as f32 / tl.duration_ms as f32).clamp(0.0, 1.0);
        let by = y + h - 4.0 * k;
        hbar(p, tx, by, tw, 4.0 * k, Some(f), [accent, lerp_rgb(accent, WHITE, 0.45)]);
    }
}

fn clock_parts(now: &chrono::DateTime<Local>) -> (String, String, String) {
    let t = now.format("%H:%M").to_string();
    let wd = i18n::weekday(now).to_uppercase();
    let date = crate::tiles::nice_case(&i18n::day_month_year(now));
    (t, wd, date)
}

fn weather_text(d: &Data) -> Option<(WeatherIcon, String, String)> {
    let wt = d.weather?;
    let temp = format!("{:.0}\u{b0}", wt.temp_in(d.fahrenheit).round() + 0.0);
    let cond = crate::tiles::nice_case(weather_icon::condition_name(wt.icon));
    let city = wt.city.clone().unwrap_or_default();
    let line = if city.is_empty() { cond } else { format!("{cond} \u{b7} {city}") };
    Some((wt.icon, temp, line))
}

fn vram_text(g: &GpuAmdData) -> (String, String, Option<f32>) {
    match (g.vram_used_mb, g.vram_total_mb) {
        (Some(u), Some(t)) if t > 0 => (format!("{:.1}", u as f32 / 1024.0), format!("/{:.0} GB", t as f32 / 1024.0), Some(u as f32 / t as f32)),
        (Some(u), _) => (format!("{:.1}", u as f32 / 1024.0), "GB".into(), None),
        _ => ("--".into(), String::new(), None),
    }
}

fn tval(v: Option<f32>, warn: f32, hot: f32) -> (String, Rgb) {
    match v {
        Some(t) => (format!("{t:.0}"), temp_color(t, warn, hot)),
        None => ("--".into(), th().dim),
    }
}

// --------------------------------------------------------------- COCKPIT
//
// Car instrument cluster: two 240° gauges (CPU left, GPU right) with ticks, the clock
// and the rest of the readings in the middle, now playing in a pill at the bottom.

fn gauge(p: &mut Painter, cx: f32, cy: f32, r: f32, sc: &Scale, colors: [Rgb; 2], label: &str, frac: Option<f32>, value: &str, name: &str, stats: [(&str, String, &str, Rgb); 3]) {
    let k = sc.k;
    let thick = 15.0 * k;
    let (start, sweep) = (240.0f32, 240.0f32);
    // ticks
    let rt = r - thick - 12.0 * k;
    for i in 0..=48 {
        let a = (start + sweep * i as f32 / 48.0).to_radians();
        let major = i % 12 == 0;
        let len = if major { 12.0 * k } else { 6.0 * k };
        let (s, c) = a.sin_cos();
        capsule(p, (cx + s * rt, cy - c * rt), (cx + s * (rt - len), cy - c * (rt - len)), if major { 1.3 * k } else { 0.8 * k }, th().text, if major { 150 } else { 60 });
    }
    arc2(p, cx, cy, r, thick, start, sweep, &|_| WHITE, 22);
    if let Some(f) = frac.map(|f| f.clamp(0.0, 1.0)) {
        if f > 0.003 {
            arc2(p, cx, cy, r, thick, start, sweep * f, &move |t| gradient_at(&colors, t), 255);
            let a = (start + sweep * f).to_radians();
            let rm = r - thick / 2.0;
            let (ex, ey) = (cx + a.sin() * rm, cy - a.cos() * rm);
            circle(p, ex, ey, thick * 1.05, colors[1], 70);
            circle(p, ex, ey, thick * 0.42, WHITE, 255);
        }
    }
    // label + value + name in the middle
    let lw = spaced_width(p, label, sc.s, W::Medium, 2.0 * k);
    spaced(p, cx - lw / 2.0, cy - 58.0 * k, label, sc.s, W::Medium, colors[1], 2.0 * k);
    let vcap = 78.0 * k;
    let vw = value_unit_width(p, value, if frac.is_some() { "%" } else { "" }, vcap, W::Light);
    value_unit(p, cx - vw / 2.0, cy + vcap / 2.0 - 6.0 * k, value, if frac.is_some() { "%" } else { "" }, vcap, W::Light, th().text);
    let name = ellipsize(p, name, sc.s, W::Regular, r * 1.3);
    let nw = p.width(&name, sc.s, W::Regular);
    p.text(cx - nw / 2.0, cy + vcap / 2.0 + 24.0 * k, &name, sc.s, W::Regular, th().dim, 255);
    // three stats in the opening at the bottom
    let open_w = 2.0 * r * (60.0f32).to_radians().sin() + 40.0 * k;
    let cw = open_w / 3.0;
    let top = cy + r * 0.5 + 4.0 * k;
    for (i, (l, v, u, c)) in stats.iter().enumerate() {
        let x = cx - open_w / 2.0 + i as f32 * cw;
        let lw = spaced_width(p, l, sc.s * 0.9, W::Medium, 1.4 * k);
        spaced(p, x + (cw - lw) / 2.0, top + sc.s, l, sc.s * 0.9, W::Medium, th().dim, 1.4 * k);
        let cap = fit_vu(p, v, u, 28.0 * k, cw - 6.0 * k, W::Medium);
        let vw = value_unit_width(p, v, u, cap, W::Medium);
        value_unit(p, x + (cw - vw) / 2.0, top + sc.s + 8.0 * k + 28.0 * k, v, u, cap, W::Medium, *c);
    }
}

fn paint_cockpit(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, now: &chrono::DateTime<Local>, cover: Option<&Cover>) {
    let k = sc.k;
    let r = 186.0 * k;
    let cy = 22.0 * k + r;
    let lcx = 34.0 * k + r;
    let rcx = w - 34.0 * k - r;
    if style() != Style::Flat {
        for cx in [lcx, rcx] {
            circle(p, cx, cy, r + 14.0 * k, th().panel, 255);
        }
    }
    let (tv, tc) = tval(d.cpu_temp, 80.0, 90.0);
    let pw = d.cpu_power.map_or("--".into(), |v| format!("{v:.0}"));
    let clk = d.cpu_mhz.map_or("--".into(), |m| format!("{:.2}", m as f32 / 1000.0));
    gauge(p, lcx, cy, r, sc, th().cpu, "CPU", Some(d.cpu_pct / 100.0), &format!("{:.0}", d.cpu_pct), d.cpu_name,
        [("TEMP", tv, "\u{b0}C", tc), ("POWER", pw, "W", th().text), ("CLOCK", clk, "GHz", th().text)]);
    let g = d.gpu;
    let (gt, gtc) = tval(g.temp_edge_c.map(|v| v as f32), 80.0, 90.0);
    let (ht, htc) = tval(g.temp_hotspot_c.map(|v| v as f32), 95.0, 105.0);
    let gp = g.power_w.map_or("--".into(), |v| v.to_string());
    let name = g.name.clone().unwrap_or_else(|| "GPU".into());
    gauge(p, rcx, cy, r, sc, th().gpu, "GPU", d.gpu_pct.map(|v| v / 100.0), &d.gpu_pct.map_or("--".into(), |v| format!("{v:.0}")), &name,
        [("TEMP", gt, "\u{b0}C", gtc), ("HOTSPOT", ht, "\u{b0}C", htc), ("POWER", gp, "W", th().text)]);

    // centre column
    let x0 = lcx + r + 46.0 * k;
    let x1 = rcx - r - 46.0 * k;
    let cw = x1 - x0;
    let mid = (x0 + x1) / 2.0;
    let (t, wd, date) = clock_parts(now);
    let tcap = 104.0 * k;
    let tw = p.width(&t, tcap, W::Light);
    p.text(mid - tw / 2.0, 22.0 * k + tcap, &t, tcap, W::Light, th().text, 255);
    let line = match weather_text(d) {
        Some((_, temp, cond)) => format!("{wd}  \u{b7}  {date}  \u{b7}  {temp} {cond}"),
        None => format!("{wd}  \u{b7}  {date}"),
    };
    let line = ellipsize(p, &line, sc.s, W::Medium, cw);
    let lw = p.width(&line, sc.s, W::Medium);
    p.text(mid - lw / 2.0, 22.0 * k + tcap + 30.0 * k, &line, sc.s, W::Medium, th().muted, 255);

    // chips row
    let (vv, vu, vf) = vram_text(g);
    let ram_f = if d.ram_total_mb > 0 { Some(d.ram_used_mb as f32 / d.ram_total_mb as f32) } else { None };
    let (dn, du) = rate(d.net_kb.0);
    let fps = g.fps.filter(|f| *f > 0);
    let chips: [(&str, String, String, Option<f32>, [Rgb; 2], Rgb); 6] = [
        ("RAM", gb(d.ram_used_mb), "GB".into(), ram_f, th().ram, th().text),
        ("VRAM", vv, vu, vf, th().gpu, th().text),
        ("SSD", d.disk.and_then(|x| x.temp_c).map_or("--".into(), |t| t.to_string()), "\u{b0}C".into(), d.disk.map(|x| x.used_frac()), th().ssd, th().text),
        ("GPU FAN", g.fan_rpm.map_or("--".into(), |v| v.max(0).to_string()), "RPM".into(), None, th().gpu, th().text),
        ("DOWN", dn, du.into(), None, th().net, th().text),
        ("FPS", fps.map_or("--".into(), |f| f.to_string()), fps.map_or(String::new(), |f| format!("{:.1} ms", 1000.0 / f as f32)), None, th().gpu, if fps.is_some() { th().fps } else { th().dim }),
    ];
    let n = chips.len() as f32;
    let chw = cw / n;
    let top = 214.0 * k;
    for (i, (l, v, u, f, cols, vc)) in chips.iter().enumerate() {
        let x = x0 + i as f32 * chw + 8.0 * k;
        let iw = chw - 16.0 * k;
        if i > 0 {
            capsule(p, (x - 8.0 * k, top + 4.0 * k), (x - 8.0 * k, top + 66.0 * k), 0.5, WHITE, 34);
        }
        spaced(p, x, top + sc.s, l, sc.s, W::Medium, th().dim, 1.4 * k);
        let cap = fit_vu(p, v, u, 30.0 * k, iw, W::Medium);
        value_unit(p, x, top + sc.s + 10.0 * k + 30.0 * k, v, u, cap, W::Medium, *vc);
        if f.is_some() {
            hbar(p, x, top + sc.s + 10.0 * k + 30.0 * k + 12.0 * k, iw, 4.0 * k, *f, *cols);
        }
    }

    // now playing pill
    let ph = 84.0 * k;
    let py = h - 14.0 * k - ph;
    let pr = (x0, py, cw, ph);
    if style() == Style::Flat {
        capsule(p, (x0, py - 10.0 * k), (x1, py - 10.0 * k), 0.5, WHITE, 34);
    } else {
        p.layer.fill_rrect(pr, ph / 2.0, &|_| th().panel, 255);
        p.layer.stroke_rrect(pr, ph / 2.0, 1.0, WHITE, 20);
    }
    let inner = (x0 + 14.0 * k, py + 12.0 * k, cw - 40.0 * k, ph - 24.0 * k);
    now_playing_compact(p, inner, sc, d, cover, true);
}

// ------------------------------------------------------------------ FOCUS
//
// A few huge numbers: the temperatures of CPU and GPU, memory use and the clock,
// each with a short row of details and a soft load graph underneath.

fn focus_section(p: &mut Painter, r: R, sc: &Scale, vcap: f32, wide_last: bool, colors: [Rgb; 2], label: &str, hero: &str, hero_unit: &str, hero_color: Rgb, side: &str, stats: &[(&str, String, String, Rgb)], hist: Option<(&VecDeque<f32>, f32)>) {
    let k = sc.k;
    let (x, y, w, h) = r;
    circle(p, x + 5.0 * k, y + sc.s * 0.5, 5.0 * k, colors[0], 255);
    spaced(p, x + 18.0 * k, y + sc.s, label, sc.s, W::Medium, th().muted, 2.0 * k);
    let hcap = 108.0 * k;
    let hb = y + sc.s + 18.0 * k + hcap;
    let cap = fit_vu(p, hero, hero_unit, hcap, w * 0.6 - 14.0 * k, W::Light);
    // a small history line in the free corner next to the big number
    if let Some((hq, max)) = hist {
        sparkline(p, (x + w * 0.6, y + sc.s + 40.0 * k, w * 0.4, 40.0 * k), hq, max, colors, k);
    }
    let hw = p.text(x - 6.0 * k, hb, hero, cap, W::Light, hero_color, 255);
    if !hero_unit.is_empty() {
        p.text(x - 6.0 * k + hw + 4.0 * k, hb - cap + sc.l, hero_unit, sc.l, W::Light, th().muted, 255);
    }
    if !side.is_empty() {
        let sw = p.width(side, sc.m, W::Regular);
        p.text(x + w - sw, hb, side, sc.m, W::Regular, th().muted, 255);
    }
    // stats: two per row, three rows spread evenly to the bottom, all one size
    let _ = wide_last;
    let cw = w / 2.0;
    let top = hb + 18.0 * k;
    let row_h = sc.s + 8.0 * k + vcap;
    let pitch = ((y + h) - top - row_h) / 2.0;
    for (i, (l, v, u, c)) in stats.iter().enumerate() {
        let sx = x + (i % 2) as f32 * cw;
        let sy = top + (i / 2) as f32 * pitch;
        spaced(p, sx, sy + sc.s, l, sc.s, W::Medium, th().dim, 1.5 * k);
        let cap = fit_vu(p, v, u, vcap, cw - 10.0 * k, W::Medium);
        value_unit(p, sx, sy + row_h, v, u, cap, W::Medium, *c);
    }
}

fn paint_focus(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, now: &chrono::DateTime<Local>, hist: (&VecDeque<f32>, &VecDeque<f32>, &VecDeque<f32>), cover: Option<&Cover>) {
    let k = sc.k;
    let m = 24.0 * k;
    let gap = 36.0 * k;
    let weights = [452.0f32, 482.0, 452.0, 386.0];
    let avail = w - 2.0 * m - gap * 3.0;
    let sum: f32 = weights.iter().sum();
    let mut xs = Vec::new();
    let mut x = m;
    for wt in weights {
        let cw = avail * wt / sum;
        xs.push((x, cw));
        x += cw + gap;
    }
    for (x, _) in xs.iter().skip(1) {
        capsule(p, (x - gap / 2.0, m), (x - gap / 2.0, h - m), 0.5, WHITE, 34);
    }
    let r = |i: usize| (xs[i].0, m, xs[i].1, h - 2.0 * m);
    // one value size for every stat of the three sections (the widest values must fit)
    let mut vcap = 60.0 * k;
    for (i, tpl) in [(0, [("4.35", "GHz"), ("1480", "RPM"), ("23:59", "h")]), (1, [("2491", "MHz"), ("1480", "RPM"), ("16.0", "GB")]), (2, [("47.9", "GB"), ("12.4", "MB/s"), ("850", "KB/s")])] {
        for (v, u) in tpl {
            vcap = vcap.min(fit_vu(p, v, u, 60.0 * k, xs[i].1 / 2.0 - 10.0 * k, W::Medium));
        }
    }
    // CPU
    let (tv, tc) = tval(d.cpu_temp, 80.0, 90.0);
    let (uv, uu) = uptime_str(d.uptime_s);
    let mut cpu_stats = vec![
        ("LOAD", format!("{:.0}", d.cpu_pct), "%".to_string(), th().text),
        ("POWER", d.cpu_power.map_or("--".into(), |v| format!("{v:.0}")), "W".into(), th().text),
        ("CLOCK", d.cpu_mhz.map_or("--".into(), |m| format!("{:.2}", m as f32 / 1000.0)), "GHz".into(), th().text),
        ("UPTIME", uv, uu, th().text),
    ];
    if let Some(rpm) = d.cpu_fan_rpm {
        cpu_stats.push(("FAN", rpm.to_string(), "RPM".into(), th().text));
    }
    focus_section(p, r(0), sc, vcap, false, th().cpu, "CPU TEMPERATURE", &tv, "\u{b0}C", tc, "", &cpu_stats, Some((hist.0, 100.0)));
    // GPU: FPS has its own cell, it never replaces another value
    let g = d.gpu;
    let (gt, gtc) = tval(g.temp_edge_c.map(|v| v as f32), 80.0, 90.0);
    let fps = g.fps.filter(|f| *f > 0);
    let side = g.temp_hotspot_c.map_or(String::new(), |t| format!("hotspot {t}\u{b0}"));
    let gpu_stats = [
        ("LOAD", d.gpu_pct.map_or("--".into(), |v| format!("{v:.0}")), "%".to_string(), th().text),
        ("POWER", g.power_w.map_or("--".into(), |v| v.to_string()), "W".into(), th().text),
        ("FAN", g.fan_rpm.map_or("--".into(), |v| v.max(0).to_string()), "RPM".into(), th().text),
        ("CLOCK", g.clock_mhz.filter(|c| *c > 0).map_or("--".into(), |v| v.to_string()), "MHz".into(), th().text),
        ("VRAM", vram_text(g).0, "GB".into(), th().text),
        match fps {
            Some(f) => ("FPS", f.to_string(), format!("{:.1}ms", 1000.0 / f as f32), th().fps),
            None => ("FPS", "--".to_string(), String::new(), th().dim),
        },
    ];
    focus_section(p, r(1), sc, vcap, false, th().gpu, "GPU TEMPERATURE", &gt, "\u{b0}C", gtc, &side, &gpu_stats, Some((hist.1, 100.0)));
    // memory + storage + network
    let ram_pct = if d.ram_total_mb > 0 { d.ram_used_mb as f32 * 100.0 / d.ram_total_mb as f32 } else { 0.0 };
    let (dn, du) = rate(d.net_kb.0);
    let (un, uu2) = rate(d.net_kb.1);
    let mem_stats = [
        ("USED", gb(d.ram_used_mb), "GB".to_string(), th().text),
        ("FREE", gb(d.ram_total_mb.saturating_sub(d.ram_used_mb)), "GB".into(), th().text),
        ("SSD", d.disk.and_then(|x| x.temp_c).map_or("--".into(), |t| t.to_string()), "\u{b0}C".into(), d.disk.and_then(|x| x.temp_c).map_or(th().dim, |t| temp_color(t as f32, 60.0, 70.0))),
        ("DISK", d.disk.map_or("--".into(), |x| format!("{:.0}", x.used_frac() * 100.0)), "%".into(), th().text),
        ("DOWN", dn, du.to_string(), th().text),
        ("UP", un, uu2.to_string(), th().text),
    ];
    let side = format!("of {} GB", gb(d.ram_total_mb));
    focus_section(p, r(2), sc, vcap, false, th().ram, "MEMORY", &format!("{ram_pct:.0}"), "%", th().text, &side, &mem_stats, Some((hist.2, hist.2.iter().cloned().fold(256.0f32, f32::max) * 1.1)));
    // time + weather + now playing
    let (x, y, sw, sh) = r(3);
    let (t, wd, date) = clock_parts(now);
    spaced(p, x, y + sc.s, &format!("{wd}  \u{b7}  {}", date.to_uppercase()), sc.s, W::Medium, th().muted, 2.0 * k);
    let hcap = 108.0 * k;
    let cap = p.fit(&t, hcap, sw, W::Light);
    let hb = y + sc.s + 18.0 * k + hcap;
    p.text(x - 6.0 * k, hb, &t, cap, W::Light, th().text, 255);
    if let Some((icon, temp, line)) = weather_text(d) {
        let wy = hb + 20.0 * k;
        weather_icon(p, x - 3.0 * k, wy, 46.0 * k, icon, !(6..19).contains(&now.hour()));
        let base = wy + 23.0 * k + sc.l / 2.0;
        let tw = p.text(x + 52.0 * k, base, &temp, sc.l, W::Medium, th().text, 255);
        let line = ellipsize(p, &line, sc.m, W::Regular, sw - 66.0 * k - tw);
        p.text(x + 64.0 * k + tw, base, &line, sc.m, W::Regular, th().muted, 255);
    }
    // the next three days: day + icon, then high / low
    if let Some(wt) = d.weather {
        let days: Vec<_> = wt.forecast.iter().skip(1).take(3).collect();
        let fy = hb + 110.0 * k;
        let cw = sw / 3.0;
        for (i, f) in days.iter().enumerate() {
            let cx = x + i as f32 * cw;
            let dn = i18n::weekday_short(f.weekday).to_uppercase();
            let dw = spaced(p, cx, fy, &dn, sc.s, W::Medium, th().dim, 1.4 * k);
            weather_icon(p, cx + dw + 8.0 * k, fy - sc.s / 2.0 - 15.0 * k, 30.0 * k, f.icon, false);
            let hi = format!("{:.0}\u{b0}", f.temp_max_in(d.fahrenheit).round() + 0.0);
            let lo = format!("{:.0}\u{b0}", f.temp_min_in(d.fahrenheit).round() + 0.0);
            let ty = fy + 14.0 * k + sc.l * 0.8;
            let hw = p.text(cx, ty, &hi, sc.l * 0.8, W::Medium, th().text, 255);
            p.text(cx + hw + 8.0 * k, ty, &lo, sc.m * 1.15, W::Regular, th().dim, 255);
        }
    }
    let big = Scale { m: sc.m * 1.3, s: sc.s * 1.25, ..*sc };
    now_playing_compact(p, (x, y + sh - 90.0 * k, sw, 90.0 * k), &big, d, cover, true);
}
