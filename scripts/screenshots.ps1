# SPDX-License-Identifier: GPL-3.0-or-later
# Regenerates docs/screenshots/*.png from the demo data, without touching your real index or folders:
#   - the app runs with FO_DATA_DIR pointing at a throwaway index,
#   - scripts/screenshot-tour.js drives the UI (debug builds only),
#   - each window is captured with PrintWindow (works even if other windows cover it).
# Usage (from the repo root):  cd src-tauri; cargo build; cd ..; .\scripts\screenshots.ps1
param(
  [string]$Exe = (Join-Path $PSScriptRoot '..\src-tauri\target\debug\file-organizer.exe'),
  [string]$Demo = (Join-Path $env:TEMP 'FileOrganizerDemo'),
  [string]$Out = (Join-Path $PSScriptRoot '..\docs\screenshots')
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path $Exe)) { throw "Build the debug app first: cd src-tauri; cargo build" }
if (-not (Test-Path $Demo)) { & (Join-Path $PSScriptRoot 'demo-data.ps1') -Target $Demo }

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public class FoCapture {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
  public struct R { public int L, T, Rt, B; }
}
'@

$data = Join-Path $env:TEMP 'FileOrganizerDemoIndex'
if (Test-Path $data) { Remove-Item $data -Recurse -Force }
$tour = Join-Path $PSScriptRoot 'screenshot-tour.js'
New-Item -ItemType Directory -Force $Out | Out-Null

Get-Process file-organizer -ErrorAction SilentlyContinue | Stop-Process
# Only the demo folders are watched from the first scan on, in a throwaway index: no real file can appear.
$env:FO_DATA_DIR = $data
$env:FO_ROOTS = (@('Unduhan', 'Dokumen', 'Gambar') | % { Join-Path $Demo $_ }) -join ';'
$env:FO_DEBUG_SCRIPT = $tour
$log = Join-Path $env:TEMP 'fo-screenshots-stderr.txt'
$p = Start-Process $Exe -PassThru -RedirectStandardError $log -RedirectStandardOutput (Join-Path $env:TEMP 'fo-screenshots-stdout.txt')
$start = Get-Date
# seconds after start (the tour starts ~3.5 s after launch) → file name
$shots = [ordered]@{ 20.5 = 'dashboard'; 28.5 = 'saran'; 35.5 = 'jelajah-pratinjau'; 44.5 = 'rencana'; 50.5 = 'pengaturan' }
foreach ($at in $shots.Keys) {
  while (((Get-Date) - $start).TotalSeconds -lt $at) { Start-Sleep -Milliseconds 200 }
  $h = [IntPtr]::Zero
  for ($i = 0; $i -lt 25 -and $h -eq [IntPtr]::Zero; $i++) {
    $p.Refresh()
    if ($p.HasExited) { throw "The app exited (code $($p.ExitCode)). Its error output: $(Get-Content $log -Tail 20 | Out-String)" }
    $h = $p.MainWindowHandle
    if ($h -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 200 }
  }
  if ($h -eq [IntPtr]::Zero) { throw 'The app window was not found.' }
  $r = New-Object FoCapture+R; [FoCapture]::GetWindowRect($h, [ref]$r) | Out-Null
  $b = New-Object Drawing.Bitmap ($r.Rt - $r.L), ($r.B - $r.T)
  $g = [Drawing.Graphics]::FromImage($b); $dc = $g.GetHdc(); [FoCapture]::PrintWindow($h, $dc, 2) | Out-Null; $g.ReleaseHdc($dc); $g.Dispose()
  $file = Join-Path $Out "$($shots[$at]).png"; $b.Save($file, [Drawing.Imaging.ImageFormat]::Png); $b.Dispose()
  "saved $file"
}
Stop-Process -Id $p.Id
Remove-Item Env:FO_DATA_DIR, Env:FO_ROOTS, Env:FO_DEBUG_SCRIPT
Start-Sleep -Milliseconds 500
Remove-Item $data -Recurse -Force -ErrorAction SilentlyContinue
