// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! File Organizer: watches the user's six Explorer folders, keeps an index (name, path, size, md5 — never the content)
//! and suggests where loose files belong. Moving only happens when the user clicks.
mod models;
mod ops;
mod plan;
mod rules;
mod sysinfo;
#[cfg(feature = "local-llm")]
mod ai;

use notify::event::ModifyKind;
use notify::{EventKind, RecursiveMode, Watcher};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

type Db = Arc<Mutex<Connection>>;
/// (Explorer name, path, the category that already lives there)
type Root = (&'static str, PathBuf, Option<&'static str>);

struct AppState {
    db: Db,
    roots: Vec<Root>,
    scanning: Arc<AtomicBool>,
}

fn known_roots() -> Vec<Root> {
    [
        ("Desktop", dirs::desktop_dir(), None),
        ("Downloads", dirs::download_dir(), None),
        ("Documents", dirs::document_dir(), Some("Dokumen")),
        ("Pictures", dirs::picture_dir(), Some("Gambar")),
        ("Music", dirs::audio_dir(), Some("Musik")),
        ("Videos", dirs::video_dir(), Some("Video")),
    ]
    .into_iter()
    .filter_map(|(n, p, home)| p.filter(|p| p.is_dir()).map(|p| (n, p, home)))
    .collect()
}

fn open_db() -> Connection {
    let dir = dirs::data_local_dir().expect("LocalAppData").join("file-organizer");
    std::fs::create_dir_all(&dir).ok();
    let c = Connection::open(dir.join("index.db")).expect("open index.db");
    c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;").ok();
    c.execute_batch(SCHEMA).expect("create schema");
    c
}

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS files(
      path TEXT PRIMARY KEY, name TEXT NOT NULL, size INTEGER NOT NULL, mtime INTEGER NOT NULL, md5 TEXT,
      suggest TEXT, reason TEXT, dismissed INTEGER NOT NULL DEFAULT 0, seen INTEGER NOT NULL DEFAULT 0);
    CREATE INDEX IF NOT EXISTS files_md5 ON files(md5);
    CREATE TABLE IF NOT EXISTS moves(batch INTEGER NOT NULL, seq INTEGER NOT NULL, op TEXT NOT NULL, src TEXT NOT NULL, dst TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);";

fn secs(t: std::io::Result<SystemTime>) -> i64 {
    t.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn md5_file(p: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(p)?;
    let mut ctx = md5::Context::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        ctx.consume(&buf[..n]);
    }
    Ok(format!("{:x}", ctx.compute()))
}

// ---------------------------------------------------------------- indexer (one background thread)

struct Indexer {
    db: Db,
    roots: Vec<Root>,
    app: AppHandle,
    gen: i64,
    skip_cache: HashMap<PathBuf, bool>,
    scanning: Arc<AtomicBool>,
}

impl Indexer {
    fn root_of(&self, p: &Path) -> Option<(PathBuf, Option<&'static str>)> {
        self.roots.iter().find(|(_, r, _)| p.starts_with(r)).map(|(_, r, h)| (r.clone(), *h))
    }

    fn skip_dir(&mut self, d: &Path) -> bool {
        if let Some(&b) = self.skip_cache.get(d) {
            return b;
        }
        let b = rules::skip_dir(d);
        self.skip_cache.insert(d.to_path_buf(), b);
        b
    }

    /// True when any folder between the root and `p` is skipped (hidden, node_modules, a code project, …).
    fn inside_skipped(&mut self, p: &Path, root: &Path) -> bool {
        let mut cur = p.parent();
        while let Some(d) = cur {
            if d == root || !d.starts_with(root) {
                return false;
            }
            if self.skip_dir(d) {
                return true;
            }
            cur = d.parent();
        }
        false
    }

    fn walk(&mut self, dir: &Path, root: &Path, home: Option<&'static str>, live: bool) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            if ft.is_symlink() {
                continue; // also junctions like "My Music"
            }
            if ft.is_dir() {
                if !self.skip_dir(&p) {
                    self.walk(&p, root, home, live);
                }
            } else {
                self.file(&p, root, home, live);
            }
        }
    }

    fn forget(&self, p: &Path) {
        let key = p.to_string_lossy().into_owned();
        let prefix = format!("{key}\\");
        self.db.lock().unwrap()
            .execute("DELETE FROM files WHERE path = ?1 OR substr(path, 1, length(?2)) = ?2", params![key, prefix])
            .ok();
    }

    fn process(&mut self, p: &Path, walk_dirs: bool) {
        let Some((root, home)) = self.root_of(p) else { return };
        if !p.exists() {
            return self.forget(p);
        }
        if p == root || self.inside_skipped(p, &root) {
            return;
        }
        if p.is_dir() {
            if walk_dirs && !self.skip_dir(p) {
                self.walk(p, &root, home, true); // a folder moved/extracted in
            }
        } else {
            self.file(p, &root, home, true);
        }
    }

    fn file(&mut self, p: &Path, root: &Path, home: Option<&'static str>, live: bool) {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if rules::ignore_file(&name) {
            return;
        }
        let Ok(meta) = p.metadata() else { return };
        let (size, mtime) = (meta.len() as i64, secs(meta.modified()));
        let key = p.to_string_lossy().into_owned();
        type Old = (i64, i64, Option<String>, Option<String>, Option<String>);
        let old: Option<Old> = self.db.lock().unwrap()
            .query_row("SELECT size, mtime, md5, suggest, reason FROM files WHERE path = ?1", [&key], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .optional()
            .unwrap_or(None);
        // Unchanged file → reuse its md5 (no re-read); the suggestion is still recomputed so rule updates apply.
        let unchanged = matches!(&old, Some((s, t, Some(_), _, _)) if *s == size && *t == mtime);
        let md5 = match &old {
            Some((_, _, Some(m), _, _)) if unchanged => m.clone(),
            // Locked or still being written → skip; the next change event for this file retries.
            _ => match md5_file(p) { Ok(m) => m, Err(_) => return },
        };

        let (mut suggest, mut reason) = (None::<String>, None::<String>);
        if p.parent() == Some(root) {
            let dup: Option<String> = self.db.lock().unwrap()
                .query_row("SELECT path FROM files WHERE md5 = ?1 AND size > 0 AND path <> ?2 LIMIT 1", params![md5, key], |r| r.get(0))
                .optional()
                .unwrap_or(None);
            if let Some(d) = dup {
                suggest = Some("_Duplikat".into());
                reason = Some(format!("Isi sama persis dengan {d}"));
            } else if let Some(c) = rules::suggest(&name, home) {
                suggest = Some(c.into());
                reason = Some(if c == "_Sensitive" { "nama mengandung data rahasia" } else { "jenis file" }.into());
            }
        }
        // Keep an earlier AI pick (Dokumen\<topic>) for an unchanged file instead of asking again.
        if unchanged && suggest.as_deref() == Some("Dokumen") {
            if let Some((_, _, _, Some(s), r)) = &old {
                if s.starts_with("Dokumen\\") {
                    (suggest, reason) = (Some(s.clone()), r.clone());
                }
            }
        }
        self.db.lock().unwrap().execute(
            "INSERT INTO files(path, name, size, mtime, md5, suggest, reason, seen) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(path) DO UPDATE SET name = excluded.name, size = excluded.size, mtime = excluded.mtime,
               md5 = excluded.md5, suggest = excluded.suggest, reason = excluded.reason, seen = excluded.seen",
            params![key, name, size, mtime, md5, suggest, reason, self.gen],
        ).ok();

        if live && old.is_none() {
            if let Some(s) = &suggest {
                self.app.notification().builder().title("File baru").body(format!("{name}\nSaran: pindah ke {s}")).show().ok();
            }
        }
        #[cfg(feature = "local-llm")]
        if !unchanged && suggest.as_deref() == Some("Dokumen") {
            if let Some(model) = models::ai_ready(&self.db) {
                self.ask_ai(p, root, &name, model);
            }
        }
    }

    /// Ambiguous documents: let the local model pick one of the topic folders that already exist under Dokumen.
    /// Only when the user switched AI on and the model is set up (Pengaturan).
    #[cfg(feature = "local-llm")]
    fn ask_ai(&self, path: &Path, root: &Path, name: &str, model: PathBuf) {
        let dir = [root.join("Dokumen"), root.join("Documents")].into_iter().find(|d| d.is_dir()).unwrap_or_default();
        let subs: Vec<String> = std::fs::read_dir(dir).into_iter().flatten().flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        if subs.is_empty() {
            return;
        }
        let (db, app, path, name) = (self.db.clone(), self.app.clone(), path.to_path_buf(), name.to_string());
        std::thread::spawn(move || {
            let Ok((folder, why)) = ai::pick_folder(&model, &name, &path, &subs) else { return };
            if subs.contains(&folder) {
                db.lock().unwrap().execute(
                    "UPDATE files SET suggest = ?2, reason = ?3 WHERE path = ?1 AND suggest = 'Dokumen'",
                    params![path.to_string_lossy(), format!("Dokumen\\{folder}"), format!("AI: {why}")],
                ).ok();
                app.emit("changed", ()).ok();
            }
        });
    }

    /// Startup scan (own thread): loose files in each root first, so suggestions show up right away; then the deep walk.
    fn scan(mut self) {
        for (_, r, home) in self.roots.clone() {
            for e in std::fs::read_dir(&r).into_iter().flatten().flatten() {
                if e.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    self.file(&e.path(), &r, home, false);
                }
            }
        }
        self.app.emit("changed", ()).ok();
        for (_, r, home) in self.roots.clone() {
            self.walk(&r, &r, home, false);
            self.app.emit("changed", ()).ok();
        }
        self.db.lock().unwrap().execute("DELETE FROM files WHERE seen < ?1", [self.gen]).ok();
        self.scanning.store(false, Ordering::Relaxed);
        self.app.emit("changed", ()).ok();
    }

    /// Live changes (own thread, runs from the start — new files are not stuck behind the first scan).
    fn watch(mut self, rx: mpsc::Receiver<(PathBuf, bool)>) {
        // Debounce: a download fires many events; handle a path once it has been quiet for 2 s.
        let mut pending: HashMap<PathBuf, (Instant, bool)> = HashMap::new();
        loop {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok((p, walk)) => {
                    let e = pending.entry(p).or_insert((Instant::now(), false));
                    e.0 = Instant::now();
                    e.1 |= walk;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
            let ready: Vec<(PathBuf, bool)> = pending.iter()
                .filter(|(_, (t, _))| t.elapsed() > Duration::from_secs(2))
                .map(|(p, (_, w))| (p.clone(), *w))
                .collect();
            if ready.is_empty() {
                continue;
            }
            for (p, walk) in ready {
                pending.remove(&p);
                self.process(&p, walk);
            }
            self.app.emit("changed", ()).ok();
        }
    }
}

// ---------------------------------------------------------------- commands

#[derive(Serialize)]
struct Row {
    path: String,
    name: String,
    size: i64,
    mtime: i64,
    md5: Option<String>,
    suggest: Option<String>,
    reason: Option<String>,
    cat: &'static str,
}

#[derive(Serialize)]
struct RootInfo {
    name: &'static str,
    path: String,
}

const DUPS: &str = "md5 IN (SELECT md5 FROM files WHERE size > 0 AND md5 IS NOT NULL GROUP BY md5 HAVING COUNT(*) > 1)";

#[tauri::command]
fn list_roots(st: State<AppState>) -> Vec<RootInfo> {
    st.roots.iter().map(|(n, p, _)| RootInfo { name: n, path: p.to_string_lossy().into_owned() }).collect()
}

/// `view`: "suggest" | "dup" | "all" | a root path. `sort`: name | mtime | size | path | md5.
#[tauri::command]
fn list(st: State<AppState>, q: String, view: String, sort: String, desc: bool) -> Result<Vec<Row>, String> {
    let filter = match view.as_str() {
        "suggest" => "suggest IS NOT NULL AND dismissed = 0".to_string(),
        "dup" => DUPS.to_string(),
        "all" => "1".to_string(),
        _ => "substr(path, 1, length(?2) + 1) = ?2 || '\\'".to_string(),
    };
    let col = match sort.as_str() { "name" => "name COLLATE NOCASE", "size" => "size", "path" => "path COLLATE NOCASE", "md5" => "md5", _ => "mtime" };
    let order = if view == "dup" { format!("md5, {col} {}", if desc { "DESC" } else { "ASC" }) } else { format!("{col} {}", if desc { "DESC" } else { "ASC" }) };
    let like = format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let sql = format!(
        "SELECT path, name, size, mtime, md5, CASE WHEN dismissed = 0 THEN suggest END, reason FROM files
         WHERE ({filter}) AND (name LIKE ?1 ESCAPE '\\' OR md5 LIKE ?1 ESCAPE '\\') ORDER BY {order} LIMIT 2000"
    );
    let db = st.db.lock().unwrap();
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let map = |r: &rusqlite::Row| {
        let name: String = r.get(1)?;
        Ok(Row { cat: rules::category(&name), path: r.get(0)?, name, size: r.get(2)?, mtime: r.get(3)?, md5: r.get(4)?, suggest: r.get(5)?, reason: r.get(6)? })
    };
    let rows = if filter.contains("?2") { stmt.query_map(params![like, view], map) } else { stmt.query_map(params![like], map) };
    let out = rows.map_err(|e| e.to_string())?.collect::<Result<_, _>>().map_err(|e| e.to_string());
    out
}

#[derive(Serialize)]
struct Counts {
    all: i64,
    suggest: i64,
    dup: i64,
    scanning: bool,
}

#[tauri::command]
fn counts(st: State<AppState>) -> Result<Counts, String> {
    let scanning = st.scanning.load(Ordering::Relaxed);
    st.db.lock().unwrap().query_row(
        &format!("SELECT COUNT(*), COALESCE(SUM(suggest IS NOT NULL AND dismissed = 0), 0), (SELECT COUNT(*) FROM files WHERE {DUPS}) FROM files"),
        [],
        |r| Ok(Counts { all: r.get(0)?, suggest: r.get(1)?, dup: r.get(2)?, scanning }),
    ).map_err(|e| e.to_string())
}

/// Move a loose file into its suggested subfolder. Never overwrites: a taken name gets " (1)", " (2)", …
#[tauri::command]
fn apply(st: State<AppState>, path: String) -> Result<Moved, String> {
    let src = PathBuf::from(&path);
    let (_, root, _) = st.roots.iter().find(|(_, r, _)| src.parent() == Some(r.as_path())).ok_or("File ini tidak lagi berada di folder utama.")?;
    let sug: Option<String> = st.db.lock().unwrap()
        .query_row("SELECT suggest FROM files WHERE path = ?1 AND dismissed = 0", [&path], |r| r.get(0))
        .optional().map_err(|e| e.to_string())?.flatten();
    let sug = sug.ok_or("Tidak ada saran untuk file ini.")?;
    // Until the user renames the old English folders (Rapikan folder), "Dokumen" still goes into an existing "Documents".
    let mut dir = root.join(&sug);
    if !dir.exists() {
        let mut parts = sug.splitn(2, '\\');
        if let Some(en) = parts.next().and_then(rules::id_to_legacy).map(|en| root.join(en)).filter(|d| d.is_dir()) {
            dir = parts.next().map(|rest| en.join(rest)).unwrap_or(en);
        }
    }
    let mut log = ops::Log::new(&st.db);
    let to = log.move_into(&src, &dir)?;
    st.db.lock().unwrap().execute("DELETE FROM files WHERE path = ?1", [&path]).ok(); // the watcher indexes the new path
    Ok(Moved { path: to.to_string_lossy().into_owned(), batch: log.batch })
}

#[derive(Serialize)]
struct Moved {
    path: String,
    batch: i64,
}

// ---------------------------------------------------------------- Explorer-style browsing and actions

#[derive(Serialize)]
struct Entry {
    path: String,
    name: String,
    is_dir: bool,
    size: i64,
    mtime: i64,
    md5: Option<String>,
    suggest: Option<String>,
    reason: Option<String>,
    cat: &'static str,
    has_children: bool,
}

/// One folder's content straight from disk (hidden/system items left out, like Explorer's default), joined with the
/// index for md5 and suggestions.
#[tauri::command(async)]
fn browse(st: State<'_, AppState>, path: String) -> Result<Vec<Entry>, String> {
    let rd = std::fs::read_dir(&path).map_err(|e| format!("Folder tidak bisa dibuka: {e}"))?;
    let mut out = Vec::new();
    for e in rd.flatten() {
        let (Ok(ft), Ok(meta)) = (e.file_type(), e.metadata()) else { continue };
        if plan::hidden(&meta) {
            continue;
        }
        let p = e.path();
        let key = p.to_string_lossy().into_owned();
        let name = e.file_name().to_string_lossy().into_owned();
        let is_dir = ft.is_dir() || (ft.is_symlink() && p.is_dir());
        let (md5, suggest, reason) = if is_dir {
            (None, None, None)
        } else {
            st.db.lock().unwrap()
                .query_row("SELECT md5, CASE WHEN dismissed = 0 THEN suggest END, reason FROM files WHERE path = ?1", [&key], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .optional().ok().flatten().unwrap_or((None, None, None))
        };
        out.push(Entry {
            has_children: is_dir && std::fs::read_dir(&p).map(|mut r| r.next().is_some()).unwrap_or(false),
            cat: if is_dir { "Folder" } else { rules::category(&name) },
            size: if is_dir { 0 } else { meta.len() as i64 },
            mtime: secs(meta.modified()),
            path: key, name, is_dir, md5, suggest, reason,
        });
    }
    Ok(out)
}

#[tauri::command(async)]
fn rename_item(path: String, name: String) -> Result<String, String> {
    ops::rename(&path, &name)
}

#[tauri::command(async)]
fn trash_items(paths: Vec<String>) -> Result<usize, String> {
    ops::to_recycle_bin(&paths)
}

#[tauri::command(async)]
fn new_folder(parent: String) -> Result<String, String> {
    ops::new_folder(&parent)
}

#[derive(Serialize)]
struct Pasted {
    batch: Option<i64>,
    done: usize,
    failed: Vec<String>,
}

#[tauri::command(async)]
fn paste_items(st: State<'_, AppState>, paths: Vec<String>, dest: String, cut: bool) -> Pasted {
    let (batch, done, failed) = ops::paste(&st.db, &paths, &dest, cut);
    Pasted { batch, done, failed }
}

#[tauri::command(async)]
fn undo_batch(st: State<'_, AppState>, batch: i64) -> Result<usize, String> {
    let (done, problems) = ops::undo(&st.db, batch);
    if problems.is_empty() { Ok(done) } else { Err(problems.join("\n")) }
}

// ---------------------------------------------------------------- Rapikan folder (plan → review → execute)

#[tauri::command(async)]
fn analyze_folders(st: State<'_, AppState>, root: String, folders: Vec<String>) -> Vec<plan::Item> {
    let homes: Vec<(PathBuf, Option<&'static str>)> = st.roots.iter().map(|(_, p, h)| (p.clone(), *h)).collect();
    let downloads = st.roots.iter().find(|(n, _, _)| *n == "Downloads").map(|(_, p, _)| p.clone());
    let folders: Vec<PathBuf> = folders.into_iter().map(PathBuf::from).collect();
    plan::analyze(&st.db, Path::new(&root), &folders, |b| homes.iter().find(|(p, _)| p == b).and_then(|(_, h)| *h), downloads.as_deref())
}

#[tauri::command(async)]
fn execute_plan(st: State<'_, AppState>, items: Vec<plan::Exec>) -> plan::Outcome {
    plan::execute(&st.db, items)
}

// ---------------------------------------------------------------- Explorer interop

/// Copy/cut onto the Windows clipboard as a file list, so Explorer can paste it (and the reverse via clip_get).
#[tauri::command]
fn clip_set(paths: Vec<String>, cut: bool) -> Result<(), String> {
    use clipboard_win::{raw, Clipboard};
    let _open = Clipboard::new_attempts(10).map_err(|e| format!("Clipboard sedang dipakai: {e}"))?;
    raw::empty().map_err(|e| e.to_string())?;
    raw::set_file_list(&paths).map_err(|e| e.to_string())?;
    if let Some(f) = raw::register_format("Preferred DropEffect") {
        let effect: u32 = if cut { 2 } else { 5 }; // DROPEFFECT_MOVE, or COPY|LINK like Explorer
        raw::set_without_clear(f.get(), &effect.to_le_bytes()).ok();
    }
    Ok(())
}

#[derive(Serialize)]
struct Clip {
    paths: Vec<String>,
    cut: bool,
}

#[tauri::command]
fn clip_get() -> Option<Clip> {
    use clipboard_win::{raw, Clipboard};
    let _open = Clipboard::new_attempts(10).ok()?;
    if !raw::is_format_avail(15) {
        return None; // CF_HDROP
    }
    let mut paths = Vec::new();
    raw::get_file_list(&mut paths).ok()?;
    let mut eff = Vec::new();
    let cut = raw::register_format("Preferred DropEffect")
        .and_then(|f| raw::get_vec(f.get(), &mut eff).ok())
        .map(|_| eff.len() >= 4 && u32::from_le_bytes([eff[0], eff[1], eff[2], eff[3]]) & 2 != 0)
        .unwrap_or(false);
    (!paths.is_empty()).then_some(Clip { paths, cut })
}

#[tauri::command]
fn clip_clear() {
    if let Ok(_open) = clipboard_win::Clipboard::new_attempts(10) {
        clipboard_win::raw::empty().ok();
    }
}

/// Hand a drag that left the window over to Windows, so files can be dropped into Explorer, the desktop, mail…
#[tauri::command]
fn drag_out(window: tauri::WebviewWindow, paths: Vec<String>) -> Result<(), String> {
    let w = window.clone();
    window.run_on_main_thread(move || {
        let files = paths.into_iter().map(PathBuf::from).collect();
        let icon = include_bytes!("../icons/32x32.png").to_vec();
        drag::start_drag(&w, drag::DragItem::Files(files), drag::Image::Raw(icon), |_, _| {}, drag::Options::default()).ok();
    }).map_err(|e| e.to_string())
}

/// (bytes, files, folders) under `dir`; None when `stop()` says the answer is no longer wanted.
fn dir_stats(dir: &Path, stop: &dyn Fn() -> bool) -> Option<(u64, u64, u64)> {
    let (mut size, mut files, mut folders) = (0u64, 0u64, 0u64);
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        if stop() {
            return None;
        }
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                folders += 1;
                stack.push(e.path());
            } else {
                files += 1;
                size += e.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    Some((size, files, folders))
}

static SIZE_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Folder sizes for the Ukuran column, one "folder-size" event each; a newer request cancels an older one.
#[tauri::command]
fn folder_sizes(app: AppHandle, paths: Vec<String>) {
    let gen = SIZE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        for p in paths {
            let stale = || SIZE_GEN.load(Ordering::Relaxed) != gen;
            let Some((size, files, _)) = dir_stats(Path::new(&p), &stale) else { return };
            app.emit("folder-size", serde_json::json!({ "path": p, "size": size, "files": files })).ok();
        }
    });
}

#[derive(Serialize)]
struct Props {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    files: u64,
    folders: u64,
    created: i64,
    modified: i64,
    accessed: i64,
    readonly: bool,
    hidden: bool,
    md5: Option<String>,
    cat: &'static str,
}

#[tauri::command(async)]
fn properties(st: State<'_, AppState>, path: String) -> Result<Props, String> {
    let p = Path::new(&path);
    let m = std::fs::metadata(p).map_err(|e| format!("Tidak bisa dibaca: {e}"))?;
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.clone());
    let (size, files, folders) = if m.is_dir() { dir_stats(p, &|| false).unwrap_or_default() } else { (m.len(), 0, 0) };
    let md5 = if m.is_dir() {
        None
    } else {
        let cached: Option<String> = st.db.lock().unwrap()
            .query_row("SELECT md5 FROM files WHERE path = ?1 AND size = ?2 AND mtime = ?3", params![path, m.len() as i64, secs(m.modified())], |r| r.get(0))
            .optional().ok().flatten().flatten();
        cached.or_else(|| md5_file(p).ok())
    };
    Ok(Props {
        cat: if m.is_dir() { "Folder" } else { rules::category(&name) },
        name, is_dir: m.is_dir(), size, files, folders, md5,
        created: secs(m.created()), modified: secs(m.modified()), accessed: secs(m.accessed()),
        readonly: m.permissions().readonly(), hidden: plan::hidden(&m), path: path.clone(),
    })
}

/// Windows' own Properties dialog (security tab, previous versions, details…).
#[tauri::command]
fn shell_properties(path: String) -> bool {
    use windows_sys::Win32::UI::Shell::{SHObjectProperties, SHOP_FILEPATH};
    let w: Vec<u16> = path.encode_utf16().chain([0]).collect();
    unsafe { SHObjectProperties(std::ptr::null_mut(), SHOP_FILEPATH as u32, w.as_ptr(), std::ptr::null()) != 0 }
}

// ---------------------------------------------------------------- optional local AI (Pengaturan)

#[tauri::command(async)]
fn ai_status(st: State<'_, AppState>) -> models::Status {
    models::status(&st.db)
}

#[tauri::command]
fn ai_download(app: AppHandle, st: State<'_, AppState>) -> Result<(), String> {
    models::download(app, st.db.clone())
}

#[tauri::command]
fn ai_use_existing(app: AppHandle, st: State<'_, AppState>, path: String) -> Result<(), String> {
    models::use_existing(app, st.db.clone(), path)
}

#[tauri::command]
fn ai_cancel() {
    models::cancel();
    GROUP_CANCEL.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn ai_remove(st: State<'_, AppState>) -> Result<(), String> {
    models::remove(&st.db)
}

#[tauri::command]
fn ai_set_enabled(st: State<'_, AppState>, on: bool) -> Result<(), String> {
    if on && !cfg!(feature = "local-llm") {
        return Err("Versi ini dibuat tanpa mesin AI.".into());
    }
    if on && models::model_path(&st.db).is_none() {
        return Err("Model belum ada. Unduh atau pilih file model dulu.".into());
    }
    models::set_setting(&st.db, "ai_enabled", on.then_some("1"));
    Ok(())
}

#[tauri::command(async)]
fn pick_model_file(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog().file().add_filter("Model GGUF", &["gguf"]).blocking_pick_file().and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
}

static GROUP_CANCEL: AtomicBool = AtomicBool::new(false);

#[derive(serde::Deserialize)]
#[cfg_attr(not(feature = "local-llm"), allow(dead_code))]
struct GroupIn {
    src: String,
    name: String,
}

#[derive(Serialize)]
struct GroupOut {
    src: String,
    topic: String,
    reason: String,
}

/// "Kelompokkan per topik": the model proposes topic folders for these documents, then places each one (or none).
/// Progress: "ai-group" events {done, total, phase}. Document text is read in memory only.
#[tauri::command(async)]
fn ai_group(app: AppHandle, st: State<'_, AppState>, items: Vec<GroupIn>, existing: Vec<String>) -> Result<Vec<GroupOut>, String> {
    let model = models::ai_ready(&st.db).ok_or("AI lokal belum aktif. Atur di Pengaturan.")?;
    #[cfg(not(feature = "local-llm"))]
    {
        let _ = (app, items, existing, model);
        Err("Versi ini dibuat tanpa mesin AI.".into())
    }
    #[cfg(feature = "local-llm")]
    {
        GROUP_CANCEL.store(false, Ordering::SeqCst);
        let total = items.len();
        let progress = |done: usize, phase: &str| { app.emit("ai-group", serde_json::json!({ "done": done, "total": total, "phase": phase })).ok(); };
        progress(0, "read");
        let docs: Vec<(String, String)> = items.iter().map(|i| (i.name.clone(), ai::read_text(Path::new(&i.src)).unwrap_or_default())).collect();
        if GROUP_CANCEL.load(Ordering::SeqCst) {
            return Ok(Vec::new());
        }
        progress(0, "topics");
        let mut topics = ai::propose_topics(&model, &docs, &existing)?;
        for e in existing {
            if !topics.contains(&e) {
                topics.push(e);
            }
        }
        let mut out = Vec::new();
        for (k, (it, (name, text))) in items.iter().zip(&docs).enumerate() {
            if GROUP_CANCEL.load(Ordering::SeqCst) {
                break;
            }
            progress(k, "assign");
            if let Ok((t, why)) = ai::choose(&model, name, text, &topics) {
                if t != "-" && topics.contains(&t) {
                    out.push(GroupOut { src: it.src.clone(), topic: t, reason: why });
                }
            }
        }
        progress(total, "done");
        Ok(out)
    }
}

#[tauri::command(async)]
fn pick_folder(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog().file().blocking_pick_folder().and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
fn dismiss(st: State<AppState>, path: String) -> Result<(), String> {
    st.db.lock().unwrap().execute("UPDATE files SET dismissed = 1 WHERE path = ?1", [&path]).map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn open(app: AppHandle, path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err("File tidak ditemukan di lokasi yang tersimpan.".into());
    }
    app.opener().open_path(path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn reveal(app: AppHandle, path: String) -> Result<(), String> {
    app.opener().reveal_item_in_dir(path).map_err(|e| e.to_string())
}

fn main() {
    let db: Db = Arc::new(Mutex::new(open_db()));
    let roots = known_roots();
    let scanning = Arc::new(AtomicBool::new(true));
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { db: db.clone(), roots: roots.clone(), scanning: scanning.clone() })
        .setup(move |app| {
            let (tx, rx) = mpsc::channel();
            let mut w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(e) = res {
                    let walk = matches!(e.kind, EventKind::Create(_) | EventKind::Modify(ModifyKind::Name(_)));
                    for p in e.paths {
                        tx.send((p, walk)).ok();
                    }
                }
            })?;
            for (_, r, _) in &roots {
                w.watch(r, RecursiveMode::Recursive).ok();
            }
            // Debug builds only: FO_DEBUG_SCRIPT=<file.js> is evaluated in the page after start (UI checks without clicking).
            #[cfg(debug_assertions)]
            if let Ok(f) = std::env::var("FO_DEBUG_SCRIPT") {
                use tauri::Manager;
                let h = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(3));
                    if let (Some(w), Ok(js)) = (h.get_webview_window("main"), std::fs::read_to_string(&f)) {
                        w.eval(&js).ok();
                    }
                });
            }
            let gen = secs(Ok(SystemTime::now()));
            let ix = || Indexer { db: db.clone(), roots: roots.clone(), app: app.handle().clone(), gen, skip_cache: HashMap::new(), scanning: scanning.clone() };
            let (scanner, watcher) = (ix(), ix());
            std::thread::spawn(move || scanner.scan());
            std::thread::spawn(move || {
                let _keep_watcher_alive = w;
                watcher.watch(rx)
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_roots, list, counts, apply, dismiss, open, reveal, browse, rename_item, trash_items, new_folder,
            paste_items, undo_batch, analyze_folders, execute_plan, pick_folder, clip_set, clip_get, clip_clear, drag_out,
            folder_sizes, properties, shell_properties, ai_status, ai_download, ai_use_existing, ai_cancel, ai_remove,
            ai_set_enabled, pick_model_file, ai_group
        ])
        .run(tauri::generate_context!())
        .expect("error while running File Organizer");
}
