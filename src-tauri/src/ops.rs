// SPDX-License-Identifier: GPL-3.0-or-later
//! File operations shared by the Explorer-style actions and the tidy plan. Rules: never overwrite (a taken name gets
//! " (1)", " (2)", …), delete only to the Recycle Bin, and log every move of a batch so it can be undone.
use crate::Db;
use rusqlite::params;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// `dir\name`, or `dir\stem (n).ext` when that name is taken.
pub fn free_name(dir: &Path, name: &OsStr) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(name);
    let is_dir_like = p.extension().map(|e| e.len() > 5 || e.to_string_lossy().contains(' ')).unwrap_or(true);
    let (stem, ext) = if is_dir_like {
        (name.to_string_lossy().into_owned(), String::new())
    } else {
        (p.file_stem().unwrap_or(name).to_string_lossy().into_owned(), format!(".{}", p.extension().unwrap().to_string_lossy()))
    };
    (1..).map(|i| dir.join(format!("{stem} ({i}){ext}"))).find(|p| !p.exists()).unwrap()
}

/// A file or folder name typed by the user (one path part).
pub fn valid_part(part: &str) -> Result<(), String> {
    let bad = part.is_empty() || part == "." || part == ".." || part.ends_with(' ') || part.ends_with('.')
        || part.chars().any(|c| "<>:\"/\\|?*".contains(c) || (c as u32) < 32);
    if bad { Err(format!("Nama \"{part}\" tidak valid di Windows.")) } else { Ok(()) }
}

/// A destination typed in the plan, relative to its folder: "Dokumen" or "Dokumen\Pajak".
pub fn valid_target(t: &str) -> Result<(), String> {
    if t.trim().is_empty() {
        return Err("Tujuan kosong.".into());
    }
    t.split(['\\', '/']).try_for_each(valid_part)
}

/// Undo log of one batch (rows in the `moves` table).
pub struct Log<'a> {
    db: &'a Db,
    pub batch: i64,
    seq: i64,
}

impl<'a> Log<'a> {
    pub fn new(db: &'a Db) -> Self {
        let batch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
        Log { db, batch, seq: 0 }
    }
    fn add(&mut self, op: &str, src: &Path, dst: &Path) {
        self.seq += 1;
        self.db.lock().unwrap().execute(
            "INSERT INTO moves(batch, seq, op, src, dst) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![self.batch, self.seq, op, src.to_string_lossy(), dst.to_string_lossy()],
        ).ok();
    }
    pub fn is_empty(&self) -> bool {
        self.seq == 0
    }

    /// Create `dir` and its missing parents, logging each so undo removes them again (if still empty).
    pub fn mkdirs(&mut self, dir: &Path) -> Result<(), String> {
        let missing: Vec<&Path> = dir.ancestors().take_while(|a| !a.exists()).collect();
        for d in missing.into_iter().rev() {
            std::fs::create_dir(d).map_err(|e| format!("Folder {} tidak bisa dibuat: {e}", d.display()))?;
            self.add("mkdir", d, d);
        }
        Ok(())
    }

    /// Move `src` into `dir` (created if needed) without overwriting. Returns the new path.
    pub fn move_into(&mut self, src: &Path, dir: &Path) -> Result<PathBuf, String> {
        if dir.starts_with(src) {
            return Err("Folder tidak bisa dipindah ke dalam dirinya sendiri.".into());
        }
        if src.parent() == Some(dir) {
            return Ok(src.to_path_buf()); // already there
        }
        self.mkdirs(dir)?;
        let to = free_name(dir, src.file_name().ok_or("Nama tidak valid.")?);
        std::fs::rename(src, &to).map_err(|e| match e.raw_os_error() {
            Some(17) => "Beda drive: pindah antar drive belum didukung.".to_string(),
            Some(5) | Some(32) => "Sedang dipakai aplikasi lain atau akses ditolak.".to_string(),
            _ => e.to_string(),
        })?;
        self.add("mv", src, &to);
        Ok(to)
    }

    /// Move everything inside `src` into `dst` (merge), then remove `src` if it ended up empty.
    pub fn merge_into(&mut self, src: &Path, dst: &Path) -> Result<(), String> {
        if !dst.exists() {
            std::fs::rename(src, dst).map_err(|e| e.to_string())?;
            self.add("mv", src, dst);
            return Ok(());
        }
        let mut errors = Vec::new();
        for e in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
            if let Err(err) = self.move_into(&e.path(), dst) {
                errors.push(format!("{}: {err}", e.file_name().to_string_lossy()));
            }
        }
        if std::fs::remove_dir(src).is_ok() {
            self.add("rmdir", src, src);
        }
        if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
    }
}

/// Reverse a batch, newest step first. Returns (steps undone, problems).
pub fn undo(db: &Db, batch: i64) -> (usize, Vec<String>) {
    let ops: Vec<(String, String, String)> = {
        let c = db.lock().unwrap();
        let mut st = c.prepare("SELECT op, src, dst FROM moves WHERE batch = ?1 ORDER BY seq DESC").unwrap();
        let rows = st.query_map([batch], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().flatten().collect();
        rows
    };
    let (mut done, mut problems) = (0, Vec::new());
    for (op, src, dst) in ops {
        let r = match op.as_str() {
            "mv" if Path::new(&src).exists() => Err(format!("{src} sudah ada lagi; tidak ditimpa")),
            "mv" => std::fs::rename(&dst, &src).map_err(|e| format!("{dst}: {e}")),
            "rmdir" => std::fs::create_dir_all(&src).map_err(|e| e.to_string()),
            "mkdir" => { std::fs::remove_dir(&dst).ok(); Ok(()) } // only if still empty
            _ => Ok(()),
        };
        match r { Ok(()) => done += 1, Err(e) => problems.push(e) }
    }
    db.lock().unwrap().execute("DELETE FROM moves WHERE batch = ?1", [batch]).ok();
    (done, problems)
}

pub fn rename(path: &str, new_name: &str) -> Result<String, String> {
    let new_name = new_name.trim();
    valid_part(new_name)?;
    let src = Path::new(path);
    let to = src.parent().ok_or("Tidak bisa mengganti nama ini.")?.join(new_name);
    if to == src {
        return Ok(path.into());
    }
    // Same name in another case is a rename of the same file on Windows; any other existing name is refused.
    if to.exists() && !to.to_string_lossy().eq_ignore_ascii_case(path) {
        return Err(format!("Sudah ada \"{new_name}\" di folder ini."));
    }
    std::fs::rename(src, &to).map_err(|e| e.to_string())?;
    Ok(to.to_string_lossy().into_owned())
}

pub fn new_folder(parent: &str) -> Result<String, String> {
    let to = free_name(Path::new(parent), OsStr::new("Folder baru"));
    std::fs::create_dir(&to).map_err(|e| e.to_string())?;
    Ok(to.to_string_lossy().into_owned())
}

pub fn to_recycle_bin(paths: &[String]) -> Result<usize, String> {
    trash::delete_all(paths).map_err(|e| format!("Tidak bisa dipindah ke Recycle Bin: {e}"))?;
    Ok(paths.len())
}

fn copy_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir(dst)?;
        for e in std::fs::read_dir(src)?.flatten() {
            copy_all(&e.path(), &dst.join(e.file_name()))?;
        }
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

/// Paste (Ctrl+V) `paths` into `dest`: cut = move (undoable batch), copy = copy. Never overwrites.
pub fn paste(db: &Db, paths: &[String], dest: &str, cut: bool) -> (Option<i64>, usize, Vec<String>) {
    let dest = Path::new(dest);
    let mut log = Log::new(db);
    let (mut done, mut failed) = (0, Vec::new());
    for p in paths {
        let src = Path::new(p);
        let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let r = if cut {
            log.move_into(src, dest).map(|_| ())
        } else if dest.starts_with(src) {
            Err("Folder tidak bisa disalin ke dalam dirinya sendiri.".into())
        } else {
            copy_all(src, &free_name(dest, src.file_name().unwrap_or_default())).map_err(|e| e.to_string())
        };
        match r { Ok(()) => done += 1, Err(e) => failed.push(format!("{name}: {e}")) }
    }
    ((!log.is_empty()).then_some(log.batch), done, failed)
}
