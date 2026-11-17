// SPDX-License-Identifier: GPL-3.0-or-later
// Evaluated in a DEBUG build via FO_DEBUG_SCRIPT (see scripts/screenshots.ps1). The app was started with
// FO_ROOTS = the demo folders and FO_DATA_DIR = a throwaway index, so only demo files exist in this session.
// Never applies a plan or deletes anything.
(async () => {
  const t0 = performance.now();
  const until = (s) => new Promise((r) => setTimeout(r, Math.max(0, t0 + s * 1000 - performance.now())));
  const pick = (name) => select(S.vis.findIndex((r) => r.name === name));

  S.mode = 'list';
  setPreview(true);
  S.roots = await invoke('list_roots');
  renderNav();

  await until(11); D.mode = {}; openDash();                                  // shot: dashboard (t+17)
  await until(18); setView('suggest'); await until(21); pick('invoice-2026-03 (1).pdf'); // shot: saran (t+25)
  await until(26); go(S.roots[0].path); await until(29); pick('foto-liburan-01.png');  // shot: jelajah (t+32)
  await until(33); openTidy(S.roots[0].path); await until(35); tidyCmd('analyze');   // shot: rencana (t+41)
  await until(42); openSettings();                                           // shot: pengaturan (t+47)
})();
