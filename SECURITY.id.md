# Kebijakan keamanan

[English](SECURITY.md) · **Bahasa Indonesia**

## Versi yang didukung

Hanya rilis terbaru yang menerima perbaikan.

## Melaporkan celah keamanan

Mohon **jangan membuka issue publik** untuk masalah keamanan. Gunakan fitur
[private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
GitHub di repository ini (tab "Security" → "Report a vulnerability").

Sertakan apa yang kamu temukan, cara memunculkannya, dan dampaknya. Laporan akan dibalas secepatnya; perbaikan dirilis
bersama advisory publik setelah pengguna bisa memperbarui aplikasinya.

## Cakupan

- Aplikasi berjalan lokal dan hanya memindahkan file setelah disetujui pengguna. Masalah ketika nama file atau
  susunan folder yang dibuat khusus membuat aplikasi memindahkan, menimpa, atau menghapus sesuatu tanpa persetujuan
  pengguna termasuk cakupan dan berprioritas tinggi.
- Pratinjau dan AI membaca file yang tidak tepercaya; cara menghabiskan memori atau menjalankan kode lewat file yang
  dibuat khusus termasuk cakupan.
- Unduhan model dikunci ke satu revisi dan dicek dengan SHA-256; cara mengakali pengecekan itu termasuk cakupan.
- Deteksi PHP mencurigakan untuk `_Karantina` hanyalah heuristik pembantu, bukan antivirus.
