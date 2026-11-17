# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-11-17

First public version.

### Suggestions and index
- Background index of the watched folders (name, path, size, date, MD5 — never file contents), with a live watcher.
- Suggestions for new loose files by type, secret-looking names and duplicate content; Windows notifications.
- Duplicate view grouped by content (MD5).

### Watched folders
- Default: Desktop, Downloads, Documents, Pictures, Music, Videos. Add the user folder, OneDrive, drives or any
  folder; reorder and remove (files are never touched).
- Plugged-in drives are detected and offered; an unplugged listed drive keeps its index and refreshes on return.
- System folders (Windows, Program Files, AppData) and hidden items are skipped.

### Browsing and file actions
- Address bar, history, list and tree views, sortable and resizable columns, search by name or MD5.
- Folder sizes in the background; sorting by size ranks folders by their real total; folder total in the status bar.
- Preview pane (Alt+P): images, PDF, audio, video, text and code, DOCX text, folder contents, file details.
- View menu: preview pane and hidden items (Ctrl+H).
- Open, rename, new folder (opens in rename mode), delete to the Recycle Bin (undoable), cut/copy/paste via the
  Windows clipboard, drag & drop (inside the app, from the desktop, out to other apps), properties, undo (Ctrl+Z).
- Recycle Bin view: restore, delete permanently, empty (always confirmed).

### Tidy a folder
- Tick folders in a tree, review and edit a clean-up plan with a live preview, apply as one undoable batch.
- Detects code projects, app folders, mostly-one-type folders, duplicates, empty items, secrets, suspicious PHP, older
  versions and English category folder names.

### Dashboard
- Headline figures, file types / folders / extensions / file age as a chart or sortable table with drill-down lists,
  largest files, most wasteful duplicates, clean-up suggestions.

### Optional local AI
- llama.cpp with Vulkan GPU or CPU, off by default: topic subfolders for documents.
- Settings page with hardware detection, requirements and advice; resumable, SHA-256-verified model download or use
  of an existing copy.

### Security
- Bounded reads of untrusted files (previews, AI text extraction); the model download stops above the official size.

### Developer
- `FO_DATA_DIR`, `FO_ROOTS` and (debug builds) `FO_DEBUG_SCRIPT`; demo data and screenshot scripts.
