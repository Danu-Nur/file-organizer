# Contributing

Thanks for helping! Bug reports, ideas and pull requests are welcome.

## Getting started

```powershell
git clone <your fork>
cd file-organizer\src-tauri
cargo test
cargo run
```

The interface lives in `ui/` (plain HTML, CSS and JavaScript — no bundler, no npm). The backend is in
`src-tauri/src/`. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the map.

### Building with the AI engine

The `local-llm` feature compiles llama.cpp with Vulkan. It needs the Vulkan SDK, LLVM/libclang and CMake:

```powershell
$env:VULKAN_SDK = 'C:\VulkanSDK\<version>'
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
$env:CARGO_BUILD_JOBS = '4'          # optional: keep the machine responsive
cargo build --features local-llm --target-dir target-ai
```

The first build takes several minutes. Do not use llama.cpp's `native` CPU feature: binaries would only run on CPUs
like the build machine's.

## Ground rules for changes

- **Never lose a user's file.** Moves go through `ops::Log` (undoable, never overwrite). Deletion only via the
  Recycle Bin and only on an explicit user action.
- **Suggest, don't act.** Nothing moves without a click.
- **No file content is stored or sent anywhere.** The index keeps name, path, size, date and MD5.
- **AI stays optional.** Every feature must work without it; the model is never downloaded or loaded unless the
  user asked.
- Keep the UI consistent with the existing Windows 11 style (tokens in `ui/app.css`), keyboard-accessible, and in
  Indonesian.
- Keep dependencies minimal; prefer the standard library and what is already in `Cargo.toml`.

## Tests

`cargo test` runs:

- `rules` — categories, ignored files, version names
- `plan` — analyse → execute → undo on a throwaway folder (the layout must be restored exactly)
- `sysinfo` — hardware advice levels

Add a test for any new logic that moves files or decides where they go.

### Driving the UI in debug builds

Debug builds evaluate a script from `FO_DEBUG_SCRIPT` a few seconds after start, which is handy for screenshots and
manual checks without clicking:

```powershell
$env:FO_DEBUG_SCRIPT = "C:\path\to\tour.js"   # e.g. go(S.roots[1].path); S.mode = 'tree'; load();
cargo run
```

Never let such a script apply a plan or delete anything on a real folder.

## Pull requests

1. Open an issue first for larger changes.
2. Keep the change focused; describe what you tested (`cargo test`, and how you checked the UI).
3. By contributing you agree that your contribution is licensed under GPL-3.0-or-later.
