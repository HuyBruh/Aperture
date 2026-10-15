//! Extra system information for the `pro` dashboard (see `pro.rs`):
//! usage of the system drive and the temperature of the disk it lives on.
//!
//! Read on a small background thread every few seconds (the disk does not need
//! more), so the render loop only copies the last value.
//!
//! - Usage: `sysinfo::Disks` (any platform).
//! - Temperature (Windows): `IOCTL_STORAGE_QUERY_PROPERTY` /
//!   `StorageDeviceTemperatureProperty` on the physical drive that holds the
//!   system drive — the same value NVMe/SATA drives report to Task Manager /
//!   CrystalDiskInfo. `None` if the drive or its driver does not report it.

use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DiskInfo {
    /// "C:" (or the mount point on Linux).
    pub label: String,
    pub used_gb: f64,
    pub total_gb: f64,
    pub temp_c: Option<i32>,
}

impl DiskInfo {
    pub fn used_frac(&self) -> f32 {
        if self.total_gb > 0.0 {
            (self.used_gb / self.total_gb).clamp(0.0, 1.0) as f32
        } else {
            0.0
        }
    }
}

/// What the background reader last found.
#[derive(Clone, Debug, Default)]
pub struct Extra {
    pub disk: Option<DiskInfo>,
    /// CPU fan speed, when HWiNFO is running with "Shared Memory Support" on.
    pub cpu_fan_rpm: Option<u32>,
}

pub type SharedExtra = Arc<Mutex<Extra>>;

/// Starts the background reader: the CPU fan every 2 s, the disk every 10 s.
pub fn spawn_watcher() -> SharedExtra {
    let shared: SharedExtra = Arc::new(Mutex::new(Extra::default()));
    let out = shared.clone();
    let _ = std::thread::Builder::new().name("sys-extra".into()).spawn(move || {
        let mut n = 0u32;
        loop {
            let fan = hwinfo_cpu_fan();
            let disk = (n % 5 == 0).then(read_disk);
            if let Ok(mut g) = out.lock() {
                g.cpu_fan_rpm = fan;
                if let Some(d) = disk {
                    g.disk = d;
                }
            }
            n = n.wrapping_add(1);
            std::thread::sleep(Duration::from_secs(2));
        }
    });
    shared
}

/// CPU fan RPM from HWiNFO's shared memory (`Global\HWiNFO_SENS_SM2`), if HWiNFO
/// is running with Settings -> "Shared Memory Support" enabled. `None` otherwise.
#[cfg(windows)]
pub fn hwinfo_cpu_fan() -> Option<u32> {
    hwinfo_fans().and_then(|fans| pick_cpu_fan(&fans))
}

#[cfg(not(windows))]
pub fn hwinfo_cpu_fan() -> Option<u32> {
    None
}

/// Picks the CPU fan among (label, rpm) readings: "CPU" / "CPU Fan" first, then any
/// label containing "CPU" (not "OPT"/"PUMP"), then "CPU OPT".
pub fn pick_cpu_fan(fans: &[(String, f64)]) -> Option<u32> {
    let l = |s: &str| s.to_ascii_lowercase();
    let exact = fans.iter().find(|(n, _)| matches!(l(n).trim(), "cpu" | "cpu fan" | "cpu_fan" | "cpu fan1"));
    let any = || fans.iter().find(|(n, _)| { let n = l(n); n.contains("cpu") && !n.contains("opt") && !n.contains("pump") });
    let opt = || fans.iter().find(|(n, _)| l(n).contains("cpu"));
    exact.or_else(any).or_else(opt).map(|(_, v)| v.max(0.0).round() as u32)
}

#[cfg(windows)]
fn hwinfo_fans() -> Option<Vec<(String, f64)>> {
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
    let mut out = Vec::new();
    // header (packed): signature 'HWiS' at 0, reading section offset/size/count at 32/36/40
    let sig = unsafe { std::slice::from_raw_parts(base, 4) };
    if sig == b"HWiS" {
        let (off, size, count) = (rd32(32) as usize, rd32(36) as usize, rd32(40) as usize);
        if size >= 292 && count < 10_000 {
            for i in 0..count {
                let e = off + i * size;
                if rd32(e) != 3 {
                    continue; // 3 = SENSOR_TYPE_FAN
                }
                let label = |o: usize| -> String {
                    let bytes = unsafe { std::slice::from_raw_parts(base.add(e + o), 128) };
                    let end = bytes.iter().position(|b| *b == 0).unwrap_or(128);
                    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
                };
                let user = label(140);
                let name = if user.is_empty() { label(12) } else { user };
                let value = unsafe { std::ptr::read_unaligned(base.add(e + 284) as *const f64) };
                out.push((name, value));
            }
        }
    }
    unsafe {
        UnmapViewOfFile(base as *const c_void);
        CloseHandle(h);
    }
    Some(out)
}

/// The system drive: `%SystemDrive%` on Windows, `/` elsewhere.
fn system_mount() -> String {
    #[cfg(windows)]
    {
        let d = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        format!("{}\\", d.trim_end_matches('\\'))
    }
    #[cfg(not(windows))]
    {
        "/".to_string()
    }
}

pub fn read_disk() -> Option<DiskInfo> {
    let mount = system_mount();
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk = disks
        .list()
        .iter()
        .find(|d| d.mount_point().to_string_lossy().eq_ignore_ascii_case(&mount))
        .or_else(|| disks.list().iter().max_by_key(|d| d.total_space()))?;
    let total = disk.total_space() as f64;
    let used = total - disk.available_space() as f64;
    let gb = 1024.0 * 1024.0 * 1024.0;
    let label = mount.trim_end_matches(['\\', '/']).to_string();
    Some(DiskInfo {
        label: if label.is_empty() { "/".into() } else { label },
        used_gb: used / gb,
        total_gb: total / gb,
        temp_c: disk_temperature(&mount),
    })
}

#[cfg(windows)]
fn disk_temperature(mount: &str) -> Option<i32> {
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateFileW(name: *const u16, access: u32, share: u32, sa: *mut c_void, disposition: u32, flags: u32, template: *mut c_void) -> isize;
        fn DeviceIoControl(h: isize, code: u32, inb: *const c_void, insz: u32, outb: *mut c_void, outsz: u32, ret: *mut u32, ov: *mut c_void) -> i32;
        fn CloseHandle(h: isize) -> i32;
    }
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;
    const OPEN_EXISTING: u32 = 3;
    const IOCTL_STORAGE_GET_DEVICE_NUMBER: u32 = 0x002D_1080;
    const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D_1400;
    // STORAGE_PROPERTY_ID::StorageDeviceTemperatureProperty (winioctl.h)
    const STORAGE_DEVICE_TEMPERATURE_PROPERTY: u32 = 52;

    let open = |path: &str| -> Option<isize> {
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let h = unsafe { CreateFileW(wide.as_ptr(), 0, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null_mut(), OPEN_EXISTING, 0, std::ptr::null_mut()) };
        (h != -1 && h != 0).then_some(h)
    };

    // 1. Which physical drive holds the system volume?
    let vol = format!("\\\\.\\{}", mount.trim_end_matches('\\'));
    let hv = open(&vol)?;
    let mut num = [0u32; 3]; // STORAGE_DEVICE_NUMBER: type, number, partition
    let mut ret = 0u32;
    let ok = unsafe {
        DeviceIoControl(hv, IOCTL_STORAGE_GET_DEVICE_NUMBER, std::ptr::null(), 0, num.as_mut_ptr() as *mut c_void, 12, &mut ret, std::ptr::null_mut())
    };
    unsafe { CloseHandle(hv) };
    if ok == 0 {
        return None;
    }

    // 2. Ask that drive for its temperature.
    let hd = open(&format!("\\\\.\\PhysicalDrive{}", num[1]))?;
    let query: [u32; 3] = [STORAGE_DEVICE_TEMPERATURE_PROPERTY, 0 /* PropertyStandardQuery */, 0];
    let mut buf = [0u8; 512];
    let ok = unsafe {
        DeviceIoControl(hd, IOCTL_STORAGE_QUERY_PROPERTY, query.as_ptr() as *const c_void, 12, buf.as_mut_ptr() as *mut c_void, buf.len() as u32, &mut ret, std::ptr::null_mut())
    };
    unsafe { CloseHandle(hd) };
    if ok == 0 || ret < 28 {
        return None;
    }
    // STORAGE_TEMPERATURE_DATA_DESCRIPTOR: InfoCount (u16) at 12, then
    // STORAGE_TEMPERATURE_INFO[] (16 bytes each) from 24; Temperature (i16) at +2.
    let count = u16::from_le_bytes([buf[12], buf[13]]) as usize;
    (0..count.min(8))
        .filter_map(|i| {
            let o = 24 + i * 16 + 2;
            (o + 2 <= ret as usize).then(|| i16::from_le_bytes([buf[o], buf[o + 1]]) as i32)
        })
        .find(|t| (1..=120).contains(t))
}

#[cfg(not(windows))]
fn disk_temperature(_mount: &str) -> Option<i32> {
    None
}

/// "AMD Ryzen 7 5800X 8-Core Processor" -> "AMD Ryzen 7 5800X".
pub fn clean_cpu_name(raw: &str) -> String {
    let mut s = raw.replace("(R)", "").replace("(TM)", "").replace("(tm)", "");
    for cut in [" with Radeon", " w/ Radeon", " CPU @", " @ "] {
        if let Some(i) = s.find(cut) {
            s.truncate(i);
        }
    }
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut out: Vec<&str> = Vec::new();
    for (i, w) in words.iter().enumerate() {
        let lw = w.to_ascii_lowercase();
        let core_word = lw.ends_with("-core") || lw == "processor" || (lw.ends_with("-core") && i + 1 < words.len());
        if core_word {
            break;
        }
        out.push(w);
    }
    let s = out.join(" ");
    if s.is_empty() { raw.trim().to_string() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_fan_is_picked() {
        let f = |v: &[(&str, f64)]| pick_cpu_fan(&v.iter().map(|(a, b)| (a.to_string(), *b)).collect::<Vec<_>>());
        assert_eq!(f(&[("System 1", 900.0), ("CPU", 1210.4), ("CPU OPT", 0.0)]), Some(1210));
        assert_eq!(f(&[("CPU OPT", 800.0)]), Some(800));
        assert_eq!(f(&[("Chassis", 800.0)]), None);
    }

    #[test]
    fn cpu_names_are_shortened() {
        assert_eq!(clean_cpu_name("AMD Ryzen 7 5800X 8-Core Processor"), "AMD Ryzen 7 5800X");
        assert_eq!(clean_cpu_name("Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz"), "Intel Core i7-9700K");
        assert_eq!(clean_cpu_name("AMD Ryzen 5 5600G with Radeon Graphics"), "AMD Ryzen 5 5600G");
    }
}
