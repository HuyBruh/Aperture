//! Minimal landscape designs: swiss and the clean collection.

use super::*;

type Hist<'a> = (&'a VecDeque<f32>, &'a VecDeque<f32>, &'a VecDeque<f32>);

#[allow(clippy::too_many_arguments)]
fn np_row(p: &mut Painter, d: &Data, cover: Option<&Cover>, k: f32, x: f32, y: f32, w: f32, cs: f32, radius: f32, ct: Rgb, ca: Rgb, cd: Rgb, bar: [Rgb; 2], bg: (Rgb, Rgb)) {
    cover_or_note(p, cover, x, y, cs, radius, cd, bg);
    let tx = x + cs + 16.0 * k;
    let tw = (w - cs - 16.0 * k).max(10.0);
    let tcap = (cs * 0.17).clamp(15.0 * k, 28.0 * k);
    match track_lines(d) {
        None => {
            p.text(tx, y + tcap * 1.6, "Nothing playing", tcap, W::Regular, cd, 255);
        }
        Some((title, artist)) => {
            title_line(p, tx, y + tcap * 1.5, tw, &title, tcap, W::Medium, ct);
            p.text(tx, y + tcap * 2.75, &ellipsize(p, &artist, tcap * 0.78, W::Regular, tw), tcap * 0.78, W::Regular, ca, 255);
            if let Some((el, tot, f)) = track_times(d) {
                let by = y + cs - 6.0 * k;
                hbar(p, tx, by, tw, 5.0 * k, Some(f), bar);
                if cs >= 84.0 * k {
                    p.text(tx, by - 9.0 * k, &el, tcap * 0.62, W::Regular, cd, 255);
                    text_r(p, tx + tw, by - 9.0 * k, &tot, tcap * 0.62, W::Regular, cd);
                }
            }
        }
    }
}

// -------------------------------------------------------------------- PAPER
//
// A newspaper front page: cream stock, ink, rules and columns.

fn swiss(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, cover: Option<&Cover>) {
    let k = sc.k;
    let bg = (238, 238, 232);
    let ink = (14, 14, 14);
    let mid = (92, 92, 90);
    let acc = if th().cpu[0] == (150, 150, 158) { (20, 20, 20) } else { th().cpu[0] };
    rect(p, (0.0, 0.0, w, h), bg, 255);
    let bw = 600.0 * k;
    rect(p, (0.0, 0.0, bw, h), acc, 255);
    let m = 36.0 * k;
    let onacc = if (acc.0 as u32 * 3 + acc.1 as u32 * 6 + acc.2 as u32) > 1750 { ink } else { (255, 255, 255) };
    p.text(m, m + 14.0 * k, &v.wd, 17.0 * k, W::Medium, onacc, 255);
    p.text(m, m + 40.0 * k, &v.date, 17.0 * k, W::Regular, onacc, 255);
    let tcap = p.fit(&v.hhmm, 168.0 * k, bw - 2.0 * m - 6.0 * k, W::Medium);
    p.text(m - 6.0 * k, 96.0 * k + tcap, &v.hhmm, tcap, W::Medium, onacc, 255);
    p.text(m, 96.0 * k + tcap + 34.0 * k, &format!(":{:02}", v.secs), 20.0 * k, W::Regular, onacc, 220);
    if let Some(wx) = &v.wx {
        let wy = h - m - 70.0 * k;
        rect(p, (m, wy - 22.0 * k, bw - 2.0 * m, 2.0), onacc, 255);
        weather_icon(p, m, wy - 6.0 * k, 64.0 * k, wx.icon, wx.night);
        p.text(m + 76.0 * k, wy + 52.0 * k, &wx.temp, 56.0 * k, W::Medium, onacc, 255);
        let c = ellipsize(p, &wx.cond, 15.0 * k, W::Medium, bw - m - 220.0 * k);
        p.text(m + 220.0 * k, wy + 26.0 * k, &c, 15.0 * k, W::Medium, onacc, 255);
        p.text(m + 220.0 * k, wy + 48.0 * k, &ellipsize(p, &wx.city, 13.0 * k, W::Regular, bw - m - 220.0 * k), 13.0 * k, W::Regular, onacc, 220);
    }
    // grid
    let gx = bw + 40.0 * k;
    let gw = w - gx - 36.0 * k;
    let cw = gw / 4.0;
    let ch = 148.0 * k;
    let cells: [(&str, String, &str, String, Rgb); 8] = [
        ("CPU TEMPERATURE", v.cpu_temp.clone(), "\u{b0}C", format!("{} W \u{b7} {} GHz", v.cpu[1].value, v.cpu[2].value), if v.cpu_temp_color == th().text { ink } else { v.cpu_temp_color }),
        ("GPU TEMPERATURE", v.gpu_temp.clone(), "\u{b0}C", format!("{} W \u{b7} {} MHz", v.gpu[2].value, v.gpu[4].value), if v.gpu_temp_color == th().text { ink } else { v.gpu_temp_color }),
        ("CPU LOAD", v.cpu_load.clone(), "%", v.cpu_name.clone(), ink),
        ("GPU LOAD", v.gpu_load.clone(), "%", v.gpu_name.clone(), ink),
        ("MEMORY", v.ram_pct.clone(), "%", format!("{} / {} GB", v.mem[0].value, v.ram_total), ink),
        ("FRAMES", v.gpu[6].value.clone(), "FPS", format!("VRAM {} GB", v.gpu[5].value), ink),
        ("STORAGE", v.mem[3].value.clone(), "%", format!("SSD {}\u{b0}C \u{b7} R {}{}", v.mem[2].value, v.mem[4].value, v.mem[4].unit), ink),
        ("NETWORK", v.net_dn.0.clone(), v.net_dn.1, format!("UP {} {}", v.net_up.0, v.net_up.1), ink),
    ];
    for (i, (label, val, unit, sub, c)) in cells.iter().enumerate() {
        let (cx, cy) = (gx + (i % 4) as f32 * cw, 34.0 * k + (i / 4) as f32 * (ch + 10.0 * k));
        let inner = cw - 20.0 * k;
        rect(p, (cx, cy, inner, 4.0 * k), ink, 255);
        p.text(cx, cy + 24.0 * k, &format!("{:02}", i + 1), 12.5 * k, W::Medium, acc, 255);
        p.text(cx + 30.0 * k, cy + 24.0 * k, label, 12.5 * k, W::Medium, mid, 255);
        let vcap = p.fit(val, 80.0 * k, inner - 52.0 * k, W::Medium);
        let vw = p.text(cx - 3.0 * k, cy + 24.0 * k + 12.0 * k + vcap, val, vcap, W::Medium, *c, 255);
        p.text(cx + vw + 5.0 * k, cy + 24.0 * k + 12.0 * k + vcap, unit, 15.0 * k, W::Regular, mid, 255);
        p.text(cx, cy + ch - 8.0 * k, &ellipsize(p, sub, 12.5 * k, W::Regular, inner), 12.5 * k, W::Regular, mid, 255);
    }
    let ny = 34.0 * k + 2.0 * (ch + 10.0 * k) + 6.0 * k;
    rect(p, (gx, ny, gw - 20.0 * k, 4.0 * k), ink, 255);
    np_row(p, d, cover, k, gx, ny + 18.0 * k, gw - 20.0 * k, h - ny - 18.0 * k - 28.0 * k, 0.0, ink, mid, mid, [acc, ink], ((206, 206, 200), (186, 186, 180)));
}

// ---------------------------------------------------------------- ISOMETRIC
//
// 3D towers on an isometric floor; the height of each tower is its value.

// =================================================================== CLEAN KIT
//
// The clean collection (studio, grid, line, split, meter, glance, daylight) is drawn with
// Inter on a strict grid. Every design shows the same, complete set of readings:
// clock + date, weather + 3 days, CPU / GPU / memory sections (a hero number and six
// cells each), and the song playing.

/// Ink of one design (dark designs use the theme, daylight its own light set).
#[derive(Clone, Copy)]
struct Ink {
    bg: Rgb,
    text: Rgb,
    muted: Rgb,
    dim: Rgb,
    /// Hairlines.
    line: (Rgb, u8),
    /// Tiles / cards.
    card: Rgb,
}

fn dark_ink() -> Ink {
    let t = th();
    Ink { bg: t.bg, text: t.text, muted: t.muted, dim: t.dim, line: (WHITE, 30), card: lerp_rgb(t.panel, WHITE, 0.025) }
}

struct Cell {
    label: &'static str,
    value: String,
    unit: String,
    color: Rgb,
    /// 0..1 when the reading has a natural full scale (bars of the meter design).
    frac: Option<f32>,
}

struct Sec<'a> {
    name: &'static str,
    device: String,
    colors: [Rgb; 2],
    hero: String,
    hero_unit: &'static str,
    hero_color: Rgb,
    hero_frac: Option<f32>,
    side: String,
    cells: [Cell; 6],
    hist: Option<(&'a VecDeque<f32>, f32)>,
}

fn cell(label: &'static str, it: &Item) -> Cell {
    Cell { label, value: it.value.clone(), unit: it.unit.clone(), color: it.color, frac: it.frac }
}

/// The three sections, the same readings as the Focus design.
fn sections<'a>(d: &Data, v: &Vm, hist: Hist<'a>) -> [Sec<'a>; 3] {
    let (dn, up) = d.net_kb;
    let (nv, nu) = if dn.max(up) >= 1024.0 {
        let f = |x: f64| if x >= 100.0 * 1024.0 { format!("{:.0}", x / 1024.0) } else { format!("{:.1}", x / 1024.0) };
        (format!("{} / {}", f(dn), f(up)), "MB/s")
    } else {
        (format!("{dn:.0} / {up:.0}"), "KB/s")
    };
    let g = d.gpu;
    let fps = g.fps.filter(|f| *f > 0);
    let fps_cell = match fps {
        Some(f) => Cell { label: "FPS", value: f.to_string(), unit: format!("{:.1} ms", 1000.0 / f as f32), color: th().fps, frac: None },
        None => Cell { label: "FPS", value: "--".into(), unit: String::new(), color: th().dim, frac: None },
    };
    let hot = &v.gpu[0];
    let gpu_side = if hot.value == "--" { String::new() } else { format!("Hotspot {}\u{b0}", hot.value) };
    let mem_max = hist.2.iter().cloned().fold(256.0f32, f32::max) * 1.1;
    let _ = mem_max;
    [
        Sec {
            name: "CPU",
            device: v.cpu_name.clone(),
            colors: th().cpu,
            hero: v.cpu_temp.clone(),
            hero_unit: "\u{b0}C",
            hero_color: v.cpu_temp_color,
            hero_frac: v.cpu_temp_f,
            side: v.cores.clone(),
            cells: [
                cell("Load", &v.cpu[0]),
                cell("Power", &v.cpu[1]),
                cell("Clock", &v.cpu[2]),
                cell("Fan", &v.cpu[3]),
                cell("Uptime", &v.cpu[4]),
                Cell { label: "Network", value: nv, unit: nu.into(), color: th().text, frac: None },
            ],
            hist: Some((hist.0, 100.0)),
        },
        Sec {
            name: "GPU",
            device: v.gpu_name.clone(),
            colors: th().gpu,
            hero: v.gpu_temp.clone(),
            hero_unit: "\u{b0}C",
            hero_color: v.gpu_temp_color,
            hero_frac: v.gpu_temp_f,
            side: gpu_side,
            cells: [cell("Load", &v.gpu[1]), cell("Power", &v.gpu[2]), cell("Fan", &v.gpu[3]), cell("Clock", &v.gpu[4]), cell("VRAM", &v.gpu[5]), fps_cell],
            hist: Some((hist.1, 100.0)),
        },
        Sec {
            name: "Memory",
            device: String::new(),
            colors: th().ram,
            hero: v.ram_pct.clone(),
            hero_unit: "%",
            hero_color: th().text,
            hero_frac: Some(v.ram_frac),
            side: format!("of {} GB", v.ram_total),
            cells: [cell("Used", &v.mem[0]), cell("Free", &v.mem[1]), cell("SSD", &v.mem[2]), cell("Disk", &v.mem[3]), cell("Read", &v.mem[4]), cell("Write", &v.mem[5])],
            hist: None,
        },
    ]
}

/// Network history (for the designs that draw it).
fn net_hist<'a>(hist: Hist<'a>) -> (&'a VecDeque<f32>, f32) {
    (hist.2, hist.2.iter().cloned().fold(256.0f32, f32::max) * 1.1)
}

fn hair_h(p: &mut Painter, x0: f32, x1: f32, y: f32, ink: &Ink) {
    p.layer.fill_rrect_solid((x0.round(), y.round(), (x1 - x0).round(), 1.0), 0.0, ink.line.0, ink.line.1);
}

fn hair_v(p: &mut Painter, x: f32, y0: f32, y1: f32, ink: &Ink) {
    p.layer.fill_rrect_solid((x.round(), y0.round(), 1.0, (y1 - y0).round()), 0.0, ink.line.0, ink.line.1);
}

/// Small label with letter spacing (the Focus style).
fn caps(p: &mut Painter, x: f32, base: f32, text: &str, cap: f32, c: Rgb, k: f32) -> f32 {
    spaced(p, x, base, &text.to_uppercase(), cap, W::Medium, c, 1.5 * k)
}

fn unit_cap(cap: f32) -> f32 {
    (cap * 0.42).max(12.0)
}

/// Value + smaller unit on the same baseline; returns the width.
fn vu(p: &mut Painter, x: f32, base: f32, value: &str, unit: &str, cap: f32, wt: W, vc: Rgb, uc: Rgb) -> f32 {
    let vw = p.text(x, base, value, cap, wt, vc, 255);
    if unit.is_empty() {
        return vw;
    }
    let gap = cap * 0.12;
    vw + gap + p.text(x + vw + gap, base, unit, unit_cap(cap), W::Regular, uc, 255)
}

fn vu_width(p: &Painter, value: &str, unit: &str, cap: f32, wt: W) -> f32 {
    let vw = p.width(value, cap, wt);
    if unit.is_empty() {
        return vw;
    }
    vw + cap * 0.12 + p.width(unit, unit_cap(cap), W::Regular)
}

/// Largest cap (<= `cap`) at which value + unit fit `max_w`.
fn vu_fit(p: &Painter, value: &str, unit: &str, cap: f32, max_w: f32, wt: W) -> f32 {
    let w = vu_width(p, value, unit, cap, wt);
    if w <= max_w { cap } else { (cap * max_w / w).floor().max(8.0) }
}

/// Hero number, its unit raised to the top of the digits right next to it ("61°C").
fn hero(p: &mut Painter, x: f32, base: f32, value: &str, unit: &str, cap: f32, vc: Rgb, uc: Rgb) -> f32 {
    let vw = p.text(x - cap * 0.05, base, value, cap, W::Light, vc, 255);
    let ucap = (cap * 0.31).max(12.0);
    let ux = x - cap * 0.05 + vw + cap * 0.01;
    let uw = p.text(ux, base - cap + ucap, unit, ucap, W::Light, uc, 255);
    ux + uw - x
}

/// Fits a hero (value + raised unit) in `max_w`.
fn hero_fit(p: &Painter, value: &str, unit: &str, cap: f32, max_w: f32) -> f32 {
    let w = p.width(value, cap, W::Light) + cap * 0.01 + p.width(unit, (cap * 0.31).max(12.0), W::Light);
    if w <= max_w { cap } else { (cap * max_w / w).floor() }
}

/// Label (small capitals) above a value; `top` is the top of the label.
fn kv(p: &mut Painter, x: f32, top: f32, w: f32, c: &Cell, lcap: f32, vcap: f32, ink: &Ink, k: f32) {
    caps(p, x, top + lcap, c.label, lcap, ink.dim, k);
    let pair = c.value.contains(" / ");
    let cap = vu_fit(p, &c.value, &c.unit, if pair { vcap * 0.8 } else { vcap }, w, W::Regular);
    let vc = if c.color == th().text || c.color == th().dim { if c.value == "--" { ink.dim } else { ink.text } } else { c.color };
    vu(p, x, top + lcap + 9.0 * k + vcap, &c.value, &c.unit, cap, W::Regular, vc, ink.muted);
}

/// Height of a `kv` block.
fn kv_h(lcap: f32, vcap: f32, k: f32) -> f32 {
    lcap + 9.0 * k + vcap
}

/// Tops of `n` rows of height `ch` spread evenly from `top` so the last one ends at `bottom`.
fn spread(top: f32, bottom: f32, ch: f32, n: usize) -> Vec<f32> {
    if n <= 1 {
        return vec![top];
    }
    let pitch = ((bottom - top - ch) / (n as f32 - 1.0)).max(ch);
    (0..n).map(|i| (top + i as f32 * pitch).round()).collect()
}

/// Thin trend line (no grid), for the clean designs.
fn trend(p: &mut Painter, r: R, hist: &VecDeque<f32>, max: f32, colors: [Rgb; 2], k: f32) {
    let (x, y, w, h) = r;
    if hist.len() < 2 || max <= 0.0 {
        return;
    }
    let step = w / (HIST as f32 - 1.0);
    let x_start = x + w - (hist.len() as f32 - 1.0) * step;
    let bottom = y + h;
    let n = hist.len();
    let pts: Vec<(f32, f32)> = (0..n).map(|i| (x_start + i as f32 * step, bottom - (hist[i] / max).clamp(0.0, 1.0) * h)).collect();
    for px in (x_start.max(x).floor() as i32)..((x + w).ceil() as i32) {
        let fx = px as f32 + 0.5;
        let f = ((fx - x_start) / step).clamp(0.0, n as f32 - 1.0);
        let i = (f.floor() as usize).min(n - 2);
        let t = f - i as f32;
        let ty = pts[i].1 * (1.0 - t) + pts[i + 1].1 * t;
        let c = gradient_at(&colors, (fx - x) / w);
        let mut py = ty.ceil() as i32;
        while (py as f32) < bottom {
            let fade = 1.0 - ((py as f32 - ty) / (bottom - ty + 1.0)).clamp(0.0, 1.0);
            let a = (46.0 * fade * fade) as u8;
            if a > 0 {
                p.layer.over(px, py, c, a);
            }
            py += 1;
        }
    }
    for wnd in pts.windows(2) {
        let c = gradient_at(&colors, ((wnd[1].0 - x) / w).clamp(0.0, 1.0));
        capsule(p, wnd[0], wnd[1], 1.3 * k, c, 255);
    }
    if let Some(&last) = pts.last() {
        circle(p, last.0, last.1, 3.4 * k, colors[1], 255);
    }
}

/// Slim bar: track + fill.
fn bar(p: &mut Painter, x: f32, y: f32, w: f32, h: f32, frac: Option<f32>, colors: [Rgb; 2], track: (Rgb, u8)) {
    p.layer.fill_rrect((x, y, w, h), h / 2.0, &move |_| track.0, track.1);
    if let Some(f) = frac.map(|f| f.clamp(0.0, 1.0)).filter(|f| *f > 0.0) {
        let fw = (w * f).max(h);
        p.layer.fill_rrect((x, y, fw, h), h / 2.0, &move |t| gradient_at(&colors, t * f.max(0.05)), 255);
    }
}

/// "SUNDAY · 25 OCTOBER 2026" in small capitals.
fn date_line(p: &mut Painter, x: f32, base: f32, v: &Vm, cap: f32, c: Rgb, k: f32) -> f32 {
    caps(p, x, base, &format!("{}  \u{b7}  {}", v.wd, v.date), cap, c, k)
}

/// Weather now as a kv block: "PARTLY CLOUDY · MILANO" above icon + temperature.
fn wx_now(p: &mut Painter, x: f32, top: f32, w: f32, v: &Vm, lcap: f32, vcap: f32, ink: &Ink, k: f32) {
    let Some(wx) = &v.wx else {
        caps(p, x, top + lcap, "Weather", lcap, ink.dim, k);
        p.text(x, top + lcap + 9.0 * k + vcap, "--", vcap, W::Regular, ink.dim, 255);
        return;
    };
    let sub = if wx.city.is_empty() { wx.cond.clone() } else { format!("{} \u{b7} {}", wx.cond, wx.city) };
    let sub = ellipsize(p, &sub.to_uppercase(), lcap, W::Medium, w * 0.8);
    caps(p, x, top + lcap, &sub, lcap, ink.dim, k);
    let base = top + lcap + 9.0 * k + vcap;
    let s = vcap * 1.5;
    weather_icon(p, x - s * 0.1, base - vcap / 2.0 - s / 2.0, s, wx.icon, wx.night);
    p.text(x + s * 0.95, base, &wx.temp, vcap, W::Regular, ink.text, 255);
}

/// The next three days as three kv blocks.
fn wx_days(p: &mut Painter, x: f32, top: f32, w: f32, v: &Vm, lcap: f32, vcap: f32, ink: &Ink, k: f32) {
    let Some(wx) = &v.wx else { return };
    let cw = w / 3.0;
    // one size for the three days, small enough that "27° 19°" fits its column
    let mut vcap = vcap;
    for f in wx.days.iter().take(3) {
        let need = p.width(&f.hi, vcap, W::Regular) + vcap * 0.2 + p.width(&f.lo, vcap * 0.62, W::Regular);
        if need > cw - 16.0 * k {
            vcap = (vcap * (cw - 16.0 * k) / need).floor();
        }
    }
    for (i, f) in wx.days.iter().take(3).enumerate() {
        let cx = x + i as f32 * cw;
        let nw = caps(p, cx, top + lcap, &f.name, lcap, ink.dim, k);
        let isz = lcap * 2.0;
        weather_icon(p, cx + nw + 6.0 * k, top + lcap / 2.0 - isz / 2.0, isz, f.icon, false);
        let base = top + lcap + 9.0 * k + vcap;
        let hw = p.text(cx, base, &f.hi, vcap, W::Regular, ink.text, 255);
        p.text(cx + hw + vcap * 0.2, base, &f.lo, vcap * 0.62, W::Regular, ink.muted, 255);
    }
}

/// Cover + title + artist + elapsed / total + progress in (x, y, w, h); the cover is h tall.
fn music(p: &mut Painter, d: &Data, cover: Option<&Cover>, r: R, tcap: f32, ink: &Ink, k: f32, light: bool) {
    let (x, y, w, h) = r;
    let accent = cover.map_or(th().cpu[0], |c| c.accent);
    let ph = if light { ((226, 228, 234), (210, 212, 220)) } else { ((40, 42, 52), (24, 25, 32)) };
    let rad = (h * 0.14).clamp(8.0 * k, 14.0 * k);
    let Some(t) = d.track else {
        p.layer.fill_rrect((x, y, h, h), rad, &move |tt| lerp_rgb(ph.0, ph.1, tt), 255);
        music_note(p, x + h / 2.0, y + h / 2.0, h * 0.32, ink.dim);
        p.text(x + h + 18.0 * k, y + h / 2.0 + tcap / 2.0, "Nothing playing", tcap, W::Regular, ink.dim, 255);
        return;
    };
    match cover {
        Some(c) => blit_cover(p, c, x, y, h, rad),
        None => {
            p.layer.fill_rrect((x, y, h, h), rad, &move |tt| lerp_rgb(ph.0, ph.1, tt), 255);
            music_note(p, x + h / 2.0, y + h / 2.0, h * 0.32, ink.muted);
        }
    }
    let tx = x + h + 18.0 * k;
    let tw = (x + w - tx).max(10.0);
    let title = clean_title(&t.title);
    let acap = (tcap * 0.78).max(12.0 * k);
    let t_base = y + tcap + 1.0 * k;
    let a_base = t_base + acap + 11.0 * k;
    title_line(p, tx, t_base, tw, &title, tcap, W::Medium, ink.text);
    let mut aw = tw;
    if let Some(tl) = t.timeline.filter(|tl| tl.duration_ms > 0) {
        let f = (tl.position_now_ms() as f32 / tl.duration_ms as f32).clamp(0.0, 1.0);
        let (el, tot) = (mmss(tl.position_now_ms().min(tl.duration_ms) / 1000), mmss(tl.duration_ms / 1000));
        let tot_s = format!(" / {tot}");
        let w2 = p.width(&tot_s, acap, W::Regular);
        let w1 = p.width(&el, acap, W::Medium);
        p.text(tx + tw - w1 - w2, a_base, &el, acap, W::Medium, ink.text, 255);
        p.text(tx + tw - w2, a_base, &tot_s, acap, W::Regular, ink.muted, 255);
        aw = tw - w1 - w2 - 14.0 * k;
        let by = (y + h - 5.0 * k).max(a_base + 10.0 * k);
        let track = if light { ((0, 0, 0), 22) } else { (WHITE, 26) };
        bar(p, tx, by, tw, 5.0 * k, Some(f), [accent, lerp_rgb(accent, WHITE, 0.4)], track);
    }
    let artist = ellipsize(p, &t.artist, acap, W::Regular, aw.max(10.0));
    p.text(tx, a_base, &artist, acap, W::Regular, ink.muted, 255);
    if h >= 110.0 * k && !t.album.trim().is_empty() && t.album.trim() != t.title.trim() {
        let al = ellipsize(p, t.album.trim(), acap * 0.85, W::Regular, tw);
        p.text(tx, a_base + acap * 0.85 + 12.0 * k, &al, acap * 0.85, W::Regular, ink.dim, 255);
    }
}

/// Section header: colored dot + name, the device on the right (dim).
fn sec_head(p: &mut Painter, x: f32, base: f32, w: f32, s: &Sec, cap: f32, ink: &Ink, k: f32) {
    circle(p, x + 5.0 * k, base - cap * 0.5, 5.0 * k, s.colors[0], 255);
    let nw = caps(p, x + 18.0 * k, base, s.name, cap, ink.muted, k);
    let room = w - 18.0 * k - nw - 20.0 * k;
    if room > 40.0 * k && !s.device.is_empty() {
        let dev = ellipsize(p, &s.device, cap, W::Regular, room);
        text_r(p, x + w, base, &dev, cap, W::Regular, ink.dim);
    }
}

/// Header + hero + trend (or bar) + side note of a section, Focus-sized.
/// Returns the hero baseline.
fn sec_top(p: &mut Painter, x: f32, head: f32, w: f32, s: &Sec, hcap: f32, ink: &Ink, k: f32, track: (Rgb, u8)) -> f32 {
    let lcap = 13.5 * k;
    sec_head(p, x, head, w, s, lcap, ink, k);
    let hb = head + 18.0 * k + hcap;
    let cap = hero_fit(p, &s.hero, s.hero_unit, hcap, w * 0.6);
    hero(p, x, hb, &s.hero, s.hero_unit, cap, if s.hero_color == th().text { ink.text } else { s.hero_color }, ink.muted);
    let tw = (w * 0.38).floor();
    let tx = x + w - tw;
    match s.hist {
        Some((hs, max)) => trend(p, (tx, head + 30.0 * k, tw, (hcap * 0.4).max(30.0 * k)), hs, max, s.colors, k),
        None => bar(p, tx, head + 30.0 * k + hcap * 0.2, tw, 6.0 * k, s.hero_frac, s.colors, track),
    }
    if !s.side.is_empty() {
        text_r(p, x + w, hb, &s.side, 17.0 * k, W::Regular, ink.muted);
    }
    hb
}

/// Six cells in two columns, three rows spread from `top` to `bottom`; optional hairlines.
fn cells_2x3(p: &mut Painter, x: f32, w: f32, top: f32, bottom: f32, s: &Sec, vcap: f32, ink: &Ink, k: f32, lines: bool) {
    let lcap = 13.5 * k;
    let ch = kv_h(lcap, vcap, k);
    let rows = spread(top, bottom, ch, 3);
    let gut = 24.0 * k;
    let cw = ((w - gut) / 2.0).floor();
    for (ci, c) in s.cells.iter().enumerate() {
        let (r, col) = (ci / 2, ci % 2);
        if lines && col == 0 {
            hair_h(p, x, x + w, rows[r] - 12.0 * k, ink);
        }
        kv(p, x + col as f32 * (cw + gut), rows[r], cw, c, lcap, vcap, ink, k);
    }
}

/// The time column of the column designs: clock aligned with the heroes, then weather,
/// next days and the song on the same three rows as the sections' cells.
fn time_column(p: &mut Painter, x: f32, w: f32, head: f32, hcap: f32, top: f32, bottom: f32, vcap: f32, d: &Data, v: &Vm, cover: Option<&Cover>, ink: &Ink, k: f32, lines: bool, light: bool) {
    let lcap = 13.5 * k;
    date_line(p, x, head, v, lcap, ink.muted, k);
    let hb = head + 18.0 * k + hcap;
    let ccap = p.fit(&v.hhmm, hcap, w, W::Light);
    p.text(x - ccap * 0.05, hb, &v.hhmm, ccap, W::Light, ink.text, 255);
    let ch = kv_h(lcap, vcap, k);
    let rows = spread(top, bottom, ch, 3);
    if lines {
        for r in &rows {
            hair_h(p, x, x + w, r - 12.0 * k, ink);
        }
    }
    wx_now(p, x, rows[0], w, v, lcap, vcap * 0.9, ink, k);
    wx_days(p, x, rows[1], w, v, lcap, vcap * 0.72, ink, k);
    let mh = (bottom - rows[2]).max(40.0 * k);
    music(p, d, cover, (x, rows[2], w, mh), 19.0 * k, ink, k, light);
}

/// Four columns (time + three sections) on one grid: x positions and section width.
fn columns(w: f32, m: f32, left_w: f32, gap: f32) -> ([f32; 4], f32) {
    let sw = ((w - 2.0 * m - left_w - 3.0 * gap) / 3.0).floor();
    let x1 = m + left_w + gap;
    ([m, x1, x1 + sw + gap, x1 + 2.0 * (sw + gap)], sw)
}

// =================================================================== STUDIO
//
// Focus' proportions with a table: time + CPU / GPU / memory, hairlines between the
// columns and above every row.

fn studio(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let m = 24.0 * k;
    let gap = 40.0 * k;
    let (xs, sw) = columns(w, m, 386.0 * k, gap);
    let head = m + 13.5 * k;
    let hcap = 108.0 * k;
    let top = head + 18.0 * k + hcap + 26.0 * k;
    let bottom = h - m;
    let vcap = 50.0 * k;
    for x in &xs[1..] {
        hair_v(p, x - gap / 2.0, m, bottom, &ink);
    }
    time_column(p, xs[0], 386.0 * k, head, hcap, top, bottom, vcap, d, v, cover, &ink, k, true, false);
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let x = xs[si + 1];
        sec_top(p, x, head, sw, s, hcap, &ink, k, (WHITE, 26));
        cells_2x3(p, x, sw, top, bottom, s, vcap, &ink, k, true);
    }
}

// =================================================================== GRID
//
// Rounded tiles like phone widgets: time, weather and music tiles on the left, one tile
// per section with its six readings in small inner tiles.

fn tile(p: &mut Painter, r: R, rad: f32, c: Rgb) {
    let r = (r.0.round(), r.1.round(), r.2.round(), r.3.round());
    p.layer.fill_rrect_solid(r, rad, c, 255);
}

fn grid(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let m = 12.0 * k;
    let g = 12.0 * k;
    let pd = 22.0 * k;
    let rad = 22.0 * k;
    let card = lerp_rgb(ink.bg, WHITE, 0.07);
    let inner = lerp_rgb(ink.bg, WHITE, 0.115);
    let left_w = 410.0 * k;
    let (xs, sw) = columns(w, m, left_w, g);
    let ih = h - 2.0 * m;
    let lcap = 13.5 * k;
    let head = m + pd + lcap;
    let hcap = 96.0 * k;
    let hb = head + 18.0 * k + hcap;
    // ---- left: time / weather / music tiles
    let x = m;
    let t_h = hb + pd - m;
    tile(p, (x, m, left_w, t_h), rad, card);
    date_line(p, x + pd, head, v, lcap, ink.muted, k);
    let ccap = p.fit(&v.hhmm, hcap, left_w - 2.0 * pd, W::Light);
    p.text(x + pd - ccap * 0.05, hb, &v.hhmm, ccap, W::Light, ink.text, 255);
    let m_h = 104.0 * k;
    let wy = m + t_h + g;
    let w_h = ih - t_h - m_h - 2.0 * g;
    tile(p, (x, wy, left_w, w_h), rad, card);
    if let Some(wx) = &v.wx {
        let s = 46.0 * k;
        let base = wy + pd + 34.0 * k;
        weather_icon(p, x + pd - 4.0 * k, base - 17.0 * k - s / 2.0, s, wx.icon, wx.night);
        let tw = p.text(x + pd + s + 6.0 * k, base, &wx.temp, 34.0 * k, W::Regular, ink.text, 255);
        let cx = x + pd + s + 6.0 * k + tw + 16.0 * k;
        let c1 = ellipsize(p, &wx.cond, 15.0 * k, W::Medium, x + left_w - pd - cx);
        p.text(cx, base - 18.0 * k, &c1, 15.0 * k, W::Medium, ink.text, 255);
        let c2 = ellipsize(p, &wx.city, 13.5 * k, W::Regular, x + left_w - pd - cx);
        p.text(cx, base, &c2, 13.5 * k, W::Regular, ink.muted, 255);
    }
    wx_days(p, x + pd, wy + w_h - pd - kv_h(12.5 * k, 26.0 * k, k), left_w - 2.0 * pd, v, 12.5 * k, 26.0 * k, &ink, k);
    let my = wy + w_h + g;
    tile(p, (x, my, left_w, m_h), rad, card);
    music(p, d, cover, (x + 16.0 * k, my + 16.0 * k, left_w - 32.0 * k, m_h - 32.0 * k), 17.0 * k, &ink, k, false);
    // ---- sections
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let x = xs[si + 1];
        tile(p, (x, m, sw, ih), rad, card);
        let ix = x + pd;
        let iw = sw - 2.0 * pd;
        sec_top(p, ix, head, iw, s, hcap, &ink, k, (WHITE, 26));
        let gt = 8.0 * k;
        let top = hb + 22.0 * k;
        let bot = m + ih - 12.0 * k;
        let bh = ((bot - top - 2.0 * gt) / 3.0).floor();
        let bw = ((sw - 24.0 * k - gt) / 2.0).floor();
        let vcap = 38.0 * k;
        let ch = kv_h(12.5 * k, vcap, k);
        for (ci, c) in s.cells.iter().enumerate() {
            let (r, col) = (ci / 2, ci % 2);
            let bx = x + 12.0 * k + col as f32 * (bw + gt);
            let by = top + r as f32 * (bh + gt);
            tile(p, (bx, by, bw, bh), 14.0 * k, inner);
            kv(p, bx + 16.0 * k, by + ((bh - ch) / 2.0).round(), bw - 32.0 * k, c, 12.5 * k, vcap, &ink, k);
        }
    }
}

// =================================================================== LINE
//
// Top: the clock and five hero numbers with their trend. Bottom: weather and music on
// the left, one row of readings per section on the right.

fn line(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let m = 24.0 * k;
    let left_w = 560.0 * k;
    let gap = 48.0 * k;
    let rx = m + left_w + gap;
    let rw = w - m - rx;
    let lcap = 13.5 * k;
    let head = m + lcap;
    let hcap = 88.0 * k;
    let hb = head + 18.0 * k + hcap;
    let split = hb + 64.0 * k;
    hair_v(p, rx - gap / 2.0, m, h - m, &ink);
    hair_h(p, m, w - m, split, &ink);
    // ---- clock
    date_line(p, m, head, v, lcap, ink.muted, k);
    let ccap = p.fit(&v.hhmm, 120.0 * k, left_w, W::Light);
    p.text(m - ccap * 0.05, hb + 30.0 * k, &v.hhmm, ccap, W::Light, ink.text, 255);
    // ---- heroes
    let secs = sections(d, v, hist);
    let fps = d.gpu.fps.filter(|f| *f > 0);
    let (nh, nmax) = net_hist(hist);
    let heroes: [(&str, [Rgb; 2], String, &str, Rgb, String); 5] = [
        ("CPU", th().cpu, secs[0].hero.clone(), "\u{b0}C", secs[0].hero_color, v.cpu_name.clone()),
        ("GPU", th().gpu, secs[1].hero.clone(), "\u{b0}C", secs[1].hero_color, if secs[1].side.is_empty() { v.gpu_name.clone() } else { secs[1].side.clone() }),
        ("Memory", th().ram, secs[2].hero.clone(), "%", th().text, secs[2].side.clone()),
        ("Frames", [th().fps, lerp_rgb(th().fps, WHITE, 0.4)], fps.map_or("--".into(), |f| f.to_string()), "fps", if fps.is_some() { th().text } else { ink.dim }, fps.map_or("No game running".into(), |f| format!("{:.1} ms per frame", 1000.0 / f as f32))),
        ("Network", th().net, v.net_dn.0.clone(), v.net_dn.1, th().text, format!("Upload {} {}", v.net_up.0, v.net_up.1)),
    ];
    let hw = (rw / 5.0).floor();
    for (i, (name, cols, val, unit, vc, sub)) in heroes.iter().enumerate() {
        let x = rx + i as f32 * hw;
        let iw = hw - 32.0 * k;
        circle(p, x + 5.0 * k, head - lcap * 0.5, 5.0 * k, cols[0], 255);
        caps(p, x + 18.0 * k, head, name, lcap, ink.muted, k);
        let cap = hero_fit(p, val, unit, hcap, iw);
        hero(p, x, hb, val, unit, cap, if *vc == th().text { ink.text } else { *vc }, ink.muted);
        let ty = hb + 14.0 * k;
        let th_ = 22.0 * k;
        match i {
            0 => trend(p, (x, ty, iw, th_), hist.0, 100.0, *cols, k),
            1 => trend(p, (x, ty, iw, th_), hist.1, 100.0, *cols, k),
            4 => trend(p, (x, ty, iw, th_), nh, nmax, *cols, k),
            2 => bar(p, x, ty + th_ - 6.0 * k, iw, 6.0 * k, secs[2].hero_frac, *cols, (WHITE, 26)),
            _ => bar(p, x, ty + th_ - 6.0 * k, iw, 6.0 * k, fps.map(|f| (f as f32 / 240.0).min(1.0)), *cols, (WHITE, 26)),
        }
        let sub = ellipsize(p, sub, lcap, W::Regular, iw);
        p.text(x, split - 12.0 * k, &sub, lcap, W::Regular, ink.muted, 255);
    }
    // ---- bottom left: weather + song
    let bt = split + 18.0 * k;
    let bb = h - m;
    let ch = kv_h(lcap, 34.0 * k, k);
    wx_now(p, m, bt, left_w * 0.4, v, lcap, 34.0 * k, &ink, k);
    wx_days(p, m + left_w * 0.4, bt, left_w * 0.6, v, lcap, 30.0 * k, &ink, k);
    let mt = bt + ch + 22.0 * k;
    music(p, d, cover, (m, mt, left_w, bb - mt), 18.0 * k, &ink, k, false);
    // ---- bottom right: one row per section
    let name_w = 118.0 * k;
    let cw = ((rw - name_w) / 6.0).floor();
    let vcap = 36.0 * k;
    let rch = kv_h(lcap, vcap, k);
    let rows = spread(bt, bb, rch, 3);
    for (si, s) in secs.iter().enumerate() {
        let y = rows[si];
        if si > 0 {
            hair_h(p, rx, w - m, y - ((rows[1] - rows[0] - rch) / 2.0).round(), &ink);
        }
        circle(p, rx + 5.0 * k, y + rch / 2.0 - 4.0 * k, 5.0 * k, s.colors[0], 255);
        caps(p, rx + 18.0 * k, y + rch / 2.0 + 1.0 * k, s.name, lcap, ink.muted, k);
        for (ci, c) in s.cells.iter().enumerate() {
            kv(p, rx + name_w + ci as f32 * cw, y, cw - 18.0 * k, c, lcap, vcap, &ink, k);
        }
    }
}

// =================================================================== SPLIT
//
// Left half: a large clock, the weather and a big now-playing card. Right half: one row
// per section, the hero number on the left of the row and its six readings beside it.

fn split(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let m = 24.0 * k;
    let gap = 56.0 * k;
    let left_w = 690.0 * k;
    let rx = m + left_w + gap;
    hair_v(p, rx - gap / 2.0, m, h - m, &ink);
    let lcap = 13.5 * k;
    // ---- left
    let head = m + lcap;
    date_line(p, m, head, v, lcap, ink.muted, k);
    let ccap = 124.0 * k;
    let cb = head + 18.0 * k + ccap;
    let cw = p.text(m - ccap * 0.05, cb, &v.hhmm, ccap, W::Light, ink.text, 255);
    let wx_x = m + cw + 48.0 * k;
    let wx_w = m + left_w - wx_x;
    if let Some(wx) = &v.wx {
        let s = 64.0 * k;
        weather_icon(p, wx_x - 6.0 * k, cb - ccap - 6.0 * k, s, wx.icon, wx.night);
        p.text(wx_x + s + 8.0 * k, cb - ccap + 46.0 * k, &wx.temp, 46.0 * k, W::Light, ink.text, 255);
        let c1 = ellipsize(p, &wx.cond, 17.0 * k, W::Medium, wx_w);
        p.text(wx_x, cb - 28.0 * k, &c1, 17.0 * k, W::Medium, ink.text, 255);
        let c2 = ellipsize(p, &wx.city, 15.0 * k, W::Regular, wx_w);
        p.text(wx_x, cb, &c2, 15.0 * k, W::Regular, ink.muted, 255);
    }
    let fy = cb + 22.0 * k;
    wx_days(p, m, fy, left_w, v, lcap, 30.0 * k, &ink, k);
    let mt = fy + kv_h(lcap, 30.0 * k, k) + 36.0 * k;
    hair_h(p, m, m + left_w, mt - 18.0 * k, &ink);
    music(p, d, cover, (m, mt, left_w, h - m - mt), 24.0 * k, &ink, k, false);
    // ---- right: one row per section
    let hero_w = 290.0 * k;
    let cx0 = rx + hero_w;
    let cw3 = ((w - m - cx0) / 3.0).floor();
    let vcap = 32.0 * k;
    let ch1 = kv_h(lcap, vcap, k);
    let rgap = 16.0 * k;
    let content = 2.0 * ch1 + rgap;
    let rows = spread(m, h - m, content, 3);
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let y = rows[si];
        if si > 0 {
            hair_h(p, rx, w - m, y - ((rows[1] - rows[0] - content) / 2.0).round(), &ink);
        }
        sec_head(p, rx, y + lcap, hero_w - 30.0 * k, s, lcap, &ink, k);
        let hcap = 68.0 * k;
        let cap = hero_fit(p, &s.hero, s.hero_unit, hcap, hero_w * 0.7);
        let hb = y + lcap + 14.0 * k + hcap;
        hero(p, rx, hb, &s.hero, s.hero_unit, cap, if s.hero_color == th().text { ink.text } else { s.hero_color }, ink.muted);
        let side = if s.side.is_empty() { s.device.clone() } else { s.side.clone() };
        let side = ellipsize(p, &side, lcap, W::Regular, hero_w - 30.0 * k);
        p.text(rx, (hb + 24.0 * k).min(y + content), &side, lcap, W::Regular, ink.muted, 255);
        for (ci, c) in s.cells.iter().enumerate() {
            let (r, col) = (ci / 3, ci % 3);
            kv(p, cx0 + col as f32 * cw3, y + r as f32 * (ch1 + rgap), cw3 - 18.0 * k, c, lcap, vcap, &ink, k);
        }
    }
}

// =================================================================== METER
//
// Studio's grid with bar meters: every section shows its hero number, four readings as
// labelled bars and two more below. Bars use a fixed full scale (e.g. 250 W for a CPU).

fn meter(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let m = 24.0 * k;
    let gap = 40.0 * k;
    let (xs, sw) = columns(w, m, 386.0 * k, gap);
    let lcap = 13.5 * k;
    let head = m + lcap;
    let hcap = 108.0 * k;
    let top = head + 18.0 * k + hcap + 26.0 * k;
    let bottom = h - m;
    for x in &xs[1..] {
        hair_v(p, x - gap / 2.0, m, bottom, &ink);
    }
    time_column(p, xs[0], 386.0 * k, head, hcap, top, bottom, 50.0 * k, d, v, cover, &ink, k, true, false);
    let g = d.gpu;
    let fr = |v: Option<f32>, full: f32| v.map(|x| (x / full).clamp(0.0, 1.0));
    let bars: [[(usize, Option<f32>); 4]; 3] = [
        [(0, Some(d.cpu_pct / 100.0)), (1, fr(d.cpu_power, 250.0)), (2, fr(d.cpu_mhz.map(|x| x as f32), 5800.0)), (3, fr(d.cpu_fan_rpm.map(|x| x as f32), 3000.0))],
        [(0, d.gpu_pct.map(|x| x / 100.0)), (1, fr(g.power_w.map(|x| x as f32), 450.0)), (3, fr(g.clock_mhz.map(|x| x as f32), 3200.0)), (4, v.vram_frac)],
        [(0, Some(v.ram_frac)), (1, Some(1.0 - v.ram_frac)), (3, v.disk_frac), (2, v.ssd_temp_f.map(|t| (t * 100.0 / 80.0).min(1.0)))],
    ];
    let rest: [[usize; 2]; 3] = [[4, 5], [2, 5], [4, 5]];
    let vcap = 44.0 * k;
    let kch = kv_h(lcap, vcap, k);
    let rest_top = bottom - kch;
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let x = xs[si + 1];
        sec_top(p, x, head, sw, s, hcap, &ink, k, (WHITE, 26));
        hair_h(p, x, x + sw, top - 12.0 * k, &ink);
        let bar_rows = spread(top, rest_top - 26.0 * k, 30.0 * k, 4);
        for (bi, (ci, f)) in bars[si].iter().enumerate() {
            let c = &s.cells[*ci];
            let y = bar_rows[bi];
            caps(p, x, y + lcap, c.label, lcap, ink.dim, k);
            let vc = if c.value == "--" { ink.dim } else if c.color == th().text { ink.text } else { c.color };
            let ucap = 13.0 * k;
            let uw = if c.unit.is_empty() { 0.0 } else { p.width(&c.unit, ucap, W::Regular) + 6.0 * k };
            if uw > 0.0 {
                text_r(p, x + sw, y + 16.0 * k, &c.unit, ucap, W::Regular, ink.muted);
            }
            text_r(p, x + sw - uw, y + 16.0 * k, &c.value, 20.0 * k, W::Medium, vc);
            bar(p, x, y + 24.0 * k, sw, 6.0 * k, *f, s.colors, (WHITE, 24));
        }
        hair_h(p, x, x + sw, rest_top - 12.0 * k, &ink);
        let gut = 24.0 * k;
        let cw = ((sw - gut) / 2.0).floor();
        for (j, ci) in rest[si].iter().enumerate() {
            kv(p, x + j as f32 * (cw + gut), rest_top, cw, &s.cells[*ci], lcap, vcap, &ink, k);
        }
    }
}

// =================================================================== GLANCE
//
// The two temperatures, very large, readable from across the room; their readings
// below them. The right column holds the clock, the weather, memory and the song.

fn glance(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let m = 24.0 * k;
    let gap = 48.0 * k;
    let right_w = 590.0 * k;
    let rx = w - m - right_w;
    let tw = ((rx - gap - m - gap) / 2.0).floor();
    let xs = [m, m + tw + gap];
    let lcap = 13.5 * k;
    let head = m + lcap;
    let secs = sections(d, v, hist);
    hair_v(p, xs[1] - gap / 2.0, m, h - m, &ink);
    hair_v(p, rx - gap / 2.0, m, h - m, &ink);
    // ---- the two temperatures
    let hcap = 176.0 * k;
    let vcap = 40.0 * k;
    let ch = kv_h(lcap, vcap, k);
    for (i, s) in secs.iter().take(2).enumerate() {
        let x = xs[i];
        let hb = sec_top(p, x, head, tw, s, hcap, &ink, k, (WHITE, 26));
        let top = hb + 30.0 * k;
        hair_h(p, x, x + tw, top - 12.0 * k, &ink);
        let rows = spread(top, h - m, ch, 2);
        let cw = (tw / 3.0).floor();
        for (ci, c) in s.cells.iter().enumerate() {
            let (r, col) = (ci / 3, ci % 3);
            kv(p, x + col as f32 * cw, rows[r], cw - 16.0 * k, c, lcap, vcap, &ink, k);
        }
    }
    // ---- right column: clock + weather now, next days, memory, song
    date_line(p, rx, head, v, lcap, ink.muted, k);
    let ccap = 84.0 * k;
    let cb = head + 18.0 * k + ccap;
    let cw = p.text(rx - ccap * 0.05, cb, &v.hhmm, ccap, W::Light, ink.text, 255);
    let wx_x = rx + cw + 34.0 * k;
    if let Some(wx) = &v.wx {
        let s = 44.0 * k;
        weather_icon(p, wx_x - 4.0 * k, cb - ccap - 2.0 * k, s, wx.icon, wx.night);
        p.text(wx_x + s + 6.0 * k, cb - ccap + 32.0 * k, &wx.temp, 32.0 * k, W::Regular, ink.text, 255);
        let sub = if wx.city.is_empty() { wx.cond.clone() } else { format!("{} \u{b7} {}", wx.cond, wx.city) };
        let sub = ellipsize(p, &sub, 14.0 * k, W::Regular, w - m - wx_x);
        p.text(wx_x, cb, &sub, 14.0 * k, W::Regular, ink.muted, 255);
    }
    let dy = cb + 18.0 * k;
    wx_days(p, rx, dy, right_w, v, lcap, 26.0 * k, &ink, k);
    let my = dy + kv_h(lcap, 26.0 * k, k) + 24.0 * k;
    hair_h(p, rx, w - m, my - 12.0 * k, &ink);
    let s = &secs[2];
    sec_head(p, rx, my + lcap, 150.0 * k, s, lcap, &ink, k);
    let mcap = 52.0 * k;
    let mb = my + lcap + 14.0 * k + mcap;
    hero(p, rx, mb, &s.hero, s.hero_unit, mcap, ink.text, ink.muted);
    p.text(rx, mb + 22.0 * k, &s.side, lcap, W::Regular, ink.muted, 255);
    let cx0 = rx + 160.0 * k;
    let cw3 = ((w - m - cx0) / 3.0).floor();
    let mv = 24.0 * k;
    let mch = kv_h(12.5 * k, mv, k);
    let mgap = 16.0 * k;
    for (ci, c) in s.cells.iter().enumerate() {
        let (r, col) = (ci / 3, ci % 3);
        kv(p, cx0 + col as f32 * cw3, my + r as f32 * (mch + mgap), cw3 - 12.0 * k, c, 12.5 * k, mv, &ink, k);
    }
    let sy = my + 2.0 * mch + mgap + 26.0 * k;
    hair_h(p, rx, w - m, sy - 13.0 * k, &ink);
    music(p, d, cover, (rx, sy, right_w, h - m - sy), 17.0 * k, &ink, k, false);
}

// =================================================================== DAYLIGHT
//
// Light mode: white cards on a soft grey; the time card stays dark so the weather icons
// and the cover keep their contrast.

/// Accent darkened until it reads on white.
fn on_light(c: Rgb) -> Rgb {
    let lum = 0.2126 * c.0 as f32 + 0.7152 * c.1 as f32 + 0.0722 * c.2 as f32;
    if lum > 150.0 { lerp_rgb(c, (0, 0, 0), ((lum - 150.0) / 160.0 + 0.15).min(0.55)) } else { c }
}

fn daylight(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let bg = (234, 235, 239);
    let light = Ink { bg, text: (17, 18, 22), muted: (98, 102, 112), dim: (128, 132, 142), line: ((0, 0, 0), 20), card: WHITE };
    let dark = Ink { bg: (22, 23, 28), text: (244, 245, 248), muted: (160, 164, 176), dim: (120, 124, 136), line: (WHITE, 30), card: (22, 23, 28) };
    rect(p, (0.0, 0.0, w, h), bg, 255);
    let m = 12.0 * k;
    let g = 12.0 * k;
    let pd = 22.0 * k;
    let rad = 24.0 * k;
    let left_w = 410.0 * k;
    let (xs, sw) = columns(w, m, left_w, g);
    let ih = h - 2.0 * m;
    let lcap = 13.5 * k;
    let head = m + pd + lcap;
    let hcap = 100.0 * k;
    let top = head + 18.0 * k + hcap + 26.0 * k;
    let bottom = h - m - pd + 4.0 * k;
    let vcap = 46.0 * k;
    tile(p, (m, m, left_w, ih), rad, dark.card);
    time_column(p, m + pd, left_w - 2.0 * pd, head, hcap, top, bottom, vcap, d, v, cover, &dark, k, true, false);
    let mut secs = sections(d, v, hist);
    for s in secs.iter_mut() {
        s.colors = [on_light(s.colors[0]), on_light(s.colors[1])];
        if s.hero_color != th().text {
            s.hero_color = on_light(s.hero_color);
        }
        for c in s.cells.iter_mut() {
            if c.color != th().text && c.color != th().dim {
                c.color = on_light(c.color);
            }
        }
    }
    for (si, s) in secs.iter().enumerate() {
        let x = xs[si + 1];
        p.layer.fill_rrect_solid((x.round(), (m + 2.0 * k).round(), sw.round(), ih.round()), rad, (0, 0, 0), 10);
        tile(p, (x, m, sw, ih), rad, WHITE);
        let ix = x + pd;
        let iw = sw - 2.0 * pd;
        sec_top(p, ix, head, iw, s, hcap, &light, k, ((0, 0, 0), 18));
        cells_2x3(p, ix, iw, top, bottom, s, vcap, &light, k, true);
    }
}

// =================================================================== CREATIVE
//
// Four more expressive designs that keep the same readings and Focus-size text:
// pulse (living temperature waves), ambient (the album cover as the room light),
// poster (one huge number, magazine style) and horizon (the day as a sky).

/// Fills the whole canvas, one pixel at a time, from `f(x, y)`.
fn fill_with(p: &mut Painter, w: f32, h: f32, f: &dyn Fn(f32, f32) -> Rgb) {
    for y in 0..(h as i32) {
        for x in 0..(w as i32) {
            p.layer.over(x, y, f(x as f32, y as f32), 255);
        }
    }
}

/// Darkens the canvas towards the bottom so text stays readable over a picture.
fn shade_down(p: &mut Painter, w: f32, y0: f32, y1: f32, max_a: u8) {
    for y in (y0 as i32)..(y1 as i32) {
        let t = ((y as f32 - y0) / (y1 - y0)).clamp(0.0, 1.0);
        let a = (max_a as f32 * t * t) as u8;
        if a > 0 {
            for x in 0..(w as i32) {
                p.layer.over(x, y, (0, 0, 0), a);
            }
        }
    }
}

/// A big soft area chart across the screen (`lo..hi` mapped to the full height).
fn wave(p: &mut Painter, r: R, hist: &VecDeque<f32>, lo: f32, hi: f32, c: [Rgb; 2], fill_a: f32, k: f32) {
    let (x, y, w, h) = r;
    let n = hist.len();
    if n < 2 {
        return;
    }
    let step = w / (HIST as f32 - 1.0);
    let x0 = x + w - (n as f32 - 1.0) * step;
    let val = |px: f32| -> f32 {
        let f = ((px - x0) / step).clamp(0.0, n as f32 - 1.0);
        let i = (f.floor() as usize).min(n - 2);
        let t = f - i as f32;
        // smooth step between samples
        let t = t * t * (3.0 - 2.0 * t);
        let v = hist[i] * (1.0 - t) + hist[i + 1] * t;
        ((v - lo) / (hi - lo)).clamp(0.02, 1.0)
    };
    let bottom = y + h;
    let mut prev: Option<(f32, f32)> = None;
    for px in (x0.max(x) as i32)..((x + w) as i32) {
        let fx = px as f32 + 0.5;
        let ty = bottom - val(fx) * h;
        let col = gradient_at(&c, ((fx - x) / w).clamp(0.0, 1.0));
        let mut py = ty.floor() as i32;
        while (py as f32) < bottom {
            let fade = 1.0 - ((py as f32 - ty) / (bottom - ty + 1.0)).clamp(0.0, 1.0);
            let a = (fill_a * (0.25 + 0.75 * fade)) as u8;
            p.layer.over(px, py, col, a);
            py += 1;
        }
        if let Some(pp) = prev {
            capsule(p, pp, (fx, ty), 1.6 * k, col, 255);
        }
        prev = Some((fx, ty));
    }
    if let Some(pp) = prev {
        circle(p, pp.0, pp.1, 5.0 * k, c[1], 255);
        circle(p, pp.0, pp.1, 11.0 * k, c[1], 50);
    }
}

// ------------------------------------------------------------------ PULSE

fn pulse(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    // the two temperature waves fill the whole screen, GPU behind CPU
    let (nh, nmax) = net_hist(hist);
    // the waves live behind the big numbers; the readings below sit on plain black
    let band = (0.0, 36.0 * k, w, 168.0 * k);
    wave(p, band, hist.1, 15.0, 100.0, th().gpu, 50.0, k);
    wave(p, band, hist.0, 15.0, 100.0, th().cpu, 58.0, k);
    let _ = (nh, nmax);
    shade_down(p, w, 150.0 * k, 205.0 * k, 255);
    rect(p, (0.0, (205.0 * k).floor(), w, h), ink.bg, 255);
    let m = 24.0 * k;
    let gap = 40.0 * k;
    let right_w = 380.0 * k;
    let sw = ((w - 2.0 * m - right_w - 3.0 * gap) / 3.0).floor();
    let lcap = 13.5 * k;
    let head = m + lcap;
    let hcap = 120.0 * k;
    let hb = head + 18.0 * k + hcap;
    let top = hb + 24.0 * k;
    let bottom = h - m;
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let x = m + si as f32 * (sw + gap);
        sec_head(p, x, head, sw, s, lcap, &ink, k);
        let cap = hero_fit(p, &s.hero, s.hero_unit, hcap, sw * 0.75);
        hero(p, x, hb, &s.hero, s.hero_unit, cap, if s.hero_color == th().text { ink.text } else { s.hero_color }, ink.muted);
        if !s.side.is_empty() {
            text_r(p, x + sw, hb, &s.side, 17.0 * k, W::Regular, ink.muted);
        }
        cells_2x3(p, x, sw, top, bottom, s, 40.0 * k, &ink, k, false);
    }
    // right column: clock, weather, song
    let rx = w - m - right_w;
    hair_v(p, rx - gap / 2.0, m, h - m, &ink);
    date_line(p, rx, head, v, lcap, ink.muted, k);
    let ccap = p.fit(&v.hhmm, 96.0 * k, right_w, W::Light);
    p.text(rx - ccap * 0.05, head + 18.0 * k + ccap, &v.hhmm, ccap, W::Light, ink.text, 255);
    let wy = head + 18.0 * k + ccap + 24.0 * k;
    wx_now(p, rx, wy, right_w, v, lcap, 30.0 * k, &ink, k);
    wx_days(p, rx, wy + kv_h(lcap, 30.0 * k, k) + 16.0 * k, right_w, v, lcap, 26.0 * k, &ink, k);
    let mh = 70.0 * k;
    music(p, d, cover, (rx, h - m - mh, right_w, mh), 17.0 * k, &ink, k, false);
}

// ---------------------------------------------------------------- AMBIENT

/// Background from the album cover: a very soft blow-up of its colors, dimmed.
fn ambient_bg(p: &mut Painter, w: f32, h: f32, cover: Option<&Cover>) {
    match cover {
        Some(c) => {
            // 6 x 6 averages, then bilinear between them = a huge blur
            let n = 6usize;
            let mut g = vec![(0.0f32, 0.0f32, 0.0f32); n * n];
            let cs = c.size as usize;
            for gy in 0..n {
                for gx in 0..n {
                    let (mut r, mut gg, mut b, mut cnt) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
                    for yy in (gy * cs / n)..((gy + 1) * cs / n) {
                        for xx in (gx * cs / n)..((gx + 1) * cs / n) {
                            let i = (yy * cs + xx) * 3;
                            r += c.rgb[i] as f32;
                            gg += c.rgb[i + 1] as f32;
                            b += c.rgb[i + 2] as f32;
                            cnt += 1.0;
                        }
                    }
                    g[gy * n + gx] = (r / cnt.max(1.0), gg / cnt.max(1.0), b / cnt.max(1.0));
                }
            }
            let at = |x: f32, y: f32| -> Rgb {
                // the wide screen shows a horizontal band of the cover
                let fx = (x / w) * (n as f32 - 1.0);
                let fy = (0.25 + 0.5 * y / h) * (n as f32 - 1.0);
                let (ix, iy) = ((fx.floor() as usize).min(n - 2), (fy.floor() as usize).min(n - 2));
                let (tx, ty) = (fx - ix as f32, fy - iy as f32);
                let (tx, ty) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
                let a = g[iy * n + ix];
                let b = g[iy * n + ix + 1];
                let cc = g[(iy + 1) * n + ix];
                let dd = g[(iy + 1) * n + ix + 1];
                let mix = |a: f32, b: f32, c: f32, d: f32| (a * (1.0 - tx) + b * tx) * (1.0 - ty) + (c * (1.0 - tx) + d * tx) * ty;
                let dim = 0.42;
                ((mix(a.0, b.0, cc.0, dd.0) * dim) as u8, (mix(a.1, b.1, cc.1, dd.1) * dim) as u8, (mix(a.2, b.2, cc.2, dd.2) * dim) as u8)
            };
            fill_with(p, w, h, &at);
        }
        None => {
            let (a, b) = (lerp_rgb(th().cpu[0], (0, 0, 0), 0.72), lerp_rgb(th().gpu[0], (0, 0, 0), 0.78));
            fill_with(p, w, h, &move |x, y| lerp_rgb(a, b, (x / w * 0.8 + y / h * 0.2).clamp(0.0, 1.0)));
        }
    }
}

fn ambient(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    ambient_bg(p, w, h, cover);
    let ink = Ink { bg: (0, 0, 0), text: (250, 250, 252), muted: (205, 208, 216), dim: (170, 174, 184), line: (WHITE, 40), card: WHITE };
    let m = 24.0 * k;
    let lcap = 13.5 * k;
    // ---- left: the song, large
    let cs = 300.0 * k;
    let cy = ((h - cs) / 2.0).round();
    p.layer.fill_rrect_solid((m + 6.0 * k, cy + 10.0 * k, cs, cs), 22.0 * k, (0, 0, 0), 70);
    match (cover, d.track) {
        (Some(c), _) => blit_cover(p, c, m, cy, cs, 20.0 * k),
        _ => {
            p.layer.fill_rrect((m, cy, cs, cs), 20.0 * k, &|t| lerp_rgb((60, 62, 76), (30, 31, 40), t), 255);
            music_note(p, m + cs / 2.0, cy + cs / 2.0, cs * 0.3, ink.muted);
        }
    }
    let tx = m + cs + 36.0 * k;
    let tw = 520.0 * k;
    date_line(p, tx, m + lcap, v, lcap, ink.muted, k);
    let ccap = 76.0 * k;
    p.text(tx - ccap * 0.05, m + lcap + 18.0 * k + ccap, &v.hhmm, ccap, W::Light, ink.text, 255);
    match d.track {
        Some(t) => {
            let title = clean_title(&t.title);
            let tcap = 34.0 * k;
            let tb = cy + cs - 118.0 * k;
            title_line(p, tx, tb, tw, &title, tcap, W::Medium, ink.text);
            let art = ellipsize(p, &t.artist, 22.0 * k, W::Regular, tw);
            p.text(tx, tb + 38.0 * k, &art, 22.0 * k, W::Regular, ink.muted, 255);
            if let Some(tl) = t.timeline.filter(|tl| tl.duration_ms > 0) {
                let f = (tl.position_now_ms() as f32 / tl.duration_ms as f32).clamp(0.0, 1.0);
                let by = cy + cs - 6.0 * k;
                bar(p, tx, by, tw, 6.0 * k, Some(f), [WHITE, WHITE], (WHITE, 50));
                let (el, tot) = (mmss(tl.position_now_ms().min(tl.duration_ms) / 1000), mmss(tl.duration_ms / 1000));
                p.text(tx, by - 14.0 * k, &el, 15.0 * k, W::Medium, ink.text, 255);
                text_r(p, tx + tw, by - 14.0 * k, &tot, 15.0 * k, W::Regular, ink.muted);
            }
        }
        None => {
            p.text(tx, cy + cs - 70.0 * k, "Nothing playing", 34.0 * k, W::Medium, ink.muted, 255);
        }
    }
    // ---- right: a frosted glass panel with the readings
    let gx = tx + tw + 40.0 * k;
    let gw = w - m - gx;
    let gr = (gx, m, gw, h - 2.0 * m);
    p.layer.fill_rrect_solid(gr, 26.0 * k, (255, 255, 255), 22);
    p.layer.stroke_rrect(gr, 26.0 * k, 1.0, WHITE, 46);
    let pd = 26.0 * k;
    let ix = gx + pd;
    let iw = gw - 2.0 * pd;
    wx_now(p, ix, m + pd - 4.0 * k, iw * 0.42, v, 12.5 * k, 28.0 * k, &ink, k);
    wx_days(p, ix + iw * 0.42, m + pd - 4.0 * k, iw * 0.58, v, 12.5 * k, 24.0 * k, &ink, k);
    let sy = m + pd + kv_h(12.5 * k, 28.0 * k, k) + 22.0 * k;
    hair_h(p, ix, ix + iw, sy - 12.0 * k, &ink);
    let cgap = 28.0 * k;
    let cw = ((iw - 2.0 * cgap) / 3.0).floor();
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let x = ix + si as f32 * (cw + cgap);
        circle(p, x + 5.0 * k, sy + lcap * 0.5, 5.0 * k, s.colors[1], 255);
        caps(p, x + 18.0 * k, sy + lcap, s.name, lcap, ink.muted, k);
        let hcap = 60.0 * k;
        let hb = sy + lcap + 14.0 * k + hcap;
        let cap = hero_fit(p, &s.hero, s.hero_unit, hcap, cw);
        hero(p, x, hb, &s.hero, s.hero_unit, cap, ink.text, ink.muted);
        let top = hb + 20.0 * k;
        let bottom = h - m - pd + 6.0 * k;
        let vcap = 24.0 * k;
        let lc = 12.0 * k;
        let ch = kv_h(lc, vcap, k);
        let rows = spread(top, bottom, ch, 3);
        let cw2 = ((cw - 14.0 * k) / 2.0).floor();
        for (ci, c) in s.cells.iter().enumerate() {
            let (r, col) = (ci / 2, ci % 2);
            let c2 = Cell { label: c.label, value: c.value.clone(), unit: c.unit.clone(), color: if c.color == th().fps { lerp_rgb(th().fps, WHITE, 0.3) } else { th().text }, frac: c.frac };
            kv(p, x + col as f32 * (cw2 + 14.0 * k), rows[r], cw2, &c2, lc, vcap, &ink, k);
        }
    }
}

// ----------------------------------------------------------------- POSTER

fn poster(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let m = 24.0 * k;
    let lcap = 13.5 * k;
    let secs = sections(d, v, hist);
    // ---- the poster: the CPU temperature, so large it runs off the bottom edge
    let pw = 760.0 * k;
    let big = &secs[0];
    let bcap = 430.0 * k;
    let bc = if big.hero_color == th().text { lerp_rgb(th().cpu[0], WHITE, 0.25) } else { big.hero_color };
    let bw = p.text(m - bcap * 0.06, h + 92.0 * k, &big.hero, bcap, W::Light, bc, 255);
    caps(p, m, m + lcap, "CPU temperature", lcap, ink.muted, k);
    p.text(m - bcap * 0.06 + bw + 6.0 * k, h + 92.0 * k - bcap + 56.0 * k, "\u{b0}C", 56.0 * k, W::Light, ink.muted, 255);
    let dev = ellipsize(p, &big.device, lcap, W::Regular, pw - 300.0 * k);
    p.text(m, m + lcap + 24.0 * k, &dev, lcap, W::Regular, ink.dim, 255);
    // ---- right: top row clock + weather + song, then GPU / memory / CPU details
    let rx = m + pw + 32.0 * k;
    let rw = w - m - rx;
    hair_v(p, rx - 16.0 * k, m, h - m, &ink);
    let head = m + lcap;
    date_line(p, rx, head, v, lcap, ink.muted, k);
    let ccap = 72.0 * k;
    let cb = head + 16.0 * k + ccap;
    let cwid = p.text(rx - ccap * 0.05, cb, &v.hhmm, ccap, W::Light, ink.text, 255);
    let wx_x = rx + cwid + 36.0 * k;
    if let Some(wx) = &v.wx {
        let s = 40.0 * k;
        weather_icon(p, wx_x - 4.0 * k, cb - ccap, s, wx.icon, wx.night);
        p.text(wx_x + s + 6.0 * k, cb - ccap + 30.0 * k, &wx.temp, 30.0 * k, W::Regular, ink.text, 255);
        let sub = ellipsize(p, &wx.cond, 13.5 * k, W::Regular, 230.0 * k);
        p.text(wx_x, cb, &sub, 13.5 * k, W::Regular, ink.muted, 255);
    }
    let mx = wx_x + 250.0 * k;
    music(p, d, cover, (mx, cb - 62.0 * k, w - m - mx, 62.0 * k), 16.0 * k, &ink, k, false);
    let dy = cb + 16.0 * k;
    wx_days(p, rx, dy, rw * 0.5, v, 12.0 * k, 22.0 * k, &ink, k);
    let sy = dy + kv_h(12.0 * k, 22.0 * k, k) + 26.0 * k;
    hair_h(p, rx, w - m, sy - 13.0 * k, &ink);
    let cgap = 32.0 * k;
    let cw = ((rw - 2.0 * cgap) / 3.0).floor();
    let order = [1usize, 2, 0];
    for (j, si) in order.iter().enumerate() {
        let s = &secs[*si];
        let x = rx + j as f32 * (cw + cgap);
        circle(p, x + 5.0 * k, sy + lcap * 0.5, 5.0 * k, s.colors[0], 255);
        caps(p, x + 18.0 * k, sy + lcap, s.name, lcap, ink.muted, k);
        let hcap = 50.0 * k;
        let hb = sy + lcap + 12.0 * k + hcap;
        let (hv, hu, hc) = if *si == 0 { (v.cpu_load.clone(), "%", th().text) } else { (s.hero.clone(), s.hero_unit, s.hero_color) };
        let cap = hero_fit(p, &hv, hu, hcap, cw * 0.6);
        hero(p, x, hb, &hv, hu, cap, if hc == th().text { ink.text } else { hc }, ink.muted);
        if !s.side.is_empty() && *si != 0 {
            text_r(p, x + cw, hb, &s.side, 14.0 * k, W::Regular, ink.muted);
        }
        let top = hb + 22.0 * k;
        let vcap = 26.0 * k;
        let lc = 12.0 * k;
        let rows = spread(top, h - m, kv_h(lc, vcap, k), 2);
        let cw2 = ((cw - 24.0 * k) / 3.0).floor();
        for (ci, c) in s.cells.iter().enumerate() {
            let (r, col) = (ci / 3, ci % 3);
            let cc = if *si == 0 && ci == 0 { Cell { label: "Cores", value: v.cores.clone(), unit: String::new(), color: th().text, frac: None } } else { Cell { label: c.label, value: c.value.clone(), unit: c.unit.clone(), color: c.color, frac: c.frac } };
            kv(p, x + col as f32 * (cw2 + 12.0 * k), rows[r], cw2, &cc, lc, vcap, &ink, k);
        }
    }
}

// ---------------------------------------------------------------- HORIZON

fn horizon(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    let now = chrono::Local::now();
    let t = now.hour() as f32 + now.minute() as f32 / 60.0;
    // how high the sun is (-1 midnight .. 1 noon), from a 6:00-18:00 day
    let day = (std::f32::consts::PI * (t - 6.0) / 12.0).sin();
    let line_y = (204.0 * k).round();
    let dl = day.clamp(0.0, 1.0);
    let (top_c, low_c) = (lerp_rgb((3, 5, 14), (12, 34, 72), dl), lerp_rgb((12, 14, 34), (44, 92, 140), dl));
    let dusk = (1.0 - day.abs() * 3.0).clamp(0.0, 1.0); // warm glow near sunrise / sunset
    fill_with(p, w, h, &|_, y| {
        if y < line_y {
            let tt = y / line_y;
            lerp_rgb(lerp_rgb(top_c, low_c, tt), (180, 90, 60), dusk * tt * tt * 0.55)
        } else {
            ink.bg
        }
    });
    let m = 24.0 * k;
    // the 24 h track: day arc above the line, hour marks under it
    let x_of = |hh: f32| m + (hh / 24.0) * (w - 2.0 * m);
    let arc_h = 150.0 * k;
    let mut prev: Option<(f32, f32)> = None;
    for i in 0..=96 {
        let hh = i as f32 / 4.0;
        let a = (std::f32::consts::PI * (hh - 6.0) / 12.0).sin();
        let pt = (x_of(hh), line_y - a.max(0.0) * arc_h);
        if let Some(pp) = prev {
            if i % 2 == 0 {
                capsule(p, pp, pt, 0.8 * k, WHITE, 40);
            }
        }
        prev = Some(pt);
    }
    hair_h(p, 0.0, w, line_y, &Ink { line: (WHITE, 70), ..ink });
    for hh in [0, 3, 6, 9, 12, 15, 18, 21, 24] {
        let x = x_of(hh as f32);
        p.layer.fill_rrect_solid((x.round(), line_y, 1.0, 7.0 * k), 0.0, WHITE, 70);
        let lab = format!("{hh:02}");
        let lw = p.width(&lab, 11.0 * k, W::Regular);
        p.text((x - lw / 2.0).clamp(m, w - m - lw), line_y + 22.0 * k, &lab, 11.0 * k, W::Regular, ink.dim, 255);
    }
    // the sun (day) or the moon (night) on the track
    let sx = x_of(t);
    if day > 0.0 {
        let sy = line_y - day * arc_h;
        for (r, a) in [(46.0, 14u8), (32.0, 26), (22.0, 50)] {
            circle(p, sx, sy, r * k, (255, 200, 90), a);
        }
        circle(p, sx, sy, 15.0 * k, (255, 214, 120), 255);
    } else {
        let sy = line_y - 70.0 * k;
        circle(p, sx, sy, 24.0 * k, (220, 228, 255), 30);
        circle(p, sx, sy, 13.0 * k, (230, 236, 250), 255);
        circle(p, sx + 6.0 * k, sy - 4.0 * k, 11.0 * k, lerp_rgb(top_c, low_c, (sy - 4.0 * k) / line_y), 255);
    }
    // sky text: clock + date left, weather right, song in the middle
    let lcap = 13.5 * k;
    let head = m + lcap;
    date_line(p, m, head, v, lcap, (225, 230, 240), k);
    let ccap = 104.0 * k;
    p.text(m - ccap * 0.05, head + 18.0 * k + ccap, &v.hhmm, ccap, W::Light, WHITE, 255);
    let sky = Ink { muted: (205, 212, 228), dim: (170, 178, 196), text: WHITE, ..ink };
    let wr = 400.0 * k;
    wx_now(p, w - m - wr, m, wr, v, lcap, 30.0 * k, &sky, k);
    wx_days(p, w - m - wr, m + kv_h(lcap, 30.0 * k, k) + 18.0 * k, wr, v, lcap, 24.0 * k, &sky, k);
    music(p, d, cover, (620.0 * k, m, 560.0 * k, 64.0 * k), 17.0 * k, &sky, k, false);
    // ---- below the horizon: the three sections side by side
    let top = line_y + 36.0 * k;
    let gap = 40.0 * k;
    let sw = ((w - 2.0 * m - 2.0 * gap) / 3.0).floor();
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let x = m + si as f32 * (sw + gap);
        if si > 0 {
            hair_v(p, x - gap / 2.0, top, h - m, &ink);
        }
        sec_head(p, x, top + lcap, 190.0 * k, s, lcap, &ink, k);
        let hcap = 84.0 * k;
        let hb = top + lcap + 16.0 * k + hcap;
        let cap = hero_fit(p, &s.hero, s.hero_unit, hcap, 190.0 * k);
        hero(p, x, hb, &s.hero, s.hero_unit, cap, if s.hero_color == th().text { ink.text } else { s.hero_color }, ink.muted);
        let side = if s.side.is_empty() { s.device.clone() } else { s.side.clone() };
        let side = ellipsize(p, &side, lcap, W::Regular, 190.0 * k);
        p.text(x, hb + 28.0 * k, &side, lcap, W::Regular, ink.muted, 255);
        let cx0 = x + 206.0 * k;
        let cw3 = ((x + sw - cx0) / 3.0).floor();
        let vcap = 32.0 * k;
        let lc = 12.5 * k;
        let ch = kv_h(lc, vcap, k);
        let content = 2.0 * ch + 30.0 * k;
        let r0 = (top + ((h - m - top - content) / 2.0).max(0.0)).round();
        let rows = [r0, r0 + ch + 30.0 * k];
        for (ci, c) in s.cells.iter().enumerate() {
            let (r, col) = (ci / 3, ci % 3);
            kv(p, cx0 + col as f32 * cw3, rows[r], cw3 - 12.0 * k, c, lc, vcap, &ink, k);
        }
    }
}

pub(super) fn paint(layout: Layout, p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) -> bool {
    match layout {
        Layout::Swiss => swiss(p, w, h, sc, d, v, cover),
        Layout::Studio => studio(p, w, h, sc, d, v, hist, cover),
        Layout::Grid => grid(p, w, h, sc, d, v, hist, cover),
        Layout::Line => line(p, w, h, sc, d, v, hist, cover),
        Layout::Split => split(p, w, h, sc, d, v, hist, cover),
        Layout::Meter => meter(p, w, h, sc, d, v, hist, cover),
        Layout::Glance => glance(p, w, h, sc, d, v, hist, cover),
        Layout::Daylight => daylight(p, w, h, sc, d, v, hist, cover),
        Layout::Pulse => pulse(p, w, h, sc, d, v, hist, cover),
        Layout::Ambient => ambient(p, w, h, sc, d, v, hist, cover),
        Layout::Poster => poster(p, w, h, sc, d, v, hist, cover),
        Layout::Horizon => horizon(p, w, h, sc, d, v, hist, cover),
        _ => return false,
    }
    true
}
