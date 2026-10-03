//! Finding fonts on the device for characters the engine's fonts lack.
//!
//! Every platform ships a face for each script it can display: Noto on
//! Android and Linux, Kohinoor, Geeza, PingFang and the like on Apple
//! systems, Nirmala, Arial and Microsoft YaHei on Windows. This module lists
//! the font files in the system's font folders once, picks candidates for a
//! character by its script, and maps the chosen file into memory so a 50 MB
//! CJK collection costs address space rather than heap.
//!
//! iOS and Android SDKs use the platform's own font APIs instead (CoreText,
//! `SystemFonts`); this serves Dart, Flutter and C hosts.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Font files under the system's font folders.
fn files() -> &'static [PathBuf] {
    static FILES: OnceLock<Vec<PathBuf>> = OnceLock::new();
    FILES.get_or_init(|| {
        let mut out = Vec::new();
        for dir in dirs() {
            walk(&dir, 0, &mut out);
        }
        out.sort();
        out.dedup();
        out
    })
}

fn dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut d: Vec<PathBuf> = vec![];
    if cfg!(target_os = "android") {
        d.extend(["/system/fonts", "/product/fonts", "/system/product/fonts"].map(PathBuf::from));
    } else if cfg!(any(target_os = "macos", target_os = "ios")) {
        d.extend(["/System/Library/Fonts", "/Library/Fonts"].map(PathBuf::from));
        if let Some(h) = &home {
            d.push(h.join("Library/Fonts"));
        }
    } else if cfg!(windows) {
        let windir = std::env::var_os("WINDIR").map_or_else(|| PathBuf::from("C:\\Windows"), PathBuf::from);
        d.push(windir.join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            d.push(PathBuf::from(local).join("Microsoft\\Windows\\Fonts"));
        }
    } else {
        d.extend(["/usr/share/fonts", "/usr/local/share/fonts"].map(PathBuf::from));
        if let Some(h) = &home {
            d.push(h.join(".local/share/fonts"));
            d.push(h.join(".fonts"));
        }
    }
    d
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth < 4 {
                walk(&p, depth + 1, out);
            }
        } else if p
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| matches!(x.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc"))
        {
            out.push(p);
        }
    }
}

/// File-name fragments of fonts for a character's script, best first,
/// across Android, Apple, Windows and Linux naming.
fn keywords(cp: u32) -> &'static [&'static str] {
    match cp {
        0x00C0..=0x052F | 0x1E00..=0x1FFF => &[
            "NotoSans-Regular",
            "Roboto-Regular",
            "Times New Roman",
            "times.ttf",
            "DejaVuSans.ttf",
            "arial.ttf",
        ],
        0x0530..=0x058F => &["Armenian"],
        0x0590..=0x05FF => &["Hebrew", "ArialHB", "David", "arial.ttf"],
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => {
            &["NaskhArabic", "GeezaPro", "SFArabic", "NotoSansArabic", "arial.ttf", "Arabic"]
        }
        0x0900..=0x097F => &["Devanagari", "Kohinoor.ttc", "Nirmala"],
        0x0980..=0x09FF => &["Bengali", "Bangla", "Nirmala"],
        0x0A00..=0x0A7F => &["Gurmukhi", "Nirmala"],
        0x0A80..=0x0AFF => &["Gujarati", "Nirmala"],
        0x0B00..=0x0B7F => &["Oriya", "Odia", "Nirmala"],
        0x0B80..=0x0BFF => &["Tamil", "Nirmala"],
        0x0C00..=0x0C7F => &["Telugu", "Nirmala"],
        0x0C80..=0x0CFF => &["Kannada", "Nirmala"],
        0x0D00..=0x0D7F => &["Malayalam", "Nirmala"],
        0x0D80..=0x0DFF => &["Sinhala", "Nirmala"],
        0x0E00..=0x0E7F => &["Thai", "Thonburi", "Leelawad", "Tahoma"],
        0x0E80..=0x0EFF => &["Lao"],
        0x0F00..=0x0FFF => &["Tibetan", "Kailasa", "himalaya"],
        0x1000..=0x109F => &["Myanmar"],
        0x10A0..=0x10FF => &["Georgian"],
        0x1200..=0x139F => &["Ethiopic", "Kefa", "ebrima"],
        0x1780..=0x17FF => &["Khmer"],
        0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF => &["AppleSDGothicNeo", "NotoSansCJK", "NotoSansKR", "malgun", "CJK"],
        0x3040..=0x30FF => &["Hiragino", "NotoSansCJK", "NotoSansJP", "YuGoth", "msgothic", "CJK"],
        0x2E80..=0x9FFF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF | 0x20000..=0x2FFFF => &[
            "PingFang",
            "Hiragino Sans GB",
            "NotoSansCJK",
            "NotoSansSC",
            "msyh",
            "STHeiti",
            "CJK",
        ],
        _ => &["NotoSansSymbols", "NotoSans-Regular", "Symbola", "seguisym", "Apple Symbols"],
    }
}

/// Candidate files for a code point, in order of preference.
pub fn candidates(cp: u32) -> Vec<&'static Path> {
    let mut out: Vec<&'static Path> = Vec::new();
    for k in keywords(cp) {
        let k = k.to_ascii_lowercase();
        let mut hits: Vec<&'static Path> = files()
            .iter()
            .map(PathBuf::as_path)
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.to_ascii_lowercase().contains(&k))
            })
            .filter(|p| !out.contains(p))
            .collect();
        // Regular weight and the text cut before bold, italic and UI cuts.
        let rank = |p: &Path| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
            (["bold", "italic", "black", "light", "thin", "medium", "condensed"]
                .iter()
                .any(|w| n.contains(w)) as u8)
                * 2
                + (n.contains("ui") as u8)
                + (n.contains("serif") as u8)
        };
        hits.sort_by_key(|p| rank(p));
        out.extend(hits);
    }
    out
}

/// A mapped font file and the face in it that has `cp`, if any.
pub fn open(path: &Path, cp: u32) -> Option<(memmap2::Mmap, u32)> {
    let file = std::fs::File::open(path).ok()?;
    // SAFETY: system font files are not modified while mapped.
    let map = unsafe { memmap2::Mmap::map(&file) }.ok()?;
    let ch = char::from_u32(cp)?;
    let count = ttf_parser::fonts_in_collection(&map).unwrap_or(1);
    let index = (0..count).find(|&i| ttf_parser::Face::parse(&map, i).is_ok_and(|f| f.glyph_index(ch).is_some()))?;
    Some((map, index))
}
