//! Readings from external monitoring programs, used when the built-in sensors have
//! nothing (Intel CPUs without PawnIO, CPU fan speed, GPU hotspot on some cards):
//!
//! - **HWiNFO** (`Global\HWiNFO_SENS_SM2` shared memory, "Shared Memory Support" on).
//! - **LibreHardwareMonitor** (free, open source): its "Remote Web Server" (Options menu)
//!   serves every sensor as JSON on `http://localhost:8085/data.json`.
//!
//! Both are optional: when neither runs, every value is `None`. The settings app (Trofeo
//! Studio) uses the same functions to show which sources are available.

/// One sensor reading: where it comes from (device name), its label and value.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub device: String,
    pub label: String,
    pub value: f64,
}

/// Every temperature and fan reading of one source.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub temps: Vec<Reading>,
    pub fans: Vec<Reading>,
}

impl Snapshot {
    /// CPU package temperature.
    pub fn cpu_temp(&self) -> Option<f32> {
        let l = |r: &&Reading| r.label.to_ascii_lowercase();
        let pick = |names: &[&str]| self.temps.iter().find(|r| names.iter().any(|n| l(r) == *n)).map(|r| r.value as f32);
        pick(&["cpu package", "core (tctl/tdie)", "cpu (tctl/tdie)", "cpu (tctl)", "cpu (tdie)", "cpu die (average)", "core (tctl)", "cpu"])
            .filter(|t| *t > 0.0 && *t < 130.0)
    }
    /// GPU hotspot (junction) temperature.
    pub fn gpu_hotspot(&self) -> Option<i32> {
        self.temps
            .iter()
            .find(|r| {
                let l = r.label.to_ascii_lowercase();
                l.contains("hot spot") || l.contains("hotspot") || l.contains("junction")
            })
            .map(|r| r.value.round() as i32)
            .filter(|t| *t > 0 && *t < 150)
    }
    /// CPU fan: the label chosen in the settings (`wanted`), else the usual CPU fan names.
    pub fn cpu_fan(&self, wanted: Option<&str>) -> Option<u32> {
        if let Some(w) = wanted.map(|w| w.trim()).filter(|w| !w.is_empty()) {
            let w = w.to_ascii_lowercase();
            return self.fans.iter().find(|r| r.label.to_ascii_lowercase() == w).map(|r| r.value.max(0.0).round() as u32);
        }
        let fans: Vec<(String, f64)> = self.fans.iter().map(|r| (r.label.clone(), r.value)).collect();
        pick_cpu_fan(&fans)
    }
}

/// Picks the CPU fan among (label, rpm) readings: "CPU" / "CPU Fan" first, then any
/// label containing "CPU" (not "OPT"/"PUMP"), then "CPU OPT".
pub fn pick_cpu_fan(fans: &[(String, f64)]) -> Option<u32> {
    let l = |s: &str| s.to_ascii_lowercase();
    let exact = fans.iter().find(|(n, _)| matches!(l(n).trim(), "cpu" | "cpu fan" | "cpu_fan" | "cpu fan1" | "cpu fan #1"));
    let any = || fans.iter().find(|(n, _)| { let n = l(n); n.contains("cpu") && !n.contains("opt") && !n.contains("pump") });
    let opt = || fans.iter().find(|(n, _)| l(n).contains("cpu"));
    exact.or_else(any).or_else(opt).map(|(_, v)| v.max(0.0).round() as u32)
}

// ------------------------------------------------------------------------- HWiNFO

/// HWiNFO's shared memory, `None` when HWiNFO is not running with it enabled.
#[cfg(windows)]
pub fn hwinfo() -> Option<Snapshot> {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenFileMappingW(access: u32, inherit: i32, name: *const u16) -> isize;
        fn MapViewOfFile(h: isize, access: u32, hi: u32, lo: u32, bytes: usize) -> *const c_void;
        fn UnmapViewOfFile(base: *const c_void) -> i32;
        fn CloseHandle(h: isize) -> i32;
    }
    const FILE_MAP_READ: u32 = 4;
    let name: Vec<u16> = "Global\\HWiNFO_SENS_SM2".encode_utf16().chain(std::iter::once(0)).collect();
    let h = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, name.as_ptr()) };
    if h == 0 {
        return None;
    }
    let base = unsafe { MapViewOfFile(h, FILE_MAP_READ, 0, 0, 0) } as *const u8;
    if base.is_null() {
        unsafe { CloseHandle(h) };
        return None;
    }
    let rd32 = |off: usize| -> u32 { unsafe { std::ptr::read_unaligned(base.add(off) as *const u32) } };
    let text = |off: usize, len: usize| -> String {
        let bytes = unsafe { std::slice::from_raw_parts(base.add(off), len) };
        let end = bytes.iter().position(|b| *b == 0).unwrap_or(len);
        String::from_utf8_lossy(&bytes[..end]).trim().to_string()
    };
    let mut snap = Snapshot::default();
    let sig = unsafe { std::slice::from_raw_parts(base, 4) };
    let mut ok = false;
    if sig == b"HWiS" {
        ok = true;
        // header (packed): sensor section offset/size/count at 20/24/28, readings at 32/36/40
        let (s_off, s_size, s_count) = (rd32(20) as usize, rd32(24) as usize, rd32(28) as usize);
        let (off, size, count) = (rd32(32) as usize, rd32(36) as usize, rd32(40) as usize);
        let device = |i: usize| -> String {
            if i >= s_count || s_size < 264 {
                return String::new();
            }
            let e = s_off + i * s_size;
            let user = text(e + 136, 128);
            if user.is_empty() { text(e + 8, 128) } else { user }
        };
        if size >= 292 && count < 10_000 {
            for i in 0..count {
                let e = off + i * size;
                let kind = rd32(e); // 1 = temperature, 3 = fan
                if kind != 1 && kind != 3 {
                    continue;
                }
                let user = text(e + 140, 128);
                let label = if user.is_empty() { text(e + 12, 128) } else { user };
                let value = unsafe { std::ptr::read_unaligned(base.add(e + 284) as *const f64) };
                let r = Reading { device: device(rd32(e + 4) as usize), label, value };
                if kind == 1 { snap.temps.push(r) } else { snap.fans.push(r) }
            }
        }
    }
    unsafe {
        UnmapViewOfFile(base as *const c_void);
        CloseHandle(h);
    }
    ok.then_some(snap)
}

#[cfg(not(windows))]
pub fn hwinfo() -> Option<Snapshot> {
    None
}

// ------------------------------------------------------------ LibreHardwareMonitor

/// The address of LibreHardwareMonitor's web server (its default port).
pub const LHM_URL: &str = "http://127.0.0.1:8085/data.json";

/// LibreHardwareMonitor's sensors, `None` when its web server does not answer.
pub fn lhm() -> Option<Snapshot> {
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_millis(400)).build();
    let body = agent.get(LHM_URL).call().ok()?.into_string().ok()?;
    parse_lhm(&body)
}

/// Walks the JSON tree of `data.json`: every node has "Text", optionally "Type",
/// "Value" (e.g. "45.0 °C", "1180 RPM") and "Children".
pub fn parse_lhm(body: &str) -> Option<Snapshot> {
    let root = json::parse(body)?;
    let mut snap = Snapshot::default();
    fn walk(v: &json::Value, device: &str, depth: usize, snap: &mut Snapshot) {
        let text = v.get("Text").and_then(|t| t.as_str()).unwrap_or("");
        let value = v.get("Value").and_then(|t| t.as_str()).unwrap_or("");
        let kind = v.get("Type").and_then(|t| t.as_str()).unwrap_or("");
        let children = v.get("Children").and_then(|c| c.as_arr()).unwrap_or(&[]);
        if children.is_empty() && !value.is_empty() {
            let num: String = value.chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == ',').collect();
            // "52.0", "52,0" (decimal comma) or "1,180" (thousands separator)
            let num = match num.rfind(',') {
                Some(p) if !num.contains('.') && num.len() - p - 1 == 3 => num.replace(',', ""),
                _ => num.replace(',', "."),
            };
            if let Ok(n) = num.parse::<f64>() {
                let r = Reading { device: device.to_string(), label: text.to_string(), value: n };
                let is_temp = kind == "Temperature" || value.ends_with("°C");
                let is_fan = kind == "Fan" || value.ends_with("RPM");
                if is_temp {
                    snap.temps.push(r);
                } else if is_fan {
                    snap.fans.push(r);
                }
            }
            return;
        }
        // depth 1 = computer name, 2 = device (CPU / GPU / motherboard / ...)
        let dev = if depth == 2 { text } else { device };
        for c in children {
            walk(c, dev, depth + 1, snap);
        }
    }
    walk(&root, "", 0, &mut snap);
    Some(snap)
}

// ---------------------------------------------------------------- the two together

/// Which program the readings came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Hwinfo,
    Lhm,
}

/// HWiNFO first, then LibreHardwareMonitor.
pub fn read_any() -> Option<(Source, Snapshot)> {
    hwinfo().map(|s| (Source::Hwinfo, s)).or_else(|| lhm().map(|s| (Source::Lhm, s)))
}

// ---------------------------------------------------------------- minimal JSON

/// Just enough JSON to read LibreHardwareMonitor's data.json (no dependency needed).
pub mod json {
    #[derive(Debug, Clone, PartialEq)]
    pub enum Value {
        Null,
        Bool(bool),
        Num(f64),
        Str(String),
        Arr(Vec<Value>),
        Obj(Vec<(String, Value)>),
    }

    impl Value {
        pub fn get(&self, key: &str) -> Option<&Value> {
            match self {
                Value::Obj(v) => v.iter().find(|(k, _)| k == key).map(|(_, v)| v),
                _ => None,
            }
        }
        pub fn as_str(&self) -> Option<&str> {
            match self {
                Value::Str(s) => Some(s),
                _ => None,
            }
        }
        pub fn as_arr(&self) -> Option<&[Value]> {
            match self {
                Value::Arr(a) => Some(a),
                _ => None,
            }
        }
    }

    pub fn parse(s: &str) -> Option<Value> {
        let b = s.as_bytes();
        let mut i = 0;
        let v = value(b, &mut i, 0)?;
        Some(v)
    }

    fn ws(b: &[u8], i: &mut usize) {
        while *i < b.len() && b[*i].is_ascii_whitespace() {
            *i += 1;
        }
    }

    fn value(b: &[u8], i: &mut usize, depth: usize) -> Option<Value> {
        if depth > 64 {
            return None;
        }
        ws(b, i);
        match *b.get(*i)? {
            b'{' => {
                *i += 1;
                let mut out = Vec::new();
                loop {
                    ws(b, i);
                    if *b.get(*i)? == b'}' {
                        *i += 1;
                        return Some(Value::Obj(out));
                    }
                    let k = string(b, i)?;
                    ws(b, i);
                    if *b.get(*i)? != b':' {
                        return None;
                    }
                    *i += 1;
                    let v = value(b, i, depth + 1)?;
                    out.push((k, v));
                    ws(b, i);
                    match *b.get(*i)? {
                        b',' => *i += 1,
                        b'}' => {}
                        _ => return None,
                    }
                }
            }
            b'[' => {
                *i += 1;
                let mut out = Vec::new();
                loop {
                    ws(b, i);
                    if *b.get(*i)? == b']' {
                        *i += 1;
                        return Some(Value::Arr(out));
                    }
                    out.push(value(b, i, depth + 1)?);
                    ws(b, i);
                    match *b.get(*i)? {
                        b',' => *i += 1,
                        b']' => {}
                        _ => return None,
                    }
                }
            }
            b'"' => string(b, i).map(Value::Str),
            b't' if b[*i..].starts_with(b"true") => {
                *i += 4;
                Some(Value::Bool(true))
            }
            b'f' if b[*i..].starts_with(b"false") => {
                *i += 5;
                Some(Value::Bool(false))
            }
            b'n' if b[*i..].starts_with(b"null") => {
                *i += 4;
                Some(Value::Null)
            }
            _ => {
                let st = *i;
                while *i < b.len() && matches!(b[*i], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E') {
                    *i += 1;
                }
                std::str::from_utf8(&b[st..*i]).ok()?.parse().ok().map(Value::Num)
            }
        }
    }

    fn string(b: &[u8], i: &mut usize) -> Option<String> {
        if *b.get(*i)? != b'"' {
            return None;
        }
        *i += 1;
        let mut out: Vec<u8> = Vec::new();
        while *i < b.len() {
            let c = b[*i];
            *i += 1;
            match c {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let e = *b.get(*i)?;
                    *i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' | b'f' => {}
                        b'u' => {
                            let hex = std::str::from_utf8(b.get(*i..*i + 4)?).ok()?;
                            *i += 4;
                            let cp = u32::from_str_radix(hex, 16).ok()?;
                            let ch = char::from_u32(cp).unwrap_or('?');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                _ => out.push(c),
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lhm_tree() {
        let body = r#"{"id":0,"Text":"Sensor","Children":[{"id":1,"Text":"MYPC","Children":[
            {"id":2,"Text":"Intel Core i5-12600K","Children":[{"id":3,"Text":"Temperatures","Children":[
                {"id":4,"Text":"CPU Package","Children":[],"Min":"30.0 °C","Value":"52.0 °C","Max":"80.0 °C","Type":"Temperature"}]}]},
            {"id":5,"Text":"ASUS PRIME","Children":[{"id":6,"Text":"Nuvoton NCT6798D","Children":[{"id":7,"Text":"Fans","Children":[
                {"id":8,"Text":"CPU Fan","Children":[],"Value":"1,180 RPM","Type":"Fan"},
                {"id":9,"Text":"Fan #2","Children":[],"Value":"800 RPM","Type":"Fan"}]}]}]},
            {"id":10,"Text":"AMD Radeon RX 9070 XT","Children":[{"id":11,"Text":"Temperatures","Children":[
                {"id":12,"Text":"GPU Hot Spot","Children":[],"Value":"71.0 °C","Type":"Temperature"}]}]}]}]}"#;
        let s = parse_lhm(body).unwrap();
        assert_eq!(s.cpu_temp(), Some(52.0));
        assert_eq!(s.gpu_hotspot(), Some(71));
        assert_eq!(s.fans.len(), 2);
        assert_eq!(s.cpu_fan(Some("Fan #2")), Some(800));
        assert_eq!(s.temps[0].device, "Intel Core i5-12600K");
    }
}
