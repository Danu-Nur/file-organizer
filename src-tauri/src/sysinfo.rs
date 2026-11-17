// SPDX-License-Identifier: GPL-3.0-or-later
//! What this computer has (RAM, CPU load, GPUs + VRAM, free disk) and what that means for the optional local AI.
//! Only advice: the user decides whether to download/enable anything.
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Clone)]
pub struct Gpu {
    pub name: String,
    pub vram: u64,
}

#[derive(Serialize)]
pub struct SysInfo {
    pub cpu_name: String,
    pub cores: usize,
    pub cpu_load: u32,
    pub ram_total: u64,
    pub ram_avail: u64,
    pub gpus: Vec<Gpu>,
    pub disk_free: u64,
    pub disk: String,
}

#[derive(Serialize)]
pub struct Check {
    pub label: &'static str,
    pub have: String,
    pub min: &'static str,
    pub rec: &'static str,
    /// ok | warn | bad
    pub state: &'static str,
}

#[derive(Serialize)]
pub struct Advice {
    /// ok | slow | no
    pub level: &'static str,
    pub title: String,
    pub points: Vec<String>,
    pub checks: Vec<Check>,
}

const GB: u64 = 1 << 30;
pub fn gb(b: u64) -> String {
    format!("{:.1} GB", b as f64 / GB as f64).replace('.', ",")
}

#[cfg(windows)]
mod win {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Registry::*;
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    pub fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    pub fn ram() -> (u64, u64) {
        let mut m: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if unsafe { GlobalMemoryStatusEx(&mut m) } == 0 {
            return (0, 0);
        }
        (m.ullTotalPhys, m.ullAvailPhys)
    }

    /// Whole-machine CPU use over a short sample.
    pub fn cpu_load() -> u32 {
        use windows_sys::Win32::System::Threading::GetSystemTimes;
        let t = |f: &FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
        let sample = || {
            let (mut i, mut k, mut u): (FILETIME, FILETIME, FILETIME) = unsafe { std::mem::zeroed() };
            unsafe { GetSystemTimes(&mut i, &mut k, &mut u) };
            (t(&i), t(&k) + t(&u)) // kernel time includes idle
        };
        let (i0, a0) = sample();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let (i1, a1) = sample();
        let (idle, all) = (i1.saturating_sub(i0), a1.saturating_sub(a0));
        if all == 0 { 0 } else { (100 - idle * 100 / all) as u32 }
    }

    fn reg_sz(key: &str, value: &str) -> Option<String> {
        let mut buf = vec![0u16; 512];
        let mut len = (buf.len() * 2) as u32;
        let r = unsafe { RegGetValueW(HKEY_LOCAL_MACHINE, wide(key).as_ptr(), wide(value).as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut len) };
        (r == 0).then(|| String::from_utf16_lossy(&buf[..(len as usize / 2).saturating_sub(1)]))
    }

    fn reg_u64(key: &str, value: &str) -> Option<u64> {
        let mut buf = [0u8; 8];
        let mut len = 8u32;
        let r = unsafe { RegGetValueW(HKEY_LOCAL_MACHINE, wide(key).as_ptr(), wide(value).as_ptr(), RRF_RT_ANY, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut len) };
        (r == 0).then(|| u64::from_le_bytes(buf))
    }

    pub fn cpu_name() -> String {
        reg_sz(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0", "ProcessorNameString").unwrap_or_default().trim().to_string()
    }

    /// Display adapters from the driver registry (dedicated VRAM; a 64-bit value, unlike WMI's 4 GB-capped one).
    pub fn gpus() -> Vec<super::Gpu> {
        let class = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";
        let mut out: Vec<super::Gpu> = Vec::new();
        for i in 0..16 {
            let key = format!(r"{class}\{i:04}");
            let Some(name) = reg_sz(&key, "DriverDesc") else { continue };
            let vram = reg_u64(&key, "HardwareInformation.qwMemorySize")
                .or_else(|| reg_u64(&key, "HardwareInformation.MemorySize").map(|v| v & 0xffff_ffff))
                .unwrap_or(0);
            if !name.contains("Basic Display") && !out.iter().any(|g| g.name == name) {
                out.push(super::Gpu { name, vram });
            }
        }
        out
    }

    /// Mounted drives that are ready: ("Label (D:)", "D:\").
    pub fn drives() -> Vec<(String, String)> {
        use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW};
        let mask = unsafe { GetLogicalDrives() };
        let mut out = Vec::new();
        for i in 0..26u32 {
            if mask & (1 << i) == 0 {
                continue;
            }
            let letter = (b'A' + i as u8) as char;
            let root = format!("{letter}:\\");
            let w = wide(&root);
            let mut label = vec![0u16; 261];
            // Fails for an empty card reader / DVD drive: skip it, like a drive that is not there.
            let ok = unsafe { GetVolumeInformationW(w.as_ptr(), label.as_mut_ptr(), 261, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), 0) };
            if ok == 0 {
                continue;
            }
            let end = label.iter().position(|&c| c == 0).unwrap_or(0);
            let mut name = String::from_utf16_lossy(&label[..end]);
            if name.is_empty() {
                name = match unsafe { GetDriveTypeW(w.as_ptr()) } { 2 => "Drive USB", 4 => "Drive jaringan", 5 => "DVD", _ => "Disk lokal" }.into();
            }
            out.push((format!("{name} ({letter}:)"), root));
        }
        out
    }

    pub fn disk_free(p: &std::path::Path) -> u64 {
        use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        let mut free = 0u64;
        let w = wide(&p.to_string_lossy());
        unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut free, std::ptr::null_mut(), std::ptr::null_mut()) };
        free
    }
}

pub fn drives() -> Vec<(String, String)> {
    #[cfg(windows)]
    return win::drives();
    #[cfg(not(windows))]
    Vec::new()
}

/// Bit per drive letter; cheap enough to poll for plugged/unplugged drives.
pub fn drive_mask() -> u32 {
    #[cfg(windows)]
    return unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
    #[cfg(not(windows))]
    0
}

pub fn ram() -> (u64, u64) {
    #[cfg(windows)]
    return win::ram();
    #[cfg(not(windows))]
    (0, 0)
}

pub fn collect(models_dir: &Path) -> SysInfo {
    let probe = models_dir.ancestors().find(|a| a.exists()).unwrap_or(models_dir);
    #[cfg(windows)]
    let (cpu_name, cpu_load, gpus, disk_free) = (win::cpu_name(), win::cpu_load(), win::gpus(), win::disk_free(probe));
    #[cfg(not(windows))]
    let (cpu_name, cpu_load, gpus, disk_free) = (String::new(), 0, Vec::new(), 0);
    let (ram_total, ram_avail) = ram();
    SysInfo {
        cpu_name, cpu_load, gpus, disk_free, ram_total, ram_avail,
        cores: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        disk: probe.components().next().map(|c| c.as_os_str().to_string_lossy().into_owned()).unwrap_or_default(),
    }
}

/// A GPU the model can run on fast: Vulkan-capable discrete card with enough VRAM for the 2B model (~1.3 GB weights +
/// context). Integrated Intel/AMD graphics share system RAM and are treated as CPU speed.
pub fn usable_gpu(s: &SysInfo) -> Option<&Gpu> {
    s.gpus.iter().filter(|g| g.vram >= 3 * GB - GB / 4).max_by_key(|g| g.vram)
}

/// Advice for the text model of `model_size` bytes, about the hardware only (whether this build has the AI engine is
/// shown separately). Never blocks anything: the UI shows it and the user decides.
pub fn advise(s: &SysInfo, model_size: u64, have_model: bool) -> Advice {
    let need_disk = model_size + GB / 2;
    let gpu = usable_gpu(s);
    let mut points = Vec::new();
    let state = |ok: bool, warn: bool| if ok { "ok" } else if warn { "warn" } else { "bad" };
    let checks = vec![
        Check { label: "RAM", have: format!("{} (tersedia {})", gb(s.ram_total), gb(s.ram_avail)), min: "8 GB", rec: "16 GB",
            state: state(s.ram_total >= 15 * GB, s.ram_total >= 7 * GB + GB / 2) },
        Check { label: "GPU", have: gpu.map(|g| format!("{} ({})", g.name, gb(g.vram))).or_else(|| s.gpus.first().map(|g| format!("{} — tidak cukup untuk AI", g.name))).unwrap_or_else(|| "tidak terdeteksi".into()),
            min: "tidak wajib (CPU)", rec: "GPU Vulkan ≥ 3 GB VRAM", state: state(gpu.is_some(), true) },
        Check { label: "CPU", have: format!("{} — {} thread, beban saat ini {}%", if s.cpu_name.is_empty() { "CPU" } else { &s.cpu_name }, s.cores, s.cpu_load), min: "4 thread", rec: "8 thread",
            state: state(s.cores >= 8, s.cores >= 4) },
        Check { label: "Disk", have: format!("{} kosong di {}", gb(s.disk_free), s.disk), min: "2 GB kosong", rec: "5 GB kosong",
            state: if have_model { "ok" } else { state(s.disk_free >= 5 * GB, s.disk_free >= need_disk) } },
    ];
    let mut level = "ok";
    if !have_model && s.disk_free < need_disk {
        level = "no";
        points.push(format!("Ruang disk kurang: butuh sekitar {}, tersedia {}.", gb(need_disk), gb(s.disk_free)));
    }
    if s.ram_total < 7 * GB + GB / 2 {
        level = if s.ram_total < 4 * GB { "no" } else if level == "ok" { "slow" } else { level };
        points.push(format!("RAM total {} — di bawah 8 GB; komputer bisa terasa berat saat AI berjalan.", gb(s.ram_total)));
    }
    match gpu {
        Some(g) => points.push(format!("GPU {} ({}) bisa dipakai: kira-kira 2–5 detik per dokumen.", g.name, gb(g.vram))),
        None => {
            if level == "ok" { level = "slow"; }
            points.push("Tidak ada GPU dengan VRAM ≥ 3 GB: AI berjalan di CPU, kira-kira 30–90 detik per dokumen. Cocok untuk sedikit file, tidak untuk ratusan.".into());
        }
    }
    if s.ram_avail < 3 * GB {
        points.push(format!("Saat ini RAM tersedia hanya {}. Tutup aplikasi berat dulu sebelum memakai AI.", gb(s.ram_avail)));
    }
    if s.cpu_load >= 60 {
        points.push(format!("Komputer sedang sibuk (CPU {}%). Memakai AI sekarang akan lambat dan bisa mengganggu pekerjaan lain.", s.cpu_load));
    }
    let title = match level {
        "ok" => "Komputer ini cocok untuk AI lokal".to_string(),
        "slow" => "Bisa dipakai, tapi akan lambat".to_string(),
        _ => "Tidak disarankan untuk komputer ini".to_string(),
    };
    Advice { level, title, points, checks }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(ram: u64, vram: u64, disk: u64) -> SysInfo {
        SysInfo { cpu_name: "x".into(), cores: 8, cpu_load: 5, ram_total: ram * GB, ram_avail: ram * GB / 2, disk_free: disk * GB, disk: "C:".into(),
            gpus: vec![Gpu { name: "GPU".into(), vram: vram * GB }] }
    }

    #[test]
    fn advice_levels() {
        assert_eq!(advise(&info(16, 4, 50), GB, false).level, "ok");
        assert_eq!(advise(&info(16, 1, 50), GB, false).level, "slow"); // no usable GPU → CPU
        assert_eq!(advise(&info(16, 4, 1), GB, false).level, "no"); // no disk room
        assert_eq!(advise(&info(16, 4, 1), GB, true).level, "ok"); // model already there: disk does not matter
        assert_eq!(advise(&info(6, 4, 50), GB, false).level, "slow"); // under 8 GB RAM
        let real = collect(&std::env::temp_dir());
        assert!(real.ram_total > 0 && real.cores > 0);
    }
}
