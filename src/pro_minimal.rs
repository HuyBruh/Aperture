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

/// Four columns (time + three sections) on one grid: x positions and section width.
fn columns(w: f32, m: f32, left_w: f32, gap: f32) -> ([f32; 4], f32) {
    let sw = ((w - 2.0 * m - left_w - 3.0 * gap) / 3.0).floor();
    let x1 = m + left_w + gap;
    ([m, x1, x1 + sw + gap, x1 + 2.0 * (sw + gap)], sw)
}

// =================================================================== GRID
//
// Rounded tiles like phone widgets: time, weather and music tiles on the left, one tile
// per section with its six readings in small inner tiles.

fn tile(p: &mut Painter, r: R, rad: f32, c: Rgb) {
    let r = (r.0.round(), r.1.round(), r.2.round(), r.3.round());
    p.layer.fill_rrect_solid(r, rad, c, 255);
}

#[derive(Clone, Copy, PartialEq)]
enum Skin {
    /// Solid dark tiles.
    Grid,
    /// Tiles tinted with each section's color (Samsung One UI).
    Tint,
    /// Translucent tiles over a soft color wallpaper (macOS / Windows 11 widgets).
    Glass,
    /// Grid tiles, each section shown as an arc gauge (NZXT CAM / Kraken).
    Gauge,
}

/// Rounded rectangle with a vertical gradient (`top` to `bottom`).
fn rrect_v(p: &mut Painter, r: R, rad: f32, top: Rgb, bottom: Rgb, alpha: u8) {
    let (x, y, w, h) = (r.0.round(), r.1.round(), r.2.round(), r.3.round());
    for py in (y as i32)..((y + h) as i32) {
        let t = ((py as f32 + 0.5 - y) / h).clamp(0.0, 1.0);
        let c = lerp_rgb(top, bottom, t);
        let fy = py as f32 + 0.5;
        let near_y = fy < y + rad || fy > y + h - rad;
        for px in (x as i32)..((x + w) as i32) {
            let fx = px as f32 + 0.5;
            let cov = if near_y && (fx < x + rad || fx > x + w - rad) { rrect_cov(fx, fy, x, y, w, h, rad) } else { 1.0 };
            if cov > 0.0 {
                p.layer.over(px, py, c, (alpha as f32 * cov).round() as u8);
            }
        }
    }
}

/// The soft wallpaper behind the Glass skin: three large color glows on near-black.
fn wallpaper(p: &mut Painter, w: f32, h: f32) {
    let t = th();
    let key = format!("glass-{}-{:?}-{}x{}", t.name, t.cpu[0], w, h);
    cached_bg(p, key, &|q: &mut Painter| {
        let blobs = [(0.12 * w, 0.15 * h, 0.55 * w, t.cpu[0]), (0.55 * w, 1.05 * h, 0.5 * w, t.gpu[0]), (0.95 * w, 0.0, 0.45 * w, t.ram[0])];
        let base = (8.0f32, 9.0f32, 14.0f32);
        for y in 0..(h as i32) {
            for x in 0..(w as i32) {
                let (fx, fy) = (x as f32, y as f32 * 2.6);
                let mut c = base;
                for (bx, by, r, col) in blobs {
                    let d = ((fx - bx).powi(2) + (fy - by * 2.6).powi(2)).sqrt() / r;
                    let a = (1.0 - d).clamp(0.0, 1.0);
                    let a = a * a * (3.0 - 2.0 * a) * 0.42;
                    c = (c.0 + (col.0 as f32 - c.0) * a, c.1 + (col.1 as f32 - c.1) * a, c.2 + (col.2 as f32 - c.2) * a);
                }
                q.layer.over(x, y, (c.0 as u8, c.1 as u8, c.2 as u8), 255);
            }
        }
    });
}

/// One tile of the board in the look of `skin`; `tint` is the section color (if any).
fn surface(p: &mut Painter, r: R, rad: f32, skin: Skin, ink: &Ink, tint: Option<Rgb>) {
    match skin {
        Skin::Grid | Skin::Gauge => tile(p, r, rad, lerp_rgb(ink.bg, WHITE, 0.07)),
        Skin::Tint => {
            let c = tint.unwrap_or(lerp_rgb(ink.bg, WHITE, 0.5));
            rrect_v(p, r, rad, lerp_rgb((10, 10, 12), c, 0.34), lerp_rgb((10, 10, 12), c, 0.12), 255);
        }
        Skin::Glass => {
            let rr = (r.0.round(), r.1.round(), r.2.round(), r.3.round());
            p.layer.fill_rrect_solid(rr, rad, (0, 0, 0), 120);
            p.layer.stroke_rrect(rr, rad, 1.0, WHITE, 34);
        }
    }
}

/// A small tile inside a section tile.
fn inner_tile(p: &mut Painter, r: R, skin: Skin, ink: &Ink, k: f32) {
    match skin {
        Skin::Grid | Skin::Gauge => tile(p, r, 14.0 * k, lerp_rgb(ink.bg, WHITE, 0.115)),
        Skin::Tint => {
            let rr = (r.0.round(), r.1.round(), r.2.round(), r.3.round());
            p.layer.fill_rrect_solid(rr, 14.0 * k, WHITE, 20);
        }
        Skin::Glass => {
            let rr = (r.0.round(), r.1.round(), r.2.round(), r.3.round());
            p.layer.fill_rrect_solid(rr, 14.0 * k, WHITE, 16);
        }
    }
}

/// 270° gauge: track, value arc and the hero number in the middle.
fn gauge(p: &mut Painter, cx: f32, cy: f32, r: f32, s: &Sec, ink: &Ink, k: f32) {
    let thick = 16.0 * k;
    arc2(p, cx, cy, r, thick, -135.0, 270.0, &|_| WHITE, 30);
    if let Some(f) = s.hero_frac.map(|f| f.clamp(0.0, 1.0)).filter(|f| *f > 0.005) {
        let cols = s.colors;
        arc2(p, cx, cy, r, thick, -135.0, 270.0 * f, &move |t| gradient_at(&cols, t), 255);
    }
    let hcap = 66.0 * k;
    let cap = hero_fit(p, &s.hero, s.hero_unit, hcap, r * 1.35);
    let hw = p.width(&s.hero, cap, W::Light) + cap * 0.01 + p.width(s.hero_unit, (cap * 0.31).max(12.0), W::Light);
    let base = cy + cap * 0.42;
    hero(p, cx - hw / 2.0 + cap * 0.05, base, &s.hero, s.hero_unit, cap, if s.hero_color == th().text { ink.text } else { s.hero_color }, ink.muted);
    let side = if s.side.is_empty() { s.name.to_string() } else { s.side.clone() };
    let side = ellipsize(p, &side, 14.0 * k, W::Regular, r * 1.3);
    text_c(p, cx, base + 34.0 * k, &side, 14.0 * k, W::Regular, ink.muted);
}

fn board(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>, skin: Skin) {
    let k = sc.k;
    let mut ink = dark_ink();
    if skin != Skin::Grid && skin != Skin::Gauge {
        ink.muted = (214, 218, 228);
        ink.dim = (172, 177, 190);
    }
    match skin {
        Skin::Glass => wallpaper(p, w, h),
        _ => rect(p, (0.0, 0.0, w, h), ink.bg, 255),
    }
    let m = 12.0 * k;
    let g = 12.0 * k;
    let pd = 22.0 * k;
    let rad = 22.0 * k;
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
    let night = v.wx.as_ref().is_some_and(|w| w.night);
    surface(p, (x, m, left_w, t_h), rad, skin, &ink, Some(lerp_rgb(th().cpu[0], th().gpu[0], 0.5)));
    date_line(p, x + pd, head, v, lcap, ink.muted, k);
    let ccap = p.fit(&v.hhmm, hcap, left_w - 2.0 * pd, W::Light);
    p.text(x + pd - ccap * 0.05, hb, &v.hhmm, ccap, W::Light, ink.text, 255);
    let m_h = 104.0 * k;
    let wy = m + t_h + g;
    let w_h = ih - t_h - m_h - 2.0 * g;
    let sky = if night { (40, 52, 120) } else { (56, 140, 230) };
    surface(p, (x, wy, left_w, w_h), rad, skin, &ink, Some(sky));
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
    surface(p, (x, my, left_w, m_h), rad, skin, &ink, Some(cover.map_or(th().ram[0], |c| c.accent)));
    music(p, d, cover, (x + 16.0 * k, my + 16.0 * k, left_w - 32.0 * k, m_h - 32.0 * k), 17.0 * k, &ink, k, false);
    // ---- sections
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let x = xs[si + 1];
        surface(p, (x, m, sw, ih), rad, skin, &ink, Some(s.colors[0]));
        let ix = x + pd;
        let iw = sw - 2.0 * pd;
        let gt = 8.0 * k;
        if skin == Skin::Gauge {
            sec_head(p, ix, head, iw, s, lcap, &ink, k);
            // right: three tiles stacked; left: the gauge as tall as them; bottom: three tiles in a row
            let top = head + 14.0 * k;
            let col_w = 184.0 * k;
            let cx0 = x + sw - 12.0 * k - col_w;
            let rh = 82.0 * k;
            let vcap = 32.0 * k;
            let ch = kv_h(12.5 * k, vcap, k);
            for i in 0..3 {
                let by = top + i as f32 * (rh + gt);
                inner_tile(p, (cx0, by, col_w, rh), skin, &ink, k);
                kv(p, cx0 + 16.0 * k, by + ((rh - ch) / 2.0).round(), col_w - 32.0 * k, &s.cells[i], 12.5 * k, vcap, &ink, k);
            }
            let col_bottom = top + 3.0 * rh + 2.0 * gt;
            let gr = ((col_bottom - top) / 2.0).min((cx0 - x - 12.0 * k - 10.0 * k) / 2.0);
            gauge(p, x + 12.0 * k + (cx0 - x - 12.0 * k - gt) / 2.0, top + (col_bottom - top) / 2.0, gr, s, &ink, k);
            let by = col_bottom + gt;
            let bh = m + ih - 12.0 * k - by;
            let bw = ((sw - 24.0 * k - 2.0 * gt) / 3.0).floor();
            for i in 0..3 {
                let bx = x + 12.0 * k + i as f32 * (bw + gt);
                inner_tile(p, (bx, by, bw, bh), skin, &ink, k);
                kv(p, bx + 16.0 * k, by + ((bh - ch) / 2.0).round(), bw - 28.0 * k, &s.cells[3 + i], 12.5 * k, vcap, &ink, k);
            }
            continue;
        }
        sec_top(p, ix, head, iw, s, hcap, &ink, k, (WHITE, 26));
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
            inner_tile(p, (bx, by, bw, bh), skin, &ink, k);
            kv(p, bx + 16.0 * k, by + ((bh - ch) / 2.0).round(), bw - 32.0 * k, c, 12.5 * k, vcap, &ink, k);
        }
    }
}

// =================================================================== NEW COMPOSITIONS
//
// Five layouts with their own structure (not the four-column board): stack, mosaic, mixer,
// chart and player. Same tiles, same large type, every reading present.

const M: f32 = 12.0; // outer margin
const G: f32 = 12.0; // gap between tiles
const PD: f32 = 22.0; // padding inside a tile
const RAD: f32 = 22.0;

fn card_c(ink: &Ink) -> Rgb {
    lerp_rgb(ink.bg, WHITE, 0.07)
}

fn inner_c(ink: &Ink) -> Rgb {
    lerp_rgb(ink.bg, WHITE, 0.115)
}

/// Date + big clock, top-left aligned in the tile.
fn clock_tile(p: &mut Painter, r: R, v: &Vm, ink: &Ink, k: f32, hcap: f32) {
    let (x, y, w, _) = r;
    tile(p, r, RAD * k, card_c(ink));
    let lcap = 13.5 * k;
    date_line(p, x + PD * k, y + PD * k + lcap, v, lcap, ink.muted, k);
    let ccap = p.fit(&v.hhmm, hcap, w - 2.0 * PD * k, W::Light);
    p.text(x + PD * k - ccap * 0.05, y + PD * k + lcap + 18.0 * k + hcap, &v.hhmm, ccap, W::Light, ink.text, 255);
}

/// Weather now on top, the next three days at the bottom.
fn weather_tile(p: &mut Painter, r: R, v: &Vm, ink: &Ink, k: f32) {
    let (x, y, w, h) = r;
    tile(p, r, RAD * k, card_c(ink));
    let pd = PD * k;
    if let Some(wx) = &v.wx {
        let s = 46.0 * k;
        let base = y + pd + 34.0 * k;
        weather_icon(p, x + pd - 4.0 * k, base - 17.0 * k - s / 2.0, s, wx.icon, wx.night);
        let tw = p.text(x + pd + s + 6.0 * k, base, &wx.temp, 34.0 * k, W::Regular, ink.text, 255);
        let cx = x + pd + s + 6.0 * k + tw + 16.0 * k;
        let c1 = ellipsize(p, &wx.cond, 15.0 * k, W::Medium, x + w - pd - cx);
        p.text(cx, base - 18.0 * k, &c1, 15.0 * k, W::Medium, ink.text, 255);
        let c2 = ellipsize(p, &wx.city, 13.5 * k, W::Regular, x + w - pd - cx);
        p.text(cx, base, &c2, 13.5 * k, W::Regular, ink.muted, 255);
    } else {
        p.text(x + pd, y + pd + 30.0 * k, "Weather unavailable", 15.0 * k, W::Regular, ink.dim, 255);
    }
    wx_days(p, x + pd, y + h - pd - kv_h(12.5 * k, 26.0 * k, k), w - 2.0 * pd, v, 12.5 * k, 26.0 * k, ink, k);
}

fn music_tile(p: &mut Painter, r: R, d: &Data, cover: Option<&Cover>, ink: &Ink, k: f32) {
    let (x, y, w, h) = r;
    tile(p, r, RAD * k, card_c(ink));
    music(p, d, cover, (x + 16.0 * k, y + 16.0 * k, w - 32.0 * k, h - 32.0 * k), 17.0 * k, ink, k, false);
}

/// "Label value · Label value" in one line: labels muted, values bright; fitted to `max_w`.
fn fnote(p: &mut Painter, x: f32, base: f32, parts: &[(&str, String)], cap: f32, ink: &Ink, max_w: f32) {
    let sep = "   ";
    let width = |p: &Painter, cap: f32| -> f32 {
        let mut t = 0.0;
        for (i, (l, v)) in parts.iter().enumerate() {
            if i > 0 {
                t += p.width(sep, cap, W::Regular);
            }
            if !l.is_empty() {
                t += p.width(l, cap, W::Regular) + p.width(" ", cap, W::Regular);
            }
            t += p.width(v, cap, W::Medium);
        }
        t
    };
    let full = width(p, cap);
    let cap = if full > max_w { (cap * max_w / full).floor().max(10.0) } else { cap };
    let mut pen = x;
    for (i, (l, v)) in parts.iter().enumerate() {
        if i > 0 {
            pen += p.text(pen, base, sep, cap, W::Regular, ink.dim, 255);
        }
        if !l.is_empty() {
            pen += p.text(pen, base, l, cap, W::Regular, ink.muted, 255);
            pen += p.width(" ", cap, W::Regular);
        }
        pen += p.text(pen, base, v, cap, W::Medium, ink.text, 255);
    }
}

fn vu_s(c: &Cell) -> String {
    if c.unit.is_empty() || c.value == "--" { c.value.clone() } else { format!("{} {}", c.value, c.unit) }
}

fn hero_c(s_color: Rgb, ink: &Ink) -> Rgb {
    if s_color == th().text { ink.text } else { s_color }
}

// ------------------------------------------------------------------ STACK
//
// Three full-width rows (CPU, GPU, memory): the hero on the left, six readings in one
// line to its right. The clock, weather and song stand in a column on the right.

fn stack(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let (m, g, pd) = (M * k, G * k, PD * k);
    let rw = 410.0 * k;
    let rx = w - m - rw;
    // right column, the same three tiles as Grid
    let hcap = 96.0 * k;
    let t_h = (pd + 13.5 * k + 18.0 * k + hcap + pd).round();
    let m_h = 104.0 * k;
    let w_h = h - 2.0 * m - t_h - m_h - 2.0 * g;
    clock_tile(p, (rx, m, rw, t_h), v, &ink, k, hcap);
    weather_tile(p, (rx, m + t_h + g, rw, w_h), v, &ink, k);
    music_tile(p, (rx, m + t_h + w_h + 2.0 * g, rw, m_h), d, cover, &ink, k);
    // the three rows
    let sw = rx - g - m;
    let rh = ((h - 2.0 * m - 2.0 * g) / 3.0).floor();
    let lcap = 13.5 * k;
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let y = m + si as f32 * (rh + g);
        tile(p, (m, y, sw, rh), RAD * k, card_c(&ink));
        let x = m + pd;
        // left block: name + device, hero, side note
        let bw = 300.0 * k;
        let head = y + pd + lcap;
        circle(p, x + 5.0 * k, head - lcap * 0.5, 5.0 * k, s.colors[0], 255);
        caps(p, x + 18.0 * k, head, s.name, lcap, ink.muted, k);
        let hcap = 66.0 * k;
        let hb = y + rh - pd - 2.0 * k;
        let cap = hero_fit(p, &s.hero, s.hero_unit, hcap, bw * 0.62);
        hero(p, x, hb, &s.hero, s.hero_unit, cap, hero_c(s.hero_color, &ink), ink.muted);
        let side = if s.side.is_empty() { s.device.clone() } else { s.side.clone() };
        let side = ellipsize(p, &side, 15.0 * k, W::Regular, bw * 0.4);
        text_r(p, x + bw, hb, &side, 15.0 * k, W::Regular, ink.muted);
        if !s.device.is_empty() {
            let dev = ellipsize(p, &s.device, lcap, W::Regular, bw - 90.0 * k);
            text_r(p, x + bw, head, &dev, lcap, W::Regular, ink.dim);
        }
        // six readings in inner tiles
        let cx0 = x + bw + 24.0 * k;
        let iy = y + 12.0 * k;
        let ih = rh - 24.0 * k;
        let gt = 8.0 * k;
        let cw = ((m + sw - 12.0 * k - cx0 - 5.0 * gt) / 6.0).floor();
        let vcap = 40.0 * k;
        let ch = kv_h(12.5 * k, vcap, k);
        for (ci, c) in s.cells.iter().enumerate() {
            let bx = cx0 + ci as f32 * (cw + gt);
            tile(p, (bx, iy, cw, ih), 14.0 * k, inner_c(&ink));
            kv(p, bx + 16.0 * k, iy + ((ih - ch) / 2.0).round(), cw - 28.0 * k, c, 12.5 * k, vcap, &ink, k);
        }
    }
}

// ----------------------------------------------------------------- MOSAIC
//
// Eight equal tiles, one big number each, with the related readings as a short note under
// it. Clock and weather on the left, the album cover on the right.

fn mosaic(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let (m, g, pd) = (M * k, G * k, PD * k);
    let lw = 360.0 * k;
    let mw = 300.0 * k;
    let th_ = ((h - 2.0 * m - g) / 2.0).floor();
    let tw = ((w - 2.0 * m - lw - mw - 5.0 * g) / 4.0).floor();
    clock_tile(p, (m, m, lw, th_), v, &ink, k, 104.0 * k);
    weather_tile(p, (m, m + th_ + g, lw, th_), v, &ink, k);
    let secs = sections(d, v, hist);
    let (cpu, gpu, mem) = (&secs[0], &secs[1], &secs[2]);
    let fps = d.gpu.fps.filter(|f| *f > 0);
    let c = |s: &Sec, i: usize| vu_s(&s.cells[i]);
    // (dot color, title, hero, unit, hero color, note line 1, note line 2)
    type T<'a> = (Rgb, &'a str, String, &'a str, Rgb, Vec<(&'a str, String)>, Vec<(&'a str, String)>);
    let tiles: [T; 8] = [
        (cpu.colors[0], "CPU temperature", cpu.hero.clone(), "\u{b0}C", cpu.hero_color, vec![("Power", c(cpu, 1))], vec![("Fan", c(cpu, 3))]),
        (cpu.colors[1], "CPU load", cpu.cells[0].value.clone(), "%", th().text, vec![("Clock", c(cpu, 2))], vec![("", v.cores.clone()), ("Up", c(cpu, 4))]),
        (gpu.colors[0], "GPU temperature", gpu.hero.clone(), "\u{b0}C", gpu.hero_color, vec![("Hotspot", v.gpu[0].value.clone() + "\u{b0}"), ("Power", c(gpu, 1))], vec![("Fan", c(gpu, 2))]),
        (gpu.colors[1], "GPU load", gpu.cells[0].value.clone(), if gpu.cells[0].value == "--" { "" } else { "%" }, th().text, vec![("Clock", c(gpu, 3))], vec![("VRAM", c(gpu, 4))]),
        (mem.colors[0], "Memory", mem.hero.clone(), "%", th().text, vec![("Used", format!("{} of {} GB", mem.cells[0].value, v.ram_total))], vec![("Free", c(mem, 1))]),
        (th().ssd[0], "Storage", mem.cells[3].value.clone(), "%", th().text, vec![("SSD", c(mem, 2))], vec![("R", c(mem, 4)), ("W", c(mem, 5))]),
        (th().fps, "Frames", fps.map_or("--".into(), |f| f.to_string()), "fps", if fps.is_some() { th().text } else { ink.dim }, vec![("Frame time", fps.map_or("--".into(), |f| format!("{:.1} ms", 1000.0 / f as f32)))], if fps.is_some() { vec![] } else { vec![("", "No game running".into())] }),
        (th().net[0], "Download", v.net_dn.0.clone(), v.net_dn.1, th().text, vec![("Upload", format!("{} {}", v.net_up.0, v.net_up.1))], vec![]),
    ];
    let lcap = 13.5 * k;
    for (i, (dot, title, val, unit, vc, n1, n2)) in tiles.iter().enumerate() {
        let (col, row) = (i % 4, i / 4);
        let x = m + lw + g + col as f32 * (tw + g);
        let y = m + row as f32 * (th_ + g);
        tile(p, (x, y, tw, th_), RAD * k, card_c(&ink));
        let head = y + pd + lcap;
        circle(p, x + pd + 5.0 * k, head - lcap * 0.5, 5.0 * k, *dot, 255);
        caps(p, x + pd + 18.0 * k, head, title, lcap, ink.muted, k);
        let hcap = 80.0 * k;
        let cap = hero_fit(p, val, unit, hcap, tw - 2.0 * pd);
        let hb = head + 16.0 * k + hcap;
        hero(p, x + pd, hb, val, unit, cap, if *vc == th().text { ink.text } else { *vc }, ink.muted);
        let ncap = 16.0 * k;
        fnote(p, x + pd, y + th_ - pd - ncap - 12.0 * k, n1, ncap, &ink, tw - 2.0 * pd);
        fnote(p, x + pd, y + th_ - pd, n2, ncap, &ink, tw - 2.0 * pd);
    }
    // the song: a tall tile with the cover on top
    let mx = w - m - mw;
    tile(p, (mx, m, mw, h - 2.0 * m), RAD * k, card_c(&ink));
    let cs = mw - 2.0 * 16.0 * k;
    let (cx, cy) = (mx + 16.0 * k, m + 16.0 * k);
    match (cover, d.track) {
        (Some(cv), Some(_)) => blit_cover(p, cv, cx, cy, cs, 14.0 * k),
        _ => {
            p.layer.fill_rrect((cx, cy, cs, cs), 14.0 * k, &|t| lerp_rgb((40, 42, 52), (24, 25, 32), t), 255);
            music_note(p, cx + cs / 2.0, cy + cs / 2.0, cs * 0.3, ink.dim);
        }
    }
    let tx = mx + pd;
    let tww = mw - 2.0 * pd;
    let tb = cy + cs + 34.0 * k;
    match d.track {
        Some(t) => {
            title_line(p, tx, tb, tww, &clean_title(&t.title), 18.0 * k, W::Medium, ink.text);
            let art = ellipsize(p, &t.artist, 14.5 * k, W::Regular, tww);
            p.text(tx, tb + 24.0 * k, &art, 14.5 * k, W::Regular, ink.muted, 255);
            if let Some(tl) = t.timeline.filter(|tl| tl.duration_ms > 0) {
                let f = (tl.position_now_ms() as f32 / tl.duration_ms as f32).clamp(0.0, 1.0);
                let by = h - m - pd - 2.0 * k;
                let accent = cover.map_or(th().cpu[0], |c| c.accent);
                bar(p, tx, by, tww, 5.0 * k, Some(f), [accent, lerp_rgb(accent, WHITE, 0.4)], (WHITE, 26));
                let (el, tot) = (mmss(tl.position_now_ms().min(tl.duration_ms) / 1000), mmss(tl.duration_ms / 1000));
                p.text(tx, by - 10.0 * k, &el, 13.0 * k, W::Medium, ink.text, 255);
                text_r(p, tx + tww, by - 10.0 * k, &tot, 13.0 * k, W::Regular, ink.muted);
            }
        }
        None => {
            p.text(tx, tb, "Nothing playing", 18.0 * k, W::Medium, ink.dim, 255);
        }
    }
}

// ------------------------------------------------------------------ MIXER
//
// A mixing console: each section is a tile of four tall level faders with the value on
// top; the remaining readings sit on one line under them.

fn mixer(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let (m, g, pd) = (M * k, G * k, PD * k);
    let lw = 410.0 * k;
    let hcap = 96.0 * k;
    let t_h = (pd + 13.5 * k + 18.0 * k + hcap + pd).round();
    let m_h = 104.0 * k;
    let w_h = h - 2.0 * m - t_h - m_h - 2.0 * g;
    clock_tile(p, (m, m, lw, t_h), v, &ink, k, hcap);
    weather_tile(p, (m, m + t_h + g, lw, w_h), v, &ink, k);
    music_tile(p, (m, m + t_h + w_h + 2.0 * g, lw, m_h), d, cover, &ink, k);
    let secs = sections(d, v, hist);
    let g_ = d.gpu;
    let fr = |x: Option<f32>, full: f32| x.map(|x| (x / full).clamp(0.0, 1.0));
    let fps = g_.fps.filter(|f| *f > 0);
    // (label, value, unit, fraction) x 4 per section, then the note
    let cpu = &secs[0];
    let gpu = &secs[1];
    let mem = &secs[2];
    let faders: [[(&str, String, String, Option<f32>); 4]; 3] = [
        [
            ("Temp", cpu.hero.clone(), "\u{b0}C".into(), v.cpu_temp_f),
            ("Load", cpu.cells[0].value.clone(), "%".into(), Some(d.cpu_pct / 100.0)),
            ("Power", cpu.cells[1].value.clone(), cpu.cells[1].unit.clone(), fr(d.cpu_power, 250.0)),
            ("Fan", cpu.cells[3].value.clone(), cpu.cells[3].unit.clone(), fr(d.cpu_fan_rpm.map(|x| x as f32), 3000.0)),
        ],
        [
            ("Temp", gpu.hero.clone(), "\u{b0}C".into(), v.gpu_temp_f),
            ("Load", gpu.cells[0].value.clone(), gpu.cells[0].unit.clone(), v.gpu_frac),
            ("Power", gpu.cells[1].value.clone(), gpu.cells[1].unit.clone(), fr(g_.power_w.map(|x| x as f32), 450.0)),
            ("FPS", fps.map_or("--".into(), |f| f.to_string()), String::new(), fps.map(|f| (f as f32 / 240.0).min(1.0))),
        ],
        [
            ("RAM", mem.hero.clone(), "%".into(), Some(v.ram_frac)),
            ("VRAM", gpu.cells[4].value.clone(), gpu.cells[4].unit.clone(), v.vram_frac),
            ("Disk", mem.cells[3].value.clone(), "%".into(), v.disk_frac),
            ("SSD", mem.cells[2].value.clone(), "\u{b0}C".into(), v.ssd_temp_f.map(|t| (t * 100.0 / 80.0).min(1.0))),
        ],
    ];
    let notes: [Vec<(&str, String)>; 3] = [
        vec![("Clock", vu_s(&cpu.cells[2])), ("Up", vu_s(&cpu.cells[4])), ("Net", vu_s(&cpu.cells[5]))],
        vec![("Hotspot", v.gpu[0].value.clone() + "\u{b0}"), ("Fan", vu_s(&gpu.cells[2])), ("Clock", vu_s(&gpu.cells[3]))],
        vec![("Used", format!("{} of {} GB", mem.cells[0].value, v.ram_total)), ("R", vu_s(&mem.cells[4])), ("W", vu_s(&mem.cells[5]))],
    ];
    let sx0 = m + lw + g;
    let sw = ((w - m - sx0 - 2.0 * g) / 3.0).floor();
    let lcap = 13.5 * k;
    for (si, s) in secs.iter().enumerate() {
        let x = sx0 + si as f32 * (sw + g);
        tile(p, (x, m, sw, h - 2.0 * m), RAD * k, card_c(&ink));
        sec_head(p, x + pd, m + pd + lcap, sw - 2.0 * pd, s, lcap, &ink, k);
        let colw = ((sw - 2.0 * pd) / 4.0).floor();
        let vcap = 34.0 * k;
        let vb = m + pd + lcap + 16.0 * k + vcap;
        let track_top = vb + 16.0 * k;
        let label_b = h - m - pd - 34.0 * k;
        let track_bot = label_b - lcap - 12.0 * k;
        let fw = 44.0 * k;
        for (fi, (label, val, unit, f)) in faders[si].iter().enumerate() {
            let cx = x + pd + fi as f32 * colw + colw / 2.0;
            let cap = vu_fit(p, val, unit, vcap, colw - 8.0 * k, W::Regular);
            let vw = vu_width(p, val, unit, cap, W::Regular);
            let vc = if val == "--" { ink.dim } else { ink.text };
            vu(p, cx - vw / 2.0, vb, val, unit, cap, W::Regular, vc, ink.muted);
            let th2 = track_bot - track_top;
            p.layer.fill_rrect((cx - fw / 2.0, track_top, fw, th2), fw / 2.0, &|_| WHITE, 22);
            if let Some(f) = f.map(|f| f.clamp(0.0, 1.0)).filter(|f| *f > 0.0) {
                let fh = (th2 * f).max(fw);
                let cols = s.colors;
                p.layer.fill_rrect((cx - fw / 2.0, track_bot - fh, fw, fh), fw / 2.0, &move |t| gradient_at(&cols, t), 255);
            }
            let lw2 = spaced_width(p, &label.to_uppercase(), lcap, W::Medium, 1.5 * k);
            caps(p, cx - lw2 / 2.0, label_b, label, lcap, ink.dim, k);
        }
        fnote(p, x + pd, h - m - pd, &notes[si], 15.0 * k, &ink, sw - 2.0 * pd);
    }
}

// ------------------------------------------------------------------ CHART
//
// A proper chart: CPU and GPU temperature over the last two minutes, with a scale and a
// legend, next to a table of every other reading.

fn chart(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let (m, g, pd) = (M * k, G * k, PD * k);
    let lw = 410.0 * k;
    let hcap = 96.0 * k;
    let t_h = (pd + 13.5 * k + 18.0 * k + hcap + pd).round();
    let m_h = 104.0 * k;
    let w_h = h - 2.0 * m - t_h - m_h - 2.0 * g;
    clock_tile(p, (m, m, lw, t_h), v, &ink, k, hcap);
    weather_tile(p, (m, m + t_h + g, lw, w_h), v, &ink, k);
    music_tile(p, (m, m + t_h + w_h + 2.0 * g, lw, m_h), d, cover, &ink, k);
    let secs = sections(d, v, hist);
    let lcap = 13.5 * k;
    // ---- chart tile
    let cx = m + lw + g;
    let cw = 760.0 * k;
    let ch = h - 2.0 * m;
    tile(p, (cx, m, cw, ch), RAD * k, card_c(&ink));
    caps(p, cx + pd, m + pd + lcap, "Temperature \u{b7} last 2 minutes", lcap, ink.muted, k);
    // legend: a colored line + name, the big value under it
    let ly = m + pd + lcap + 30.0 * k;
    let lb = ly + 14.0 * k + 56.0 * k;
    let mut lx = cx + pd;
    for s in secs.iter().take(2) {
        capsule(p, (lx + 2.0 * k, ly - lcap * 0.4), (lx + 22.0 * k, ly - lcap * 0.4), 2.0 * k, s.colors[0], 255);
        caps(p, lx + 32.0 * k, ly, s.name, lcap, ink.muted, k);
        let hw = hero(p, lx, lb, &s.hero, s.hero_unit, 56.0 * k, hero_c(s.hero_color, &ink), ink.muted);
        lx += hw.max(150.0 * k) + 56.0 * k;
    }
    let hot = &v.gpu[0].value;
    if hot != "--" {
        let t = format!("Hotspot {hot}\u{b0}");
        text_r(p, cx + cw - pd, lb, &t, 16.0 * k, W::Regular, ink.muted);
    }
    // plot area
    let px0 = cx + pd + 34.0 * k;
    let px1 = cx + cw - pd;
    let py0 = lb + 30.0 * k;
    let py1 = m + ch - pd - 6.0 * k;
    let (lo, hi) = (20.0f32, 100.0f32);
    let y_of = |t: f32| py1 - ((t - lo) / (hi - lo)).clamp(0.0, 1.0) * (py1 - py0);
    for t in [20.0f32, 40.0, 60.0, 80.0, 100.0] {
        let y = y_of(t).round();
        p.layer.fill_rrect_solid((px0, y, (px1 - px0).round(), 1.0), 0.0, WHITE, if t == 20.0 { 40 } else { 18 });
        let lab = format!("{t:.0}");
        text_r(p, px0 - 10.0 * k, y + 5.0 * k, &lab, 12.0 * k, W::Regular, ink.dim);
    }
    for s in secs.iter().take(2).rev() {
        if let Some((hs, _)) = s.hist {
            let n = hs.len();
            if n >= 2 {
                let step = (px1 - px0) / (HIST as f32 - 1.0);
                let x0 = px1 - (n as f32 - 1.0) * step;
                let pts: Vec<(f32, f32)> = (0..n).map(|i| (x0 + i as f32 * step, y_of(hs[i]))).collect();
                // soft fill
                for wnd in pts.windows(2) {
                    let (a, b) = (wnd[0], wnd[1]);
                    for pxi in (a.0 as i32)..(b.0.ceil() as i32) {
                        let t = ((pxi as f32 - a.0) / (b.0 - a.0).max(0.01)).clamp(0.0, 1.0);
                        let ty = a.1 + (b.1 - a.1) * t;
                        let mut py = ty.ceil() as i32;
                        while (py as f32) < py1 {
                            let fade = 1.0 - ((py as f32 - ty) / (py1 - ty + 1.0)).clamp(0.0, 1.0);
                            let al = (40.0 * fade * fade) as u8;
                            if al > 0 {
                                p.layer.over(pxi, py, s.colors[0], al);
                            }
                            py += 1;
                        }
                    }
                }
                for wnd in pts.windows(2) {
                    capsule(p, wnd[0], wnd[1], 1.6 * k, s.colors[0], 255);
                }
                if let Some(&last) = pts.last() {
                    circle(p, last.0, last.1, 5.0 * k, s.colors[1], 255);
                }
            }
        }
    }
    // ---- table tile: three columns, six rows
    let tx = cx + cw + g;
    let tw = w - m - tx;
    tile(p, (tx, m, tw, ch), RAD * k, card_c(&ink));
    let colw = ((tw - 2.0 * pd) / 3.0).floor();
    let top = m + pd + lcap + 16.0 * k;
    let rows = 6;
    let vcap = 28.0 * k;
    let lc = 12.0 * k;
    let rch = kv_h(lc, vcap, k);
    let pitch = ((m + ch - pd - top - rch) / (rows as f32 - 1.0)).floor();
    for (si, s) in secs.iter().enumerate() {
        let x = tx + pd + si as f32 * colw;
        circle(p, x + 5.0 * k, m + pd + lcap * 0.5, 5.0 * k, s.colors[0], 255);
        let name = if si == 2 { format!("Memory {}%", s.hero) } else { s.name.to_string() };
        caps(p, x + 18.0 * k, m + pd + lcap, &name, lcap, ink.muted, k);
        for (ci, c) in s.cells.iter().enumerate() {
            let y = top + ci as f32 * pitch;
            if ci > 0 {
                p.layer.fill_rrect_solid((x.round(), (y - (pitch - rch) / 2.0).round(), (colw - 18.0 * k).round(), 1.0), 0.0, WHITE, 22);
            }
            kv(p, x, y, colw - 22.0 * k, c, lc, vcap, &ink, k);
        }
    }
}

// ----------------------------------------------------------------- PLAYER
//
// Music first: the album cover as a large square tile with the title over it, then clock
// and weather, then the three sections.

fn player(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let (m, g, pd) = (M * k, G * k, PD * k);
    let cs = h - 2.0 * m;
    // ---- cover tile
    match (cover, d.track) {
        (Some(cv), Some(_)) => blit_cover(p, cv, m, m, cs, RAD * k),
        _ => {
            p.layer.fill_rrect((m, m, cs, cs), RAD * k, &|t| lerp_rgb((44, 46, 58), (22, 23, 30), t), 255);
            music_note(p, m + cs / 2.0, m + cs * 0.42, cs * 0.26, ink.dim);
        }
    }
    // a dark fade at the bottom of the cover for the text
    let fade_top = m + cs * 0.52;
    for y in (fade_top as i32)..((m + cs) as i32) {
        let t = ((y as f32 - fade_top) / (m + cs - fade_top)).clamp(0.0, 1.0);
        let a = (220.0 * t.powf(1.3)) as u8;
        for x in (m as i32)..((m + cs) as i32) {
            let cov = rrect_cov(x as f32 + 0.5, y as f32 + 0.5, m, m, cs, cs, RAD * k);
            if cov > 0.0 {
                p.layer.over(x, y, (0, 0, 0), (a as f32 * cov) as u8);
            }
        }
    }
    let tx = m + pd;
    let tw = cs - 2.0 * pd;
    match d.track {
        Some(t) => {
            let by = m + cs - pd - 4.0 * k;
            let accent = cover.map_or(th().cpu[0], |c| c.accent);
            if let Some(tl) = t.timeline.filter(|tl| tl.duration_ms > 0) {
                let f = (tl.position_now_ms() as f32 / tl.duration_ms as f32).clamp(0.0, 1.0);
                bar(p, tx, by, tw, 5.0 * k, Some(f), [WHITE, WHITE], (WHITE, 60));
                let (el, tot) = (mmss(tl.position_now_ms().min(tl.duration_ms) / 1000), mmss(tl.duration_ms / 1000));
                p.text(tx, by - 12.0 * k, &el, 14.0 * k, W::Medium, WHITE, 255);
                text_r(p, tx + tw, by - 12.0 * k, &tot, 14.0 * k, W::Regular, (220, 222, 230));
            }
            let _ = accent;
            let art = ellipsize(p, &t.artist, 16.0 * k, W::Regular, tw);
            p.text(tx, by - 44.0 * k, &art, 16.0 * k, W::Regular, (225, 228, 236), 255);
            title_line(p, tx, by - 70.0 * k, tw, &clean_title(&t.title), 24.0 * k, W::Medium, WHITE);
        }
        None => {
            p.text(tx, m + cs - pd - 8.0 * k, "Nothing playing", 24.0 * k, W::Medium, ink.muted, 255);
        }
    }
    // ---- clock + weather
    let cx = m + cs + g;
    let cw = 380.0 * k;
    let half = ((h - 2.0 * m - g) / 2.0).floor();
    clock_tile(p, (cx, m, cw, half), v, &ink, k, 104.0 * k);
    weather_tile(p, (cx, m + half + g, cw, h - 2.0 * m - half - g), v, &ink, k);
    // ---- three section tiles
    let sx0 = cx + cw + g;
    let sw = ((w - m - sx0 - 2.0 * g) / 3.0).floor();
    let lcap = 13.5 * k;
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let x = sx0 + si as f32 * (sw + g);
        tile(p, (x, m, sw, h - 2.0 * m), RAD * k, card_c(&ink));
        let ix = x + pd;
        let iw = sw - 2.0 * pd;
        sec_head(p, ix, m + pd + lcap, iw, s, lcap, &ink, k);
        let hcap = 84.0 * k;
        let hb = m + pd + lcap + 16.0 * k + hcap;
        let cap = hero_fit(p, &s.hero, s.hero_unit, hcap, iw * 0.7);
        hero(p, ix, hb, &s.hero, s.hero_unit, cap, hero_c(s.hero_color, &ink), ink.muted);
        if !s.side.is_empty() {
            let side = ellipsize(p, &s.side, 14.0 * k, W::Regular, iw * 0.4);
            text_r(p, ix + iw, hb, &side, 14.0 * k, W::Regular, ink.muted);
        }
        let gt = 8.0 * k;
        let top = hb + 20.0 * k;
        let bot = h - m - 12.0 * k;
        let bh = ((bot - top - 2.0 * gt) / 3.0).floor();
        let bw = ((sw - 24.0 * k - gt) / 2.0).floor();
        let vcap = 30.0 * k;
        let ch2 = kv_h(12.5 * k, vcap, k);
        for (ci, c) in s.cells.iter().enumerate() {
            let (r, col) = (ci / 2, ci % 2);
            let bx = x + 12.0 * k + col as f32 * (bw + gt);
            let by = top + r as f32 * (bh + gt);
            tile(p, (bx, by, bw, bh), 14.0 * k, inner_c(&ink));
            kv(p, bx + 14.0 * k, by + ((bh - ch2) / 2.0).round(), bw - 26.0 * k, c, 12.5 * k, vcap, &ink, k);
        }
    }
}

pub(super) fn paint(layout: Layout, p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) -> bool {
    match layout {
        Layout::Swiss => swiss(p, w, h, sc, d, v, cover),
        Layout::Grid => board(p, w, h, sc, d, v, hist, cover, Skin::Grid),
        Layout::Tint => board(p, w, h, sc, d, v, hist, cover, Skin::Tint),
        Layout::Glass => board(p, w, h, sc, d, v, hist, cover, Skin::Glass),
        Layout::Gauge => board(p, w, h, sc, d, v, hist, cover, Skin::Gauge),
        Layout::Stack => stack(p, w, h, sc, d, v, hist, cover),
        Layout::Mosaic => mosaic(p, w, h, sc, d, v, hist, cover),
        Layout::Mixer => mixer(p, w, h, sc, d, v, hist, cover),
        Layout::Chart => chart(p, w, h, sc, d, v, hist, cover),
        Layout::Player => player(p, w, h, sc, d, v, hist, cover),
        _ => return false,
    }
    true
}
