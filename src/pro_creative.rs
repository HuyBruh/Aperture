//! Creative collection (4.2): board, studio, pulse and cluster.
//!
//! Four layouts with an idea of their own, built on the same rules as Grid: one grid, even
//! gaps, large numbers and every reading present (clock, weather + 3 days, the song, and the
//! hero + six readings of CPU, GPU and memory).
use super::*;

const LC: f32 = 13.5; // small-capitals label

// =================================================================== BOARD
//
// An airport departure board: every number on split-flap tiles, one row per section, the
// clock, weather and song on the top row.

const FLAP_TOP: Rgb = (34, 35, 40);
const FLAP_BOT: Rgb = (26, 27, 31);
const FLAP_INK: Rgb = (240, 240, 236);

/// One field of a row: label, characters on flaps, unit after them.
struct Field {
    label: &'static str,
    value: String,
    unit: String,
    color: Rgb,
}

fn field(label: &'static str, value: &str, unit: &str, color: Rgb) -> Field {
    let value = if value.trim().is_empty() { "--".to_string() } else { value.split_whitespace().collect::<Vec<_>>().join(" ") };
    let unit = if value == "--" { String::new() } else { unit.to_string() };
    Field { label, value, unit, color }
}

fn cell_f(label: &'static str, c: &Cell, color: Rgb) -> Field {
    field(label, &c.value, &c.unit, color)
}

/// Width of one character slot: digits and letters get a flap, '.' and ':' sit between.
fn slot_w(ch: char, fw: f32) -> f32 {
    if matches!(ch, '.' | ':' | ',' | ' ') { fw * 0.36 } else { fw }
}

fn field_w(p: &Painter, f: &Field, fw: f32, gap: f32, ucap: f32) -> f32 {
    let chars: f32 = f.value.chars().map(|c| slot_w(c, fw) + gap).sum::<f32>() - gap;
    let unit = if f.unit.is_empty() { 0.0 } else { ucap * 0.45 + p.width(&f.unit, ucap, W::Medium) };
    let label = spaced_width(p, &f.label.to_uppercase(), 11.5 * (ucap / 17.0), W::Medium, 1.2);
    chars.max(0.0).max(label) + unit
}

/// Draws one flap (two halves and the split) with `ch` centred on it.
fn flap(p: &mut Painter, x: f32, y: f32, w: f32, h: f32, ch: &str, cap: f32, ink: Rgb, k: f32) {
    let r = (6.0 * k).min(w * 0.18);
    p.layer.fill_rrect_solid((x, y, w, h), r, FLAP_BOT, 255);
    p.layer.fill_rrect_solid((x, y, w, (h / 2.0).round()), r, FLAP_TOP, 255);
    p.layer.fill_rrect_solid((x, y + h / 2.0 - r, w, r), 0.0, FLAP_TOP, 255);
    if !ch.trim().is_empty() {
        let cw = p.width(ch, cap, W::Medium);
        p.text(x + (w - cw) / 2.0, y + h / 2.0 + cap / 2.0, ch, cap, W::Medium, ink, 255);
    }
    // the split line and the two hinges
    rect(p, (x, (y + h / 2.0 - 1.0 * k).round(), w, (2.0 * k).max(1.0)), (8, 8, 10), 255);
    rect(p, (x - 1.0 * k, y + h / 2.0 - 4.0 * k, 2.0 * k, 8.0 * k), (60, 61, 66), 255);
    rect(p, (x + w - 1.0 * k, y + h / 2.0 - 4.0 * k, 2.0 * k, 8.0 * k), (60, 61, 66), 255);
}

/// Draws a field at x; returns its width.
#[allow(clippy::too_many_arguments)]
fn draw_field(p: &mut Painter, x: f32, top: f32, f: &Field, fw: f32, fh: f32, gap: f32, cap: f32, ucap: f32, ink: &Ink, k: f32) -> f32 {
    let lcap = 11.5 * (ucap / 17.0);
    caps(p, x, top + lcap, f.label, lcap, ink.dim, k * 0.8);
    let fy = top + lcap + 9.0 * k;
    let mut pen = x;
    let mut buf = [0u8; 4];
    for ch in f.value.chars() {
        let sw = slot_w(ch, fw);
        if sw < fw {
            let s = ch.encode_utf8(&mut buf);
            let cw = p.width(s, cap, W::Medium);
            p.text(pen + (sw - cw) / 2.0, fy + fh / 2.0 + cap / 2.0, s, cap, W::Medium, f.color, 255);
        } else {
            flap(p, pen, fy, sw, fh, ch.encode_utf8(&mut buf), cap, f.color, k);
        }
        pen += sw + gap;
    }
    pen -= gap;
    if !f.unit.is_empty() {
        pen += ucap * 0.45;
        pen += p.text(pen, fy + fh / 2.0 + ucap / 2.0, &f.unit, ucap, W::Medium, ink.muted, 255);
    }
    let lw = spaced_width(p, &f.label.to_uppercase(), lcap, W::Medium, 1.2 * k * 0.8);
    (pen - x).max(lw)
}

fn board_layout(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let mut ink = dark_ink();
    ink.bg = (9, 9, 11);
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let m = 12.0 * k;
    let g = 10.0 * k;
    let rh = ((h - 2.0 * m - 3.0 * g) / 4.0).floor();
    let lw = 176.0 * k;
    let fx0 = m + lw;
    let avail = w - m - fx0;
    let secs = sections(d, v, hist);

    // ---- rows: the fields of each section
    let temp_c = |c: Rgb| if c == th().text { FLAP_INK } else { c };
    let rows: [Vec<Field>; 3] = [
        vec![
            field("Temp", &secs[0].hero, "\u{b0}C", temp_c(secs[0].hero_color)),
            cell_f("Load", &secs[0].cells[0], FLAP_INK),
            cell_f("Power", &secs[0].cells[1], FLAP_INK),
            cell_f("Clock", &secs[0].cells[2], FLAP_INK),
            cell_f("Fan", &secs[0].cells[3], FLAP_INK),
            cell_f("Uptime", &secs[0].cells[4], FLAP_INK),
            cell_f("Net", &secs[0].cells[5], FLAP_INK),
        ],
        vec![
            field("Temp", &secs[1].hero, "\u{b0}C", temp_c(secs[1].hero_color)),
            field("Hotspot", &v.gpu[0].value, "\u{b0}C", FLAP_INK),
            cell_f("Load", &secs[1].cells[0], FLAP_INK),
            cell_f("Power", &secs[1].cells[1], FLAP_INK),
            cell_f("Fan", &secs[1].cells[2], FLAP_INK),
            cell_f("Clock", &secs[1].cells[3], FLAP_INK),
            cell_f("VRAM", &secs[1].cells[4], FLAP_INK),
            field("FPS", &secs[1].cells[5].value, "", if secs[1].cells[5].value == "--" { FLAP_INK } else { th().fps }),
        ],
        vec![
            field("Used", &secs[2].hero, "%", FLAP_INK),
            cell_f("In use", &secs[2].cells[0], FLAP_INK),
            cell_f("Free", &secs[2].cells[1], FLAP_INK),
            cell_f("SSD", &secs[2].cells[2], FLAP_INK),
            cell_f("Disk", &secs[2].cells[3], FLAP_INK),
            cell_f("Read", &secs[2].cells[4], FLAP_INK),
            cell_f("Write", &secs[2].cells[5], FLAP_INK),
        ],
    ];
    // one flap size for the whole board: the widest row decides
    let mut fw = 46.0 * k;
    let fgap = 3.0 * k;
    let field_gap = 26.0 * k;
    let ucap = 17.0 * k;
    for row in &rows {
        let need: f32 = row.iter().map(|f| field_w(p, f, fw, fgap, ucap)).sum::<f32>() + field_gap * (row.len() as f32 - 1.0);
        if need > avail {
            fw = (fw * avail / need).floor();
        }
    }
    let fh = (rh - 11.5 * k - 9.0 * k - 6.0 * k).min(fw * 1.62);
    let cap = (fh * 0.56).min(fw * 0.92);

    for (ri, row) in rows.iter().enumerate() {
        let y = m + (ri + 1) as f32 * (rh + g);
        let s = &secs[ri];
        // left: colour bar, name, device
        p.layer.fill_rrect_solid((m, y, 5.0 * k, rh), 2.5 * k, s.colors[0], 255);
        let nb = y + 6.0 * k + 22.0 * k;
        let ncap = p.fit(&s.name.to_uppercase(), 22.0 * k, lw - 40.0 * k, W::Medium);
        spaced(p, m + 20.0 * k, nb, &s.name.to_uppercase(), ncap, W::Medium, ink.text, 1.5 * k);
        let dev = if s.device.is_empty() { s.side.clone() } else { s.device.clone() };
        let dev = ellipsize(p, &dev, 13.0 * k, W::Regular, lw - 32.0 * k);
        p.text(m + 20.0 * k, nb + 24.0 * k, &dev, 13.0 * k, W::Regular, ink.dim, 255);
        if !s.side.is_empty() && !s.device.is_empty() {
            let side = ellipsize(p, &s.side, 13.0 * k, W::Regular, lw - 32.0 * k);
            p.text(m + 20.0 * k, nb + 44.0 * k, &side, 13.0 * k, W::Regular, ink.muted, 255);
        }
        // fields, spread evenly over the row
        let widths: Vec<f32> = row.iter().map(|f| field_w(p, f, fw, fgap, ucap)).collect();
        let used: f32 = widths.iter().sum();
        let spare = ((avail - used) / (row.len() as f32 - 1.0).max(1.0)).max(field_gap * 0.6);
        let mut x = fx0;
        let top = y + 3.0 * k;
        for (f, fwid) in row.iter().zip(&widths) {
            draw_field(p, x, top, f, fw, fh, fgap, cap, ucap, &ink, k);
            x += fwid + spare;
        }
    }

    // ---- top row: clock, date, weather, song
    let y = m;
    let top = y + 3.0 * k;
    let lcap = 11.5 * k;
    let fy = top + lcap + 9.0 * k;
    spaced(p, m + 20.0 * k, y + 6.0 * k + 22.0 * k, "TIME", 22.0 * k, W::Medium, ink.text, 2.0 * k);
    let wd = ellipsize(p, &v.wd.to_uppercase(), 13.0 * k, W::Regular, lw - 32.0 * k);
    p.text(m + 20.0 * k, y + 52.0 * k, &wd, 13.0 * k, W::Regular, ink.muted, 255);
    let dcap = p.fit(&v.date, 13.0 * k, lw - 26.0 * k, W::Regular);
    p.text(m + 20.0 * k, y + 72.0 * k, &v.date, dcap, W::Regular, ink.dim, 255);
    let mut x = fx0;
    let clock = field("Local time", &v.hhmm, "", FLAP_INK);
    x += draw_field(p, x, top, &clock, fw, fh, fgap, cap, ucap, &ink, k) + 40.0 * k;
    // weather: temperature on flaps, the sky and three days as text
    if let Some(wx) = &v.wx {
        let t = wx.temp.trim_end_matches(['\u{b0}', 'C', 'F', 'c', 'f']).to_string();
        let tf = field("Weather", &t, "\u{b0}", FLAP_INK);
        let tw = draw_field(p, x, top, &tf, fw, fh, fgap, cap, ucap, &ink, k);
        let ix = x + tw + 14.0 * k;
        let isz = 40.0 * k;
        weather_icon(p, ix, fy + fh / 2.0 - isz / 2.0, isz, wx.icon, wx.night);
        let cx = ix + isz + 12.0 * k;
        let cond = ellipsize(p, &wx.cond, 17.0 * k, W::Medium, 210.0 * k);
        p.text(cx, fy + fh / 2.0 - 5.0 * k, &cond, 17.0 * k, W::Medium, ink.text, 255);
        let city = ellipsize(p, &wx.city, 14.0 * k, W::Regular, 210.0 * k);
        p.text(cx, fy + fh / 2.0 + 19.0 * k, &city, 14.0 * k, W::Regular, ink.muted, 255);
        let cw = p.width(&cond, 17.0 * k, W::Medium).max(p.width(&city, 14.0 * k, W::Regular));
        x = cx + cw + 34.0 * k;
        for f in wx.days.iter().take(3) {
            caps(p, x, top + lcap, &f.name, lcap, ink.dim, k * 0.8);
            let base = fy + fh / 2.0 + 11.0 * k;
            let hw = p.text(x, base, &f.hi, 26.0 * k, W::Medium, ink.text, 255);
            let lw2 = p.text(x + hw + 6.0 * k, base, &f.lo, 18.0 * k, W::Regular, ink.muted, 255);
            x += (hw + 6.0 * k + lw2).max(70.0 * k) + 22.0 * k;
        }
    }
    // the song: cover and a destination strip with the title
    let sx = x.max(w - m - 560.0 * k);
    let cs = rh - 6.0 * k;
    let sy = y + 3.0 * k;
    match (cover, d.track) {
        (Some(c), Some(_)) => blit_cover(p, c, sx, sy, cs, 10.0 * k),
        _ => {
            p.layer.fill_rrect_solid((sx, sy, cs, cs), 10.0 * k, FLAP_BOT, 255);
            music_note(p, sx + cs / 2.0, sy + cs / 2.0, cs * 0.3, ink.dim);
        }
    }
    let tx = sx + cs + 14.0 * k;
    let tw = w - m - tx;
    caps(p, tx, top + lcap, "Now playing", lcap, ink.dim, k * 0.8);
    let strip_h = 44.0 * k;
    p.layer.fill_rrect_solid((tx, fy, tw, strip_h), 6.0 * k, FLAP_BOT, 255);
    p.layer.fill_rrect_solid((tx, fy, tw, strip_h / 2.0), 6.0 * k, FLAP_TOP, 255);
    rect(p, (tx, (fy + strip_h / 2.0 - 1.0 * k).round(), tw, (2.0 * k).max(1.0)), (8, 8, 10), 255);
    let (title, artist) = match d.track {
        Some(t) => (clean_title(&t.title), t.artist.clone()),
        None => ("Nothing playing".to_string(), String::new()),
    };
    title_line(p, tx + 12.0 * k, fy + strip_h / 2.0 + 10.0 * k, tw - 24.0 * k, &title.to_uppercase(), 20.0 * k, W::Medium, if d.track.is_some() { FLAP_INK } else { ink.dim });
    let ab = fy + strip_h + 22.0 * k;
    let mut aw = tw;
    if let Some((el, tot, f)) = track_times(d) {
        let s = format!("{el} / {tot}");
        let sw = text_r(p, tx + tw, ab, &s, 14.0 * k, W::Regular, ink.muted);
        aw = tw - sw - 16.0 * k;
        let by = ab + 9.0 * k;
        if by + 3.0 * k < y + rh {
            bar(p, tx, by, tw, 3.0 * k, Some(f), [th().cpu[0], th().cpu[1]], (WHITE, 24));
        }
    }
    let artist = ellipsize(p, &artist, 15.0 * k, W::Regular, aw.max(10.0));
    p.text(tx, ab, &artist, 15.0 * k, W::Regular, ink.muted, 255);
}

// =================================================================== STUDIO
//
// Paper and ink: a light layout in the spirit of Braun and Teenage Engineering. Off-white
// page, black numbers, hairline rules, one orange accent.

const PAPER: Rgb = (238, 236, 230);
const INK_BLACK: Rgb = (22, 22, 24);
const ORANGE: Rgb = (236, 84, 30);

fn studio_ink() -> Ink {
    Ink { bg: PAPER, text: INK_BLACK, muted: (92, 92, 96), dim: (138, 136, 130), line: ((0, 0, 0), 46), card: (228, 226, 219) }
}

/// Temperatures stay black unless they are worth a look.
fn studio_temp(hero: &str) -> Rgb {
    match hero.trim().parse::<i32>() {
        Ok(t) if t >= 85 => (200, 36, 36),
        Ok(t) if t >= 75 => ORANGE,
        _ => INK_BLACK,
    }
}

fn hairline(p: &mut Painter, x: f32, y: f32, w: f32, h: f32, ink: &Ink) {
    rect(p, (x.round(), y.round(), w.max(1.0), h.max(1.0)), ink.line.0, ink.line.1);
}

/// Label above value, black value (the paper has no room for colored digits).
fn studio_kv(p: &mut Painter, x: f32, top: f32, w: f32, c: &Cell, lcap: f32, vcap: f32, ink: &Ink, k: f32) {
    caps(p, x, top + lcap, c.label, lcap, ink.dim, k);
    let pair = c.value.contains(" / ");
    let cap = vu_fit(p, &c.value, &c.unit, if pair { vcap * 0.8 } else { vcap }, w, W::Regular);
    let vc = if c.value == "--" { ink.dim } else { ink.text };
    vu(p, x, top + lcap + 9.0 * k + vcap, &c.value, &c.unit, cap, W::Regular, vc, ink.muted);
}

fn studio(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = studio_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let m = 28.0 * k;
    let c0 = 404.0 * k;
    let gap = 28.0 * k;
    let sw = ((w - 2.0 * m - c0 - 3.0 * gap) / 3.0).floor();
    let top = m;
    let bottom = h - m;
    let lcap = LC * k;
    let head = top + lcap;
    let hcap = 104.0 * k;
    let hb = head + 22.0 * k + hcap;
    // a thick rule on top of every column, like a printed spec sheet
    let xs = [m, m + c0 + gap, m + c0 + gap + sw + gap, m + c0 + gap + 2.0 * (sw + gap)];
    for (i, x) in xs.iter().enumerate() {
        let cw = if i == 0 { c0 } else { sw };
        rect(p, (*x, top - 12.0 * k, cw, 3.0 * k), INK_BLACK, 255);
    }
    // ---- column 0: time, weather, song
    let x = m;
    caps(p, x, head, &format!("{}  \u{b7}  {}", v.wd, v.date), lcap, ink.muted, k);
    let ccap = p.fit(&v.hhmm, hcap, c0, W::Light);
    p.text(x - ccap * 0.05, hb, &v.hhmm, ccap, W::Light, INK_BLACK, 255);
    let wy = hb + 22.0 * k;
    hairline(p, x, wy, c0, 1.0 * k, &ink);
    if let Some(wx) = &v.wx {
        let s = 40.0 * k;
        let base = wy + 22.0 * k + 30.0 * k;
        weather_icon(p, x - 3.0 * k, base - 15.0 * k - s / 2.0, s, wx.icon, wx.night);
        let tw = p.text(x + s + 6.0 * k, base, &wx.temp, 30.0 * k, W::Regular, INK_BLACK, 255);
        let cx = x + s + 6.0 * k + tw + 16.0 * k;
        let cond = format!("{} \u{b7} {}", wx.cond, wx.city);
        let cond = ellipsize(p, &cond, 14.0 * k, W::Regular, x + c0 - cx);
        p.text(cx, base - 8.0 * k, &cond, 14.0 * k, W::Regular, ink.muted, 255);
        // three days in one line under it
        let dy = base + 16.0 * k;
        let cw = c0 / 3.0;
        for (i, f) in wx.days.iter().take(3).enumerate() {
            let dx = x + i as f32 * cw;
            caps(p, dx, dy + 12.0 * k, &f.name, 12.0 * k, ink.dim, k);
            let b = dy + 12.0 * k + 8.0 * k + 24.0 * k;
            let hw = p.text(dx, b, &f.hi, 24.0 * k, W::Regular, INK_BLACK, 255);
            p.text(dx + hw + 6.0 * k, b, &f.lo, 16.0 * k, W::Regular, ink.muted, 255);
        }
    }
    let mh = 76.0 * k;
    let my = bottom - mh;
    hairline(p, x, my - 18.0 * k, c0, 1.0 * k, &ink);
    music(p, d, cover, (x, my, c0, mh), 17.0 * k, &ink, k, true);

    // ---- sections: hero + a 3 x 2 table with hairlines
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let x = xs[si + 1];
        rect(p, (x, head - lcap * 0.85, 10.0 * k, 10.0 * k), ORANGE, 255);
        let nw = caps(p, x + 20.0 * k, head, s.name, lcap, INK_BLACK, k);
        let room = sw - 20.0 * k - nw - 18.0 * k;
        if room > 40.0 * k && !s.device.is_empty() {
            let dev = ellipsize(p, &s.device, lcap, W::Regular, room);
            text_r(p, x + sw, head, &dev, lcap, W::Regular, ink.dim);
        }
        let hc = if s.hero_unit == "%" { INK_BLACK } else { studio_temp(&s.hero) };
        let cap = hero_fit(p, &s.hero, s.hero_unit, hcap, sw * 0.62);
        hero(p, x, hb, &s.hero, s.hero_unit, cap, hc, ink.muted);
        // side: a slim level meter + note
        let mw = (sw * 0.32).floor();
        let mx = x + sw - mw;
        if let Some(f) = s.hero_frac.map(|f| f.clamp(0.0, 1.0)) {
            let ty = hb - 40.0 * k;
            for i in 0..10 {
                let seg_w = (mw - 9.0 * 3.0 * k) / 10.0;
                let sx = mx + i as f32 * (seg_w + 3.0 * k);
                let on = (i as f32 + 0.5) / 10.0 <= f;
                rect(p, (sx, ty, seg_w, 14.0 * k), if on { if i >= 8 { ORANGE } else { INK_BLACK } } else { (210, 207, 199) }, 255);
            }
        }
        if !s.side.is_empty() {
            text_r(p, x + sw, hb, &s.side, 15.0 * k, W::Regular, ink.muted);
        }
        let ty = hb + 22.0 * k;
        let rh = ((bottom - ty) / 3.0).floor();
        let cw = (sw / 2.0).floor();
        let vcap = 36.0 * k;
        let ch = kv_h(12.5 * k, vcap, k);
        for r in 0..3 {
            let ry = ty + r as f32 * rh;
            hairline(p, x, ry, sw, 1.0 * k, &ink);
            for c in 0..2 {
                let cell = &s.cells[r * 2 + c];
                let cx = x + c as f32 * cw + if c == 1 { 16.0 * k } else { 0.0 };
                if c == 1 {
                    hairline(p, x + cw, ry + 12.0 * k, 1.0 * k, rh - 24.0 * k, &ink);
                }
                studio_kv(p, cx, ry + ((rh - ch) / 2.0).round(), cw - 22.0 * k, cell, 12.5 * k, vcap, &ink, k);
            }
        }
    }
}

// =================================================================== PULSE
//
// Live charts: every section is a large chart of the last two minutes, the hero above it,
// the six readings under it. Clock, weather and song in the column on the left.

/// Area chart with soft fill, faint grid, the line and the newest point.
fn area_chart(p: &mut Painter, r: R, hist: &VecDeque<f32>, max: f32, colors: [Rgb; 2], ink: &Ink, k: f32) {
    let (x, y, w, h) = r;
    for i in 1..4 {
        let gy = (y + h * i as f32 / 4.0).round();
        let mut gx = x;
        while gx < x + w {
            rect(p, (gx, gy, (4.0 * k).min(x + w - gx), 1.0), WHITE, 18);
            gx += 9.0 * k;
        }
    }
    rect(p, (x, (y + h).round(), w, 1.0), WHITE, 40);
    if hist.len() < 2 || max <= 0.0 {
        p.text(x, y + h / 2.0, "Collecting\u{2026}", 14.0 * k, W::Regular, ink.dim, 255);
        return;
    }
    let n = hist.len();
    let step = w / (HIST as f32 - 1.0);
    let x_start = x + w - (n as f32 - 1.0) * step;
    let bottom = y + h;
    let pts: Vec<(f32, f32)> = (0..n).map(|i| (x_start + i as f32 * step, bottom - (hist[i] / max).clamp(0.0, 1.0) * h)).collect();
    for px in (x_start.max(x).floor() as i32)..((x + w).ceil() as i32) {
        let fx = px as f32 + 0.5;
        let f = ((fx - x_start) / step).clamp(0.0, n as f32 - 1.0);
        let i = (f.floor() as usize).min(n - 2);
        let t = f - i as f32;
        let ty = pts[i].1 * (1.0 - t) + pts[i + 1].1 * t;
        let c = gradient_at(&colors, ((fx - x) / w).clamp(0.0, 1.0));
        let mut py = ty.ceil() as i32;
        while (py as f32) < bottom {
            let fade = 1.0 - ((py as f32 - ty) / (h + 1.0)).clamp(0.0, 1.0);
            let a = (70.0 * fade * fade + 6.0) as u8;
            p.layer.over(px, py, c, a);
            py += 1;
        }
    }
    for wnd in pts.windows(2) {
        let c = gradient_at(&colors, ((wnd[1].0 - x) / w).clamp(0.0, 1.0));
        capsule(p, wnd[0], wnd[1], 1.6 * k, c, 255);
    }
    if let Some(&last) = pts.last() {
        circle(p, last.0, last.1, 7.0 * k, colors[1], 60);
        circle(p, last.0, last.1, 4.0 * k, colors[1], 255);
    }
}

fn pulse(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let (m, g, pd) = (M * k, G * k, PD * k);
    let left_w = 410.0 * k;
    let (xs, sw) = columns(w, m, left_w, g);
    let ih = h - 2.0 * m;
    // left column: the same three tiles as Grid
    let hcap = 96.0 * k;
    let t_h = (pd + LC * k + 18.0 * k + hcap + pd).round();
    let m_h = 104.0 * k;
    let w_h = ih - t_h - m_h - 2.0 * g;
    clock_tile(p, (m, m, left_w, t_h), v, &ink, k, hcap);
    weather_tile(p, (m, m + t_h + g, left_w, w_h), v, &ink, k);
    music_tile(p, (m, m + t_h + w_h + 2.0 * g, left_w, m_h), d, cover, &ink, k);
    // the three charts
    let lcap = LC * k;
    let net_max = hist.2.iter().cloned().fold(512.0f32, f32::max) * 1.15;
    for (si, s) in sections(d, v, hist).iter().enumerate() {
        let x = xs[si + 1];
        tile(p, (x, m, sw, ih), RAD * k, card_c(&ink));
        let ix = x + pd;
        let iw = sw - 2.0 * pd;
        let head = m + pd + lcap;
        sec_head(p, ix, head, iw, s, lcap, &ink, k);
        let hc = 70.0 * k;
        let hb = head + 16.0 * k + hc;
        let cap = hero_fit(p, &s.hero, s.hero_unit, hc, iw * 0.55);
        hero(p, ix, hb, &s.hero, s.hero_unit, cap, hero_c(s.hero_color, &ink), ink.muted);
        // what the chart shows, with its current value, right of the hero
        let (series, max, what, now): (&VecDeque<f32>, f32, &str, String) = match si {
            0 => (hist.0, 100.0, "Load \u{b7} 2 min", vu_s(&s.cells[0])),
            1 => (hist.1, 100.0, "Load \u{b7} 2 min", vu_s(&s.cells[0])),
            _ => (hist.2, net_max, "Download \u{b7} 2 min", {
                let dn = hist.2.back().copied().unwrap_or(0.0);
                if dn >= 1024.0 { format!("{:.1} MB/s", dn / 1024.0) } else { format!("{dn:.0} KB/s") }
            }),
        };
        let nb = hb - 30.0 * k;
        text_r(p, ix + iw, nb, &now, 30.0 * k, W::Regular, s.colors[0]);
        let what_w = spaced_width(p, &what.to_uppercase(), 12.0 * k, W::Medium, 1.5 * k);
        caps(p, ix + iw - what_w, hb, what, 12.0 * k, ink.dim, k);
        // readings: two rows of three at the bottom
        let vcap = 32.0 * k;
        let ch = kv_h(12.5 * k, vcap, k);
        let rows_top = m + ih - pd - 2.0 * ch - 18.0 * k;
        let cw = (iw / 3.0).floor();
        for (ci, c) in s.cells.iter().enumerate() {
            let cx = ix + (ci % 3) as f32 * cw;
            let cy = rows_top + (ci / 3) as f32 * (ch + 18.0 * k);
            kv(p, cx, cy, cw - 14.0 * k, c, 12.5 * k, vcap, &ink, k);
        }
        let cy0 = hb + 20.0 * k;
        let chh = rows_top - 20.0 * k - cy0;
        area_chart(p, (ix, cy0, iw, chh), series, max, s.colors, &ink, k);
    }
}

// =================================================================== CLUSTER
//
// A car instrument cluster: three dials with a real scale (CPU in the middle, larger), the
// readings of each dial in the two tiles under its wings, clock and weather on the left,
// the song on the right.

/// A dial: ticks every 2.5 %, numbered every 25 %, a lit arc up to the value and a marker.
#[allow(clippy::too_many_arguments)]
fn dial(p: &mut Painter, cx: f32, cy: f32, r: f32, frac: Option<f32>, colors: [Rgb; 2], scale: [&str; 5], ink: &Ink, k: f32) {
    let sweep = 240.0;
    let start = -120.0;
    let ang = |f: f32| (start + sweep * f).to_radians();
    let pt = |a: f32, rr: f32| (cx + rr * a.sin(), cy - rr * a.cos());
    // ticks
    for i in 0..=40 {
        let f = i as f32 / 40.0;
        let a = ang(f);
        let major = i % 10 == 0;
        let mid = i % 5 == 0;
        let len = if major { 16.0 } else if mid { 11.0 } else { 6.0 } * k;
        let lit = frac.is_some_and(|v| f <= v + 0.001);
        let c = if lit { gradient_at(&colors, f) } else { WHITE };
        let al = if lit { 255 } else if major { 150 } else { 60 };
        capsule(p, pt(a, r - 2.0 * k), pt(a, r - 2.0 * k - len), if major { 1.6 } else { 1.0 } * k, c, al);
        if major {
            let lbl = scale[i / 10];
            let (lx, ly) = pt(a, r - 38.0 * k);
            let lc = 13.0 * k;
            let lw = p.width(lbl, lc, W::Regular);
            p.text(lx - lw / 2.0, ly + lc / 2.0, lbl, lc, W::Regular, ink.dim, 255);
        }
    }
    // arc inside the ticks
    let ar = r - 22.0 * k;
    arc2(p, cx, cy, ar, 5.0 * k, start, sweep, &|_| WHITE, 22);
    if let Some(f) = frac.map(|f| f.clamp(0.0, 1.0)).filter(|f| *f > 0.003) {
        let cols = colors;
        arc2(p, cx, cy, ar, 5.0 * k, start, sweep * f, &move |t| gradient_at(&cols, t), 255);
        // marker: a short needle across ticks and arc
        let a = ang(f);
        capsule(p, pt(a, ar - 12.0 * k), pt(a, r + 2.0 * k), 2.6 * k, WHITE, 255);
    }
}

fn cluster(p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) {
    let k = sc.k;
    let ink = dark_ink();
    rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    let (m, g, pd) = (M * k, G * k, PD * k);
    let side = 352.0 * k;
    let ih = h - 2.0 * m;
    let secs = sections(d, v, hist);
    // ---- left: clock, weather, network + uptime
    let hcap = 90.0 * k;
    let t_h = (pd + LC * k + 18.0 * k + hcap + pd).round();
    let n_h = 96.0 * k;
    let w_h = ih - t_h - n_h - 2.0 * g;
    clock_tile(p, (m, m, side, t_h), v, &ink, k, hcap);
    weather_tile(p, (m, m + t_h + g, side, w_h), v, &ink, k);
    let ny = m + t_h + w_h + 2.0 * g;
    tile(p, (m, ny, side, n_h), RAD * k, card_c(&ink));
    let ch = kv_h(12.5 * k, 32.0 * k, k);
    let half = ((side - 2.0 * pd) / 2.0).floor();
    kv(p, m + pd, ny + ((n_h - ch) / 2.0).round(), half + 34.0 * k, &secs[0].cells[5], 12.5 * k, 32.0 * k, &ink, k);
    kv(p, m + pd + half + 50.0 * k, ny + ((n_h - ch) / 2.0).round(), half - 50.0 * k, &secs[0].cells[4], 12.5 * k, 32.0 * k, &ink, k);

    // ---- right: the song, the cover as large as the tile allows
    let rx = w - m - side;
    tile(p, (rx, m, side, ih), RAD * k, card_c(&ink));
    let mp = 18.0 * k;
    let cs = (side - 2.0 * mp).min(ih - 2.0 * mp - 96.0 * k);
    let cx = rx + (side - cs) / 2.0;
    let cy = m + mp;
    match (cover, d.track) {
        (Some(c), Some(_)) => blit_cover(p, c, cx, cy, cs, 14.0 * k),
        _ => {
            p.layer.fill_rrect((cx, cy, cs, cs), 14.0 * k, &|t| lerp_rgb((40, 42, 52), (24, 25, 32), t), 255);
            music_note(p, cx + cs / 2.0, cy + cs / 2.0, cs * 0.28, ink.dim);
        }
    }
    let tw = side - 2.0 * mp;
    let tx = rx + mp;
    let tb = cy + cs + 30.0 * k;
    match d.track {
        Some(t) => {
            title_line(p, tx, tb, tw, &clean_title(&t.title), 19.0 * k, W::Medium, ink.text);
            let ab = tb + 25.0 * k;
            let mut aw = tw;
            if let Some((el, tot, f)) = track_times(d) {
                let s = format!("{el} / {tot}");
                aw -= text_r(p, tx + tw, ab, &s, 14.5 * k, W::Regular, ink.muted) + 14.0 * k;
                let accent = cover.map_or(th().cpu[0], |c| c.accent);
                bar(p, tx, m + ih - mp - 4.0 * k, tw, 4.0 * k, Some(f), [accent, lerp_rgb(accent, WHITE, 0.4)], (WHITE, 26));
            }
            let a = ellipsize(p, &t.artist, 15.0 * k, W::Regular, aw.max(10.0));
            p.text(tx, ab, &a, 15.0 * k, W::Regular, ink.muted, 255);
        }
        None => {
            p.text(tx, tb, "Nothing playing", 18.0 * k, W::Regular, ink.dim, 255);
        }
    }

    // ---- the dials: GPU, CPU (larger, in the middle), memory
    let cx0 = m + side + g;
    let cw = rx - g - cx0;
    let rb = (ih / 2.0 - 6.0 * k).min(cw * 0.18);
    let rs = (rb * 0.85).floor();
    let gap = ((cw - 2.0 * rb - 4.0 * rs) / 2.0).max(8.0 * k);
    let mid = cx0 + cw / 2.0;
    let cy = m + ih / 2.0 - 4.0 * k;
    let centers = [mid - rb - gap - rs, mid, mid + rb + gap + rs];
    let order = [1usize, 0, 2];
    for (pos, &si) in order.iter().enumerate() {
        let s = &secs[si];
        let big = si == 0;
        let r = if big { rb } else { rs };
        let dx = centers[pos];
        let pct = s.hero_unit == "%";
        let scale = if pct { ["", "25", "50", "75", ""] } else { ["", "40", "60", "80", ""] };
        let frac = if pct { s.hero_frac } else { s.hero.parse::<f32>().ok().map(|t| ((t - 20.0) / 80.0).clamp(0.0, 1.0)) };
        dial(p, dx, cy, r, frac, s.colors, scale, &ink, k);
        // name
        let nc = 14.0 * k;
        let ny = cy - r * 0.40;
        let nw = spaced_width(p, &s.name.to_uppercase(), nc, W::Medium, 1.5 * k);
        circle(p, dx - nw / 2.0 - 10.0 * k, ny - nc * 0.5, 4.0 * k, s.colors[0], 255);
        caps(p, dx - nw / 2.0 + 2.0 * k, ny, s.name, nc, ink.muted, k);
        // hero
        let hc = if big { 96.0 } else { 80.0 } * k;
        let cap = hero_fit(p, &s.hero, s.hero_unit, hc, r * 1.2);
        let hw = p.width(&s.hero, cap, W::Light) + cap * 0.01 + p.width(s.hero_unit, (cap * 0.31).max(12.0), W::Light);
        let hb = cy + cap * 0.36;
        hero(p, dx - hw / 2.0 + cap * 0.05, hb, &s.hero, s.hero_unit, cap, hero_c(s.hero_color, &ink), ink.muted);
        // load + power under the hero
        let line = format!("{}     {}", vu_s(&s.cells[0]), vu_s(&s.cells[1]));
        let lc = p.fit(&line, 26.0 * k, r * 1.25, W::Regular);
        text_c(p, dx, hb + 12.0 * k + lc, &line, lc, W::Regular, ink.text);
        // the open bottom of the dial: two (CPU) or four (GPU, memory) readings, two per row
        let rest: &[Cell] = if big { &s.cells[2..4] } else { &s.cells[2..6] };
        let span = if big { r * 1.5 } else { r * 1.7 };
        let colw = span / 2.0;
        let lcap = 12.0 * k;
        let vcap = 28.0 * k;
        let row_h = kv_h(lcap, vcap, k) + 10.0 * k;
        let first = if big { cy + r * 0.58 } else { cy + r * 0.56 };
        for (i, c) in rest.iter().enumerate() {
            let col = i % 2;
            let row = i / 2;
            let mid_x = dx - span / 2.0 + colw * (col as f32 + 0.5);
            let top = first + row as f32 * row_h;
            let vc = vu_fit(p, &c.value, &c.unit, vcap, colw - 12.0 * k, W::Regular);
            let tw2 = vu_width(p, &c.value, &c.unit, vc, W::Regular);
            let lw = spaced_width(p, &c.label.to_uppercase(), lcap, W::Medium, 1.5 * k);
            caps(p, mid_x - lw / 2.0, top + lcap, c.label, lcap, ink.dim, k);
            let col_v = if c.value == "--" { ink.dim } else if c.color == th().text || c.color == th().dim { ink.text } else { c.color };
            vu(p, mid_x - tw2 / 2.0, top + lcap + 8.0 * k + vcap, &c.value, &c.unit, vc, W::Regular, col_v, ink.muted);
        }
    }
}

pub(super) fn paint(layout: Layout, p: &mut Painter, w: f32, h: f32, sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) -> bool {
    match layout {
        Layout::Board => board_layout(p, w, h, sc, d, v, hist, cover),
        Layout::Studio => studio(p, w, h, sc, d, v, hist, cover),
        Layout::Pulse => pulse(p, w, h, sc, d, v, hist, cover),
        Layout::Cluster => cluster(p, w, h, sc, d, v, hist, cover),
        _ => return false,
    }
    true
}
