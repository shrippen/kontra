//! Glyphs (Harmonicon W11): every special character in Kontra's own texts is drawn by a font
//! Kontra ships, so no system without it shows an empty box (tofu).
//!
//! ```text
//! i18n de/en ─┐
//! lessons ────┼─► characters above ASCII ─► in the cmap of a shipped font?
//! own QML ────┘   (private use area: only Bravura, the notation font)
//! ```
//!
//! The fonts are read with a small `cmap` reader (formats 4 and 12) instead of a font crate.

// Test helpers: a missing font or text file fails the test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The fonts Kontra ships (Kante); Qt falls back to them for a glyph the system font lacks.
const FONTS: &str = "crates/app/qml/Kante/fonts";
/// The notation font: the only one whose private-use glyphs mean something (SMuFL).
const NOTATION_FONT: &str = "Bravura.otf";
/// Kontra's own QML; `Kante/` below it is vendored and checked in Kante.
const QML: &str = "crates/app/qml";
const VENDORED: &str = "Kante";
const I18N: [&str; 2] = ["crates/appcore/i18n/de.json", "crates/appcore/i18n/en.json"];
const LESSONS: &str = "lessons";

/// Unicode private use area (SMuFL puts notation glyphs there).
const PRIVATE_USE: std::ops::RangeInclusive<u32> = 0xE000..=0xF8FF;

/// `cmap` subtable formats: segments over the BMP (4) and groups over all planes (12).
const SEGMENTS: u16 = 4;
const GROUPS: u16 = 12;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

/// The `cmap` table of an OpenType/TrueType file, from its table directory.
fn cmap_table(font: &[u8]) -> Option<&[u8]> {
    let tables = usize::from(u16_at(font, 4)?);
    (0..tables).find_map(|i| {
        let record = 12 + i * 16;
        if font.get(record..record + 4)? != b"cmap" {
            return None;
        }
        let offset = u32_at(font, record + 8)? as usize;
        let length = u32_at(font, record + 12)? as usize;
        font.get(offset..offset + length)
    })
}

/// Characters a format 4 subtable maps to a glyph other than 0 (`.notdef`).
fn segments(table: &[u8], chars: &mut BTreeSet<u32>) -> Option<()> {
    let count = usize::from(u16_at(table, 6)? / 2);
    let ends = 14;
    let starts = ends + 2 * count + 2;
    let deltas = starts + 2 * count;
    let ranges = deltas + 2 * count;
    for i in 0..count {
        let (start, end) = (u16_at(table, starts + 2 * i)?, u16_at(table, ends + 2 * i)?);
        let delta = u16_at(table, deltas + 2 * i)?;
        let range = usize::from(u16_at(table, ranges + 2 * i)?);
        for c in start..=end {
            if c == 0xFFFF {
                continue;
            }

            // Without a range offset the glyph is char + delta; with one, an array entry + delta.
            let glyph = if range == 0 {
                c.wrapping_add(delta)
            } else {
                let at = ranges + 2 * i + range + 2 * usize::from(c - start);
                match u16_at(table, at)? {
                    0 => 0,
                    g => g.wrapping_add(delta),
                }
            };
            if glyph != 0 {
                chars.insert(u32::from(c));
            }
        }
    }
    Some(())
}

/// Characters a format 12 subtable maps (groups of consecutive characters).
fn groups(table: &[u8], chars: &mut BTreeSet<u32>) -> Option<()> {
    let count = u32_at(table, 12)? as usize;
    for i in 0..count {
        let group = 16 + i * 12;
        let (start, end) = (u32_at(table, group)?, u32_at(table, group + 4)?);
        let glyph = u32_at(table, group + 8)?;
        let first = if glyph == 0 { start + 1 } else { start };
        chars.extend(first..=end);
    }
    Some(())
}

/// Every character the font maps, from all Unicode subtables it has.
fn coverage(font: &[u8]) -> BTreeSet<u32> {
    let mut chars = BTreeSet::new();
    let Some(cmap) = cmap_table(font) else {
        return chars;
    };
    let count = usize::from(u16_at(cmap, 2).unwrap_or(0));
    for i in 0..count {
        let Some(offset) = u32_at(cmap, 4 + i * 8 + 4) else {
            continue;
        };
        let Some(table) = cmap.get(offset as usize..) else {
            continue;
        };
        match u16_at(table, 0) {
            Some(SEGMENTS) => segments(table, &mut chars),
            Some(GROUPS) => groups(table, &mut chars),
            _ => None,
        };
    }
    chars
}

/// Shipped fonts by file name, with the characters each covers.
fn fonts() -> BTreeMap<String, BTreeSet<u32>> {
    let dir = std::fs::read_dir(root().join(FONTS)).expect("Kante fonts");
    dir.flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ttf" || x == "otf"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            (name, coverage(&std::fs::read(&p).expect("font")))
        })
        .collect()
}

/// Every string value of a JSON document.
fn strings(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(items) => items.iter().for_each(|v| strings(v, out)),
        serde_json::Value::Object(map) => map.values().for_each(|v| strings(v, out)),
        _ => {}
    }
}

/// String literals of a QML/JS line, with `\uXXXX` escapes resolved ("♯" → "♯").
fn literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '/' && chars.clone().next() == Some('/') {
            break;
        }
        if c != '"' && c != '\'' {
            continue;
        }

        let mut text = String::new();
        while let Some(d) = chars.next() {
            if d == c {
                break;
            }
            if d != '\\' {
                text.push(d);
                continue;
            }
            match chars.next() {
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    text.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
                }
                Some(e) => text.push(e),
                None => {}
            }
        }
        out.push(text);
    }
    out
}

fn files(dir: &Path, extensions: &[&str], skip: Option<&str>, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            if skip.is_some_and(|s| path.ends_with(s)) {
                continue;
            }
            files(&path, extensions, skip, out);
            continue;
        }
        if path
            .extension()
            .is_some_and(|x| extensions.iter().any(|e| x == *e))
        {
            out.push(path);
        }
    }
}

/// Kontra's own texts above ASCII, each with the first file it appears in.
fn special_chars() -> BTreeMap<char, String> {
    let root = root();
    let mut texts: Vec<(String, String)> = Vec::new();
    for path in I18N {
        let json = std::fs::read_to_string(root.join(path)).expect("i18n");
        let mut values = Vec::new();
        strings(
            &serde_json::from_str(&json).expect("i18n json"),
            &mut values,
        );
        texts.extend(values.into_iter().map(|v| (path.to_string(), v)));
    }

    let mut lessons = Vec::new();
    files(&root.join(LESSONS), &["md", "json"], None, &mut lessons);
    let mut qml = Vec::new();
    files(&root.join(QML), &["qml", "js"], Some(VENDORED), &mut qml);
    for path in lessons {
        let text = std::fs::read_to_string(&path).expect("lesson");
        texts.push((path.display().to_string(), text));
    }
    for path in qml {
        let text = std::fs::read_to_string(&path).expect("qml");
        let name = path.display().to_string();
        texts.extend(text.lines().flat_map(literals).map(|l| (name.clone(), l)));
    }

    let mut chars = BTreeMap::new();
    for (file, text) in texts {
        for c in text.chars().filter(|c| !c.is_ascii() && !c.is_whitespace()) {
            chars.entry(c).or_insert_with(|| file.clone());
        }
    }
    chars
}

#[test]
fn shipped_fonts_cover_special_chars() {
    let fonts = fonts();
    assert!(fonts.contains_key(NOTATION_FONT), "{fonts:?}");
    let mut missing = Vec::new();
    for (c, file) in special_chars() {
        let code = u32::from(c);
        let covered = if PRIVATE_USE.contains(&code) {
            fonts[NOTATION_FONT].contains(&code)
        } else {
            fonts.values().any(|f| f.contains(&code))
        };
        if !covered {
            missing.push(format!("{c} U+{code:04X} ({file})"));
        }
    }
    assert!(
        missing.is_empty(),
        "no shipped font has: {}\nuse a covered character or add a font to Kante",
        missing.join(", ")
    );
}

/// The reader itself: known characters of the shipped fonts (format 4 and format 12).
#[test]
fn cmap_reader_finds_known_glyphs() {
    let fonts = fonts();
    let mono = &fonts["JetBrainsMono-400.ttf"];
    assert!(mono.contains(&u32::from('A')) && mono.contains(&u32::from('…')));
    assert!(!mono.contains(&u32::from('♯')));
    let bravura = &fonts[NOTATION_FONT];
    // SMuFL F clef (U+E062) and the sharp in the BMP (U+266F); 𝄞 beyond it (format 12 only).
    assert!(bravura.contains(&0xE062) && bravura.contains(&0x266F));
    assert!(bravura.contains(&u32::from('𝄞')));
    assert!(!fonts["Rajdhani-700.ttf"].contains(&u32::from('𝄞')));
}

#[test]
fn literals_resolve_escapes() {
    assert_eq!(literals(r#"text: "C♯" + 'x' // "ignored ✓""#), ["C♯", "x"]);
}
