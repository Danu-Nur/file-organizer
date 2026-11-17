# Architecture

```
ui/                      plain HTML/CSS/JS, loaded by Tauri (no bundler)
  index.html             layout: command bar, navigation pane, panes, status bar
  app.css                Windows 11 style tokens (light/dark)
  app.js                 state, rendering, keyboard/mouse, Tidy wizard, Settings
src-tauri/
  src/main.rs            app setup, background indexer + file watcher, Tauri commands
  src/rules.rs           pure rules: category of a name, ignored files/folders, version names
  src/plan.rs            "Rapikan folder": analyse ticked folders → plan items; execute a plan
  src/ops.rs             file operations: move without overwrite, merge, undo log, rename, Recycle Bin, paste
  src/sysinfo.rs         RAM / CPU load / GPU + VRAM / free disk and the AI advice
  src/models.rs          optional model: settings, resumable SHA-256-verified download, reuse of a local copy
  src/ai.rs              (feature `local-llm`) llama.cpp worker: JSON answers with a grammar, topic proposals
  build.rs               Tauri build; links two llama.cpp helper libraries on Windows when `local-llm` is on
```

## Data

One SQLite file, `%LOCALAPPDATA%\file-organizer\index.db`:

| table | contents |
|---|---|
| `files` | path (key), name, size, mtime, md5, suggestion, reason, dismissed, seen (scan generation) |
| `moves` | undo log: batch, seq, op (`mv` / `mkdir` / `rmdir`), src, dst |
| `settings` | `ai_enabled`, `ai_model_path` |

## Indexing

At start two threads run side by side:

- **scanner** — first the loose files of each root (so suggestions appear at once), then a recursive walk. Files whose
  size and mtime are unchanged reuse their stored MD5; suggestions are still recomputed so rule changes apply. Rows not
  seen in this scan are removed at the end.
- **watcher** — `notify` events on the six roots, debounced (a path is handled once it has been quiet for 2 s, so
  downloads in progress are not hashed repeatedly). New folders moved in are walked.

Skipped everywhere: hidden folders, dependency/build folders, folders named `Projects`/`Proyek`, and code projects
(a folder with `.git`, `package.json`, `Cargo.toml`, …).

## Suggestions

For a loose file in a root: same MD5 as another indexed file → `_Duplikat`; otherwise `rules::suggest` by name
(secret-looking names → `_Sensitif`). A root that is itself a category (Pictures = Gambar, …) keeps its own files.
With AI on, a document suggested for `Dokumen` may get a topic subfolder that already exists there.

## Tidy plan

`plan::analyze(root, ticked folders)` looks at each ticked folder's direct children:

- folders: English category name → rename/merge to Indonesian; category folders are destinations; code project →
  `Proyek`; `.exe` directly inside → `Program`; empty → `_Kosong`; ≥ 60 % one type → that type; otherwise "leave".
- files: empty → `_Kosong`; suspicious PHP / image with PHP inside → `_Karantina`; MD5 duplicate → `_Duplikat` (one
  copy is kept: one outside the ticked folders if any, else the shortest name); secret-looking name → `_Sensitif`;
  older version of `name (n).ext` with different content → `_Versi Lama` (unticked); else by type. Inside a category
  folder only the special cases are flagged.

`plan::execute` runs deepest folders first (a ticked subfolder is tidied before it may move itself), renames before
files before folders, through one `ops::Log` batch. `ops::undo` replays the batch backwards.

## UI

`app.js` keeps one state object `S` (view, folder, rows, selection, clipboard, undo stack) and the wizard state `T`.
Views: Saran / Duplikat / Semua file (from the index) and folder browsing (from disk). Rows are rendered as a flat list
or a tree (folders from disk, or folders rebuilt from result paths). Events from the backend: `changed` (index),
`folder-size`, `ai-model` (download progress), `ai-group` (AI progress), and Tauri's file drag-drop events.

## Optional AI

`ai.rs` owns one worker thread (the queue): it loads the model on first use (GPU, CPU fallback), refuses when less than
2 GB RAM is free, and releases the model after 5 idle minutes. Answers are constrained by a JSON schema → grammar so
the model can only pick from the allowed folders.
