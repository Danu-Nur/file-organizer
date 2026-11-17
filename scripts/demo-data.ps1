# SPDX-License-Identifier: GPL-3.0-or-later
# Creates a messy sample folder tree with made-up files, for screenshots and manual testing.
# Nothing outside -Target is touched. Large files are created with SetLength (fast, no real data written).
param([string]$Target = (Join-Path $env:TEMP 'FileOrganizerDemo'))

$ErrorActionPreference = 'Stop'
if (Test-Path $Target) { Remove-Item $Target -Recurse -Force }
Add-Type -AssemblyName System.Drawing

function New-Dir($p) { New-Item -ItemType Directory -Force $p | Out-Null; $p }
function Set-Age($p, [int]$daysAgo) { $t = (Get-Date).AddDays(-$daysAgo); (Get-Item $p).LastWriteTime = $t }
function New-Blob($p, [long]$bytes) { $f = [IO.File]::Create($p); $f.SetLength($bytes); $f.Close() }
function New-Text($p, $text) { [IO.File]::WriteAllText($p, $text, (New-Object Text.UTF8Encoding $false)) }

function New-Image($p, [int]$seed) {
  $r = New-Object Random $seed
  $b = New-Object Drawing.Bitmap 960, 640
  $g = [Drawing.Graphics]::FromImage($b); $g.SmoothingMode = 'AntiAlias'
  $sky = [Drawing.Color]::FromArgb(255, $r.Next(20, 90), $r.Next(90, 160), $r.Next(150, 220))
  $g.Clear($sky)
  $g.FillEllipse([Drawing.Brushes]::Gold, $r.Next(500, 760), $r.Next(40, 160), 150, 150)
  $hill = New-Object Drawing.SolidBrush ([Drawing.Color]::FromArgb(255, $r.Next(30, 80), $r.Next(110, 170), $r.Next(60, 100)))
  $g.FillPolygon($hill, @((New-Object Drawing.Point 0, 640), (New-Object Drawing.Point $r.Next(200, 420), $r.Next(260, 380)), (New-Object Drawing.Point 960, 640)))
  $b.Save($p, [Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $b.Dispose()
}

function New-Pdf($p, $title) {
  $content = "BT /F1 26 Tf 50 300 Td ($title) Tj ET BT /F1 12 Tf 50 270 Td (Contoh dokumen - File Organizer demo) Tj ET"
  $objs = @(
    '<</Type/Catalog/Pages 2 0 R>>', '<</Type/Pages/Kids[3 0 R]/Count 1>>',
    '<</Type/Page/Parent 2 0 R/MediaBox[0 0 420 400]/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>',
    "<</Length $($content.Length)>>`nstream`n$content`nendstream", '<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>')
  $sb = New-Object Text.StringBuilder; [void]$sb.Append("%PDF-1.4`n"); $offs = @()
  for ($i = 0; $i -lt $objs.Count; $i++) { $offs += $sb.Length; [void]$sb.Append("$($i + 1) 0 obj`n$($objs[$i])`nendobj`n") }
  $x = $sb.Length; [void]$sb.Append("xref`n0 $($objs.Count + 1)`n0000000000 65535 f `n")
  foreach ($o in $offs) { [void]$sb.Append(('{0:D10} 00000 n ' -f $o) + "`n") }
  [void]$sb.Append("trailer`n<</Size $($objs.Count + 1)/Root 1 0 R>>`nstartxref`n$x`n%%EOF`n")
  [IO.File]::WriteAllText($p, $sb.ToString(), [Text.Encoding]::ASCII)
}

$dl = New-Dir (Join-Path $Target 'Unduhan')
$doc = New-Dir (Join-Path $Target 'Dokumen')
$pic = New-Dir (Join-Path $Target 'Gambar')

# --- Unduhan: a typical messy downloads folder
New-Pdf "$dl\invoice-2026-03.pdf" 'Invoice Maret 2026'
Copy-Item "$dl\invoice-2026-03.pdf" "$dl\invoice-2026-03 (1).pdf"          # exact duplicate
New-Pdf "$dl\laporan-tahunan-2025.pdf" 'Laporan Tahunan 2025'
New-Image "$dl\foto-liburan-01.png" 1; New-Image "$dl\foto-liburan-02.png" 2
New-Text "$dl\catatan-rapat.md" "# Catatan rapat`n`n- Anggaran Q1 disetujui`n- Peluncuran: Desember`n- PIC: tim produk`n"
New-Text "$dl\catatan-rapat (1).md" "# Catatan rapat (draf lama)`n`n- Anggaran masih dibahas`n"; Set-Age "$dl\catatan-rapat (1).md" 40
New-Text "$dl\rekap-penjualan.csv" "bulan,unit,omzet`nJan,120,36000000`nFeb,98,29400000`nMar,143,42900000`n"
New-Text "$dl\recovery-codes.txt" "contoh-kode-1`ncontoh-kode-2`n"
New-Text "$dl\kosong.txt" ''
New-Blob "$dl\setup-editor-teks.exe" 48MB
New-Blob "$dl\video-presentasi.mp4" 210MB
New-Blob "$dl\arsip-proyek-lama.zip" 160MB; Set-Age "$dl\arsip-proyek-lama.zip" 400
New-Blob "$dl\podcast-episode-12.mp3" 38MB
New-Blob "$dl\model-gantungan.stl" 12MB
New-Blob "$dl\kontak-tim.vcf" 4KB
$bali = New-Dir "$dl\Foto Bali"; 1..6 | % { New-Image "$bali\IMG_20$(20 + $_)_0$_.png" (10 + $_) }
$app = New-Dir "$dl\aplikasi-portable"; New-Blob "$app\alat.exe" 22MB; New-Text "$app\baca-saya.txt" 'Aplikasi portable contoh.'
$web = New-Dir "$dl\proyek-web"; New-Text "$web\package.json" '{ "name": "proyek-web" }'; New-Text "$web\index.html" '<h1>Halo</h1>'
New-Dir "$dl\folder-kosong" | Out-Null
Get-ChildItem $dl -File | ? { $_.Name -notmatch 'arsip-proyek|\(1\)\.md' } | % { Set-Age $_.FullName (Get-Random -Min 0 -Max 120) }

# --- Dokumen: years of documents (for the file-age chart)
$kantor = New-Dir "$doc\Kantor"; $pribadi = New-Dir "$doc\Pribadi"
$years = 2019..2026
foreach ($y in $years) {
  $f = "$kantor\laporan-$y.pdf"; New-Pdf $f "Laporan $y"; (Get-Item $f).LastWriteTime = Get-Date -Year $y -Month 6 -Day 15
  $z = "$kantor\backup-$y.zip"; New-Blob $z ((($y - 2018) * 9) * 1MB); (Get-Item $z).LastWriteTime = Get-Date -Year $y -Month 9 -Day 1
}
New-Pdf "$pribadi\ijazah-scan.pdf" 'Ijazah (contoh)'; New-Pdf "$pribadi\kontrak-sewa.pdf" 'Kontrak sewa (contoh)'
New-Text "$doc\ide-aplikasi.md" "# Ide`n`n- Aplikasi pencatat keuangan`n"
New-Blob "$doc\anggaran-2026.xlsx" 2MB

# --- Gambar: photos, including a copy that also sits in Unduhan
1..14 | % { New-Image "$pic\foto-$('{0:D3}' -f $_).png" (100 + $_) }
Copy-Item "$dl\foto-liburan-01.png" "$pic\liburan-favorit.png"

"Demo data created in $Target"
