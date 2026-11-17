# File Organizer

**A Windows desktop app that keeps your everyday folders tidy — and never moves a file without asking.**

File Organizer watches the folders you choose (by default Desktop, Downloads, Documents, Pictures, Music and Videos;
add any folder, your user folder, OneDrive or whole drives), suggests where new files belong,
finds exact duplicates, and turns a messy folder into an editable clean-up plan. Nothing is moved until you approve
it, nothing is deleted, and every run can be undone.

Built with [Tauri 2](https://tauri.app) (Rust) and a dependency-free HTML/JS interface that follows the Windows 11
look in light and dark mode. The interface is in Indonesian (Bahasa Indonesia).

> Screenshots: see [`docs/screenshots/`](docs/screenshots/) (add your own from a sample folder).

## Features

**Suggestions**
- New loose files in the six folders get a suggested destination by type (Dokumen, Gambar, Spreadsheet, Arsip,
  Program, Kode, Musik, Video, …).
- Secret-looking names (recovery codes, keys, `.env`, passwords) are routed to `_Sensitif`.
- A file whose content already exists elsewhere is routed to `_Duplikat`.
- One click to move, or dismiss. A Windows notification announces new suggestions.

**Dashboard**
- Headline figures: indexed size and files, space wasted by duplicates, pending suggestions, files untouched for a year.
- File types, watched folders, extensions and file age (size per year modified), each as a chart (size and share)
  or a sortable table. Click a bar or row to open a temporary list of those files (all `.exe`, all videos, files
  from 2021, …) with every list feature: sorting, tree view, preview, actions.
- Largest files, duplicate groups that waste the most space, and a "can be cleaned up" list with one-click actions.

**Duplicates and index**
- Exact duplicates by MD5, grouped by content.
- The index holds name, path, size, date and MD5 only — file contents are never stored.

**Watched folders**
- The navigation pane lists the watched folders; the + button adds user folders, the user folder, OneDrive, any
  drive or any other folder. Reorder or remove from the context menu (files are never touched).
- External drives are detected when plugged in and can be added with one click. A listed drive that is unplugged is
  shown as not connected, keeps its index, and is re-watched and refreshed when it comes back.
- System folders (Windows, Program Files, AppData) and hidden items are skipped when a whole drive is watched.

**Folder browsing**
- Address bar with back / forward / up, list and tree views, sortable and resizable columns, search by name or MD5.
- Folder sizes are computed in the background; sorting by size ranks folders by their real total, and the status
  bar shows the total of the open folder.
- Preview pane (Alt+P, resizable, closable): images, PDF, audio, video, text and code files, DOCX text, folder
  contents, plus details (type, size, date, location, MD5) and the suggested destination.
- View menu with check options: preview pane, hidden items (Ctrl+H; shown dimmed, protected OS files stay hidden).
- Recycle Bin in the navigation pane: see what was deleted and from where, restore to the original location, delete
  permanently or empty it (always confirmed). Delete can be undone with Ctrl+Z.
- Open, rename (F2), delete to the Recycle Bin (Del), new folder (Ctrl+Shift+N, opens straight in rename mode),
  properties (Alt+Enter).
- Cut / copy / paste through the Windows clipboard, so files can be pasted to and from other windows.
- Drag & drop inside the app, from the desktop into the app, and out of the app.
- Undo the last move with Ctrl+Z.

**Tidy a folder**
1. Tick which folders should be tidied (unticked subfolders keep their content).
2. Review the plan: files by type, code projects → `Proyek`, app folders → `Program`, folders that are mostly one
   type, duplicates → `_Duplikat`, empty files/folders → `_Kosong`, secrets → `_Sensitif`, suspicious PHP →
   `_Karantina`, older versions (`report (1).md` with different content) → `_Versi Lama` (unticked by default), and
   English category folder names → Indonesian (merged).
3. Edit anything: tick/untick, type another destination (`Dokumen\Pajak`), bulk-change selected rows, rename a
   destination for all rows at once. A live preview shows the result.
4. Apply. Nothing is deleted or overwritten; the whole run can be undone.

**Optional local AI** (off by default)
- Suggests topic subfolders for documents, in the suggestions list and in the tidy plan.
- Runs fully on your computer with llama.cpp (GPU via Vulkan, CPU fallback). Document text is read in memory only.
- The settings page checks this computer (RAM, GPU/VRAM, CPU load, free disk), compares it with the requirements and
  gives advice. Downloading the model (≈1.2 GB, pinned revision, SHA-256 verified, resumable) is always the user's
  choice. The model refuses to load when less than 2 GB of RAM is available.

## Safety rules

- Suggestions only; every move needs a click.
- Never overwrites: a taken name becomes `name (1).ext`.
- Never deletes, except an explicit Delete, which goes to the Recycle Bin.
- Every batch of moves is logged and can be undone.
- Hidden/system files, code projects and dependency folders (`node_modules`, `target`, …) are left alone.

## Requirements

| | Minimum | Recommended |
|---|---|---|
| OS | Windows 10/11 x64 | Windows 11 |
| Without AI | any machine that runs Windows 10/11 | — |
| AI: RAM | 8 GB | 16 GB |
| AI: GPU | not required (CPU, slow) | Vulkan GPU with ≥ 3 GB VRAM |
| AI: disk | 2 GB free | 5 GB free |

## Build from source

Prerequisites: Rust (stable), WebView2 runtime (included in Windows 11), Visual Studio Build Tools (C++).

```powershell
cd src-tauri
cargo test              # self-checks: rules, plan → execute → undo, hardware advice
cargo run               # development build (without the AI engine)
cargo build --release   # src-tauri\target\release\file-organizer.exe
```

With the AI engine (additionally needs the Vulkan SDK, LLVM/libclang and CMake):

```powershell
$env:VULKAN_SDK = 'C:\VulkanSDK\<version>'
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
cargo build --release --features local-llm
```

An installer can be produced with the Tauri CLI: `cargo install tauri-cli --version "^2"` then `cargo tauri build`
(add `--features local-llm` for the AI engine).

The model is not part of the build or the repository; users download it (or pick an existing copy) in the app.

## Data and privacy

See [PRIVACY.md](PRIVACY.md). In short: everything stays on your computer in
`%LOCALAPPDATA%\file-organizer\`. The only network request is the model download, and only when you start it.

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — how the app is put together
- [CONTRIBUTING.md](CONTRIBUTING.md) — building, testing, pull requests
- [CHANGELOG.md](CHANGELOG.md) — release notes
- [SECURITY.md](SECURITY.md) — reporting vulnerabilities

## Model

Text model: Qwen3.5-2B, GGUF Q4_K_M, downloaded from Hugging Face at a pinned revision and verified by SHA-256 before
use. The model has its own license; check it before redistributing the model file.

## License

Copyright (C) 2026 Danu-Nur and contributors.

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public
License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later
version. It is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY. See [LICENSE](LICENSE).
