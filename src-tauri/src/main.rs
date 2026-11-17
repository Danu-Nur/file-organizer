// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! File Organizer: watches the user's six Explorer folders, keeps an index (name, path, size, md5 — never the content)
//! and suggests where loose files belong. Moving only happens when the user clicks.
mod models;
mod ops;
mod plan;
mod preview;
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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

type Db = Arc<Mutex<Connection>>;

/// A watched folder in the navigation pane (the user picks which ones).
#[derive(Clone)]
struct Root {
    name: String,
    path: PathBuf,
    /// The category that already lives there (Pictures = Gambar, …).
    home: Option<&'static str>,
    /// Desktop | Downloads | Documents | Pictures | Music | Videos | Home | OneDrive | Drive | Folder
    kind: &'static str,
}
type Roots = Arc<std::sync::RwLock<Vec<Root>>>;

struct AppState {
    db: Db,
    roots: Roots,
    /// Number of scans running (startup scan, folders added later).
    scanning: Arc<AtomicUsize>,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

/// The six folders watched until the user changes the list.
const DEFAULT_KINDS: [&str; 6] = ["Desktop", "Downloads", "Documents", "Pictures", "Music", "Videos"];

/// "C:\x" → "C:\x\", "C:\" stays: the prefix that everything inside a folder starts with.
fn dir_prefix(p: &str) -> String {
    if p.ends_with('\\') { p.to_string() } else { format!("{p}\\") }
}

fn same_path(a: &Path, b: &Path) -> bool {
    let n = |p: &Path| p.to_string_lossy().trim_end_matches('\\').to_lowercase();
    n(a) == n(b)
}

/// Every place File Explorer shows on the left: user folders, home, OneDrive, and the drives.
fn known_places() -> Vec<Root> {
    let mut v: Vec<Root> = Vec::new();
    let mut add = |kind: &'static str, name: String, p: Option<PathBuf>, home: Option<&'static str>| {
        if let Some(p) = p.filter(|p| p.is_dir()) {
            if !v.iter().any(|r| same_path(&r.path, &p)) {
                v.push(Root { name, path: p, home, kind });
            }
        }
    };
    add("Desktop", "Desktop".into(), dirs::desktop_dir(), None);
    add("Downloads", "Downloads".into(), dirs::download_dir(), None);
    add("Documents", "Documents".into(), dirs::document_dir(), Some("Dokumen"));
    add("Pictures", "Pictures".into(), dirs::picture_dir(), Some("Gambar"));
    add("Music", "Music".into(), dirs::audio_dir(), Some("Musik"));
    add("Videos", "Videos".into(), dirs::video_dir(), Some("Video"));
    let home = dirs::home_dir();
    let home_name = home.as_ref().and_then(|h| h.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Home".into());
    add("Home", home_name, home, None);
    for var in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        let p = std::env::var_os(var).map(PathBuf::from);
        let name = p.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "OneDrive".into());
        add("OneDrive", name, p, None);
    }
    for (name, root) in sysinfo::drives() {
        add("Drive", name, Some(PathBuf::from(root)), None);
    }
    v
}

fn make_root(p: PathBuf, places: &[Root]) -> Root {
    places.iter().find(|r| same_path(&r.path, &p)).cloned().unwrap_or_else(|| Root {
        name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.to_string_lossy().into_owned()),
        path: p,
        home: None,
        kind: "Folder",
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedRoot {
    path: String,
    name: String,
}

/// The user's list (setting "roots"), or the six default folders. A folder on a drive that is not plugged in stays in
/// the list (shown as not connected) with the name it had.
fn load_roots(db: &Db) -> Vec<Root> {
    let places = known_places();
    match models::setting(db, "roots").and_then(|s| serde_json::from_str::<Vec<SavedRoot>>(&s).ok()) {
        Some(list) => list.into_iter().map(|s| {
            let p = PathBuf::from(&s.path);
            if p.is_dir() {
                make_root(p, &places)
            } else {
                let kind = if s.path.len() <= 3 && s.path.ends_with(":\\") { "Drive" } else { "Folder" };
                Root { name: s.name, path: p, home: None, kind }
            }
        }).collect(),
        None => places.into_iter().filter(|r| DEFAULT_KINDS.contains(&r.kind)).collect(),
    }
}

/// Polls the drive letters: announces plugged-in drives, and re-watches + re-scans a listed folder whose drive came
/// back (or stops watching it when the drive is removed). The index of an unplugged drive is kept.
fn drive_watch(app: AppHandle) {
    use tauri::Manager;
    let mut mask = sysinfo::drive_mask();
    let mut drives = sysinfo::drives();
    let mut online: HashMap<PathBuf, bool> = app.state::<AppState>().roots.read().unwrap().iter().map(|r| (r.path.clone(), r.path.is_dir())).collect();
    loop {
        std::thread::sleep(Duration::from_secs(3));
        let now = sysinfo::drive_mask();
        if now == mask {
            continue;
        }
        mask = now;
        let st = app.state::<AppState>();
        let fresh = sysinfo::drives();
        for (name, path) in fresh.iter().filter(|d| !drives.iter().any(|o| o.1 == d.1)) {
            app.emit("drive-added", serde_json::json!({ "name": name, "path": path })).ok();
        }
        drives = fresh;
        let roots = st.roots.read().unwrap().clone();
        let mut back = Vec::new();
        for r in &roots {
            let on = r.path.is_dir();
            let was = *online.entry(r.path.clone()).or_insert(on); // added later via set_roots: already handled there
            if on == was {
                continue;
            }
            online.insert(r.path.clone(), on);
            if let Some(w) = st.watcher.lock().unwrap().as_mut() {
                if on { w.watch(&r.path, RecursiveMode::Recursive).ok(); } else { w.unwatch(&r.path).ok(); }
            }
            if on {
                back.push(r.clone());
            }
        }
        if !back.is_empty() {
            st.scanning.fetch_add(1, Ordering::SeqCst);
            let ix = Indexer::new(&st, &app, secs(Ok(SystemTime::now())));
            std::thread::spawn(move || ix.scan(back, false));
        }
        app.emit("roots-changed", ()).ok();
    }
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
    roots: Roots,
    app: AppHandle,
    gen: i64,
    skip_cache: HashMap<PathBuf, bool>,
    scanning: Arc<AtomicUsize>,
}

impl Indexer {
    fn new(st: &AppState, app: &AppHandle, gen: i64) -> Self {
        Indexer { db: st.db.clone(), roots: st.roots.clone(), app: app.clone(), gen, skip_cache: HashMap::new(), scanning: st.scanning.clone() }
    }

    fn root_of(&self, p: &Path) -> Option<(PathBuf, Option<&'static str>)> {
        self.roots.read().unwrap().iter().find(|r| p.starts_with(&r.path)).map(|r| (r.path.clone(), r.home))
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
            let (Ok(ft), Ok(meta)) = (e.file_type(), e.metadata()) else { continue };
            let p = e.path();
            if ft.is_symlink() || plan::hidden(&meta) {
                continue; // junctions like "My Music", hidden/system items (as File Explorer hides them)
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
        let prefix = dir_prefix(&key);
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

    /// Scan (own thread): loose files of each root first, so suggestions show up right away; then the deep walk.
    /// `purge` (startup only): drop rows that were not seen, i.e. files that disappeared while the app was closed.
    /// The caller has already counted this scan in `scanning`.
    // ponytail: a root removed while its scan runs keeps being walked until the scan ends; its rows go at next start.
    fn scan(mut self, roots: Vec<Root>, purge: bool) {
        for r in &roots {
            for e in std::fs::read_dir(&r.path).into_iter().flatten().flatten() {
                if e.file_type().map(|t| t.is_file()).unwrap_or(false) && !e.metadata().map(|m| plan::hidden(&m)).unwrap_or(false) {
                    self.file(&e.path(), &r.path, r.home, false);
                }
            }
        }
        self.app.emit("changed", ()).ok();
        for r in &roots {
            self.walk(&r.path, &r.path, r.home, false);
            self.app.emit("changed", ()).ok();
        }
        if purge {
            // Only inside the folders actually scanned: an unplugged drive keeps its index.
            for r in &roots {
                let prefix = dir_prefix(&r.path.to_string_lossy());
                self.db.lock().unwrap()
                    .execute("DELETE FROM files WHERE seen < ?1 AND substr(path, 1, length(?2)) = ?2", params![self.gen, prefix])
                    .ok();
            }
        }
        self.scanning.fetch_sub(1, Ordering::SeqCst);
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
    name: String,
    path: String,
    kind: &'static str,
    /// For "places": already in the user's list.
    added: bool,
    /// False when its drive is not plugged in.
    online: bool,
}

fn root_info(r: &Root, added: bool) -> RootInfo {
    RootInfo { name: r.name.clone(), path: r.path.to_string_lossy().into_owned(), kind: r.kind, added, online: r.path.is_dir() }
}

const DUPS: &str = "md5 IN (SELECT md5 FROM files WHERE size > 0 AND md5 IS NOT NULL GROUP BY md5 HAVING COUNT(*) > 1)";

#[tauri::command]
fn list_roots(st: State<AppState>) -> Vec<RootInfo> {
    st.roots.read().unwrap().iter().map(|r| root_info(r, true)).collect()
}

/// Everything that can be added to the navigation pane (user folders, home, OneDrive, drives).
#[tauri::command(async)]
fn places(st: State<'_, AppState>) -> Vec<RootInfo> {
    let roots = st.roots.read().unwrap().clone();
    known_places().iter().map(|p| root_info(p, roots.iter().any(|r| same_path(&r.path, &p.path)))).collect()
}

/// Replace the watched folders (add, remove, reorder). New folders are watched and indexed at once; rows of removed
/// folders are dropped from the index. Files themselves are never touched.
#[tauri::command(async)]
fn set_roots(app: AppHandle, st: State<'_, AppState>, paths: Vec<String>) -> Result<Vec<RootInfo>, String> {
    let places = known_places();
    let old = st.roots.read().unwrap().clone();
    let mut new: Vec<Root> = Vec::new();
    for p in &paths {
        let pb = PathBuf::from(p);
        if let Some(o) = old.iter().find(|o| same_path(&o.path, &pb)) {
            new.push(o.clone()); // keeps its name, also while its drive is unplugged
        } else if pb.is_dir() {
            new.push(make_root(pb, &places));
        } else {
            return Err(format!("Folder tidak ditemukan: {p}"));
        }
    }
    for (i, a) in new.iter().enumerate() {
        for b in &new[i + 1..] {
            if a.path.starts_with(&b.path) || b.path.starts_with(&a.path) {
                return Err(format!("\"{}\" dan \"{}\" saling mencakup; cukup salah satu.", a.name, b.name));
            }
        }
    }
    let added: Vec<Root> = new.iter().filter(|r| !old.iter().any(|o| same_path(&o.path, &r.path))).cloned().collect();
    let removed: Vec<Root> = old.iter().filter(|o| !new.iter().any(|r| same_path(&o.path, &r.path))).cloned().collect();
    if let Some(w) = st.watcher.lock().unwrap().as_mut() {
        for r in &removed {
            w.unwatch(&r.path).ok();
        }
        for r in &added {
            w.watch(&r.path, RecursiveMode::Recursive).map_err(|e| format!("{} tidak bisa dipantau: {e}", r.name))?;
        }
    }
    *st.roots.write().unwrap() = new.clone();
    let saved: Vec<SavedRoot> = new.iter().map(|r| SavedRoot { path: r.path.to_string_lossy().into_owned(), name: r.name.clone() }).collect();
    models::set_setting(&st.db, "roots", Some(&serde_json::to_string(&saved).map_err(|e| e.to_string())?));
    for r in removed.iter().filter(|r| !new.iter().any(|n| r.path.starts_with(&n.path))) {
        let prefix = dir_prefix(&r.path.to_string_lossy());
        st.db.lock().unwrap().execute("DELETE FROM files WHERE substr(path, 1, length(?1)) = ?1", [prefix]).ok();
    }
    if !added.is_empty() {
        st.scanning.fetch_add(1, Ordering::SeqCst);
        let ix = Indexer::new(&st, &app, secs(Ok(SystemTime::now())));
        std::thread::spawn(move || ix.scan(added, false));
    }
    app.emit("changed", ()).ok();
    Ok(new.iter().map(|r| root_info(r, true)).collect())
}

/// Extension used for the dashboard's "Ekstensi" card and the `ext:` drill-down ("" when there is none).
fn ext_of(name: &str) -> String {
    Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// `view`: "suggest" | "dup" | "all" | a root path | "cat:<category>" | "ext:<extension>" | "year:<yyyy>"
/// (dashboard drill-downs). `sort`: name | mtime | size | path | md5.
#[tauri::command]
fn list(st: State<AppState>, q: String, view: String, sort: String, desc: bool) -> Result<Vec<Row>, String> {
    // Drill-downs are decided per row in Rust (same rules as the dashboard), then capped like the other views.
    let drill: Option<Box<dyn Fn(&Row) -> bool>> = match view.split_once(':') {
        Some(("cat", c)) => { let c = c.to_string(); Some(Box::new(move |r: &Row| r.cat == c)) }
        Some(("ext", e)) => { let e = e.to_string(); Some(Box::new(move |r: &Row| ext_of(&r.name) == e)) }
        Some(("year", y)) => { let y: i64 = y.parse().unwrap_or(0); Some(Box::new(move |r: &Row| r.mtime > 0 && year_of(r.mtime) == y)) }
        _ => None,
    };
    let filter = match view.as_str() {
        "suggest" => "suggest IS NOT NULL AND dismissed = 0".to_string(),
        "dup" => DUPS.to_string(),
        "all" => "1".to_string(),
        _ if drill.is_some() => "1".to_string(),
        _ => "substr(path, 1, length(?2)) = ?2".to_string(),
    };
    let col = match sort.as_str() { "name" => "name COLLATE NOCASE", "size" => "size", "path" => "path COLLATE NOCASE", "md5" => "md5", _ => "mtime" };
    let order = if view == "dup" { format!("md5, {col} {}", if desc { "DESC" } else { "ASC" }) } else { format!("{col} {}", if desc { "DESC" } else { "ASC" }) };
    let like = format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let sql = format!(
        "SELECT path, name, size, mtime, md5, CASE WHEN dismissed = 0 THEN suggest END, reason FROM files
         WHERE ({filter}) AND (name LIKE ?1 ESCAPE '\\' OR md5 LIKE ?1 ESCAPE '\\') ORDER BY {order}{}",
        if drill.is_some() { "" } else { " LIMIT 2000" }
    );
    let db = st.db.lock().unwrap();
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let map = |r: &rusqlite::Row| {
        let name: String = r.get(1)?;
        Ok(Row { cat: rules::category(&name), path: r.get(0)?, name, size: r.get(2)?, mtime: r.get(3)?, md5: r.get(4)?, suggest: r.get(5)?, reason: r.get(6)? })
    };
    let rows = if filter.contains("?2") { stmt.query_map(params![like, dir_prefix(&view)], map) } else { stmt.query_map(params![like], map) };
    let rows = rows.map_err(|e| e.to_string())?.flatten();
    let out = match &drill {
        Some(keep) => rows.filter(|r| keep(r)).take(2000).collect(),
        None => rows.collect(),
    };
    Ok(out)
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
    let scanning = st.scanning.load(Ordering::Relaxed) > 0;
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
    let root = st.roots.read().unwrap().iter().find(|r| src.parent() == Some(r.path.as_path())).map(|r| r.path.clone())
        .ok_or("File ini tidak lagi berada di folder utama.")?;
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
    hidden: bool,
}

/// One folder's content straight from disk, joined with the index for md5 and suggestions. Hidden items are left out
/// unless `show_hidden` (the "Item tersembunyi" view option); protected OS files (hidden + system) always are.
#[tauri::command(async)]
fn browse(st: State<'_, AppState>, path: String, show_hidden: Option<bool>) -> Result<Vec<Entry>, String> {
    let show_hidden = show_hidden.unwrap_or(false);
    let rd = std::fs::read_dir(&path).map_err(|e| format!("Folder tidak bisa dibuka: {e}"))?;
    let mut out = Vec::new();
    for e in rd.flatten() {
        let (Ok(ft), Ok(meta)) = (e.file_type(), e.metadata()) else { continue };
        let (hidden, system) = plan::attrs(&meta);
        if (hidden && system) || (hidden && !show_hidden) {
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
            path: key, name, is_dir, md5, suggest, reason, hidden,
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
    let roots = st.roots.read().unwrap().clone();
    let homes: Vec<(PathBuf, Option<&'static str>)> = roots.iter().map(|r| (r.path.clone(), r.home)).collect();
    let downloads = roots.iter().find(|r| r.kind == "Downloads").map(|r| r.path.clone());
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

// ---------------------------------------------------------------- Dashboard

#[derive(Serialize, Default, Clone)]
struct Bucket {
    label: String,
    path: Option<String>,
    files: i64,
    size: i64,
}

#[derive(Serialize)]
struct DupGroup {
    md5: String,
    count: i64,
    size: i64,
    wasted: i64,
    paths: Vec<String>,
}

#[derive(Serialize)]
struct Dash {
    files: i64,
    size: i64,
    suggest: i64,
    dup_files: i64,
    dup_wasted: i64,
    empty_files: i64,
    by_cat: Vec<Bucket>,
    /// Top extensions by size.
    by_ext: Vec<Bucket>,
    by_root: Vec<Bucket>,
    by_year: Vec<Bucket>,
    largest: Vec<Row>,
    dup_groups: Vec<DupGroup>,
    /// Not modified for more than a year.
    stale: Bucket,
    /// In Downloads and older than 90 days.
    old_downloads: Bucket,
}

/// Calendar year of a unix time (UTC), without a date library.
fn year_of(secs: i64) -> i64 {
    // days → civil date (Howard Hinnant's algorithm)
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    yoe + era * 400 + i64::from(month <= 2)
}

/// Everything the dashboard shows, from the index in one pass (no disk access).
#[tauri::command(async)]
fn dashboard(st: State<'_, AppState>) -> Result<Dash, String> {
    let roots = st.roots.read().unwrap().clone();
    let now = secs(Ok(SystemTime::now()));
    let downloads = roots.iter().find(|r| r.kind == "Downloads").map(|r| dir_prefix(&r.path.to_string_lossy()));
    let prefixes: Vec<(String, &Root)> = roots.iter().map(|r| (dir_prefix(&r.path.to_string_lossy()), r)).collect();
    let mut by_cat: HashMap<&'static str, Bucket> = HashMap::new();
    let mut by_ext: HashMap<String, Bucket> = HashMap::new();
    let mut by_year: HashMap<i64, Bucket> = HashMap::new();
    let mut by_root: Vec<Bucket> = roots.iter().map(|r| Bucket { label: r.name.clone(), path: Some(r.path.to_string_lossy().into_owned()), ..Default::default() }).collect();
    let (mut files, mut size, mut empty_files) = (0i64, 0i64, 0i64);
    let (mut stale, mut old_downloads) = (Bucket::default(), Bucket::default());
    let db = st.db.lock().unwrap();
    {
        let mut stmt = db.prepare("SELECT path, name, size, mtime FROM files").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?)))
            .map_err(|e| e.to_string())?;
        for (path, name, sz, mtime) in rows.flatten() {
            files += 1;
            size += sz;
            empty_files += i64::from(sz == 0);
            let cat = rules::category(&name);
            let b = by_cat.entry(cat).or_insert_with(|| Bucket { label: cat.into(), ..Default::default() });
            b.files += 1;
            b.size += sz;
            let e = ext_of(&name);
            let b = by_ext.entry(e.clone()).or_insert_with(|| Bucket { label: e, ..Default::default() });
            b.files += 1;
            b.size += sz;
            if mtime > 0 {
                let y = year_of(mtime);
                let b = by_year.entry(y).or_insert_with(|| Bucket { label: y.to_string(), ..Default::default() });
                b.files += 1;
                b.size += sz;
            }
            if let Some(i) = prefixes.iter().position(|(p, _)| path.starts_with(p.as_str())) {
                by_root[i].files += 1;
                by_root[i].size += sz;
            }
            if mtime > 0 && now - mtime > 365 * 86_400 {
                stale.files += 1;
                stale.size += sz;
            }
            if mtime > 0 && now - mtime > 90 * 86_400 && downloads.as_deref().is_some_and(|d| path.starts_with(d)) {
                old_downloads.files += 1;
                old_downloads.size += sz;
            }
        }
    }
    let mut by_cat: Vec<Bucket> = by_cat.into_values().collect();
    by_cat.sort_by(|a, b| b.size.cmp(&a.size));
    let mut by_ext: Vec<Bucket> = by_ext.into_values().collect();
    by_ext.sort_by(|a, b| b.size.cmp(&a.size));
    by_ext.truncate(30);
    by_root.sort_by(|a, b| b.size.cmp(&a.size));
    let mut by_year: Vec<Bucket> = by_year.into_values().collect();
    by_year.sort_by(|a, b| a.label.cmp(&b.label));

    let largest = {
        let mut stmt = db.prepare("SELECT path, name, size, mtime, md5, CASE WHEN dismissed = 0 THEN suggest END, reason FROM files ORDER BY size DESC LIMIT 15")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |r| {
            let name: String = r.get(1)?;
            Ok(Row { cat: rules::category(&name), path: r.get(0)?, name, size: r.get(2)?, mtime: r.get(3)?, md5: r.get(4)?, suggest: r.get(5)?, reason: r.get(6)? })
        }).map_err(|e| e.to_string())?;
        rows.flatten().collect::<Vec<_>>()
    };
    let groups = "SELECT md5, COUNT(*) AS c, MAX(size) AS s FROM files WHERE size > 0 AND md5 IS NOT NULL GROUP BY md5 HAVING c > 1";
    let (dup_files, dup_wasted): (i64, i64) = db
        .query_row(&format!("SELECT COALESCE(SUM(c), 0), COALESCE(SUM(s * (c - 1)), 0) FROM ({groups})"), [], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?;
    let mut dup_groups = Vec::new();
    {
        let mut stmt = db.prepare(&format!("SELECT md5, c, s FROM ({groups}) ORDER BY s * (c - 1) DESC LIMIT 8")).map_err(|e| e.to_string())?;
        let top: Vec<(String, i64, i64)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(|e| e.to_string())?.flatten().collect();
        let mut paths_of = db.prepare("SELECT path FROM files WHERE md5 = ?1 ORDER BY length(path) LIMIT 5").map_err(|e| e.to_string())?;
        for (md5, count, s) in top {
            let paths = paths_of.query_map([&md5], |r| r.get(0)).map_err(|e| e.to_string())?.flatten().collect();
            dup_groups.push(DupGroup { md5, count, size: s, wasted: s * (count - 1), paths });
        }
    }
    let suggest: i64 = db.query_row("SELECT COUNT(*) FROM files WHERE suggest IS NOT NULL AND dismissed = 0", [], |r| r.get(0)).unwrap_or(0);
    Ok(Dash { files, size, suggest, dup_files, dup_wasted, empty_files, by_cat, by_ext, by_root, by_year, largest, dup_groups, stale, old_downloads })
}

// ---------------------------------------------------------------- Recycle Bin

#[derive(Serialize)]
struct BinItem {
    /// Shell id of the item (its file inside the Recycle Bin).
    path: String,
    name: String,
    /// Where it was before it was deleted.
    origin: String,
    deleted: i64,
    size: i64,
    is_dir: bool,
    cat: &'static str,
}

fn bin_items() -> Result<Vec<trash::TrashItem>, String> {
    trash::os_limited::list().map_err(|e| format!("Recycle Bin tidak bisa dibaca: {e}"))
}

#[tauri::command(async)]
fn bin_list() -> Result<Vec<BinItem>, String> {
    Ok(bin_items()?.into_iter().map(|it| {
        // The id is the item's real file inside $Recycle.Bin: read type and size there (the shell's own metadata
        // treats .zip files as folders).
        let (size, is_dir) = match std::fs::metadata(&it.id) {
            Ok(m) if m.is_dir() => (0, true),
            Ok(m) => (m.len() as i64, false),
            Err(_) => match trash::os_limited::metadata(&it).map(|m| m.size) {
                Ok(trash::TrashItemSize::Bytes(n)) => (n as i64, false),
                Ok(trash::TrashItemSize::Entries(_)) => (0, true),
                Err(_) => (0, false),
            },
        };
        let name = it.name.to_string_lossy().into_owned();
        BinItem {
            path: it.id.to_string_lossy().into_owned(),
            origin: it.original_parent.join(&it.name).to_string_lossy().into_owned(),
            cat: if is_dir { "Folder" } else { rules::category(&name) },
            deleted: it.time_deleted, name, size, is_dir,
        }
    }).collect())
}

fn bin_pick(ids: &[String]) -> Result<Vec<trash::TrashItem>, String> {
    Ok(bin_items()?.into_iter().filter(|it| ids.iter().any(|id| it.id.to_string_lossy() == id.as_str())).collect())
}

/// Put items back where they were. Refused (nothing restored) if a file with the same name is there now.
#[tauri::command(async)]
fn bin_restore(ids: Vec<String>) -> Result<usize, String> {
    let items = bin_pick(&ids)?;
    let n = items.len();
    trash::os_limited::restore_all(items).map_err(|e| match e {
        trash::Error::RestoreCollision { path, .. } => format!("Tidak dipulihkan: sudah ada \"{}\" di lokasi asalnya.", path.display()),
        e => e.to_string(),
    })?;
    Ok(n)
}

/// Undo of a Delete: restore the most recent Recycle Bin entry of each of these original paths.
#[tauri::command(async)]
fn bin_restore_paths(paths: Vec<String>) -> Result<usize, String> {
    let mut pick: Vec<trash::TrashItem> = Vec::new();
    for it in bin_items()? {
        let orig = it.original_parent.join(&it.name);
        if !paths.iter().any(|p| same_path(Path::new(p), &orig)) {
            continue;
        }
        if let Some(i) = pick.iter().position(|x| same_path(&x.original_parent.join(&x.name), &orig)) {
            if pick[i].time_deleted < it.time_deleted { pick[i] = it; }
        } else {
            pick.push(it);
        }
    }
    let n = pick.len();
    trash::os_limited::restore_all(pick).map_err(|e| e.to_string())?;
    Ok(n)
}

/// Permanently delete (the UI always asks first). `ids` empty = empty the whole Recycle Bin.
#[tauri::command(async)]
fn bin_purge(ids: Vec<String>) -> Result<usize, String> {
    let items = if ids.is_empty() { bin_items()? } else { bin_pick(&ids)? };
    let n = items.len();
    trash::os_limited::purge_all(items).map_err(|e| e.to_string())?;
    Ok(n)
}

/// Preview pane: the file's bytes (image, PDF, audio, video) as a raw binary response — no base64 — up to `max`.
#[tauri::command(async)]
fn preview_bytes(path: String, max: u64) -> Result<tauri::ipc::Response, String> {
    preview::bytes(Path::new(&path), max.min(300 << 20)).map(tauri::ipc::Response::new)
}

/// Preview pane: the opening text of a text/code file or a DOCX.
#[tauri::command(async)]
fn preview_text(path: String) -> Result<String, String> {
    preview::text(Path::new(&path))
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
    let roots: Roots = Arc::new(std::sync::RwLock::new(load_roots(&db)));
    let scanning = Arc::new(AtomicUsize::new(1)); // the startup scan
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { db, roots: roots.clone(), scanning, watcher: Mutex::new(None) })
        .setup(move |app| {
            use tauri::Manager;
            let (tx, rx) = mpsc::channel();
            let mut w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(e) = res {
                    let walk = matches!(e.kind, EventKind::Create(_) | EventKind::Modify(ModifyKind::Name(_)));
                    for p in e.paths {
                        tx.send((p, walk)).ok();
                    }
                }
            })?;
            for r in roots.read().unwrap().iter() {
                w.watch(&r.path, RecursiveMode::Recursive).ok();
            }
            // Debug builds only: FO_DEBUG_SCRIPT=<file.js> is evaluated in the page after start (UI checks without clicking).
            #[cfg(debug_assertions)]
            if let Ok(f) = std::env::var("FO_DEBUG_SCRIPT") {
                let h = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(3));
                    if let (Some(w), Ok(js)) = (h.get_webview_window("main"), std::fs::read_to_string(&f)) {
                        w.eval(&js).ok();
                    }
                });
            }
            let gen = secs(Ok(SystemTime::now()));
            let st = app.state::<AppState>();
            *st.watcher.lock().unwrap() = Some(w); // kept in the state so folders can be added/removed later
            let (scanner, watcher) = (Indexer::new(&st, app.handle(), gen), Indexer::new(&st, app.handle(), gen));
            let startup_roots: Vec<Root> = roots.read().unwrap().iter().filter(|r| r.path.is_dir()).cloned().collect();
            std::thread::spawn(move || scanner.scan(startup_roots, true));
            std::thread::spawn(move || watcher.watch(rx));
            let h = app.handle().clone();
            std::thread::spawn(move || drive_watch(h));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_roots, places, set_roots, list, counts, dashboard, preview_bytes, preview_text, bin_list, bin_restore,
            bin_restore_paths, bin_purge, apply, dismiss, open, reveal, browse, rename_item, trash_items, new_folder,
            paste_items, undo_batch, analyze_folders, execute_plan, pick_folder, clip_set, clip_get, clip_clear, drag_out,
            folder_sizes, properties, shell_properties, ai_status, ai_download, ai_use_existing, ai_cancel, ai_remove,
            ai_set_enabled, pick_model_file, ai_group
        ])
        .run(tauri::generate_context!())
        .expect("error while running File Organizer");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_and_folder_prefixes() {
        assert_eq!(dir_prefix(r"C:\"), r"C:\"); // a drive root already ends with the separator
        assert_eq!(dir_prefix(r"C:\Users\x"), r"C:\Users\x\");
        assert!(same_path(Path::new(r"C:\Users\X\"), Path::new(r"c:\users\x")));
        assert!(!same_path(Path::new(r"C:\Users\X"), Path::new(r"C:\Users\Xy")));
    }

    #[test]
    fn years() {
        assert_eq!(year_of(0), 1970);
        assert_eq!(year_of(1_704_067_199), 2023); // 2023-12-31 23:59:59 UTC
        assert_eq!(year_of(1_704_067_200), 2024); // 2024-01-01 00:00:00 UTC
        assert_eq!(year_of(1_709_164_800), 2024); // 2024-02-29 (leap day)
    }
}
