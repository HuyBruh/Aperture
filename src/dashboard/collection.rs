//! The Aperture collection (panorama, cinema, axis, ledger, duet, orbit, atelier).
//!
//! Panels come from `layout_geometry` (12 px margins and gaps, equal columns) and are drawn
//! with the same kit as Grid: the same header, hero, readings, weather and music blocks, so
//! every layout shares one type scale and one rhythm.
use super::*;

struct Panel {
    kind: &'static str,
    r: R,
    mode: &'static str,
}

fn geometry(layout: Layout) -> Option<Vec<Panel>> {
    let g = aperture::custom::layout_geometry(layout.name());
    (!g.is_empty()).then(|| g.into_iter().map(|(kind, r, mode)| Panel { kind, r: (r[0], r[1], r[2], r[3]), mode }).collect())
}

const PRAD: f32 = 20.0;

fn surface(p: &mut Painter, r: R, ink: &Ink, k: f32) {
    let r = (r.0.round(), r.1.round(), r.2.round(), r.3.round());
    p.layer.fill_rrect_solid(r, PRAD * k, ink.card, 255);
}

/// A thin vertical rule between the hero block and the readings.
fn vrule(p: &mut Painter, x: f32, y: f32, h: f32) {
    rect(p, (x.round(), y.round(), 1.0, h.round()), WHITE, 20);
}

/// Six readings on a grid of `cols` columns inside (x, y, w, h); each block is centred in
/// its cell. Readings hidden in the editor keep their place empty.
#[allow(clippy::too_many_arguments)]
fn readings(p: &mut Painter, cells: &[Cell], x: f32, y: f32, w: f32, h: f32, cols: usize, vcap: f32, ink: &Ink, k: f32) {
    let rows = cells.len().div_ceil(cols);
    let cw = w / cols as f32;
    let rh = h / rows as f32;
    let lcap = 12.5 * k;
    let bh = kv_h(lcap, vcap, k);
    for (i, c) in cells.iter().enumerate() {
        if c.label.is_empty() {
            continue;
        }
        let cx = x + (i % cols) as f32 * cw;
        let cy = y + (i / cols) as f32 * rh + ((rh - bh) / 2.0).round();
        kv(p, cx, cy, cw - 16.0 * k, c, lcap, vcap, ink, k);
    }
}

fn hardware(p: &mut Painter, s: &Sec, r: R, mode: &str, ink: &Ink, k: f32, ts: f32) {
    let (x, y, w, h) = r;
    surface(p, r, ink, k);
    let pd = PD * k;
    let lcap = LC * k;
    match mode {
        // name + hero on the left, the six readings in one line
        "row" => {
            let lb = (w * 0.22).clamp(260.0 * k, 330.0 * k);
            let pv = (h * 0.15).clamp(14.0 * k, 20.0 * k);
            let head = y + pv + lcap;
            circle(p, x + pd + 5.0 * k, head - lcap * 0.5, 5.0 * k, s.colors[0], 255);
            caps(p, x + pd + 18.0 * k, head, s.name, lcap, ink.muted, k);
            let hc = (h - 2.0 * pv - lcap - 12.0 * k).min(80.0 * k);
            let hb = y + h - pv - 2.0 * k;
            let cap = hero_fit(p, &s.hero, s.hero_unit, hc, lb * 0.6);
            let hw = hero(p, x + pd, hb, &s.hero, s.hero_unit, cap, hero_c(s.hero_color, ink), ink.muted);
            let side = if s.side.is_empty() { s.device.clone() } else { s.side.clone() };
            let side = ellipsize(p, &side, 15.0 * k, W::Regular, lb - hw - 18.0 * k);
            text_r(p, x + pd + lb, hb, &side, 15.0 * k, W::Regular, ink.muted);
            if !s.device.is_empty() {
                let dev = ellipsize(p, &s.device, lcap, W::Regular, lb - 90.0 * k);
                text_r(p, x + pd + lb, head, &dev, lcap, W::Regular, ink.dim);
            }
            let x0 = x + pd + lb + 32.0 * k;
            vrule(p, x0 - 16.0 * k, y + 16.0 * k, h - 32.0 * k);
            let vcap = (h - 2.0 * pv - 12.5 * k - 9.0 * k).min(40.0 * k) * ts;
            readings(p, &s.cells, x0, y, x + w - pd - x0, h, 6, vcap, ink, k);
        }
        // hero block on the left (with its trend), readings 3 x 2 on the right
        "wide" => {
            let lw = 380.0 * k;
            let head = y + pd + lcap;
            let hcap = (h - 2.0 * pd - lcap - 18.0 * k).min(116.0 * k);
            sec_top(p, x + pd, head, lw, s, hcap, ink, k, (WHITE, 26));
            let x0 = x + pd + lw + 40.0 * k;
            vrule(p, x0 - 20.0 * k, y + 18.0 * k, h - 36.0 * k);
            readings(p, &s.cells, x0, y + pd, x + w - pd - x0, h - 2.0 * pd, 3, 42.0 * k * ts, ink, k);
        }
        // tall and narrow: hero on top, readings 2 x 3 under it
        "column" => {
            let iw = w - 2.0 * pd;
            let head = y + pd + lcap;
            let hb = sec_top(p, x + pd, head, iw, s, 86.0 * k, ink, k, (WHITE, 26));
            let top = hb + 24.0 * k;
            rect(p, (x + pd, (top - 1.0).round(), iw, 1.0), WHITE, 20);
            readings(p, &s.cells, x + pd, top, iw, y + h - pd - top, 2, 36.0 * k * ts, ink, k);
        }
        // a 270° gauge with the hero inside, readings 3 x 2 under it
        "ring" => {
            let iw = w - 2.0 * pd;
            let head = y + pd + lcap;
            sec_head(p, x + pd, head, iw, s, lcap, ink, k);
            let vcap = 28.0 * k * ts;
            let bh = kv_h(12.5 * k, vcap, k);
            let rows_top = y + h - pd - 2.0 * bh - 30.0 * k;
            let gtop = head + 22.0 * k;
            let gr = ((rows_top - 18.0 * k - gtop) / 2.0).min(iw / 2.0 - 16.0 * k);
            gauge(p, x + w / 2.0, gtop + gr, gr, s, ink, k);
            readings(p, &s.cells, x + pd, rows_top - 6.0 * k, iw + 10.0 * k, y + h - pd - rows_top + 12.0 * k, 3, vcap, ink, k);
        }
        // header, hero (sized to what is left) and readings 3 x 2 at the bottom
        _ => {
            let iw = w - 2.0 * pd;
            let head = y + pd + lcap;
            let vcap = (if h >= 300.0 * k { 30.0 } else { 28.0 }) * k * ts;
            let bh = kv_h(12.5 * k, vcap, k);
            // two rows with room between a value and the next label
            let rows_h = 2.0 * bh + 32.0 * k;
            let rows_top = y + h - pd - rows_h;
            let hcap = (rows_top - 28.0 * k - head - 18.0 * k).clamp(44.0 * k, 100.0 * k);
            let hb = sec_top(p, x + pd, head, iw, s, hcap, ink, k, (WHITE, 26));
            let top = (hb + 28.0 * k).max(rows_top);
            rect(p, (x + pd, (top - 14.0 * k).round(), iw, 1.0), WHITE, 18);
            readings(p, &s.cells, x + pd, top - 8.0 * k, iw, y + h - pd - top + 16.0 * k, 3, vcap, ink, k);
        }
    }
}

fn clock(p: &mut Painter, r: R, v: &Vm, ink: &Ink, k: f32) {
    let (x, y, w, h) = r;
    surface(p, r, ink, k);
    if h <= 120.0 * k {
        // time on the left, weekday and date stacked beside it
        let pv = (h * 0.2).max(14.0 * k);
        let cap = (h - 2.0 * pv).min(58.0 * k);
        let pd = PD * k;
        let mid = y + h / 2.0;
        let tw = p.text(x + pd - cap * 0.04, mid + cap / 2.0, &v.hhmm, cap, W::Light, ink.text, 255);
        let dx = x + pd + tw + 22.0 * k;
        let room = x + w - pd - dx;
        let (c1, c2) = (12.5 * k, 16.0 * k);
        let top = mid - (c1 + 9.0 * k + c2) / 2.0;
        let wd = v.wd.to_uppercase();
        let ww = spaced_width(p, &wd, c1, W::Medium, 1.5 * k);
        let c1f = if ww > room { (c1 * room / ww).floor() } else { c1 };
        caps(p, dx, top + c1, &wd, c1f, ink.muted, k);
        let c2f = p.fit(&v.date, c2, room, W::Regular);
        p.text(dx, top + c1 + 9.0 * k + c2, &v.date, c2f, W::Regular, ink.text, 255);
        return;
    }
    let hcap = h - 2.0 * PD * k - LC * k - 18.0 * k;
    clock_tile(p, r, v, ink, k, hcap.min(112.0 * k));
}

fn weather(p: &mut Painter, r: R, v: &Vm, ink: &Ink, k: f32) {
    surface(p, r, ink, k);
    weather_in(p, r, v, ink, k);
}

pub(super) fn paint(layout: Layout, p: &mut Painter, w: f32, h: f32, _sc: &Scale, d: &Data, v: &Vm, hist: Hist, cover: Option<&Cover>) -> bool {
    let Some(panels) = geometry(layout) else { return false };
    let k = (w / 1920.0).min(h / 462.0);
    let base = dark_ink();
    let ink = Ink { card: lerp_rgb(base.bg, WHITE, 0.06), ..base };
    let settings = opts().pro;
    let editing = settings.edit_layout == layout.name();
    let ts = if editing { settings.edit_text_scale } else { 1.0 };
    if !(settings.edit_background && editing) {
        rect(p, (0.0, 0.0, w, h), ink.bg, 255);
    }
    let mut secs = sections(d, v, hist);
    if editing {
        for (group, section) in ["cpu", "gpu", "mem"].iter().zip(secs.iter_mut()) {
            for cell in &mut section.cells {
                if settings.edit_hidden.split(',').any(|key| key == format!("{}:{}", group, cell.label.to_ascii_lowercase())) {
                    cell.label = "";
                }
            }
        }
    }
    for panel in panels {
        let edit = if editing { settings.edit_widgets.iter().find(|w| w.sensor.strip_prefix("group_") == Some(panel.kind)) } else { None };
        if edit.is_some_and(|w| !w.panel) {
            continue;
        }
        let r = edit.map(|w| (w.x * k, w.y * k, w.w * k, w.h * k)).unwrap_or((panel.r.0 * k, panel.r.1 * k, panel.r.2 * k, panel.r.3 * k));
        let mut ink = ink;
        if let Some(w) = edit {
            if w.color != [242, 243, 239] {
                ink.text = (w.color[0], w.color[1], w.color[2]);
            }
        }
        match panel.kind {
            "cpu" => hardware(p, &secs[0], r, panel.mode, &ink, k, ts),
            "gpu" => hardware(p, &secs[1], r, panel.mode, &ink, k, ts),
            "mem" => hardware(p, &secs[2], r, panel.mode, &ink, k, ts),
            "time" => clock(p, r, v, &ink, k),
            "weather" => weather(p, r, v, &ink, k),
            "music" => {
                surface(p, r, &ink, k);
                if !settings.logo.is_empty() {
                    let size = (r.2.min(r.3) - 36.0 * k).max(1.0);
                    cover_or_note(p, cover, r.0 + (r.2 - size) / 2.0, r.1 + (r.3 - size) / 2.0, size, 14.0 * k, ink.dim, (ink.card, ink.bg));
                } else {
                    music_in(p, r, d, cover, &ink, k);
                }
            }
            _ => {}
        }
    }
    true
}
