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

const PANEL: Rgb = (12, 13, 17);
const TEXT: Rgb = (242, 243, 246);
const MUTED: Rgb = (168, 172, 184);
const DIM: Rgb = (118, 122, 136);
const WHITE: Rgb = (255, 255, 255);
const CPU_C: [Rgb; 2] = [(124, 92, 255), (196, 140, 255)];
const GPU_C: [Rgb; 2] = [(14, 165, 233), (103, 232, 249)];
const RAM_C: [Rgb; 2] = [(236, 72, 153), (251, 164, 206)];
const SSD_C: [Rgb; 2] = [(245, 158, 11), (253, 214, 99)];
const NET_C: [Rgb; 2] = [(16, 185, 129), (110, 231, 183)];
const UP_C: Rgb = (96, 165, 250);
const FPS_C: Rgb = (163, 230, 53);
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
        fb.clear(0, 0, 0);
        self.push_history(d);
        self.update_cover(d);
        let now = Local::now();
        let progress = d.track.and_then(|t| t.timeline).map(|tl| (tl.position_now_ms() / 1000, tl.duration_ms / 1000));
        let sig = format!(
            "{area:?}|{:?}|{}|{}|{:.0}|{:?}|{:?}|{:?}|{:?}|{:?}|{}|{}|{:?}|{:.1}{:.1}|{:.0}{:.0}|{:?}|{}|{:?}|{:?}|{}",
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
            paint(&mut p, w as f32, h as f32, d, &now, hist, cover);
            self.sig = sig;
        }
        self.layer.composite_onto(fb, area.0, area.1);
    }
}

// ------------------------------------------------------------------- layout

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
    let m = (10.0 * k).round();
    let gap = (10.0 * k).round();
    let ph = h - 2.0 * m;
    // Column widths (design units), shared out over the available width.
    let weights = [272.0f32, 458.0, 508.0, 288.0, 334.0];
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

    draw_time(p, (xs[0].0, top, xs[0].1, ph), k, d, now);
    draw_cpu(p, (xs[1].0, top, xs[1].1, ph), k, d, hist.0);
    draw_gpu(p, (xs[2].0, top, xs[2].1, ph), k, d, hist.1);
    draw_ram(p, (xs[3].0, top, xs[3].1, half), k, d);
    draw_disk(p, (xs[3].0, top + half + gap, xs[3].1, ph - half - gap), k, d);
    let media_h = ((ph - gap) * 0.60).floor();
    draw_media(p, (xs[4].0, top, xs[4].1, media_h), k, d, cover);
    draw_net(p, (xs[4].0, top + media_h + gap, xs[4].1, ph - media_h - gap), k, d, hist.2);
}

// --------------------------------------------------------------- primitives

fn panel(p: &mut Painter, r: R, k: f32) {
    let r = (r.0.round(), r.1.round(), r.2.round(), r.3.round());
    let rad = 18.0 * k;
    p.layer.fill_rrect_solid(r, rad, PANEL, 255);
    p.layer.stroke_rrect(r, rad, 1.0, WHITE, 20);
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
    let cap = 15.0 * k;
    let base = r.1 + pd + cap;
    circle(p, r.0 + pd + 4.5 * k, base - cap / 2.0, 4.5 * k, accent, 255);
    spaced(p, r.0 + pd + 17.0 * k, base, title, cap, W::Medium, MUTED, 2.0 * k);
    if !right.is_empty() {
        let iw = spaced_width(p, right, cap, W::Medium, 1.2 * k);
        spaced(p, r.0 + r.2 - pd - iw, base, right, cap, W::Medium, DIM, 1.2 * k);
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
    vw + g + p.text(x + vw + g, base, unit, (cap * 0.46).max(7.0), W::Regular, MUTED, 255)
}

fn value_unit_width(p: &Painter, value: &str, unit: &str, cap: f32, wt: W) -> f32 {
    let mut t = p.width(value, cap, wt);
    if !unit.is_empty() {
        t += (cap * 0.12).max(2.0) + p.width(unit, (cap * 0.46).max(7.0), W::Regular);
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
    value_unit(p, cx - tw / 2.0, base, value, unit, cap, W::Medium, TEXT);
    if !label.is_empty() {
        let lw = spaced_width(p, label, lcap, W::Medium, 1.5 * k);
        spaced(p, cx - lw / 2.0, base + 9.0 * k + lcap, label, lcap, W::Medium, DIM, 1.5 * k);
    }
}

/// One stat: small label, big value + unit under it. Returns the value baseline.
fn stat(p: &mut Painter, x: f32, top: f32, w: f32, label: &str, value: &str, unit: &str, vc: Rgb, k: f32, vcap: f32) -> f32 {
    let lcap = 13.0 * k;
    spaced(p, x, top + lcap, label, lcap, W::Medium, DIM, 1.6 * k);
    let cap = fit_vu(p, value, unit, vcap, w - 6.0 * k, W::Medium);
    let base = top + lcap + 10.0 * k + vcap;
    value_unit(p, x, base, value, unit, cap, W::Medium, vc);
    base
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
        TEXT
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

fn draw_time(p: &mut Painter, r: R, k: f32, d: &Data, now: &chrono::DateTime<Local>) {
    panel(p, r, k);
    let (x, y, w, h) = r;
    let pd = pad(k);
    let inner = w - 2.0 * pd;
    // weekday + date
    let wd = i18n::weekday(now).to_uppercase();
    let wcap = 15.0 * k;
    spaced(p, x + pd, y + pd + wcap, &wd, wcap, W::Medium, MUTED, 2.4 * k);
    let date = crate::tiles::nice_case(&i18n::day_month_year(now));
    let dcap = p.fit(&date, 23.0 * k, inner, W::Regular);
    let date_base = y + pd + wcap + 13.0 * k + dcap;
    p.text(x + pd, date_base, &date, dcap, W::Regular, TEXT, 255);
    // big time
    let t = now.format("%H:%M").to_string();
    let tcap = p.fit(&t, 100.0 * k, inner, W::Light);
    let tw = p.width(&t, tcap, W::Light);
    let tbase = date_base + 26.0 * k + tcap;
    let tx = x + (w - tw) / 2.0;
    p.text(tx, tbase, &t, tcap, W::Light, TEXT, 255);
    // seconds: a thin progress line under the time
    let sy = tbase + 18.0 * k;
    let frac = now.second() as f32 / 59.0;
    p.layer.fill_rrect((tx, sy, tw, 4.0 * k), 2.0 * k, &|_| WHITE, 24);
    if frac > 0.0 {
        let fw = (tw * frac).max(4.0 * k);
        p.layer.fill_rrect((tx, sy, fw, 4.0 * k), 2.0 * k, &move |t| gradient_at(&[(150, 154, 168), WHITE], t), 255);
    }
    // separator + weather
    let sep = (sy + 22.0 * k).round();
    capsule(p, (x + pd, sep), (x + w - pd, sep), 0.5, WHITE, 30);
    let lcap = 14.0 * k;
    let Some(wt) = d.weather else {
        spaced(p, x + pd, sep + 18.0 * k + lcap, "WEATHER", lcap, W::Medium, DIM, 1.6 * k);
        p.text(x + pd, sep + 70.0 * k, "--", 34.0 * k, W::Regular, DIM, 255);
        return;
    };
    let city = wt.city.clone().unwrap_or_else(|| "WEATHER".into()).to_uppercase();
    let city = ellipsize(p, &city, lcap, W::Medium, inner - 24.0 * k);
    spaced(p, x + pd, sep + 18.0 * k + lcap, &city, lcap, W::Medium, DIM, 1.6 * k);
    let wy = sep + 42.0 * k;
    let isz = 58.0 * k;
    let night = !(6..19).contains(&now.hour());
    weather_icon(p, x + pd - 3.0 * k, wy, isz, wt.icon, night);
    let ttx = x + pd + isz + 10.0 * k;
    let temp = format!("{:.0}\u{b0}", wt.temp_in(d.fahrenheit));
    let tcap2 = 46.0 * k;
    p.text(ttx, wy + tcap2 + 2.0 * k, &temp, tcap2, W::Regular, TEXT, 255);
    let cond = crate::tiles::nice_case(weather_icon::condition_name(wt.icon));
    let cond = match wt.humidity {
        Some(hm) => format!("{cond} \u{b7} {hm}%"),
        None => cond,
    };
    let ccap = p.fit(&cond, 17.0 * k, inner, W::Regular).max(13.0 * k);
    let cond = ellipsize(p, &cond, ccap, W::Regular, inner);
    p.text(x + pd, wy + isz + 12.0 * k + ccap, &cond, ccap, W::Regular, MUTED, 255);
    // the next days, anchored at the bottom
    let days: Vec<_> = wt.forecast.iter().skip(1).take(3).collect();
    if !days.is_empty() {
        let cw = inner / days.len() as f32;
        let tb = y + h - pd;
        let fcap = 17.0 * k;
        let lb = tb - fcap - 12.0 * k;
        for (i, f) in days.iter().enumerate() {
            let cx = x + pd + i as f32 * cw;
            let dn = i18n::weekday_short(f.weekday).to_uppercase();
            let dw = spaced(p, cx, lb, &dn, lcap, W::Medium, DIM, 1.2 * k);
            let is = 24.0 * k;
            weather_icon(p, cx + dw + 5.0 * k, lb - lcap / 2.0 - is / 2.0, is, f.icon, false);
            let hi = format!("{:.0}\u{b0}", f.temp_max_in(d.fahrenheit));
            let lo = format!("{:.0}\u{b0}", f.temp_min_in(d.fahrenheit));
            let hw = p.text(cx, tb, &hi, fcap, W::Medium, TEXT, 255);
            p.text(cx + hw + 3.0 * k, tb, &lo, fcap * 0.72, W::Regular, DIM, 255);
        }
    }
}

/// Header, subtitle and ring shared by the processor and graphics panels.
/// Returns (stats x, stats width, ring top, ring diameter).
fn big_panel_top(p: &mut Painter, r: R, k: f32, colors: [Rgb; 2], subtitle: &str, frac: Option<f32>, value: &str, unit: &str, hb: f32) -> (f32, f32, f32, f32) {
    let (x, _y, w, _h) = r;
    let pd = pad(k);
    let scap = 17.0 * k;
    let sub = ellipsize(p, subtitle, scap, W::Regular, w - 2.0 * pd);
    p.text(x + pd, hb + 12.0 * k + scap, &sub, scap, W::Regular, TEXT, 255);
    let rd = 184.0 * k;
    let ring_top = hb + 12.0 * k + scap + 16.0 * k;
    ring(p, x + pd + rd / 2.0, ring_top + rd / 2.0, rd, frac, colors, value, unit, "LOAD", k);
    let sx = x + pd + rd + 22.0 * k;
    (sx, x + w - pd - sx, ring_top, rd)
}

/// The row of three small stats under the ring, then a small history graph.
fn big_panel_bottom(p: &mut Painter, r: R, k: f32, below: f32, cells: [(&str, String, &str, Rgb); 3], hist: &VecDeque<f32>, max: f32, colors: [Rgb; 2]) {
    let (x, y, w, h) = r;
    let pd = pad(k);
    let top = below + 12.0 * k;
    let cw = ((w - 2.0 * pd) / 3.0).floor();
    let mut base = top;
    for (i, (label, value, unit, c)) in cells.iter().enumerate() {
        base = stat(p, x + pd + i as f32 * cw, top, cw, label, value, unit, *c, k, 31.0 * k);
    }
    let gtop = base + 14.0 * k;
    let bottom = y + h - pd;
    if bottom - gtop > 14.0 * k {
        sparkline(p, (x + pd, gtop, w - 2.0 * pd, bottom - gtop), hist, max, colors, k);
    }
}

fn draw_cpu(p: &mut Painter, r: R, k: f32, d: &Data, hist: &VecDeque<f32>) {
    panel(p, r, k);
    let hb = header(p, r, k, CPU_C[0], "PROCESSOR", "01");
    let subtitle = if d.cpu_fan_rpm.is_some() && d.cores.0 > 0 {
        format!("{}  \u{b7}  {}C / {}T", d.cpu_name, d.cores.0, d.cores.1)
    } else {
        d.cpu_name.to_string()
    };
    let pct = format!("{:.0}", d.cpu_pct);
    let (sx, sw, rt, rd) = big_panel_top(p, r, k, CPU_C, &subtitle, Some(d.cpu_pct / 100.0), &pct, "%", hb);
    let cw = (sw / 2.0).floor();
    let row = rd / 2.0;
    let vcap = 42.0 * k;
    let (tv, tc) = match d.cpu_temp {
        Some(t) => (format!("{t:.0}"), temp_color(t, 80.0, 90.0)),
        None => ("--".into(), DIM),
    };
    stat(p, sx, rt + 8.0 * k, cw, "TEMP", &tv, "\u{b0}C", tc, k, vcap);
    let pw = d.cpu_power.map_or("--".into(), |v| format!("{v:.0}"));
    stat(p, sx + cw, rt + 8.0 * k, cw, "POWER", &pw, "W", TEXT, k, vcap);
    let clk = d.cpu_mhz.map_or("--".into(), |m| format!("{:.2}", m as f32 / 1000.0));
    stat(p, sx, rt + row + 8.0 * k, cw, "CLOCK", &clk, "GHz", TEXT, k, vcap);
    match d.cpu_fan_rpm {
        Some(rpm) => stat(p, sx + cw, rt + row + 8.0 * k, cw, "FAN", &rpm.to_string(), "RPM", TEXT, k, vcap),
        None => {
            let cores = if d.cores.0 > 0 { format!("{}/{}", d.cores.0, d.cores.1) } else { d.cores.1.to_string() };
            stat(p, sx + cw, rt + row + 8.0 * k, cw, "CORES", &cores, "", TEXT, k, vcap)
        }
    };
    let (avg, peak) = if hist.is_empty() {
        (d.cpu_pct, d.cpu_pct)
    } else {
        (hist.iter().sum::<f32>() / hist.len() as f32, hist.iter().cloned().fold(0.0, f32::max))
    };
    let s = d.uptime_s;
    let (dd, hh, mm) = (s / 86_400, (s % 86_400) / 3600, (s % 3600) / 60);
    let up = if dd > 0 { format!("{dd}d {hh}h") } else { format!("{hh}h {mm:02}m") };
    big_panel_bottom(
        p,
        r,
        k,
        rt + rd,
        [("AVG 2M", format!("{avg:.0}"), "%", TEXT), ("PEAK 2M", format!("{peak:.0}"), "%", TEXT), ("UPTIME", up, "", TEXT)],
        hist,
        100.0,
        CPU_C,
    );
}

fn draw_gpu(p: &mut Painter, r: R, k: f32, d: &Data, hist: &VecDeque<f32>) {
    panel(p, r, k);
    let g = d.gpu;
    let fps = g.fps.filter(|f| *f > 0);
    let hb = header(p, r, k, GPU_C[0], "GRAPHICS", "02");
    let name = g.name.clone().unwrap_or_else(|| "GPU".into());
    let pct = d.gpu_pct.map_or("--".into(), |v| format!("{v:.0}"));
    let unit = if d.gpu_pct.is_some() { "%" } else { "" };
    let (sx, sw, rt, rd) = big_panel_top(p, r, k, GPU_C, &name, d.gpu_pct.map(|v| v / 100.0), &pct, unit, hb);
    let cw = (sw / 2.0).floor();
    let row = rd / 2.0;
    let vcap = 42.0 * k;
    let t = |v: Option<i32>, warn: f32, hot: f32| match v {
        Some(t) => (t.to_string(), temp_color(t as f32, warn, hot)),
        None => ("--".to_string(), DIM),
    };
    let (tv, tc) = t(g.temp_edge_c, 80.0, 90.0);
    stat(p, sx, rt + 8.0 * k, cw, "TEMP", &tv, "\u{b0}C", tc, k, vcap);
    let (hv, hc) = t(g.temp_hotspot_c, 95.0, 105.0);
    stat(p, sx + cw, rt + 8.0 * k, cw, "HOTSPOT", &hv, "\u{b0}C", hc, k, vcap);
    let pw = g.power_w.map_or("--".into(), |v| v.to_string());
    stat(p, sx, rt + row + 8.0 * k, cw, "POWER", &pw, "W", TEXT, k, vcap);
    let fan = g.fan_rpm.map_or("--".into(), |v| v.max(0).to_string());
    stat(p, sx + cw, rt + row + 8.0 * k, cw, "FAN", &fan, "RPM", TEXT, k, vcap);

    let clk = g.clock_mhz.filter(|c| *c > 0).map_or("--".into(), |v| v.to_string());
    let (vv, vu) = match (g.vram_used_mb, g.vram_total_mb) {
        (Some(u), Some(t)) if t > 0 => (format!("{:.1}", u as f32 / 1024.0), format!("/{:.0} GB", t as f32 / 1024.0)),
        (Some(u), _) => (format!("{:.1}", u as f32 / 1024.0), "GB".to_string()),
        _ => ("--".into(), String::new()),
    };
    let (fv, fu, fc) = match fps {
        Some(f) => (f.to_string(), format!("{:.1} ms", 1000.0 / f as f32), FPS_C),
        None => ("--".to_string(), String::new(), DIM),
    };
    big_panel_bottom(
        p,
        r,
        k,
        rt + rd,
        [("CLOCK", clk, "MHz", TEXT), ("VRAM", vv, vu.as_str(), TEXT), ("FPS", fv, fu.as_str(), fc)],
        hist,
        100.0,
        GPU_C,
    );
}

/// Small card: header, a ring on the right, a big value and two lines on the left.
fn card(p: &mut Painter, r: R, k: f32, colors: [Rgb; 2], title: &str, index: &str, frac: Option<f32>, ring_txt: &str, value: (&str, &str, Rgb), line1: &str, line2: &str) {
    panel(p, r, k);
    let (x, y, w, h) = r;
    let pd = pad(k);
    let hb = header(p, r, k, colors[0], title, index);
    let body_top = hb + 10.0 * k;
    let body_h = y + h - pd - body_top;
    let rd = body_h.min(w * 0.38).max(30.0);
    let cx = x + w - pd - rd / 2.0;
    let cy = body_top + body_h / 2.0;
    ring(p, cx, cy, rd, frac, colors, ring_txt, if frac.is_some() { "%" } else { "" }, "", k);
    let lw = cx - rd / 2.0 - 12.0 * k - (x + pd);
    let vcap = fit_vu(p, value.0, value.1, 50.0 * k, lw, W::Medium);
    let l1 = p.fit(line1, 17.0 * k, lw, W::Regular).max(12.0 * k);
    let l2 = if line2.is_empty() { 0.0 } else { p.fit(line2, 15.0 * k, lw, W::Regular).max(10.5 * k) };
    let block = vcap + 14.0 * k + l1 + if line2.is_empty() { 0.0 } else { 10.0 * k + l2 };
    let vb = cy - block / 2.0 + vcap;
    value_unit(p, x + pd, vb, value.0, value.1, vcap, W::Medium, value.2);
    let s1 = ellipsize(p, line1, l1, W::Regular, lw);
    p.text(x + pd, vb + 14.0 * k + l1, &s1, l1, W::Regular, MUTED, 255);
    if !line2.is_empty() {
        let s2 = ellipsize(p, line2, l2, W::Regular, lw);
        p.text(x + pd, vb + 14.0 * k + l1 + 10.0 * k + l2, &s2, l2, W::Regular, DIM, 255);
    }
}

fn draw_ram(p: &mut Painter, r: R, k: f32, d: &Data) {
    let frac = if d.ram_total_mb > 0 { d.ram_used_mb as f32 / d.ram_total_mb as f32 } else { 0.0 };
    let pct = format!("{:.0}", frac * 100.0);
    let used = gb(d.ram_used_mb);
    let total = format!("of {} GB", gb(d.ram_total_mb));
    let free = format!("{} GB free", gb(d.ram_total_mb.saturating_sub(d.ram_used_mb)));
    card(p, r, k, RAM_C, "MEMORY", "03", Some(frac), &pct, (used.as_str(), "GB", TEXT), &total, &free);
}

fn draw_disk(p: &mut Painter, r: R, k: f32, d: &Data) {
    let (rd, wr) = d.disk_rate;
    let io = format!("R/W {:.1}/{:.1} MB/s", rd, wr);
    match d.disk {
        Some(di) => {
            let pct = format!("{:.0}", di.used_frac() * 100.0);
            let (tv, tc) = match di.temp_c {
                Some(t) => (t.to_string(), temp_color(t as f32, 60.0, 70.0)),
                None => ("--".to_string(), DIM),
            };
            let space = format!("{} {:.0}/{:.0} GB", di.label, di.used_gb, di.total_gb);
            card(p, r, k, SSD_C, "STORAGE", "04", Some(di.used_frac()), &pct, (tv.as_str(), "\u{b0}C", tc), &space, &io);
        }
        None => card(p, r, k, SSD_C, "STORAGE", "04", None, "--", ("--", "", DIM), &io, ""),
    }
}

fn draw_media(p: &mut Painter, r: R, k: f32, d: &Data, cover: Option<&Cover>) {
    panel(p, r, k);
    let (x, y, w, h) = r;
    let pd = pad(k);
    let accent = cover.map_or((200, 204, 214), |c| c.accent);
    let hb = header(p, r, k, accent, "NOW PLAYING", "05");
    let Some(t) = d.track else {
        // idle: a note and a line of text
        let cx = x + w / 2.0;
        let cy = hb + (y + h - hb) / 2.0 - 10.0 * k;
        music_note(p, cx, cy - 8.0 * k, 34.0 * k, DIM);
        let s = "Nothing playing";
        let cap = 17.0 * k;
        let sw = p.width(s, cap, W::Regular);
        p.text(cx - sw / 2.0, cy + 40.0 * k, s, cap, W::Regular, DIM, 255);
        return;
    };
    let body_top = hb + 14.0 * k;
    let bar_zone = 42.0 * k;
    let cs = (y + h - pd - bar_zone - body_top).min(w * 0.40).max(24.0).floor();
    let (cx0, cy0) = (x + pd, body_top);
    match cover {
        Some(c) => blit_cover(p, c, cx0, cy0, cs, 12.0 * k),
        None => {
            p.layer.fill_rrect((cx0, cy0, cs, cs), 12.0 * k, &|t| lerp_rgb((40, 42, 52), (24, 25, 32), t), 255);
            music_note(p, cx0 + cs / 2.0, cy0 + cs / 2.0, cs * 0.32, MUTED);
        }
    }
    // title (2 lines max) + artist
    let tx = cx0 + cs + 14.0 * k;
    let tw = x + w - pd - tx;
    let tcap = 20.0 * k;
    let lines = wrap2(p, &t.title, tcap, W::Medium, tw);
    let mut base = cy0 + tcap + 2.0 * k;
    for l in &lines {
        p.text(tx, base, l, tcap, W::Medium, TEXT, 255);
        base += tcap + 10.0 * k;
    }
    let acap = 17.0 * k;
    let artist = ellipsize(p, &t.artist, acap, W::Regular, tw);
    p.text(tx, base + 4.0 * k, &artist, acap, W::Regular, MUTED, 255);
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
        let cap = 15.0 * k;
        let l = mmss(pos / 1000);
        let rr = mmss(tl.duration_ms / 1000);
        p.text(x + pd, by - 11.0 * k, &l, cap, W::Medium, MUTED, 255);
        let rw = p.width(&rr, cap, W::Medium);
        p.text(x + w - pd - rw, by - 11.0 * k, &rr, cap, W::Medium, DIM, 255);
    } else {
        p.layer.fill_rrect((x + pd, by, bw, bh), bh / 2.0, &|_| WHITE, 20);
    }
}

fn draw_net(p: &mut Painter, r: R, k: f32, d: &Data, hist: &VecDeque<f32>) {
    panel(p, r, k);
    let (x, y, w, h) = r;
    let pd = pad(k);
    let hb = header(p, r, k, NET_C[0], "NETWORK", "06");
    let peak = hist.iter().cloned().fold(64.0f32, f32::max) * 1.15;
    let half = (w - 2.0 * pd) / 2.0;
    let vcap = 40.0 * k;
    let vb = hb + 16.0 * k + vcap;
    for (i, (kb, c, down)) in [(d.net_kb.0, NET_C[1], true), (d.net_kb.1, UP_C, false)].into_iter().enumerate() {
        let cx = x + pd + i as f32 * half;
        arrow(p, cx + 8.0 * k, vb - vcap / 2.0, 9.0 * k, down, c);
        let (v, u) = rate(kb);
        let vc = fit_vu(p, &v, u, vcap, half - 30.0 * k, W::Medium);
        value_unit(p, cx + 24.0 * k, vb, &v, u, vc, W::Medium, TEXT);
    }
    let st = vb + 14.0 * k;
    let bottom = y + h - pd;
    if bottom - st > 10.0 * k {
        sparkline(p, (x + pd, st, w - 2.0 * pd, bottom - st), hist, peak, NET_C, k);
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
    circle(p, cx + r * 0.45, cy - r * 0.35, r * 0.85, PANEL, 255);
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
