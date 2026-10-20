//! Ten more designs for the `pro` screen. Same information as the first three (clock,
//! weather, CPU, GPU, memory, storage, network, now playing), dressed in very
//! different styles:
//!
//! * landscape (the screen as mounted): `bento`, `terminal`, `chamfer`, `rings`, `dots`
//! * portrait (screen mounted vertically, drawn upright on a 462 x 1920 canvas and turned
//!   90 degrees): `lock`, `equalizer`, `receipt`, `synthwave`, `blueprint`
//!
//! Everything is painted on the same transparent [`Layer`] as the other layouts, so the
//! cost is the same (drawn once a second at most).

use super::*;

// ------------------------------------------------------------- view model

/// One labelled reading.
pub(super) struct Item {
    pub label: &'static str,
    pub value: String,
    pub unit: String,
    pub color: Rgb,
    /// 0..1 when the reading has a natural full scale.
    pub frac: Option<f32>,
}

fn item(label: &'static str, value: impl Into<String>, unit: impl Into<String>, color: Rgb, frac: Option<f32>) -> Item {
    Item { label, value: value.into(), unit: unit.into(), color, frac }
}

pub(super) struct DayFc {
    pub name: String,
    pub icon: WeatherIcon,
    pub hi: String,
    pub lo: String,
}

pub(super) struct Wx {
    pub icon: WeatherIcon,
    pub temp: String,
    pub cond: String,
    pub city: String,
    pub night: bool,
    pub days: Vec<DayFc>,
}

/// Every number of the screen, already formatted.
pub(super) struct Vm {
    pub cpu_name: String,
    pub cores: String,
    pub cpu_temp: String,
    pub cpu_temp_color: Rgb,
    pub cpu_temp_f: Option<f32>,
    pub cpu_load: String,
    pub cpu_frac: f32,
    /// LOAD, POWER, CLOCK, FAN, UPTIME, DOWN, UP
    pub cpu: Vec<Item>,
    pub gpu_name: String,
    pub gpu_temp: String,
    pub gpu_temp_color: Rgb,
    pub gpu_temp_f: Option<f32>,
    pub gpu_load: String,
    pub gpu_frac: Option<f32>,
    /// HOTSPOT, LOAD, POWER, FAN, CLOCK, VRAM, FPS
    pub gpu: Vec<Item>,
    pub vram_frac: Option<f32>,
    pub ram_pct: String,
    pub ram_frac: f32,
    pub ram_total: String,
    /// USED, FREE, SSD, DISK, READ, WRITE
    pub mem: Vec<Item>,
    pub disk_frac: Option<f32>,
    pub ssd_temp_f: Option<f32>,
    pub hhmm: String,
    pub secs: u32,
    pub wd: String,
    pub date: String,
    pub wx: Option<Wx>,
    pub net_dn: (String, &'static str),
    pub net_up: (String, &'static str),
}

pub(super) fn vm(d: &Data, now: &chrono::DateTime<Local>) -> Vm {
    let g = d.gpu;
    let dash = || "--".to_string();
    let (cpu_temp, cpu_temp_color) = tval(d.cpu_temp, 80.0, 90.0);
    let (gpu_temp, gpu_temp_color) = tval(g.temp_edge_c.map(|v| v as f32), 80.0, 90.0);
    let (hot, hot_c) = tval(g.temp_hotspot_c.map(|v| v as f32), 95.0, 105.0);
    let (uv, uu) = uptime_str(d.uptime_s);
    let (dn, up) = (rate(d.net_kb.0), rate(d.net_kb.1));
    let fan_cpu = match d.cpu_fan_rpm {
        Some(r) => item("FAN", r.to_string(), "RPM", th().text, None),
        None => item("FAN", dash(), "", th().dim, None),
    };
    let cpu = vec![
        item("LOAD", format!("{:.0}", d.cpu_pct), "%", th().text, Some(d.cpu_pct / 100.0)),
        item("POWER", d.cpu_power.map_or(dash(), |v| format!("{v:.0}")), "W", th().text, None),
        item("CLOCK", d.cpu_mhz.map_or(dash(), |m| format!("{:.2}", m as f32 / 1000.0)), "GHz", th().text, None),
        fan_cpu,
        item("UPTIME", uv, uu, th().text, None),
        item("DOWN", dn.0.clone(), dn.1, th().text, None),
        item("UP", up.0.clone(), up.1, th().text, None),
    ];
    let (vv, _vu, vf) = vram_text(g);
    let fps = g.fps.filter(|f| *f > 0);
    let gpu = vec![
        item("HOTSPOT", hot, "\u{b0}C", hot_c, None),
        item("LOAD", d.gpu_pct.map_or(dash(), |v| format!("{v:.0}")), if d.gpu_pct.is_some() { "%" } else { "" }, th().text, d.gpu_pct.map(|v| v / 100.0)),
        item("POWER", g.power_w.map_or(dash(), |v| v.to_string()), "W", th().text, None),
        item("FAN", g.fan_rpm.map_or(dash(), |v| v.max(0).to_string()), "RPM", th().text, None),
        item("CLOCK", g.clock_mhz.filter(|c| *c > 0).map_or(dash(), |v| v.to_string()), "MHz", th().text, None),
        item("VRAM", vv, "GB", th().text, vf),
        match fps {
            Some(f) => item("FPS", f.to_string(), format!("{:.1}ms", 1000.0 / f as f32), th().fps, None),
            None => item("FPS", dash(), "", th().dim, None),
        },
    ];
    let ram_frac = if d.ram_total_mb > 0 { d.ram_used_mb as f32 / d.ram_total_mb as f32 } else { 0.0 };
    let (rv, ru) = rate(d.disk_rate.0 * 1024.0);
    let (wv, wu) = rate(d.disk_rate.1 * 1024.0);
    let ssd = d.disk.and_then(|x| x.temp_c);
    let mem = vec![
        item("USED", gb(d.ram_used_mb), "GB", th().text, Some(ram_frac)),
        item("FREE", gb(d.ram_total_mb.saturating_sub(d.ram_used_mb)), "GB", th().text, None),
        item("SSD", ssd.map_or(dash(), |t| t.to_string()), "\u{b0}C", ssd.map_or(th().dim, |t| temp_color(t as f32, 60.0, 70.0)), None),
        item("DISK", d.disk.map_or(dash(), |x| format!("{:.0}", x.used_frac() * 100.0)), "%", th().text, d.disk.map(|x| x.used_frac())),
        item("READ", rv, ru, th().text, None),
        item("WRITE", wv, wu, th().text, None),
    ];
    let (hhmm, wd, date) = clock_parts(now);
    let wx = d.weather.map(|wt| Wx {
        icon: wt.icon,
        temp: format!("{:.0}\u{b0}", wt.temp_in(d.fahrenheit).round() + 0.0),
        cond: crate::tiles::nice_case(weather_icon::condition_name(wt.icon)),
        city: wt.city.clone().unwrap_or_default(),
        night: !(6..19).contains(&now.hour()),
        days: wt
            .forecast
            .iter()
            .skip(1)
            .take(3)
            .map(|f| DayFc {
                name: i18n::weekday_short(f.weekday).to_uppercase(),
                icon: f.icon,
                hi: format!("{:.0}\u{b0}", f.temp_max_in(d.fahrenheit).round() + 0.0),
                lo: format!("{:.0}\u{b0}", f.temp_min_in(d.fahrenheit).round() + 0.0),
            })
            .collect(),
    });
    Vm {
        cpu_name: d.cpu_name.to_string(),
        cores: format!("{}C / {}T", d.cores.0, d.cores.1),
        cpu_temp,
        cpu_temp_color,
        cpu_temp_f: d.cpu_temp.map(|t| (t / 100.0).clamp(0.0, 1.0)),
        cpu_load: format!("{:.0}", d.cpu_pct),
        cpu_frac: (d.cpu_pct / 100.0).clamp(0.0, 1.0),
        cpu,
        gpu_name: g.name.clone().unwrap_or_else(|| "GPU".into()),
        gpu_temp,
        gpu_temp_color,
        gpu_temp_f: g.temp_edge_c.map(|t| (t as f32 / 100.0).clamp(0.0, 1.0)),
        gpu_load: d.gpu_pct.map_or(dash(), |v| format!("{v:.0}")),
        gpu_frac: d.gpu_pct.map(|v| (v / 100.0).clamp(0.0, 1.0)),
        gpu,
        vram_frac: vf,
        ram_pct: format!("{:.0}", ram_frac * 100.0),
        ram_frac,
        ram_total: gb(d.ram_total_mb),
        mem,
        disk_frac: d.disk.map(|x| x.used_frac()),
        ssd_temp_f: ssd.map(|t| (t as f32 / 100.0).clamp(0.0, 1.0)),
        hhmm,
        secs: now.second(),
        wd,
        date,
        wx,
        net_dn: dn,
        net_up: up,
    }
}

// ------------------------------------------------------------ small helpers

/// Text whose right edge is at `rx`.
fn text_r(p: &mut Painter, rx: f32, base: f32, text: &str, cap: f32, wt: W, c: Rgb) -> f32 {
    let w = p.width(text, cap, wt);
    p.text(rx - w, base, text, cap, wt, c, 255);
    w
}

/// Text centered on `cx`.
fn text_c(p: &mut Painter, cx: f32, base: f32, text: &str, cap: f32, wt: W, c: Rgb) -> f32 {
    let w = p.width(text, cap, wt);
    p.text(cx - w / 2.0, base, text, cap, wt, c, 255);
    w
}

/// Filled polygon (any simple outline), anti-aliased with 4 sub-scanlines per row.
fn poly(p: &mut Painter, pts: &[(f32, f32)], c: Rgb, alpha: u8) {
    if pts.len() < 3 || alpha == 0 {
        return;
    }
    let (mut y0, mut y1, mut x0, mut x1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for &(x, y) in pts {
        y0 = y0.min(y);
        y1 = y1.max(y);
        x0 = x0.min(x);
        x1 = x1.max(x);
    }
    let (xi0, xi1) = (x0.floor() as i32, x1.ceil() as i32);
    let bw = (xi1 - xi0).max(1) as usize;
    let mut cov = vec![0f32; bw];
    const SUB: usize = 4;
    for py in (y0.floor() as i32)..(y1.ceil() as i32) {
        cov.iter_mut().for_each(|v| *v = 0.0);
        let mut any = false;
        for s in 0..SUB {
            let sy = py as f32 + (s as f32 + 0.5) / SUB as f32;
            let mut xs: Vec<f32> = Vec::new();
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                if (a.1 <= sy && b.1 > sy) || (b.1 <= sy && a.1 > sy) {
                    xs.push(a.0 + (sy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(|a, b| a.total_cmp(b));
            for pair in xs.chunks_exact(2) {
                let (xa, xb) = (pair[0] - xi0 as f32, pair[1] - xi0 as f32);
                let (ia, ib) = (xa.floor().max(0.0) as usize, (xb.ceil() as usize).min(bw));
                for ix in ia..ib {
                    let o = ((ix as f32 + 1.0).min(xb) - (ix as f32).max(xa)).max(0.0);
                    cov[ix] += o / SUB as f32;
                    any = true;
                }
            }
        }
        if any {
            for (ix, &cv) in cov.iter().enumerate() {
                if cv > 0.003 {
                    p.layer.over(xi0 + ix as i32, py, c, (alpha as f32 * cv.min(1.0)).round() as u8);
                }
            }
        }
    }
}

fn poly_outline(p: &mut Painter, pts: &[(f32, f32)], r: f32, c: Rgb, alpha: u8) {
    for i in 0..pts.len() {
        capsule(p, pts[i], pts[(i + 1) % pts.len()], r, c, alpha);
    }
}

/// Text slanted like italics (`shear` = horizontal shift per pixel of height).
fn skew_text(p: &mut Painter, x: f32, base: f32, text: &str, cap: f32, wt: W, c: Rgb, shear: f32) -> f32 {
    let px = crate::smooth_font::px_for_cap(cap);
    let layer = &mut *p.layer;
    let by = base.round();
    crate::smooth_font::draw_with(text, x, base, px, sfw(wt), |gx, gy, cov| {
        let sx = gx as f32 + (by - gy as f32) * shear;
        layer.over(sx.round() as i32, gy, c, cov);
    })
}

/// Text with a soft halo (cheap glow: the same text a pixel around, faint).
fn glow_text(p: &mut Painter, x: f32, base: f32, text: &str, cap: f32, wt: W, c: Rgb, halo: u8) -> f32 {
    if halo > 0 {
        for (dx, dy) in [(-2.0, 0.0), (2.0, 0.0), (0.0, -2.0), (0.0, 2.0), (-1.5, -1.5), (1.5, 1.5), (-1.5, 1.5), (1.5, -1.5)] {
            p.text(x + dx, base + dy, text, cap, wt, c, halo);
        }
    }
    p.text(x, base, text, cap, wt, c, 255)
}

/// Rectangle filled with an even color (not rounded), any float position.
fn rect(p: &mut Painter, r: R, c: Rgb, a: u8) {
    p.layer.fill_rrect(r, 0.0, &move |_| c, a);
}

/// Segmented bar: `n` blocks, `frac` of them lit, colored by `col(i / n)`.
fn seg_h(p: &mut Painter, x: f32, y: f32, w: f32, h: f32, n: usize, frac: f32, gap: f32, col: &dyn Fn(f32) -> Rgb) {
    let bw = (w - gap * (n as f32 - 1.0)) / n as f32;
    let lit = (frac.clamp(0.0, 1.0) * n as f32).round() as usize;
    for i in 0..n {
        let bx = x + i as f32 * (bw + gap);
        if i < lit {
            rect(p, (bx, y, bw, h), col(i as f32 / n as f32), 255);
        } else {
            rect(p, (bx, y, bw, h), WHITE, 22);
        }
    }
}

fn weather_days_row(p: &mut Painter, sc: &Scale, x: f32, base: f32, w: f32, days: &[DayFc], name_c: Rgb, hi_c: Rgb, lo_c: Rgb) {
    if days.is_empty() {
        return;
    }
    let k = sc.k;
    let cw = w / days.len() as f32;
    for (i, f) in days.iter().enumerate() {
        let cx = x + i as f32 * cw;
        let dw = spaced(p, cx, base - sc.m - 8.0 * k, &f.name, sc.s, W::Medium, name_c, 1.2 * k);
        weather_icon(p, cx + dw + 6.0 * k, base - sc.m - 8.0 * k - sc.s / 2.0 - 12.0 * k, 24.0 * k, f.icon, false);
        let hw = p.text(cx, base, &f.hi, sc.m, W::Medium, hi_c, 255);
        p.text(cx + hw + 4.0 * k, base, &f.lo, sc.s, W::Regular, lo_c, 255);
    }
}

/// Elapsed / total, "1:23 / 3:45", and the fraction played.
fn track_times(d: &Data) -> Option<(String, String, f32)> {
    let tl = d.track?.timeline.filter(|tl| tl.duration_ms > 0)?;
    let pos = tl.position_now_ms().min(tl.duration_ms);
    Some((mmss(pos / 1000), mmss(tl.duration_ms / 1000), pos as f32 / tl.duration_ms as f32))
}

fn track_lines(d: &Data) -> Option<(String, String)> {
    let t = d.track?;
    let artist = if t.artist.trim().is_empty() { "Unknown artist".to_string() } else { t.artist.clone() };
    Some((clean_title(&t.title), artist))
}

fn cover_or_note(p: &mut Painter, cover: Option<&Cover>, x: f32, y: f32, s: f32, radius: f32, note: Rgb, bg: (Rgb, Rgb)) {
    match cover {
        Some(c) => blit_cover(p, c, x, y, s, radius),
        None => {
            p.layer.fill_rrect((x, y, s, s), radius, &move |t| lerp_rgb(bg.0, bg.1, t), 255);
            music_note(p, x + s / 2.0, y + s / 2.0, s * 0.34, note);
        }
    }
}

pub(super) fn paint(layout: Layout, p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, now: &chrono::DateTime<Local>, hist: (&VecDeque<f32>, &VecDeque<f32>, &VecDeque<f32>), cover: Option<&Cover>) {
    let v = vm(d, now);
    match layout {
        Layout::Bento => bento(p, w, h, sc, d, &v, hist, cover),
        Layout::Terminal => terminal(p, w, h, sc, d, &v, cover),
        Layout::Chamfer => chamfer(p, w, h, sc, d, &v, cover),
        Layout::Rings => rings(p, w, h, sc, d, &v, cover),
        Layout::Dots => dots(p, w, h, sc, d, &v, cover),
        Layout::Lock => lock(p, w, h, sc, d, &v, cover),
        Layout::Equalizer => equalizer(p, w, h, sc, d, &v, cover),
        Layout::Receipt => receipt(p, w, h, sc, d, &v, cover),
        Layout::Synthwave => synthwave(p, w, h, sc, d, &v, cover),
        Layout::Blueprint => blueprint(p, w, h, sc, d, &v, cover),
        _ => {}
    }
}

// ------------------------------------------------------------------- BENTO
//
// Solid, saturated rounded tiles of different sizes on black (iOS widgets / Material You).
// Every tile takes its color from the theme and carries dark ink.

const INK: Rgb = (12, 12, 16);

fn tint(c: [Rgb; 2]) -> Rgb {
    lerp_rgb(c[0], c[1], 0.55)
}

fn on(tile: Rgb, f: f32) -> Rgb {
    lerp_rgb(tile, INK, f)
}

fn bento_stat(p: &mut Painter, sc: &Scale, tile: Rgb, x: f32, y: f32, w: f32, it: &Item, vcap: f32) {
    let k = sc.k;
    spaced(p, x, y + sc.s, it.label, sc.s, W::Medium, on(tile, 0.62), 1.6 * k);
    let cap = fit_vu(p, &it.value, &it.unit, vcap, w - 4.0 * k, W::Medium);
    let uw = value_unit_width(p, &it.value, &it.unit, cap, W::Medium);
    let _ = uw;
    let vw = p.text(x, y + sc.s + 8.0 * k + vcap, &it.value, cap, W::Medium, INK, 255);
    if !it.unit.is_empty() {
        p.text(x + vw + (cap * 0.12).max(2.0), y + sc.s + 8.0 * k + vcap, &it.unit, (cap * 0.40).max(7.0), W::Regular, on(tile, 0.66), 255);
    }
}

fn bento(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: (&VecDeque<f32>, &VecDeque<f32>, &VecDeque<f32>), cover: Option<&Cover>) {
    let k = sc.k;
    let m = 12.0 * k;
    let g = 10.0 * k;
    let rad = 28.0 * k;
    let weights = [300.0f32, 432.0, 452.0, 330.0, 356.0];
    let avail = w - 2.0 * m - 4.0 * g;
    let sum: f32 = weights.iter().sum();
    let mut cols = Vec::new();
    let mut x = m;
    for wt in weights {
        let cw = (avail * wt / sum).floor();
        cols.push((x, cw));
        x += cw + g;
    }
    let ph = h - 2.0 * m;
    let pd = 22.0 * k;
    let tile = |p: &mut Painter, r: R, c: Rgb| p.layer.fill_rrect_solid((r.0.round(), r.1.round(), r.2.round(), r.3.round()), rad, c, 255);

    // --- clock + weather (white tile)
    {
        let r = (cols[0].0, m, cols[0].1, ph);
        let c = th().text;
        tile(p, r, c);
        let (x, y, tw, th_) = r;
        let inner = tw - 2.0 * pd;
        spaced(p, x + pd, y + pd + sc.s, &v.wd, sc.s, W::Medium, on(c, 0.55), 2.0 * k);
        let date = ellipsize(p, &v.date, sc.m, W::Regular, inner);
        p.text(x + pd, y + pd + sc.s + 12.0 * k + sc.m, &date, sc.m, W::Regular, on(c, 0.75), 255);
        let tcap = p.fit(&v.hhmm, 96.0 * k, inner, W::Medium);
        let tb = y + pd + sc.s + 12.0 * k + sc.m + 22.0 * k + tcap;
        p.text(x + pd - 3.0 * k, tb, &v.hhmm, tcap, W::Medium, INK, 255);
        // seconds
        let sy = tb + 16.0 * k;
        p.layer.fill_rrect((x + pd, sy, inner, 5.0 * k), 2.5 * k, &|_| INK, 36);
        let sw = (inner * v.secs as f32 / 59.0).max(5.0 * k);
        p.layer.fill_rrect((x + pd, sy, sw, 5.0 * k), 2.5 * k, &|_| INK, 255);
        if let Some(wx) = &v.wx {
            // weather: a dark card inside the white tile
            let cy0 = sy + 26.0 * k;
            let card = (x + 10.0 * k, cy0, tw - 20.0 * k, y + th_ - 10.0 * k - cy0);
            p.layer.fill_rrect_solid((card.0.round(), card.1.round(), card.2.round(), card.3.round()), 20.0 * k, INK, 255);
            let (cx0, cw0) = (card.0 + 14.0 * k, card.2 - 28.0 * k);
            let is = 52.0 * k;
            weather_icon(p, cx0 - 4.0 * k, cy0 + 8.0 * k, is, wx.icon, wx.night);
            p.text(cx0 + is + 6.0 * k, cy0 + 8.0 * k + is / 2.0 + sc.l / 2.0, &wx.temp, sc.l, W::Medium, WHITE, 255);
            let line = if wx.city.is_empty() { wx.cond.clone() } else { format!("{} \u{b7} {}", wx.cond, wx.city) };
            let line = ellipsize(p, &line, sc.m, W::Regular, cw0);
            p.text(cx0, cy0 + 8.0 * k + is + 8.0 * k + sc.m, &line, sc.m, W::Regular, (190, 194, 206), 255);
            // forecast: one row per day
            let n = wx.days.len().max(1) as f32;
            let fy0 = cy0 + 8.0 * k + is + 8.0 * k + sc.m + 14.0 * k;
            let rowh = ((card.1 + card.3 - 10.0 * k) - fy0) / n;
            capsule(p, (cx0, fy0 - 4.0 * k), (cx0 + cw0, fy0 - 4.0 * k), 0.5 * k, WHITE, 40);
            for (j, f) in wx.days.iter().enumerate() {
                let by = fy0 + j as f32 * rowh + rowh * 0.66;
                spaced(p, cx0, by, &f.name, sc.s, W::Medium, (140, 144, 158), 1.2 * k);
                weather_icon(p, cx0 + 56.0 * k, by - 20.0 * k, 26.0 * k, f.icon, false);
                let hw = p.text(cx0 + 96.0 * k, by, &f.hi, sc.m * 1.1, W::Medium, WHITE, 255);
                p.text(cx0 + 96.0 * k + hw + 6.0 * k, by, &f.lo, sc.m, W::Regular, (140, 144, 158), 255);
            }
        }
    }

    // --- CPU and GPU tiles
    for (i, (colors, label, name, temp, tcol, side)) in [
        (th().cpu, "CPU", v.cpu_name.as_str(), v.cpu_temp.as_str(), v.cpu_temp_color, String::new()),
        (th().gpu, "GPU", v.gpu_name.as_str(), v.gpu_temp.as_str(), v.gpu_temp_color, String::new()),
    ]
    .into_iter()
    .enumerate()
    {
        let r = (cols[1 + i].0, m, cols[1 + i].1, ph);
        let c = tint(colors);
        tile(p, r, c);
        let _ = (tcol, side);
        let (x, y, tw, _) = r;
        let inner = tw - 2.0 * pd;
        // header
        spaced(p, x + pd, y + pd + sc.s, label, sc.s * 1.1, W::Medium, INK, 2.4 * k);
        let nm = ellipsize(p, name, sc.s, W::Regular, inner * 0.62);
        text_r(p, x + tw - pd, y + pd + sc.s, &nm, sc.s, W::Regular, on(c, 0.62));
        // hero temperature
        let hcap = 132.0 * k;
        let hb = y + pd + sc.s + 14.0 * k + hcap;
        let hw = p.text(x + pd - 6.0 * k, hb, temp, hcap, W::Medium, INK, 255);
        p.text(x + pd - 6.0 * k + hw + 6.0 * k, hb - hcap + sc.l, "\u{b0}C", sc.l, W::Regular, on(c, 0.55), 255);
        // load bar next to the hero number
        let (frac, hq) = if i == 0 { (v.cpu_frac, hist.0) } else { (v.gpu_frac.unwrap_or(0.0), hist.1) };
        let bx = x + pd + (hw + 56.0 * k).min(inner * 0.62);
        let bw = x + tw - pd - bx;
        if bw > 40.0 * k {
            // tiny history inside the free corner, ink on the tile
            let gh = 50.0 * k;
            let gy = hb - gh;
            let n = hq.len();
            if n > 1 {
                let step = bw / (HIST as f32 - 1.0);
                let x0 = bx + bw - (n as f32 - 1.0) * step;
                for j in 1..n {
                    let (a, b) = (hq[j - 1], hq[j]);
                    capsule(p, (x0 + (j as f32 - 1.0) * step, gy + gh - gh * (a / 100.0).clamp(0.0, 1.0)), (x0 + j as f32 * step, gy + gh - gh * (b / 100.0).clamp(0.0, 1.0)), 1.6 * k, INK, 230);
                }
            }
            let _ = frac;
        }
        // stats grid: 3 x 2
        let items: Vec<&Item> = if i == 0 {
            vec![&v.cpu[0], &v.cpu[1], &v.cpu[2], &v.cpu[3], &v.cpu[4]]
        } else {
            vec![&v.gpu[1], &v.gpu[2], &v.gpu[3], &v.gpu[4], &v.gpu[5], &v.gpu[6]]
        };
        let gy = hb + 26.0 * k;
        let cw = inner / 3.0;
        let rows = 2usize;
        let row_h = sc.s + 8.0 * k + 36.0 * k;
        let pitch = ((y + ph) - pd - gy - row_h) / (rows as f32 - 1.0);
        for (j, it) in items.iter().enumerate() {
            bento_stat(p, sc, c, x + pd + (j % 3) as f32 * cw, gy + (j / 3) as f32 * pitch, cw - 8.0 * k, it, 36.0 * k);
        }
        if i == 0 {
            // sixth cell: cores
            let j = 5;
            let it = item("CORES", v.cores.replace(" / ", "/"), "", INK, None);
            bento_stat(p, sc, c, x + pd + (j % 3) as f32 * cw, gy + (j / 3) as f32 * pitch, cw - 8.0 * k, &it, 36.0 * k);
        }
        if i == 1 {
            // hotspot sits at the top right of the hero number
            let hs = &v.gpu[0];
            let t = format!("hotspot {}\u{b0}", hs.value);
            text_r(p, x + tw - pd, hb - 66.0 * k, &t, sc.m, W::Regular, on(c, 0.66));
        }
    }

    // --- memory + storage
    {
        let (x, cw) = cols[3];
        let top_h = (ph * 0.56 - g / 2.0).floor();
        let r1 = (x, m, cw, top_h);
        let c = tint(th().ram);
        tile(p, r1, c);
        let inner = cw - 2.0 * pd;
        spaced(p, x + pd, m + pd + sc.s, "MEMORY", sc.s * 1.1, W::Medium, INK, 2.4 * k);
        text_r(p, x + cw - pd, m + pd + sc.s, &format!("of {} GB", v.ram_total), sc.s, W::Regular, on(c, 0.62));
        let hcap = 78.0 * k;
        let hb = m + pd + sc.s + 10.0 * k + hcap;
        let hw = p.text(x + pd - 4.0 * k, hb, &v.ram_pct, hcap, W::Medium, INK, 255);
        p.text(x + pd + hw, hb - hcap + sc.l * 0.9, "%", sc.l * 0.9, W::Regular, on(c, 0.55), 255);
        let by = hb + 14.0 * k;
        p.layer.fill_rrect((x + pd, by, inner, 8.0 * k), 4.0 * k, &|_| INK, 34);
        p.layer.fill_rrect((x + pd, by, (inner * v.ram_frac).max(8.0 * k), 8.0 * k), 4.0 * k, &|_| INK, 255);
        let sy = by + 16.0 * k;
        for (j, it) in v.mem[0..2].iter().enumerate() {
            bento_stat(p, sc, c, x + pd + j as f32 * inner / 2.0, sy, inner / 2.0 - 6.0 * k, it, 30.0 * k);
        }
        let r2 = (x, m + top_h + g, cw, ph - top_h - g);
        let c2 = tint(th().ssd);
        tile(p, r2, c2);
        let y2 = r2.1;
        spaced(p, x + pd, y2 + pd + sc.s, "STORAGE", sc.s * 1.1, W::Medium, INK, 2.4 * k);
        let bw = inner / 2.0 - 6.0 * k;
        let order = [&v.mem[2], &v.mem[3], &v.mem[4], &v.mem[5]];
        let top = y2 + pd + sc.s + 12.0 * k;
        let pitch = (r2.3 - (top - y2) - pd - (sc.s + 8.0 * k + 30.0 * k)).max(0.0);
        for (j, it) in order.iter().enumerate() {
            bento_stat(p, sc, c2, x + pd + (j % 2) as f32 * inner / 2.0, top + (j / 2) as f32 * pitch, bw, it, 30.0 * k);
        }
    }

    // --- now playing + network
    {
        let (x, cw) = cols[4];
        let top_h = (ph * 0.58 - g / 2.0).floor();
        let r1 = (x, m, cw, top_h);
        let c = cover.map_or(lerp_rgb(th().muted, WHITE, 0.2), |c| lerp_rgb(c.accent, WHITE, 0.15));
        tile(p, r1, c);
        let inner = cw - 2.0 * pd;
        spaced(p, x + pd, m + pd + sc.s, "NOW PLAYING", sc.s * 1.1, W::Medium, INK, 2.4 * k);
        match track_lines(d) {
            None => {
                music_note(p, x + cw / 2.0, m + top_h / 2.0 + 6.0 * k, 44.0 * k, on(c, 0.5));
                text_c(p, x + cw / 2.0, m + top_h / 2.0 + 56.0 * k, "Nothing playing", sc.m, W::Regular, on(c, 0.62));
            }
            Some((title, artist)) => {
                let cs = (top_h - 2.0 * pd - sc.s - 12.0 * k - 46.0 * k).min(inner * 0.46).floor();
                let cy = m + pd + sc.s + 14.0 * k;
                cover_or_note(p, cover, x + pd, cy, cs, 16.0 * k, on(c, 0.5), (on(c, 0.2), on(c, 0.35)));
                let tx = x + pd + cs + 14.0 * k;
                let tw = x + cw - pd - tx;
                title_line(p, tx, cy + sc.m * 1.1 + 4.0 * k, tw, &title, sc.m * 1.1, W::Medium, INK);
                let a = ellipsize(p, &artist, sc.m, W::Regular, tw);
                p.text(tx, cy + sc.m * 1.1 + 4.0 * k + 12.0 * k + sc.m, &a, sc.m, W::Regular, on(c, 0.65), 255);
                if let Some((el, tot, f)) = track_times(d) {
                    let by = m + top_h - pd - 6.0 * k;
                    p.layer.fill_rrect((x + pd, by, inner, 6.0 * k), 3.0 * k, &|_| INK, 34);
                    p.layer.fill_rrect((x + pd, by, (inner * f).max(6.0 * k), 6.0 * k), 3.0 * k, &|_| INK, 255);
                    let ty = by - 11.0 * k;
                    let tw1 = p.text(x + pd, ty, &el, sc.s, W::Medium, INK, 255);
                    let _ = tw1;
                    text_r(p, x + cw - pd, ty, &tot, sc.s, W::Medium, on(c, 0.6));
                }
            }
        }
        let r2 = (x, m + top_h + g, cw, ph - top_h - g);
        let c2 = tint(th().net);
        tile(p, r2, c2);
        let y2 = r2.1;
        spaced(p, x + pd, y2 + pd + sc.s, "NETWORK", sc.s * 1.1, W::Medium, INK, 2.4 * k);
        let rows = [(&v.net_dn, true), (&v.net_up, false)];
        let top = y2 + pd + sc.s + 8.0 * k;
        let rh = (r2.3 - (top - y2) - pd) / 2.0;
        for (j, (rt, down)) in rows.iter().enumerate() {
            let base = top + j as f32 * rh + (rh + 40.0 * k) / 2.0;
            arrow(p, x + pd + 10.0 * k, base - 20.0 * k, 10.0 * k, *down, INK);
            let cap = fit_vu(p, &rt.0, rt.1, 40.0 * k, inner - 36.0 * k, W::Medium);
            let vw = p.text(x + pd + 32.0 * k, base, &rt.0, cap, W::Medium, INK, 255);
            p.text(x + pd + 32.0 * k + vw + (cap * 0.12).max(2.0), base, rt.1, (cap * 0.4).max(7.0), W::Regular, on(c2, 0.66), 255);
        }
    }
}

// ---------------------------------------------------------------- TERMINAL
//
// A green-phosphor console: monospaced-looking rows, block bars, scanlines.

fn terminal(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, cover: Option<&Cover>) {
    let k = sc.k;
    let ph = lerp_rgb(th().net[0], th().net[1], 0.5);
    let bg = lerp_rgb((0, 0, 0), ph, 0.045);
    let hi = lerp_rgb(ph, WHITE, 0.35);
    let mid = lerp_rgb(bg, ph, 0.72);
    let dim = lerp_rgb(bg, ph, 0.42);
    rect(p, (0.0, 0.0, w, h), bg, 255);
    let m = 24.0 * k;
    let cap = 17.0 * k;
    let vcap = 21.0 * k;
    // prompt line + clock
    let ty = m + cap;
    let pw = glow_text(p, m, ty, "trofeo@lcd:~$", cap, W::Medium, hi, 40);
    p.text(m + pw + 10.0 * k, ty, "monitor --live", cap, W::Regular, mid, 255);
    let blink = if v.secs % 2 == 0 { 255 } else { 40 };
    rect(p, (m + pw + 10.0 * k + p.width("monitor --live", cap, W::Regular) + 8.0 * k, ty - cap, 10.0 * k, cap + 2.0 * k), hi, blink);
    let stamp = format!("{} {}", v.wd.chars().take(3).collect::<String>(), v.date.to_uppercase());
    text_r(p, w - m, ty, &stamp, cap, W::Regular, mid);
    rect(p, (m, ty + 12.0 * k, w - 2.0 * m, 1.0), ph, 70);
    // columns
    let gap = 34.0 * k;
    let widths = [452.0f32, 452.0, 424.0, 430.0];
    let sum: f32 = widths.iter().sum();
    let avail = w - 2.0 * m - 3.0 * gap;
    let mut cx = Vec::new();
    let mut x = m;
    for wd in widths {
        let cw = avail * wd / sum;
        cx.push((x, cw));
        x += cw + gap;
    }
    let top = ty + 40.0 * k;
    let pitch = ((h - m - top) / 8.0).floor();
    let row = |p: &mut Painter, c: (f32, f32), i: usize, label: &str, value: &str, unit: &str, vcol: Rgb, frac: Option<f32>| {
        let y = top + i as f32 * pitch + pitch * 0.62;
        p.text(c.0, y, label, cap * 0.92, W::Regular, dim, 255);
        let bx = c.0 + 78.0 * k;
        let bw = c.1 - 78.0 * k - 126.0 * k;
        if let Some(f) = frac {
            seg_h(p, bx, y - cap * 0.82, bw, cap * 0.9, 16, f, 2.0 * k, &|t| lerp_rgb(ph, hi, t));
        } else {
            // dotted leader
            let mut dx = bx;
            while dx < bx + bw {
                rect(p, (dx, y - 2.0 * k, 2.0 * k, 2.0 * k), dim, 150);
                dx += 8.0 * k;
            }
        }
        let uw = if unit.is_empty() { 0.0 } else { p.width(unit, cap * 0.7, W::Regular) + 5.0 * k };
        if !unit.is_empty() {
            text_r(p, c.0 + c.1, y, unit, cap * 0.7, W::Regular, dim);
        }
        text_r(p, c.0 + c.1 - uw, y, value, vcap, W::Medium, vcol);
    };
    let head = |p: &mut Painter, c: (f32, f32), name: &str, sub: &str| {
        let hw = glow_text(p, c.0, ty + 42.0 * k - 12.0 * k + 0.0, name, cap, W::Medium, hi, 36);
        let s = ellipsize(p, sub, cap * 0.85, W::Regular, c.1 - hw - 16.0 * k);
        text_r(p, c.0 + c.1, ty + 30.0 * k, &s, cap * 0.85, W::Regular, dim);
    };
    let _ = head;
    // headers: "## CPU"
    for (c, name, sub) in [(cx[0], "[ CPU ]", v.cpu_name.as_str()), (cx[1], "[ GPU ]", v.gpu_name.as_str()), (cx[2], "[ MEM ]", "RAM + STORAGE")] {
        let hy = ty + 32.0 * k;
        let hw = glow_text(p, c.0, hy, name, cap, W::Medium, hi, 36);
        let s = ellipsize(p, sub, cap * 0.82, W::Regular, c.1 - hw - 14.0 * k);
        text_r(p, c.0 + c.1, hy, &s, cap * 0.82, W::Regular, dim);
    }
    let it = |it: &Item| (it.value.clone(), it.unit.clone(), if it.color == th().text { hi } else { it.color });
    // CPU
    let c0 = cx[0];
    row(p, c0, 0, "TEMP", &v.cpu_temp, "\u{b0}C", if v.cpu_temp_color == th().text { hi } else { v.cpu_temp_color }, v.cpu_temp_f);
    for (i, idx) in [0usize, 1, 2, 3, 4, 5, 6].iter().enumerate() {
        let x = &v.cpu[*idx];
        let (val, un, col) = it(x);
        row(p, c0, i + 1, x.label, &val, &un, col, x.frac);
    }
    // GPU
    let c1 = cx[1];
    row(p, c1, 0, "TEMP", &v.gpu_temp, "\u{b0}C", if v.gpu_temp_color == th().text { hi } else { v.gpu_temp_color }, v.gpu_temp_f);
    for (i, x) in v.gpu.iter().enumerate() {
        let (val, un, col) = it(x);
        row(p, c1, i + 1, x.label, &val, &un, col, x.frac);
    }
    // MEM
    let c2 = cx[2];
    row(p, c2, 0, "RAM", &v.ram_pct, "%", hi, Some(v.ram_frac));
    row(p, c2, 1, "USED", &v.mem[0].value, "GB", hi, None);
    row(p, c2, 2, "FREE", &v.mem[1].value, "GB", hi, None);
    row(p, c2, 3, "TOTAL", &v.ram_total, "GB", hi, None);
    let (sv, su, sc_) = it(&v.mem[2]);
    row(p, c2, 4, "SSD", &sv, &su, sc_, v.ssd_temp_f);
    row(p, c2, 5, "DISK", &v.mem[3].value, "%", hi, v.disk_frac);
    row(p, c2, 6, "READ", &v.mem[4].value, &v.mem[4].unit, hi, None);
    row(p, c2, 7, "WRITE", &v.mem[5].value, &v.mem[5].unit, hi, None);
    // clock, weather, now playing
    let (x4, w4) = cx[3];
    let tcap = p.fit(&v.hhmm, 96.0 * k, w4, W::Light);
    glow_text(p, x4 - 4.0 * k, ty + 34.0 * k + tcap, &v.hhmm, tcap, W::Light, hi, 34);
    let sx = x4 + p.width(&v.hhmm, tcap, W::Light) + 8.0 * k;
    p.text(sx, ty + 34.0 * k + tcap, &format!(":{:02}", v.secs), cap, W::Regular, mid, 255);
    let mut y = ty + 34.0 * k + tcap + 30.0 * k;
    if let Some(wx) = &v.wx {
        p.text(x4, y, "$ weather", cap * 0.9, W::Regular, dim, 255);
        weather_icon(p, x4 + w4 - 50.0 * k, y - 20.0 * k, 44.0 * k, wx.icon, wx.night);
        p.text(x4, y + 28.0 * k, &format!("{}  {}", wx.temp, wx.cond), vcap, W::Medium, hi, 255);
        let line = if wx.city.is_empty() { String::new() } else { wx.city.clone() };
        p.text(x4, y + 52.0 * k, &ellipsize(p, &line, cap * 0.85, W::Regular, w4), cap * 0.85, W::Regular, mid, 255);
        let days: Vec<String> = wx.days.iter().map(|f| format!("{} {}/{}", f.name, f.hi.trim_end_matches('\u{b0}'), f.lo.trim_end_matches('\u{b0}'))).collect();
        let dl = days.join("   ");
        p.text(x4, y + 76.0 * k, &ellipsize(p, &dl, cap * 0.82, W::Regular, w4), cap * 0.82, W::Regular, dim, 255);
        y += 108.0 * k;
    }
    p.text(x4, y, "$ now-playing", cap * 0.9, W::Regular, dim, 255);
    match track_lines(d) {
        None => {
            p.text(x4, y + 28.0 * k, "(nothing playing)", cap, W::Regular, dim, 255);
        }
        Some((title, artist)) => {
            let cs = 76.0 * k;
            cover_or_note(p, cover, x4, y + 12.0 * k, cs, 4.0 * k, mid, (lerp_rgb(bg, ph, 0.12), lerp_rgb(bg, ph, 0.2)));
            let tx = x4 + cs + 12.0 * k;
            let tw = w4 - cs - 12.0 * k;
            title_line(p, tx, y + 30.0 * k, tw, &title, cap, W::Medium, hi);
            p.text(tx, y + 52.0 * k, &ellipsize(p, &artist, cap * 0.88, W::Regular, tw), cap * 0.88, W::Regular, mid, 255);
            if let Some((el, tot, f)) = track_times(d) {
                seg_h(p, tx, y + 64.0 * k, tw, 8.0 * k, 24, f, 2.0 * k, &|_| ph);
                p.text(tx, y + 92.0 * k, &el, cap * 0.8, W::Regular, mid, 255);
                text_r(p, tx + tw, y + 92.0 * k, &tot, cap * 0.8, W::Regular, dim);
            }
        }
    }
    // scanlines
    let mut sy = 0.0;
    while sy < h {
        p.layer.span_over(sy as i32, 0, w as i32, (0, 0, 0), 46);
        sy += 3.0;
    }
}

// ----------------------------------------------------------------- CHAMFER
//
// Angular HUD: slanted panels, italic numbers, hatch stripes and an LED rail.

fn chamfer(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, cover: Option<&Cover>) {
    let k = sc.k;
    let m = 14.0 * k;
    let slant = 26.0 * k;
    let ca = th().cpu[0];
    let cb = th().gpu[0];
    let cc = th().ram[0];
    // LED rail: CPU load from the left, GPU load from the right
    let ry = m;
    let rh = 9.0 * k;
    let half = (w - 2.0 * m) / 2.0 - 6.0 * k;
    seg_h(p, m, ry, half, rh, 40, v.cpu_frac, 3.0 * k, &|t| lerp_rgb(th().cpu[0], th().cpu[1], t));
    let g_frac = v.gpu_frac.unwrap_or(0.0);
    let lit = (g_frac * 40.0).round() as usize;
    let bw = (half - 3.0 * k * 39.0) / 40.0;
    for i in 0..40 {
        let bx = m + half + 12.0 * k + (39 - i) as f32 * (bw + 3.0 * k);
        if i < lit {
            rect(p, (bx, ry, bw, rh), lerp_rgb(th().gpu[0], th().gpu[1], i as f32 / 40.0), 255);
        } else {
            rect(p, (bx, ry, bw, rh), WHITE, 22);
        }
    }
    let top = ry + rh + 12.0 * k;
    let ph = h - top - m;
    let weights = [300.0f32, 436.0, 452.0, 330.0, 352.0];
    let gap = 8.0 * k;
    let avail = w - 2.0 * m - 4.0 * gap - slant;
    let sum: f32 = weights.iter().sum();
    let mut cols = Vec::new();
    let mut x = m;
    for wt in weights {
        let cw = avail * wt / sum;
        cols.push((x, cw + slant));
        x += cw + gap;
    }
    let shape = |x: f32, wd: f32| [(x + slant, top), (x + wd, top), (x + wd - slant, top + ph), (x, top + ph)];
    let panel_bg = lerp_rgb(th().panel, WHITE, 0.03);
    for (i, &(x, wd)) in cols.iter().enumerate() {
        let acc = [cc, ca, cb, cc, ca][i];
        poly(p, &shape(x, wd), panel_bg, 255);
        poly_outline(p, &shape(x, wd), 0.7 * k, WHITE, 36);
        // accent edge on the left slant
        capsule(p, (x + slant, top + 1.0), (x, top + ph - 1.0), 2.0 * k, acc, 255);
        // hatch in the top right corner
        let hx1 = x + wd - 6.0 * k;
        let mut hx = hx1 - 64.0 * k;
        while hx < hx1 {
            capsule(p, (hx + 10.0 * k, top + 10.0 * k), (hx, top + 22.0 * k), 0.8 * k, acc, 150);
            hx += 8.0 * k;
        }
    }
    let tag = |p: &mut Painter, x: f32, y: f32, text: &str, c: Rgb| -> f32 {
        let tw = spaced_width(p, text, sc.s * 1.05, W::Medium, 2.0 * k);
        let pts = [(x + 8.0 * k, y), (x + tw + 22.0 * k, y), (x + tw + 14.0 * k, y + 24.0 * k), (x, y + 24.0 * k)];
        poly(p, &pts, c, 255);
        spaced(p, x + 10.0 * k, y + 17.5 * k, text, sc.s * 1.05, W::Medium, INK, 2.0 * k);
        tw + 26.0 * k
    };
    let lx = |c: (f32, f32)| c.0 + slant + 16.0 * k;
    let rx = |c: (f32, f32)| c.0 + c.1 - slant * 0.4 - 16.0 * k;
    // helper for a row "LABEL ..... value unit"
    let rowf = |p: &mut Painter, x0: f32, x1: f32, base: f32, it: &Item, label: &str| {
        p.text(x0, base, label, sc.s, W::Medium, th().dim, 255);
        let uw = if it.unit.is_empty() { 0.0 } else { p.width(&it.unit, sc.s, W::Regular) + 5.0 * k };
        if !it.unit.is_empty() {
            text_r(p, x1, base, &it.unit, sc.s, W::Regular, th().muted);
        }
        text_r(p, x1 - uw, base, &it.value, sc.m * 1.3, W::Medium, it.color);
        let lw = p.width(label, sc.s, W::Medium) + 10.0 * k;
        let vw = p.width(&it.value, sc.m * 1.3, W::Medium) + uw + 10.0 * k;
        let mut dx = x0 + lw;
        while dx < x1 - vw {
            rect(p, (dx, base - 3.0 * k, 2.0 * k, 2.0 * k), WHITE, 40);
            dx += 7.0 * k;
        }
    };
    // CPU / GPU / MEM panels
    for (i, (acc, label, name, hero, hcol, unit_s, items)) in [
        (ca, "CPU", v.cpu_name.clone(), v.cpu_temp.clone(), v.cpu_temp_color, "\u{b0}C", v.cpu.iter().collect::<Vec<_>>()),
        (cb, "GPU", v.gpu_name.clone(), v.gpu_temp.clone(), v.gpu_temp_color, "\u{b0}C", v.gpu.iter().collect::<Vec<_>>()),
        (cc, "MEM", format!("of {} GB", v.ram_total), v.ram_pct.clone(), th().text, "%", v.mem.iter().collect::<Vec<_>>()),
    ]
    .into_iter()
    .enumerate()
    {
        let c = cols[1 + i];
        let x0 = lx(c);
        let x1 = rx(c);
        let tw = tag(p, x0, top + 12.0 * k, label, acc);
        let nm = ellipsize(p, &name, sc.s, W::Regular, x1 - x0 - tw - 80.0 * k);
        text_r(p, x1 - 66.0 * k, top + 28.0 * k, &nm, sc.s, W::Regular, th().dim);
        // hero, italic
        let hcap = 98.0 * k;
        let hb = top + 44.0 * k + hcap;
        let hw = skew_text(p, x0 - 4.0 * k, hb, &hero, hcap, W::Medium, hcol, 0.2);
        skew_text(p, x0 + hw + 6.0 * k, hb - hcap + sc.l, unit_s, sc.l, W::Regular, th().muted, 0.2);
        // slab under the hero
        poly(p, &[(x0 + 8.0 * k, hb + 10.0 * k), (x0 + 150.0 * k, hb + 10.0 * k), (x0 + 142.0 * k, hb + 16.0 * k), (x0, hb + 16.0 * k)], acc, 255);
        // rows
        let ry0 = hb + 30.0 * k;
        let n = items.len();
        let pitch = ((top + ph - 12.0 * k) - ry0) / n as f32;
        for (j, it) in items.iter().enumerate() {
            let base = ry0 + j as f32 * pitch + pitch * 0.68;
            rowf(p, x0, x1, base, it, if it.label == "DOWN" { "NET DOWN" } else if it.label == "UP" { "NET UP" } else { it.label });
        }
    }
    // clock + weather
    {
        let c = cols[0];
        let x0 = lx(c);
        let x1 = rx(c);
        let tw = tag(p, x0, top + 12.0 * k, &v.wd, cc);
        let _ = tw;
        let tcap = p.fit(&v.hhmm, 92.0 * k, x1 - x0 - 6.0 * k, W::Medium);
        let tb = top + 48.0 * k + tcap;
        skew_text(p, x0 - 2.0 * k, tb, &v.hhmm, tcap, W::Medium, th().text, 0.2);
        p.text(x0, tb + 26.0 * k, &v.date, sc.m, W::Regular, th().muted, 255);
        // seconds as a slanted bar
        let sy = tb + 40.0 * k;
        poly(p, &[(x0 + 6.0 * k, sy), (x1, sy), (x1 - 6.0 * k, sy + 6.0 * k), (x0, sy + 6.0 * k)], WHITE, 26);
        let fw = (x1 - x0) * v.secs as f32 / 59.0;
        if fw > 8.0 * k {
            poly(p, &[(x0 + 6.0 * k, sy), (x0 + fw, sy), (x0 + fw - 6.0 * k, sy + 6.0 * k), (x0, sy + 6.0 * k)], cc, 255);
        }
        if let Some(wx) = &v.wx {
            let wy = sy + 22.0 * k;
            weather_icon(p, x0 - 4.0 * k, wy, 50.0 * k, wx.icon, wx.night);
            p.text(x0 + 52.0 * k, wy + 25.0 * k + sc.l / 2.0, &wx.temp, sc.l, W::Medium, th().text, 255);
            let line = if wx.city.is_empty() { wx.cond.clone() } else { format!("{} \u{b7} {}", wx.cond, wx.city) };
            p.text(x0, wy + 50.0 * k + 10.0 * k + sc.m * 0.9, &ellipsize(p, &line, sc.m * 0.9, W::Regular, x1 - x0), sc.m * 0.9, W::Regular, th().muted, 255);
            weather_days_row(p, sc, x0, top + ph - 14.0 * k, x1 - x0, &wx.days, th().dim, th().text, th().dim);
        }
    }
    // now playing
    {
        let c = cols[4];
        let x0 = lx(c);
        let x1 = rx(c);
        let tw = tag(p, x0, top + 12.0 * k, "NOW PLAYING", cc);
        let _ = tw;
        match track_lines(d) {
            None => {
                music_note(p, (x0 + x1) / 2.0, top + ph / 2.0, 52.0 * k, th().dim);
            }
            Some((title, artist)) => {
                let cs = (x1 - x0).min(ph - 190.0 * k).floor();
                // cover in a slanted frame
                cover_or_note(p, cover, x0 + (x1 - x0 - cs) / 2.0, top + 44.0 * k, cs, 6.0 * k, th().muted, ((30, 32, 40), (20, 21, 27)));
                let ty = top + 44.0 * k + cs + 24.0 * k;
                title_line(p, x0, ty, x1 - x0, &title, sc.m * 1.1, W::Medium, th().text);
                p.text(x0, ty + 24.0 * k, &ellipsize(p, &artist, sc.m, W::Regular, x1 - x0), sc.m, W::Regular, th().muted, 255);
                if let Some((el, tot, f)) = track_times(d) {
                    let by = top + ph - 40.0 * k;
                    seg_h(p, x0, by, x1 - x0, 6.0 * k, 28, f, 2.0 * k, &|t| lerp_rgb(cc, WHITE, t * 0.5));
                    p.text(x0, by + 24.0 * k, &el, sc.s, W::Medium, th().text, 255);
                    text_r(p, x1, by + 24.0 * k, &tot, sc.s, W::Medium, th().dim);
                }
            }
        }
    }
}

// ------------------------------------------------------------------- RINGS
//
// Three clusters of concentric rings (Apple-activity style), readings around them.

fn ring_set(p: &mut Painter, cx: f32, cy: f32, d: f32, rings: &[(Option<f32>, [Rgb; 2])], k: f32) {
    let thick = 19.0 * k;
    let gap = 7.0 * k;
    for (i, (f, colors)) in rings.iter().enumerate() {
        let ro = d / 2.0 - i as f32 * (thick + gap);
        p.layer.arc(cx, cy, ro, thick, 1.0, &|_| colors[0], 34);
        if let Some(f) = f {
            let f = f.clamp(0.0, 1.0);
            if f > 0.004 {
                let span = f.max(0.02);
                let cl = *colors;
                p.layer.arc(cx, cy, ro, thick, f, &move |t| gradient_at(&cl, t / span), 255);
            }
        }
    }
}

fn rings(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, cover: Option<&Cover>) {
    let k = sc.k;
    let m = 26.0 * k;
    let right_w = 352.0 * k;
    let gw = (w - 2.0 * m - right_w - 3.0 * 22.0 * k) / 3.0;
    let ring_d = 252.0 * k;
    let cy = h / 2.0 - 10.0 * k;
    let side_w = (gw - ring_d) / 2.0 - 12.0 * k;
    struct G<'a> {
        title: &'a str,
        sub: String,
        colors: [[Rgb; 2]; 3],
        fracs: [Option<f32>; 3],
        center: (String, String, Rgb),
        left: Vec<&'a Item>,
        right: Vec<&'a Item>,
        legend: [&'static str; 3],
    }
    let clock_f = d.cpu_mhz.map(|m| (m as f32 / 5200.0).clamp(0.0, 1.0));
    let groups = [
        G {
            title: "CPU",
            sub: v.cpu_name.clone(),
            colors: [th().cpu, [lerp_rgb(th().cpu[0], th().gpu[0], 0.5), lerp_rgb(th().cpu[1], th().gpu[1], 0.5)], th().ssd],
            fracs: [Some(v.cpu_frac), v.cpu_temp_f, clock_f],
            center: (v.cpu_temp.clone(), "\u{b0}C".into(), v.cpu_temp_color),
            left: vec![&v.cpu[0], &v.cpu[1], &v.cpu[2], &v.cpu[3]],
            right: vec![&v.cpu[4], &v.cpu[5], &v.cpu[6]],
            legend: ["LOAD", "TEMP", "CLOCK"],
        },
        G {
            title: "GPU",
            sub: v.gpu_name.clone(),
            colors: [th().gpu, [lerp_rgb(th().gpu[0], th().ram[0], 0.5), lerp_rgb(th().gpu[1], th().ram[1], 0.5)], th().net],
            fracs: [v.gpu_frac, v.gpu_temp_f, v.vram_frac],
            center: (v.gpu_temp.clone(), "\u{b0}C".into(), v.gpu_temp_color),
            left: vec![&v.gpu[0], &v.gpu[1], &v.gpu[2], &v.gpu[3]],
            right: vec![&v.gpu[4], &v.gpu[5], &v.gpu[6]],
            legend: ["LOAD", "TEMP", "VRAM"],
        },
        G {
            title: "MEMORY",
            sub: format!("{} GB", v.ram_total),
            colors: [th().ram, th().ssd, [lerp_rgb(th().ssd[0], th().net[0], 0.5), lerp_rgb(th().ssd[1], th().net[1], 0.5)]],
            fracs: [Some(v.ram_frac), v.disk_frac, v.ssd_temp_f],
            center: (v.ram_pct.clone(), "%".into(), th().text),
            left: vec![&v.mem[0], &v.mem[1], &v.mem[4]],
            right: vec![&v.mem[2], &v.mem[3], &v.mem[5]],
            legend: ["RAM", "DISK", "SSD"],
        },
    ];
    for (gi, g) in groups.iter().enumerate() {
        let gx = m + gi as f32 * (gw + 22.0 * k);
        let cx = gx + gw / 2.0;
        let rings: Vec<(Option<f32>, [Rgb; 2])> = (0..3).map(|i| (g.fracs[i], g.colors[i])).collect();
        ring_set(p, cx, cy, ring_d, &rings, k);
        // center
        let inner = ring_d - 3.0 * 2.0 * (19.0 + 7.0) * k + 10.0 * k;
        let cap = fit_vu(p, &g.center.0, &g.center.1, 64.0 * k, inner, W::Medium);
        let tw = value_unit_width(p, &g.center.0, &g.center.1, cap, W::Medium);
        value_unit(p, cx - tw / 2.0, cy + cap / 2.0 - 2.0 * k, &g.center.0, &g.center.1, cap, W::Medium, g.center.2);
        // title under the rings
        let ty = cy + ring_d / 2.0 + 28.0 * k;
        let tw = spaced_width(p, g.title, sc.s * 1.1, W::Medium, 3.0 * k);
        spaced(p, cx - tw / 2.0, ty, g.title, sc.s * 1.1, W::Medium, th().text, 3.0 * k);
        // ring legend dots
        let sub = ellipsize(p, &g.sub, sc.s * 0.9, W::Regular, gw - 20.0 * k);
        text_c(p, cx, ty + 20.0 * k, &sub, sc.s * 0.9, W::Regular, th().dim);
        // readings: left column right-aligned, right column left-aligned
        let sw = side_w.max(60.0 * k);
        for (side, list) in [(0, &g.left), (1, &g.right)] {
            let n = list.len();
            let pitch = 62.0 * k;
            let y0 = cy - (n as f32 * pitch) / 2.0 + 6.0 * k;
            for (j, it) in list.iter().enumerate() {
                let y = y0 + j as f32 * pitch;
                let label = it.label;
                let vcap = fit_vu(p, &it.value, &it.unit, 29.0 * k, sw, W::Medium);
                let vw = value_unit_width(p, &it.value, &it.unit, vcap, W::Medium);
                let lw = spaced_width(p, label, sc.s * 0.9, W::Medium, 1.4 * k);
                if side == 0 {
                    let x = gx + sw;
                    spaced(p, x - lw, y + sc.s, label, sc.s * 0.9, W::Medium, th().dim, 1.4 * k);
                    value_unit(p, x - vw, y + sc.s + 10.0 * k + vcap, &it.value, &it.unit, vcap, W::Medium, it.color);
                } else {
                    let x = gx + gw - sw;
                    spaced(p, x, y + sc.s, label, sc.s * 0.9, W::Medium, th().dim, 1.4 * k);
                    value_unit(p, x, y + sc.s + 10.0 * k + vcap, &it.value, &it.unit, vcap, W::Medium, it.color);
                }
            }
        }
        // legend (which ring is what) at the very bottom of the group
        let ly = h - 22.0 * k;
        let mut lxp = cx - 118.0 * k;
        for i in 0..3 {
            circle(p, lxp + 4.0 * k, ly - sc.s * 0.4, 4.0 * k, g.colors[i][1], 255);
            lxp += 12.0 * k + spaced(p, lxp + 12.0 * k, ly, g.legend[i], sc.s * 0.8, W::Medium, th().dim, 1.0 * k) + 14.0 * k;
        }
    }
    // right column: clock, weather, now playing
    let x = w - m - right_w;
    let tcap = p.fit(&v.hhmm, 88.0 * k, right_w, W::Light);
    p.text(x - 4.0 * k, m + 4.0 * k + tcap, &v.hhmm, tcap, W::Light, th().text, 255);
    spaced(p, x, m + tcap + 34.0 * k, &format!("{}  \u{b7}  {}", v.wd, v.date.to_uppercase()), sc.s * 0.95, W::Medium, th().muted, 1.6 * k);
    let mut y = m + tcap + 58.0 * k;
    if let Some(wx) = &v.wx {
        weather_icon(p, x - 3.0 * k, y, 44.0 * k, wx.icon, wx.night);
        let tw = p.text(x + 50.0 * k, y + 22.0 * k + sc.l / 2.0, &wx.temp, sc.l, W::Medium, th().text, 255);
        let line = if wx.city.is_empty() { wx.cond.clone() } else { format!("{} \u{b7} {}", wx.cond, wx.city) };
        p.text(x + 60.0 * k + tw, y + 22.0 * k + sc.m / 2.0, &ellipsize(p, &line, sc.m, W::Regular, right_w - 70.0 * k - tw), sc.m, W::Regular, th().muted, 255);
        y += 50.0 * k + 38.0 * k;
        weather_days_row(p, sc, x, y, right_w, &wx.days, th().dim, th().text, th().dim);
        y += 18.0 * k;
    }
    // media at the bottom
    let mh = 92.0 * k;
    let my = h - m - mh;
    capsule(p, (x, my - 14.0 * k), (x + right_w, my - 14.0 * k), 0.5 * k, WHITE, 36);
    let big = Scale { m: sc.m * 1.15, s: sc.s * 1.1, ..*sc };
    now_playing_compact(p, (x, my, right_w, mh), &big, d, cover, true);
}

// -------------------------------------------------------------------- DOTS
//
// Dot-matrix digits on black (Nothing-style): numbers made of round dots, labels in
// a small sans, one red accent.

const DOT_FONT: [(char, [u8; 7]); 16] = [
    ('0', [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
    ('1', [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('2', [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111]),
    ('3', [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110]),
    ('4', [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
    ('5', [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
    ('6', [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
    ('7', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
    ('8', [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
    ('9', [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100]),
    (':', [0b0, 0b0, 0b1, 0b0, 0b1, 0b0, 0b0]),
    ('.', [0b0, 0b0, 0b0, 0b0, 0b0, 0b0, 0b1]),
    ('-', [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000]),
    ('/', [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000]),
    ('%', [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011]),
    (' ', [0; 7]),
];

/// Width in pixels of `text` made of dots with pitch `u`.
fn dot_width(text: &str, u: f32) -> f32 {
    let mut w = 0.0;
    for c in text.chars() {
        w += match c {
            ':' | '.' => 2.4 * u,
            ' ' => 3.0 * u,
            _ => 6.2 * u,
        };
    }
    (w - 1.2 * u).max(0.0)
}

/// Draws `text` as dots (unknown characters in the regular font) with the top-left of the
/// first character at (x, y); returns the width.
fn dot_text(p: &mut Painter, x: f32, y: f32, text: &str, u: f32, on: Rgb, off_alpha: u8) -> f32 {
    let r = u * 0.40;
    let mut pen = x;
    for c in text.chars() {
        let Some((_, rows)) = DOT_FONT.iter().find(|(ch, _)| *ch == c) else {
            let cap = u * 7.0 * 0.92;
            let wch = p.text(pen, y + u * 6.5 + r, &c.to_string(), cap, W::Regular, on, 255);
            pen += wch + 1.2 * u;
            continue;
        };
        let cols = match c {
            ':' | '.' => 1,
            ' ' => 0,
            _ => 5,
        };
        for (ry, bits) in rows.iter().enumerate() {
            for cx in 0..cols {
                let lit = (bits >> (cols - 1 - cx)) & 1 == 1;
                let (px, py) = (pen + cx as f32 * u + u / 2.0, y + ry as f32 * u + u / 2.0);
                if lit {
                    circle(p, px, py, r, on, 255);
                } else if off_alpha > 0 {
                    circle(p, px, py, r * 0.55, WHITE, off_alpha);
                }
            }
        }
        pen += match c {
            ':' | '.' => 2.4 * u,
            ' ' => 3.0 * u,
            _ => 6.2 * u,
        };
    }
    (pen - x - 1.2 * u).max(0.0)
}

fn dots(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, cover: Option<&Cover>) {
    let k = sc.k;
    let red = (255, 54, 40);
    let m = 24.0 * k;
    let right_w = 372.0 * k;
    let left_w = w - 2.0 * m - right_w - 26.0 * k;
    let rows = 3usize;
    let rh = (h - 2.0 * m) / rows as f32;
    let hero_w = 318.0 * k;
    let cell_w = (left_w - hero_w) / 7.0;
    // dotted separators
    let sep = |p: &mut Painter, x0: f32, y0: f32, x1: f32, y1: f32| {
        let n = (((x1 - x0).hypot(y1 - y0)) / (7.0 * k)) as i32;
        for i in 0..=n {
            let t = i as f32 / n.max(1) as f32;
            circle(p, x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, 1.1 * k, WHITE, 70);
        }
    };
    for r in 1..rows {
        sep(p, m, m + r as f32 * rh, m + left_w, m + r as f32 * rh);
    }
    sep(p, m + left_w + 13.0 * k, m, m + left_w + 13.0 * k, h - m);
    let u_hero = 10.2 * k;
    let u_small = 4.7 * k;
    let sets: [(&str, String, String, String, Rgb, Vec<&Item>); 3] = [
        ("CPU", v.cpu_name.clone(), v.cpu_temp.clone(), "\u{b0}C".into(), v.cpu_temp_color, v.cpu.iter().collect()),
        ("GPU", v.gpu_name.clone(), v.gpu_temp.clone(), "\u{b0}C".into(), v.gpu_temp_color, v.gpu.iter().collect()),
        ("MEMORY", "RAM".to_string(), v.ram_pct.clone(), "%".into(), th().text, v.mem.iter().collect()),
    ];
    for (ri, (label, sub, hero, hunit, hcol, items)) in sets.iter().enumerate() {
        let y0 = m + ri as f32 * rh;
        // label + hero
        circle(p, m + 5.0 * k, y0 + 17.0 * k, 4.0 * k, red, 255);
        spaced(p, m + 18.0 * k, y0 + 22.0 * k, label, sc.s, W::Medium, th().muted, 2.2 * k);
        let sub = ellipsize(p, sub, sc.s * 0.85, W::Regular, hero_w - 30.0 * k);
        p.text(m + 18.0 * k, y0 + 40.0 * k, &sub, sc.s * 0.85, W::Regular, th().dim, 255);
        let hw = dot_text(p, m, y0 + 56.0 * k, hero, u_hero, *hcol, 14);
        p.text(m + hw + 10.0 * k, y0 + 56.0 * k + 22.0 * k, hunit, sc.l * 0.8, W::Regular, th().muted, 255);
        // readings (memory has one cell more: the total)
        let total_item = item("TOTAL", v.ram_total.clone(), "GB", th().text, None);
        let all: Vec<&Item> = if ri == 2 { items.iter().copied().chain(std::iter::once(&total_item)).collect() } else { items.clone() };
        for (j, it) in all.iter().enumerate() {
            let x = m + hero_w + j as f32 * cell_w;
            spaced(p, x, y0 + 22.0 * k, it.label, sc.s * 0.9, W::Medium, th().dim, 1.4 * k);
            let col = if it.color == th().text { WHITE } else { it.color };
            let mut u = u_small;
            while dot_width(&it.value, u) > cell_w - 16.0 * k && u > 2.5 * k {
                u -= 0.2 * k;
            }
            let vw = dot_text(p, x, y0 + 50.0 * k, &it.value, u, col, 12);
            if !it.unit.is_empty() {
                p.text(x + vw + 5.0 * k, y0 + 50.0 * k + 7.0 * u, &it.unit, sc.s * 0.85, W::Regular, th().muted, 255);
            }
            if let Some(f) = it.frac {
                // a row of dots as a gauge
                let n = 12;
                for q in 0..n {
                    let on_ = (q as f32) < f.clamp(0.0, 1.0) * n as f32;
                    circle(p, x + 2.0 * k + q as f32 * ((cell_w - 22.0 * k) / n as f32), y0 + rh - 22.0 * k, 1.5 * k, if on_ { red } else { WHITE }, if on_ { 255 } else { 40 });
                }
            }
        }
    }
    // right column
    let x = m + left_w + 26.0 * k;
    let hhmm = v.hhmm.clone();
    let uc = ((right_w) / dot_width(&hhmm, 1.0)).min(11.0 * k);
    let cw_ = dot_text(p, x, m + 4.0 * k, &hhmm, uc, WHITE, 14);
    let _ = cw_;
    spaced(p, x, m + 7.0 * uc + 34.0 * k, &format!("{}  \u{b7}  {}", v.wd, v.date.to_uppercase()), sc.s, W::Medium, th().muted, 1.8 * k);
    let mut y = m + 7.0 * uc + 62.0 * k;
    if let Some(wx) = &v.wx {
        weather_icon(p, x - 3.0 * k, y - 8.0 * k, 46.0 * k, wx.icon, wx.night);
        let tw = dot_text(p, x + 52.0 * k, y + 2.0 * k, wx.temp.trim_end_matches('\u{b0}'), 4.8 * k, WHITE, 12);
        p.text(x + 58.0 * k + tw, y + 14.0 * k, "\u{b0}", sc.l * 0.7, W::Regular, th().muted, 255);
        let line = if wx.city.is_empty() { wx.cond.clone() } else { format!("{} \u{b7} {}", wx.cond, wx.city) };
        p.text(x, y + 52.0 * k, &ellipsize(p, &line, sc.m * 0.95, W::Regular, right_w), sc.m * 0.95, W::Regular, th().muted, 255);
        let days: Vec<String> = wx.days.iter().map(|f| format!("{} {} {}", f.name, f.hi, f.lo)).collect();
        p.text(x, y + 78.0 * k, &ellipsize(p, &days.join("   "), sc.s, W::Regular, right_w), sc.s, W::Regular, th().dim, 255);
        y += 100.0 * k;
    }
    let my = h - m - 112.0 * k;
    sep(p, x, my - 10.0 * k, x + right_w, my - 10.0 * k);
    let big = Scale { m: sc.m * 1.2, s: sc.s * 1.12, ..*sc };
    now_playing_compact(p, (x, my, right_w, 112.0 * k), &big, d, cover, true);
    let _ = y;
}

// ======================================================================
//                         PORTRAIT DESIGNS (462 x 1920)
// ======================================================================

thread_local! {
    /// Cached backgrounds (soft blobs, gradients): drawn once per theme and size.
    static BG_CACHE: std::cell::RefCell<Option<(String, Layer)>> = const { std::cell::RefCell::new(None) };
}

/// Paints the cached background `key` (calls `make` to build it the first time).
fn cached_bg(p: &mut Painter, key: String, make: &dyn Fn(&mut Painter)) {
    BG_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let ok = c.as_ref().is_some_and(|(k, l)| *k == key && l.w == p.layer.w && l.h == p.layer.h);
        if !ok {
            let mut l = Layer::new(p.layer.w as u32, p.layer.h as u32);
            {
                let mut q = Painter { layer: &mut l, font: FontStyle::Smooth };
                make(&mut q);
            }
            *c = Some((key, l));
        }
        if let Some((_, l)) = c.as_ref() {
            p.layer.copy_from(l);
        }
    });
}

/// Soft round glow: full color at the centre, fading to nothing at `r`.
fn blob(p: &mut Painter, cx: f32, cy: f32, r: f32, c: Rgb, alpha: f32) {
    for py in ((cy - r).floor() as i32)..((cy + r).ceil() as i32) {
        for px in ((cx - r).floor() as i32)..((cx + r).ceil() as i32) {
            let dd = ((px as f32 + 0.5 - cx).hypot(py as f32 + 0.5 - cy) / r).min(1.0);
            let f = (1.0 - dd) * (1.0 - dd);
            let a = (alpha * f) as u8;
            if a > 0 {
                p.layer.over(px, py, c, a);
            }
        }
    }
}

/// "LABEL ....... value unit" (dotted leader between), cap = value size.
fn kv_row(p: &mut Painter, x0: f32, x1: f32, base: f32, label: &str, value: &str, unit: &str, cap: f32, lc: Rgb, vc: Rgb, uc: Rgb, dot: Rgb, k: f32) {
    let lcap = cap * 0.62;
    p.text(x0, base, label, lcap, W::Medium, lc, 255);
    let lw = p.width(label, lcap, W::Medium) + 8.0 * k;
    let uw = if unit.is_empty() { 0.0 } else { p.width(unit, cap * 0.5, W::Regular) + 4.0 * k };
    if !unit.is_empty() {
        text_r(p, x1, base, unit, cap * 0.5, W::Regular, uc);
    }
    let vw = text_r(p, x1 - uw, base, value, cap, W::Medium, vc);
    let mut dx = x0 + lw;
    while dx < x1 - uw - vw - 8.0 * k {
        rect(p, (dx, base - 2.5 * k, 2.0 * k, 2.0 * k), dot, 120);
        dx += 6.0 * k;
    }
}

fn glass(p: &mut Painter, r: R, rad: f32, tintc: Rgb) {
    let r = (r.0.round(), r.1.round(), r.2.round(), r.3.round());
    p.layer.fill_rrect(r, rad, &|_| WHITE, 15);
    p.layer.fill_rrect(r, rad, &move |_| tintc, 14);
    p.layer.stroke_rrect(r, rad, 1.0, WHITE, 34);
    capsule(p, (r.0 + rad, r.1 + 1.0), (r.0 + r.2 - rad, r.1 + 1.0), 0.6, WHITE, 90);
}

// ------------------------------------------------------------------- LOCK
//
// A phone lock screen: a huge clock over a soft aurora, glass cards below.

fn lock(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, cover: Option<&Cover>) {
    let k = sc.k;
    let key = format!("lock{:?}{:?}{:?}{:?}{}", th().cpu, th().gpu, th().ram, th().bg, h as i32);
    cached_bg(p, key, &|q| {
        rect(q, (0.0, 0.0, w, h), th().bg, 255);
        blob(q, w * 0.15, h * 0.10, 380.0 * k, th().cpu[0], 120.0);
        blob(q, w * 0.95, h * 0.30, 420.0 * k, th().gpu[0], 105.0);
        blob(q, w * 0.05, h * 0.55, 440.0 * k, th().ram[0], 100.0);
        blob(q, w * 0.95, h * 0.80, 440.0 * k, th().ssd[0], 90.0);
        blob(q, w * 0.30, h * 0.98, 400.0 * k, th().net[0], 90.0);
    });
    let m = 24.0 * k;
    let cw = w - 2.0 * m;
    // date + clock
    let dl = format!("{}  \u{b7}  {}", v.wd, v.date.to_uppercase());
    let dw = spaced_width(p, &dl, sc.s * 1.1, W::Medium, 2.4 * k);
    spaced(p, w / 2.0 - dw / 2.0, 52.0 * k, &dl, sc.s * 1.1, W::Medium, th().text, 2.4 * k);
    let tcap = p.fit(&v.hhmm, 168.0 * k, cw, W::Light);
    text_c(p, w / 2.0, 70.0 * k + tcap, &v.hhmm, tcap, W::Light, th().text);
    let sy = 70.0 * k + tcap + 22.0 * k;
    p.layer.fill_rrect((m + 40.0 * k, sy, cw - 80.0 * k, 4.0 * k), 2.0 * k, &|_| WHITE, 30);
    p.layer.fill_rrect((m + 40.0 * k, sy, ((cw - 80.0 * k) * v.secs as f32 / 59.0).max(4.0 * k), 4.0 * k), 2.0 * k, &|_| WHITE, 220);
    // weather
    let wy = sy + 30.0 * k;
    if let Some(wx) = &v.wx {
        glass(p, (m, wy, cw, 180.0 * k), 28.0 * k, th().time);
        weather_icon(p, m + 16.0 * k, wy + 14.0 * k, 66.0 * k, wx.icon, wx.night);
        p.text(m + 96.0 * k, wy + 82.0 * k, &wx.temp, sc.xl * 1.1, W::Medium, th().text, 255);
        let line = if wx.city.is_empty() { wx.cond.clone() } else { format!("{} \u{b7} {}", wx.cond, wx.city) };
        p.text(m + 20.0 * k, wy + 118.0 * k, &ellipsize(p, &line, sc.m, W::Regular, cw - 40.0 * k), sc.m, W::Regular, th().muted, 255);
        weather_days_row(p, sc, m + 20.0 * k, wy + 166.0 * k, cw - 40.0 * k, &wx.days, th().dim, th().text, th().dim);
    }
    // cards
    let mut y = wy + 180.0 * k + 14.0 * k;
    let card_h = [372.0 * k, 372.0 * k, 306.0 * k];
    let specs: [(&str, String, String, String, Rgb, [Rgb; 2], Vec<&Item>); 3] = [
        ("CPU", v.cpu_name.clone(), v.cpu_temp.clone(), "\u{b0}C".into(), v.cpu_temp_color, th().cpu, v.cpu.iter().collect()),
        ("GPU", v.gpu_name.clone(), v.gpu_temp.clone(), "\u{b0}C".into(), v.gpu_temp_color, th().gpu, v.gpu.iter().collect()),
        ("MEMORY", format!("of {} GB", v.ram_total), v.ram_pct.clone(), "%".into(), th().text, th().ram, v.mem.iter().collect()),
    ];
    for (i, (label, sub, hero, hu, hc, cols, items)) in specs.iter().enumerate() {
        let r = (m, y, cw, card_h[i]);
        glass(p, r, 28.0 * k, cols[0]);
        circle(p, m + 22.0 * k, y + 26.0 * k, 4.5 * k, cols[0], 255);
        spaced(p, m + 34.0 * k, y + 31.0 * k, label, sc.s, W::Medium, th().muted, 2.0 * k);
        text_r(p, m + cw - 20.0 * k, y + 31.0 * k, &ellipsize(p, sub, sc.s, W::Regular, cw * 0.55), sc.s, W::Regular, th().dim);
        let hcap = 88.0 * k;
        let hb = y + 46.0 * k + hcap;
        let hw = p.text(m + 14.0 * k, hb, hero, hcap, W::Light, *hc, 255);
        p.text(m + 18.0 * k + hw, hb - hcap + sc.l, hu, sc.l, W::Light, th().muted, 255);
        // grid 3 columns
        let colw = (cw - 40.0 * k) / 3.0;
        let gy = hb + 22.0 * k;
        let rows = (items.len() + 2) / 3;
        let rh = ((y + card_h[i] - 18.0 * k) - gy) / rows as f32;
        for (j, it) in items.iter().enumerate() {
            let x = m + 20.0 * k + (j % 3) as f32 * colw;
            let yy = gy + (j / 3) as f32 * rh;
            spaced(p, x, yy + sc.s, it.label, sc.s * 0.9, W::Medium, th().dim, 1.3 * k);
            let cap = fit_vu(p, &it.value, &it.unit, sc.l * 0.88, colw - 6.0 * k, W::Medium);
            value_unit(p, x, yy + sc.s + 8.0 * k + sc.l * 0.88, &it.value, &it.unit, cap, W::Medium, it.color);
        }
        y += card_h[i] + 14.0 * k;
    }
    // now playing
    let mh = h - m - y;
    glass(p, (m, y, cw, mh), 28.0 * k, th().net[0]);
    match track_lines(d) {
        None => {
            music_note(p, w / 2.0, y + mh / 2.0 - 8.0 * k, 50.0 * k, th().dim);
            text_c(p, w / 2.0, y + mh / 2.0 + 56.0 * k, "Nothing playing", sc.m, W::Regular, th().dim);
        }
        Some((title, artist)) => {
            let tall = mh > 250.0 * k;
            if tall {
                // vertical card: big cover, centred text, progress bar
                let cs = (mh - 170.0 * k).min(cw - 60.0 * k).floor();
                spaced(p, m + 24.0 * k, y + 34.0 * k, "NOW PLAYING", sc.s * 0.9, W::Medium, th().dim, 1.6 * k);
                cover_or_note(p, cover, w / 2.0 - cs / 2.0, y + 50.0 * k, cs, 22.0 * k, th().muted, ((40, 42, 52), (24, 25, 32)));
                let ty = y + 50.0 * k + cs + 34.0 * k;
                let tw = cw - 48.0 * k;
                title_line(p, m + 24.0 * k, ty, tw, &title, sc.m * 1.3, W::Medium, th().text);
                p.text(m + 24.0 * k, ty + 26.0 * k, &ellipsize(p, &artist, sc.m, W::Regular, tw), sc.m, W::Regular, th().muted, 255);
                if let Some((el, tot, f)) = track_times(d) {
                    let by = y + mh - 36.0 * k;
                    p.layer.fill_rrect((m + 24.0 * k, by, tw, 5.0 * k), 2.5 * k, &|_| WHITE, 34);
                    p.layer.fill_rrect((m + 24.0 * k, by, (tw * f).max(5.0 * k), 5.0 * k), 2.5 * k, &|_| WHITE, 240);
                    p.text(m + 24.0 * k, by + 24.0 * k, &el, sc.s, W::Medium, th().text, 255);
                    text_r(p, m + 24.0 * k + tw, by + 24.0 * k, &tot, sc.s, W::Medium, th().dim);
                }
            } else {
                let cs = (mh - 40.0 * k).min(cw * 0.42).floor();
                cover_or_note(p, cover, m + 20.0 * k, y + 20.0 * k, cs, 18.0 * k, th().muted, ((40, 42, 52), (24, 25, 32)));
                let tx = m + 20.0 * k + cs + 16.0 * k;
                let tw = m + cw - 20.0 * k - tx;
                spaced(p, tx, y + 36.0 * k, "NOW PLAYING", sc.s * 0.9, W::Medium, th().dim, 1.6 * k);
                title_line(p, tx, y + 72.0 * k, tw, &title, sc.m * 1.2, W::Medium, th().text);
                p.text(tx, y + 98.0 * k, &ellipsize(p, &artist, sc.m, W::Regular, tw), sc.m, W::Regular, th().muted, 255);
                if let Some((el, tot, f)) = track_times(d) {
                    let by = y + mh - 34.0 * k;
                    p.layer.fill_rrect((tx, by, tw, 5.0 * k), 2.5 * k, &|_| WHITE, 34);
                    p.layer.fill_rrect((tx, by, (tw * f).max(5.0 * k), 5.0 * k), 2.5 * k, &|_| WHITE, 240);
                    p.text(tx, by - 10.0 * k, &el, sc.s, W::Medium, th().text, 255);
                    text_r(p, tx + tw, by - 10.0 * k, &tot, sc.s, W::Medium, th().dim);
                }
            }
        }
    }
}

// -------------------------------------------------------------- EQUALIZER
//
// Hi-fi LED level meters: six tall columns for the main readings, a table for the rest.

fn equalizer(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, cover: Option<&Cover>) {
    let k = sc.k;
    let panel_c = lerp_rgb(th().bg, WHITE, 0.05);
    let m = 24.0 * k;
    let cw = w - 2.0 * m;
    // header: clock + date + weather
    let tcap = p.fit(&v.hhmm, 104.0 * k, cw * 0.74, W::Light);
    let tw0 = p.text(m - 4.0 * k, m + 6.0 * k + tcap, &v.hhmm, tcap, W::Light, th().text, 255);
    let secs = format!(":{:02}", v.secs);
    p.text(m + tw0 + 6.0 * k, m + 6.0 * k + tcap, &secs, sc.l, W::Light, th().dim, 255);
    let dl = format!("{}  \u{b7}  {}", v.wd.to_uppercase(), v.date);
    spaced(p, m, m + 6.0 * k + tcap + 32.0 * k, &dl, sc.m, W::Medium, th().muted, 1.6 * k);
    let mut y = m + 6.0 * k + tcap + 50.0 * k;
    if let Some(wx) = &v.wx {
        weather_icon(p, m - 3.0 * k, y, 46.0 * k, wx.icon, wx.night);
        let tw = p.text(m + 50.0 * k, y + 23.0 * k + sc.l / 2.0, &wx.temp, sc.l, W::Medium, th().text, 255);
        let line = if wx.city.is_empty() { wx.cond.clone() } else { format!("{} \u{b7} {}", wx.cond, wx.city) };
        p.text(m + 60.0 * k + tw, y + 23.0 * k + sc.m / 2.0, &ellipsize(p, &line, sc.m, W::Regular, cw - 70.0 * k - tw), sc.m, W::Regular, th().muted, 255);
        y += 60.0 * k;
        weather_days_row(p, sc, m, y + 32.0 * k, cw, &wx.days, th().dim, th().text, th().dim);
        y += 50.0 * k;
    }
    // meter bank
    let by = y + 14.0 * k;
    let bank_h = 860.0 * k;
    p.layer.fill_rrect_solid((m.round(), by.round(), cw.round(), (bank_h + 120.0 * k).round()), 20.0 * k, panel_c, 255);
    p.layer.stroke_rrect((m.round(), by.round(), cw.round(), (bank_h + 120.0 * k).round()), 20.0 * k, 1.0, WHITE, 22);
    let vram = v.vram_frac;
    let meters: [(&str, Option<f32>, [Rgb; 2], String, &str); 6] = [
        ("CPU", v.cpu_temp_f, th().cpu, v.cpu_temp.clone(), "\u{b0}C"),
        ("LOAD", Some(v.cpu_frac), th().cpu, v.cpu_load.clone(), "%"),
        ("GPU", v.gpu_temp_f, th().gpu, v.gpu_temp.clone(), "\u{b0}C"),
        ("LOAD", v.gpu_frac, th().gpu, v.gpu_load.clone(), "%"),
        ("RAM", Some(v.ram_frac), th().ram, v.ram_pct.clone(), "%"),
        ("VRAM", vram, th().net, v.gpu[5].value.clone(), "GB"),
    ];
    let cols = 6.0;
    let colw = (cw - 24.0 * k) / cols;
    let segs = 40usize;
    let seg_gap = 5.0 * k;
    let seg_h = (bank_h - 40.0 * k - seg_gap * (segs as f32 - 1.0)) / segs as f32;
    for (i, (label, f, colors, val, unit)) in meters.iter().enumerate() {
        let x = m + 12.0 * k + i as f32 * colw;
        let bw = colw - 8.0 * k;
        let lit = (f.unwrap_or(0.0).clamp(0.0, 1.0) * segs as f32).round() as usize;
        let top = by + 20.0 * k;
        for sidx in 0..segs {
            let yy = top + (segs - 1 - sidx) as f32 * (seg_h + seg_gap);
            let t = sidx as f32 / segs as f32;
            let c = if t > 0.92 { HOT } else if t > 0.8 { WARN } else { lerp_rgb(colors[0], colors[1], t / 0.8) };
            if sidx < lit {
                rect(p, (x, yy, bw, seg_h), c, 255);
            } else {
                rect(p, (x, yy, bw, seg_h), WHITE, 18);
            }
        }
        let ly = by + bank_h - 6.0 * k;
        text_c(p, x + bw / 2.0, ly + 10.0 * k, label, sc.s * 0.85, W::Medium, th().dim);
        let vcap = fit_vu(p, val, unit, sc.l * 0.86, bw + 2.0 * k, W::Medium);
        let vw = value_unit_width(p, val, unit, vcap, W::Medium);
        value_unit(p, x + bw / 2.0 - vw / 2.0, ly + 48.0 * k, val, unit, vcap, W::Medium, th().text);
    }
    // tables
    let mut ty = by + bank_h + 120.0 * k + 18.0 * k;
    let groups: [(&str, Rgb, Vec<&Item>); 3] = [
        ("CPU", th().cpu[0], vec![&v.cpu[1], &v.cpu[2], &v.cpu[3], &v.cpu[4], &v.cpu[5], &v.cpu[6]]),
        ("GPU", th().gpu[0], vec![&v.gpu[0], &v.gpu[2], &v.gpu[3], &v.gpu[4], &v.gpu[6]]),
        ("MEMORY", th().ram[0], vec![&v.mem[0], &v.mem[1], &v.mem[2], &v.mem[3], &v.mem[4], &v.mem[5]]),
    ];
    let colw2 = (cw - 16.0 * k) / 2.0;
    for (name, c, items) in groups.iter() {
        circle(p, m + 5.0 * k, ty + 8.0 * k, 4.5 * k, *c, 255);
        spaced(p, m + 18.0 * k, ty + 13.0 * k, name, sc.s, W::Medium, th().muted, 2.0 * k);
        capsule(p, (m + 18.0 * k + spaced_width(p, name, sc.s, W::Medium, 2.0 * k) + 12.0 * k, ty + 8.0 * k), (m + cw, ty + 8.0 * k), 0.5 * k, WHITE, 34);
        ty += 28.0 * k;
        let rows = (items.len() + 1) / 2;
        for (j, it) in items.iter().enumerate() {
            let x0 = m + (j % 2) as f32 * (colw2 + 16.0 * k);
            let base = ty + (j / 2) as f32 * 38.0 * k + 22.0 * k;
            kv_row(p, x0, x0 + colw2, base, it.label, &it.value, &it.unit, sc.m * 1.2, th().dim, it.color, th().muted, WHITE, k);
        }
        ty += rows as f32 * 38.0 * k + 22.0 * k;
    }
    // now playing
    let my = ty;
    let mh = h - m - my;
    capsule(p, (m, my - 8.0 * k), (m + cw, my - 8.0 * k), 0.5 * k, WHITE, 34);
    let big = Scale { m: sc.m * 1.25, s: sc.s * 1.15, ..*sc };
    now_playing_compact(p, (m, my, cw, mh.min(96.0 * k)), &big, d, cover, true);
}

// ----------------------------------------------------------------- RECEIPT
//
// A paper receipt: black ink, dotted leaders, a halftone cover and a barcode.

fn receipt(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, cover: Option<&Cover>) {
    let k = sc.k;
    let paper = (238, 234, 224);
    let ink = (28, 28, 32);
    let faint = (122, 118, 108);
    let m = 14.0 * k;
    let (px0, px1) = (m, w - m);
    let (py0, py1) = (18.0 * k, h - 18.0 * k);
    // paper body with a zig-zag top and bottom edge
    rect(p, (px0, py0 + 10.0 * k, px1 - px0, py1 - py0 - 20.0 * k), paper, 255);
    let tooth = 14.0 * k;
    let n = ((px1 - px0) / tooth).ceil() as i32;
    for i in 0..n {
        let x = px0 + i as f32 * tooth;
        poly(p, &[(x, py0 + 10.0 * k), (x + tooth / 2.0, py0), (x + tooth, py0 + 10.0 * k)], paper, 255);
        poly(p, &[(x, py1 - 10.0 * k), (x + tooth / 2.0, py1), (x + tooth, py1 - 10.0 * k)], paper, 255);
    }
    let x0 = px0 + 24.0 * k;
    let x1 = px1 - 24.0 * k;
    let cwid = x1 - x0;
    let acc = lerp_rgb(th().cpu[0], INK, 0.25);
    let dash = |p: &mut Painter, y: f32| {
        let mut x = x0;
        while x < x1 {
            rect(p, (x, y, 7.0 * k, 1.5 * k), faint, 255);
            x += 12.0 * k;
        }
    };
    let mut y = py0 + 40.0 * k;
    // header
    text_c(p, w / 2.0, y + 30.0 * k, "TROFEO LCD", 40.0 * k, W::Medium, ink);
    y += 44.0 * k;
    let sub = "SYSTEM STATUS RECEIPT";
    let sw = spaced_width(p, sub, sc.s, W::Medium, 3.0 * k);
    spaced(p, w / 2.0 - sw / 2.0, y + 14.0 * k, sub, sc.s, W::Medium, faint, 3.0 * k);
    y += 34.0 * k;
    dash(p, y);
    y += 20.0 * k;
    // date / time
    p.text(x0, y + 22.0 * k, &format!("{} {}", v.wd.chars().take(3).collect::<String>(), v.date.to_uppercase()), sc.m * 1.1, W::Medium, ink, 255);
    y += 36.0 * k;
    let tcap = p.fit(&v.hhmm, 92.0 * k, cwid * 0.8, W::Medium);
    let tw = p.text(x0 - 2.0 * k, y + tcap, &v.hhmm, tcap, W::Medium, ink, 255);
    p.text(x0 + tw + 8.0 * k, y + tcap, &format!(":{:02}", v.secs), sc.l, W::Regular, faint, 255);
    y += tcap + 22.0 * k;
    if let Some(wx) = &v.wx {
        dash(p, y);
        y += 14.0 * k;
        // weather icon on a dark chip so its colours stay readable
        p.layer.fill_rrect_solid(((x1 - 62.0 * k).round(), y.round(), (62.0 * k).round(), (62.0 * k).round()), 10.0 * k, INK, 255);
        weather_icon(p, x1 - 59.0 * k, y + 1.0 * k, 56.0 * k, wx.icon, wx.night);
        p.text(x0, y + 24.0 * k, &format!("WEATHER  {}", wx.temp), sc.m * 1.15, W::Medium, ink, 255);
        let line = if wx.city.is_empty() { wx.cond.to_uppercase() } else { format!("{} \u{b7} {}", wx.cond.to_uppercase(), wx.city.to_uppercase()) };
        p.text(x0, y + 46.0 * k, &ellipsize(p, &line, sc.s, W::Regular, cwid - 76.0 * k), sc.s, W::Regular, faint, 255);
        y += 66.0 * k;
        let days: Vec<String> = wx.days.iter().map(|f| format!("{} {}/{}", f.name, f.hi, f.lo)).collect();
        p.text(x0, y + 14.0 * k, &days.join("   "), sc.s, W::Regular, ink, 255);
        y += 28.0 * k;
    }
    dash(p, y);
    y += 8.0 * k;
    let sections: [(&str, String, String, Rgb, Vec<(&str, String, String, Rgb)>); 3] = [
        (
            "CPU",
            v.cpu_name.clone(),
            "".into(),
            acc,
            std::iter::once(("TEMP", v.cpu_temp.clone(), "\u{b0}C".to_string(), v.cpu_temp_color))
                .chain(v.cpu.iter().map(|i| (i.label, i.value.clone(), i.unit.clone(), i.color)))
                .collect(),
        ),
        (
            "GPU",
            v.gpu_name.clone(),
            "".into(),
            acc,
            std::iter::once(("TEMP", v.gpu_temp.clone(), "\u{b0}C".to_string(), v.gpu_temp_color))
                .chain(v.gpu.iter().map(|i| (i.label, i.value.clone(), i.unit.clone(), i.color)))
                .collect(),
        ),
        (
            "MEMORY",
            format!("{} GB TOTAL", v.ram_total),
            "".into(),
            acc,
            std::iter::once(("RAM", v.ram_pct.clone(), "%".to_string(), th().text))
                .chain(v.mem.iter().map(|i| (i.label, i.value.clone(), i.unit.clone(), i.color)))
                .collect(),
        ),
    ];
    let ink_of = |c: Rgb| if c == th().text { ink } else if c == th().fps { (150, 100, 0) } else { lerp_rgb(c, INK, 0.35) };
    for (name, sub, _, c, rows) in sections.iter() {
        y += 14.0 * k;
        // section tag
        let tw = spaced_width(p, name, sc.s * 1.05, W::Medium, 2.4 * k);
        p.layer.fill_rrect_solid((x0.round(), y.round(), (tw + 18.0 * k).round(), (24.0 * k).round()), 4.0 * k, ink, 255);
        spaced(p, x0 + 9.0 * k, y + 17.0 * k, name, sc.s * 1.05, W::Medium, paper, 2.4 * k);
        let _ = c;
        text_r(p, x1, y + 17.0 * k, &ellipsize(p, sub, sc.s, W::Regular, cwid - tw - 30.0 * k), sc.s, W::Regular, faint);
        y += 38.0 * k;
        for (label, val, unit, col) in rows {
            let label = if *label == "DOWN" { "NET DOWN" } else if *label == "UP" { "NET UP" } else { *label };
            kv_row(p, x0, x1, y + 22.0 * k, label, val, unit, sc.m * 1.3, faint, ink_of(*col), faint, ink, k);
            y += 42.0 * k;
        }
    }
    dash(p, y + 8.0 * k);
    y += 24.0 * k;
    // now playing: halftone cover + text
    let cs = 156.0 * k;
    match (track_lines(d), cover) {
        (Some((title, artist)), cv) => {
            halftone(p, cv, x0, y, cs, ink);
            let tx = x0 + cs + 14.0 * k;
            let tw = x1 - tx;
            p.text(tx, y + 16.0 * k, "NOW PLAYING", sc.s, W::Medium, faint, 255);
            title_line(p, tx, y + 48.0 * k, tw, &title, sc.m * 1.2, W::Medium, ink);
            p.text(tx, y + 72.0 * k, &ellipsize(p, &artist, sc.m, W::Regular, tw), sc.m, W::Regular, faint, 255);
            if let Some((el, tot, f)) = track_times(d) {
                let by = y + cs - 28.0 * k;
                p.text(tx, by + 22.0 * k, &format!("{el} / {tot}"), sc.s, W::Medium, ink, 255);
                p.layer.fill_rrect_solid((tx.round(), (by - 6.0 * k).round(), tw.round(), (5.0 * k).round()), 0.0, faint, 255);
                rect(p, (tx.round(), (by - 6.0 * k).round(), (tw * f).max(5.0 * k).round(), (5.0 * k).round()), ink, 255);
            }
        }
        (None, _) => {
            p.text(x0, y + 24.0 * k, "NOW PLAYING: --", sc.m, W::Medium, faint, 255);
        }
    }
    y += cs + 18.0 * k;
    dash(p, y);
    // barcode (decorative: seeded by the time)
    let seed = v.hhmm.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32)) ^ v.secs;
    let mut s = seed | 1;
    let mut bx = x0 + 18.0 * k;
    let by = h - py0 - 108.0 * k;
    while bx < x1 - 18.0 * k {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        let bw = (1.0 + (s % 4) as f32) * k * 1.2;
        if (s >> 8) % 3 != 0 {
            rect(p, (bx, by, bw, 50.0 * k), ink, 255);
        }
        bx += bw + 1.6 * k;
    }
    text_c(p, w / 2.0, by + 72.0 * k, &format!("NO. {}-{:04}   THANK YOU", v.hhmm.replace(':', ""), v.secs), sc.s * 0.9, W::Regular, faint);
}

/// A photo as black halftone dots on paper (thermal-printer look).
fn halftone(p: &mut Painter, cover: Option<&Cover>, x: f32, y: f32, s: f32, ink: Rgb) {
    let n = 44usize;
    let cell = s / n as f32;
    rect(p, (x, y, s, s), (226, 222, 211), 255);
    let Some(c) = cover else {
        music_note(p, x + s / 2.0, y + s / 2.0, s * 0.35, ink);
        return;
    };
    for j in 0..n {
        for i in 0..n {
            let sx = ((i as f32 + 0.5) / n as f32 * c.size as f32) as u32;
            let sy = ((j as f32 + 0.5) / n as f32 * c.size as f32) as u32;
            let idx = ((sy.min(c.size - 1) * c.size + sx.min(c.size - 1)) * 3) as usize;
            let lum = (0.299 * c.rgb[idx] as f32 + 0.587 * c.rgb[idx + 1] as f32 + 0.114 * c.rgb[idx + 2] as f32) / 255.0;
            let dark = (1.0 - lum).powf(0.8);
            let r = cell * 0.5 * dark.sqrt() * 1.05;
            if r > 0.35 {
                circle(p, x + (i as f32 + 0.5) * cell, y + (j as f32 + 0.5) * cell, r, ink, 255);
            }
        }
    }
}

// --------------------------------------------------------------- SYNTHWAVE
//
// A retro sunset: striped sun on the horizon, a perspective grid floor, neon text.

fn synthwave(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, cover: Option<&Cover>) {
    let k = sc.k;
    let n_rows = [v.cpu.len(), v.gpu.len(), v.mem.len()];
    let panels_h: f32 = n_rows.iter().map(|n| 126.0 * k + *n as f32 * 31.0 * k + 12.0 * k).sum();
    let hor = (h - 20.0 * k - 124.0 * k - 12.0 * k - panels_h - 30.0 * k).max(520.0 * k);
    let neon_a = th().cpu[0];
    let neon_b = th().gpu[0];
    let key = format!("synth{:?}{:?}{:?}{}", th().cpu, th().gpu, th().bg, h as i32);
    cached_bg(p, key, &|q| {
        // sky: dark at the top, glowing at the horizon
        let top = lerp_rgb((4, 2, 18), th().bg, 0.3);
        for yy in 0..(hor as i32) {
            let t = yy as f32 / hor;
            let c = gradient_at(&[top, lerp_rgb(top, neon_a, 0.28), lerp_rgb(neon_a, neon_b, 0.5)], t.powf(1.6));
            q.layer.span_over(yy, 0, w as i32, c, 255);
        }
        // stars
        let mut s = 0x9E3779B9u32;
        for _ in 0..70 {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            let sx = (s % (w as u32)) as f32;
            let sy = ((s >> 9) % (hor as u32 * 7 / 10)) as f32;
            circle(q, sx, sy, 0.9 * k, WHITE, 120 + (s >> 20) as u8 % 100);
        }
        // sun
        let (cx, cy, r) = (w / 2.0, hor, 168.0 * k);
        for yy in ((cy - r) as i32)..(cy as i32) {
            let t = (yy as f32 - (cy - r)) / r;
            // stripes get thicker towards the horizon
            let stripe = ((yy as f32 - cy + r) / (7.0 * k)) as i32;
            let cut = t > 0.28 && (((yy as f32 - cy) / (9.0 * k)).abs().fract() < (0.12 + t * 0.55));
            let _ = stripe;
            if cut {
                continue;
            }
            let c = gradient_at(&[(255, 226, 90), (255, 120, 120), lerp_rgb(neon_a, WHITE, 0.2)], t);
            let half = (r * r - (yy as f32 + 0.5 - cy).powi(2)).max(0.0).sqrt();
            q.layer.span_over(yy, (cx - half) as i32, (cx + half) as i32, c, 255);
        }
        // floor
        for yy in (hor as i32)..(h as i32) {
            let t = (yy as f32 - hor) / (h - hor);
            q.layer.span_over(yy, 0, w as i32, lerp_rgb((10, 4, 28), lerp_rgb(th().bg, (24, 6, 40), 0.7), t), 255);
        }
        // grid
        let gc = lerp_rgb(neon_a, neon_b, 0.35);
        for i in -18..=18 {
            let bx = cx + i as f32 * 120.0 * k;
            capsule(q, (cx + (bx - cx) * 0.02, hor), (bx, h), 0.9 * k, gc, 110);
        }
        let n = 30;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let yy = hor + (h - hor) * t * t;
            capsule(q, (0.0, yy), (w, yy), (0.6 + t) * k, gc, (60.0 + 90.0 * t) as u8);
        }
        // glow along the horizon
        for i in 0..14 {
            let a = (50 - i * 3).max(0) as u8;
            q.layer.span_over(hor as i32 - i, 0, w as i32, neon_a, a);
        }
    });
    let hi = lerp_rgb(neon_b, WHITE, 0.55);
    // clock + date (neon)
    let dl = format!("{}  \u{b7}  {}", v.wd, v.date.to_uppercase());
    let dw = spaced_width(p, &dl, sc.s * 1.1, W::Medium, 3.0 * k);
    spaced(p, w / 2.0 - dw / 2.0, 54.0 * k, &dl, sc.s * 1.1, W::Medium, hi, 3.0 * k);
    let tcap = p.fit(&v.hhmm, 150.0 * k, w - 40.0 * k, W::Light);
    let tw = p.width(&v.hhmm, tcap, W::Light);
    glow_text(p, w / 2.0 - tw / 2.0, 80.0 * k + tcap, &v.hhmm, tcap, W::Light, WHITE, 60);
    if let Some(wx) = &v.wx {
        let y = 80.0 * k + tcap + 36.0 * k;
        let tw1 = p.width(&wx.temp, sc.xl, W::Medium);
        let cwid = 168.0 * k;
        let gw = 60.0 * k + tw1 + 14.0 * k + cwid;
        let xs = (w / 2.0 - gw / 2.0).max(14.0 * k);
        weather_icon(p, xs, y - 8.0 * k, 52.0 * k, wx.icon, wx.night);
        p.text(xs + 60.0 * k, y + 34.0 * k, &wx.temp, sc.xl, W::Medium, WHITE, 255);
        let lines = [wx.cond.clone(), wx.city.clone()];
        for (i, l) in lines.iter().enumerate() {
            p.text(xs + 74.0 * k + tw1, y + 14.0 * k + i as f32 * 22.0 * k, &ellipsize(p, l, sc.m, W::Regular, cwid), sc.m, W::Regular, hi, 255);
        }
        let dy = y + 92.0 * k;
        weather_days_row(p, sc, 34.0 * k, dy + 30.0 * k, w - 68.0 * k, &wx.days, hi, WHITE, lerp_rgb(hi, (60, 20, 90), 0.4));
    }
    // panels on the grid floor
    let m = 20.0 * k;
    let cw = w - 2.0 * m;
    let mut y = hor + 30.0 * k;
    let specs: [(&str, String, String, String, Rgb, Rgb, Vec<&Item>); 3] = [
        ("CPU", v.cpu_name.clone(), v.cpu_temp.clone(), "\u{b0}C".into(), v.cpu_temp_color, neon_a, v.cpu.iter().collect()),
        ("GPU", v.gpu_name.clone(), v.gpu_temp.clone(), "\u{b0}C".into(), v.gpu_temp_color, neon_b, v.gpu.iter().collect()),
        ("MEMORY", format!("of {} GB", v.ram_total), v.ram_pct.clone(), "%".into(), th().text, th().ram[0], v.mem.iter().collect()),
    ];
    for (name, sub, hero, hu, hc, c, items) in specs.iter() {
        let n = items.len();
        let ph = 126.0 * k + n as f32 * 31.0 * k;
        let r = (m, y, cw, ph);
        p.layer.fill_rrect(r, 14.0 * k, &|_| (8, 3, 22), 205);
        p.layer.stroke_rrect(r, 14.0 * k, 1.6 * k, *c, 220);
        p.layer.stroke_rrect((r.0 - 2.0 * k, r.1 - 2.0 * k, r.2 + 4.0 * k, r.3 + 4.0 * k), 16.0 * k, 3.0 * k, *c, 40);
        glow_text(p, m + 18.0 * k, y + 28.0 * k, name, sc.s * 1.15, W::Medium, lerp_rgb(*c, WHITE, 0.4), 40);
        text_r(p, m + cw - 18.0 * k, y + 28.0 * k, &ellipsize(p, sub, sc.s, W::Regular, cw * 0.55), sc.s, W::Regular, hi);
        let hcap = 56.0 * k;
        let hb = y + 40.0 * k + hcap;
        let hw = glow_text(p, m + 16.0 * k, hb, hero, hcap, W::Medium, if *hc == th().text { WHITE } else { *hc }, 50);
        p.text(m + 20.0 * k + hw, hb - hcap + sc.l * 0.9, hu, sc.l * 0.9, W::Regular, hi, 255);
        for (j, it) in items.iter().enumerate() {
            let base = hb + 22.0 * k + (j as f32 + 0.8) * 31.0 * k;
            let label = if it.label == "DOWN" { "NET DOWN" } else if it.label == "UP" { "NET UP" } else { it.label };
            kv_row(p, m + 18.0 * k, m + cw - 18.0 * k, base, label, &it.value, &it.unit, sc.m * 1.15, lerp_rgb(hi, (40, 20, 70), 0.35), if it.color == th().text { WHITE } else { it.color }, hi, hi, k);
        }
        y += ph + 12.0 * k;
    }
    // now playing
    let mh = (h - m - y).clamp(80.0 * k, 124.0 * k);
    let r = (m, y, cw, mh);
    p.layer.fill_rrect(r, 14.0 * k, &|_| (8, 3, 22), 205);
    p.layer.stroke_rrect(r, 14.0 * k, 1.6 * k, th().net[0], 220);
    let big = Scale { m: sc.m * 1.2, s: sc.s * 1.1, ..*sc };
    now_playing_compact(p, (m + 14.0 * k, y + 14.0 * k, cw - 28.0 * k, mh - 28.0 * k), &big, d, cover, true);
}

// --------------------------------------------------------------- BLUEPRINT
//
// A technical drawing: blue paper, white line work, dimension marks, a title block.

fn blueprint(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, cover: Option<&Cover>) {
    let k = sc.k;
    let paper = (11, 52, 120);
    let line = (206, 226, 255);
    let soft = lerp_rgb(paper, line, 0.55);
    let accent = th().fps;
    let key = format!("bp{}", h as i32);
    cached_bg(p, key, &|q| {
        rect(q, (0.0, 0.0, w, h), (11, 52, 120), 255);
        let mut x = 0.0;
        let mut i = 0;
        while x <= w {
            let major = i % 5 == 0;
            capsule(q, (x, 0.0), (x, h), if major { 0.8 } else { 0.5 }, (206, 226, 255), if major { 38 } else { 16 });
            x += 24.0 * k;
            i += 1;
        }
        let mut y = 0.0;
        i = 0;
        while y <= h {
            let major = i % 5 == 0;
            capsule(q, (0.0, y), (w, y), if major { 0.8 } else { 0.5 }, (206, 226, 255), if major { 38 } else { 16 });
            y += 24.0 * k;
            i += 1;
        }
    });
    let m = 18.0 * k;
    // frame (double line) with corner ticks
    p.layer.stroke_rrect((m, m, w - 2.0 * m, h - 2.0 * m), 0.0, 2.0 * k, line, 230);
    p.layer.stroke_rrect((m + 7.0 * k, m + 7.0 * k, w - 2.0 * m - 14.0 * k, h - 2.0 * m - 14.0 * k), 0.0, 1.0, line, 120);
    let x0 = m + 24.0 * k;
    let x1 = w - m - 24.0 * k;
    let cwid = x1 - x0;
    // dimension line across the top
    let dim = |p: &mut Painter, xa: f32, xb: f32, y: f32, label: &str| {
        capsule(p, (xa, y), (xb, y), 0.8 * k, soft, 220);
        capsule(p, (xa, y - 7.0 * k), (xa, y + 7.0 * k), 0.8 * k, soft, 220);
        capsule(p, (xb, y - 7.0 * k), (xb, y + 7.0 * k), 0.8 * k, soft, 220);
        let tw = p.width(label, sc.s * 0.9, W::Regular);
        rect(p, ((xa + xb) / 2.0 - tw / 2.0 - 6.0 * k, y - 8.0 * k, tw + 12.0 * k, 16.0 * k), (11, 52, 120), 255);
        text_c(p, (xa + xb) / 2.0, y + sc.s * 0.33, label, sc.s * 0.9, W::Regular, soft);
    };
    let mut y = m + 40.0 * k;
    // clock
    let tcap = p.fit(&v.hhmm, 112.0 * k, cwid * 0.74, W::Light);
    let tw = p.text(x0 - 3.0 * k, y + tcap, &v.hhmm, tcap, W::Light, WHITE, 255);
    p.text(x0 + tw + 8.0 * k, y + tcap, &format!(":{:02}", v.secs), sc.l, W::Light, soft, 255);
    spaced(p, x0, y + tcap + 30.0 * k, &format!("{}  \u{b7}  {}", v.wd.to_uppercase(), v.date), sc.m, W::Medium, line, 1.6 * k);
    y += tcap + 52.0 * k;
    dim(p, x0, x1, y, "462 mm");
    y += 26.0 * k;
    if let Some(wx) = &v.wx {
        // weather as a boxed note
        let r = (x0, y, cwid, 166.0 * k);
        p.layer.stroke_rrect((r.0.round(), r.1.round(), r.2.round(), r.3.round()), 0.0, 1.2 * k, line, 200);
        weather_icon(p, x0 + 12.0 * k, y + 12.0 * k, 50.0 * k, wx.icon, wx.night);
        p.text(x0 + 70.0 * k, y + 62.0 * k, &wx.temp, sc.xl * 0.95, W::Medium, WHITE, 255);
        let l2 = if wx.city.is_empty() { wx.cond.clone() } else { format!("{} \u{b7} {}", wx.cond, wx.city) };
        p.text(x0 + 12.0 * k, y + 92.0 * k, &ellipsize(p, &l2, sc.m, W::Regular, cwid - 24.0 * k), sc.m, W::Regular, line, 255);
        weather_days_row(p, sc, x0 + 12.0 * k, y + 152.0 * k, cwid - 24.0 * k, &wx.days, soft, WHITE, soft);
        y += 166.0 * k + 18.0 * k;
    }
    // three "parts"
    let tbh = 104.0 * k;
    let n_all = (v.cpu.len() + v.gpu.len() + v.mem.len()) as f32;
    let avail = (h - m - tbh) - y - 120.0 * k - 26.0 * k;
    let pitch = ((avail - 3.0 * (150.0 * k + 14.0 * k)) / n_all).clamp(31.0 * k, 44.0 * k);
    let specs: [(&str, &str, String, String, String, Rgb, Vec<&Item>); 3] = [
        ("1", "CPU", v.cpu_name.clone(), v.cpu_temp.clone(), "\u{b0}C".into(), v.cpu_temp_color, v.cpu.iter().collect()),
        ("2", "GPU", v.gpu_name.clone(), v.gpu_temp.clone(), "\u{b0}C".into(), v.gpu_temp_color, v.gpu.iter().collect()),
        ("3", "MEMORY", format!("{} GB", v.ram_total), v.ram_pct.clone(), "%".into(), WHITE, v.mem.iter().collect()),
    ];
    for (num, name, sub, hero, hu, hc, items) in specs.iter() {
        let n = items.len();
        let ph = 150.0 * k + n as f32 * pitch;
        let r = (x0, y, cwid, ph);
        p.layer.stroke_rrect((r.0.round(), r.1.round(), r.2.round(), r.3.round()), 0.0, 1.2 * k, line, 210);
        // number bubble
        circle(p, x0 + 24.0 * k, y + 24.0 * k, 14.0 * k, paper, 255);
        p.layer.arc(x0 + 24.0 * k, y + 24.0 * k, 14.0 * k, 1.6 * k, 1.0, &|_| (206, 226, 255), 255);
        text_c(p, x0 + 24.0 * k, y + 24.0 * k + sc.s * 0.36, num, sc.s, W::Medium, accent);
        spaced(p, x0 + 46.0 * k, y + 29.0 * k, name, sc.s * 1.1, W::Medium, WHITE, 2.4 * k);
        text_r(p, x1 - 12.0 * k, y + 29.0 * k, &ellipsize(p, sub, sc.s, W::Regular, cwid * 0.5), sc.s, W::Regular, soft);
        // hero in a gauge-sight circle
        let (cx, cy, cr) = (x0 + 70.0 * k, y + 94.0 * k, 46.0 * k);
        p.layer.arc(cx, cy, cr, 1.4 * k, 1.0, &|_| (206, 226, 255), 230);
        capsule(p, (cx - cr - 10.0 * k, cy), (cx - cr + 12.0 * k, cy), 0.7 * k, soft, 200);
        capsule(p, (cx + cr - 12.0 * k, cy), (cx + cr + 10.0 * k, cy), 0.7 * k, soft, 200);
        capsule(p, (cx, cy - cr - 10.0 * k), (cx, cy - cr + 12.0 * k), 0.7 * k, soft, 200);
        capsule(p, (cx, cy + cr - 12.0 * k), (cx, cy + cr + 10.0 * k), 0.7 * k, soft, 200);
        let vc = fit_vu(p, hero, hu, 50.0 * k, cr * 1.5, W::Medium);
        let vw = value_unit_width(p, hero, hu, vc, W::Medium);
        value_unit(p, cx - vw / 2.0, cy + vc / 2.0, hero, hu, vc, W::Medium, if *hc == WHITE { WHITE } else { *hc });
        // first three readings beside the circle, the rest below
        for (j, it) in items.iter().enumerate() {
            let label = if it.label == "DOWN" { "NET DOWN" } else if it.label == "UP" { "NET UP" } else { it.label };
            let row_y = y + 142.0 * k + j as f32 * pitch;
            kv_row(p, x0 + 14.0 * k, x1 - 12.0 * k, row_y + 20.0 * k, label, &it.value, &it.unit, sc.m * 1.25, soft, if it.color == th().text { WHITE } else { it.color }, soft, line, k);
        }
        y += ph + 14.0 * k;
    }
    // now playing as a labelled detail
    let mh = 120.0 * k;
    if mh > 60.0 * k {
        let big = Scale { m: sc.m * 1.2, s: sc.s * 1.1, ..*sc };
        p.layer.stroke_rrect((x0.round(), y.round(), cwid.round(), mh.round()), 0.0, 1.2 * k, line, 210);
        now_playing_compact(p, (x0 + 12.0 * k, y + 12.0 * k, cwid - 24.0 * k, (mh - 24.0 * k).min(100.0 * k)), &big, d, cover, true);
    }
    // title block
    let ty = h - m - tbh;
    p.layer.stroke_rrect((x0.round(), ty.round(), cwid.round(), (tbh - 12.0 * k).round()), 0.0, 1.5 * k, line, 230);
    capsule(p, (x0 + cwid * 0.55, ty), (x0 + cwid * 0.55, ty + tbh - 12.0 * k), 0.8 * k, line, 200);
    capsule(p, (x0, ty + (tbh - 12.0 * k) / 2.0), (x1, ty + (tbh - 12.0 * k) / 2.0), 0.8 * k, line, 200);
    spaced(p, x0 + 10.0 * k, ty + 22.0 * k, "TROFEO LCD", sc.m, W::Medium, WHITE, 2.0 * k);
    p.text(x0 + 10.0 * k, ty + 42.0 * k, "MONITORING UNIT", sc.s * 0.9, W::Regular, soft, 255);
    p.text(x0 + 10.0 * k, ty + 74.0 * k, &format!("DWG {}", v.date.to_uppercase()), sc.s * 0.9, W::Regular, line, 255);
    let bx = x0 + cwid * 0.55 + 10.0 * k;
    p.text(bx, ty + 22.0 * k, "SCALE 1:1", sc.s * 0.9, W::Regular, soft, 255);
    p.text(bx, ty + 42.0 * k, "SHEET 1/1", sc.s * 0.9, W::Regular, soft, 255);
    p.text(bx, ty + 74.0 * k, &format!("REV {}:{:02}", v.hhmm, v.secs), sc.s * 0.9, W::Medium, accent, 255);
}
