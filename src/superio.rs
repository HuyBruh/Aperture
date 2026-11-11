//! Fan speeds straight from the motherboard, with no other program running.
//!
//! Every desktop board has a "Super I/O" chip (Nuvoton, ITE or Fintek) that counts the
//! tachometer pulses of the fan headers. HWiNFO and LibreHardwareMonitor read that chip;
//! this module does the same through the signed PawnIO driver that is already used for the
//! CPU temperature (module `LpcIO`), so the CPU fan shows up without any of them.
//!
//! What is read: fan speeds only. Nothing is ever written to a fan-control register; the
//! only writes are the standard ones every monitoring tool performs to *address* the chip
//! (enter/leave configuration mode, select a register bank, and the Nuvoton "I/O space
//! lock" bit that some boards set at boot).
//!
//! The register maps follow the chips' datasheets and the Linux `nct6775`, `nct6683` and
//! `it87` drivers; the detection order and chip IDs match LibreHardwareMonitor (MPL-2.0),
//! which is the reference for which board uses which chip.
//!
//! On Linux the kernel already does this: fans come from `/sys/class/hwmon`.

/// One fan header.
#[derive(Clone, Debug, PartialEq)]
pub struct Fan {
    /// "Fan 2" (Windows: the header's number on the chip) or the kernel's label (Linux).
    pub label: String,
    pub rpm: u32,
}

/// What the board reports.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Board {
    /// "NCT6798D", "IT8689E" ... (several chips: joined with " + ").
    pub chip: String,
    pub fans: Vec<Fan>,
    /// Index in `fans` of the header that is the CPU fan on most boards with this chip.
    pub cpu_hint: Option<usize>,
}

impl Board {
    /// "NCT6798D / Fan 2": how a fan is named in `pro_fan_sensor`.
    pub fn selector(&self, fan: &Fan) -> String {
        format!("{} / {}", self.chip, fan.label)
    }

    /// The fan the user picked (`wanted` = a selector or just a label).
    pub fn selected(&self, wanted: &str) -> Option<u32> {
        let w = wanted.trim();
        self.fans.iter().find(|f| self.selector(f).eq_ignore_ascii_case(w) || f.label.eq_ignore_ascii_case(w)).map(|f| f.rpm)
    }

    /// The CPU fan: the usual header for this chip if it spins, otherwise the first fan that
    /// spins (a pump or a fan on another header), otherwise the usual header at 0 RPM.
    pub fn cpu_fan(&self) -> Option<u32> {
        let named = self.fans.iter().find(|f| {
            let l = f.label.to_ascii_lowercase();
            l.contains("cpu") && !l.contains("opt")
        });
        if let Some(f) = named {
            return Some(f.rpm);
        }
        let hint = self.cpu_hint.and_then(|i| self.fans.get(i));
        match hint {
            Some(f) if f.rpm > 0 => Some(f.rpm),
            _ => self.fans.iter().find(|f| f.rpm > 0).or(hint).map(|f| f.rpm),
        }
    }
}

/// Fan tachometer counts to RPM for the Nuvoton 13-bit counters.
/// `None` = the header reports nothing usable; `Some(0)` = connected but stopped / empty.
pub fn nct_count_rpm(high: u8, low: u8) -> Option<u32> {
    let count = (u32::from(high) << 5) | (u32::from(low) & 0x1F);
    if count >= 0x1FFF {
        Some(0)
    } else if count >= 0x15 {
        Some((1.35e6 / count as f64).round() as u32)
    } else {
        None
    }
}

/// ITE 16-bit tachometer value to RPM.
pub fn ite_rpm(value: u16) -> Option<u32> {
    if value <= 0x3F {
        None
    } else if value == 0xFFFF {
        Some(0)
    } else {
        Some((1.35e6 / (f64::from(value) * 2.0)).round() as u32)
    }
}

/// A plausible fan speed (drops the garbage a floating header can produce).
fn sane(rpm: u32) -> bool {
    rpm <= 30_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nuvoton_counts() {
        // 1125 counts -> 1200 RPM; an empty header reads all ones
        assert_eq!(nct_count_rpm((1125u32 >> 5) as u8, (1125u32 & 0x1F) as u8), Some(1200));
        assert_eq!(nct_count_rpm(0xFF, 0x1F), Some(0));
        assert_eq!(nct_count_rpm(0, 3), None);
    }

    #[test]
    fn ite_counts() {
        assert_eq!(ite_rpm(450), Some(1500));
        assert_eq!(ite_rpm(0xFFFF), Some(0));
        assert_eq!(ite_rpm(0x20), None);
    }

    #[test]
    fn cpu_fan_prefers_the_usual_header_then_anything_that_spins() {
        let fan = |l: &str, rpm| Fan { label: l.into(), rpm };
        let mut b = Board { chip: "NCT6798D".into(), fans: vec![fan("Fan 1", 900), fan("Fan 2", 1250), fan("Fan 3", 0)], cpu_hint: Some(1) };
        assert_eq!(b.cpu_fan(), Some(1250));
        b.fans[1].rpm = 0; // AIO: nothing on CPU_FAN, pump elsewhere
        assert_eq!(b.cpu_fan(), Some(900));
        b.fans[0].rpm = 0;
        assert_eq!(b.cpu_fan(), Some(0));
        assert_eq!(b.selected("nct6798d / fan 1"), Some(0));
        assert_eq!(b.selected("Fan 2"), Some(0));
        assert_eq!(b.selected("Fan 9"), None);
        let named = Board { chip: "hwmon".into(), fans: vec![fan("Chassis", 700), fan("CPU Fan", 1100)], cpu_hint: None };
        assert_eq!(named.cpu_fan(), Some(1100));
        assert_eq!(Board::default().cpu_fan(), None);
    }
}

// ---------------------------------------------------------------------------
// Windows: PawnIO LpcIO
// ---------------------------------------------------------------------------
#[cfg(windows)]
mod imp {
    use super::{ite_rpm, nct_count_rpm, sane, Board, Fan};
    use crate::pawnio::PawnIo;
    use std::time::{Duration, Instant};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

    /// Official PawnIO module for the LPC Super I/O ports (PawnIO.Modules, LGPL-2.1-or-later).
    static LPC_MODULE: &[u8] = include_bytes!("resources/LpcIO.bin");

    /// The mutex all hardware monitors share so they do not talk to the chip at once.
    struct IsaBus(HANDLE);

    impl IsaBus {
        fn open() -> Option<IsaBus> {
            let name: Vec<u16> = "Global\\Access_ISABUS.HTP.Method".encode_utf16().chain(std::iter::once(0)).collect();
            unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }.ok().map(IsaBus)
        }

        fn lock(&self, ms: u32) -> Option<IsaGuard<'_>> {
            let r = unsafe { WaitForSingleObject(self.0, ms) };
            (r == WAIT_OBJECT_0 || r == WAIT_ABANDONED).then_some(IsaGuard(self))
        }
    }

    impl Drop for IsaBus {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    struct IsaGuard<'a>(&'a IsaBus);

    impl Drop for IsaGuard<'_> {
        fn drop(&mut self) {
            unsafe {
                let _ = ReleaseMutex(self.0 .0);
            }
        }
    }

    /// One Super I/O slot (0x2E or 0x4E) behind its own PawnIO module instance.
    struct Lpc {
        io: PawnIo,
        port: u16,
    }

    impl Lpc {
        fn open(slot: u64) -> Option<Lpc> {
            let io = PawnIo::open_with_module(LPC_MODULE)?;
            io.execute("ioctl_select_slot", &[slot], 0)?;
            Some(Lpc { io, port: if slot == 0 { 0x2E } else { 0x4E } })
        }

        fn inb(&self, port: u16) -> Option<u8> {
            self.io.execute("ioctl_pio_inb", &[u64::from(port)], 1)?.first().map(|v| *v as u8)
        }

        fn outb(&self, port: u16, value: u8) -> Option<()> {
            self.io.execute("ioctl_pio_outb", &[u64::from(port), u64::from(value)], 0).map(|_| ())
        }

        /// Configuration-space register (only valid in configuration mode).
        fn cfg(&self, reg: u8) -> Option<u8> {
            self.io.execute("ioctl_superio_inb", &[u64::from(reg)], 1)?.first().map(|v| *v as u8)
        }

        fn cfg_word(&self, reg: u8) -> Option<u16> {
            self.io.execute("ioctl_superio_inw", &[u64::from(reg)], 1)?.first().map(|v| *v as u16)
        }

        fn cfg_write(&self, reg: u8, value: u8) -> Option<()> {
            self.io.execute("ioctl_superio_outb", &[u64::from(reg), u64::from(value)], 0).map(|_| ())
        }

        /// Lets the module learn the chip's I/O ranges (it refuses any other port).
        fn find_bars(&self) -> Option<()> {
            self.io.execute("ioctl_find_bars", &[], 0).map(|_| ())
        }

        /// The hardware-monitor base address of logical device `ldn`, read twice.
        fn base(&self, ldn: u8) -> Option<u16> {
            self.cfg_write(0x07, ldn)?;
            let a = self.cfg_word(0x60)?;
            std::thread::sleep(Duration::from_millis(1));
            let b = self.cfg_word(0x60)?;
            (a == b).then_some(a)
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Kind {
        /// NCT6771F / NCT6776F: RPM registers 0x656+.
        NctOld { fans: usize },
        /// NCT6779D ... NCT6799D: 13-bit counters 0x4B0+.
        Nct { fans: usize },
        /// NCT6106D / NCT6116D: RPM registers 0x030+.
        Nct610,
        /// NCT6683D / NCT6686D / NCT6687D (MSI): embedded-controller space, RPM at 0x140+.
        NctEc { dr: bool },
        /// ITE IT86xx / IT87xx.
        Ite { fans: usize, alt6: bool, wide: bool },
        /// Fintek F718xx.
        Fintek { fans: usize },
    }

    struct Chip {
        lpc: Lpc,
        kind: Kind,
        name: &'static str,
        base: u16,
        /// Headers that never reported anything: not read again every second.
        dead: Vec<u8>,
    }

    fn valid_base(a: u16) -> bool {
        a >= 0x100 && a & 0xF007 == 0
    }

    /// Nuvoton / Winbond / Fintek: (chip id, revision) -> chip.
    fn nuvoton_kind(id: u8, rev: u8, msi_new: bool) -> Option<(Kind, &'static str, u8)> {
        const LDN: u8 = 0x0B;
        Some(match (id, rev) {
            (0xB4, r) if r & 0xF0 == 0x70 => (Kind::NctOld { fans: 4 }, "NCT6771F", LDN),
            (0xC3, r) if r & 0xF0 == 0x30 => (Kind::NctOld { fans: 5 }, "NCT6776F", LDN),
            (0xC4, r) if r & 0xF0 == 0x50 => (Kind::Nct610, "NCT6106D", LDN),
            (0xC5, r) if r & 0xF0 == 0x60 => (Kind::Nct { fans: 5 }, "NCT6779D", LDN),
            (0xC7, 0x32) => (Kind::NctEc { dr: false }, "NCT6683D", LDN),
            (0xC8, 0x03) => (Kind::Nct { fans: 6 }, "NCT6791D", LDN),
            (0xC9, 0x11) => (Kind::Nct { fans: 6 }, "NCT6792D", LDN),
            (0xC9, 0x13) => (Kind::Nct { fans: 6 }, "NCT6792D-A", LDN),
            (0xD1, 0x21) => (Kind::Nct { fans: 6 }, "NCT6793D", LDN),
            (0xD3, 0x52) => (Kind::Nct { fans: 6 }, "NCT6795D", LDN),
            (0xD4, 0x23) => (Kind::Nct { fans: 6 }, "NCT6796D", LDN),
            (0xD4, 0x2A) => (Kind::Nct { fans: 7 }, "NCT6796D-R", LDN),
            (0xD4, 0x51) => (Kind::Nct { fans: 7 }, "NCT6797D", LDN),
            (0xD4, 0x2B) => (Kind::Nct { fans: 7 }, "NCT6798D", LDN),
            (0xD4, 0x40) | (0xD4, 0x41) => (Kind::NctEc { dr: false }, "NCT6686D", LDN),
            (0xD5, 0x92) => (Kind::NctEc { dr: msi_new }, if msi_new { "NCT6687D-R" } else { "NCT6687D" }, LDN),
            (0xD8, 0x02) => (Kind::Nct { fans: 7 }, "NCT6799D", LDN),
            (0xD8, 0x06) => (Kind::Nct { fans: 7 }, "NCT6701D", LDN),
            (0x05, 0x07) => (Kind::Fintek { fans: 4 }, "F71858", 0x02),
            (0x05, 0x41) => (Kind::Fintek { fans: 4 }, "F71882", 0x04),
            (0x06, 0x01) => (Kind::Fintek { fans: 3 }, "F71862", 0x04),
            (0x07, 0x23) => (Kind::Fintek { fans: 3 }, "F71889F", 0x04),
            (0x08, 0x14) => (Kind::Fintek { fans: 3 }, "F71869", 0x04),
            (0x09, 0x01) => (Kind::Fintek { fans: 3 }, "F71808E", 0x04),
            (0x09, 0x09) => (Kind::Fintek { fans: 3 }, "F71889ED", 0x04),
            (0x10, 0x05) => (Kind::Fintek { fans: 3 }, "F71889AD", 0x04),
            (0x10, 0x07) => (Kind::Fintek { fans: 3 }, "F71869A", 0x04),
            (0x11, 0x06) => (Kind::Fintek { fans: 3 }, "F71878AD", 0x04),
            _ => return None,
        })
    }

    fn ite_kind(id: u16) -> Option<(Kind, &'static str)> {
        let k = |fans, alt6, wide| Kind::Ite { fans, alt6, wide };
        Some(match id {
            0x8613 => (k(5, false, true), "IT8613E"),
            0x8620 => (k(5, false, true), "IT8620E"),
            0x8625 => (k(6, true, true), "IT8625E"),
            0x8628 => (k(6, false, true), "IT8628E"),
            0x8631 => (k(2, false, true), "IT8631E"),
            0x8638 => (k(2, false, true), "IT8638E"),
            0x8665 => (k(6, true, true), "IT8665E"),
            0x8655 => (k(3, false, true), "IT8655E"),
            0x8686 => (k(6, false, true), "IT8686E"),
            0x8688 => (k(6, false, true), "IT8688E"),
            0x8689 => (k(6, false, true), "IT8689E"),
            0x8696 => (k(6, false, true), "IT8696E"),
            0x8712 => (k(5, false, true), "IT8712F"),
            0x8716 => (k(5, false, true), "IT8716F"),
            0x8718 => (k(5, false, true), "IT8718F"),
            0x8720 => (k(5, false, true), "IT8720F"),
            0x8721 => (k(5, false, true), "IT8721F"),
            0x8726 => (k(5, false, true), "IT8726F"),
            0x8728 => (k(5, false, true), "IT8728F"),
            0x8771 => (k(5, false, true), "IT8771E"),
            0x8772 => (k(5, false, true), "IT8772E"),
            0x8790 => (k(3, false, true), "IT8790E"),
            0x8733 => (k(3, false, true), "IT8792E"),
            0x8695 => (k(3, false, true), "IT87952E"),
            _ => return None,
        })
    }

    impl Chip {
        /// Nuvoton banked register (address = bank << 8 | register).
        fn nct(&self, address: u16) -> Option<u8> {
            self.lpc.outb(self.base + 5, 0x4E)?;
            self.lpc.outb(self.base + 6, (address >> 8) as u8)?;
            self.lpc.outb(self.base + 5, address as u8)?;
            self.lpc.inb(self.base + 6)
        }

        /// NCT668x embedded-controller space (address = page << 8 | index).
        fn ec(&self, address: u16) -> Option<u8> {
            // wait for the page register to be free (0xFF), briefly; then take it
            let t0 = Instant::now();
            loop {
                if self.lpc.inb(self.base + 4)? == 0xFF {
                    break;
                }
                if t0.elapsed() > Duration::from_millis(40) {
                    self.lpc.outb(self.base + 4, 0xFF)?;
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            self.lpc.outb(self.base + 4, (address >> 8) as u8)?;
            self.lpc.outb(self.base + 5, address as u8)?;
            let v = self.lpc.inb(self.base + 6);
            self.lpc.outb(self.base + 4, 0xFF)?;
            v
        }

        /// ITE / Fintek plain index-data pair; `None` if the index did not stick (bus noise).
        fn idx(&self, reg: u8) -> Option<u8> {
            self.lpc.outb(self.base + 5, reg)?;
            let v = self.lpc.inb(self.base + 6)?;
            let check = self.name == "IT8688E" || !matches!(self.kind, Kind::Ite { .. }) || self.lpc.inb(self.base + 5)? == reg;
            check.then_some(v)
        }

        fn fan_count(&self) -> usize {
            match self.kind {
                Kind::NctOld { fans } | Kind::Nct { fans } | Kind::Ite { fans, .. } | Kind::Fintek { fans } => fans,
                Kind::Nct610 => 3,
                Kind::NctEc { .. } => 16,
            }
        }

        /// The header that carries the CPU fan on most boards with this chip.
        fn cpu_hint(&self) -> usize {
            match self.kind {
                // Nuvoton 67xx: SYSFANIN is input 0, CPUFANIN is input 1
                Kind::NctOld { .. } | Kind::Nct { .. } | Kind::Nct610 => 1,
                _ => 0,
            }
        }

        /// RPM of header `i`; `None` = not readable / no tachometer there.
        fn fan(&self, i: usize) -> Option<u32> {
            let rpm = match self.kind {
                Kind::NctOld { .. } => {
                    let r = 0x656 + 2 * i as u16;
                    let v = u32::from(self.nct(r)?) << 8 | u32::from(self.nct(r + 1)?);
                    if v > 20 { v } else { 0 }
                }
                Kind::Nct { .. } => {
                    const REG: [u16; 7] = [0x4B0, 0x4B2, 0x4B4, 0x4B6, 0x4B8, 0x4BA, 0x4CC];
                    let r = *REG.get(i)?;
                    nct_count_rpm(self.nct(r)?, self.nct(r + 1)?)?
                }
                Kind::Nct610 => {
                    let r = 0x030 + 2 * i as u16;
                    let v = u32::from(self.nct(r)?) << 8 | u32::from(self.nct(r + 1)?);
                    if v > 164 { v } else { 0 }
                }
                Kind::NctEc { dr } => {
                    const OLD: [u16; 16] = [0x140, 0x142, 0x144, 0x146, 0x148, 0x14A, 0x14C, 0x14E, 0x150, 0x152, 0x154, 0x156, 0x158, 0x15A, 0x15C, 0x15E];
                    const NEW: [u16; 16] = [0x140, 0x142, 0x144, 0x146, 0x148, 0x14A, 0x14C, 0x14E, 0x150, 0x152, 0x15E, 0x15C, 0x15A, 0x158, 0x156, 0x154];
                    let r = *(if dr { NEW } else { OLD }).get(i)?;
                    let (h, l) = (self.ec(r)?, self.ec(r + 1)?);
                    if h == 0xFF && l >= 0xF8 {
                        return None; // header not populated on this board
                    }
                    u32::from(h) << 8 | u32::from(l)
                }
                Kind::Ite { alt6, wide, .. } => {
                    const LOW: [u8; 6] = [0x0D, 0x0E, 0x0F, 0x80, 0x82, 0x4C];
                    const EXT: [u8; 6] = [0x18, 0x19, 0x1A, 0x81, 0x83, 0x4D];
                    let (lo, hi) = if alt6 && i == 5 { (0x93, 0x94) } else { (*LOW.get(i)?, *EXT.get(i)?) };
                    if !wide {
                        return None;
                    }
                    let v = u16::from(self.idx(lo)?) | u16::from(self.idx(hi)?) << 8;
                    ite_rpm(v)?
                }
                Kind::Fintek { .. } => {
                    let r = 0xA0 + 0x10 * i as u8;
                    let v = u32::from(self.idx(r)?) << 8 | u32::from(self.idx(r + 1)?);
                    if v == 0 {
                        return None;
                    }
                    if v < 0x0FFF { (1.5e6 / v as f64).round() as u32 } else { 0 }
                }
            };
            sane(rpm).then_some(rpm)
        }
    }

    /// Looks for a chip on one slot. Mirrors the order every monitor uses:
    /// Nuvoton/Winbond/Fintek key first, then the ITE key.
    fn detect(slot: u64, msi_new: bool) -> Option<Chip> {
        let lpc = Lpc::open(slot)?;
        let port = lpc.port;

        // --- Nuvoton / Winbond / Fintek: key 0x87 0x87, leave with 0xAA
        lpc.outb(port, 0x87)?;
        lpc.outb(port, 0x87)?;
        let (id, rev) = (lpc.cfg(0x20)?, lpc.cfg(0x21)?);
        if let Some((kind, name, ldn)) = nuvoton_kind(id, rev, msi_new) {
            lpc.find_bars();
            let mut base = lpc.base(ldn);
            let fintek_vendor = lpc.cfg_word(0x23);
            if base.is_some() && matches!(kind, Kind::Nct { .. }) && !matches!(name, "NCT6779D") {
                // NCT679x: some boards lock the hardware-monitor I/O space at boot
                if let Some(options) = lpc.cfg(0x28) {
                    if options & 0x10 != 0 {
                        let _ = lpc.cfg_write(0x28, options & !0x10);
                    }
                }
            }
            lpc.outb(port, 0xAA)?;
            if matches!(kind, Kind::Fintek { .. }) {
                if fintek_vendor != Some(0x1934) {
                    return None;
                }
                // some Fintek chips report the address with the +5 offset already added
                base = base.map(|a| if a & 0x07 == 0x05 { a & 0xFFF8 } else { a });
            }
            let base = base.filter(|a| valid_base(*a))?;
            let chip = Chip { lpc, kind, name, base, dead: Vec::new() };
            // the banked chips answer with Nuvoton's vendor id; anything else is not what we think
            let vendor_ok = match kind {
                Kind::Nct { .. } | Kind::NctOld { .. } if name != "NCT6701D" => {
                    (u16::from(chip.nct(0x804F)?) << 8 | u16::from(chip.nct(0x004F)?)) == 0x5CA3
                }
                Kind::Nct610 => (u16::from(chip.nct(0x80FE)?) << 8 | u16::from(chip.nct(0x00FE)?)) == 0x5CA3,
                _ => true,
            };
            return vendor_ok.then_some(chip);
        }
        if id != 0 && id != 0xFF {
            lpc.outb(port, 0xAA)?;
        }

        // --- ITE: key 0x87 0x01 0x55 0x55 (0xAA on the second slot).
        // A second-slot chip that already answers is left as it is (entering it again
        // confuses the IT8792E).
        let mut chip_id = if port == 0x4E { lpc.cfg_word(0x20).filter(|v| *v != 0xFFFF) } else { None };
        if chip_id.is_none() {
            lpc.outb(port, 0x87)?;
            lpc.outb(port, 0x01)?;
            lpc.outb(port, 0x55)?;
            lpc.outb(port, if port == 0x4E { 0xAA } else { 0x55 })?;
            chip_id = lpc.cfg_word(0x20);
        }
        let chip_id = chip_id?;
        let leave = |lpc: &Lpc| {
            // the second chip must stay in configuration mode
            if port != 0x4E {
                let _ = lpc.outb(port, 0x02);
                let _ = lpc.outb(port + 1, 0x02);
            }
        };
        let Some((kind, name)) = ite_kind(chip_id) else {
            if chip_id != 0 && chip_id != 0xFFFF {
                leave(&lpc);
            }
            return None;
        };
        lpc.find_bars();
        let base = lpc.base(0x04);
        let version = lpc.cfg(0x22).unwrap_or(0) & 0x0F;
        leave(&lpc);
        let base = base.filter(|a| valid_base(*a))?;
        let mut chip = Chip { lpc, kind, name, base, dead: Vec::new() };
        // vendor id 0x90 (or 0x7F), and bit 4 of the configuration register set
        let vendor = chip.idx(0x58)?;
        if vendor != 0x90 && vendor != 0x7F {
            return None;
        }
        let conf = chip.idx(0x00)?;
        if conf & 0x10 == 0 && !matches!(name, "IT8655E" | "IT8665E") {
            return None;
        }
        // very old revisions only have 8-bit counters: not supported here
        if name == "IT8712F" && version < 8 {
            if let Kind::Ite { fans, alt6, .. } = kind {
                chip.kind = Kind::Ite { fans, alt6, wide: false };
            }
        }
        // headers 4-6 exist only when the chip is told to count them
        if let (Kind::Ite { fans, .. }, Some(modes)) = (chip.kind, chip.idx(0x0C)) {
            if fans >= 5 {
                if modes & (1 << 4) == 0 {
                    chip.dead.push(3);
                }
                if modes & (1 << 5) == 0 {
                    chip.dead.push(4);
                }
            }
            if fans >= 6 {
                let on = if name == "IT8665E" { chip.idx(0x0B).is_some_and(|m| m & (1 << 3) != 0) } else { modes & (1 << 2) != 0 };
                if !on {
                    chip.dead.push(5);
                }
            }
        }
        Some(chip)
    }

    /// The board's fan reader. Created once; `read` every second.
    pub struct Reader {
        bus: IsaBus,
        chips: Vec<Chip>,
        polls: u32,
    }

    impl Reader {
        /// `None` when PawnIO is missing or no known chip answers.
        pub fn open() -> Option<Reader> {
            let bus = IsaBus::open()?;
            let msi_new = board_is_new_msi();
            let chips: Vec<Chip> = {
                let _guard = bus.lock(200)?;
                (0..2).filter_map(|slot| detect(slot, msi_new)).collect()
            };
            (!chips.is_empty()).then_some(Reader { bus, chips, polls: 0 })
        }

        pub fn chip_names(&self) -> String {
            self.chips.iter().map(|c| c.name).collect::<Vec<_>>().join(" + ")
        }

        /// All fan headers; `None` when the bus is busy right now (keep the last reading).
        pub fn read(&mut self) -> Option<Board> {
            let _guard = self.bus.lock(20)?;
            self.polls += 1;
            let many = self.chips.len() > 1;
            let mut board = Board { chip: self.chip_names(), fans: Vec::new(), cpu_hint: None };
            let mut number = 0usize;
            for (ci, chip) in self.chips.iter_mut().enumerate() {
                for i in 0..chip.fan_count() {
                    number += 1;
                    if chip.dead.contains(&(i as u8)) {
                        continue;
                    }
                    match chip.fan(i) {
                        // MSI's EC exposes 16 channels; past the eight headers only real fans
                        Some(0) if matches!(chip.kind, Kind::NctEc { .. }) && i >= 8 => {}
                        Some(rpm) => {
                            if ci == 0 && i == chip.cpu_hint() {
                                board.cpu_hint = Some(board.fans.len());
                            }
                            let name = match (chip.kind, i) {
                                // MSI boards (NCT668x): the header names printed on the board
                                (Kind::NctEc { .. }, 0) => "CPU Fan".to_string(),
                                (Kind::NctEc { .. }, 1) => "Pump Fan".to_string(),
                                (Kind::NctEc { .. }, 2..=7) => format!("System Fan {}", i - 1),
                                _ => format!("Fan {number}"),
                            };
                            let label = if many { format!("{name} ({})", chip.name) } else { name };
                            board.fans.push(Fan { label, rpm });
                        }
                        // never answered in the first polls: stop asking for it
                        None if self.polls >= 3 => chip.dead.push(i as u8),
                        None => {}
                    }
                }
            }
            // headers that are simply empty (always 0 RPM) are noise in a picker with 16
            // channels: keep at most the ones that have ever turned, plus the CPU header
            Some(board)
        }
    }

    /// MSI's 800-series boards use the "R" register order of the NCT6687D.
    fn board_is_new_msi() -> bool {
        use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
        let read = |value: &str| -> String {
            let key: Vec<u16> = "HARDWARE\\DESCRIPTION\\System\\BIOS".encode_utf16().chain(std::iter::once(0)).collect();
            let val: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
            let mut buf = [0u16; 256];
            let mut size = (buf.len() * 2) as u32;
            let ok = unsafe { RegGetValueW(HKEY_LOCAL_MACHINE, PCWSTR(key.as_ptr()), PCWSTR(val.as_ptr()), RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut size)) };
            if ok.is_err() {
                return String::new();
            }
            String::from_utf16_lossy(&buf[..(size as usize / 2).min(buf.len())]).trim_end_matches('\0').to_string()
        };
        let maker = read("BaseBoardManufacturer").to_ascii_lowercase();
        let product = read("BaseBoardProduct").to_ascii_uppercase();
        (maker.contains("micro-star") || maker.contains("msi")) && ["B840", "B850", "B860", "X870", "Z890"].iter().any(|m| product.contains(m))
    }
}

// ---------------------------------------------------------------------------
// Linux: the kernel's hwmon
// ---------------------------------------------------------------------------
#[cfg(target_os = "linux")]
mod imp {
    use super::{sane, Board, Fan};

    pub struct Reader {
        /// (chip name, label, path of fanN_input)
        inputs: Vec<(String, String, std::path::PathBuf)>,
    }

    impl Reader {
        pub fn open() -> Option<Reader> {
            let mut inputs = Vec::new();
            for dir in std::fs::read_dir("/sys/class/hwmon").ok()?.flatten() {
                let dir = dir.path();
                let chip = std::fs::read_to_string(dir.join("name")).unwrap_or_default().trim().to_string();
                // graphics cards report their own fan elsewhere on the dashboard
                if ["amdgpu", "nouveau", "nvidia", "i915", "xe"].contains(&chip.as_str()) {
                    continue;
                }
                for n in 1..=16 {
                    let input = dir.join(format!("fan{n}_input"));
                    if !input.is_file() {
                        continue;
                    }
                    let label = std::fs::read_to_string(dir.join(format!("fan{n}_label"))).map(|s| s.trim().to_string()).ok().filter(|s| !s.is_empty()).unwrap_or_else(|| format!("Fan {n}"));
                    inputs.push((chip.clone(), label, input));
                }
            }
            (!inputs.is_empty()).then_some(Reader { inputs })
        }

        pub fn chip_names(&self) -> String {
            let mut names: Vec<&str> = self.inputs.iter().map(|i| i.0.as_str()).collect();
            names.dedup();
            names.join(" + ")
        }

        pub fn read(&mut self) -> Option<Board> {
            let fans = self
                .inputs
                .iter()
                .filter_map(|(_, label, path)| {
                    let rpm = std::fs::read_to_string(path).ok()?.trim().parse::<u32>().ok()?;
                    sane(rpm).then(|| Fan { label: label.clone(), rpm })
                })
                .collect::<Vec<_>>();
            // the kernel drivers number the Nuvoton inputs the same way: fan2 = CPU
            let cpu_hint = (fans.len() > 1).then_some(1);
            Some(Board { chip: self.chip_names(), fans, cpu_hint })
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    use super::Board;
    pub struct Reader;
    impl Reader {
        pub fn open() -> Option<Reader> {
            None
        }
        pub fn chip_names(&self) -> String {
            String::new()
        }
        pub fn read(&mut self) -> Option<Board> {
            None
        }
    }
}

pub use imp::Reader;
