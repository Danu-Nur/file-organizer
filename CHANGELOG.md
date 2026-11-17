# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Choose the watched folders: add user folders, the user folder, OneDrive, drives or any folder; reorder and remove.
- Plugged-in drives are detected and offered; unplugged listed drives keep their index and refresh on return.
- Sorting by size uses the real total size of folders; the status bar shows the open folder's total.
- Preview pane (Alt+P): images, PDF, audio, video, text/code, DOCX text and folder contents, with file details.
- View menu: preview pane and hidden items (Ctrl+H) as check options.
- Dashboard: headline figures; file types, folders, extensions and file age as chart or sortable table with
  drill-down lists; largest files; most wasteful duplicates; clean-up suggestions.
- Recycle Bin view: restore, delete permanently, empty; undo of Delete restores from the Recycle Bin.

### Fixed
- A new folder now reliably opens in rename mode (a redraw could close the name box).
- The content security policy blocked Tauri's binary IPC channel, so every call fell back to the slower JSON path.
- Index queries for folders at a drive root (`C:\`).
- System folders and hidden items are no longer indexed when a whole drive is watched.

## [0.1.0] — 2026-11-17

First public version.

### Added
- Background index of Desktop, Downloads, Documents, Pictures, Music and Videos (name, path, size, date, MD5).
- Suggestions for new loose files by type, secret-looking names and duplicate content, with Windows notifications.
- Duplicate view grouped by MD5.
- Folder browsing with address bar, history, list and tree views, sortable and resizable columns, folder sizes and
  search by name or MD5.
- File actions: open, rename, delete to the Recycle Bin, new folder, cut/copy/paste via the Windows clipboard,
  drag & drop (inside, in and out of the app), properties, undo.
- "Rapikan folder": pick folders in a tree, review and edit a clean-up plan with live preview, apply as one undoable
  batch. Detects code projects, app folders, mostly-one-type folders, duplicates, empty items, secrets, suspicious PHP,
  older versions and English category names.
- Optional local AI (llama.cpp, Vulkan GPU or CPU): topic subfolders for documents; settings page with hardware
  detection, requirements, advice, resumable SHA-256-verified model download or use of an existing copy.
- Windows 11 style interface in light and dark mode (Indonesian).
