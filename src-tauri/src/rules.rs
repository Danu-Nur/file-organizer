// SPDX-License-Identifier: GPL-3.0-or-later
//! Pure rules: which category (folder name, Indonesian) a file belongs to, and what the index ignores.
use std::path::Path;

const CATEGORIES: &[(&str, &[&str])] = &[
    ("Gambar", &["png", "jpg", "jpeg", "webp", "svg", "ico", "gif", "bmp", "heic", "avif", "tif", "tiff"]),
    ("Dokumen", &["pdf", "docx", "doc", "pptx", "ppt", "pub", "txt", "md", "bib", "rtf", "odt", "epub"]),
    ("Spreadsheet", &["xlsx", "xls", "csv", "ods"]),
    ("Arsip", &["zip", "rar", "tgz", "gz", "7z", "tar", "xz"]),
    ("Program", &["msi", "exe", "vsix", "apk", "msix"]),
    ("Kode", &["html", "htm", "php", "js", "ts", "tsx", "jsx", "css", "sql", "json", "xml", "map", "lock", "py", "mq5", "skill", "yml", "yaml", "ps1", "bat", "cmd", "sh", "reg"]),
    ("Model 3D", &["stl", "3mf", "obj", "step", "stp"]),
    ("Musik", &["mp3", "wav", "m4a", "flac", "ogg", "aac"]),
    ("Video", &["mp4", "mkv", "mov", "avi", "webm"]),
    ("Kontak", &["vcf"]),
];
pub const OTHER: &str = "Lainnya";
/// Folders this app creates for special cases.
pub const SPECIAL: &[&str] = &["_Sensitif", "_Duplikat", "_Karantina", "_Kosong", "_Versi Lama", "Proyek"];
/// Common English category folder names → their Indonesian name (offered as a rename in "Rapikan folder").
pub const LEGACY: &[(&str, &str)] = &[
    ("Documents", "Dokumen"), ("Images", "Gambar"), ("Sheets", "Spreadsheet"), ("Compressed", "Arsip"),
    ("Programs", "Program"), ("Code", "Kode"), ("3D", "Model 3D"), ("Music", "Musik"), ("Contacts", "Kontak"),
    ("Misc", "Lainnya"), ("_Sensitive", "_Sensitif"), ("Projects", "Proyek"),
];

/// Category folder name for a file.
pub fn category(name: &str) -> &'static str {
    let n = name.to_lowercase();
    if is_secret(&n) {
        return "_Sensitif";
    }
    let ext = Path::new(&n).extension().and_then(|e| e.to_str()).unwrap_or("");
    CATEGORIES.iter().find(|(_, exts)| exts.contains(&ext)).map(|c| c.0).unwrap_or(OTHER)
}

/// True for a folder name this app uses as a category (so it is a destination, not something to sort).
pub fn is_category_folder(name: &str) -> bool {
    let n = name.to_lowercase();
    CATEGORIES.iter().any(|c| c.0.to_lowercase() == n) || n == OTHER.to_lowercase() || SPECIAL.iter().any(|s| s.to_lowercase() == n)
}

/// The category a folder already stands for ("Dokumen", or the old "Documents") — its matching files are home.
pub fn folder_category(name: &str) -> Option<&'static str> {
    CATEGORIES.iter().map(|c| c.0).chain([OTHER]).chain(SPECIAL.iter().copied())
        .find(|c| c.eq_ignore_ascii_case(name))
        .or_else(|| legacy_to_id(name))
}

/// Indonesian name for an English category folder name.
pub fn legacy_to_id(name: &str) -> Option<&'static str> {
    LEGACY.iter().find(|(en, _)| en.eq_ignore_ascii_case(name)).map(|l| l.1)
}

/// English folder that may still hold an Indonesian category (before the user renames it).
pub fn id_to_legacy(name: &str) -> Option<&'static str> {
    LEGACY.iter().find(|(_, id)| id.eq_ignore_ascii_case(name)).map(|l| l.0)
}

/// Suggested subfolder for a loose file in a root whose own kind is `home` (Pictures = Gambar, …); None = already home.
pub fn suggest(name: &str, home: Option<&str>) -> Option<&'static str> {
    let c = category(name);
    (Some(c) != home).then_some(c)
}

fn is_secret(n: &str) -> bool {
    n == "env" || n == ".env" || n.starts_with("env.") || n.starts_with(".env.")
        || ["recovery", "client_secret", "adminsdk", "id_rsa", "pub_codes", "private_key", ".pem", ".pfx", "password", "credential", "kata sandi"]
            .iter().any(|k| n.contains(k))
}

/// In-progress downloads, shortcuts and system files: never indexed or moved.
pub fn ignore_file(name: &str) -> bool {
    let n = name.to_lowercase();
    n == "desktop.ini" || n == "thumbs.db" || n.starts_with("~$")
        || [".crdownload", ".part", ".partial", ".tmp", ".download", ".lnk", ".url"].iter().any(|e| n.ends_with(e))
}

/// A code project: has its own structure, so it is kept whole.
pub fn is_project(dir: &Path) -> bool {
    [".git", "package.json", "Cargo.toml", "composer.json", "pyproject.toml", "platformio.ini", "CMakeLists.txt", "go.mod", "pom.xml", "build.gradle", "build.zig"]
        .iter().any(|m| dir.join(m).exists())
}

/// Windows and app folders that show up when a whole drive or the user folder is watched; never organised.
const SYSTEM_DIRS: &[&str] = &[
    "windows", "program files", "program files (x86)", "programdata", "system volume information", "appdata",
    "$windows.~bt", "$windows.~ws", "$winreagent", "recovery", "perflogs", "msocache", "intel", "amd", "nvidia",
];

/// Hidden folders, dependency/build folders, project containers, code projects and system folders are not indexed.
// ponytail: checked once per folder and cached by the caller; a project created later in an already-indexed folder is
// noticed on the next app start.
pub fn skip_dir(dir: &Path) -> bool {
    let n = dir.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    n.starts_with('.') || ["node_modules", "target", "vendor", "__pycache__", "venv", "$recycle.bin", "projects", "proyek"].contains(&n.as_str())
        || SYSTEM_DIRS.contains(&n.as_str())
        || is_project(dir)
}

/// "laporan (2).pdf" / "laporan - Copy.pdf" → "laporan.pdf" (lowercase): files that are versions of each other.
pub fn base_name(name: &str) -> String {
    let p = Path::new(name);
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let mut stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    loop {
        let t = stem.trim_end();
        let next = if let Some(s) = t.strip_suffix(" - Copy").or_else(|| t.strip_suffix(" - Salinan")) {
            s.to_string()
        } else if t.ends_with(')') && t.rfind(" (").is_some_and(|i| t[i + 2..t.len() - 1].chars().all(|c| c.is_ascii_digit()) && t.len() > i + 3) {
            t[..t.rfind(" (").unwrap()].to_string()
        } else {
            break;
        };
        stem = next;
    }
    format!("{stem}{ext}").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules() {
        assert_eq!(category("Invoice.PDF"), "Dokumen");
        assert_eq!(category("github-recovery-codes.txt"), "_Sensitif");
        assert_eq!(category("env.testing"), "_Sensitif");
        assert_eq!(category("username dan password untuk aplikasi.txt"), "_Sensitif");
        assert_eq!(category("photo.jpeg"), "Gambar");
        assert_eq!(category("no-extension"), "Lainnya");
        assert_eq!(suggest("a.png", Some("Gambar")), None);
        assert_eq!(suggest("a.png", None), Some("Gambar"));
        assert!(ignore_file("setup.exe.crdownload") && ignore_file("desktop.ini") && !ignore_file("a.pdf"));
        assert!(is_category_folder("dokumen") && is_category_folder("_Duplikat") && !is_category_folder("Dokumen Nola"));
        assert_eq!(legacy_to_id("Compressed"), Some("Arsip"));
        assert_eq!(base_name("laporan (2).pdf"), "laporan.pdf");
        assert_eq!(base_name("Laporan - Copy (1).PDF"), "laporan.pdf");
        assert_eq!(base_name("files (4)"), "files");
        assert_eq!(base_name("Q (A).pdf"), "q (a).pdf");
    }
}
