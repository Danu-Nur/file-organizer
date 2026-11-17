# Screenshots

Generated from demo data with made-up file names — never from real personal folders.

| File | Shows |
|---|---|
| `dashboard.png` | Dashboard: headline figures, file types, watched folders |
| `saran.png` | Suggestions with the preview pane |
| `jelajah-pratinjau.png` | Browsing a folder with an image preview |
| `rencana.png` | A "Rapikan folder" plan before applying |
| `pengaturan.png` | Settings: optional local AI with the hardware check |

Regenerate (from the repo root):

```powershell
cd src-tauri; cargo build; cd ..
.\scripts\screenshots.ps1        # add -Exe <path> to use another debug build, e.g. one with the AI engine
```

See [CONTRIBUTING.md](../../CONTRIBUTING.md#demo-data-and-screenshots) for how it works.
