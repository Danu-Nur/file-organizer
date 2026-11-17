# Berkontribusi

[English](CONTRIBUTING.md) · **Bahasa Indonesia**

Terima kasih sudah membantu! Laporan bug, ide, dan pull request sangat diterima.

## Memulai

```powershell
git clone <fork-mu>
cd file-organizer\src-tauri
cargo test
cargo run
```

Antarmuka ada di `ui/` (HTML, CSS, dan JavaScript biasa — tanpa bundler, tanpa npm). Backend ada di
`src-tauri/src/`. Lihat [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) untuk petanya.

### Build dengan mesin AI

Fitur `local-llm` meng-compile llama.cpp dengan Vulkan. Butuh Vulkan SDK, LLVM/libclang, dan CMake:

```powershell
$env:VULKAN_SDK = 'C:\VulkanSDK\<versi>'
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
$env:CARGO_BUILD_JOBS = '4'          # opsional: supaya komputer tetap responsif
cargo build --features local-llm --target-dir target-ai
```

Build pertama memakan beberapa menit. Jangan aktifkan fitur CPU `native` dari llama.cpp: hasilnya hanya bisa jalan di
CPU yang mirip dengan mesin build.

## Aturan dasar perubahan

- **Jangan pernah menghilangkan file pengguna.** Pemindahan lewat `ops::Log` (bisa diurungkan, tidak pernah menimpa).
  Penghapusan hanya ke Recycle Bin dan hanya atas tindakan pengguna; hapus permanen selalu minta konfirmasi.
- **Menyarankan, bukan bertindak.** Tidak ada yang dipindah tanpa diklik.
- **Isi file tidak disimpan atau dikirim ke mana pun.** Indeks hanya menyimpan nama, path, ukuran, tanggal, dan MD5.
- **AI tetap opsional.** Setiap fitur harus jalan tanpa AI; model tidak diunduh atau dimuat kecuali pengguna
  memintanya.
- **File tidak tepercaya, pembacaan dibatasi.** Semua yang dibaca untuk pratinjau atau AI dibatasi ukurannya.
- Jaga UI tetap konsisten dengan gaya Windows 11 yang ada (token di `ui/app.css`), bisa dipakai dengan keyboard, dan
  berbahasa Indonesia.
- Minimalkan dependensi; utamakan library standar dan yang sudah ada di `Cargo.toml`.

## Tes

`cargo test` menjalankan:

- `rules` — kategori, file yang diabaikan, nama versi
- `plan` — analisis → eksekusi → urungkan di folder sementara (susunan harus kembali persis)
- `preview` — pembacaan yang dibatasi
- `sysinfo` — tingkat saran hardware
- prefix path untuk akar drive, perhitungan tahun

Tambahkan tes untuk setiap logika baru yang memindahkan file atau menentukan tujuannya.

## Saklar untuk pengembang

| Variabel | Efek |
|---|---|
| `FO_DATA_DIR` | Pakai folder lain untuk indeks dan pengaturan (data demo, tes, mode portable). |
| `FO_ROOTS` | Pantau persis folder-folder ini (dipisah `;`) sejak pemindaian pertama, mengabaikan daftar tersimpan. |
| `FO_DEBUG_SCRIPT` | Hanya build debug: jalankan file JavaScript ini di halaman ±3 detik setelah start. |

### Data demo dan tangkapan layar

```powershell
.\scripts\demo-data.ps1                  # folder contoh berantakan berisi file karangan (di %TEMP%)
cd src-tauri; cargo build; cd ..
.\scripts\screenshots.ps1                # membuat ulang docs/screenshots/*.png
```

`screenshots.ps1` menjalankan build debug dengan `FO_DATA_DIR` (indeks sementara), `FO_ROOTS` (hanya folder demo),
dan `FO_DEBUG_SCRIPT` (`scripts/screenshot-tour.js`), lalu merekam jendela dengan `PrintWindow`, sehingga tetap
berhasil walau tertutup jendela lain. Jangan pernah mengambil tangkapan layar dari folder pribadi.

## Pull request

1. Buka issue dulu untuk perubahan yang besar.
2. Jaga perubahan tetap fokus; jelaskan apa yang sudah diuji (`cargo test`, dan cara mengecek UI-nya).
3. Dengan berkontribusi, kamu setuju kontribusimu dilisensikan di bawah GPL-3.0-or-later.
