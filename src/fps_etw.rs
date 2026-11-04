//! FPS of the foreground game, for ANY GPU (NVIDIA/AMD/Intel), via ETW:
//! it listens for DXGI `Present` events (provider Microsoft-Windows-DXGI,
//! event 42 = Present_Start) — the same source PresentMon uses — and counts
//! Presents per second for the process that owns the foreground window.
//!
//! Honest notes:
//! - The program needs to run **as administrator** (real-time ETW).
//! - Covers games that present through DXGI, usually DirectX 10/11/12. Native
//!   Vulkan / OpenGL games don't generate these events and will be left
//!   without FPS.
//! - Windows only; on other systems `start()` returns an error.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    NotStarted,
    Running,
    /// ETW couldn't be started (usually: missing administrator access).
    NeedsAdmin,
    Failed,
}

/// Process-scoped ETW is independent of the output selected in game settings.
/// Keep the last presenting process through transient focus loss during an
/// output switch; never keep a cached FPS once that process stops presenting.
#[derive(Default)]
pub struct Target {
    pid: Option<u32>,
}

impl Target {
    pub fn sample(&mut self, foreground: Option<u32>, mut read: impl FnMut(u32) -> Option<u32>) -> Option<u32> {
        if let Some(pid) = foreground {
            if let Some(fps) = read(pid).filter(|fps| *fps > 0) {
                self.pid = Some(pid);
                return Some(fps);
            }
        }
        self.pid.and_then(|pid| read(pid)).filter(|fps| *fps > 0)
    }
}

/// ADL FrameMetrics is tied to an adapter/fullscreen presentation path. It
/// must not override measured per-process frames on another display.
pub fn prefer_process_fps(process: Option<u32>, driver: Option<i32>) -> Option<i32> {
    process.filter(|fps| *fps > 0).map(|fps| fps.min(i32::MAX as u32) as i32)
        .or_else(|| driver.filter(|fps| *fps > 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_frames_override_zero_or_stale_driver_frames() {
        assert_eq!(prefer_process_fps(Some(95), Some(0)), Some(95));
        assert_eq!(prefer_process_fps(Some(95), Some(60)), Some(95));
        assert_eq!(prefer_process_fps(None, Some(60)), Some(60));
        assert_eq!(prefer_process_fps(None, Some(0)), None);
        assert_eq!(prefer_process_fps(None, Some(-1)), None);
    }

    #[test]
    fn output_switch_keeps_live_process_but_never_stale_frames() {
        let mut target = Target::default();
        assert_eq!(target.sample(Some(7), |pid| (pid == 7).then_some(95)), Some(95));
        assert_eq!(target.sample(None, |pid| (pid == 7).then_some(90)), Some(90));
        assert_eq!(target.sample(Some(9), |pid| (pid == 7).then_some(88)), Some(88));
        assert_eq!(target.sample(Some(9), |_| None), None);
        assert_eq!(target.sample(Some(10), |pid| (pid == 10).then_some(120)), Some(120));
        assert_eq!(target.sample(None, |pid| (pid == 7).then_some(100)), None);
    }
}

#[cfg(windows)]
mod imp {
    use super::Status;
    use std::collections::{HashMap, VecDeque};
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::{Mutex, OnceLock};
    use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
    use windows::core::{GUID, PCWSTR, PWSTR};
    use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
    use windows::Win32::System::Diagnostics::Etw::*;

    const SESSION_NAME: &str = "TrofeoLcdFpsSession";
    /// Microsoft-Windows-DXGI
    const DXGI_PROVIDER: GUID = GUID::from_u128(0xCA11C036_0102_4A2D_A6AD_F03CFED5D3C9);
    const DXGI_PRESENT_START: u16 = 42;
    type SwapChains = HashMap<(u32, u64), VecDeque<i64>>;
    static PRESENTS: OnceLock<Mutex<SwapChains>> = OnceLock::new();
    static STATUS: AtomicU8 = AtomicU8::new(0);
    static FREQUENCY: OnceLock<i64> = OnceLock::new();

    fn presents() -> &'static Mutex<SwapChains> {
        PRESENTS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub fn status() -> Status {
        match STATUS.load(Ordering::Relaxed) {
            1 => Status::Running,
            2 => Status::NeedsAdmin,
            3 => Status::Failed,
            _ => Status::NotStarted,
        }
    }

    unsafe extern "system" fn on_event(rec: *mut EVENT_RECORD) {
        if rec.is_null() {
            return;
        }
        let h = &(*rec).EventHeader;
        if h.ProviderId != DXGI_PROVIDER || !matches!(h.EventDescriptor.Id, DXGI_PRESENT_START | 55) {
            return;
        }
        if h.EventDescriptor.Version != 0 || (*rec).UserData.is_null() {
            return;
        }
        let pointer_bytes = if h.Flags & 0x20 != 0 { 4 } else { 8 };
        if usize::from((*rec).UserDataLength) < pointer_bytes + 4 {
            return;
        }
        let payload = std::slice::from_raw_parts((*rec).UserData.cast::<u8>(), pointer_bytes + 4);
        let mut address = [0; 8];
        address[..pointer_bytes].copy_from_slice(&payload[..pointer_bytes]);
        let flags = u32::from_le_bytes(payload[pointer_bytes..].try_into().unwrap());
        if flags & 1 != 0 { // DXGI_PRESENT_TEST
            return;
        }
        let timestamp = h.TimeStamp;
        let Some(&frequency) = FREQUENCY.get() else { return };
        if let Ok(mut map) = presents().lock() {
            if map.len() >= 256 {
                map.retain(|_, events| events.back().is_some_and(|last| timestamp - last <= frequency));
            }
            let key = (h.ProcessId, u64::from_le_bytes(address));
            let queue = map.entry(key).or_default();
            queue.push_back(timestamp);
            while queue.front().is_some_and(|t| timestamp - t > frequency) {
                queue.pop_front();
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// EVENT_TRACE_PROPERTIES buffer + session name, 8-byte aligned.
    fn make_props() -> Vec<u64> {
        let name_bytes = (SESSION_NAME.len() + 1) * 2;
        let total = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + name_bytes + 64;
        let mut buf = vec![0u64; total.div_ceil(8)];
        let p = buf.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
        unsafe {
            (*p).Wnode.BufferSize = (buf.len() * 8) as u32;
            (*p).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
            (*p).Wnode.ClientContext = 1; // QPC
            (*p).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
            (*p).LoggerNameOffset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;
        }
        buf
    }

    pub fn start() -> Result<(), String> {
        let mut frequency = 0;
        unsafe { QueryPerformanceFrequency(&mut frequency) }
            .map_err(|error| format!("QPC frequency: {error}"))?;
        if frequency <= 0 { return Err("Invalid QPC frequency".into()); }
        let _ = FREQUENCY.set(frequency);

        let name = wide(SESSION_NAME);
        let mut props = make_props();
        let mut handle = CONTROLTRACE_HANDLE::default();
        unsafe {
            let mut err = StartTraceW(
                &mut handle,
                PCWSTR(name.as_ptr()),
                props.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES,
            );
            if err == ERROR_ALREADY_EXISTS {
                // A session left over from a previous run (crash): stop it and retry.
                let mut old = make_props();
                let _ = ControlTraceW(
                    CONTROLTRACE_HANDLE::default(),
                    PCWSTR(name.as_ptr()),
                    old.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES,
                    EVENT_TRACE_CONTROL_STOP,
                );
                props = make_props();
                err = StartTraceW(
                    &mut handle,
                    PCWSTR(name.as_ptr()),
                    props.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES,
                );
            }
            if err.0 != 0 {
                STATUS.store(if err.0 == 5 { 2 } else { 3 }, Ordering::Relaxed);
                return Err(format!("StartTrace error {} (5 = administrator required)", err.0));
            }

            let err = EnableTraceEx2(
                handle,
                &DXGI_PROVIDER,
                EVENT_CONTROL_CODE_ENABLE_PROVIDER.0,
                5, // TRACE_LEVEL_VERBOSE
                u64::MAX,
                0,
                0,
                None,
            );
            if err.0 != 0 {
                STATUS.store(3, Ordering::Relaxed);
                return Err(format!("EnableTraceEx2 error {}", err.0));
            }
        }

        let logger = wide(SESSION_NAME);
        std::thread::spawn(move || unsafe {
            let mut logger = logger;
            let mut file: EVENT_TRACE_LOGFILEW = std::mem::zeroed();
            file.LoggerName = PWSTR(logger.as_mut_ptr());
            file.Anonymous1.ProcessTraceMode =
                PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD | PROCESS_TRACE_MODE_RAW_TIMESTAMP;
            file.Anonymous2.EventRecordCallback = Some(on_event);
            let th = OpenTraceW(&mut file);
            if th.Value == u64::MAX {
                STATUS.store(3, Ordering::Relaxed);
                eprintln!("FPS: OpenTrace failed");
                return;
            }
            let _ = ProcessTrace(&[th], None, None); // blocks while the session is alive
            STATUS.store(3, Ordering::Relaxed);
        });
        STATUS.store(1, Ordering::Relaxed);
        Ok(())
    }

    /// FPS (Presents/s in the last second) for process `pid`; `None` if too little data.
    pub fn fps_for_pid(pid: u32) -> Option<u32> {
        let frequency = *FREQUENCY.get()?;
        let mut now = 0;
        unsafe { QueryPerformanceCounter(&mut now) }.ok()?;
        let mut map = presents().lock().ok()?;
        let cutoff = now - frequency;
        map.retain(|_, events| events.back().is_some_and(|last| *last >= cutoff));
        map.iter()
            .filter(|((process, _), _)| *process == pid)
            .map(|(_, events)| events.iter().filter(|&&time| time >= cutoff && time <= now).count() as u32)
            .max()
            .filter(|&fps| fps >= 5)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Status;
    pub fn status() -> Status {
        Status::NotStarted
    }
    pub fn start() -> Result<(), String> {
        Err("FPS via ETW is only available on Windows".into())
    }
    pub fn fps_for_pid(_pid: u32) -> Option<u32> {
        None
    }
}

pub use imp::{fps_for_pid, start, status};
