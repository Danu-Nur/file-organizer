// SPDX-License-Identifier: GPL-3.0-or-later
//! "Rapikan folder": analyse the folders the user ticked into an editable plan, then execute the approved rows as one
//! undoable batch. Each ticked folder is tidied at its own level (its files and direct subfolders); nothing deeper
//! moves unless that subfolder was ticked too.
use crate::{ops, rules, Db};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct Item {
    src: String,
    base: String,
    name: String,
    is_dir: bool,
    size: i64,
    mtime: i64,
    /// pindah | duplikat | versi | sensitif | curiga | kosong | proyek | program | ganti-nama | biarkan
    kind: &'static str,
    /// Relative to `base`; "" = stays.
    target: String,
    reason: String,
    checked: bool,
    md5: Option<String>,
}

struct FileInfo {
    path: PathBuf,
    base: PathBuf,
    name: String,
    size: i64,
    mtime: i64,
    md5: Option<String>,
}

fn secs(m: &std::fs::Metadata) -> i64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(windows)]
pub fn hidden(m: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    m.file_attributes() & 0x6 != 0 // HIDDEN | SYSTEM
}
#[cfg(not(windows))]
pub fn hidden(_: &std::fs::Metadata) -> bool {
    false
}

/// md5 from the index when size+mtime still match, else read the file.
fn md5_of(db: &Db, p: &Path, size: i64, mtime: i64) -> Option<String> {
    let cached: Option<String> = db.lock().unwrap()
        .query_row("SELECT md5 FROM files WHERE path = ?1 AND size = ?2 AND mtime = ?3", params![p.to_string_lossy(), size, mtime], |r| r.get(0))
        .optional().ok().flatten().flatten();
    cached.or_else(|| crate::md5_file(p).ok())
}

/// Webshell-looking PHP, or an "image" that is really PHP.
fn suspicious(p: &Path) -> bool {
    use std::io::Read;
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let is_php = ["php", "phtml", "php5", "php7", "inc"].contains(&ext.as_str());
    let is_img = ["png", "jpg", "jpeg", "gif", "webp", "ico", "svg"].contains(&ext.as_str());
    if !is_php && !is_img {
        return false;
    }
    let mut buf = Vec::new();
    let limit = if is_php { 1 << 20 } else { 4096 };
    if std::fs::File::open(p).and_then(|f| f.take(limit).read_to_end(&mut buf)).is_err() {
        return false;
    }
    let s = String::from_utf8_lossy(&buf).to_lowercase();
    if is_img {
        return s.contains("<?php");
    }
    let hits: usize = ["eval(", "base64_decode", "gzinflate", "str_rot13", "shell_exec", "passthru", "system(", "assert(", "proc_open", "$_request", "$_files"]
        .iter().map(|k| s.matches(k).count()).sum();
    hits >= 12
}

/// Category counts of a folder's content (bounded walk).
fn look_inside(dir: &Path) -> (HashMap<&'static str, usize>, usize, i64, bool) {
    let (mut counts, mut total, mut bytes, mut exe_on_top) = (HashMap::new(), 0usize, 0i64, false);
    let mut stack = vec![(dir.to_path_buf(), 0)];
    while let Some((d, depth)) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            if total >= 5000 {
                break;
            }
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() && !ft.is_symlink() && depth < 8 && !rules::skip_dir(&e.path()) {
                stack.push((e.path(), depth + 1));
            } else if ft.is_file() {
                let name = e.file_name().to_string_lossy().into_owned();
                if rules::ignore_file(&name) {
                    continue;
                }
                total += 1;
                bytes += e.metadata().map(|m| m.len() as i64).unwrap_or(0);
                exe_on_top |= depth == 0 && name.to_lowercase().ends_with(".exe"); // an app folder, not a folder that holds installers somewhere
                *counts.entry(rules::category(&name)).or_insert(0) += 1;
            }
        }
    }
    (counts, total, bytes, exe_on_top)
}

/// `home`: the category a base already is (Pictures → Gambar). `downloads`: download tools may keep writing to
/// English category folders there, so renaming those gets a note.
pub fn analyze(db: &Db, root: &Path, folders: &[PathBuf], home_of: impl Fn(&Path) -> Option<&'static str>, downloads: Option<&Path>) -> Vec<Item> {
    let mut items = Vec::new();
    let mut files: Vec<FileInfo> = Vec::new();
    // per base: lowercase subfolder name → real name, so "dokumen" reuses an existing "Dokumen"
    let mut existing: HashMap<PathBuf, HashMap<String, String>> = HashMap::new();
    let mut category_bases: HashSet<PathBuf> = HashSet::new();

    for base in folders {
        let base_lc = base.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        let project_container = base_lc == "proyek" || base_lc == "projects";
        // A category folder (Dokumen, old "Documents", _Duplikat, …) is already home for its files: inside it only
        // duplicates / empty / secrets / old versions are flagged, nothing is re-sorted into sub-sub-folders.
        let cat_folder = base.file_name().and_then(|n| rules::folder_category(&n.to_string_lossy()));
        let home = home_of(base).or(cat_folder);
        if cat_folder.is_some() {
            category_bases.insert(base.clone());
        }
        let entries: Vec<_> = std::fs::read_dir(base).into_iter().flatten().flatten().collect();
        let names = existing.entry(base.clone()).or_default();
        for e in &entries {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let n = e.file_name().to_string_lossy().into_owned();
                names.insert(n.to_lowercase(), n);
            }
        }
        for e in entries {
            let (Ok(ft), Ok(meta)) = (e.file_type(), e.metadata()) else { continue };
            let name = e.file_name().to_string_lossy().into_owned();
            if ft.is_symlink() || hidden(&meta) || name.starts_with('.') {
                continue;
            }
            let path = e.path();
            let mtime = secs(&meta);
            if ft.is_dir() {
                let mk = |kind, target: &str, reason: String, checked, size| Item {
                    src: path.to_string_lossy().into_owned(), base: base.to_string_lossy().into_owned(), name: name.clone(),
                    is_dir: true, size, mtime, kind, target: target.into(), reason, checked, md5: None,
                };
                if let Some(id) = rules::legacy_to_id(&name) {
                    let in_downloads = downloads == Some(base.as_path());
                    let note = if in_downloads { " — jika ada program yang otomatis menyimpan unduhan ke folder ini, sesuaikan juga pengaturannya" } else { "" };
                    items.push(mk("ganti-nama", id, format!("nama Inggris → Indonesia{note}"), true, 0));
                    continue;
                }
                if rules::is_category_folder(&name) || project_container {
                    continue;
                }
                if rules::is_project(&path) {
                    items.push(mk("proyek", "Proyek", "folder project kode — disimpan utuh".into(), true, 0));
                    continue;
                }
                let (counts, total, bytes, exe) = look_inside(&path);
                if total == 0 {
                    items.push(mk("kosong", "_Kosong", "folder kosong".into(), true, 0));
                } else if exe {
                    items.push(mk("program", "Program", "berisi aplikasi (.exe)".into(), true, bytes));
                } else {
                    let (cat, n) = counts.iter().max_by_key(|(_, n)| **n).map(|(c, n)| (*c, *n)).unwrap();
                    let pct = n * 100 / total;
                    if pct >= 60 {
                        if Some(cat) != home {
                            items.push(mk("pindah", cat, format!("{pct}% dari {total} file berjenis {cat}"), true, bytes));
                        } // else: already in the right place
                    } else {
                        items.push(mk("biarkan", "", format!("isi campuran ({total} file)"), false, bytes));
                    }
                }
            } else if !rules::ignore_file(&name) {
                let size = meta.len() as i64;
                files.push(FileInfo { md5: md5_of(db, &path, size, mtime), path, base: base.clone(), name, size, mtime });
            }
        }
    }

    // Duplicates (same md5): keep one copy — one outside the tidied folders if there is one, else the shortest name.
    let scoped: HashSet<String> = files.iter().map(|f| f.path.to_string_lossy().into_owned()).collect();
    let mut by_md5: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, f) in files.iter().enumerate() {
        if let (Some(m), true) = (f.md5.as_deref(), f.size > 0) {
            by_md5.entry(m).or_default().push(i);
        }
    }
    let rel = |p: &str| p.strip_prefix(&*root.to_string_lossy()).map(|s| s.trim_start_matches('\\').to_string()).unwrap_or(p.to_string());
    let root_prefix = format!("{}\\", root.to_string_lossy());
    let mut dup_of: HashMap<usize, String> = HashMap::new();
    for (md5, idx) in &by_md5 {
        let outside: Option<String> = {
            let c = db.lock().unwrap();
            let mut st = c.prepare("SELECT path FROM files WHERE md5 = ?1 AND substr(path, 1, length(?2)) = ?2 ORDER BY length(path)").unwrap();
            let found = st.query_map(params![md5, root_prefix], |r| r.get::<_, String>(0)).unwrap().flatten()
                .find(|p| !scoped.contains(p) && Path::new(p).exists());
            found
        };
        if let Some(o) = outside {
            for &i in idx {
                dup_of.insert(i, rel(&o));
            }
        } else if idx.len() > 1 {
            let keep = *idx.iter().min_by_key(|&&i| (files[i].name.len(), files[i].mtime)).unwrap();
            for &i in idx.iter().filter(|&&i| i != keep) {
                dup_of.insert(i, rel(&files[keep].path.to_string_lossy()));
            }
        }
    }

    // Versions: "x.md" + "x (1).md" with different content → keep the newest, older ones go to review (unticked).
    let mut groups: HashMap<(PathBuf, String), Vec<usize>> = HashMap::new();
    for (i, f) in files.iter().enumerate() {
        if !dup_of.contains_key(&i) && f.size > 0 {
            groups.entry((f.base.clone(), rules::base_name(&f.name))).or_default().push(i);
        }
    }
    let mut older_than: HashMap<usize, String> = HashMap::new();
    for idx in groups.values().filter(|g| g.len() > 1) {
        let newest = *idx.iter().max_by_key(|&&i| files[i].mtime).unwrap();
        for &i in idx.iter().filter(|&&i| i != newest) {
            older_than.insert(i, files[newest].name.clone());
        }
    }

    for (i, f) in files.iter().enumerate() {
        let cat = rules::category(&f.name);
        let (kind, target, reason, checked): (&'static str, &str, String, bool) = if f.size == 0 {
            ("kosong", "_Kosong", "file kosong (0 byte)".into(), true)
        } else if suspicious(&f.path) {
            ("curiga", "_Karantina", "berisi pola kode berbahaya (kemungkinan webshell)".into(), true)
        } else if let Some(k) = dup_of.get(&i) {
            ("duplikat", "_Duplikat", format!("isi sama persis dengan {k}"), true)
        } else if cat == "_Sensitif" {
            ("sensitif", "_Sensitif", "nama mengandung data rahasia".into(), true)
        } else if let Some(n) = older_than.get(&i) {
            ("versi", "_Versi Lama", format!("versi lebih lama dari \"{n}\" (isinya berbeda) — cek dulu"), false)
        } else if Some(cat) == home_of(&f.base) || category_bases.contains(&f.base) {
            continue; // already in the right place
        } else {
            let stem = f.path.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            let extracted = cat == "Arsip" && existing.get(&f.base).is_some_and(|n| n.contains_key(&stem));
            ("pindah", cat, if extracted { format!("jenis file — sudah diekstrak ke folder \"{stem}\"") } else { "jenis file".into() }, true)
        };
        let target = existing.get(&f.base).and_then(|n| n.get(&target.to_lowercase())).cloned().unwrap_or_else(|| target.to_string());
        items.push(Item {
            src: f.path.to_string_lossy().into_owned(), base: f.base.to_string_lossy().into_owned(), name: f.name.clone(),
            is_dir: false, size: f.size, mtime: f.mtime, kind, target, reason, checked, md5: f.md5.clone(),
        });
    }
    items
}

#[derive(Deserialize)]
pub struct Exec {
    src: String,
    base: String,
    target: String,
    kind: String,
}

#[derive(Serialize)]
pub struct Outcome {
    batch: Option<i64>,
    done: usize,
    failed: Vec<String>,
}

pub fn execute(db: &Db, mut items: Vec<Exec>) -> Outcome {
    // Deepest first (a ticked subfolder is tidied before it may itself move); per level: renames, files, folders.
    let rank = |e: &Exec| if e.kind == "ganti-nama" { 0 } else if Path::new(&e.src).is_dir() { 2 } else { 1 };
    items.sort_by_key(|e| (std::cmp::Reverse(Path::new(&e.src).components().count()), rank(e)));
    let mut log = ops::Log::new(db);
    let (mut done, mut failed) = (0, Vec::new());
    for e in &items {
        let src = Path::new(&e.src);
        let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let r = (|| {
            ops::valid_target(&e.target)?;
            if !src.exists() {
                return Err("sudah tidak ada".to_string());
            }
            if src.parent() != Some(Path::new(&e.base)) {
                return Err("lokasinya berubah sejak dianalisis".into());
            }
            let dst = Path::new(&e.base).join(e.target.replace('/', "\\"));
            if e.kind == "ganti-nama" { log.merge_into(src, &dst) } else { log.move_into(src, &dst).map(|_| ()) }
        })();
        match r { Ok(()) => done += 1, Err(err) => failed.push(format!("{name}: {err}")) }
    }
    Outcome { batch: (!log.is_empty()).then_some(log.batch), done, failed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Arc, Mutex};

    /// analyse → execute everything ticked → undo, on a throwaway folder.
    #[test]
    fn plan_execute_undo() {
        let t = std::env::temp_dir().join(format!("fo-plan-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&t);
        let w = |p: &str, s: &str| { let p = t.join(p); fs::create_dir_all(p.parent().unwrap()).unwrap(); fs::write(p, s).unwrap(); };
        w("a.pdf", "same");
        w("a (1).pdf", "same");
        w("b.png", "img");
        w("c.md", "old");
        w("c (1).md", "new!");
        fs::File::options().write(true).open(t.join("c.md")).unwrap()
            .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3600)).unwrap();
        w("empty.txt", "");
        w("github-recovery-codes.txt", "x");
        w("Documents/d.txt", "d");
        w("foto/1.png", "1"); w("foto/2.jpg", "2"); w("foto/3.png", "3");
        w("proj/package.json", "{}");
        fs::create_dir_all(t.join("kosong")).unwrap();
        let before = layout(&t);

        let db: Db = Arc::new(Mutex::new(rusqlite::Connection::open_in_memory().unwrap()));
        db.lock().unwrap().execute_batch(crate::SCHEMA).unwrap();
        w("Documents/e.xlsx", "e");
        let inner = analyze(&db, &t, &[t.join("Documents")], |_| None, None);
        assert!(inner.is_empty(), "a category folder is home for its files: {:?}", inner.iter().map(|i| &i.name).collect::<Vec<_>>());
        fs::remove_file(t.join("Documents/e.xlsx")).unwrap();
        let items = analyze(&db, &t, &[t.clone()], |_| None, None);
        let kind = |n: &str| items.iter().find(|i| i.name == n).map(|i| (i.kind, i.target.as_str())).unwrap_or(("-", ""));
        assert_eq!(kind("a (1).pdf"), ("duplikat", "_Duplikat"));
        assert_eq!(kind("a.pdf"), ("pindah", "Dokumen"));
        assert_eq!(kind("b.png"), ("pindah", "Gambar"));
        assert_eq!(kind("c.md"), ("versi", "_Versi Lama"));
        assert_eq!(kind("c (1).md"), ("pindah", "Dokumen"));
        assert_eq!(kind("empty.txt"), ("kosong", "_Kosong"));
        assert_eq!(kind("github-recovery-codes.txt"), ("sensitif", "_Sensitif"));
        assert_eq!(kind("Documents"), ("ganti-nama", "Dokumen"));
        assert_eq!(kind("foto"), ("pindah", "Gambar"));
        assert_eq!(kind("proj"), ("proyek", "Proyek"));
        assert_eq!(kind("kosong"), ("kosong", "_Kosong"));

        let exec: Vec<Exec> = items.iter().filter(|i| i.checked)
            .map(|i| Exec { src: i.src.clone(), base: i.base.clone(), target: i.target.clone(), kind: i.kind.into() }).collect();
        let out = execute(&db, exec);
        assert!(out.failed.is_empty(), "{:?}", out.failed);
        for p in ["Dokumen/a.pdf", "Dokumen/d.txt", "Dokumen/c (1).md", "Gambar/b.png", "Gambar/foto/1.png", "_Duplikat/a (1).pdf", "Proyek/proj/package.json", "_Kosong/kosong", "c.md"] {
            assert!(t.join(p).exists(), "missing {p}");
        }
        assert!(!t.join("Documents").exists());

        let (_, problems) = ops::undo(&db, out.batch.unwrap());
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(layout(&t), before);
        fs::remove_dir_all(&t).unwrap();
    }

    fn layout(dir: &Path) -> Vec<String> {
        let mut v = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d).unwrap().flatten() {
                v.push(e.path().strip_prefix(dir).unwrap().to_string_lossy().into_owned());
                if e.path().is_dir() { stack.push(e.path()); }
            }
        }
        v.sort();
        v
    }
}
