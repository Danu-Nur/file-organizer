# Privasi

[English](PRIVACY.md) · **Bahasa Indonesia**

File Organizer dirancang supaya semua data tetap di komputermu.

## Yang disimpan

Di `%LOCALAPPDATA%\file-organizer\index.db` (SQLite):

- untuk setiap file yang diindeks: nama, path lengkap, ukuran, waktu diubah, dan checksum MD5;
- keputusanmu (saran yang diabaikan), catatan urungkan untuk pemindahan, daftar folder yang dipantau, dan
  pengaturan.

**Isi file tidak pernah disimpan.** Preferensi tampilan (mode tampilan, lebar kolom, panel pratinjau) disimpan di
penyimpanan web lokal aplikasi.

## Yang dibaca

- Isi file dibaca untuk menghitung checksum MD5 (lalu dibuang).
- Panel pratinjau membaca file yang dipilih (dengan batas ukuran) untuk menampilkannya; tidak ada yang disimpan.
- File PHP dan gambar diperiksa sebentar untuk mencari kode mencurigakan saat kamu menganalisis folder.
- Jika AI opsional dinyalakan, ±2000 karakter pertama sebuah dokumen dibaca ke memori dan diberikan ke model lokal;
  tidak ada yang disimpan atau dikirim.
- Halaman pengaturan membaca RAM, beban CPU, GPU, dan ruang disk komputer ini untuk memberi saran; datanya hanya
  tampil di layar.

## Jaringan

Aplikasi ini **tidak melakukan permintaan jaringan apa pun**, dengan satu pengecualian: mengunduh model AI opsional
dari Hugging Face, dan itu pun hanya setelah kamu menekan "Unduh". Tidak ada telemetri, analitik, maupun akun.

## Menghapus semua data

Tutup aplikasi, lalu hapus folder `%LOCALAPPDATA%\file-organizer\` (indeks, pengaturan, model yang diunduh).
