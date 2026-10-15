//! Smooth (anti-aliased) text for the `tiles` layout.
//!
//! The font is Poppins (SIL Open Font License, see `assets/OFL.txt`), reduced to
//! the Latin characters (three weights, ~15 KB each) and embedded in the program.
//! Each glyph is rasterized once per size and kept in a small cache, so drawing a
//! frame only copies coverage values — the same cost as the pixel font.
//!
//! Sizes are given as the height of the capital letters / digits in pixels
//! (`cap` below), which is what a layout cares about; the conversion to the font's
//! em size is done here.

use fontdue::{Font, FontSettings};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Weight {
    Light,
    Regular,
    Medium,
}

static LIGHT: &[u8] = include_bytes!("../assets/Poppins-Light-subset.ttf");
static REGULAR: &[u8] = include_bytes!("../assets/Poppins-Regular-subset.ttf");
static MEDIUM: &[u8] = include_bytes!("../assets/Poppins-Medium-subset.ttf");

/// Height of the digits, as a fraction of the em size (Poppins: digits ~0.725, capitals 0.698).
const CAP_RATIO: f32 = 0.725;

struct Fonts {
    light: Font,
    regular: Font,
    medium: Font,
    /// A system font for the characters the embedded subset does not have
    /// (Vietnamese, Greek, Cyrillic, ... in song titles). Windows only.
    fallback: Option<Font>,
    fallback_bold: Option<Font>,
}

fn system_font(names: &[&str]) -> Option<Font> {
    #[cfg(windows)]
    {
        let dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        for n in names {
            if let Ok(bytes) = std::fs::read(format!("{dir}\\Fonts\\{n}")) {
                if let Ok(f) = Font::from_bytes(bytes, FontSettings::default()) {
                    return Some(f);
                }
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        let _ = names;
        None
    }
}

fn fonts() -> &'static Fonts {
    static F: OnceLock<Fonts> = OnceLock::new();
    F.get_or_init(|| {
        let load = |data: &[u8]| Font::from_bytes(data, FontSettings::default()).expect("embedded font is valid");
        Fonts {
            light: load(LIGHT),
            regular: load(REGULAR),
            medium: load(MEDIUM),
            fallback: system_font(&["segoeui.ttf", "arial.ttf"]),
            fallback_bold: system_font(&["seguisb.ttf", "segoeui.ttf", "arial.ttf"]),
        }
    })
}

fn font(w: Weight) -> &'static Font {
    let f = fonts();
    match w {
        Weight::Light => &f.light,
        Weight::Regular => &f.regular,
        Weight::Medium => &f.medium,
    }
}

struct Glyph {
    w: usize,
    h: usize,
    /// Offset of the bitmap's left edge from the pen position.
    xmin: i32,
    /// Offset of the bitmap's BOTTOM edge above the baseline (negative = below).
    ymin: i32,
    advance: f32,
    cov: Vec<u8>,
}

thread_local! {
    static CACHE: RefCell<HashMap<(Weight, char, u32), Rc<Glyph>>> = RefCell::new(HashMap::new());
}

/// Characters the subset does not have are shown as the closest thing it has.
fn usable(c: char, w: Weight) -> char {
    if font(w).lookup_glyph_index(c) != 0 {
        return c;
    }
    match c {
        '\u{2212}' => '-',
        '\u{2019}' | '\u{2018}' => '\'',
        '\u{201C}' | '\u{201D}' => '"',
        '\u{2022}' | '\u{25CF}' => '\u{2022}',
        _ => '?',
    }
}

fn glyph(c: char, px: f32, w: Weight) -> Rc<Glyph> {
    // Half-pixel steps are plenty and keep the cache small.
    let key = ((px * 2.0).round().max(1.0)) as u32;
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() > 3000 {
            cache.clear();
        }
        cache
            .entry((w, c, key))
            .or_insert_with(|| {
                let f = fonts();
                let fb = if w == Weight::Medium { f.fallback_bold.as_ref() } else { f.fallback.as_ref() };
                let (m, cov) = match fb {
                    Some(fbf) if font(w).lookup_glyph_index(c) == 0 && fbf.lookup_glyph_index(c) != 0 => {
                        fbf.rasterize(c, key as f32 / 2.0)
                    }
                    _ => font(w).rasterize(usable(c, w), key as f32 / 2.0),
                };
                Rc::new(Glyph { w: m.width, h: m.height, xmin: m.xmin, ymin: m.ymin, advance: m.advance_width, cov })
            })
            .clone()
    })
}

/// Em size (pixels) whose capital letters are `cap` pixels tall.
pub fn px_for_cap(cap: f32) -> f32 {
    (cap / CAP_RATIO).max(1.0)
}

/// Width of `text` in pixels at the given em size.
pub fn text_width(text: &str, px: f32, w: Weight) -> f32 {
    text.chars().map(|c| glyph(c, px, w).advance).sum()
}

/// Calls `plot(x, y, coverage 0-255)` for every covered pixel of `text`, with the pen
/// starting at `x` on the baseline `baseline`. Returns the advance (width) of the text.
pub fn draw_with(text: &str, x: f32, baseline: f32, px: f32, w: Weight, mut plot: impl FnMut(i32, i32, u8)) -> f32 {
    let mut pen = x;
    for c in text.chars() {
        let g = glyph(c, px, w);
        let gx = (pen.round() as i32) + g.xmin;
        let gy = (baseline.round() as i32) - g.ymin - g.h as i32;
        for row in 0..g.h {
            let line = &g.cov[row * g.w..(row + 1) * g.w];
            for (col, &a) in line.iter().enumerate() {
                if a != 0 {
                    plot(gx + col as i32, gy + row as i32, a);
                }
            }
        }
        pen += g.advance;
    }
    pen - x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_fonts_load_and_measure() {
        for w in [Weight::Light, Weight::Regular, Weight::Medium] {
            let a = text_width("00:00", 40.0, w);
            let b = text_width("00:00", 80.0, w);
            assert!(a > 50.0 && b > a * 1.8 && b < a * 2.2, "{w:?}: {a} {b}");
        }
        assert!(text_width("", 40.0, Weight::Regular) == 0.0);
    }

    #[test]
    fn digits_have_the_requested_cap_height() {
        let cap = 60.0;
        let px = px_for_cap(cap);
        let (mut top, mut bottom) = (i32::MAX, i32::MIN);
        draw_with("8", 0.0, 100.0, px, Weight::Regular, |_, y, a| {
            if a > 128 {
                top = top.min(y);
                bottom = bottom.max(y);
            }
        });
        let h = (bottom - top + 1) as f32;
        assert!((h - cap).abs() <= 4.0, "digit height {h} for cap {cap}");
        assert!(bottom <= 101, "the baseline is at the bottom of a digit ({bottom})");
    }

    #[test]
    fn accents_degree_and_unknown_characters_draw_something() {
        for s in ["°C", "à è é ì ò ù", "€", "漢"] {
            let mut n = 0;
            draw_with(s, 0.0, 40.0, 30.0, Weight::Medium, |_, _, _| n += 1);
            assert!(n > 0, "{s:?} drew nothing");
        }
    }
}
