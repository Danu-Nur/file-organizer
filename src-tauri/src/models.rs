// SPDX-License-Identifier: GPL-3.0-or-later
//! Optional local-AI model: where it is, download (resumable, SHA-256 checked) or reuse of an existing copy, settings.
//! Nothing here runs unless the user clicks; progress goes to the UI as "ai-model" events.
use crate::Db;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

pub struct Model {
    pub id: &'static str,
    pub label: &'static str,
    pub purpose: &'static str,
    pub file: &'static str,
    pub url: &'static str,
    pub source: &'static str,
    pub sha256: &'static str,
    pub size: u64,
}

/// Official upload pinned to a fixed revision; every byte is checked against the hash.
pub const TEXT: Model = Model {
    id: "text",
    label: "Model teks Qwen3.5-2B (Q4_K_M)",
    purpose: "Memilih subfolder topik untuk dokumen baru di Saran, dan tombol \"Kelompokkan per topik\" di Rapikan folder.",
    file: "qwen3.5-2b-q4_k_m.gguf",
    url: "https://huggingface.co/unsloth/Qwen3.5-2B-GGUF/resolve/f6d5376be1edb4d416d56da11e5397a961aca8ae/Qwen3.5-2B-Q4_K_M.gguf",
    source: "Hugging Face · unsloth/Qwen3.5-2B-GGUF (revisi f6d5376)",
    sha256: "aaf42c8b7c3cab2bf3d69c355048d4a0ee9973d48f16c731c0520ee914699223",
    size: 1_280_835_840,
};

static BUSY: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);

pub fn models_dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_default().join("file-organizer").join("models")
}

pub fn setting(db: &Db, key: &str) -> Option<String> {
    db.lock().unwrap().query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional().ok().flatten()
}
pub fn set_setting(db: &Db, key: &str, value: Option<&str>) {
    let c = db.lock().unwrap();
    match value {
        Some(v) => c.execute("INSERT INTO settings(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", params![key, v]),
        None => c.execute("DELETE FROM settings WHERE key = ?1", [key]),
    }.ok();
}

/// The verified model file, if the user set one up and it is still there.
pub fn model_path(db: &Db) -> Option<PathBuf> {
    setting(db, "ai_model_path").map(PathBuf::from).filter(|p| p.metadata().map(|m| m.len() == TEXT.size).unwrap_or(false))
}

/// AI is used only when compiled in, switched on by the user, and the model is present.
pub fn ai_ready(db: &Db) -> Option<PathBuf> {
    if !cfg!(feature = "local-llm") || setting(db, "ai_enabled").as_deref() != Some("1") {
        return None;
    }
    model_path(db)
}

#[derive(Serialize)]
pub struct Status {
    engine: bool,
    enabled: bool,
    busy: bool,
    model: ModelInfo,
    sys: crate::sysinfo::SysInfo,
    advice: crate::sysinfo::Advice,
}

#[derive(Serialize)]
pub struct ModelInfo {
    id: &'static str,
    label: &'static str,
    purpose: &'static str,
    source: &'static str,
    size: u64,
    sha256: &'static str,
    path: Option<String>,
    /// True when `path` is our own download (so "Hapus" may delete it); a file the user picked is never deleted.
    owned: bool,
    partial: u64,
}

pub fn status(db: &Db) -> Status {
    let path = model_path(db);
    let owned = path.as_ref().is_some_and(|p| p.starts_with(models_dir()));
    let partial = models_dir().join(format!("{}.part", TEXT.file)).metadata().map(|m| m.len()).unwrap_or(0);
    let sys = crate::sysinfo::collect(&models_dir());
    let advice = crate::sysinfo::advise(&sys, TEXT.size, path.is_some());
    Status {
        engine: cfg!(feature = "local-llm"),
        enabled: setting(db, "ai_enabled").as_deref() == Some("1") && path.is_some(),
        busy: BUSY.load(Ordering::Relaxed),
        model: ModelInfo {
            id: TEXT.id, label: TEXT.label, purpose: TEXT.purpose, source: TEXT.source, size: TEXT.size, sha256: TEXT.sha256,
            path: path.map(|p| p.to_string_lossy().into_owned()), owned, partial,
        },
        sys, advice,
    }
}

#[derive(Serialize, Clone)]
struct Progress {
    /// download | verify | done | error | cancelled
    state: &'static str,
    done: u64,
    total: u64,
    msg: String,
}

fn emit(app: &AppHandle, state: &'static str, done: u64, total: u64, msg: impl Into<String>) {
    app.emit("ai-model", Progress { state, done, total, msg: msg.into() }).ok();
}

/// Runs `job` on a background thread unless one is already running.
fn background(app: AppHandle, job: impl FnOnce(&AppHandle) -> Result<String, String> + Send + 'static) -> Result<(), String> {
    if BUSY.swap(true, Ordering::SeqCst) {
        return Err("Sudah ada proses model yang berjalan.".into());
    }
    CANCEL.store(false, Ordering::SeqCst);
    std::thread::spawn(move || {
        match job(&app) {
            Ok(msg) => emit(&app, "done", 0, 0, msg),
            Err(e) if CANCEL.load(Ordering::SeqCst) => emit(&app, "cancelled", 0, 0, e),
            Err(e) => emit(&app, "error", 0, 0, e),
        }
        BUSY.store(false, Ordering::SeqCst);
    });
    Ok(())
}

pub fn cancel() {
    CANCEL.store(true, Ordering::SeqCst);
}

/// Hash `reader` into `hasher`, reporting progress; stops when cancelled.
fn hash_into(app: &AppHandle, state: &'static str, mut reader: impl Read, hasher: &mut Sha256, mut out: Option<&mut std::fs::File>, start: u64, total: u64) -> Result<u64, String> {
    let mut buf = vec![0u8; 1 << 20];
    let mut done = start;
    let mut last = std::time::Instant::now();
    loop {
        if CANCEL.load(Ordering::Relaxed) {
            return Err("Dibatalkan.".into());
        }
        let n = reader.read(&mut buf).map_err(|e| format!("Koneksi/baca terputus: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        if let Some(f) = out.as_deref_mut() {
            f.write_all(&buf[..n]).map_err(|e| format!("Gagal menulis file: {e}"))?;
        }
        done += n as u64;
        if last.elapsed().as_millis() > 250 {
            emit(app, state, done, total, "");
            last = std::time::Instant::now();
        }
    }
    Ok(done)
}

/// Download into models\<file>.part (resuming a previous partial download), verify SHA-256, then rename into place.
pub fn download(app: AppHandle, db: Db) -> Result<(), String> {
    background(app, move |app| {
        let dir = models_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let part = dir.join(format!("{}.part", TEXT.file));
        let dst = dir.join(TEXT.file);
        let mut hasher = Sha256::new();
        let mut have = part.metadata().map(|m| m.len()).unwrap_or(0);
        if have > TEXT.size {
            std::fs::remove_file(&part).ok();
            have = 0;
        }
        if have > 0 {
            emit(app, "verify", 0, have, "Memeriksa bagian yang sudah terunduh…");
            hash_into(app, "verify", std::fs::File::open(&part).map_err(|e| e.to_string())?, &mut hasher, None, 0, have)?;
        }
        let mut req = ureq::get(TEXT.url).set("User-Agent", "file-organizer");
        if have > 0 {
            req = req.set("Range", &format!("bytes={have}-"));
        }
        let resp = req.call().map_err(|e| format!("Unduhan gagal: {e}"))?;
        if have > 0 && resp.status() != 206 {
            have = 0; // server ignored the range: start over
            hasher = Sha256::new();
        }
        let mut file = std::fs::OpenOptions::new().create(true).write(true).append(have > 0).truncate(have == 0).open(&part).map_err(|e| e.to_string())?;
        let done = hash_into(app, "download", resp.into_reader(), &mut hasher, Some(&mut file), have, TEXT.size)?;
        drop(file);
        let got = format!("{:x}", hasher.finalize());
        if done != TEXT.size || got != TEXT.sha256 {
            std::fs::remove_file(&part).ok();
            return Err(format!("File tidak cocok dengan checksum resmi (ukuran {done}, sha256 {}…). File dihapus; coba unduh lagi.", &got[..12]));
        }
        std::fs::rename(&part, &dst).map_err(|e| e.to_string())?;
        set_setting(&db, "ai_model_path", Some(&dst.to_string_lossy()));
        Ok("Model terunduh dan cocok dengan checksum resmi.".into())
    })
}

/// Reuse a copy the user already has on disk, after checking its SHA-256. The file is not copied or moved.
pub fn use_existing(app: AppHandle, db: Db, path: String) -> Result<(), String> {
    background(app, move |app| {
        let p = Path::new(&path);
        let len = p.metadata().map_err(|e| e.to_string())?.len();
        if len != TEXT.size {
            return Err("Ukuran file tidak sama dengan model resmi.".into());
        }
        let mut hasher = Sha256::new();
        hash_into(app, "verify", std::fs::File::open(p).map_err(|e| e.to_string())?, &mut hasher, None, 0, len)?;
        if format!("{:x}", hasher.finalize()) != TEXT.sha256 {
            return Err("Checksum file ini tidak cocok dengan model resmi; tidak dipakai.".into());
        }
        set_setting(&db, "ai_model_path", Some(&path));
        Ok("File model cocok dan siap dipakai (tidak disalin).".into())
    })
}

/// Forget the model; delete the file only if this app downloaded it (and any partial download).
pub fn remove(db: &Db) -> Result<(), String> {
    if BUSY.load(Ordering::Relaxed) {
        return Err("Tunggu proses unduh/cek selesai atau batalkan dulu.".into());
    }
    if let Some(p) = model_path(db).filter(|p| p.starts_with(models_dir())) {
        std::fs::remove_file(p).map_err(|e| e.to_string())?;
    }
    std::fs::remove_file(models_dir().join(format!("{}.part", TEXT.file))).ok();
    set_setting(db, "ai_model_path", None);
    set_setting(db, "ai_enabled", None);
    Ok(())
}
