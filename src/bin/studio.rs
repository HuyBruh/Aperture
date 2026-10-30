//! `huybruh_dash` (Aperture): the friendly settings app of the pro dashboard (English / Tiếng Việt).
//!
//! It edits `trofeo.conf` next to it; the running `aperture-display` re-reads the file every
//! second, so every change shows on the display right away. Pictures (the live preview and
//! the layout thumbnails) are drawn by `aperture-display.exe --preview-png`, so they always match
//! the display. The "Sensors & help" page explains what is missing and how to fix it.
#![cfg_attr(windows, windows_subsystem = "windows")]

use eframe::egui::{self, Align, Color32, FontFamily, FontId, Layout as ELayout, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use aperture::conf_edit::ConfDoc;
use aperture::sensors_ext::{self, Snapshot, Source};

// ------------------------------------------------------------------------ look

const BG: Color32 = Color32::from_rgb(16, 18, 19);
const CARD: Color32 = Color32::from_rgb(25, 28, 30);
const CARD_HI: Color32 = Color32::from_rgb(27, 30, 36);
const LINE: Color32 = Color32::from_rgb(38, 41, 49);
const TEXT: Color32 = Color32::from_rgb(236, 238, 242);
const MUTED: Color32 = Color32::from_rgb(150, 155, 166);
const DIM: Color32 = Color32::from_rgb(100, 105, 116);
const ACCENT: Color32 = Color32::from_rgb(137, 178, 151);
const GOOD: Color32 = Color32::from_rgb(52, 199, 123);
const WARN: Color32 = Color32::from_rgb(255, 176, 32);
const BAD: Color32 = Color32::from_rgb(255, 84, 84);

const R: f32 = 12.0;
const GAP: f32 = 16.0;

fn font(size: f32, w: Weight) -> FontId {
    FontId::new(size, FontFamily::Name(w.name().into()))
}

#[derive(Clone, Copy)]
enum Weight {
    Regular,
    Medium,
    Semibold,
}

impl Weight {
    fn name(self) -> &'static str {
        match self {
            Weight::Regular => "inter",
            Weight::Medium => "inter-medium",
            Weight::Semibold => "inter-semibold",
        }
    }
}

fn setup_style(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let add = |fonts: &mut egui::FontDefinitions, name: &str, data: &'static [u8]| {
        fonts.font_data.insert(name.to_string(), egui::FontData::from_static(data));
        let mut fam = vec![name.to_string()];
        fam.extend(fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
        fonts.families.insert(FontFamily::Name(name.into()), fam);
    };
    add(&mut fonts, "inter", include_bytes!("../../assets/Inter-Regular-subset.ttf"));
    add(&mut fonts, "inter-medium", include_bytes!("../../assets/Inter-Medium-subset.ttf"));
    add(&mut fonts, "inter-semibold", include_bytes!("../../assets/Inter-SemiBold-subset.ttf"));
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "inter".into());
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (egui::TextStyle::Heading, FontId::new(20.0, FontFamily::Name("inter-semibold".into()))),
        (egui::TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (egui::TextStyle::Button, FontId::new(14.0, FontFamily::Name("inter-medium".into()))),
        (egui::TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        (egui::TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
    ]
    .into();
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(14.0, 7.0);
    style.spacing.interact_size.y = 32.0;
    style.spacing.slider_width = 260.0;
    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = Color32::from_rgb(14, 15, 19);
    v.faint_bg_color = CARD_HI;
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.hyperlink_color = ACCENT;
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.rounding = Rounding::same(8.0);
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
    v.widgets.inactive.bg_fill = CARD_HI;
    v.widgets.inactive.weak_bg_fill = CARD_HI;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);
    v.widgets.hovered.bg_fill = Color32::from_rgb(36, 40, 48);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(36, 40, 48);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(70, 76, 90));
    v.widgets.active.bg_fill = Color32::from_rgb(44, 49, 59);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(44, 49, 59);
    v.window_rounding = Rounding::same(R);
    v.menu_rounding = Rounding::same(8.0);
    ctx.set_style(style);
}

// ------------------------------------------------------------------------ language

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    En,
    Vi,
}

// ------------------------------------------------------------------------ catalog

#[derive(Clone, Copy)]
struct LayoutInfo {
    key: &'static str,
    name: &'static str,
    en: &'static str,
    vi: &'static str,
    portrait: bool,
}

include!("../dashboard/studio_catalog.rs");

const LAYOUTS: &[LayoutInfo] = &[
    APERTURE_LAYOUTS[0],
    APERTURE_LAYOUTS[1],
    APERTURE_LAYOUTS[2],
    APERTURE_LAYOUTS[3],
    APERTURE_LAYOUTS[4],
    LayoutInfo { key: "focus", name: "Focus", en: "Huge numbers, nothing else", vi: "Số cực lớn, tối giản", portrait: false },
    LayoutInfo { key: "grid", name: "Grid", en: "Even tiles like phone widgets", vi: "Ô đều nhau như widget điện thoại", portrait: false },
    LayoutInfo { key: "stack", name: "Stack", en: "Three full-width rows, six readings each", vi: "Ba hàng ngang, mỗi hàng sáu thông số", portrait: false },
    LayoutInfo { key: "mosaic", name: "Mosaic", en: "Eight tiles, one big number each", vi: "Tám ô, mỗi ô một con số lớn", portrait: false },
    LayoutInfo { key: "mixer", name: "Mixer", en: "Level faders like a mixing console", vi: "Cần gạt mức như bàn mixer", portrait: false },
    LayoutInfo { key: "chart", name: "Chart", en: "Temperature chart and a full table", vi: "Biểu đồ nhiệt độ và bảng thông số", portrait: false },
    LayoutInfo { key: "player", name: "Player", en: "Big album cover, music first", vi: "Ảnh bìa lớn, ưu tiên nhạc", portrait: false },
    LayoutInfo { key: "tint", name: "Tint", en: "Tiles tinted with each section's color", vi: "Ô nhuộm màu theo từng mục", portrait: false },
    LayoutInfo { key: "glass", name: "Glass", en: "Frosted tiles over a soft color wallpaper", vi: "Ô kính mờ trên nền màu dịu", portrait: false },
    LayoutInfo { key: "gauge", name: "Gauge", en: "A big arc gauge for CPU, GPU and memory", vi: "Đồng hồ vòng cung lớn cho CPU, GPU, RAM", portrait: false },
    LayoutInfo { key: "swiss", name: "Swiss", en: "Swiss typographic grid", vi: "Lưới chữ phong cách Thụy Sĩ", portrait: false },
    LayoutInfo { key: "classic", name: "Classic", en: "Panels in a row", vi: "Các ô xếp ngang", portrait: false },
    LayoutInfo { key: "cockpit", name: "Cockpit", en: "Two gauges, clock in the middle", vi: "Hai đồng hồ đo, giờ ở giữa", portrait: false },
    LayoutInfo { key: "bento", name: "Bento", en: "Colored tiles", vi: "Ô màu kiểu bento", portrait: false },
    LayoutInfo { key: "terminal", name: "Terminal", en: "Green console", vi: "Màn hình console xanh", portrait: false },
    LayoutInfo { key: "chamfer", name: "Chamfer", en: "Angular game HUD", vi: "Khung vát góc kiểu game", portrait: false },
    LayoutInfo { key: "rings", name: "Rings", en: "Activity rings", vi: "Vòng tròn hoạt động", portrait: false },
    LayoutInfo { key: "dots", name: "Dots", en: "LED dot matrix", vi: "Chữ ma trận chấm LED", portrait: false },
    LayoutInfo { key: "lock", name: "Lock", en: "Phone lock screen", vi: "Màn hình khóa điện thoại", portrait: true },
    LayoutInfo { key: "equalizer", name: "Equalizer", en: "LED level meters", vi: "Cột đèn LED", portrait: true },
    LayoutInfo { key: "receipt", name: "Receipt", en: "Paper receipt", vi: "Hóa đơn giấy", portrait: true },
    LayoutInfo { key: "synthwave", name: "Synthwave", en: "Retro sunset", vi: "Hoàng hôn retro", portrait: true },
    LayoutInfo { key: "blueprint", name: "Blueprint", en: "Technical drawing", vi: "Bản vẽ kỹ thuật", portrait: true },
];

#[derive(Clone, Copy)]
struct ThemeInfo {
    key: &'static str,
    name: &'static str,
    colors: [(u8, u8, u8); 3],
}

const THEMES: &[ThemeInfo] = &[
    APERTURE_PALETTES[0],
    APERTURE_PALETTES[1],
    APERTURE_PALETTES[2],
    APERTURE_PALETTES[3],
    APERTURE_PALETTES[4],
    ThemeInfo { key: "nzxt", name: "NZXT", colors: [(124, 92, 255), (34, 211, 238), (244, 114, 182)] },
    ThemeInfo { key: "graphite", name: "Graphite", colors: [(150, 150, 158), (210, 210, 214), (255, 255, 255)] },
    ThemeInfo { key: "crimson", name: "Crimson", colors: [(220, 20, 50), (255, 99, 99), (251, 146, 60)] },
    ThemeInfo { key: "arctic", name: "Arctic", colors: [(56, 189, 248), (45, 212, 191), (186, 230, 253)] },
    ThemeInfo { key: "ember", name: "Ember", colors: [(251, 146, 60), (250, 204, 21), (254, 205, 140)] },
    ThemeInfo { key: "neon", name: "Neon", colors: [(255, 0, 153), (0, 229, 255), (250, 255, 0)] },
];

// ------------------------------------------------------------------------ rendering

fn find_main_exe() -> Option<PathBuf> {
    let name = if cfg!(windows) { "aperture-display.exe" } else { "aperture-display" };
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let p = dir.join(name);
    p.is_file().then_some(p)
}

/// Draws `text` (a configuration) with `layout` forced, through the main program.
fn render(exe: &Path, workdir: &Path, text: &str, layout: Option<&str>, music: bool, tag: &str) -> Option<egui::ColorImage> {
    let tmp = std::env::temp_dir();
    let id = std::process::id();
    let conf = tmp.join(format!("aperture_{id}_{tag}.conf"));
    let png = tmp.join(format!("aperture_{id}_{tag}.png"));
    let mut doc = ConfDoc::parse(text);
    doc.set("layout", "pro");
    doc.set("flip", "false");
    if let Some(l) = layout {
        doc.set("pro_layout", l);
    }
    // a single theme in the pictures (not the whole rotation)
    if let Some(t) = doc.get("pro_theme") {
        let first = t.split(',').next().unwrap_or("nzxt").trim().to_string();
        doc.set("pro_theme", if first == "all" { "nzxt" } else { first.as_str() });
    }
    if doc.get("pro_layout").as_deref() == Some("all") {
        doc.set("pro_layout", "focus");
    }
    if doc.get("pro_layout").as_deref() == Some("portrait") {
        doc.set("pro_layout", "lock");
    }
    std::fs::write(&conf, doc.to_text()).ok()?;
    let _ = std::fs::remove_file(&png);
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--config").arg(&conf).arg("--preview-png").arg(&png).arg("--preview-state").arg(if music { "music" } else { "idle" });
    cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    if workdir.is_dir() {
        cmd.current_dir(workdir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let ok = cmd.status().map(|s| s.success()).unwrap_or(false);
    let _ = std::fs::remove_file(&conf);
    if !ok {
        return None;
    }
    let bytes = std::fs::read(&png).ok()?;
    let _ = std::fs::remove_file(&png);
    let mut img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    // portrait pictures come out turned for the panel: show them upright
    let portrait = layout.map(|l| LAYOUTS.iter().any(|x| x.key == l && x.portrait)).unwrap_or(false);
    if portrait {
        img = image::imageops::rotate270(&img);
    }
    let size = [img.width() as usize, img.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

enum Job {
    Preview { text: String, music: bool, rev: u64 },
    Thumbs { text: String, rev: u64 },
}

enum Done {
    Preview(u64, Option<egui::ColorImage>),
    Thumb(u64, String, Option<egui::ColorImage>),
}

fn spawn_renderer(exe: PathBuf, workdir: PathBuf, ctx: egui::Context) -> (mpsc::Sender<Job>, mpsc::Receiver<Done>) {
    let (tx, rx_jobs) = mpsc::channel::<Job>();
    let (tx_done, rx) = mpsc::channel::<Done>();
    std::thread::spawn(move || {
        let mut preview: Option<(String, bool, u64)> = None;
        let mut thumbs: Option<(String, u64, usize)> = None;
        loop {
            // collect requests: only the newest preview / thumbnail set matters
            let take = |j: Job, preview: &mut Option<(String, bool, u64)>, thumbs: &mut Option<(String, u64, usize)>| match j {
                Job::Preview { text, music, rev } => *preview = Some((text, music, rev)),
                Job::Thumbs { text, rev } => *thumbs = Some((text, rev, 0)),
            };
            if preview.is_none() && thumbs.is_none() {
                match rx_jobs.recv() {
                    Ok(j) => take(j, &mut preview, &mut thumbs),
                    Err(_) => return,
                }
            }
            while let Ok(j) = rx_jobs.try_recv() {
                take(j, &mut preview, &mut thumbs);
            }
            if let Some((text, music, rev)) = preview.take() {
                let img = render(&exe, &workdir, &text, None, music, "p");
                let _ = tx_done.send(Done::Preview(rev, img));
                ctx.request_repaint();
                continue;
            }
            // one thumbnail at a time, so a new preview request is served quickly
            if let Some((text, rev, i)) = thumbs.take() {
                if let Some(l) = LAYOUTS.get(i) {
                    let img = render(&exe, &workdir, &text, Some(l.key), true, "t");
                    let _ = tx_done.send(Done::Thumb(rev, l.key.to_string(), img));
                    ctx.request_repaint();
                    thumbs = Some((text, rev, i + 1));
                }
            }
        }
    });
    (tx, rx)
}

// ------------------------------------------------------------------------ status

#[derive(Default, Clone)]
struct Status {
    /// trofeo-status.txt (written by the running display program).
    lcd: Option<String>,
    cpu_temp: Option<String>,
    cpu_fan: Option<String>,
    gpu_hotspot: Option<String>,
    /// The display program wrote its file in the last 30 s.
    alive: bool,
    pawnio: Option<bool>,
    ext: Option<(Source, Snapshot)>,
}

fn read_status(dir: &Path) -> Status {
    let mut s = Status::default();
    if let Ok(t) = std::fs::read_to_string(dir.join("trofeo-status.txt")) {
        let mut updated = 0u64;
        for line in t.lines() {
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().to_string();
                match k.trim() {
                    "updated" => updated = v.parse().unwrap_or(0),
                    "lcd" => s.lcd = Some(v),
                    "cpu_temp" => s.cpu_temp = Some(v),
                    "cpu_fan" => s.cpu_fan = Some(v),
                    "gpu_hotspot" => s.gpu_hotspot = Some(v),
                    _ => {}
                }
            }
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        s.alive = now.saturating_sub(updated) < 30;
    }
    s.alive = s.alive && process_running();
    s.pawnio = pawnio_installed();
    s.ext = sensors_ext::read_any();
    s
}

#[cfg(windows)]
fn hidden(cmd: &mut std::process::Command) -> &mut std::process::Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000)
}

#[cfg(windows)]
fn process_running() -> bool {
    let out = hidden(std::process::Command::new("tasklist").args(["/FI", "IMAGENAME eq aperture-display.exe", "/NH"])).output();
    out.map(|o| String::from_utf8_lossy(&o.stdout).to_ascii_lowercase().contains("aperture-display.exe")).unwrap_or(false)
}

#[cfg(not(windows))]
fn process_running() -> bool {
    true
}

#[cfg(windows)]
fn pawnio_installed() -> Option<bool> {
    let out = hidden(std::process::Command::new("sc").args(["query", "PawnIO"])).output().ok()?;
    Some(out.status.success())
}

#[cfg(not(windows))]
fn pawnio_installed() -> Option<bool> {
    None
}

/// Runs a PowerShell command as Administrator (one UAC prompt).
fn run_elevated(script: &str) {
    #[cfg(windows)]
    {
        let arg = format!(
            "Start-Process powershell -Verb RunAs -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-Command',\"{}\"",
            script.replace('"', "`\"")
        );
        let _ = hidden(std::process::Command::new("powershell").args(["-NoProfile", "-Command", &arg])).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = script;
    }
}

const RESTART_SCRIPT: &str = "Stop-ScheduledTask -TaskName ApertureDisplay -ErrorAction SilentlyContinue; Get-Process aperture-display -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep -Seconds 1; Start-ScheduledTask -TaskName ApertureDisplay";
const PAWNIO_SCRIPT: &str = "winget install --id namazso.PawnIO -e --silent --accept-source-agreements --accept-package-agreements; Stop-ScheduledTask -TaskName ApertureDisplay -ErrorAction SilentlyContinue; Get-Process aperture-display -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep -Seconds 1; Start-ScheduledTask -TaskName ApertureDisplay";

// ------------------------------------------------------------------------ app

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Layouts,
    Colors,
    Display,
    Features,
    Help,
}

struct App {
    lang: Lang,
    page: Page,
    path: PathBuf,
    dir: PathBuf,
    doc: ConfDoc,
    original: String,
    rev: u64,
    saved_rev: u64,
    changed_at: Option<Instant>,
    save_error: Option<String>,
    restart_needed: bool,
    // pictures
    jobs: Option<mpsc::Sender<Job>>,
    done: Option<mpsc::Receiver<Done>>,
    preview: Option<egui::TextureHandle>,
    preview_rev: u64,
    preview_music: bool,
    preview_asked: Option<(u64, bool)>,
    thumbs: HashMap<String, egui::TextureHandle>,
    thumbs_key: String,
    thumbs_asked_at: Option<Instant>,
    // status
    status: Status,
    status_rx: Option<mpsc::Receiver<Status>>,
    status_at: Option<Instant>,
    // text boxes
    city: String,
    custom: [u8; 3],
    logo: Option<egui::TextureHandle>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> App {
        setup_style(&cc.egui_ctx);
        let path = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
            std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("trofeo.conf"))).unwrap_or_else(|| PathBuf::from("trofeo.conf"))
        });
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|_| "layout = pro\npro_layout = axis\npro_theme = mineral\n".to_string());
        let doc = ConfDoc::parse(&text);
        let lang = match doc.get("studio_language").as_deref() {
            Some("vi") => Lang::Vi,
            Some("en") => Lang::En,
            _ => {
                let sys = std::env::var("LANG").unwrap_or_default();
                if sys.starts_with("vi") || system_is_vietnamese() { Lang::Vi } else { Lang::En }
            }
        };
        let city = doc.get("weather_city").unwrap_or_default();
        let custom = doc
            .get("pro_theme")
            .and_then(|t| parse_hex(&t))
            .or_else(|| doc.get("pro_accent").and_then(|t| parse_hex(&t)))
            .unwrap_or([255, 106, 0]);
        let (jobs, done) = match find_main_exe() {
            Some(exe) => {
                let (a, b) = spawn_renderer(exe, dir.clone(), cc.egui_ctx.clone());
                (Some(a), Some(b))
            }
            None => (None, None),
        };
        App {
            lang,
            page: match std::env::var("TROFEO_STUDIO_PAGE").as_deref() {
                Ok("colors") => Page::Colors,
                Ok("display") => Page::Display,
                Ok("features") => Page::Features,
                Ok("help") => Page::Help,
                _ => Page::Layouts,
            },
            path,
            dir,
            doc,
            original: text,
            rev: 1,
            saved_rev: 1,
            changed_at: None,
            save_error: None,
            restart_needed: false,
            jobs,
            done,
            preview: None,
            preview_rev: 0,
            preview_music: true,
            preview_asked: None,
            thumbs: HashMap::new(),
            thumbs_key: String::new(),
            thumbs_asked_at: None,
            status: Status::default(),
            status_rx: None,
            status_at: None,
            city,
            custom,
            logo: image::load_from_memory(include_bytes!("../../assets/studio-icon.png")).ok().map(|i| {
                let i = i.to_rgba8();
                let img = egui::ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], i.as_raw());
                cc.egui_ctx.load_texture("logo", img, egui::TextureOptions::LINEAR)
            }),
        }
    }

    fn t<'a>(&self, en: &'a str, vi: &'a str) -> &'a str {
        match self.lang {
            Lang::En => en,
            Lang::Vi => vi,
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        self.doc.get(key)
    }

    fn get_or(&self, key: &str, def: &str) -> String {
        self.get(key).unwrap_or_else(|| def.to_string())
    }

    fn set(&mut self, key: &str, v: &str) {
        if self.doc.get(key).as_deref() != Some(v) {
            self.doc.set(key, v);
            self.touch();
        }
    }

    fn unset(&mut self, key: &str) {
        if self.doc.is_set(key) {
            self.doc.unset(key);
            self.touch();
        }
    }

    fn touch(&mut self) {
        self.rev += 1;
        self.changed_at = Some(Instant::now());
    }

    fn bool_of(&self, key: &str, def: bool) -> bool {
        match self.get(key).map(|v| v.to_ascii_lowercase()) {
            Some(v) => matches!(v.as_str(), "true" | "yes" | "on" | "1"),
            None => def,
        }
    }

    /// Writes the file a moment after the last change (the display picks it up by itself).
    fn autosave(&mut self) {
        if self.rev == self.saved_rev {
            return;
        }
        if self.changed_at.is_some_and(|t| t.elapsed() < Duration::from_millis(250)) {
            return;
        }
        // the dashboard needs `layout = pro`
        if self.doc.get("layout").as_deref() != Some("pro") {
            self.doc.set("layout", "pro");
        }
        match std::fs::write(&self.path, self.doc.to_text()) {
            Ok(()) => {
                self.saved_rev = self.rev;
                self.save_error = None;
            }
            Err(e) => self.save_error = Some(e.to_string()),
        }
    }

    fn pump(&mut self, ctx: &egui::Context) {
        // finished pictures
        if let Some(rx) = &self.done {
            while let Ok(d) = rx.try_recv() {
                match d {
                    Done::Preview(rev, Some(img)) => {
                        if rev >= self.preview_rev {
                            self.preview_rev = rev;
                            self.preview = Some(ctx.load_texture("preview", img, egui::TextureOptions::LINEAR));
                        }
                    }
                    Done::Thumb(_, key, Some(img)) => {
                        self.thumbs.insert(key.clone(), ctx.load_texture(format!("thumb-{key}"), img, egui::TextureOptions::LINEAR));
                    }
                    _ => {}
                }
            }
        }
        // ask for a new preview after changes
        let want = (self.rev, self.preview_music);
        if self.preview_asked != Some(want) && self.changed_at.map_or(true, |t| t.elapsed() > Duration::from_millis(200)) {
            if let Some(tx) = &self.jobs {
                let _ = tx.send(Job::Preview { text: self.doc.to_text(), music: self.preview_music, rev: self.rev });
            }
            self.preview_asked = Some(want);
        }
        // thumbnails follow the colors (theme / style / clock): redraw when those change
        let key = format!(
            "{}|{}|{}|{}|{}",
            self.get_or("pro_theme", "mineral"),
            self.get_or("pro_style", "cards"),
            self.get_or("pro_accent", ""),
            self.get_or("pro_clock", "24h"),
            self.get_or("weather_unit", "c")
        );
        if key != self.thumbs_key {
            if self.thumbs_asked_at.is_none() {
                self.thumbs_asked_at = Some(Instant::now());
            }
            if self.thumbs_asked_at.is_some_and(|t| t.elapsed() > Duration::from_millis(900)) {
                if let Some(tx) = &self.jobs {
                    let _ = tx.send(Job::Thumbs { text: self.doc.to_text(), rev: self.rev });
                }
                self.thumbs_key = key;
                self.thumbs_asked_at = None;
            }
        }
        // status, every 3 s in the background
        if let Some(rx) = &self.status_rx {
            if let Ok(s) = rx.try_recv() {
                self.status = s;
                self.status_rx = None;
            }
        }
        if self.status_rx.is_none() && self.status_at.map_or(true, |t| t.elapsed() > Duration::from_secs(3)) {
            let (tx, rx) = mpsc::channel();
            let dir = self.dir.clone();
            let c = ctx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(read_status(&dir));
                c.request_repaint();
            });
            self.status_rx = Some(rx);
            self.status_at = Some(Instant::now());
        }
        ctx.request_repaint_after(Duration::from_millis(300));
    }
}

#[cfg(windows)]
fn system_is_vietnamese() -> bool {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    unsafe { GetUserDefaultUILanguage() & 0x3FF == 0x2A }
}

#[cfg(not(windows))]
fn system_is_vietnamese() -> bool {
    false
}

fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

// ------------------------------------------------------------------------ widgets

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, LINE))
        .rounding(R)
        .inner_margin(egui::Margin::same(20.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn section_title(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(title).font(font(17.0, Weight::Semibold)).color(TEXT));
    if !sub.is_empty() {
        ui.add_space(-6.0);
        ui.label(RichText::new(sub).font(font(13.0, Weight::Regular)).color(MUTED));
    }
    ui.add_space(6.0);
}

/// A row: label + help on the left, the control on the right.
fn row(ui: &mut egui::Ui, label: &str, help: &str, control: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_height(44.0);
        ui.vertical(|ui| {
            ui.set_max_width((ui.available_width() - 340.0).max(240.0));
            ui.add_space(if help.is_empty() { 12.0 } else { 4.0 });
            ui.label(RichText::new(label).font(font(14.5, Weight::Medium)).color(TEXT));
            if !help.is_empty() {
                ui.add_space(-6.0);
                ui.label(RichText::new(help).font(font(12.5, Weight::Regular)).color(MUTED));
            }
        });
        ui.with_layout(ELayout::right_to_left(Align::Center), control);
    });
}

fn divider(ui: &mut egui::Ui) {
    let r = ui.available_rect_before_wrap();
    let y = ui.cursor().top() + 2.0;
    ui.painter().hline(r.left()..=r.right(), y, Stroke::new(1.0_f32, LINE));
    ui.add_space(6.0);
}

/// iOS-style switch.
fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = Vec2::new(44.0, 26.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let bg = lerp_color(Color32::from_rgb(52, 56, 66), ACCENT, t);
    ui.painter().rect_filled(rect, Rounding::same(13.0), bg);
    let x = egui::lerp(rect.left() + 13.0..=rect.right() - 13.0, t);
    ui.painter().circle_filled(Pos2::new(x, rect.center().y), 10.0, Color32::WHITE);
    resp
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// Segmented control; returns the newly picked value.
fn segmented(ui: &mut egui::Ui, current: &str, options: &[(&str, &str)]) -> Option<String> {
    let mut picked = None;
    let h = 32.0;
    let pad = 14.0;
    let widths: Vec<f32> = options
        .iter()
        .map(|(_, label)| {
            let g = ui.painter().layout_no_wrap(label.to_string(), font(13.5, Weight::Medium), TEXT);
            g.size().x + 2.0 * pad
        })
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + 4.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total, h), Sense::hover());
    ui.painter().rect(rect, Rounding::same(9.0), Color32::from_rgb(14, 15, 19), Stroke::new(1.0_f32, LINE));
    let mut x = rect.left() + 2.0;
    for ((value, label), w) in options.iter().zip(widths) {
        let r = Rect::from_min_size(Pos2::new(x, rect.top() + 2.0), Vec2::new(w, h - 4.0));
        let resp = ui.interact(r, ui.id().with(("seg", value, label)), Sense::click());
        let on = *value == current;
        if on {
            ui.painter().rect_filled(r, Rounding::same(7.0), Color32::from_rgb(48, 53, 64));
        } else if resp.hovered() {
            ui.painter().rect_filled(r, Rounding::same(7.0), Color32::from_rgb(28, 31, 38));
        }
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, *label, font(13.5, Weight::Medium), if on { TEXT } else { MUTED });
        if resp.clicked() && !on {
            picked = Some(value.to_string());
        }
        x += w;
    }
    picked
}

fn status_dot(ui: &mut egui::Ui, color: Color32, text: &str) {
    let g = ui.painter().layout_no_wrap(text.to_string(), font(13.5, Weight::Medium), TEXT);
    let (r, _) = ui.allocate_exact_size(Vec2::new(g.size().x + 18.0, 22.0), Sense::hover());
    ui.painter().circle_filled(Pos2::new(r.left() + 5.0, r.center().y), 4.5, color);
    ui.painter().galley(Pos2::new(r.left() + 18.0, r.center().y - g.size().y / 2.0), g, TEXT);
}

fn bullet(ui: &mut egui::Ui, text: &str) {
    ui.add_space(-4.0);
    ui.label(RichText::new(format!("•   {text}")).font(font(13.0, Weight::Regular)).color(MUTED));
}

// ------------------------------------------------------------------------ pages

impl App {
    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.set_min_height(40.0);
            if let Some(logo) = &self.logo {
                ui.add(egui::Image::new((logo.id(), Vec2::new(34.0, 34.0))));
            }
            ui.add_space(2.0);
            ui.label(RichText::new("Aperture").font(font(20.0, Weight::Semibold)).color(TEXT));
            ui.label(RichText::new("for Trofeo Vision 9.16″").font(font(13.0, Weight::Regular)).color(DIM));
            ui.with_layout(ELayout::right_to_left(Align::Center), |ui| {
                let cur = if self.lang == Lang::En { "en" } else { "vi" };
                if let Some(v) = segmented(ui, cur, &[("en", "English"), ("vi", "Tiếng Việt")]) {
                    self.lang = if v == "vi" { Lang::Vi } else { Lang::En };
                    self.set("studio_language", &v);
                }
                ui.add_space(12.0);
                let (color, text) = self.connection();
                status_dot(ui, color, &text);
            });
        });
    }

    fn connection(&self) -> (Color32, String) {
        let s = &self.status;
        if self.status_at.is_none() {
            return (DIM, self.t("Checking…", "Đang kiểm tra…").into());
        }
        if !s.alive {
            return (BAD, self.t("Display program not running", "Chương trình màn hình chưa chạy").into());
        }
        match s.lcd.as_deref() {
            Some("connected") => (GOOD, self.t("Display connected", "Đã kết nối màn hình").into()),
            Some("not_found") => (BAD, self.t("Display not found", "Không tìm thấy màn hình").into()),
            _ => (WARN, self.t("Starting…", "Đang khởi động…").into()),
        }
    }

    fn preview_card(&mut self, ui: &mut egui::Ui) {
        let w = ui.available_width();
        let h = (w * 462.0 / 1920.0).round();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
        ui.painter().rect_filled(rect, Rounding::same(R), Color32::BLACK);
        if let Some(tex) = &self.preview {
            let img = egui::Image::new((tex.id(), rect.size())).rounding(Rounding::same(R));
            ui.put(rect, img);
        } else {
            let msg = if self.jobs.is_none() {
                self.t("aperture-display.exe was not found next to Aperture", "Không tìm thấy aperture-display.exe cạnh Aperture")
            } else {
                self.t("Drawing the preview…", "Đang vẽ bản xem trước…")
            };
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, msg, font(14.0, Weight::Regular), MUTED);
        }
        ui.painter().rect_stroke(rect, Rounding::same(R), Stroke::new(1.0_f32, LINE));
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let saved = self.rev == self.saved_rev;
            let msg = if let Some(e) = &self.save_error {
                format!("{} {e}", self.t("Could not save:", "Không lưu được:"))
            } else if saved {
                self.t("Layout preview · sample data · settings saved automatically", "Xem trước bố cục · dữ liệu mẫu · tự lưu cài đặt").to_string()
            } else {
                self.t("Saving…", "Đang lưu…").to_string()
            };
            ui.label(RichText::new(msg).font(font(12.5, Weight::Regular)).color(if self.save_error.is_some() { BAD } else { DIM }));
            ui.with_layout(ELayout::right_to_left(Align::Center), |ui| {
                let cur = if self.preview_music { "m" } else { "i" };
                let lm = self.t("Music playing", "Đang phát nhạc").to_string();
                let li = self.t("No music", "Không phát nhạc").to_string();
                if let Some(v) = segmented(ui, cur, &[("i", &li), ("m", &lm)]) {
                    self.preview_music = v == "m";
                }
                if self.original != self.doc.to_text() {
                    if ui.button(self.t("Undo all changes", "Hoàn tác tất cả")).clicked() {
                        self.doc = ConfDoc::parse(&self.original);
                        self.city = self.doc.get("weather_city").unwrap_or_default();
                        self.touch();
                    }
                }
            });
        });
    }

    fn tabs(&mut self, ui: &mut egui::Ui) {
        let pages = [
            (Page::Layouts, self.t("Layouts", "Bố cục")),
            (Page::Colors, self.t("Colors", "Màu sắc")),
            (Page::Display, self.t("Display", "Hiển thị")),
            (Page::Features, self.t("Features", "Tính năng")),
            (Page::Help, self.t("Sensors & help", "Cảm biến & trợ giúp")),
        ];
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (p, name) in pages {
                let on = self.page == p;
                let g = ui.painter().layout_no_wrap(name.to_string(), font(14.0, Weight::Medium), TEXT);
                let size = Vec2::new(g.size().x + 28.0, 36.0);
                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                if on {
                    ui.painter().rect_filled(rect, Rounding::same(9.0), CARD_HI);
                    ui.painter().rect_stroke(rect, Rounding::same(9.0), Stroke::new(1.0_f32, LINE));
                } else if resp.hovered() {
                    ui.painter().rect_filled(rect, Rounding::same(9.0), Color32::from_rgb(25, 28, 30));
                }
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, name, font(14.0, Weight::Medium), if on { TEXT } else { MUTED });
                if resp.clicked() {
                    self.page = p;
                }
            }
        });
    }

    fn page_layouts(&mut self, ui: &mut egui::Ui) {
        let current = self.get_or("pro_layout", "axis");
        section_title(ui, self.t("The collection", "Bộ sưu tập"), self.t("Five new compositions, plus Focus and Grid. Designed for the 1920 × 462 display.", "Năm bố cục mới cùng Focus và Grid. Thiết kế cho màn 1920 × 462."));
        let land: Vec<&LayoutInfo> = LAYOUTS.iter().filter(|l| !l.portrait && matches!(l.key,"axis"|"ledger"|"duet"|"orbit"|"atelier"|"focus"|"grid")).collect();
        if let Some(k) = self.grid(ui, &land, &current, 2) {
            self.set("pro_layout", &k);
        }
        ui.add_space(8.0);
        card(ui, |ui| {
            let mut on = current == "all";
            let lbl = self.t("Rotate the Aperture collection", "Xoay vòng bộ sưu tập Aperture").to_string();
            let help = self.t("Changes layout every 30 minutes", "Đổi bố cục mỗi 30 phút").to_string();
            row(ui, &lbl, &help, |ui| {
                if toggle(ui, &mut on).changed() {
                    self.set("pro_layout", if on { "all" } else { "axis" });
                }
            });
        });
        ui.add_space(12.0);
        ui.collapsing(self.t("Legacy landscape layouts", "Các bố cục ngang trước đây"), |ui| {
            let legacy: Vec<&LayoutInfo> = LAYOUTS.iter().filter(|l| !l.portrait && !matches!(l.key,"axis"|"ledger"|"duet"|"orbit"|"atelier"|"focus"|"grid")).collect();
            if let Some(k) = self.grid(ui,&legacy,&current,3) {self.set("pro_layout",&k);}
        });
        section_title(ui, self.t("Portrait", "Màn hình dựng đứng"), self.t("For a display mounted vertically. Choose the turn direction on the Display page.", "Dành cho màn hình gắn dọc. Chọn chiều xoay ở trang Hiển thị."));
        let port: Vec<&LayoutInfo> = LAYOUTS.iter().filter(|l| l.portrait).collect();
        if let Some(k) = self.grid(ui, &port, &current, 5) {
            self.set("pro_layout", &k);
        }
    }

    /// Thumbnail cards; returns the clicked layout.
    fn grid(&self, ui: &mut egui::Ui, items: &[&LayoutInfo], current: &str, cols: usize) -> Option<String> {
        let mut picked = None;
        let w = ui.available_width();
        let cw = ((w - GAP * (cols as f32 - 1.0)) / cols as f32).floor();
        for chunk in items.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                for l in chunk {
                    let img_h = if l.portrait { (cw * 1920.0 / 462.0).min(cw * 2.2) } else { (cw * 462.0 / 1920.0).round() };
                    let size = Vec2::new(cw, img_h + 62.0);
                    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                    let on = l.key == current;
                    let p = ui.painter();
                    p.rect(rect, Rounding::same(R), if resp.hovered() { CARD_HI } else { CARD }, Stroke::new(if on { 2.0_f32 } else { 1.0 }, if on { ACCENT } else { LINE }));
                    let ir = Rect::from_min_size(rect.min + Vec2::new(8.0, 8.0), Vec2::new(cw - 16.0, img_h - 8.0));
                    p.rect_filled(ir, Rounding::same(8.0), Color32::BLACK);
                    if let Some(tex) = self.thumbs.get(l.key) {
                        let ts = tex.size_vec2();
                        // fit inside, keep the aspect
                        let s = (ir.width() / ts.x).min(ir.height() / ts.y);
                        let r2 = Rect::from_center_size(ir.center(), ts * s);
                        egui::Image::new((tex.id(), r2.size())).rounding(Rounding::same(6.0)).paint_at(ui, r2);
                    } else {
                        p.text(ir.center(), egui::Align2::CENTER_CENTER, "…", font(16.0, Weight::Regular), DIM);
                    }
                    let ty = rect.bottom() - 46.0;
                    p.text(Pos2::new(rect.left() + 14.0, ty), egui::Align2::LEFT_TOP, l.name, font(14.5, Weight::Semibold), TEXT);
                    let d = if self.lang == Lang::En { l.en } else { l.vi };
                    let galley = p.layout(d.to_string(), font(12.0, Weight::Regular), MUTED, cw - 48.0);
                    p.galley(Pos2::new(rect.left()+14.0,ty+20.0),galley,MUTED);
                    if on {
                        let c = Pos2::new(rect.right() - 22.0, ty + 16.0);
                        p.circle_filled(c, 10.0, ACCENT);
                        p.line_segment([c + Vec2::new(-4.5, 0.0), c + Vec2::new(-1.0, 3.5)], Stroke::new(2.0_f32, Color32::WHITE));
                        p.line_segment([c + Vec2::new(-1.0, 3.5), c + Vec2::new(5.0, -3.5)], Stroke::new(2.0_f32, Color32::WHITE));
                    }
                    if resp.clicked() {
                        picked = Some(l.key.to_string());
                    }
                }
            });
            ui.add_space(GAP - 10.0);
        }
        picked
    }

    fn page_colors(&mut self, ui: &mut egui::Ui) {
        let theme = self.get_or("pro_theme", "mineral").to_ascii_lowercase();
        section_title(ui, self.t("Color theme", "Bộ màu"), self.t("Quiet surfaces. One accent. Color is reserved for what matters.", "Nền dịu, một màu nhấn, màu cảnh báo có ý nghĩa."));
        let w = ui.available_width();
        let cols = 4usize;
        let cw = ((w - GAP * (cols as f32 - 1.0)) / cols as f32).floor();
        let mut all: Vec<(String, String, [(u8, u8, u8); 3])> = THEMES.iter().take(5).map(|t| (t.key.to_string(), t.name.to_string(), t.colors)).collect();
        let c = self.custom;
        all.push(("custom".into(), self.t("Custom color", "Màu tùy chọn").into(), [(c[0], c[1], c[2]), lighten(c, 0.35), lighten(c, 0.6)]));
        all.push(("all".into(), self.t("Rotate all", "Xoay vòng tất cả").into(), [(124, 92, 255), (251, 146, 60), (56, 189, 248)]));
        let is_custom = theme.starts_with('#');
        for chunk in all.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                for (key, name, cols3) in chunk {
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(cw, 92.0), Sense::click());
                    let on = (*key == theme) || (key == "custom" && is_custom);
                    let p = ui.painter();
                    p.rect(rect, Rounding::same(R), if resp.hovered() { CARD_HI } else { CARD }, Stroke::new(if on { 2.0_f32 } else { 1.0 }, if on { ACCENT } else { LINE }));
                    for (i, col) in cols3.iter().enumerate() {
                        let cx = rect.left() + 28.0 + i as f32 * 22.0;
                        p.circle(Pos2::new(cx, rect.top() + 32.0), 12.0, Color32::from_rgb(col.0, col.1, col.2), Stroke::new(2.0_f32, CARD));
                    }
                    p.text(Pos2::new(rect.left() + 16.0, rect.bottom() - 30.0), egui::Align2::LEFT_TOP, name, font(14.0, Weight::Semibold), TEXT);
                    if resp.clicked() {
                        if key == "custom" {
                            let hex = format!("#{:02x}{:02x}{:02x}", self.custom[0], self.custom[1], self.custom[2]);
                            self.set("pro_theme", &hex);
                        } else {
                            self.set("pro_theme", key);
                        }
                    }
                }
            });
            ui.add_space(GAP - 10.0);
        }
        ui.collapsing(self.t("Legacy palettes", "Bộ màu trước đây"), |ui| {
            ui.horizontal_wrapped(|ui| {
                for t in THEMES.iter().skip(5) {
                    if ui.selectable_label(t.key == theme, t.name).clicked() {
                        self.set("pro_theme", t.key);
                    }
                }
            });
        });
        if is_custom {
            card(ui, |ui| {
                let lbl = self.t("Pick your color", "Chọn màu của bạn").to_string();
                let help = self.t("Trofeo builds a full matching palette from it.", "Trofeo tự tạo cả bộ màu hợp với màu này.").to_string();
                let mut c = self.custom;
                row(ui, &lbl, &help, |ui| {
                    ui.label(RichText::new(format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])).font(font(13.0, Weight::Medium)).color(MUTED));
                    egui::color_picker::color_edit_button_srgb(ui, &mut c);
                });
                if c != self.custom {
                    self.custom = c;
                    let hex = format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
                    self.set("pro_theme", &hex);
                }
            });
            ui.add_space(8.0);
        }
        card(ui, |ui| {
            let style = self.get_or("pro_style", "cards");
            let (cards, glass, flat) = (self.t("Cards", "Ô bo góc"), self.t("Glass", "Kính mờ"), self.t("Flat", "Phẳng"));
            let lbl = self.t("Panel style", "Kiểu ô").to_string();
            let help = self.t("Used by Classic, Cockpit and Focus", "Áp dụng cho Classic, Cockpit và Focus").to_string();
            let mut pick = None;
            row(ui, &lbl, &help, |ui| {
                pick = segmented(ui, &style, &[("flat", flat), ("glass", glass), ("cards", cards)]);
            });
            if let Some(v) = pick {
                self.set("pro_style", &v);
            }
            divider(ui);
            let mut on = self.bool_of("pro_cover_color", true);
            let lbl = self.t("Colors follow the album cover", "Màu theo ảnh bìa bài hát").to_string();
            let help = self.t("While music plays, accents take the colors of the cover art", "Khi phát nhạc, màu nhấn lấy theo màu ảnh bìa").to_string();
            row(ui, &lbl, &help, |ui| {
                if toggle(ui, &mut on).changed() {
                    self.set("pro_cover_color", if on { "true" } else { "false" });
                }
            });
        });
    }

    fn page_display(&mut self, ui: &mut egui::Ui) {
        section_title(ui, self.t("Screen", "Màn hình"), "");
        card(ui, |ui| {
            let mut flip = self.bool_of("flip", false);
            let lbl = self.t("Picture upside down? Turn it 180°", "Hình bị ngược? Xoay 180°").to_string();
            let help = self.t("Use this when the display is mounted the other way round", "Dùng khi màn hình được gắn ngược chiều").to_string();
            row(ui, &lbl, &help, |ui| {
                if toggle(ui, &mut flip).changed() {
                    self.set("flip", if flip { "true" } else { "false" });
                }
            });
            divider(ui);
            let mut b: u32 = self.get("brightness").and_then(|v| v.parse().ok()).unwrap_or(100).min(100);
            let lbl = self.t("Brightness", "Độ sáng").to_string();
            row(ui, &lbl, "", |ui| {
                if ui.add(egui::Slider::new(&mut b, 5..=100).suffix(" %")).changed() {
                    self.set("brightness", &b.to_string());
                }
            });
            divider(ui);
            let rot = self.get_or("pro_rotate", "cw");
            let lbl = self.t("Portrait turn direction", "Chiều xoay khi dựng đứng").to_string();
            let help = self.t("Only for portrait layouts. Upside down? Pick the other one.", "Chỉ dùng cho bố cục dọc. Bị ngược thì chọn bên còn lại.").to_string();
            let (a, c) = (self.t("Clockwise", "Xoay phải"), self.t("Counter-clockwise", "Xoay trái"));
            let mut pick = None;
            row(ui, &lbl, &help, |ui| {
                pick = segmented(ui, &rot, &[("ccw", c), ("cw", a)]);
            });
            if let Some(v) = pick {
                self.set("pro_rotate", &v);
            }
        });
        ui.add_space(12.0);
        section_title(ui, self.t("Clock & weather", "Đồng hồ & thời tiết"), "");
        card(ui, |ui| {
            let clock = if self.get_or("pro_clock", "24h").starts_with("12") { "12h" } else { "24h" };
            let lbl = self.t("Clock format", "Kiểu đồng hồ").to_string();
            let mut pick = None;
            row(ui, &lbl, "", |ui| {
                pick = segmented(ui, clock, &[("12h", "12h  (2:30 PM)"), ("24h", "24h  (14:30)")]);
            });
            if let Some(v) = pick {
                self.set("pro_clock", &v);
            }
            divider(ui);
            let unit = self.get_or("weather_unit", "c").to_ascii_lowercase();
            let lbl = self.t("Temperature unit (weather)", "Đơn vị nhiệt độ (thời tiết)").to_string();
            let mut pick = None;
            row(ui, &lbl, "", |ui| {
                pick = segmented(ui, &unit, &[("f", "°F"), ("c", "°C")]);
            });
            if let Some(v) = pick {
                self.set("weather_unit", &v);
            }
            divider(ui);
            let lbl = self.t("City for the weather", "Thành phố cho thời tiết").to_string();
            let help = self.t("“City, Country” or coordinates like “21.03, 105.85”. Empty = found from your internet connection.", "“Thành phố, Quốc gia” hoặc tọa độ như “21.03, 105.85”. Để trống = tự đoán theo mạng.").to_string();
            let apply = self.t("Apply", "Áp dụng").to_string();
            let mut do_apply = false;
            row(ui, &lbl, &help, |ui| {
                if ui.button(&apply).clicked() {
                    do_apply = true;
                }
                let r = ui.add(egui::TextEdit::singleline(&mut self.city).desired_width(240.0).hint_text("Hanoi, Vietnam"));
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    do_apply = true;
                }
            });
            if do_apply {
                let c = self.city.trim().to_string();
                if c.is_empty() {
                    self.unset("weather_city");
                } else {
                    self.set("weather_city", &c);
                }
                self.restart_needed = true;
            }
        });
        if self.restart_needed {
            ui.add_space(10.0);
            self.restart_banner(ui);
        }
    }

    fn restart_banner(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none().fill(Color32::from_rgb(40, 33, 14)).stroke(Stroke::new(1.0_f32, Color32::from_rgb(96, 74, 20))).rounding(R).inner_margin(egui::Margin::same(14.0)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(self.t("The weather city is used after the display program restarts.", "Thành phố mới được dùng sau khi khởi động lại chương trình màn hình.")).color(TEXT));
                ui.with_layout(ELayout::right_to_left(Align::Center), |ui| {
                    if ui.button(self.t("Restart display", "Khởi động lại màn hình")).clicked() {
                        run_elevated(RESTART_SCRIPT);
                        self.restart_needed = false;
                    }
                });
            });
        });
    }

    fn layout_combo(&self, ui: &mut egui::Ui, id: &str, current: &str) -> Option<String> {
        let mut picked = None;
        let name = LAYOUTS.iter().find(|l| l.key == current).map(|l| l.name).unwrap_or(current);
        egui::ComboBox::from_id_salt(id).selected_text(name).width(180.0).show_ui(ui, |ui| {
            for l in LAYOUTS.iter().filter(|l| !l.portrait) {
                if ui.selectable_label(l.key == current, l.name).clicked() {
                    picked = Some(l.key.to_string());
                }
            }
        });
        picked
    }

    fn theme_combo(&self, ui: &mut egui::Ui, id: &str, current: &str) -> Option<String> {
        let mut picked = None;
        let same = self.t("Same as normal", "Giống bình thường").to_string();
        let name = THEMES.iter().find(|t| t.key == current).map(|t| t.name.to_string()).unwrap_or(if current.is_empty() { same.clone() } else { current.to_string() });
        egui::ComboBox::from_id_salt(id).selected_text(name).width(180.0).show_ui(ui, |ui| {
            if ui.selectable_label(current.is_empty(), &same).clicked() {
                picked = Some(String::new());
            }
            for t in THEMES {
                if ui.selectable_label(t.key == current, t.name).clicked() {
                    picked = Some(t.key.to_string());
                }
            }
        });
        picked
    }

    fn page_features(&mut self, ui: &mut egui::Ui) {
        section_title(ui, self.t("Music", "Nhạc"), "");
        card(ui, |ui| {
            let mut on = self.bool_of("pro_song_popup", true);
            let lbl = self.t("New song pop-up", "Thông báo khi đổi bài").to_string();
            let help = self.t("Shows the cover, title and artist for 3 seconds", "Hiện ảnh bìa, tên bài và ca sĩ trong 3 giây").to_string();
            row(ui, &lbl, &help, |ui| {
                if toggle(ui, &mut on).changed() {
                    self.set("pro_song_popup", if on { "true" } else { "false" });
                }
            });
        });
        ui.add_space(12.0);
        section_title(ui, self.t("Game mode", "Chế độ chơi game"), self.t("Switches layout while a game shows FPS, and back when you quit.", "Tự đổi bố cục khi game có FPS, thoát game thì tự trở lại."));
        card(ui, |ui| {
            let cur = self.get("pro_game_layout").unwrap_or_default();
            let mut on = !cur.is_empty();
            let lbl = self.t("Turn on game mode", "Bật chế độ chơi game").to_string();
            row(ui, &lbl, "", |ui| {
                if toggle(ui, &mut on).changed() {
                    if on {
                        self.set("pro_game_layout", "focus");
                    } else {
                        self.unset("pro_game_layout");
                        self.unset("pro_game_theme");
                    }
                }
            });
            if on {
                divider(ui);
                let lbl = self.t("Layout while gaming", "Bố cục khi chơi game").to_string();
                let mut pick = None;
                row(ui, &lbl, "", |ui| pick = self.layout_combo(ui, "game_layout", &cur));
                if let Some(v) = pick {
                    self.set("pro_game_layout", &v);
                }
                divider(ui);
                let gt = self.get("pro_game_theme").unwrap_or_default();
                let lbl = self.t("Colors while gaming", "Màu khi chơi game").to_string();
                let mut pick = None;
                row(ui, &lbl, "", |ui| pick = self.theme_combo(ui, "game_theme", &gt));
                match pick.as_deref() {
                    Some("") => self.unset("pro_game_theme"),
                    Some(v) => self.set("pro_game_theme", v),
                    None => {}
                }
            }
        });
        ui.add_space(12.0);
        section_title(ui, self.t("Night mode", "Chế độ ban đêm"), self.t("Dims the display (and can change colors) during the hours you choose.", "Giảm độ sáng (và có thể đổi màu) trong khung giờ bạn chọn."));
        card(ui, |ui| {
            let night = self.get("pro_night").unwrap_or_default();
            let mut on = !night.is_empty();
            let lbl = self.t("Turn on night mode", "Bật chế độ ban đêm").to_string();
            row(ui, &lbl, "", |ui| {
                if toggle(ui, &mut on).changed() {
                    if on {
                        self.set("pro_night", "22:00-07:00");
                    } else {
                        self.unset("pro_night");
                        self.unset("pro_night_theme");
                    }
                }
            });
            if on {
                let (mut a, mut b) = parse_range(&night).unwrap_or((22 * 60, 7 * 60));
                divider(ui);
                let lbl = self.t("From – to", "Từ – đến").to_string();
                let mut changed = false;
                row(ui, &lbl, "", |ui| {
                    changed |= time_edit(ui, "night_b", &mut b);
                    ui.label(RichText::new("–").color(MUTED));
                    changed |= time_edit(ui, "night_a", &mut a);
                });
                if changed {
                    self.set("pro_night", &format!("{:02}:{:02}-{:02}:{:02}", a / 60, a % 60, b / 60, b % 60));
                }
                divider(ui);
                let mut dim: u32 = self.get("pro_night_dim").and_then(|v| v.parse().ok()).unwrap_or(40).clamp(5, 100);
                let lbl = self.t("Brightness at night", "Độ sáng ban đêm").to_string();
                row(ui, &lbl, "", |ui| {
                    if ui.add(egui::Slider::new(&mut dim, 5..=100).suffix(" %")).changed() {
                        self.set("pro_night_dim", &dim.to_string());
                    }
                });
                divider(ui);
                let nt = self.get("pro_night_theme").unwrap_or_default();
                let lbl = self.t("Colors at night", "Màu ban đêm").to_string();
                let mut pick = None;
                row(ui, &lbl, "", |ui| pick = self.theme_combo(ui, "night_theme", &nt));
                match pick.as_deref() {
                    Some("") => self.unset("pro_night_theme"),
                    Some(v) => self.set("pro_night_theme", v),
                    None => {}
                }
            }
        });
        ui.add_space(12.0);
        section_title(ui, self.t("Heat alert", "Cảnh báo nóng"), self.t("A red frame pulses on the display when a temperature goes too high.", "Viền đỏ nhấp nháy trên màn hình khi nhiệt độ quá cao."));
        card(ui, |ui| {
            let cpu: u32 = self.get("pro_alert_cpu").and_then(|v| v.parse().ok()).unwrap_or(0);
            let gpu: u32 = self.get("pro_alert_gpu").and_then(|v| v.parse().ok()).unwrap_or(0);
            let mut on = cpu > 0 || gpu > 0;
            let lbl = self.t("Turn on heat alert", "Bật cảnh báo nóng").to_string();
            row(ui, &lbl, "", |ui| {
                if toggle(ui, &mut on).changed() {
                    self.set("pro_alert_cpu", if on { "90" } else { "0" });
                    self.set("pro_alert_gpu", if on { "85" } else { "0" });
                }
            });
            if on {
                for (key, en, vi, val) in [("pro_alert_cpu", "CPU above", "CPU vượt quá", cpu), ("pro_alert_gpu", "GPU above", "GPU vượt quá", gpu)] {
                    divider(ui);
                    let mut v = if val == 0 { 85 } else { val };
                    let lbl = self.t(en, vi).to_string();
                    row(ui, &lbl, "", |ui| {
                        if ui.add(egui::Slider::new(&mut v, 60..=105).suffix(" °C")).changed() {
                            self.set(key, &v.to_string());
                        }
                    });
                }
            }
        });
    }

    fn page_help(&mut self, ui: &mut egui::Ui) {
        let s = self.status.clone();
        section_title(ui, self.t("Display", "Màn hình"), "");
        card(ui, |ui| {
            let (color, text) = self.connection();
            ui.horizontal(|ui| {
                status_dot(ui, color, &text);
                ui.with_layout(ELayout::right_to_left(Align::Center), |ui| {
                    if ui.button(self.t("Restart display", "Khởi động lại màn hình")).clicked() {
                        run_elevated(RESTART_SCRIPT);
                    }
                });
            });
            ui.add_space(4.0);
            ui.label(RichText::new(self.t("Nothing on the screen?", "Màn hình không hiện gì?")).font(font(13.5, Weight::Semibold)).color(TEXT));
            bullet(ui, self.t("Close Thermalright TRCC and remove it from Windows startup: two programs cannot drive the display.", "Tắt phần mềm TRCC của Thermalright và gỡ khỏi khởi động cùng Windows: hai chương trình không thể dùng màn hình cùng lúc."));
            bullet(ui, self.t("Check the USB cable (a motherboard USB header or a rear port).", "Kiểm tra dây USB (cắm vào header USB trên mainboard hoặc cổng sau)."));
            bullet(ui, self.t("Only the Trofeo Vision 9.16″ (1920 × 462) is supported; the 6.86″ model is not.", "Chỉ hỗ trợ Trofeo Vision 9.16″ (1920 × 462); bản 6.86″ không được hỗ trợ."));
            bullet(ui, self.t("Picture upside down? Display → “Turn it 180°”.", "Hình bị ngược? Vào Hiển thị → “Xoay 180°”."));
        });
        ui.add_space(12.0);
        section_title(ui, self.t("Sensors", "Cảm biến"), self.t("Where each reading comes from, and how to fix the missing ones.", "Mỗi thông số lấy từ đâu, và cách sửa khi bị thiếu."));
        let ext_name = |src: Option<Source>| match src {
            Some(Source::Hwinfo) => "HWiNFO",
            Some(Source::Lhm) => "LibreHardwareMonitor",
            None => "",
        };
        let ext_src = s.ext.as_ref().map(|(src, _)| *src);
        let from = |lang: Lang, v: Option<&str>| -> (Color32, String) {
            let t = |en: &str, vi: &str| if lang == Lang::En { en.to_string() } else { vi.to_string() };
            match v {
                Some("builtin") => (GOOD, t("Working (built-in)", "Đang hoạt động (có sẵn)")),
                Some("hwinfo") => (GOOD, t("Working, from HWiNFO", "Đang hoạt động, lấy từ HWiNFO")),
                Some("lhm") => (GOOD, t("Working, from LibreHardwareMonitor", "Đang hoạt động, lấy từ LibreHardwareMonitor")),
                Some(_) => (BAD, t("Not available", "Không có dữ liệu")),
                None => (DIM, t("Unknown (display program not running)", "Chưa rõ (chương trình màn hình chưa chạy)")),
            }
        };
        let st_temp = from(self.lang, s.cpu_temp.as_deref());
        let st_fan = from(self.lang, s.cpu_fan.as_deref());
        let st_hot = from(self.lang, s.gpu_hotspot.as_deref());
        card(ui, |ui| {
            // CPU temperature
            let (c, t) = st_temp;
            let lbl = self.t("CPU temperature", "Nhiệt độ CPU").to_string();
            row(ui, &lbl, "", |ui| status_dot(ui, c, &t));
            if s.cpu_temp.as_deref() == Some("none") || s.pawnio == Some(false) {
                if s.pawnio == Some(false) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(self.t("The PawnIO driver (reads AMD Ryzen and Intel Core temperatures) is not installed.", "Chưa cài driver PawnIO (dùng để đọc nhiệt độ AMD Ryzen và Intel Core).")).font(font(13.0, Weight::Regular)).color(MUTED));
                        ui.with_layout(ELayout::right_to_left(Align::Center), |ui| {
                            if ui.button(self.t("Install PawnIO", "Cài PawnIO")).clicked() {
                                run_elevated(PAWNIO_SCRIPT);
                            }
                        });
                    });
                }
            }
            divider(ui);
            // CPU fan
            let (c, t) = st_fan;
            let lbl = self.t("CPU fan speed", "Tốc độ quạt CPU").to_string();
            row(ui, &lbl, "", |ui| status_dot(ui, c, &t));
            if let Some((_, snap)) = &s.ext {
                if !snap.fans.is_empty() {
                    let cur = self.get("pro_fan_sensor").unwrap_or_default();
                    let auto = self.t("Automatic", "Tự động").to_string();
                    let lbl = self.t("Which fan is the CPU fan?", "Quạt nào là quạt CPU?").to_string();
                    let help = format!("{} {}", self.t("Fans reported by", "Các quạt mà"), ext_name(ext_src)) + self.t("", " báo về");
                    let mut pick: Option<String> = None;
                    row(ui, &lbl, &help, |ui| {
                        let shown = if cur.is_empty() { auto.clone() } else { cur.clone() };
                        egui::ComboBox::from_id_salt("fan_pick").selected_text(shown).width(260.0).show_ui(ui, |ui| {
                            if ui.selectable_label(cur.is_empty(), &auto).clicked() {
                                pick = Some(String::new());
                            }
                            for f in &snap.fans {
                                let txt = format!("{}  ·  {:.0} RPM", f.label, f.value);
                                if ui.selectable_label(f.label == cur, txt).clicked() {
                                    pick = Some(f.label.clone());
                                }
                            }
                        });
                    });
                    match pick.as_deref() {
                        Some("") => self.unset("pro_fan_sensor"),
                        Some(v) => self.set("pro_fan_sensor", v),
                        None => {}
                    }
                }
            } else {
                ui.label(RichText::new(self.t("Fan speeds come from a free monitoring program. Easiest: LibreHardwareMonitor (free, no time limit).", "Tốc độ quạt lấy từ một phần mềm theo dõi miễn phí. Dễ nhất: LibreHardwareMonitor (miễn phí, không giới hạn thời gian).")).font(font(13.0, Weight::Regular)).color(MUTED));
                bullet(ui, self.t("1. Download it and start it (it can start with Windows: Options → Run On Windows Startup).", "1. Tải về và mở nó (có thể cho chạy cùng Windows: Options → Run On Windows Startup)."));
                bullet(ui, self.t("2. Options → Remote Web Server → Run. Trofeo finds it by itself within seconds.", "2. Options → Remote Web Server → Run. Trofeo tự tìm thấy trong vài giây."));
                bullet(ui, self.t("HWiNFO also works (Settings → Shared Memory Support); the free version stops sharing after 12 hours.", "HWiNFO cũng dùng được (Settings → Shared Memory Support); bản miễn phí ngừng chia sẻ sau 12 giờ."));
                if ui.link(self.t("Download LibreHardwareMonitor", "Tải LibreHardwareMonitor")).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab("https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/releases/latest"));
                }
            }
            divider(ui);
            // GPU hotspot
            let (c, t) = st_hot;
            let lbl = self.t("GPU hotspot", "Hotspot GPU").to_string();
            let help = self.t("NVIDIA RTX 50 cards have no hotspot sensor at all; every program shows nothing there.", "Card NVIDIA RTX 50 không có cảm biến hotspot; phần mềm nào cũng không đọc được.").to_string();
            row(ui, &lbl, &help, |ui| status_dot(ui, c, &t));
        });
        ui.add_space(12.0);
        section_title(ui, self.t("Files", "Tệp"), "");
        card(ui, |ui| {
            let lbl = self.t("Settings file", "Tệp cài đặt").to_string();
            let p = self.path.display().to_string();
            row(ui, &lbl, &p, |ui| {
                if ui.button(self.t("Open folder", "Mở thư mục")).clicked() {
                    #[cfg(windows)]
                    {
                        let _ = std::process::Command::new("explorer").arg(&self.dir).spawn();
                    }
                }
            });
        });
    }
}

fn lighten(c: [u8; 3], t: f32) -> (u8, u8, u8) {
    let l = |x: u8| (x as f32 + (255.0 - x as f32) * t).round() as u8;
    (l(c[0]), l(c[1]), l(c[2]))
}

fn parse_range(s: &str) -> Option<(u32, u32)> {
    let (a, b) = s.split_once('-')?;
    let one = |x: &str| -> Option<u32> {
        let (h, m) = x.trim().split_once(':')?;
        Some(h.parse::<u32>().ok()?.min(23) * 60 + m.parse::<u32>().ok()?.min(59))
    };
    Some((one(a)?, one(b)?))
}

/// hh:mm editor (drag or type), in minutes after midnight.
fn time_edit(ui: &mut egui::Ui, id: &str, minutes: &mut u32) -> bool {
    let mut h = *minutes / 60;
    let mut m = *minutes % 60;
    let mut changed = false;
    ui.push_id(id, |ui| {
        changed |= ui.add(egui::DragValue::new(&mut m).range(0..=59).custom_formatter(|v, _| format!("{:02}", v as u32))).changed();
        ui.label(":");
        changed |= ui.add(egui::DragValue::new(&mut h).range(0..=23).custom_formatter(|v, _| format!("{:02}", v as u32))).changed();
    });
    if changed {
        *minutes = h * 60 + m;
    }
    changed
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump(ctx);
        egui::CentralPanel::default().frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin::symmetric(28.0, 20.0))).show(ctx, |ui| {
            self.header(ui);
            ui.add_space(12.0);
            self.preview_card(ui);
            ui.add_space(12.0);
            self.tabs(ui);
            ui.add_space(8.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(4.0);
                match self.page {
                    Page::Layouts => self.page_layouts(ui),
                    Page::Colors => self.page_colors(ui),
                    Page::Display => self.page_display(ui),
                    Page::Features => self.page_features(ui),
                    Page::Help => self.page_help(ui),
                }
                ui.add_space(24.0);
            });
        });
        self.autosave();
    }
}

fn main() -> eframe::Result {
    let icon = image::load_from_memory(include_bytes!("../../assets/studio-icon.png")).ok().map(|i| {
        let i = i.to_rgba8();
        egui::IconData { width: i.width(), height: i.height(), rgba: i.into_raw() }
    });
    let mut viewport = egui::ViewportBuilder::default().with_inner_size([1180.0, 860.0]).with_min_inner_size([980.0, 640.0]).with_title("Aperture");
    if let Some(i) = icon {
        viewport = viewport.with_icon(i);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("Aperture", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
