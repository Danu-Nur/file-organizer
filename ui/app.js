// SPDX-License-Identifier: GPL-3.0-or-later
'use strict';
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const store = {
  get(k, d) { try { const v = localStorage.getItem(k); return v === null ? d : JSON.parse(v); } catch { return d; } },
  set(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch { /* private mode: not remembered */ } },
};

// ================================================================ icons (Fluent-style 20px line icons)
const I = {
  sparkle: '<path d="M10 2.5l1.7 4.3 4.3 1.7-4.3 1.7L10 14.5l-1.7-4.3L4 8.5l4.3-1.7z"/><path d="M15.5 13.5l.6 1.4 1.4.6-1.4.6-.6 1.4-.6-1.4-1.4-.6 1.4-.6z"/>',
  dup: '<rect x="6.5" y="6.5" width="10" height="11" rx="1.5"/><path d="M13.5 6.5V4A1.5 1.5 0 0 0 12 2.5H5A1.5 1.5 0 0 0 3.5 4v9A1.5 1.5 0 0 0 5 14.5h1.5"/>',
  all: '<rect x="3" y="3" width="14" height="14" rx="2"/><path d="M6.5 7.5h7M6.5 10h7M6.5 12.5h4"/>',
  Desktop: '<rect x="2.5" y="3.5" width="15" height="10" rx="1.5"/><path d="M7 16.5h6M10 13.5v3"/>',
  Downloads: '<path d="M10 3v10m-4.5-4.5L10 13l4.5-4.5M4 16.5h12"/>',
  Documents: '<path d="M5.5 2.5h6l3 3v12h-9z"/><path d="M11.5 2.5v3h3M7.5 10h5M7.5 13h5"/>',
  Pictures: '<rect x="2.5" y="4" width="15" height="12" rx="1.5"/><circle cx="7" cy="8" r="1.3"/><path d="M3 15l4.5-4.5 3 3 2-2 4.5 4"/>',
  Music: '<path d="M7.5 15V4.5l9-2V13"/><circle cx="5.5" cy="15" r="2"/><circle cx="14.5" cy="13" r="2"/>',
  Videos: '<rect x="2.5" y="4.5" width="11" height="11" rx="1.5"/><path d="M13.5 8.5l4-2.5v8l-4-2.5"/>',
  move: '<path d="M3 10h10.5M10 6l4 4-4 4M17 4v12"/>',
  hide: '<path d="M3 3l14 14M8.4 5.2A7.6 7.6 0 0 1 10 5c4.5 0 7.5 5 7.5 5a13 13 0 0 1-2.2 2.8M12.6 14.6A6.8 6.8 0 0 1 10 15c-4.5 0-7.5-5-7.5-5a13 13 0 0 1 3-3.4"/>',
  open: '<path d="M11 3.5h5.5V9M16.5 3.5L9 11M14 12v3.5a1 1 0 0 1-1 1H4.5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1H8"/>',
  folder: '<path d="M2.5 5.5A1.5 1.5 0 0 1 4 4h3.5l2 2H16a1.5 1.5 0 0 1 1.5 1.5V15a1.5 1.5 0 0 1-1.5 1.5H4A1.5 1.5 0 0 1 2.5 15z"/>',
  folderAdd: '<path d="M2.5 5.5A1.5 1.5 0 0 1 4 4h3.5l2 2H16a1.5 1.5 0 0 1 1.5 1.5V15a1.5 1.5 0 0 1-1.5 1.5H4A1.5 1.5 0 0 1 2.5 15z"/><path d="M10 9v5M7.5 11.5h5"/>',
  link: '<path d="M8 12l4-4M9 5.5l1.2-1.2a3.2 3.2 0 0 1 4.5 4.5L13.5 10M11 14.5l-1.2 1.2a3.2 3.2 0 0 1-4.5-4.5L6.5 10"/>',
  hash: '<path d="M7.5 3L6 17M14 3l-1.5 14M3.5 7.5h13M3 12.5h13"/>',
  search: '<circle cx="8.5" cy="8.5" r="5"/><path d="M12.5 12.5L17 17"/>',
  check: '<circle cx="10" cy="10" r="7.5"/><path d="M6.8 10.2l2.2 2.2 4.2-4.6"/>',
  error: '<circle cx="10" cy="10" r="7.5"/><path d="M10 6v4.5M10 13.5v.1"/>',
  cut: '<circle cx="5.5" cy="14.5" r="2.5"/><circle cx="14.5" cy="14.5" r="2.5"/><path d="M7.3 12.7L15 3M12.7 12.7L5 3"/>',
  copy: '<rect x="6.5" y="6.5" width="10" height="11" rx="1.5"/><path d="M13.5 6.5V4A1.5 1.5 0 0 0 12 2.5H5A1.5 1.5 0 0 0 3.5 4v9A1.5 1.5 0 0 0 5 14.5h1.5"/>',
  paste: '<rect x="4.5" y="4" width="11" height="13.5" rx="1.5"/><path d="M7.5 4V3a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1v1M7.5 9h5M7.5 12h5"/>',
  rename: '<path d="M3.5 14v2.5H6l9-9L12.5 5zM11 6.5L13.5 9"/>',
  trash: '<path d="M3.5 5.5h13M8 5.5v-2h4v2M5 5.5l.8 11a1 1 0 0 0 1 .9h6.4a1 1 0 0 0 1-.9l.8-11M8.5 8.5v6M11.5 8.5v6"/>',
  back: '<path d="M16 10H4m5-5l-5 5 5 5"/>',
  fwd: '<path d="M4 10h12m-5-5l5 5-5 5"/>',
  up: '<path d="M10 16V4M5 9l5-5 5 5"/>',
  refresh: '<path d="M16 10a6 6 0 1 1-1.8-4.3M16 3.5V7h-3.5"/>',
  chev: '<path d="M7.5 5l5 5-5 5"/>',
  listv: '<path d="M7 5.5h10M7 10h10M7 14.5h10M3.5 5.5h.5M3.5 10h.5M3.5 14.5h.5"/>',
  treev: '<path d="M4 3.5v11.5a1 1 0 0 0 1 1h3M4 9.5h4"/><rect x="9" y="7.5" width="8" height="4" rx="1"/><rect x="9" y="13.5" width="8" height="4" rx="1"/><rect x="2" y="1.5" width="6" height="3" rx="1"/>',
  undo: '<path d="M7.5 5.5L4 9l3.5 3.5M4 9h8.5a4 4 0 0 1 0 8H9"/>',
  warn: '<path d="M10 3L2.5 16.5h15z"/><path d="M10 8v4M10 14.5v.1"/>',
  gear: '<circle cx="10" cy="10" r="2.5"/><path d="M8.6 2.5h2.8l.4 2 1.6.9 1.9-.7 1.4 2.4-1.5 1.4v1.9l1.5 1.4-1.4 2.4-1.9-.7-1.6.9-.4 2H8.6l-.4-2-1.6-.9-1.9.7-1.4-2.4 1.5-1.4V9.1L3.3 7.7l1.4-2.4 1.9.7 1.6-.9z"/>',
  info: '<circle cx="10" cy="10" r="7.5"/><path d="M10 9v5M10 6.5v.1"/>',
  plus: '<path d="M10 4v12M4 10h12"/>',
  down: '<path d="M10 4v12M5 11l5 5 5-5"/>',
  remove: '<path d="M4 10h12"/>',
  home: '<path d="M3 9l7-6 7 6M5 7.5V17h10V7.5M8.5 17v-5h3v5"/>',
  cloud: '<path d="M6 15.5h8.5a3.5 3.5 0 0 0 .4-7 5 5 0 0 0-9.6 1.2A2.9 2.9 0 0 0 6 15.5z"/>',
  drive: '<rect x="2.5" y="6" width="15" height="8" rx="1.5"/><path d="M5.5 10h5M14 10h.5"/>',
  panel: '<rect x="2.5" y="3.5" width="15" height="13" rx="1.5"/><path d="M11.5 3.5v13"/>',
  eye: '<path d="M2.5 10s3-5.5 7.5-5.5S17.5 10 17.5 10s-3 5.5-7.5 5.5S2.5 10 2.5 10z"/><circle cx="10" cy="10" r="2.3"/>',
  close: '<path d="M5 5l10 10M15 5L5 15"/>',
  chevDown: '<path d="M6 8l4 4 4-4"/>',
  tick: '<path d="M4.5 10.5l3.5 3.5 7.5-8"/>',
  none: '',
  restore: '<path d="M4 10a6 6 0 1 0 1.8-4.3M4 3.5V7h3.5M10 7v3.5l2.5 1.5"/>',
  chart: '<path d="M3.5 16.5h13"/><rect x="5" y="9" width="2.5" height="5.5" rx="1"/><rect x="9" y="5" width="2.5" height="9.5" rx="1"/><rect x="13" y="11" width="2.5" height="3.5" rx="1"/>',
  bars: '<path d="M4 5.5h9M4 10h12M4 14.5h6"/>',
};
/** Navigation icon and colour per kind of watched place. */
const KIND_ICON = { Desktop: 'Desktop', Downloads: 'Downloads', Documents: 'Documents', Pictures: 'Pictures', Music: 'Music', Videos: 'Videos', Home: 'home', OneDrive: 'cloud', Drive: 'drive', Folder: 'folder' };
const icon = (n) => `<svg class="ic" viewBox="0 0 20 20" aria-hidden="true">${I[n] || ''}</svg>`;
$$('[data-icon]').forEach((el) => (el.outerHTML = icon(el.dataset.icon)));

const ROOT_COLOR = { Desktop: '#3a96dd', Downloads: '#13a10e', Documents: '#7a8fa6', Pictures: '#0078d4', Music: '#f7630c', Videos: '#8764b8', Home: '#e8a33c', OneDrive: '#0f6cbd', Drive: '#8a8a8a', Folder: '#e8a33c' };
const CAT = {
  Folder: ['Folder', '#e8a33c'], Gambar: ['Gambar', '#0f6cbd'], Dokumen: ['Dokumen', '#2b579a'], Spreadsheet: ['Spreadsheet', '#217346'],
  Arsip: ['Arsip', '#c19c00'], Program: ['Program', '#5c2d91'], Kode: ['Kode', '#6e6e6e'], 'Model 3D': ['Model 3D', '#ca5010'],
  Musik: ['Musik', '#c239b3'], Video: ['Video', '#8764b8'], Kontak: ['Kontak', '#038387'], Lainnya: ['Lainnya', '#8a8a8a'],
  _Sensitif: ['Sensitif', '#c42b1c'], _Duplikat: ['Duplikat', '#da7b11'], _Karantina: ['Karantina', '#c42b1c'], _Kosong: ['Kosong', '#8a8a8a'],
  '_Versi Lama': ['Versi lama', '#8764b8'], Proyek: ['Proyek', '#038387'],
};
const LEGACY = { Documents: 'Dokumen', Images: 'Gambar', Sheets: 'Spreadsheet', Compressed: 'Arsip', Programs: 'Program', Code: 'Kode', '3D': 'Model 3D', Music: 'Musik', Contacts: 'Kontak', Misc: 'Lainnya', _Sensitive: '_Sensitif', Projects: 'Proyek' };
const catColor = (dest) => { const top = (dest || '').split('\\')[0]; return (CAT[top] || CAT[LEGACY[top]] || CAT.Lainnya)[1]; };
const folderIcon = '<svg class="fi" viewBox="0 0 18 18" aria-hidden="true"><path d="M1.5 4A1.5 1.5 0 0 1 3 2.5h4l1.5 1.5H15A1.5 1.5 0 0 1 16.5 5.5V14A1.5 1.5 0 0 1 15 15.5H3A1.5 1.5 0 0 1 1.5 14z" fill="var(--folder-back)"/><path d="M1.5 7A1.5 1.5 0 0 1 3 5.5h12A1.5 1.5 0 0 1 16.5 7v7a1.5 1.5 0 0 1-1.5 1.5H3A1.5 1.5 0 0 1 1.5 14z" fill="var(--folder-front)"/></svg>';
const fileIcon = (cat) => cat === 'Folder' ? folderIcon
  : `<svg class="fi" viewBox="0 0 18 18" aria-hidden="true"><path d="M4 1.5h6.5l3.5 3.5v11.5H4z" fill="var(--card)" stroke="var(--text3)"/><path d="M10.5 1.5V5H14" fill="none" stroke="var(--text3)"/><rect x="2" y="9.5" width="9" height="5" rx="1" fill="${(CAT[cat] || CAT.Lainnya)[1]}"/></svg>`;

// ================================================================ helpers
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const nf = new Intl.NumberFormat('id-ID');
const df = new Intl.DateTimeFormat('id-ID', { day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit' });
const kb = (n) => `${nf.format(Math.ceil(n / 1024))} KB`; // Explorer shows every size in KB
const human = (n) => (n < 1048576 ? kb(n) : n < 1073741824 ? `${(n / 1048576).toLocaleString('id-ID', { maximumFractionDigits: 1 })} MB` : `${(n / 1073741824).toLocaleString('id-ID', { maximumFractionDigits: 2 })} GB`);
const parentOf = (p) => { const i = p.lastIndexOf('\\'); if (i < 0) return p; const d = p.slice(0, i); return d.endsWith(':') ? d + '\\' : d; };
const baseName = (p) => p.slice(p.lastIndexOf('\\') + 1) || p;
const withSep = (p) => (p.endsWith('\\') ? p : p + '\\'); // "C:\" already ends with one
const rootOf = (p) => S.roots.find((r) => p === r.path || p.startsWith(withSep(r.path)));
const cmpText = new Intl.Collator('id-ID', { numeric: true, sensitivity: 'base' }).compare;
/** "C:\Users\x\Downloads\Gambar" → "Downloads › Gambar" */
function pretty(dir) {
  const r = rootOf(dir);
  if (!r) return dir;
  return [r.name, ...dir.slice(r.path.length).split('\\').filter(Boolean)].join(' › ');
}
const badPart = (s) => !s || /[<>:"/|?*\x00-\x1f]/.test(s) || /[ .]$/.test(s) || s === '.' || s === '..';
const badTarget = (t) => !t.trim() || t.split(/[\\/]/).some(badPart);

// ================================================================ state
const S = {
  roots: [], view: 'suggest', cwd: null, hist: [], hi: -1,
  mode: store.get('mode', 'list'),
  sorts: { db: { sort: 'mtime', desc: true }, browse: { sort: 'name', desc: false } },
  q: '', rows: [], vis: [], sel: new Set(), anchor: -1, focus: -1,
  counts: { all: 0, suggest: 0, dup: 0, scanning: true },
  clip: null, undo: [], kids: new Map(), open: new Set(), closed: new Set(), renaming: null, menuOpen: false,
  pane: 'explorer', sizes: new Map(), drag: null, dragStart: null, pendingSelect: null,
  showHidden: store.get('hidden', false),
};
const isBrowse = () => S.view === 'browse';
const browsing = () => isBrowse() && !S.q; // folder content straight from disk
const sortOf = () => S.sorts[browsing() ? 'browse' : 'db'];

// ================================================================ columns (resizable, widths remembered)
const COLS = [
  { k: 'name', label: 'Nama', w: 320, sort: 'name' },
  { k: 'suggest', label: 'Saran', w: 260, show: () => S.view === 'suggest' || (isBrowse() && S.vis.some((r) => r.suggest)) },
  { k: 'loc', label: 'Lokasi', w: 220, sort: 'path', show: () => !browsing() && S.mode === 'list' && S.view !== 'bin' },
  { k: 'mtime', label: 'Diubah', w: 140, sort: 'mtime' },
  { k: 'cat', label: 'Jenis', w: 100, show: () => S.view !== 'suggest' },
  // Recycle Bin: where the item was before it was deleted
  { k: 'origin', label: 'Lokasi asal', w: 260, sort: 'path', show: () => S.view === 'bin' },
  { k: 'size', label: 'Ukuran', w: 104, sort: 'size', num: true },
  { k: 'md5', label: 'MD5', w: 110, sort: 'md5', show: () => S.view !== 'suggest' && S.view !== 'bin' },
];
const widths = store.get('widths', {});
const cols = () => COLS.filter((c) => !c.show || c.show());
const colW = (c) => widths[c.k] || c.w;

function cell(c, r) {
  switch (c.k) {
    case 'name': {
      const tw = S.mode === 'tree' && r.is_dir
        ? `<button class="twisty${r.expanded ? ' open' : ''}${r.has_children === false ? ' none' : ''}" data-twisty tabindex="-1" aria-label="${r.expanded ? 'Tutup' : 'Buka'}">${icon('chev')}</button>`
        : S.mode === 'tree' ? '<span class="twisty none"></span>' : '';
      const pad = S.mode === 'tree' ? ` style="padding-left:${r.depth * 20}px"` : '';
      const cnt = r.synthetic ? ` <span class="count">${nf.format(r.count)}</span>` : '';
      return `<td title="${esc(r.path)}"><div class="name"${pad}>${tw}${fileIcon(r.cat)}<span class="nm">${esc(r.name)}</span>${cnt}</div></td>`;
    }
    case 'suggest': {
      if (!r.suggest) return '<td></td>';
      return `<td title="${esc(r.reason || '')}"><span class="chip"><span class="dot" style="background:${catColor(r.suggest)}"></span>${esc(r.suggest.replaceAll('\\', ' › '))}</span><span class="reason">${esc(r.reason || '')}</span></td>`;
    }
    case 'loc': return `<td title="${esc(parentOf(r.path))}">${esc(pretty(parentOf(r.path)))}</td>`;
    case 'origin': return `<td title="${esc(r.origin)}">${esc(pretty(parentOf(r.origin || '')))}</td>`;
    case 'mtime': return `<td>${r.mtime ? esc(df.format(new Date(r.mtime * 1000))) : ''}</td>`;
    case 'cat': return `<td>${r.synthetic ? 'Folder' : esc((CAT[r.cat] || CAT.Lainnya)[0])}</td>`;
    case 'size': {
      if (!r.is_dir) return `<td class="num sz">${kb(r.size)}</td>`;
      if (r.bin) return '<td class="num sz"></td>'; // folders in the Recycle Bin are not measured
      const z = S.sizes.get(r.path);
      if (r.synthetic) return '<td class="num sz"></td>';
      return `<td class="num sz"${z ? ` title="${nf.format(z.files)} file"` : ' title="Menghitung ukuran…"'}>${z ? human(z.size) : '<span class="muted">…</span>'}</td>`;
    }
    case 'md5': return `<td class="mono" title="${esc(r.md5 || '')}">${esc((r.md5 || '').slice(0, 10))}</td>`;
  }
}

// ================================================================ data
let req = 0;
async function load() {
  if (S.renaming) return; // don't wipe an open rename box
  const id = ++req;
  const counts = invoke('counts').catch(() => S.counts);
  try {
    let rows;
    if (S.view === 'bin') {
      const { sort, desc } = S.sorts.db;
      const q = S.q.toLowerCase();
      const key = { name: (r) => r.name.toLowerCase(), size: (r) => r.size, path: (r) => r.origin.toLowerCase() }[sort] || ((r) => r.mtime);
      rows = (await invoke('bin_list')).map((b) => ({ ...b, mtime: b.deleted, bin: true }))
        .filter((r) => !q || r.name.toLowerCase().includes(q))
        .sort((a, b) => (key(a) < key(b) ? -1 : key(a) > key(b) ? 1 : 0) * (desc ? -1 : 1));
    } else if (browsing()) {
      rows = await invoke('browse', { path: S.cwd, showHidden: S.showHidden });
      if (S.mode === 'tree') {
        for (const p of [...S.open]) {
          try { S.kids.set(p, await invoke('browse', { path: p, showHidden: S.showHidden })); } catch { S.open.delete(p); S.kids.delete(p); }
        }
      }
    } else {
      const { sort, desc } = S.sorts.db;
      rows = await invoke('list', { q: S.q, view: isBrowse() ? S.cwd : S.view, sort, desc });
    }
    S.counts = await counts;
    if (id !== req) return; // a newer request is on its way
    S.rows = rows;
  } catch (e) {
    if (id !== req) return;
    S.rows = [];
    toast(String(e), { error: true });
  }
  build();
  render();
}
let timer;
const reload = (ms = 250) => { clearTimeout(timer); timer = setTimeout(load, ms); };

const sizeOf = (r) => (r.is_dir ? S.sizes.get(r.path)?.size ?? -1 : r.size);

/** Folder sizes are computed in the background (one event per folder) and kept for a minute. */
function requestSizes() {
  if (!browsing()) return;
  const now = Date.now();
  const dirs = S.vis.filter((r) => r.is_dir && !r.synthetic && !(S.sizes.get(r.path)?.t > now - 60000)).map((r) => r.path).slice(0, 300);
  // Re-asking for the same folders would restart the one being measured; only ask when the set changed.
  const key = dirs.join('|');
  if (!dirs.length || key === S.sizeKey) return;
  S.sizeKey = key;
  invoke('folder_sizes', { paths: dirs });
}
/** Folder total when all subfolder sizes are known, else a lower bound. */
function folderTotal() {
  let sum = 0;
  let pending = 0;
  for (const r of S.rows) {
    if (!r.is_dir) sum += r.size;
    else { const z = S.sizes.get(r.path); z ? (sum += z.size) : pending++; }
  }
  return { sum, pending };
}

function sortEntries(list) {
  const { sort, desc } = S.sorts.browse;
  const d = desc ? -1 : 1;
  const by = {
    name: (a, b) => cmpText(a.name, b.name), path: (a, b) => cmpText(a.path, b.path),
    size: (a, b) => sizeOf(a) - sizeOf(b), mtime: (a, b) => a.mtime - b.mtime, md5: (a, b) => cmpText(a.md5 || '', b.md5 || ''),
  }[sort] || ((a, b) => cmpText(a.name, b.name));
  // Folders first (like Explorer), except when sorting by size: then folders compete with their real total size.
  if (sort === 'size') return [...list].sort((a, b) => d * by(a, b) || cmpText(a.name, b.name));
  return [...list].sort((a, b) => (b.is_dir - a.is_dir) || d * by(a, b) || cmpText(a.name, b.name));
}

/** Rows as shown: a flat list, or a tree (folders on disk, or folders rebuilt from the paths of search/DB results). */
function build() {
  const vis = [];
  if (browsing()) {
    const walk = (list, depth) => {
      for (const e of sortEntries(list)) {
        const expanded = S.mode === 'tree' && e.is_dir && S.open.has(e.path);
        vis.push({ ...e, depth, expanded });
        if (expanded) walk(S.kids.get(e.path) || [], depth + 1);
      }
    };
    walk(S.rows, 0);
  } else if (S.mode === 'tree' && S.view !== 'bin') {
    const top = new Map();
    const node = (map, path, name) => map.get(path) || map.set(path, { path, name, dirs: new Map(), files: [], count: 0 }).get(path);
    for (const r of S.rows) {
      const root = rootOf(r.path) || { path: parentOf(r.path), name: parentOf(r.path) };
      let n = node(top, root.path, root.name);
      n.count++;
      let cur = root.path;
      for (const part of parentOf(r.path).slice(root.path.length).split('\\').filter(Boolean)) {
        cur += '\\' + part;
        n = node(n.dirs, cur, part);
        n.count++;
      }
      n.files.push(r);
    }
    const walk = (n, depth) => {
      const expanded = !S.closed.has(n.path);
      vis.push({ path: n.path, name: n.name, is_dir: true, synthetic: true, cat: 'Folder', size: 0, mtime: 0, count: n.count, depth, expanded });
      if (!expanded) return;
      [...n.dirs.values()].sort((a, b) => cmpText(a.name, b.name)).forEach((d) => walk(d, depth + 1));
      n.files.forEach((f) => vis.push({ ...f, depth: depth + 1 }));
    };
    [...top.values()].forEach((n) => walk(n, 0));
  } else {
    S.rows.forEach((r) => vis.push({ ...r, depth: 0 }));
  }
  S.vis = vis;
  const have = new Set(vis.map((r) => r.path));
  S.sel = new Set([...S.sel].filter((p) => have.has(p)));
  const fp = S.focusPath;
  S.focus = fp ? vis.findIndex((r) => r.path === fp) : -1;
}

// ================================================================ rendering
function renderNav() {
  const cur = isBrowse() ? rootOf(S.cwd)?.path : S.view;
  const item = (view, ic, label, extra = '', color = '') =>
    `<button class="nav-item" data-view="${esc(view)}"${cur === view ? ' aria-current="page"' : ''} title="${esc(view.includes('\\') ? view : '')}">
       <span style="${color ? `color:${color}` : ''}">${icon(ic)}</span><span class="label">${esc(label)}</span>${extra}</button>`;
  $('#nav').innerHTML =
    '<div class="nav-head">Rapikan</div>' +
    item('dash', 'chart', 'Dashboard') +
    item('suggest', 'sparkle', 'Saran', `<span class="badge accent" data-n="${S.counts.suggest}">${nf.format(S.counts.suggest)}</span>`, 'var(--accent)') +
    item('dup', 'dup', 'Duplikat', `<span class="badge" data-n="${S.counts.dup}">${nf.format(S.counts.dup)}</span>`) +
    item('all', 'all', 'Semua file', `<span class="count">${nf.format(S.counts.all)}</span>`) +
    `<div class="nav-sep"></div><div class="nav-head row">Folder<button class="btn icon add" data-addroot title="Tambah folder atau drive" aria-label="Tambah folder atau drive">${icon('plus')}</button></div>` +
    (S.roots.length ? S.roots.map((r) => item(r.path, KIND_ICON[r.kind] || 'folder', r.name, r.online ? '' : '<span class="count">tidak terhubung</span>', ROOT_COLOR[r.kind])).join('')
      : '<p class="nav-empty">Belum ada folder. Klik + untuk menambah.</p>') +
    item('bin', 'trash', 'Recycle Bin', '', 'var(--text2)') +
    '<div class="nav-sep"></div>' +
    item('settings', 'gear', 'Pengaturan', AI.status?.enabled ? '<span class="count">AI aktif</span>' : '');
  S.roots.filter((r) => !r.online).forEach((r) => $(`.nav-item[data-view="${CSS.escape(r.path)}"]`)?.classList.add('off'));
  if (S.pane !== 'explorer' || S.view.includes(':')) {
    $$('.nav-item').forEach((b) => b.removeAttribute('aria-current'));
    const on = S.pane === 'settings' ? 'settings' : 'dash'; // a drill-down belongs to the dashboard
    if (S.pane !== 'tidy') $(`.nav-item[data-view="${on}"]`)?.setAttribute('aria-current', 'page');
  }
}

const VIEWS = {
  suggest: { title: 'Saran', sub: () => (S.counts.suggest ? `${nf.format(S.counts.suggest)} file baru menunggu keputusan. Tidak ada yang dipindah tanpa persetujuanmu.` : 'File baru di folder utama akan muncul di sini.') },
  dup: { title: 'Duplikat', sub: () => 'File dengan isi sama persis (MD5 sama), dikelompokkan per isi.' },
  all: { title: 'Semua file', sub: () => `${nf.format(S.counts.all)} file terindeks — hanya nama, lokasi, ukuran, dan MD5. Isi file tidak disimpan.` },
  bin: { title: 'Recycle Bin', sub: () => `${nf.format(S.rows.length)} item · ${human(S.rows.reduce((a, r) => a + r.size, 0))}. Item bisa dipulihkan ke lokasi asalnya.` },
};
/** Title/subtitle of a view, including the dashboard drill-downs "cat:", "ext:" and "year:". */
function viewMeta(v) {
  if (VIEWS[v]) return VIEWS[v];
  const [k, val] = v.split(':');
  const title = k === 'cat' ? `Jenis: ${(CAT[val] || [val])[0]}` : k === 'ext' ? `Ekstensi: ${val ? `.${val}` : '(tanpa ekstensi)'}` : `Diubah tahun ${val}`;
  return {
    title,
    sub: () => `${nf.format(S.rows.length)}${S.rows.length >= 2000 ? '+' : ''} file · ${human(S.rows.reduce((a, r) => a + r.size, 0))} di folder yang dipantau`,
    drill: true,
  };
}

function renderHead() {
  const addr = $('#addr');
  addr.hidden = !isBrowse();
  $('#title').hidden = isBrowse();
  if (isBrowse()) {
    const r = rootOf(S.cwd);
    const parts = [];
    if (r) {
      parts.push([r.name, r.path]);
      let acc = r.path;
      for (const p of S.cwd.slice(r.path.length).split('\\').filter(Boolean)) { acc += '\\' + p; parts.push([p, acc]); }
    } else {
      let acc = '';
      for (const p of S.cwd.split('\\').filter(Boolean)) { acc = acc ? acc + '\\' + p : p + '\\'; parts.push([p, acc.replace(/\\$/, '') || acc]); }
    }
    $('#crumbs').innerHTML = parts.map(([n, p], i) => `${i ? `<span class="chev">${icon('chev')}</span>` : ''}<button data-go="${esc(p)}">${esc(n)}</button>`).join('');
    $('#title').textContent = baseName(S.cwd);
    $('#subtitle').textContent = S.q ? `Hasil pencarian “${S.q}” di ${baseName(S.cwd)} dan subfoldernya` : '';
    $('#subtitle').hidden = !S.q;
    $('[data-nav="back"]').disabled = S.hi <= 0;
    $('[data-nav="fwd"]').disabled = S.hi >= S.hist.length - 1;
    $('[data-nav="up"]').disabled = parentOf(S.cwd) === S.cwd;
  } else {
    const m = viewMeta(S.view);
    $('#title').textContent = m.title;
    $('#subtitle').innerHTML = esc(m.sub()) + (m.drill ? ` <button class="btn std back-dash" data-back-dash>${icon('back')}Dashboard</button>` : '');
    $('#subtitle').hidden = false;
  }
  $('#q').placeholder = `Cari di ${isBrowse() ? baseName(S.cwd) : viewMeta(S.view).title}`;
  $$('#modes button').forEach((b) => b.setAttribute('aria-pressed', b.dataset.mode === S.mode));
}

function render() {
  // A rename box is open: redrawing would destroy it. finishing the rename reloads and redraws.
  if (S.renaming) return;
  renderNav();
  renderHead();
  const cs = cols();
  const { sort, desc } = sortOf();
  $('#cols').innerHTML = cs.map((c) => `<col style="width:${colW(c)}px">`).join('');
  $('#grid').style.width = `${cs.reduce((a, c) => a + colW(c), 0)}px`;
  $('#head').innerHTML = cs.map((c) => {
    const on = c.sort && sort === c.sort;
    const label = c.k === 'mtime' && S.view === 'bin' ? 'Dihapus' : c.label;
    return `<th class="${c.sort ? 'sortable' : ''} ${c.num ? 'num' : ''}" data-sort="${c.sort || ''}" data-k="${c.k}" scope="col"${c.sort ? ` aria-sort="${on ? (desc ? 'descending' : 'ascending') : 'none'}"` : ''}>${label}${on ? `<span class="chev">${desc ? '▼' : '▲'}</span>` : ''}<span class="grip" data-grip="${c.k}"></span></th>`;
  }).join('');

  let html = '';
  let lastMd5 = null;
  const dupList = S.view === 'dup' && S.mode === 'list';
  S.vis.forEach((r, i) => {
    if (dupList && r.md5 !== lastMd5) {
      lastMd5 = r.md5;
      const n = S.vis.filter((x) => x.md5 === r.md5).length;
      html += `<tr class="group"><td colspan="${cs.length}">${n} file identik · ${human(r.size)} per file<span class="mono">${esc(r.md5)}</span></td></tr>`;
    }
    const cut = (S.clip?.cut && S.clip.paths.includes(r.path) ? ' cut' : '') + (r.hidden ? ' hid' : '');
    html += `<tr class="row${cut}" data-i="${i}" aria-selected="${S.sel.has(r.path)}"${r.is_dir ? ` aria-expanded="${!!r.expanded}"` : ''}>${cs.map((c) => cell(c, r)).join('')}</tr>`;
  });
  $('#rows').innerHTML = html;
  renderEmpty();
  paintSel(false);
  requestSizes();
  if (S.pendingRename) {
    const r = S.vis.find((x) => x.path === S.pendingRename);
    if (r) { S.pendingRename = null; startRename(r); }
  }
}

function renderEmpty() {
  const el = $('#empty');
  if (S.vis.length) return (el.hidden = true);
  el.hidden = false;
  if (S.q) el.innerHTML = `${icon('search')}<b>Tidak ada yang cocok</b><p>Tidak ada file dengan nama atau MD5 berisi “${esc(S.q)}”.</p>`;
  else if (browsing()) el.innerHTML = `${icon('folder')}<b>Folder ini kosong</b><p>Tekan Ctrl+Shift+N untuk membuat folder baru.</p>`;
  else if (S.counts.scanning) el.innerHTML = '<span class="ring lg"></span><b>Mengindeks folder…</b><p>Pertama kali bisa beberapa menit. Setelah itu hanya file yang berubah yang dibaca ulang.</p>';
  else if (S.view === 'suggest') el.innerHTML = `${icon('check')}<b>Semua rapi</b><p>File baru di Desktop, Downloads, dan folder lain akan muncul di sini beserta saran tujuannya.</p>`;
  else if (S.view === 'dup') el.innerHTML = `${icon('dup')}<b>Tidak ada duplikat</b><p>Tidak ada dua file dengan isi yang sama persis.</p>`;
  else el.innerHTML = `${icon('folder')}<b>Belum ada file</b><p>Indeks masih kosong.</p>`;
}

const selected = () => S.vis.filter((r) => S.sel.has(r.path));
const realSel = () => selected().filter((r) => !r.synthetic);

function paintSel(scroll = true) {
  $$('#rows tr.row').forEach((tr) => {
    const i = +tr.dataset.i;
    tr.setAttribute('aria-selected', S.sel.has(S.vis[i].path));
    tr.classList.toggle('focus', i === S.focus);
    if (scroll && i === S.focus) tr.scrollIntoView({ block: 'nearest' });
  });
  const sel = selected();
  const real = realSel();
  const sugg = sel.filter((r) => r.suggest);
  const btn = (a) => $(`.cmdbar [data-act="${a}"]`);
  btn('apply').disabled = !sugg.length;
  btn('apply').querySelector('.lbl').textContent =
    sugg.length === 1 ? `Pindah ke ${sugg[0].suggest.replaceAll('\\', ' › ')}` : sugg.length > 1 ? `Pindahkan ${sugg.length} file` : 'Pindah ke saran';
  btn('apply').hidden = btn('dismiss').hidden = !(S.view === 'suggest' || sugg.length);
  btn('dismiss').disabled = !sugg.length;
  btn('newFolder').disabled = !browsing();
  btn('paste').disabled = !pasteTarget(); // the clipboard may hold files copied in File Explorer too
  for (const a of ['cut', 'copy', 'delete']) btn(a).disabled = !real.length;
  btn('rename').disabled = real.length !== 1;
  const inBin = S.view === 'bin';
  if (S.pane === 'explorer') { $('#filecmds').hidden = inBin; $('#bincmds').hidden = !inBin; }
  btn('binRestore').disabled = btn('binPurge').disabled = !sel.length;
  btn('binEmpty').disabled = !S.rows.length;

  let total = '';
  if (browsing()) {
    const t = folderTotal();
    total = t.pending ? `  ·  total ≥ ${human(t.sum)} (menghitung ${t.pending} folder…)` : `  ·  total ${human(t.sum)}`;
  }
  $('#st-items').textContent = `${nf.format(S.vis.filter((r) => !r.synthetic).length)} item${S.rows.length >= 2000 && !browsing() ? ' (2.000 teratas — persempit pencarian)' : ''}${total}`;
  $('#st-sel').textContent = sel.length ? `${nf.format(sel.length)} item dipilih  ${human(real.reduce((a, r) => a + (r.size || 0), 0))}` : '';
  $('#st-clip').textContent = S.clip ? `${S.clip.paths.length} item ${S.clip.cut ? 'dipotong' : 'disalin'} — Ctrl+V untuk menempel` : '';
  $('#st-watch').innerHTML = S.counts.scanning ? '<span class="ring"></span>Mengindeks…' : `Memantau ${S.roots.length} folder`;
  schedulePreview();
}

// ================================================================ navigation
function resetSel() { S.sel.clear(); S.focus = S.anchor = -1; S.focusPath = null; }

/** One of the three panes: the explorer, the Rapikan wizard, or Pengaturan. */
function setPane(name) {
  S.pane = name;
  $('#explorer').hidden = name !== 'explorer';
  $('#tidy').hidden = name !== 'tidy';
  $('#settings').hidden = name !== 'settings';
  $('#dashboard').hidden = name !== 'dashboard';
  const full = name === 'explorer';
  $('#filecmds').hidden = $('#modes').hidden = $('#searchbox').hidden = !full;
  $('#bincmds').hidden = !full || S.view !== 'bin';
  $('#pvbtn').hidden = $('#viewbtn').hidden = !full; // view options only apply to the file list
  $('.cmdbar [data-act="tidy"]').hidden = !full;
  if (!full) { $('#st-sel').textContent = $('#st-clip').textContent = ''; $('#st-items').textContent = { tidy: 'Rapikan folder', settings: 'Pengaturan', dashboard: 'Dashboard' }[name]; }
  closeMenu();
  renderNav();
}
function go(path, push = true) {
  closeTidy();
  S.view = 'browse';
  S.cwd = path;
  S.q = '';
  $('#q').value = '';
  resetSel();
  S.open.clear();
  S.kids.clear();
  if (push) { S.hist = S.hist.slice(0, S.hi + 1); S.hist.push(path); S.hi = S.hist.length - 1; }
  load();
  $('#list').scrollTop = 0;
  $('#list').focus();
}
function setView(v) {
  closeTidy();
  S.view = v;
  S.cwd = null;
  S.q = '';
  $('#q').value = '';
  resetSel();
  S.closed.clear();
  load();
  $('#list').scrollTop = 0;
}
function nav(kind) {
  if (kind === 'back' && S.hi > 0) go(S.hist[--S.hi], false);
  else if (kind === 'fwd' && S.hi < S.hist.length - 1) go(S.hist[++S.hi], false);
  else if (kind === 'up' && isBrowse() && parentOf(S.cwd) !== S.cwd) { const from = S.cwd; go(parentOf(S.cwd)); S.focusPath = from; S.sel = new Set([from]); }
  else if (kind === 'refresh') load();
}

async function toggle(r, open = !r.expanded) {
  if (!r.is_dir) return;
  if (r.synthetic) { open ? S.closed.delete(r.path) : S.closed.add(r.path); build(); render(); return; }
  if (open) {
    S.open.add(r.path);
    if (!S.kids.has(r.path)) {
      try { S.kids.set(r.path, await invoke('browse', { path: r.path, showHidden: S.showHidden })); } catch (e) { S.open.delete(r.path); toast(String(e), { error: true }); }
    }
  } else {
    S.open.delete(r.path);
  }
  build();
  render();
}

/** Double-click / Enter: a folder opens (navigates into it), a file opens in its app. */
function activate(r) {
  if (!r) return;
  if (r.bin) return toast('Item di Recycle Bin perlu dipulihkan dulu sebelum dibuka.');
  if (r.is_dir) go(r.path);
  else act('open', [r]);
}

// ================================================================ actions
function toast(msg, { error = false, action = null, ms } = {}) {
  const t = document.createElement('div');
  t.className = `toast${error ? ' error' : ''}`;
  t.setAttribute('role', error ? 'alert' : 'status');
  t.innerHTML = `${icon(error ? 'error' : 'check')}<div>${esc(msg)}</div>${action ? `<button class="btn std">${esc(action.label)}</button>` : ''}`;
  if (action) t.querySelector('button').onclick = () => { t.remove(); action.fn(); };
  $('#toasts').append(t);
  setTimeout(() => t.remove(), ms ?? (error ? 8000 : action ? 9000 : 3500));
}
const undoAction = (batch) => batch ? { label: 'Urungkan', fn: () => undo(batch) } : null;
/** Undo a move batch (number) or a delete ({ bin: [original paths] } → restore from the Recycle Bin). */
async function undo(batch = S.undo.pop()) {
  if (!batch) return toast('Tidak ada yang bisa diurungkan.');
  S.undo = S.undo.filter((b) => b !== batch);
  try {
    if (batch.bin) await invoke('bin_restore_paths', { paths: batch.bin });
    else await invoke('undo_batch', { batch });
    toast(batch.bin ? 'Dipulihkan dari Recycle Bin' : 'Diurungkan');
  } catch (e) { toast(`Sebagian tidak bisa diurungkan:\n${e}`, { error: true }); }
  load();
}

/** Where Ctrl+V goes: the open folder (like Explorer); "Tempel ke <folder>" in the menu targets the selected folder. */
function pasteTarget(rows = selected(), into = false) {
  if (into && rows.length === 1 && rows[0].is_dir && !rows[0].synthetic) return rows[0].path;
  return browsing() ? S.cwd : null;
}

async function act(kind, rows = selected()) {
  const real = rows.filter((r) => !r.synthetic);
  // In the Recycle Bin only restore / delete permanently make sense; Delete means "delete permanently".
  if (S.view === 'bin' && S.pane === 'explorer') {
    if (kind === 'delete') kind = 'binPurge';
    if (!['binRestore', 'binPurge', 'binEmpty', 'copyPath', 'preview', 'viewMenu', 'reveal'].includes(kind)) return;
  }
  try {
    switch (kind) {
      case 'binRestore': {
        if (!rows.length) return;
        const n = await invoke('bin_restore', { ids: rows.map((r) => r.path) });
        toast(`${n} item dipulihkan ke lokasi asalnya`);
        resetSel();
        return load();
      }
      case 'binPurge':
      case 'binEmpty': {
        const all = kind === 'binEmpty';
        const n = all ? S.rows.length : rows.length;
        if (!n) return;
        if (!(await confirmDialog(all ? 'Kosongkan Recycle Bin?' : `Hapus permanen ${n} item?`,
          `${n} item akan dihapus selamanya dan tidak bisa dipulihkan lagi.`, 'Hapus permanen'))) return;
        await invoke('bin_purge', { ids: all ? [] : rows.map((r) => r.path) });
        toast(`${n} item dihapus permanen`);
        resetSel();
        return load();
      }
      case 'open': {
        const dir = rows.find((r) => r.is_dir);
        if (dir) return go(dir.path);
        for (const r of real.slice(0, 10)) await invoke('open', { path: r.path });
        break;
      }
      case 'openLocation': return rows[0] && go(rows[0].is_dir ? rows[0].path : parentOf(rows[0].path));
      case 'reveal': return rows[0] && invoke('reveal', { path: rows[0].path });
      case 'copyPath':
        await navigator.clipboard.writeText(rows.map((r) => r.origin || r.path).join('\r\n'));
        return toast(rows.length > 1 ? `${rows.length} path disalin` : 'Path disalin');
      case 'copyMd5':
        await navigator.clipboard.writeText(real.map((r) => r.md5 || '').join('\r\n'));
        return toast('MD5 disalin');
      case 'cut':
      case 'copy':
        if (!real.length) return;
        S.clip = { paths: real.map((r) => r.path), cut: kind === 'cut' };
        await invoke('clip_set', S.clip); // Windows clipboard: File Explorer can paste it too
        render();
        return;
      case 'paste':
      case 'pasteInto': {
        const dest = pasteTarget(rows, kind === 'pasteInto');
        if (!dest) return;
        const clip = (await invoke('clip_get')) || S.clip;
        if (!clip) return toast('Clipboard kosong — salin atau potong file dulu, di sini atau di File Explorer.');
        const r = await invoke('paste_items', { paths: clip.paths, dest, cut: clip.cut });
        if (clip.cut) { S.clip = null; invoke('clip_clear'); }
        if (r.batch) S.undo.push(r.batch);
        if (r.done) toast(`${r.done} item ${r.batch ? 'dipindah' : 'disalin'} ke ${baseName(dest)}`, { action: undoAction(r.batch) });
        if (r.failed.length) toast(r.failed.join('\n'), { error: true });
        return load();
      }
      case 'props': return rows[0] && showProps(rows[0].path);
      case 'preview': return setPreview(!S.previewOn);
      case 'viewMenu': {
        // Explorer's "View ▸ Show": options with a check mark, remembered.
        const r = $('#viewbtn').getBoundingClientRect();
        return showMenuAt(
          '<div class="menu-head">Tampilkan</div>' +
          mi('data-viewopt="preview"', S.previewOn ? 'tick' : 'none', 'Panel pratinjau', false, 'Alt+P') +
          mi('data-viewopt="hidden"', S.showHidden ? 'tick' : 'none', 'Item tersembunyi', false, 'Ctrl+H'), r.left, r.bottom + 4);
      }
      case 'rename': return real.length === 1 && startRename(real[0]);
      case 'delete': {
        if (!real.length) return;
        const paths = real.map((r) => r.path);
        const n = await invoke('trash_items', { paths });
        S.undo.push({ bin: paths });
        toast(`${n} item dipindah ke Recycle Bin`, { action: { label: 'Urungkan', fn: () => undo(S.undo.find((u) => u.bin === paths)) } });
        resetSel();
        return load();
      }
      case 'newFolder': {
        if (!browsing()) return;
        const p = await invoke('new_folder', { parent: S.cwd });
        S.sel = new Set([p]);
        S.focusPath = p;
        S.pendingRename = p; // render() opens the name box as soon as the new row is on screen
        return load();
      }
      case 'apply': {
        let ok = 0;
        const fails = [];
        let batch = null;
        for (const r of real.filter((r) => r.suggest)) {
          try { const m = await invoke('apply', { path: r.path }); S.undo.push(m.batch); batch = m.batch; ok++; } catch (e) { fails.push(`${r.name}: ${e}`); }
        }
        if (ok) toast(ok === 1 ? 'File dipindahkan' : `${ok} file dipindahkan`, { action: ok === 1 ? undoAction(batch) : null });
        if (fails.length) toast(fails.join('\n'), { error: true });
        resetSel();
        return load();
      }
      case 'dismiss': {
        const rs = real.filter((r) => r.suggest);
        for (const r of rs) await invoke('dismiss', { path: r.path });
        toast(rs.length === 1 ? 'Saran diabaikan' : `${rs.length} saran diabaikan`);
        resetSel();
        return load();
      }
      case 'tidy': {
        const dir = rows.length === 1 && rows[0].is_dir ? rows[0].path : isBrowse() ? S.cwd : null;
        return openTidy(dir);
      }
    }
  } catch (e) {
    toast(String(e), { error: true });
  }
}

function startRename(r) {
  const i = S.vis.findIndex((x) => x.path === r.path);
  const tr = $(`#rows tr[data-i="${i}"]`);
  if (!tr) return;
  const span = $('.nm', tr);
  const inp = document.createElement('input');
  inp.className = 'rename';
  inp.value = r.name;
  inp.setAttribute('aria-label', 'Nama baru');
  span.replaceWith(inp);
  S.renaming = r.path;
  tr.scrollIntoView({ block: 'nearest' });
  inp.focus();
  const dot = r.is_dir ? -1 : r.name.lastIndexOf('.');
  inp.setSelectionRange(0, dot > 0 ? dot : r.name.length); // like Explorer: the extension is not selected
  let done = false;
  const finish = async (commit) => {
    if (done) return;
    done = true;
    S.renaming = null;
    const v = inp.value.trim();
    if (commit && v && v !== r.name) {
      try {
        const np = await invoke('rename_item', { path: r.path, name: v });
        S.sel = new Set([np]);
        S.focusPath = np;
      } catch (e) { toast(String(e), { error: true }); }
    }
    await load();
    $('#list').focus();
  };
  inp.addEventListener('keydown', (e) => {
    e.stopPropagation();
    if (e.key === 'Enter') finish(true);
    if (e.key === 'Escape') finish(false);
  });
  inp.addEventListener('blur', () => finish(true));
  ['mousedown', 'dblclick', 'click'].forEach((ev) => inp.addEventListener(ev, (e) => e.stopPropagation()));
}

function select(i, { shift = false, ctrl = false } = {}) {
  if (i < 0 || i >= S.vis.length) return;
  const p = S.vis[i].path;
  if (shift && S.anchor >= 0) {
    if (!ctrl) S.sel.clear();
    const [a, b] = [Math.min(S.anchor, i), Math.max(S.anchor, i)];
    for (let k = a; k <= b; k++) S.sel.add(S.vis[k].path);
  } else if (ctrl) {
    S.sel.has(p) ? S.sel.delete(p) : S.sel.add(p);
    S.anchor = i;
  } else {
    S.sel = new Set([p]);
    S.anchor = i;
  }
  S.focus = i;
  S.focusPath = p;
  paintSel();
}

// ================================================================ context menu
function openMenu(x, y, onRow) {
  const sel = selected();
  const real = sel.filter((r) => !r.synthetic);
  const sugg = real.filter((r) => r.suggest);
  const one = sel.length === 1 ? sel[0] : null;
  const b = (a, ic, label, key = '', disabled = false) =>
    `<button role="menuitem" data-act="${a}"${disabled ? ' disabled' : ''}>${icon(ic)}${esc(label)}<span class="key">${key}</span></button>`;
  let html;
  if (S.view === 'bin') {
    if (!onRow || !sel.length) return;
    html = b('binRestore', 'restore', sel.length > 1 ? `Pulihkan ${sel.length} item` : 'Pulihkan') +
      b('binPurge', 'trash', 'Hapus permanen', 'Del') + '<hr>' + b('copyPath', 'link', 'Salin lokasi asal');
  } else if (onRow && sel.length) {
    html = b('open', 'open', one?.is_dir ? 'Buka' : sel.length > 1 ? `Buka ${Math.min(real.length, 10)} file` : 'Buka', 'Enter') +
      (!browsing() ? b('openLocation', 'folder', 'Buka lokasi file') : '') +
      b('reveal', 'folder', 'Tampilkan di File Explorer') +
      (sugg.length ? '<hr>' + b('apply', 'move', sugg.length === 1 ? `Pindah ke ${sugg[0].suggest.replaceAll('\\', ' › ')}` : `Pindahkan ${sugg.length} file ke saran`) + b('dismiss', 'hide', 'Abaikan saran') : '') +
      '<hr>' + b('cut', 'cut', 'Potong', 'Ctrl+X', !real.length) + b('copy', 'copy', 'Salin', 'Ctrl+C', !real.length) +
      (S.clip && one?.is_dir && !one.synthetic ? b('pasteInto', 'paste', `Tempel ke ${one.name}`) : '') +
      b('rename', 'rename', 'Ganti nama', 'F2', real.length !== 1) + b('delete', 'trash', 'Hapus', 'Del', !real.length) +
      '<hr>' + b('copyPath', 'link', 'Salin path', 'Ctrl+Shift+C') + (real.some((r) => r.md5) ? b('copyMd5', 'hash', 'Salin MD5') : '') +
      (one?.is_dir && !one.synthetic ? '<hr>' + b('tidy', 'sparkle', 'Rapikan folder ini…') : '') +
      (one ? '<hr>' + b('props', 'info', 'Properti', 'Alt+Enter') : '');
  } else if (browsing()) {
    resetSel(); paintSel(false);
    html = b('newFolder', 'folderAdd', 'Folder baru', 'Ctrl+Shift+N') + b('paste', 'paste', 'Tempel', 'Ctrl+V') +
      '<hr>' + b('refresh', 'refresh', 'Muat ulang', 'F5') + b('tidy', 'sparkle', 'Rapikan folder ini…') +
      '<hr>' + b('propsCwd', 'info', 'Properti folder ini');
  } else return;
  const m = $('#menu');
  m.innerHTML = html;
  m.hidden = false;
  S.menuOpen = true;
  const r = m.getBoundingClientRect();
  m.style.left = `${Math.max(4, Math.min(x, innerWidth - r.width - 8))}px`;
  m.style.top = `${Math.max(4, Math.min(y, innerHeight - r.height - 8))}px`;
  m.querySelector('button:not(:disabled)')?.focus();
}
const closeMenu = () => { $('#menu').hidden = true; S.menuOpen = false; };

// ================================================================ dialog
function confirmDialog(title, text, ok = 'OK') {
  return new Promise((resolve) => {
    const d = $('#dialog');
    d.innerHTML = `<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dlg-t"><div class="body"><h2 id="dlg-t">${esc(title)}</h2><p>${esc(text)}</p></div>
      <div class="foot"><button class="btn primary" data-r="1">${esc(ok)}</button><button class="btn std" data-r="0">Batal</button></div></div>`;
    d.hidden = false;
    const close = (v) => { d.hidden = true; d.innerHTML = ''; document.removeEventListener('keydown', key, true); resolve(v); };
    const key = (e) => { if (e.key === 'Escape') { e.stopPropagation(); close(false); } };
    document.addEventListener('keydown', key, true);
    d.querySelectorAll('[data-r]').forEach((b) => (b.onclick = () => close(b.dataset.r === '1')));
    d.querySelector('[data-r="1"]').focus();
  });
}

// ================================================================ preview pane (Alt+P)
const EXT = (name) => (name.includes('.') ? name.split('.').pop().toLowerCase() : '');
const PV = {
  image: { exts: 'png jpg jpeg gif webp bmp ico svg avif', max: 40 << 20 },
  pdf: { exts: 'pdf', max: 80 << 20 },
  audio: { exts: 'mp3 wav m4a ogg flac aac', max: 120 << 20 },
  video: { exts: 'mp4 webm mov m4v', max: 300 << 20 },
  text: { exts: 'txt md csv json js ts tsx jsx css html htm xml yml yaml log ini cfg conf sql py rs php sh bat ps1 cmd reg env gitignore toml lock map mq5 bib vcf srt docx' },
};
const MIME = { png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', webp: 'image/webp', bmp: 'image/bmp', ico: 'image/x-icon', svg: 'image/svg+xml', avif: 'image/avif',
  pdf: 'application/pdf', mp3: 'audio/mpeg', wav: 'audio/wav', m4a: 'audio/mp4', ogg: 'audio/ogg', flac: 'audio/flac', aac: 'audio/aac', mp4: 'video/mp4', webm: 'video/webm', mov: 'video/mp4', m4v: 'video/mp4' };
const pvKind = (r) => (r.is_dir ? 'folder' : Object.keys(PV).find((k) => PV[k].exts.split(' ').includes(EXT(r.name))) || 'none');

let pvReq = 0;
let pvUrl = null;
let pvKey = '';
let pvTimer;
function setPreview(on) {
  S.previewOn = on;
  store.set('preview', on);
  $('#pvbtn').setAttribute('aria-pressed', on);
  $('#preview').hidden = $('#pvgrip').hidden = !on;
  pvKey = '';
  schedulePreview();
}
/** View option "Item tersembunyi": hidden files and folders are listed (dimmed); protected OS files never are. */
function setHidden(on) {
  S.showHidden = on;
  store.set('hidden', on);
  S.kids.clear();
  toast(on ? 'Item tersembunyi ditampilkan' : 'Item tersembunyi disembunyikan');
  load();
}

/** Called whenever the selection may have changed; only reloads when it really did. */
function schedulePreview() {
  if (!S.previewOn) return;
  const key = selected().map((r) => r.path).join('|');
  if (key === pvKey) return;
  pvKey = key;
  clearTimeout(pvTimer);
  pvTimer = setTimeout(updatePreview, 120);
}

async function updatePreview() {
  const el = $('#pv-content');
  const id = ++pvReq;
  if (pvUrl) { URL.revokeObjectURL(pvUrl); pvUrl = null; }
  const sel = selected();
  if (!sel.length) {
    el.innerHTML = `<div class="pv-empty">${icon('panel')}<b>Pratinjau</b><p>Pilih file atau folder untuk melihat isinya di sini.</p></div>`;
    return;
  }
  if (sel.length > 1) {
    const files = sel.filter((r) => !r.is_dir);
    const size = files.reduce((a, r) => a + (r.size || 0), 0) + sel.filter((r) => r.is_dir).reduce((a, r) => a + (S.sizes.get(r.path)?.size || 0), 0);
    el.innerHTML = `<div class="pv-empty">${icon('copy')}<b>${sel.length} item dipilih</b><p>${files.length} file · ${sel.length - files.length} folder · ${human(size)}</p></div>`;
    return;
  }
  const r = sel[0];
  const kind = pvKind(r);
  const z = r.is_dir ? S.sizes.get(r.path) : null;
  const meta = [
    ['Jenis', r.synthetic || r.is_dir ? 'Folder' : `${(CAT[r.cat] || CAT.Lainnya)[0]}${EXT(r.name) ? ` (.${EXT(r.name)})` : ''}`],
    ['Ukuran', r.is_dir ? (z ? `${human(z.size)} · ${nf.format(z.files)} file` : '…') : human(r.size)],
    [r.bin ? 'Dihapus' : 'Diubah', r.mtime ? df.format(new Date(r.mtime * 1000)) : '—'],
    [r.bin ? 'Asal' : 'Lokasi', r.bin ? parentOf(r.origin) : pretty(parentOf(r.path))],
  ];
  if (r.md5) meta.push(['MD5', `<span class="mono">${esc(r.md5)}</span>`]);
  const sug = r.suggest ? `<div class="pv-sug"><span class="chip"><span class="dot" style="background:${catColor(r.suggest)}"></span>→ ${esc(r.suggest.replaceAll('\\', ' › '))}</span>
      <button class="btn primary sm2" data-pv="apply">Pindah</button></div>` : '';
  el.innerHTML = `<div class="pv-body" id="pv-body"><div class="pv-loading"><span class="ring"></span>Memuat pratinjau…</div></div>
    <div class="pv-info"><div class="pv-title">${fileIcon(r.is_dir ? 'Folder' : r.cat)}<b title="${esc(r.name)}">${esc(r.name)}</b></div>${sug}
      <dl>${meta.map(([k, v]) => `<dt>${k}</dt><dd>${k === 'MD5' ? v : esc(v)}</dd>`).join('')}</dl>
      <div class="pv-actions">${r.bin ? `<button class="btn primary" data-pv="restore">${icon('restore')}Pulihkan</button>`
        : `<button class="btn std" data-pv="open">${icon('open')}Buka</button><button class="btn std" data-pv="reveal">${icon('folder')}Di Explorer</button>`}</div></div>`;
  const body = $('#pv-body');
  const none = (msg) => { if (id === pvReq) body.innerHTML = `<div class="pv-none">${fileIcon(r.is_dir ? 'Folder' : r.cat)}<p>${esc(msg)}</p></div>`; };
  try {
    if (kind === 'folder') {
      const kids = sortByName(await invoke('browse', { path: r.path, showHidden: S.showHidden }));
      if (id !== pvReq) return;
      if (!kids.length) return none('Folder ini kosong.');
      kids.sort((a, b) => b.is_dir - a.is_dir);
      body.innerHTML = `<ul class="pv-list">${kids.slice(0, 200).map((k) => `<li title="${esc(k.name)}">${fileIcon(k.is_dir ? 'Folder' : k.cat)}<span>${esc(k.name)}</span><small>${k.is_dir ? '' : human(k.size)}</small></li>`).join('')}</ul>
        ${kids.length > 200 ? `<p class="pv-more">+ ${nf.format(kids.length - 200)} item lainnya</p>` : ''}`;
    } else if (kind === 'text') {
      const t = await invoke('preview_text', { path: r.path });
      if (id !== pvReq) return;
      if (!t.trim()) return none('File ini kosong.');
      const pre = document.createElement('pre');
      pre.className = 'pv-text';
      pre.textContent = t; // shown as text, never rendered as HTML
      body.replaceChildren(pre);
    } else if (kind !== 'none') {
      const buf = await invoke('preview_bytes', { path: r.path, max: PV[kind].max });
      if (id !== pvReq) return;
      // ArrayBuffer over the binary IPC channel; a plain number array if the JSON fallback was used.
      pvUrl = URL.createObjectURL(new Blob([new Uint8Array(buf)], { type: MIME[EXT(r.name)] || 'application/octet-stream' }));
      body.innerHTML = kind === 'image' ? `<img class="pv-img" src="${pvUrl}" alt="${esc(r.name)}">`
        : kind === 'pdf' ? `<iframe class="pv-frame" src="${pvUrl}" title="${esc(r.name)}"></iframe>`
          : kind === 'audio' ? `<div class="pv-none">${fileIcon(r.cat)}<audio controls src="${pvUrl}"></audio></div>`
            : `<video class="pv-img" controls src="${pvUrl}"></video>`;
    } else {
      none('Tidak ada pratinjau untuk jenis file ini.');
    }
  } catch (e) { none(String(e)); }
}

$('#preview').addEventListener('click', (e) => {
  const b = e.target.closest('[data-pv]');
  if (!b) return;
  if (b.dataset.pv === 'close') return setPreview(false);
  const r = selected()[0];
  if (!r) return;
  if (b.dataset.pv === 'restore') act('binRestore', [r]);
  if (b.dataset.pv === 'open') activate(r);
  if (b.dataset.pv === 'reveal') act('reveal', [r]);
  if (b.dataset.pv === 'apply') act('apply', [r]);
});
// drag the divider to resize the pane (width remembered)
$('#pvgrip').addEventListener('mousedown', (e) => {
  e.preventDefault();
  const x0 = e.clientX;
  const w0 = $('#preview').getBoundingClientRect().width;
  const move = (ev) => { const w = Math.max(240, Math.min(innerWidth * 0.6, w0 - (ev.clientX - x0))); $('#preview').style.width = `${w}px`; };
  const up = () => { removeEventListener('mousemove', move); removeEventListener('mouseup', up); store.set('previewW', $('#preview').getBoundingClientRect().width); };
  addEventListener('mousemove', move);
  addEventListener('mouseup', up);
});

// ================================================================ Dashboard
const D = { data: null, mode: store.get('dashModes', {}), sort: {}, lists: {} };
async function openDash() {
  T.on = false;
  setPane('dashboard');
  renderDash();
  try { D.data = await invoke('dashboard'); } catch (e) { toast(String(e), { error: true }); }
  if (S.pane === 'dashboard') renderDash();
}
const pct = (n, total) => (total ? (n * 100) / total : 0);
const pctTxt = (p) => (p > 0 && p < 1 ? '<1%' : `${Math.round(p)}%`);
const NAME_OF = {
  cat: (b) => (CAT[b.label] || [b.label])[0],
  ext: (b) => (b.label ? `.${b.label}` : '(tanpa ekstensi)'),
  root: (b) => b.label,
  year: (b) => b.label,
};
const tipOf = (key, b, total) => `${NAME_OF[key](b)}\n${human(b.size)} · ${pctTxt(pct(b.size, total))} dari total\n${nf.format(b.files)} file — klik untuk melihat daftarnya`;

/** Ranked bars, one hue, largest first; name + count on the left, size + share on the right (no legend needed). */
function meterChart(key, rows, total) {
  const max = Math.max(...rows.map((b) => b.size), 1);
  const shown = [...rows].sort((a, b) => b.size - a.size).slice(0, 12);
  return `<div class="meters">${shown.map((b) => `<div class="meter link" data-open="${key}:${rows.indexOf(b)}" data-tip="${esc(tipOf(key, b, total))}" tabindex="0" role="button" aria-label="${esc(`${NAME_OF[key](b)}: ${human(b.size)}, ${pctTxt(pct(b.size, total))}, ${b.files} file`)}">
      <span class="m-name">${esc(NAME_OF[key](b))}<small>${nf.format(b.files)}</small></span>
      <span class="m-bar"><i style="width:${Math.max(0.6, (b.size * 100) / max)}%"></i></span>
      <span class="m-val">${human(b.size)} · ${pctTxt(pct(b.size, total))}</span></div>`).join('')}</div>
    ${rows.length > 12 ? `<p class="more">+ ${rows.length - 12} lainnya — lihat mode Daftar</p>` : ''}`;
}

/** Size per year modified: thin columns on one baseline; the tallest one carries its value, the rest on hover. */
function yearChart(rows, total) {
  const max = Math.max(...rows.map((b) => b.size), 1);
  const top = rows.reduce((a, b) => (b.size > a.size ? b : a), rows[0]);
  const step = rows.length > 14 ? 2 : 1;
  return `<div class="cols">${rows.map((b, i) => `<div class="c" data-open="year:${i}" data-tip="${esc(tipOf('year', b, total))}" tabindex="0" role="button" aria-label="${esc(`${b.label}: ${human(b.size)}`)}">
      ${b === top ? `<b>${human(b.size)}</b>` : ''}<i style="height:${Math.max(1.5, (b.size * 100) / max)}%"></i></div>`).join('')}</div>
    <div class="col-labels">${rows.map((b, i) => `<span>${i % step === 0 || i === rows.length - 1 ? `'${b.label.slice(2)}` : ''}</span>`).join('')}</div>`;
}

/** Sortable table view of the same rows (the accessible alternative to the chart). */
function bucketTable(key, rows, total) {
  const s = D.sort[key] || { k: 'size', desc: true };
  const val = { name: (b) => NAME_OF[key](b).toLowerCase(), files: (b) => b.files, size: (b) => b.size };
  const sorted = rows.map((b, i) => ({ b, i })).sort((x, y) => {
    const a = val[s.k](x.b);
    const c = val[s.k](y.b);
    return (a < c ? -1 : a > c ? 1 : 0) * (s.desc ? -1 : 1);
  });
  const th = (k, label, num) => `<th class="sortable${num ? ' num' : ''}" data-dsort="${key}:${k}" aria-sort="${s.k === k ? (s.desc ? 'descending' : 'ascending') : 'none'}">${label}${s.k === k ? `<span class="chev">${s.desc ? '▼' : '▲'}</span>` : ''}</th>`;
  return `<div class="dtable-wrap"><table class="dtable"><thead><tr>${th('name', 'Nama')}${th('files', 'File', 1)}${th('size', 'Ukuran', 1)}<th class="num">%</th></tr></thead>
    <tbody>${sorted.map(({ b, i }) => `<tr data-open="${key}:${i}" tabindex="0"><td>${esc(NAME_OF[key](b))}</td><td class="num">${nf.format(b.files)}</td><td class="num">${human(b.size)}</td><td class="num">${pctTxt(pct(b.size, total))}</td></tr>`).join('')}</tbody></table></div>`;
}

function bucketCard(key, title, note, rows, total, chart) {
  const mode = D.mode[key] || 'chart';
  const toggle = `<div class="seg mini" role="group" aria-label="Tampilan ${esc(title)}">
      <button data-dmode="${key}:chart" aria-pressed="${mode === 'chart'}" title="Chart" aria-label="Chart">${icon('chart')}</button>
      <button data-dmode="${key}:list" aria-pressed="${mode === 'list'}" title="Daftar (bisa diurutkan)" aria-label="Daftar">${icon('listv')}</button></div>`;
  const body = !rows.length ? '<p class="reason">Belum ada data.</p>' : mode === 'list' ? bucketTable(key, rows, total) : chart(key, rows, total);
  return `<section class="card dcard"><h2>${esc(title)}<small>${esc(note)}</small><span class="grow"></span>${toggle}</h2>${body}</section>`;
}

function renderDash() {
  const el = $('#dashboard');
  const d = D.data;
  const head = `<div class="page-head"><h1>Dashboard</h1><p>${d ? `Ringkasan indeks: ${nf.format(d.files)} file di ${S.roots.length} folder yang dipantau. Klik bar atau baris untuk melihat daftar filenya.` : 'Menghitung…'}</p></div>`;
  if (!d) { el.innerHTML = head + '<div class="result"><span class="ring lg"></span><b>Menghitung ringkasan…</b></div>'; return; }
  D.lists = { cat: d.by_cat, ext: d.by_ext, root: d.by_root, year: d.by_year };
  const tile = (label, value, sub, act = '', actLabel = '') => `<div class="card kpi"><span class="k-label">${label}</span><span class="k-value">${value}</span>
      <span class="k-sub">${sub}</span>${act ? `<button class="btn" data-dash="${act}">${actLabel} ›</button>` : ''}</div>`;
  const kpis = tile('Total terindeks', human(d.size), `${nf.format(d.files)} file`) +
    tile('Terbuang oleh duplikat', human(d.dup_wasted), `${nf.format(d.dup_files)} file punya salinan identik`, 'dup', 'Lihat duplikat') +
    tile('Saran menunggu', nf.format(d.suggest), 'file baru belum dirapikan', 'suggest', 'Buka Saran') +
    tile('Tidak diubah > 1 tahun', human(d.stale.size), `${nf.format(d.stale.files)} file — kandidat arsip`);

  const big = d.largest.map((r, i) => `<div class="drow" data-big="${i}" title="${esc(r.path)}">${fileIcon(r.cat)}
      <div class="grow"><span>${esc(r.name)}</span><small>${esc(pretty(parentOf(r.path)))}</small></div>
      <span class="v">${human(r.size)}</span><button class="btn std" data-bigloc="${i}">Lihat</button></div>`).join('') || '<p class="reason">Belum ada data.</p>';
  const dups = d.dup_groups.map((g, i) => `<div class="drow" title="${esc(g.paths.join('\n'))}">${fileIcon(rulesCat(g.paths[0]))}
      <div class="grow"><span>${esc(baseName(g.paths[0] || ''))}</span><small>${g.count} salinan · ${human(g.size)} per file</small></div>
      <span class="v">${human(g.wasted)} terbuang</span><button class="btn std" data-dup="${i}">Lihat</button></div>`).join('') || '<p class="reason">Tidak ada duplikat.</p>';
  const tipRow = (ic, text, sub, act, label) => `<div class="drow tipsrow">${icon(ic)}<div class="grow"><span>${text}</span><small>${sub}</small></div>${act ? `<button class="btn std" data-dash="${act}">${label}</button>` : ''}</div>`;
  const tips = [
    d.dup_wasted ? tipRow('dup', `Hapus salinan duplikat: hemat ${human(d.dup_wasted)}`, `${nf.format(d.dup_files)} file punya salinan dengan isi sama persis`, 'dup', 'Lihat') : '',
    d.old_downloads.files ? tipRow('Downloads', `${nf.format(d.old_downloads.files)} file di Downloads lebih dari 90 hari`, `${human(d.old_downloads.size)} — sering sudah tidak dibutuhkan`, 'oldDownloads', 'Buka') : '',
    d.suggest ? tipRow('sparkle', `${nf.format(d.suggest)} file baru menunggu dirapikan`, 'Saran tujuan sudah disiapkan', 'suggest', 'Buka Saran') : '',
    d.empty_files ? tipRow('warn', `${nf.format(d.empty_files)} file kosong (0 byte)`, 'Rapikan folder akan memindahkannya ke _Kosong', 'tidy', 'Rapikan') : '',
    tipRow('trash', 'Recycle Bin', 'Lihat isinya, pulihkan, atau kosongkan untuk mengosongkan ruang', 'bin', 'Buka'),
  ].join('');

  el.innerHTML = head + `<div class="dash">
    <div class="kpis">${kpis}</div>
    <div class="dash-grid">
      ${bucketCard('cat', 'Jenis file', 'menurut ukuran', d.by_cat, d.size, meterChart)}
      ${bucketCard('root', 'Folder yang dipantau', 'menurut ukuran', d.by_root, d.size, meterChart)}
      ${bucketCard('ext', 'Ekstensi', '30 terbesar', d.by_ext, d.size, meterChart)}
      ${bucketCard('year', 'Umur file', 'ukuran per tahun terakhir diubah', d.by_year, d.size, (k, rows, total) => yearChart(rows, total))}
      <section class="card dcard"><h2>File terbesar<small>${d.largest.length} teratas</small></h2><div class="dlist">${big}</div></section>
      <section class="card dcard"><h2>Duplikat paling boros<small>ruang yang bisa dihemat</small></h2><div class="dlist">${dups}</div></section>
      <section class="card dcard"><h2>Bisa dibersihkan</h2><div class="dlist">${tips}</div></section>
    </div></div>`;
}
const rulesCat = (p) => catOfName(baseName(p || ''));

/** A file somewhere in the index → open its folder in the app with the file selected. */
function showInApp(path) {
  go(parentOf(path));
  S.sel = new Set([path]);
  S.focusPath = path;
}
function openDrill(key, i) {
  const b = D.lists[key]?.[i];
  if (!b) return;
  if (key === 'root') return go(b.path);
  setView(`${key}:${b.label}`);
  S.sorts.db = { sort: 'size', desc: true }; // a drill-down starts with the biggest files
  load();
}
$('#dashboard').addEventListener('click', (e) => {
  const m = e.target.closest('[data-dmode]');
  if (m) { const [k, v] = m.dataset.dmode.split(':'); D.mode[k] = v; store.set('dashModes', D.mode); return renderDash(); }
  const s = e.target.closest('[data-dsort]');
  if (s) {
    const [k, col] = s.dataset.dsort.split(':');
    const cur = D.sort[k] || { k: 'size', desc: true };
    D.sort[k] = cur.k === col ? { k: col, desc: !cur.desc } : { k: col, desc: col !== 'name' };
    return renderDash();
  }
  const bl = e.target.closest('[data-bigloc]');
  if (bl) return showInApp(D.data.largest[+bl.dataset.bigloc].path);
  const dp = e.target.closest('[data-dup]');
  if (dp) {
    const g = D.data.dup_groups[+dp.dataset.dup];
    setView('dup');
    S.q = g.md5;
    $('#q').value = g.md5;
    return load();
  }
  const a = e.target.closest('[data-dash]');
  if (a) {
    const act = a.dataset.dash;
    if (act === 'dup' || act === 'suggest' || act === 'bin') return setView(act);
    if (act === 'tidy') return openTidy(S.roots.find((r) => r.kind === 'Downloads')?.path);
    if (act === 'oldDownloads') { const dl = S.roots.find((r) => r.kind === 'Downloads'); if (dl) { S.sorts.browse = { sort: 'mtime', desc: false }; go(dl.path); } }
    return;
  }
  const big = e.target.closest('[data-big]');
  if (big) return showInApp(D.data.largest[+big.dataset.big].path);
  const o = e.target.closest('[data-open]');
  if (o) { const [k, i] = o.dataset.open.split(':'); openDrill(k, +i); }
});
$('#dashboard').addEventListener('keydown', (e) => {
  const o = e.target.closest('[data-open]');
  if (o && (e.key === 'Enter' || e.key === ' ')) { e.preventDefault(); const [k, i] = o.dataset.open.split(':'); openDrill(k, +i); }
});
// hover tooltip for every chart mark (and table rows that carry one)
document.addEventListener('mousemove', (e) => {
  const t = e.target.closest?.('[data-tip]');
  const tip = $('#tip');
  if (!t) { tip.hidden = true; return; }
  tip.textContent = t.dataset.tip;
  tip.hidden = false;
  const r = tip.getBoundingClientRect();
  tip.style.left = `${Math.min(e.clientX + 14, innerWidth - r.width - 8)}px`;
  tip.style.top = `${Math.min(e.clientY + 16, innerHeight - r.height - 8)}px`;
});
$('#subtitle').addEventListener('click', (e) => { if (e.target.closest('[data-back-dash]')) openDash(); });

// ================================================================ properties
async function showProps(path) {
  const d = $('#dialog');
  const close = () => { d.hidden = true; d.innerHTML = ''; document.removeEventListener('keydown', key, true); $('#list').focus(); };
  const key = (e) => { if (e.key === 'Escape') { e.stopPropagation(); close(); } };
  document.addEventListener('keydown', key, true);
  d.innerHTML = `<div class="dialog" role="dialog" aria-modal="true" aria-label="Properti"><div class="body"><div class="props-head">${folderIcon}<h2>${esc(baseName(path))}</h2></div>
    <p style="margin-top:14px"><span class="ring"></span>Menghitung…</p></div></div>`;
  d.hidden = false;
  let p;
  try { p = await invoke('properties', { path }); } catch (e) { close(); return toast(String(e), { error: true }); }
  if (d.hidden) return;
  const when = (t) => (t ? df.format(new Date(t * 1000)) : '—');
  const attrs = [p.readonly && 'Hanya-baca', p.hidden && 'Tersembunyi'].filter(Boolean).join(', ') || '—';
  d.innerHTML = `<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="pp-t"><div class="body">
      <div class="props-head">${fileIcon(p.cat)}<h2 id="pp-t">${esc(p.name)}</h2></div>
      <dl class="props">
        <dt>Jenis</dt><dd>${esc(p.is_dir ? 'Folder' : `${(CAT[p.cat] || CAT.Lainnya)[0]} (.${esc(p.name.split('.').pop())})`)}</dd>
        <dt>Lokasi</dt><dd>${esc(parentOf(p.path))}</dd>
        <dt>Ukuran</dt><dd>${human(p.size)} (${nf.format(p.size)} byte)</dd>
        ${p.is_dir ? `<dt>Isi</dt><dd>${nf.format(p.files)} file, ${nf.format(p.folders)} folder</dd>` : ''}
        <dt>Dibuat</dt><dd>${when(p.created)}</dd>
        <dt>Diubah</dt><dd>${when(p.modified)}</dd>
        <dt>Diakses</dt><dd>${when(p.accessed)}</dd>
        <dt>Atribut</dt><dd>${esc(attrs)}</dd>
        ${p.md5 ? `<dt>MD5</dt><dd class="mono" style="color:var(--text)">${esc(p.md5)}</dd>` : ''}
      </dl></div>
    <div class="foot"><button class="btn std" data-p="win">Properti Windows…</button><button class="btn std" data-p="copy">Salin path</button><button class="btn primary" data-p="close">Tutup</button></div></div>`;
  d.querySelector('[data-p="close"]').focus();
  d.querySelector('[data-p="close"]').onclick = close;
  d.querySelector('[data-p="copy"]').onclick = async () => { await navigator.clipboard.writeText(p.path); toast('Path disalin'); };
  d.querySelector('[data-p="win"]').onclick = () => { invoke('shell_properties', { path: p.path }); close(); };
}

// ================================================================ drag & drop (inside the app, to and from Explorer)
const sameDrive = (a, b) => a.slice(0, 2).toLowerCase() === b.slice(0, 2).toLowerCase();
let dropEl = null;
function setDrop(el) {
  if (dropEl === el) return;
  dropEl?.classList.remove('drop');
  dropEl = el;
  el?.classList.add('drop');
}
/** Folder under the cursor that `paths` may be dropped into: a folder row, a root in the nav pane, or a crumb. */
function dropTargetAt(x, y, paths) {
  const el = document.elementFromPoint(x, y);
  const ok = (dest) => dest && !paths.some((p) => dest === p || dest.startsWith(p + '\\')) && !(paths.length && paths.every((p) => parentOf(p) === dest));
  const tr = el?.closest('#rows tr.row');
  if (tr) { const r = S.vis[+tr.dataset.i]; if (r?.is_dir && !r.synthetic && ok(r.path)) return { el: tr, path: r.path }; }
  const nav = el?.closest('.nav-item');
  if (nav?.dataset.view.includes('\\') && ok(nav.dataset.view)) return { el: nav, path: nav.dataset.view };
  const cr = el?.closest('#crumbs [data-go]');
  if (cr && ok(cr.dataset.go)) return { el: cr, path: cr.dataset.go };
  return null;
}
/** Explorer rule: same drive = move, another drive (or Ctrl held) = copy. */
async function dropPaths(paths, dest, forceCopy) {
  const cut = !forceCopy && sameDrive(paths[0], dest);
  try {
    const r = await invoke('paste_items', { paths, dest, cut });
    if (r.batch) S.undo.push(r.batch);
    if (r.done) toast(`${r.done} item ${cut ? 'dipindah' : 'disalin'} ke ${baseName(dest)}`, { action: undoAction(r.batch) });
    if (r.failed.length) toast(r.failed.join('\n'), { error: true });
  } catch (e) { toast(String(e), { error: true }); }
  load();
}
function endDrag() {
  S.drag = null;
  $('#ghost').hidden = true;
  setDrop(null);
}
document.addEventListener('mousemove', (e) => {
  if (!S.dragStart || !(e.buttons & 1)) return;
  if (!S.drag) {
    if (Math.hypot(e.clientX - S.dragStart.x, e.clientY - S.dragStart.y) < 6) return;
    if (S.pendingSelect != null) { S.pendingSelect = null; }
    const items = realSel();
    if (!items.length) { S.dragStart = null; return; }
    S.drag = { paths: items.map((r) => r.path), target: null };
    $('#ghost').innerHTML = `${items.length === 1 ? fileIcon(items[0].is_dir ? 'Folder' : items[0].cat) : icon('copy')}<span>${esc(items.length === 1 ? items[0].name : `${items.length} item`)}</span><b></b>`;
    $('#ghost').hidden = false;
  }
  // Leaving the window hands the drag to Windows: drop onto Explorer, the desktop, an e-mail…
  if (e.clientX <= 0 || e.clientY <= 0 || e.clientX >= innerWidth - 1 || e.clientY >= innerHeight - 1) {
    const paths = S.drag.paths;
    endDrag();
    S.dragStart = null;
    invoke('drag_out', { paths }).catch((err) => toast(String(err), { error: true }));
    return;
  }
  const g = $('#ghost');
  g.style.left = `${e.clientX + 14}px`;
  g.style.top = `${e.clientY + 12}px`;
  const t = dropTargetAt(e.clientX, e.clientY, S.drag.paths);
  setDrop(t?.el);
  S.drag.target = t;
  $('#ghost b').textContent = t ? `${e.ctrlKey || !sameDrive(S.drag.paths[0], t.path) ? 'Salin' : 'Pindah'} ke ${baseName(t.path)}` : '';
});
document.addEventListener('mouseup', (e) => {
  if (e.button !== 0) return;
  if (S.drag) {
    const { paths, target } = S.drag;
    endDrag();
    if (target) dropPaths(paths, target.path, e.ctrlKey);
  } else if (S.pendingSelect != null) {
    select(S.pendingSelect); // plain click on an already selected row: now select only that row (like Explorer)
  }
  S.dragStart = null;
  S.pendingSelect = null;
});

// ================================================================ Pengaturan: optional local AI
const AI = { status: null, prog: null };
async function openSettings() {
  T.on = false;
  setPane('settings');
  renderSettings();
  await refreshAi();
}
async function refreshAi() {
  try { AI.status = await invoke('ai_status'); } catch (e) { toast(String(e), { error: true }); }
  if (S.pane === 'settings') renderSettings();
  renderNav();
}

function renderSettings() {
  const el = $('#settings');
  const head = '<div class="page-head"><h1>Pengaturan</h1><p>Semua fitur utama berjalan tanpa AI. AI lokal hanya tambahan dan mati sampai kamu menyalakannya.</p></div>';
  const st = AI.status;
  if (!st) { el.innerHTML = head + '<div class="result"><span class="ring lg"></span><b>Memeriksa komputer ini…</b></div>'; return; }
  const m = st.model;
  const a = st.advice;
  const pr = AI.prog;
  const running = st.busy || (pr && (pr.state === 'download' || pr.state === 'verify'));
  const pct = pr?.total ? Math.min(100, Math.floor((pr.done * 100) / pr.total)) : 0;
  const lvlIcon = { ok: 'check', slow: 'warn', no: 'error' }[a.level];
  let actions;
  if (running) {
    actions = `<div class="bar"><i style="width:${pct}%"></i></div>
      <div class="meta">${pr?.state === 'verify' ? 'Memeriksa checksum' : 'Mengunduh'} — ${human(pr?.done || 0)} dari ${human(pr?.total || m.size)} (${pct}%)</div>
      <div class="actions"><button class="btn std" data-s="cancel">Batal</button></div>`;
  } else if (m.path) {
    actions = `<div class="meta">${icon('check')} Siap dipakai: ${esc(m.path)}</div>
      <div class="actions">${m.owned ? `<button class="btn std" data-s="remove">Hapus model (${human(m.size)})</button>` : '<button class="btn std" data-s="remove">Jangan pakai file ini lagi</button>'}</div>`;
  } else {
    actions = `<div class="meta">Sudah punya file model ini? Pilih saja file .gguf-nya — dicek checksum dulu, tidak disalin.</div>
      ${m.partial ? `<div class="meta">Unduhan sebelumnya berhenti di ${human(m.partial)} — akan dilanjutkan, bukan diulang.</div>` : ''}
      <div class="actions">
        <button class="btn primary" data-s="download">${m.partial ? 'Lanjutkan unduhan' : `Unduh (${human(m.size)})`}</button>
        <button class="btn std" data-s="pick">Pilih file .gguf…</button></div>`;
  }
  el.innerHTML = head + `<div class="settings-body">
    <h2>AI lokal (opsional)
      <label class="toggle"><span>${st.enabled ? 'Aktif' : 'Mati'}</span><input type="checkbox" data-s="toggle" ${st.enabled ? 'checked' : ''} ${st.engine && m.path && !running ? '' : 'disabled'} aria-label="Aktifkan AI lokal"></label></h2>
    <p>Yang ditambahkan AI: (1) dokumen baru di <b>Saran</b> diarahkan ke subfolder topik yang sudah ada di Dokumen, dan (2) tombol <b>Kelompokkan per topik</b> di Rapikan folder.
      AI berjalan sepenuhnya di komputer ini: isi dokumen hanya dibaca di memori, tidak dikirim ke internet dan tidak disimpan.
      Internet hanya dipakai sekali saat kamu memilih mengunduh model.</p>
    ${st.engine ? '' : '<div class="infobar no">' + icon('error') + '<div><b>Versi aplikasi ini dibuat tanpa mesin AI</b><ul><li>Build ulang dengan <code>cargo build --release --features local-llm</code> (butuh Vulkan SDK) untuk memakai AI.</li></ul></div></div>'}
    <div class="infobar ${a.level}">${icon(lvlIcon)}<div><b>Hardware komputer ini: ${esc(a.title)}</b><ul>${a.points.map((p) => `<li>${esc(p)}</li>`).join('')}</ul></div></div>
    <div class="card section"><b>Komputer ini dibanding kebutuhan</b>
      <table class="spec"><thead><tr><th style="width:110px">Komponen</th><th>Komputer ini</th><th style="width:150px">Minimum</th><th style="width:170px">Disarankan</th></tr></thead>
      <tbody>${a.checks.map((c) => `<tr><td><span class="st ${c.state}"></span>${esc(c.label)}</td><td>${esc(c.have)}</td><td>${esc(c.min)}</td><td>${esc(c.rec)}</td></tr>`).join('')}</tbody></table>
      <div class="meta reason">Diperiksa saat halaman ini dibuka. <button class="btn std" data-s="refresh" style="height:26px;margin-left:6px">${icon('refresh')}Periksa ulang</button></div></div>
    <div class="card section model">
      <div class="row1">${icon('sparkle')}<b>${esc(m.label)}</b><span class="count">${human(m.size)}</span></div>
      <div class="meta">${esc(m.purpose)}</div>
      <div class="meta">Sumber: ${esc(m.source)} · SHA-256 ${esc(m.sha256.slice(0, 16))}… (setiap byte dicek sebelum dipakai)</div>
      ${actions}
    </div>
    <p>Hanya model ini yang dibutuhkan; tidak ada model lain yang perlu diunduh.
      Saat AI berjalan, model memakai ±2 GB RAM/VRAM dan dilepas lagi setelah 5 menit tidak dipakai. Jika RAM tersedia di bawah 2 GB, AI menolak berjalan.</p>
    <h2>Tentang</h2>
    <p>File Organizer 0.1.0 — perangkat lunak bebas di bawah lisensi GNU GPL versi 3 atau yang lebih baru.
      Program ini disediakan tanpa garansi apa pun. Indeks dan pengaturan tersimpan di %LOCALAPPDATA%\\file-organizer.</p>
  </div>`;
}

$('#settings').addEventListener('click', async (e) => {
  const b = e.target.closest('[data-s]');
  if (!b || b.matches('input')) return;
  const st = AI.status;
  const warn = st && st.advice.level !== 'ok' ? `\n\nPerhatian — ${st.advice.title}:\n• ${st.advice.points.join('\n• ')}` : '';
  try {
    switch (b.dataset.s) {
      case 'download':
        if (!(await confirmDialog(`Unduh model AI (${human(st.model.size)})?`, `Sumber: ${st.model.source}. Unduhan bisa dibatalkan dan dilanjutkan nanti; file dicek SHA-256 sebelum dipakai.${warn}`, 'Unduh'))) return;
        AI.prog = { state: 'download', done: st.model.partial, total: st.model.size };
        await invoke('ai_download');
        break;
      case 'pick': {
        const path = await invoke('pick_model_file');
        if (!path) return;
        AI.prog = { state: 'verify', done: 0, total: st.model.size };
        await invoke('ai_use_existing', { path });
        break;
      }
      case 'cancel': await invoke('ai_cancel'); return;
      case 'remove':
        if (!(await confirmDialog(st.model.owned ? 'Hapus model?' : 'Berhenti memakai file ini?', st.model.owned ? `File model (${human(st.model.size)}) dihapus dan AI dimatikan. Bisa diunduh lagi kapan saja.` : 'AI dimatikan. File-nya tidak dihapus karena bukan diunduh oleh aplikasi ini.', st.model.owned ? 'Hapus' : 'OK'))) return;
        await invoke('ai_remove');
        break;
      case 'refresh': AI.status = null; renderSettings(); break;
    }
  } catch (err) { toast(String(err), { error: true }); }
  refreshAi();
});
$('#settings').addEventListener('change', async (e) => {
  if (!e.target.matches('[data-s="toggle"]')) return;
  const on = e.target.checked;
  const st = AI.status;
  if (on && st.advice.level !== 'ok' && !(await confirmDialog('Nyalakan AI lokal?', `${st.advice.title}:\n• ${st.advice.points.join('\n• ')}`, 'Tetap nyalakan'))) { e.target.checked = false; return; }
  try { await invoke('ai_set_enabled', { on }); toast(on ? 'AI lokal aktif' : 'AI lokal dimatikan'); } catch (err) { toast(String(err), { error: true }); }
  refreshAi();
});
listen('ai-model', (e) => {
  AI.prog = e.payload;
  const s = e.payload.state;
  if (s === 'done' || s === 'error' || s === 'cancelled') {
    AI.prog = null;
    toast(e.payload.msg || (s === 'cancelled' ? 'Dibatalkan' : ''), { error: s === 'error' });
    refreshAi();
  } else if (S.pane === 'settings' && AI.status) {
    AI.status.busy = true;
    renderSettings();
  }
});

// ================================================================ Rapikan folder (pick → plan → done)
const KIND = {
  pindah: ['Pindah', 'var(--accent)'], 'ganti-nama': ['Ganti nama', '#3a96dd'], duplikat: ['Duplikat', '#da7b11'],
  kosong: ['Kosong', '#8a8a8a'], versi: ['Versi lama', '#8764b8'], sensitif: ['Sensitif', '#c42b1c'], curiga: ['Curiga', '#c42b1c'],
  proyek: ['Proyek', '#038387'], program: ['Program', '#5c2d91'], biarkan: ['Biarkan', '#8a8a8a'],
};
const FILTERS = [
  ['all', 'Semua', () => true],
  ['move', 'Pindah', (k) => ['pindah', 'ganti-nama', 'proyek', 'program'].includes(k)],
  ['dup', 'Duplikat & kosong', (k) => ['duplikat', 'kosong'].includes(k)],
  ['safe', 'Keamanan', (k) => ['sensitif', 'curiga'].includes(k)],
  ['check', 'Perlu dicek', (k) => ['versi', 'biarkan'].includes(k)],
];
const TARGETS = ['Dokumen', 'Gambar', 'Spreadsheet', 'Arsip', 'Program', 'Kode', 'Model 3D', 'Musik', 'Video', 'Kontak', 'Lainnya', 'Proyek', '_Duplikat', '_Sensitif', '_Karantina', '_Kosong', '_Versi Lama'];
const T = { on: false, step: 'pick', root: null, checked: new Set(), open: new Set(), kids: new Map(), items: [], filter: 'all', sel: new Set(), anchor: -1, result: null, busy: false };

function openTidy(dir) {
  T.on = true;
  T.root = dir || S.roots.find((r) => r.name === 'Downloads')?.path || S.roots[0]?.path;
  T.checked = new Set([T.root]);
  T.open = new Set([T.root]);
  T.kids.clear();
  T.step = 'pick';
  T.items = [];
  T.result = null;
  setPane('tidy');
  loadKids(T.root).then(renderTidy);
  renderTidy();
}
/** Back to the explorer from the wizard or Pengaturan. */
function closeTidy() {
  T.on = false;
  if (S.pane !== 'explorer') setPane('explorer');
}
async function loadKids(p) {
  if (T.kids.has(p)) return;
  try { T.kids.set(p, sortByName((await invoke('browse', { path: p, showHidden: S.showHidden })).filter((e) => e.is_dir))); } catch { T.kids.set(p, []); }
}
const sortByName = (l) => l.sort((a, b) => cmpText(a.name, b.name));

function renderTidy() {
  const el = $('#tidy');
  const rootName = pretty(T.root || '');
  if (T.aiRun) {
    const a = T.aiRun;
    const phase = { read: 'Membaca isi awal dokumen…', topics: 'Menyusun daftar topik…', assign: `Menempatkan dokumen ${a.done + 1} dari ${a.total}…`, done: 'Selesai' }[a.phase] || 'Menyiapkan AI…';
    const pct = a.total ? Math.floor((a.done * 100) / a.total) : 0;
    el.innerHTML = `<div class="page-head"><h1>Kelompokkan per topik</h1><p>${esc(rootName)}</p></div>
      <div class="result"><span class="ring lg"></span><b>${esc(phase)}</b>
        <div class="bar" style="width:320px"><i style="width:${pct}%"></i></div>
        <p class="reason">AI lokal berjalan di komputer ini. Hasilnya hanya mengisi kolom Tujuan — belum ada yang dipindah.</p>
        <button class="btn std" data-t="ai-cancel">Batal</button></div>`;
    return;
  }
  if (T.busy) {
    el.innerHTML = `<div class="page-head"><h1>Rapikan folder</h1><p>${esc(rootName)}</p></div>
      <div class="result"><span class="ring lg"></span><b>${T.step === 'pick' ? 'Menganalisis…' : 'Memindahkan…'}</b><p class="reason">${T.step === 'pick' ? 'Membaca jenis, isi folder, dan MD5 untuk mencari duplikat. Belum ada yang dipindah.' : 'Jangan tutup aplikasi.'}</p></div>`;
    return;
  }
  if (T.step === 'pick') return renderPick(el, rootName);
  if (T.step === 'plan') return renderPlan(el, rootName);
  renderDone(el, rootName);
}

function renderPick(el, rootName) {
  const rows = [];
  const walk = (p, name, depth) => {
    const kids = T.kids.get(p);
    const open = T.open.has(p);
    rows.push(`<div class="trow" style="padding-left:${8 + depth * 22}px" data-p="${esc(p)}">
      <button class="twisty${open ? ' open' : ''}${kids && !kids.length ? ' none' : ''}" data-tw aria-label="${open ? 'Tutup' : 'Buka'}">${icon('chev')}</button>
      <input type="checkbox" data-cb ${T.checked.has(p) ? 'checked' : ''} aria-label="Rapikan isi ${esc(name)}">
      <span class="name">${folderIcon}<span>${esc(name)}</span></span>
      ${T.checked.has(p) ? `<span class="hint">${depth ? 'isinya ikut dirapikan' : 'file & subfolder langsung dirapikan'}</span>` : ''}</div>`);
    if (open) (kids || []).forEach((k) => walk(k.path, k.name, depth + 1));
  };
  if (T.root) walk(T.root, rootName, 0);
  const opts = S.roots.map((r) => `<option value="${esc(r.path)}"${r.path === T.root ? ' selected' : ''}>${esc(r.name)}</option>`).join('') +
    (S.roots.some((r) => r.path === T.root) ? '' : `<option value="${esc(T.root)}" selected>${esc(T.root)}</option>`);
  el.innerHTML = `<div class="page-head"><h1>Rapikan folder</h1>
      <p>Centang folder yang isinya mau dirapikan. Folder yang tidak dicentang isinya tidak diubah — paling jauh dipindah utuh.</p></div>
    <div class="tidy-body">
      <div class="field"><label for="t-root">Folder</label><select id="t-root" class="input">${opts}</select><button class="btn std" data-t="browse">Telusuri…</button></div>
      <div class="tree card" role="tree">${rows.join('')}</div>
    </div>
    <div class="tidy-foot"><button class="btn std" data-t="cancel">Batal</button><span class="grow"></span>
      <span class="info">${T.checked.size} folder dicentang</span>
      <button class="btn primary" data-t="analyze" ${T.checked.size ? '' : 'disabled'}>${icon('sparkle')}Buat rencana</button></div>`;
}

const visibleItems = () => {
  const f = FILTERS.find((x) => x[0] === T.filter)[2];
  return T.items.filter((it) => f(it.kind));
};

function renderPlan(el, rootName) {
  const counts = Object.fromEntries(FILTERS.map(([k, , f]) => [k, T.items.filter((it) => f(it.kind)).length]));
  const items = visibleItems();
  const bases = [...new Set(items.map((it) => it.base))];
  let body = '';
  for (const base of bases) {
    if (bases.length > 1 || T.checked.size > 1) body += `<tr class="group"><td colspan="5">${folderIcon.replace('class="fi"', 'class="fi" style="vertical-align:-4px;margin-right:6px"')}${esc(pretty(base))}</td></tr>`;
    for (const it of items.filter((x) => x.base === base)) {
      const i = T.items.indexOf(it);
      const [label, color] = KIND[it.kind] || KIND.pindah;
      body += `<tr class="row two${it.checked ? '' : ' off'}" data-ti="${i}" aria-selected="${T.sel.has(i)}">
        <td style="text-align:center"><input type="checkbox" data-ck ${it.checked ? 'checked' : ''} aria-label="Sertakan ${esc(it.name)}"></td>
        <td title="${esc(it.src)}\n${esc(it.reason)}"><div class="name">${it.is_dir ? folderIcon : fileIcon(catOfName(it.name))}
          <div class="two-l"><span>${esc(it.name)}</span><small>${esc(it.reason)}</small></div></div></td>
        <td><span class="kind"><span class="dot" style="background:${color}"></span>${label}</span></td>
        <td class="tg"><input class="input${it.bad ? ' bad' : ''}" list="targets" data-tg value="${esc(it.target)}" placeholder="tetap di sini" aria-label="Tujuan ${esc(it.name)}"></td>
        <td class="num">${it.size ? human(it.size) : ''}</td></tr>`;
    }
  }
  const n = T.items.filter((it) => it.checked && it.target && !it.bad).length;
  const pills = FILTERS.map(([k, label]) => `<button class="pill" data-f="${k}" aria-pressed="${T.filter === k}">${label}<span class="n">${counts[k]}</span></button>`).join('');
  const listTop = $('#tidy .plan-grid .list')?.scrollTop || 0;
  el.innerHTML = `<div class="page-head"><h1>Rencana: ${esc(rootName)}</h1>
      <p>Periksa dan ubah dulu. Tidak ada yang dihapus — duplikat hanya dipindah ke _Duplikat, dan semuanya bisa diurungkan.</p></div>
    <div class="tidy-body">
      <div class="pills">${pills}<button class="btn std pill-ai" data-t="ai-group" title="Opsional: AI lokal mengusulkan subfolder topik untuk dokumen">${icon('sparkle')}Kelompokkan dokumen per topik (AI)</button></div>
      <div class="bulk" ${T.sel.size > 1 ? '' : 'hidden'}><b>${T.sel.size} baris dipilih</b><span>Tujuan:</span>
        <input class="input" list="targets" id="bulk-tg" placeholder="mis. Dokumen\\Pajak"><button class="btn std" data-t="bulk-set">Terapkan</button>
        <button class="btn std" data-t="bulk-on">Centang</button><button class="btn std" data-t="bulk-off">Lepas centang</button></div>
      <div class="plan-grid">
        <div class="list card" tabindex="0" id="plan-list"><table><colgroup><col style="width:44px"><col><col style="width:116px"><col style="width:190px"><col style="width:84px"></colgroup>
          <thead><tr><th></th><th>Item · alasan</th><th>Aksi</th><th>Tujuan</th><th class="num">Ukuran</th></tr></thead>
          <tbody>${body || `<tr><td colspan="5" style="text-align:center;padding:40px;color:var(--text3)">Tidak ada item di filter ini.</td></tr>`}</tbody></table></div>
        <aside class="side card" id="plan-side">${renderSide()}</aside>
      </div>
    </div>
    <div class="tidy-foot"><button class="btn std" data-t="back">${icon('back')}Kembali</button><span class="grow"></span>
      <span class="info">${n} dari ${T.items.length} item akan dipindah</span>
      <button class="btn primary" data-t="execute" ${n ? '' : 'disabled'}>Terapkan ${n} perubahan</button></div>`;
  $('#plan-list').scrollTop = listTop;
  $('#targets').innerHTML = [...new Set([...TARGETS, ...T.items.map((it) => it.target).filter(Boolean)])].map((t) => `<option value="${esc(t)}">`).join('');
}
const catOfName = (name) => {
  const ext = name.includes('.') ? name.split('.').pop().toLowerCase() : '';
  const map = { Gambar: 'png jpg jpeg webp svg ico gif bmp heic', Dokumen: 'pdf docx doc pptx ppt txt md rtf', Spreadsheet: 'xlsx xls csv', Arsip: 'zip rar 7z gz tgz tar', Program: 'exe msi', Kode: 'html php js css json sql xml py', Musik: 'mp3 wav m4a flac', Video: 'mp4 mkv mov avi' };
  return Object.keys(map).find((k) => map[k].split(' ').includes(ext)) || 'Lainnya';
};

/** Right pane: destination folders (rename = all rows follow) and a preview of the result. */
function renderSide() {
  const on = T.items.filter((it) => it.checked && it.target && !it.bad);
  const tops = new Map();
  for (const it of on) { const t = it.target.split(/[\\/]/)[0]; tops.set(t, (tops.get(t) || 0) + 1); }
  const tg = [...tops].sort((a, b) => b[1] - a[1]).map(([t, n]) =>
    `<div class="tgt-row" data-top="${esc(t)}"><span class="dot" style="width:8px;height:8px;border-radius:50%;background:${catColor(t)}"></span><span class="grow">${esc(t)}</span><span class="count">${n}</span>
      <button class="btn icon" data-t="rename-top" title="Ganti nama folder tujuan ini" aria-label="Ganti nama ${esc(t)}">${icon('rename')}</button></div>`).join('');
  const bases = [...new Set(T.items.map((it) => it.base))];
  const pv = bases.map((b) => {
    const mine = on.filter((it) => it.base === b);
    const per = new Map();
    mine.forEach((it) => { const t = it.target.split(/[\\/]/)[0]; per.set(t, (per.get(t) || 0) + 1); });
    const stay = T.items.filter((it) => it.base === b && !(it.checked && it.target && !it.bad)).length;
    return `<div class="pv">${folderIcon}<b>${esc(baseName(b))}</b></div>` +
      [...per].sort((a, c) => cmpText(a[0], c[0])).map(([t, n]) => `<div class="pv" style="padding-left:22px">${folderIcon}${esc(t)}<span class="plus">+${n}</span></div>`).join('') +
      (stay ? `<div class="pv muted" style="padding-left:22px">${stay} item tetap di tempat</div>` : '');
  }).join('');
  return `<h3>Folder tujuan</h3>${tg || '<div class="reason">Belum ada.</div>'}<h3>Pratinjau</h3><div class="preview">${pv}</div>`;
}

function renderDone(el, rootName) {
  const r = T.result;
  el.innerHTML = `<div class="page-head"><h1>Rapikan folder</h1><p>${esc(rootName)}</p></div>
    <div class="result">${icon(r.failed.length ? 'warn' : 'check')}
      <h2 style="margin:0">${r.done} item dipindahkan</h2>
      <p class="reason">${r.failed.length ? `${r.failed.length} item gagal:` : 'Semua langkah tercatat dan bisa diurungkan.'}</p>
      ${r.failed.length ? `<div class="fails">${esc(r.failed.join('\n'))}</div>` : ''}
      <div style="display:flex;gap:8px;margin-top:12px">
        ${r.batch ? `<button class="btn std" data-t="undo">${icon('undo')}Urungkan semua</button>` : ''}
        <button class="btn std" data-t="open-root">${icon('folder')}Buka folder</button>
        <button class="btn primary" data-t="finish">Selesai</button></div></div>`;
}

$('#tidy').addEventListener('click', async (e) => {
  const tw = e.target.closest('[data-tw]');
  if (tw) {
    const p = tw.closest('[data-p]').dataset.p;
    if (T.open.has(p)) T.open.delete(p); else { T.open.add(p); await loadKids(p); }
    return renderTidy();
  }
  const f = e.target.closest('[data-f]');
  if (f) { T.filter = f.dataset.f; T.sel.clear(); return renderTidy(); }
  const tb = e.target.closest('[data-t]');
  if (tb) return tidyCmd(tb.dataset.t, tb);
  const tr = e.target.closest('tr[data-ti]');
  if (tr && !e.target.closest('input')) {
    const i = +tr.dataset.ti;
    const vis = visibleItems().map((it) => T.items.indexOf(it));
    if (e.shiftKey && T.anchor >= 0) {
      const [a, b] = [vis.indexOf(T.anchor), vis.indexOf(i)].sort((x, y) => x - y);
      if (!e.ctrlKey) T.sel.clear();
      vis.slice(a, b + 1).forEach((k) => T.sel.add(k));
    } else if (e.ctrlKey) { T.sel.has(i) ? T.sel.delete(i) : T.sel.add(i); T.anchor = i; }
    else { T.sel = new Set([i]); T.anchor = i; }
    renderTidy();
  }
});
$('#tidy').addEventListener('change', (e) => {
  if (e.target.matches('[data-cb]')) {
    const p = e.target.closest('[data-p]').dataset.p;
    e.target.checked ? T.checked.add(p) : T.checked.delete(p);
    return renderTidy();
  }
  if (e.target.id === 't-root') { openTidy(e.target.value); return; }
  const tr = e.target.closest('tr[data-ti]');
  if (!tr) return;
  const it = T.items[+tr.dataset.ti];
  if (e.target.matches('[data-ck]')) {
    const rows = T.sel.has(+tr.dataset.ti) ? [...T.sel] : [+tr.dataset.ti]; // ticking a selected row ticks the whole selection
    rows.forEach((k) => (T.items[k].checked = e.target.checked && !!T.items[k].target && !T.items[k].bad));
  } else if (e.target.matches('[data-tg]')) {
    setTarget(it, e.target.value);
  }
  renderTidy();
});
function setTarget(it, v) {
  it.target = v.trim().replace(/\//g, '\\');
  it.bad = !!it.target && badTarget(it.target);
  if (!it.target || it.bad) it.checked = false;
  else { if (it.kind === 'biarkan') it.kind = 'pindah'; it.checked = true; }
}

async function tidyCmd(cmd, btn) {
  switch (cmd) {
    case 'cancel':
    case 'finish':
      closeTidy();
      return load();
    case 'browse': {
      const p = await invoke('pick_folder');
      if (p) openTidy(p);
      return;
    }
    case 'analyze': {
      T.busy = true;
      renderTidy();
      try {
        const items = await invoke('analyze_folders', { root: T.root, folders: [...T.checked] });
        T.items = items.map((it) => ({ ...it, bad: false }));
        T.step = 'plan';
        T.filter = 'all';
        T.sel.clear();
      } catch (e) { toast(String(e), { error: true }); }
      T.busy = false;
      return renderTidy();
    }
    case 'back': T.step = 'pick'; return renderTidy();
    case 'ai-cancel': return invoke('ai_cancel');
    case 'ai-group': {
      if (!AI.status) await refreshAi();
      if (!AI.status?.enabled) {
        if (await confirmDialog('AI lokal belum aktif', 'Fitur ini opsional dan butuh model AI di komputer ini. Buka Pengaturan untuk melihat kebutuhan, saran untuk komputer ini, dan pilihan unduhan?', 'Buka Pengaturan')) openSettings();
        return;
      }
      const docs = T.items.filter((it) => !it.is_dir && it.checked && /^(dokumen|documents)$/i.test(it.target.split('\\')[0]) && /\.(pdf|docx|txt|md|rtf)$/i.test(it.name));
      if (!docs.length) return toast('Tidak ada dokumen (PDF/DOCX/TXT/MD) bertujuan Dokumen yang dicentang di rencana ini.');
      await refreshAi(); // fresh RAM / CPU numbers for the warning
      const st = AI.status;
      const gpu = st.advice.checks.find((c) => c.label === 'GPU')?.state === 'ok';
      const mins = Math.max(1, Math.ceil((docs.length * (gpu ? 4 : 60)) / 60));
      const warn = st.advice.points.filter((p) => /RAM tersedia|sibuk|CPU,/.test(p));
      const ok = await confirmDialog('Kelompokkan per topik dengan AI?',
        `${docs.length} dokumen dibaca isi awalnya (hanya di memori, tidak disimpan) oleh AI lokal.\nPerkiraan waktu: ±${mins} menit ${gpu ? '(GPU)' : '(CPU — lambat)'}. RAM tersedia sekarang: ${human(st.sys.ram_avail)}.` +
        `${warn.length ? `\n\nPerhatian:\n• ${warn.join('\n• ')}` : ''}\n\nHasilnya hanya mengisi kolom Tujuan dan tetap bisa diedit.`, 'Mulai');
      if (!ok) return;
      // topic folders that already exist under each base's Dokumen/Documents are offered to the model first
      const existing = new Set();
      for (const base of new Set(docs.map((d) => d.base))) {
        for (const top of ['Dokumen', 'Documents']) {
          try { (await invoke('browse', { path: `${base}\\${top}` })).filter((x) => x.is_dir).forEach((x) => existing.add(x.name)); } catch { /* folder not there */ }
        }
      }
      T.aiRun = { done: 0, total: docs.length, phase: 'read' };
      renderTidy();
      try {
        const res = await invoke('ai_group', { items: docs.map((d) => ({ src: d.src, name: d.name })), existing: [...existing] });
        for (const r of res) {
          const it = T.items.find((x) => x.src === r.src);
          if (!it) continue;
          it.target = `${it.target.split('\\')[0]}\\${r.topic}`;
          it.reason = `AI: ${r.reason}`;
          it.bad = badTarget(it.target);
        }
        const topics = new Set(res.map((r) => r.topic));
        toast(res.length ? `${res.length} dokumen diberi tujuan di ${topics.size} topik — periksa kolom Tujuan.` : 'AI tidak menemukan pengelompokan yang cocok.');
      } catch (err) { toast(String(err), { error: true }); }
      T.aiRun = null;
      return renderTidy();
    }
    case 'bulk-set': {
      const v = $('#bulk-tg').value;
      if (badTarget(v)) return toast('Nama folder tujuan tidak valid.', { error: true });
      T.sel.forEach((k) => setTarget(T.items[k], v));
      return renderTidy();
    }
    case 'bulk-on':
    case 'bulk-off':
      T.sel.forEach((k) => { const it = T.items[k]; it.checked = cmd === 'bulk-on' && !!it.target && !it.bad; });
      return renderTidy();
    case 'rename-top': {
      const row = btn.closest('[data-top]');
      const old = row.dataset.top;
      row.innerHTML = `<input class="input" value="${esc(old)}" aria-label="Nama baru untuk ${esc(old)}">`;
      const inp = row.querySelector('input');
      inp.focus();
      inp.select();
      let done = false;
      const finish = (commit) => {
        if (done) return;
        done = true;
        const v = inp.value.trim();
        if (commit && v && v !== old) {
          if (badPart(v)) toast('Nama folder tidak valid.', { error: true });
          else T.items.forEach((it) => { const parts = it.target.split('\\'); if (parts[0] === old) { parts[0] = v; it.target = parts.join('\\'); } });
        }
        renderTidy();
      };
      inp.addEventListener('keydown', (e) => { e.stopPropagation(); if (e.key === 'Enter') finish(true); if (e.key === 'Escape') finish(false); });
      inp.addEventListener('blur', () => finish(true));
      return;
    }
    case 'execute': {
      const todo = T.items.filter((it) => it.checked && it.target && !it.bad);
      const ok = await confirmDialog('Terapkan rencana?', `${todo.length} item akan dipindah di ${pretty(T.root)}.\nTidak ada yang dihapus dan tidak ada yang ditimpa. Semua bisa diurungkan.`, `Pindahkan ${todo.length} item`);
      if (!ok) return;
      T.busy = true;
      T.step = 'run';
      renderTidy();
      try {
        T.result = await invoke('execute_plan', { items: todo.map(({ src, base, target, kind }) => ({ src, base, target, kind })) });
        if (T.result.batch) S.undo.push(T.result.batch);
      } catch (e) { T.result = { done: 0, failed: [String(e)], batch: null }; }
      T.busy = false;
      T.step = 'done';
      return renderTidy();
    }
    case 'undo':
      await undo(T.result.batch);
      T.result.batch = null;
      return renderTidy();
    case 'open-root': {
      const root = T.root;
      closeTidy();
      return go(root);
    }
  }
}

// ================================================================ column resize
$('#head').addEventListener('mousedown', (e) => {
  const grip = e.target.closest('[data-grip]');
  if (!grip) return;
  e.preventDefault();
  e.stopPropagation();
  const k = grip.dataset.grip;
  const c = COLS.find((x) => x.k === k);
  const col = $('#cols').children[cols().indexOf(c)];
  const x0 = e.clientX;
  const w0 = colW(c);
  grip.classList.add('drag');
  const move = (ev) => {
    widths[k] = Math.max(60, Math.round(w0 + ev.clientX - x0));
    col.style.width = `${widths[k]}px`;
    $('#grid').style.width = `${cols().reduce((a, x) => a + colW(x), 0)}px`;
  };
  const up = () => {
    grip.classList.remove('drag');
    removeEventListener('mousemove', move);
    removeEventListener('mouseup', up);
    store.set('widths', widths);
    S.justResized = true;
    setTimeout(() => (S.justResized = false), 50);
  };
  addEventListener('mousemove', move);
  addEventListener('mouseup', up);
});
$('#head').addEventListener('dblclick', (e) => {
  const grip = e.target.closest('[data-grip]');
  if (!grip) return;
  delete widths[grip.dataset.grip]; // double-click the divider: back to the default width
  store.set('widths', widths);
  render();
});

// ================================================================ events
$('#nav').addEventListener('click', (e) => {
  const add = e.target.closest('[data-addroot]');
  if (add) { const r = add.getBoundingClientRect(); return openAddMenu(r.left, r.bottom + 4); }
  const b = e.target.closest('.nav-item');
  if (!b) return;
  const v = b.dataset.view;
  if (v === 'settings') return openSettings();
  if (v === 'dash') return openDash();
  const root = S.roots.find((r) => r.path === v);
  if (root && !root.online) return toast(`"${root.name}" tidak terhubung. Colokkan drive-nya; folder ini otomatis aktif lagi.`, { error: true });
  v.includes('\\') ? go(v) : setView(v);
});
function showMenuAt(html, x, y) {
  const m = $('#menu');
  m.innerHTML = html;
  m.hidden = false;
  S.menuOpen = true;
  const r = m.getBoundingClientRect();
  m.style.left = `${Math.max(4, Math.min(x, innerWidth - r.width - 8))}px`;
  m.style.top = `${Math.max(4, Math.min(y, innerHeight - r.height - 8))}px`;
  m.querySelector('button:not(:disabled)')?.focus();
}
const mi = (attrs, ic, label, disabled = false, key = '') =>
  `<button role="menuitem" ${attrs}${disabled ? ' disabled' : ''}>${icon(ic)}${esc(label)}<span class="key">${key}</span></button>`;

$('#nav').addEventListener('contextmenu', (e) => {
  const b = e.target.closest('.nav-item');
  if (!b || !b.dataset.view.includes('\\')) return;
  e.preventDefault();
  e.stopPropagation();
  const p = b.dataset.view;
  const i = S.roots.findIndex((r) => r.path === p);
  const off = S.roots[i] && !S.roots[i].online;
  showMenuAt(
    mi(`data-navopen="${esc(p)}"`, 'open', 'Buka', off) + mi(`data-navtidy="${esc(p)}"`, 'sparkle', 'Rapikan folder ini…', off) +
    '<hr>' + mi(`data-rootmove="${i}:-1"`, 'up', 'Naikkan', i <= 0) + mi(`data-rootmove="${i}:1"`, 'down', 'Turunkan', i >= S.roots.length - 1) +
    '<hr>' + mi(`data-rootprops="${esc(p)}"`, 'info', 'Properti', off) +
    mi(`data-rootdel="${esc(p)}"`, 'remove', 'Hapus dari daftar'), e.clientX, e.clientY);
});

// ---------------------------------------------------------------- watched folders (navigation pane)
async function setRoots(paths, msg) {
  try {
    S.roots = await invoke('set_roots', { paths });
    if (isBrowse() && !rootOf(S.cwd)) setView('suggest'); // the open folder is no longer watched
    else { renderNav(); load(); }
    if (msg) toast(msg);
  } catch (e) { toast(String(e), { error: true }); }
}

/** Add a folder or drive; a parent replaces folders inside it (after confirmation); a child of a listed one is refused. */
async function addRoot(path, name, kind) {
  const inside = S.roots.find((r) => path === r.path || path.startsWith(withSep(r.path)));
  if (inside) return toast(`"${name}" sudah tercakup oleh "${inside.name}" di daftar.`, { error: true });
  const children = S.roots.filter((r) => r.path.startsWith(withSep(path)));
  const notes = [];
  if (children.length) notes.push(`Folder ini mencakup ${children.map((c) => `"${c.name}"`).join(', ')} — semuanya digabung ke dalamnya.`);
  if (kind === 'Drive' || kind === 'Home' || /^[A-Z]:\\$/i.test(path)) {
    notes.push('Seluruh isinya akan diindeks (nama + MD5 setiap file). Pertama kali bisa lama dan membuat komputer sibuk; folder sistem (Windows, Program Files, AppData, tersembunyi) dilewati.');
  }
  if (notes.length && !(await confirmDialog(`Tambahkan "${name}"?`, notes.join('\n\n'), 'Tambahkan'))) return;
  const paths = S.roots.filter((r) => !children.includes(r)).map((r) => r.path).concat(path);
  setRoots(paths, `"${name}" ditambahkan — sedang diindeks`);
}

async function openAddMenu(x, y) {
  let places = [];
  try { places = await invoke('places'); } catch (e) { return toast(String(e), { error: true }); }
  const row = (p) => mi(`data-addpath="${esc(p.path)}" data-addname="${esc(p.name)}" data-addkind="${p.kind}"`, p.added ? 'check' : KIND_ICON[p.kind] || 'folder', p.name, p.added, p.added ? 'sudah ada' : '');
  const folders = places.filter((p) => p.kind !== 'Drive');
  const drives = places.filter((p) => p.kind === 'Drive');
  showMenuAt(
    '<div class="menu-head">Folder</div>' + folders.map(row).join('') +
    (drives.length ? '<hr><div class="menu-head">Drive</div>' + drives.map(row).join('') : '') +
    '<hr>' + mi('data-addpick', 'folderAdd', 'Pilih folder lain…') + mi('data-addreset', 'refresh', 'Kembalikan daftar bawaan'), x, y);
}
$('#addr').addEventListener('click', (e) => {
  const n = e.target.closest('[data-nav]');
  if (n) return nav(n.dataset.nav);
  const g = e.target.closest('[data-go]');
  if (g) go(g.dataset.go);
});
$('#crumbs').addEventListener('dblclick', () => {
  const c = $('#crumbs');
  c.innerHTML = `<input value="${esc(S.cwd)}" aria-label="Path folder" spellcheck="false">`;
  const inp = c.querySelector('input');
  inp.focus();
  inp.select();
  inp.addEventListener('keydown', (e) => {
    e.stopPropagation();
    if (e.key === 'Enter') go(inp.value.trim().replace(/\\+$/, '') || S.cwd);
    if (e.key === 'Escape') renderHead();
  });
  inp.addEventListener('blur', () => renderHead());
});
$('#head').addEventListener('click', (e) => {
  if (S.justResized || e.target.closest('[data-grip]')) return;
  const th = e.target.closest('th[data-sort]');
  if (!th || !th.dataset.sort) return;
  const s = sortOf();
  if (s.sort === th.dataset.sort) s.desc = !s.desc;
  else { s.sort = th.dataset.sort; s.desc = s.sort === 'mtime' || s.sort === 'size'; }
  if (browsing()) { build(); render(); } else load();
});
$('#rows').addEventListener('mousedown', (e) => {
  const tr = e.target.closest('tr.row');
  if (!tr) return;
  const i = +tr.dataset.i;
  if (e.target.closest('[data-twisty]')) { e.preventDefault(); return toggle(S.vis[i]); }
  if (e.button === 2) { if (!S.sel.has(S.vis[i].path)) select(i); return; }
  if (e.button !== 0 || e.target.closest('input')) return;
  e.preventDefault(); // no text selection while dragging
  $('#list').focus();
  // Like Explorer: pressing on an already selected row keeps the selection, so several items can be dragged.
  if (S.sel.has(S.vis[i].path) && !e.shiftKey && !e.ctrlKey) S.pendingSelect = i;
  else select(i, { shift: e.shiftKey, ctrl: e.ctrlKey });
  if (!S.vis[i].synthetic) S.dragStart = { x: e.clientX, y: e.clientY };
});
$('#rows').addEventListener('dblclick', (e) => {
  const tr = e.target.closest('tr.row');
  if (tr && !e.target.closest('[data-twisty]')) activate(S.vis[+tr.dataset.i]);
});
$('#list').addEventListener('mousedown', (e) => {
  if (e.target.closest('tr, th') || e.button !== 0) return;
  resetSel();
  paintSel(false); // click on empty space clears the selection, like Explorer
});
document.addEventListener('contextmenu', (e) => {
  e.preventDefault(); // no browser menu anywhere
  if (T.on || e.target.closest('input')) return;
  if (e.target.closest('#rows tr.row')) openMenu(e.clientX, e.clientY, true);
  else if (e.target.closest('#list')) openMenu(e.clientX, e.clientY, false);
});
document.addEventListener('mousedown', (e) => { if (S.menuOpen && !e.target.closest('#menu')) closeMenu(); });
document.addEventListener('mouseup', (e) => {
  if (e.button === 3) { e.preventDefault(); nav('back'); } // mouse side buttons
  if (e.button === 4) { e.preventDefault(); nav('fwd'); }
});
$('#menu').addEventListener('click', async (e) => {
  const b = e.target.closest('button');
  if (!b) return;
  closeMenu();
  if (b.dataset.navopen) return go(b.dataset.navopen);
  if (b.dataset.navtidy) return openTidy(b.dataset.navtidy);
  if (b.dataset.rootprops) return showProps(b.dataset.rootprops);
  if (b.dataset.viewopt === 'preview') return setPreview(!S.previewOn);
  if (b.dataset.viewopt === 'hidden') return setHidden(!S.showHidden);
  if (b.dataset.rootdel) {
    const r = S.roots.find((x) => x.path === b.dataset.rootdel);
    return setRoots(S.roots.filter((x) => x !== r).map((x) => x.path), `"${r.name}" dihapus dari daftar — file-nya tidak disentuh`);
  }
  if (b.dataset.rootmove) {
    const [i, d] = b.dataset.rootmove.split(':').map(Number);
    const list = S.roots.map((x) => x.path);
    [list[i], list[i + d]] = [list[i + d], list[i]];
    return setRoots(list);
  }
  if (b.dataset.addpath) return addRoot(b.dataset.addpath, b.dataset.addname, b.dataset.addkind);
  if (b.dataset.addpick !== undefined) {
    const p = await invoke('pick_folder');
    return p && addRoot(p, baseName(p.replace(/\\$/, '')) || p, /^[A-Z]:\\?$/i.test(p) ? 'Drive' : 'Folder');
  }
  if (b.dataset.addreset !== undefined) {
    const places = await invoke('places');
    const def = ['Desktop', 'Downloads', 'Documents', 'Pictures', 'Music', 'Videos'];
    if (!(await confirmDialog('Kembalikan daftar bawaan?', 'Daftar folder kembali ke Desktop, Downloads, Documents, Pictures, Music dan Videos. File tidak disentuh.', 'Kembalikan'))) return;
    return setRoots(places.filter((p) => def.includes(p.kind)).map((p) => p.path), 'Daftar folder dikembalikan');
  }
  if (b.dataset.act === 'refresh') return load();
  if (b.dataset.act === 'propsCwd') return showProps(S.cwd);
  act(b.dataset.act);
});
$$('.cmdbar [data-act]').forEach((b) => b.addEventListener('click', () => act(b.dataset.act)));
$('#modes').addEventListener('click', (e) => {
  const b = e.target.closest('[data-mode]');
  if (!b || S.mode === b.dataset.mode) return;
  S.mode = b.dataset.mode;
  store.set('mode', S.mode);
  load();
});
$('#q').addEventListener('input', (e) => { S.q = e.target.value.trim(); resetSel(); reload(150); });

document.addEventListener('keydown', (e) => {
  const k = e.key.toLowerCase();
  if (!$('#dialog').hidden) return;
  if (S.drag && e.key === 'Escape') { endDrag(); S.dragStart = null; return; }
  if (S.pane === 'settings') return;
  if (S.menuOpen) {
    if (e.key === 'Escape') { closeMenu(); $('#list').focus(); }
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      const items = $$('#menu button:not(:disabled)');
      const j = items.indexOf(document.activeElement);
      items[(j + (e.key === 'ArrowDown' ? 1 : items.length - 1)) % items.length]?.focus();
      e.preventDefault();
    }
    return;
  }
  if (T.on) {
    if (e.key === 'Escape' && !e.target.closest('input')) { T.step === 'plan' ? tidyCmd('back') : tidyCmd('cancel'); }
    if (e.key === ' ' && T.step === 'plan' && !e.target.closest('input') && T.sel.size) {
      const on = !T.items[[...T.sel][0]].checked;
      T.sel.forEach((i) => { const it = T.items[i]; it.checked = on && !!it.target && !it.bad; });
      renderTidy();
      e.preventDefault();
    }
    return;
  }
  if (e.ctrlKey && k === 'f') { $('#q').focus(); $('#q').select(); return e.preventDefault(); }
  if (e.key === 'F5') { load(); return e.preventDefault(); }
  if (e.altKey && e.key === 'ArrowLeft') { nav('back'); return e.preventDefault(); }
  if (e.altKey && e.key === 'ArrowRight') { nav('fwd'); return e.preventDefault(); }
  if (e.altKey && e.key === 'ArrowUp') { nav('up'); return e.preventDefault(); }
  const inField = e.target.matches('input, select');
  if (e.target === $('#q')) {
    if (e.key === 'Escape') { $('#q').value = ''; S.q = ''; load(); }
    if (e.key === 'ArrowDown' || e.key === 'Enter') { $('#list').focus(); if (S.focus < 0) select(0); e.preventDefault(); }
    return;
  }
  if (inField) return;
  if (e.ctrlKey && e.shiftKey && k === 'n') { act('newFolder'); return e.preventDefault(); }
  if (e.ctrlKey && e.shiftKey && k === 'c') { act('copyPath'); return e.preventDefault(); }
  if (e.ctrlKey && k === 'x') return act('cut');
  if (e.ctrlKey && k === 'c') return act('copy');
  if (e.ctrlKey && k === 'v') return act('paste');
  if (e.ctrlKey && k === 'z') return undo();
  if (e.ctrlKey && k === 'a') { S.sel = new Set(S.vis.map((r) => r.path)); paintSel(false); return e.preventDefault(); }
  if (e.altKey && e.key === 'Enter') { act('props'); return e.preventDefault(); }
  if (e.altKey && k === 'p') { setPreview(!S.previewOn); return e.preventDefault(); }
  if (e.ctrlKey && k === 'h') { setHidden(!S.showHidden); return e.preventDefault(); }
  if (e.key === 'F2') { act('rename'); return e.preventDefault(); }
  if (e.key === 'Delete') return act('delete');
  if (e.key === 'Backspace') { nav(isBrowse() ? 'back' : ''); return e.preventDefault(); }
  const n = S.vis.length;
  const page = Math.max(1, Math.floor($('#list').clientHeight / 32) - 2);
  const move = (i) => { select(Math.max(0, Math.min(n - 1, i)), { shift: e.shiftKey }); e.preventDefault(); };
  const cur = S.vis[S.focus];
  switch (e.key) {
    case 'ArrowDown': return move(S.focus + 1);
    case 'ArrowUp': return move(S.focus < 0 ? 0 : S.focus - 1);
    case 'PageDown': return move(S.focus + page);
    case 'PageUp': return move(S.focus - page);
    case 'Home': return move(0);
    case 'End': return move(n - 1);
    case 'ArrowRight':
      if (S.mode === 'tree' && cur?.is_dir) { e.preventDefault(); return cur.expanded ? move(S.focus + 1) : toggle(cur, true); }
      return;
    case 'ArrowLeft':
      if (S.mode === 'tree' && cur) {
        e.preventDefault();
        if (cur.is_dir && cur.expanded) return toggle(cur, false);
        for (let j = S.focus - 1; j >= 0; j--) if (S.vis[j].depth < cur.depth) return select(j); // jump to the parent row
      }
      return;
    case 'Enter': e.preventDefault(); return activate(cur || selected()[0]);
    case 'Escape': resetSel(); return paintSel(false);
    case 'ContextMenu': {
      const tr = $('#rows tr.focus') || $('#rows tr[aria-selected="true"]');
      if (tr) { const r = tr.getBoundingClientRect(); openMenu(r.left + 40, r.bottom, true); }
      return e.preventDefault();
    }
  }
  if (e.shiftKey && e.key === 'F10') { const tr = $('#rows tr.focus'); if (tr) { const r = tr.getBoundingClientRect(); openMenu(r.left + 40, r.bottom, true); } e.preventDefault(); }
});

// ================================================================ start
(async () => {
  S.roots = await invoke('list_roots');
  const pw = store.get('previewW', 0);
  if (pw) $('#preview').style.width = `${pw}px`;
  setPreview(store.get('preview', true));
  await load();
  $('#list').focus();
  listen('changed', () => { if (S.pane === 'explorer') reload(); });
  listen('folder-size', (e) => {
    const { path, size, files } = e.payload;
    S.sizes.set(path, { size, files, t: Date.now() });
    if (!browsing()) return;
    if (S.sorts.browse.sort === 'size') {
      // order depends on folder sizes: re-sort once a burst of results has arrived
      clearTimeout(S.sizeTimer);
      S.sizeTimer = setTimeout(() => { build(); render(); }, 350);
    } else {
      const i = S.vis.findIndex((r) => r.path === path);
      const td = i >= 0 && $(`#rows tr[data-i="${i}"] td.sz`);
      if (td) { td.textContent = human(size); td.title = `${nf.format(files)} file`; }
      paintSel(false); // refresh the total in the status bar
    }
  });
  listen('roots-changed', async () => { S.roots = await invoke('list_roots'); renderNav(); });
  // A drive was plugged in: offer it (unless it is already in the list — that one comes back on its own).
  listen('drive-added', (e) => {
    const d = e.payload;
    const known = S.roots.find((r) => r.path === d.path);
    if (known) return toast(`"${known.name}" terhubung lagi — sedang diperbarui.`);
    toast(`Drive terdeteksi: ${d.name}`, { action: { label: 'Tambahkan ke daftar', fn: () => addRoot(d.path, d.name, 'Drive') }, ms: 15000 });
  });
  // Files dragged in from File Explorer / the desktop.
  const at = (p) => [p.x / devicePixelRatio, p.y / devicePixelRatio];
  listen('tauri://drag-over', (e) => {
    if (S.pane !== 'explorer') return;
    const t = dropTargetAt(...at(e.payload.position), []);
    setDrop(t?.el);
    $('#list').classList.toggle('drop-here', !t && browsing());
  });
  listen('tauri://drag-leave', () => { setDrop(null); $('#list').classList.remove('drop-here'); });
  listen('tauri://drag-drop', (e) => {
    setDrop(null);
    $('#list').classList.remove('drop-here');
    if (S.pane !== 'explorer') return toast('Buka tampilan folder dulu, lalu jatuhkan file di sana.', { error: true });
    const t = dropTargetAt(...at(e.payload.position), []);
    const dest = t?.path || (browsing() ? S.cwd : null);
    if (!dest) return toast('Buka folder tujuan dulu (panel kiri), lalu jatuhkan file di sana.', { error: true });
    const paths = e.payload.paths.filter((p) => parentOf(p) !== dest);
    if (paths.length) dropPaths(paths, dest, false);
  });
  listen('ai-group', (e) => { if (T.aiRun) { T.aiRun = e.payload; renderTidy(); } });
  refreshAi();
  // the first scan can end without a row change: keep the "indexing" state fresh until it is done
  const poll = setInterval(() => { if (!S.counts.scanning) clearInterval(poll); else if (!T.on) reload(0); }, 3000);
})();
