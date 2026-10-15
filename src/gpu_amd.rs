//! AMD GPU sensor (temperature, power, fan) via **AMD Display Library (ADL)** —
//! more precisely the PMLog path (`ADL2_New_QueryPMLogData_Get`) used by
//! RDNA/RDNA2/RDNA3 GPUs (RX 5000 and up), including the RX 6600.
//!
//! `atiadlxx.dll` is automatically installed alongside the Radeon driver — no
//! ADL SDK needed, no extra software needed. The DLL is loaded dynamically
//! (like pawnio.rs), so the build still succeeds on a machine without an AMD
//! GPU, and the program still runs (sensors N/A) on a machine without an
//! AMD GPU/driver.
//!
//! Four values are displayed:
//! - **Edge temperature** (°C) — GPU die surface temperature, equivalent to
//!   "GPU Temperature" in Radeon Software/HWiNFO. Sensor index 8 in the
//!   `ADL_PMLOG_SENSORS` enum.
//! - **ASIC Power** (Watts) — power draw of the entire GPU chip. Sensor index 23.
//! - **Fan RPM** — actual fan speed. Sensor index 14.
//! - **Fullscreen FPS** — via a different ADL path (`ADL2_Adapter_FrameMetrics_*`,
//!   not PMLog), using the same ADL2 context so a second connection doesn't
//!   need to be opened. Only populated when a game is actually running in
//!   true *exclusive fullscreen* mode — "borderless windowed" mode is not
//!   detected; this is a limitation of ADL itself, not a bug here.
//!
//! This struct ALSO stores the Hotspot/Junction temperature (index 27) so it
//! can easily be added to the display later if needed, but it is not
//! currently shown (row 1 is already crowded enough).
//!
//! Source: AMD's official ADL SDK (`adl_structures.h`, `ADL_PMLOG_SENSORS`
//! field), confirmed with `adl_probe.exe` output on actual RX 6600 hardware.

#[cfg(windows)]
mod imp {
    use std::ffi::{c_int, c_void, CString};

    use windows::core::PCSTR;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

    // === Types from adl_structures.h ===

    const ADL_MAX_PATH: usize = 256;

    /// Subset of `AdapterInfo` (adl_structures.h). The layout MUST be
    /// byte-exact with the original C header because it is read directly
    /// from memory filled in by the DLL.
    #[repr(C)]
    struct AdapterInfo {
        size: c_int,
        adapter_index: c_int,
        udid: [i8; ADL_MAX_PATH],
        bus_number: c_int,
        device_number: c_int,
        function_number: c_int,
        vendor_id: c_int,
        adapter_name: [i8; ADL_MAX_PATH],
        display_name: [i8; ADL_MAX_PATH],
        present: c_int,
        exist: c_int,
        driver_path: [i8; ADL_MAX_PATH],
        driver_path_ext: [i8; ADL_MAX_PATH],
        pnp_string: [i8; ADL_MAX_PATH],
        os_display_index: c_int,
    }

    impl Default for AdapterInfo {
        fn default() -> Self {
            // SAFETY: all fields are numeric/byte arrays, zeroed-out is valid.
            unsafe { std::mem::zeroed() }
        }
    }

    /// One sensor entry in `ADLPMLogDataOutput.sensors[]`.
    /// `supported` is a Win32 BOOL (4 bytes), not a 1-byte C++ bool.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct AdlSingleSensorData {
        supported: c_int,
        value: c_int,
    }

    /// `ADLPMLogDataOutput` (adl_structures.h): the `sensors[256]` array is
    /// indexed directly by the sensor ID from the `ADL_PMLOG_SENSORS` enum —
    /// not a packed list. `sensors[8]` = Edge temperature, `sensors[23]` =
    /// ASIC power, etc.
    #[repr(C)]
    struct AdlPMLogDataOutput {
        size: c_int,
        sensors: [AdlSingleSensorData; 256],
    }

    impl Default for AdlPMLogDataOutput {
        fn default() -> Self {
            unsafe { std::mem::zeroed() }
        }
    }

    // Sensor IDs from the ADL_PMLOG_SENSORS enum (adl_structures.h), confirmed
    // with adl_probe.exe output on an RX 6600.
    pub(super) const PMLOG_TEMPERATURE_EDGE: usize = 8;
    pub(super) const PMLOG_TEMPERATURE_HOTSPOT: usize = 27;
    pub(super) const PMLOG_FAN_RPM: usize = 14;
    pub(super) const PMLOG_ASIC_POWER: usize = 23;
    pub(super) const PMLOG_CLK_GFXCLK: usize = 1;
    pub(super) const PMLOG_CLK_MEMCLK: usize = 2;
    pub(super) const PMLOG_FAN_PERCENTAGE: usize = 15;
    pub(super) const PMLOG_TEMPERATURE_HOTSPOT_GCD: usize = 50;

    /// `ADLMemoryInfo2` (adl_structures.h), with spare room at the end in case a
    /// driver writes a longer version of the struct.
    #[repr(C)]
    struct AdlMemoryInfo2 {
        memory_size: i64,
        memory_type: [u8; ADL_MAX_PATH],
        memory_bandwidth: i64,
        hyper_memory_size: i64,
        invisible_memory_size: i64,
        visible_memory_size: i64,
        _spare: [u8; 512],
    }
    type AdlVramUsageFn = unsafe extern "system" fn(AdlContext, c_int, *mut c_int) -> c_int;
    type AdlMemoryInfo2Fn = unsafe extern "system" fn(AdlContext, c_int, *mut AdlMemoryInfo2) -> c_int;

    // === Function pointer types, resolved from the DLL ===

    type AdlMallocFn = unsafe extern "system" fn(c_int) -> *mut c_void;
    type AdlMainControlCreateFn = unsafe extern "system" fn(AdlMallocFn, c_int) -> c_int;
    type AdlMainControlDestroyFn = unsafe extern "system" fn() -> c_int;
    type AdlAdaptersGetFn = unsafe extern "system" fn(*mut c_int) -> c_int;
    type AdlAdapterInfoGetFn = unsafe extern "system" fn(*mut AdapterInfo, c_int) -> c_int;
    type AdlContext = *mut c_void;
    type AdlMainControlCreate2Fn =
        unsafe extern "system" fn(AdlMallocFn, c_int, *mut AdlContext) -> c_int;
    type AdlMainControlDestroy2Fn = unsafe extern "system" fn(AdlContext) -> c_int;
    type AdlPMLogQueryFn =
        unsafe extern "system" fn(AdlContext, c_int, *mut AdlPMLogDataOutput) -> c_int;

    // FrameMetrics (Fullscreen FPS) — an ADL path separate from PMLog, but
    // using the same ADL2 context and adapter_index.
    type AdlFrameMetricsCapsFn = unsafe extern "system" fn(AdlContext, c_int, *mut c_int) -> c_int;
    type AdlFrameMetricsStartFn = unsafe extern "system" fn(AdlContext, c_int, c_int) -> c_int;
    type AdlFrameMetricsGetFn =
        unsafe extern "system" fn(AdlContext, c_int, c_int, *mut f32) -> c_int;
    type AdlFrameMetricsStopFn = unsafe extern "system" fn(AdlContext, c_int, c_int) -> c_int;

    // Allocation callback ADL asks for at init. ADL only calls this
    // occasionally during initialization (not per-sample), so the small leak
    // here is not an issue in the context of a program that runs continuously.
    unsafe extern "system" fn adl_malloc(size: c_int) -> *mut c_void {
        if size <= 0 {
            return std::ptr::null_mut();
        }
        match std::alloc::Layout::from_size_align(size as usize, 8) {
            Ok(layout) => std::alloc::alloc(layout) as *mut c_void,
            Err(_) => std::ptr::null_mut(),
        }
    }

    unsafe fn resolve<T: Copy>(module: HMODULE, name: &str) -> Option<T> {
        let c_name = CString::new(name).ok()?;
        let addr = GetProcAddress(module, PCSTR(c_name.as_ptr() as *const u8))?;
        Some(std::mem::transmute_copy(&addr))
    }

    /// Internal ADL state: ADL2 context + target adapter index.
    /// Created once in `GpuAmdSensor::new()` and reused on every sample.
    pub(super) struct GpuAmdInner {
        query_fn: AdlPMLogQueryFn,
        // `Some` only if Caps + Start FrameMetrics succeeded during init.
        frame_metrics_get_fn: Option<AdlFrameMetricsGetFn>,
        frame_metrics_stop_fn: Option<AdlFrameMetricsStopFn>,
        destroy2_fn: Option<AdlMainControlDestroy2Fn>,
        context: AdlContext,
        adapter_index: c_int,
        vram_usage_fn: Option<AdlVramUsageFn>,
        vram_total_mb: Option<i32>,
        name: Option<String>,
    }

    impl GpuAmdInner {
        pub(super) fn new() -> Option<Self> {
            // Load the DLL (present in PATH once the Radeon driver is installed)
            let dll = CString::new("atiadlxx.dll").ok()?;
            let module = unsafe {
                LoadLibraryA(PCSTR(dll.as_ptr() as *const u8))
            }
            .ok()?;
            if module.is_invalid() {
                return None;
            }

            unsafe {
                // --- ADL v1: only used to enumerate adapters, then destroyed ---
                let create1 = resolve::<AdlMainControlCreateFn>(module, "ADL_Main_Control_Create")?;
                let get_num = resolve::<AdlAdaptersGetFn>(module, "ADL_Adapter_NumberOfAdapters_Get")?;
                let get_info = resolve::<AdlAdapterInfoGetFn>(module, "ADL_Adapter_AdapterInfo_Get")?;

                if create1(adl_malloc, 1) != 0 {
                    return None;
                }

                let mut num: c_int = 0;
                if get_num(&mut num) != 0 || num <= 0 {
                    return None;
                }

                let mut adapters: Vec<AdapterInfo> =
                    (0..num).map(|_| AdapterInfo::default()).collect();
                let buf_sz = (std::mem::size_of::<AdapterInfo>() as c_int) * num;
                if get_info(adapters.as_mut_ptr(), buf_sz) != 0 {
                    return None;
                }

                // Find the first "present" AMD adapter (vendor_id 1002 decimal —
                // not hex 0x1002 = 4098 — a slightly odd ADL quirk, confirmed
                // from probe output).
                let chosen = adapters.iter().find(|a| a.present != 0 && a.vendor_id == 1002)?;
                let adapter_index = chosen.adapter_index;
                let name = {
                    let raw: Vec<u8> = chosen.adapter_name.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
                    let n = String::from_utf8_lossy(&raw).trim().to_string();
                    (!n.is_empty()).then_some(n)
                };

                // Done with ADL v1, can destroy it now.
                if let Some(destroy1) =
                    resolve::<AdlMainControlDestroyFn>(module, "ADL_Main_Control_Destroy")
                {
                    destroy1();
                }

                // --- ADL2: for repeated PMLog queries ---
                let create2 =
                    resolve::<AdlMainControlCreate2Fn>(module, "ADL2_Main_Control_Create")?;
                let query_fn =
                    resolve::<AdlPMLogQueryFn>(module, "ADL2_New_QueryPMLogData_Get")?;
                let destroy2_fn =
                    resolve::<AdlMainControlDestroy2Fn>(module, "ADL2_Main_Control_Destroy");

                let mut context: AdlContext = std::ptr::null_mut();
                if create2(adl_malloc, 1, &mut context) != 0 || context.is_null() {
                    return None;
                }

                // --- FrameMetrics (Fullscreen FPS) — uses the context above ---
                // If the adapter doesn't support it, or one of the symbols is
                // missing (old driver), `frame_metrics_get_fn` stays `None` and
                // `sample()` automatically reports fps as N/A. This does not
                // affect the PMLog sensors already working above.
                let (frame_metrics_get_fn, frame_metrics_stop_fn) = (|| -> Option<(
                    AdlFrameMetricsGetFn,
                    AdlFrameMetricsStopFn,
                )> {
                    // Closure = new scope: the `unsafe` block in the outer
                    // function does NOT automatically apply here, so it has
                    // to be repeated explicitly.
                    unsafe {
                        let caps_fn = resolve::<AdlFrameMetricsCapsFn>(
                            module,
                            "ADL2_Adapter_FrameMetrics_Caps",
                        )?;
                        let start_fn = resolve::<AdlFrameMetricsStartFn>(
                            module,
                            "ADL2_Adapter_FrameMetrics_Start",
                        )?;
                        let get_fn = resolve::<AdlFrameMetricsGetFn>(
                            module,
                            "ADL2_Adapter_FrameMetrics_Get",
                        )?;
                        let stop_fn = resolve::<AdlFrameMetricsStopFn>(
                            module,
                            "ADL2_Adapter_FrameMetrics_Stop",
                        )?;

                        let mut supported: c_int = 0;
                        if caps_fn(context, adapter_index, &mut supported) != 0 || supported == 0 {
                            return None;
                        }
                        if start_fn(context, adapter_index, 0) != 0 {
                            return None;
                        }
                        Some((get_fn, stop_fn))
                    }
                })()
                .map_or((None, None), |(g, s)| (Some(g), Some(s)));

                // VRAM: used (MB) every sample, total (bytes) once.
                let vram_usage_fn = resolve::<AdlVramUsageFn>(module, "ADL2_Adapter_DedicatedVRAMUsage_Get")
                    .or_else(|| resolve::<AdlVramUsageFn>(module, "ADL2_Adapter_VRAMUsage_Get"));
                #[allow(unused_unsafe)]
                let vram_total_mb = resolve::<AdlMemoryInfo2Fn>(module, "ADL2_Adapter_MemoryInfo2_Get").and_then(|f| unsafe {
                    let mut info: AdlMemoryInfo2 = std::mem::zeroed();
                    (f(context, adapter_index, &mut info) == 0 && info.memory_size > 0)
                        .then(|| (info.memory_size / (1024 * 1024)) as i32)
                });

                Some(GpuAmdInner {
                    query_fn,
                    frame_metrics_get_fn,
                    frame_metrics_stop_fn,
                    destroy2_fn,
                    context,
                    adapter_index,
                    vram_usage_fn,
                    vram_total_mb,
                    name,
                })
            }
        }

        pub(super) fn sample(&self) -> super::GpuAmdData {
            let mut output = AdlPMLogDataOutput::default();
            output.size = std::mem::size_of::<AdlPMLogDataOutput>() as c_int;

            let status = unsafe { (self.query_fn)(self.context, self.adapter_index, &mut output) };
            if status != 0 {
                return super::GpuAmdData::default();
            }

            // Read a sensor value — returns None if `supported` == 0
            let get = |idx: usize| -> Option<i32> {
                let s = output.sensors[idx];
                if s.supported != 0 { Some(s.value) } else { None }
            };

            // Fullscreen FPS: `None` if FrameMetrics is unsupported/failed to
            // start (see `new()`), OR if ADL returns -1 (meaning no
            // exclusive-fullscreen game is currently running).
            let fps = self.frame_metrics_get_fn.and_then(|get_fn| {
                let mut value: f32 = -1.0;
                let status =
                    unsafe { get_fn(self.context, self.adapter_index, 0, &mut value) };
                if status == 0 && value >= 0.0 {
                    Some(value.round() as i32)
                } else {
                    None
                }
            });

            let vram_used_mb = self.vram_usage_fn.and_then(|f| {
                let mut mb: c_int = -1;
                let st = unsafe { f(self.context, self.adapter_index, &mut mb) };
                (st == 0 && mb >= 0).then_some(mb)
            });

            super::GpuAmdData {
                temp_edge_c: get(PMLOG_TEMPERATURE_EDGE),
                temp_hotspot_c: get(PMLOG_TEMPERATURE_HOTSPOT)
                    .filter(|t| *t > 0)
                    .or_else(|| get(PMLOG_TEMPERATURE_HOTSPOT_GCD).filter(|t| *t > 0)),
                power_w: get(PMLOG_ASIC_POWER),
                fan_rpm: get(PMLOG_FAN_RPM),
                fps,
                clock_mhz: get(PMLOG_CLK_GFXCLK),
                mem_clock_mhz: get(PMLOG_CLK_MEMCLK),
                fan_pct: get(PMLOG_FAN_PERCENTAGE),
                vram_used_mb,
                vram_total_mb: self.vram_total_mb,
                name: self.name.clone(),
            }
        }

        /// Every PMLog sensor the driver reports as supported (for `--diag`).
        pub(super) fn dump(&self) -> String {
            let mut output = AdlPMLogDataOutput::default();
            output.size = std::mem::size_of::<AdlPMLogDataOutput>() as c_int;
            let status = unsafe { (self.query_fn)(self.context, self.adapter_index, &mut output) };
            let mut out = format!("ADL PMLog status={status} name={:?} vram_total_mb={:?}\n", self.name, self.vram_total_mb);
            for (i, s) in output.sensors.iter().enumerate() {
                if s.supported != 0 {
                    out.push_str(&format!("  sensor[{i}] = {}\n", s.value));
                }
            }
            if let Some(f) = self.vram_usage_fn {
                let mut mb: c_int = -1;
                let st = unsafe { f(self.context, self.adapter_index, &mut mb) };
                out.push_str(&format!("  vram_used: status={st} mb={mb}\n"));
            }
            out
        }
    }

    impl Drop for GpuAmdInner {
        fn drop(&mut self) {
            if let Some(stop) = self.frame_metrics_stop_fn {
                unsafe { stop(self.context, self.adapter_index, 0) };
            }
            if let Some(destroy) = self.destroy2_fn {
                unsafe { destroy(self.context) };
            }
        }
    }

    // SAFETY: accessed from a single thread (main loop). No concurrent access.
    unsafe impl Send for GpuAmdInner {}
}

// ---------------------------------------------------------------------------
// Linux: sysfs hwmon (amdgpu kernel driver) — no extra ADL library/driver
// needed, just read plain text files under
// /sys/class/drm/cardN/device/hwmon/hwmonM/.
//
// There is NO path for "Fullscreen FPS" here: that's a Windows-specific ADL
// feature (`ADL2_Adapter_FrameMetrics_*`), there is no equivalent generic API
// on Linux — each compositor (X11/Wayland/gamescope) has its own way (or no
// way at all) to expose this number. `fps` is always `None` on this platform.
// ---------------------------------------------------------------------------
#[cfg(target_os = "linux")]
mod imp {
    use std::fs;
    use std::path::{Path, PathBuf};

    pub(super) struct GpuAmdInner {
        edge_path: Option<PathBuf>,
        hotspot_path: Option<PathBuf>,
        power_path: Option<PathBuf>,
        fan_path: Option<PathBuf>,
    }

    impl GpuAmdInner {
        pub(super) fn new() -> Option<Self> {
            let hwmon_dir = find_amd_gpu_hwmon()?;
            let edge_path = temp_path_for(&hwmon_dir, "edge", 1);
            let hotspot_path = temp_path_for(&hwmon_dir, "junction", 2);
            let power_path = existing(hwmon_dir.join("power1_average"))
                .or_else(|| existing(hwmon_dir.join("power1_input")));
            let fan_path = existing(hwmon_dir.join("fan1_input"));
            Some(Self { edge_path, hotspot_path, power_path, fan_path })
        }

        pub(super) fn sample(&self) -> super::GpuAmdData {
            super::GpuAmdData {
                temp_edge_c: self.edge_path.as_deref().and_then(read_i64).map(|m| (m / 1000) as i32),
                temp_hotspot_c: self.hotspot_path.as_deref().and_then(read_i64).map(|m| (m / 1000) as i32),
                power_w: self.power_path.as_deref().and_then(read_i64).map(|u| (u / 1_000_000) as i32),
                fan_rpm: self.fan_path.as_deref().and_then(read_i64).map(|v| v as i32),
                ..Default::default()
            }
        }
    }

    fn existing(path: PathBuf) -> Option<PathBuf> {
        path.exists().then_some(path)
    }

    fn read_i64(path: &Path) -> Option<i64> {
        fs::read_to_string(path).ok()?.trim().parse().ok()
    }

    /// Find the hwmon folder (`/sys/class/drm/cardN/device/hwmon/hwmonM`) for
    /// the card with PCI vendor ID `0x1002` (AMD/ATI). Usually there is only
    /// 1 hwmonM subfolder inside `device/hwmon/`.
    fn find_amd_gpu_hwmon() -> Option<PathBuf> {
        let entries = fs::read_dir("/sys/class/drm").ok()?;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Some(suffix) = name.strip_prefix("card") else { continue };
            if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            let device_dir = entry.path().join("device");
            let is_amd = fs::read_to_string(device_dir.join("vendor"))
                .map(|v| v.trim().eq_ignore_ascii_case("0x1002"))
                .unwrap_or(false);
            if !is_amd {
                continue;
            }

            if let Ok(hwmon_entries) = fs::read_dir(device_dir.join("hwmon")) {
                if let Some(hw) = hwmon_entries.flatten().next() {
                    return Some(hw.path());
                }
            }
        }
        None
    }

    /// Find the `tempN_input` whose label (`tempN_label`) contains
    /// `label_substr` (e.g. "edge", "junction"). If not found (older driver
    /// version without labels), fall back to `temp{fallback_index}_input`
    /// as-is.
    fn temp_path_for(hwmon_dir: &Path, label_substr: &str, fallback_index: u32) -> Option<PathBuf> {
        if let Ok(entries) = fs::read_dir(hwmon_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                if let Some(idx) = file_name.strip_prefix("temp").and_then(|s| s.strip_suffix("_label")) {
                    if let Ok(label) = fs::read_to_string(entry.path()) {
                        if label.to_lowercase().contains(label_substr) {
                            return existing(hwmon_dir.join(format!("temp{idx}_input")));
                        }
                    }
                }
            }
        }
        existing(hwmon_dir.join(format!("temp{fallback_index}_input")))
    }
}

// ---------------------------------------------------------------------------

/// Result of one AMD GPU sensor reading.
/// All fields are `Option` — `None` if the sensor is not supported by this
/// GPU/driver, or if ADL is not available at all.
#[derive(Default, Clone, Debug)]
pub struct GpuAmdData {
    /// Edge/die surface temperature (°C) — equivalent to "GPU Temperature" in HWiNFO.
    pub temp_edge_c: Option<i32>,
    /// Hotspot/Junction temperature (°C) — the hottest point on the die. Present
    /// but not shown on row 1 to avoid clutter (easy to add later).
    pub temp_hotspot_c: Option<i32>,
    /// Power draw of the entire GPU chip (Watts).
    pub power_w: Option<i32>,
    /// GPU fan speed (RPM).
    pub fan_rpm: Option<i32>,
    /// Fullscreen FPS via ADL FrameMetrics. `None` if no exclusive-fullscreen
    /// game is currently running, the GPU/driver doesn't support it, or ADL
    /// is not available at all.
    pub fps: Option<i32>,
    /// GPU core clock (MHz), from NVML/nvidia-smi or ADL PMLog (GFXCLK).
    pub clock_mhz: Option<i32>,
    /// Memory clock (MHz), ADL PMLog.
    pub mem_clock_mhz: Option<i32>,
    /// Fan speed in percent, ADL PMLog.
    pub fan_pct: Option<i32>,
    /// Dedicated VRAM in use / total (MB), ADL.
    pub vram_used_mb: Option<i32>,
    pub vram_total_mb: Option<i32>,
    /// Adapter name reported by the driver ("AMD Radeon RX 6600").
    pub name: Option<String>,
}

#[cfg(windows)]
type GpuAmdInnerAlias = imp::GpuAmdInner;
#[cfg(target_os = "linux")]
type GpuAmdInnerAlias = imp::GpuAmdInner;
#[cfg(not(any(windows, target_os = "linux")))]
type GpuAmdInnerAlias = ();

/// Public `GpuAmdSensor` wrapper — can always be constructed on any platform,
/// but is only active on Windows (AMD GPU + Radeon driver) or Linux (AMD GPU
/// + amdgpu kernel driver).
pub struct GpuAmdSensor {
    inner: Option<GpuAmdInnerAlias>,
}

impl GpuAmdSensor {
    /// Initialize. If the sensor isn't found (missing driver/DLL, or not an
    /// AMD GPU), print a warning and continue (sensor N/A).
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            let inner = imp::GpuAmdInner::new();
            if inner.is_none() {
                eprintln!(
                    "AMD GPU ADL: not available — make sure the AMD Radeon driver \
                     is installed and the GPU is AMD. GPU temperature/power/fan will be N/A."
                );
            }
            return GpuAmdSensor { inner };
        }

        #[cfg(target_os = "linux")]
        {
            let inner = imp::GpuAmdInner::new();
            if inner.is_none() {
                eprintln!(
                    "AMD GPU hwmon: not available — make sure the GPU is AMD and the \
                     amdgpu kernel driver is active (check: ls /sys/class/drm/*/device/hwmon). \
                     GPU temperature/power/fan will be N/A. Fullscreen FPS is always N/A \
                     on Linux (no generic API for it)."
                );
            }
            return GpuAmdSensor { inner };
        }

        #[cfg(not(any(windows, target_os = "linux")))]
        GpuAmdSensor { inner: None }
    }

    /// Read the sensor. Called on every sysinfo refresh interval (~500 ms),
    /// not every frame — the cost is indeed light, but there's no need to do
    /// it every frame.
    pub fn sample(&self) -> GpuAmdData {
        if let Some(inner) = &self.inner {
            return inner.sample();
        }
        GpuAmdData::default()
    }

    /// Raw dump of every sensor (for `--diag`).
    pub fn dump(&self) -> String {
        #[cfg(windows)]
        if let Some(inner) = &self.inner {
            return inner.dump();
        }
        String::from("ADL: not available\n")
    }
}
