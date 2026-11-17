// SPDX-License-Identifier: GPL-3.0-or-later
//! Preview pane: raw bytes for images/PDF/media (size-capped) and text excerpts for text files and DOCX.
//! Files may come from the internet, so every read is bounded.
use std::io::Read;
use std::path::Path;

/// Text of a DOCX body, paragraphs on their own lines; the decompressed XML read is capped at `max_xml` bytes.
pub fn docx_text(p: &Path, max_xml: u64) -> Option<String> {
    let mut z = zip::ZipArchive::new(std::fs::File::open(p).ok()?).ok()?;
    let entry = z.by_name("word/document.xml").ok()?;
    let mut raw = Vec::new();
    entry.take(max_xml).read_to_end(&mut raw).ok()?;
    let xml = String::from_utf8_lossy(&raw).replace("</w:p>", "\n");
    let mut out = String::new();
    let mut in_tag = false;
    for c in xml.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    Some(out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'"))
}

/// The first `max` bytes of a file as text (invalid UTF-8 replaced).
pub fn head_text(p: &Path, max: u64) -> Option<String> {
    let mut b = Vec::new();
    std::fs::File::open(p).ok()?.take(max).read_to_end(&mut b).ok()?;
    Some(String::from_utf8_lossy(&b).into_owned())
}

/// Text for the preview pane (~64 KB).
pub fn text(p: &Path) -> Result<String, String> {
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let t = if ext == "docx" { docx_text(p, 2 << 20) } else { head_text(p, 64 << 10) };
    t.ok_or_else(|| "Isi file tidak bisa dibaca.".to_string())
}

/// Whole file for an image/PDF/media preview, refused above `max` bytes.
pub fn bytes(p: &Path, max: u64) -> Result<Vec<u8>, String> {
    let len = std::fs::metadata(p).map_err(|e| e.to_string())?.len();
    if len > max {
        return Err(format!("File terlalu besar untuk pratinjau ({} MB). Buka dengan aplikasinya.", len >> 20));
    }
    std::fs::read(p).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_reads() {
        let dir = std::env::temp_dir().join(format!("fo-preview-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.txt");
        std::fs::write(&f, "x".repeat(100_000)).unwrap();
        assert_eq!(text(&f).unwrap().len(), 64 << 10);
        assert!(bytes(&f, 10).is_err());
        assert_eq!(bytes(&f, 1 << 20).unwrap().len(), 100_000);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
