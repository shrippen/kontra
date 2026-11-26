//! `FileKind::of_file`: a plain `.xml` counts as a score only if it is MusicXML.

// Test helpers outside #[test] functions may panic too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kontra_content::FileKind;
use std::path::PathBuf;

const DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
const DOCTYPE: &str = "<!DOCTYPE score-partwise PUBLIC \"-//Recordare//DTD MusicXML 4.0 \
Partwise//EN\" \"http://www.musicxml.org/dtds/partwise.dtd\">\n";

/// Writes `content` as `name` in a fresh folder and returns the path.
fn file(test: &str, name: &str, content: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kontra-kind-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

fn kind_of(test: &str, name: &str, content: &str) -> Option<FileKind> {
    let path = file(test, name, content);
    let kind = FileKind::of_file(&path);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    kind
}

#[test]
fn musicxml_with_declaration_and_doctype() {
    let content =
        format!("{DECLARATION}{DOCTYPE}<score-partwise version=\"4.0\"></score-partwise>");
    assert_eq!(
        kind_of("full", "song.xml", &content),
        Some(FileKind::MusicXml)
    );
}

#[test]
fn musicxml_without_declaration() {
    let content = "<score-partwise><part-list/></score-partwise>";
    assert_eq!(
        kind_of("bare", "song.XML", content),
        Some(FileKind::MusicXml)
    );
}

#[test]
fn musicxml_after_comments_and_bom() {
    let content = format!("\u{feff}{DECLARATION}<!-- made by hand -->\n<score-partwise>");
    assert_eq!(
        kind_of("comment", "song.xml", &content),
        Some(FileKind::MusicXml)
    );
}

#[test]
fn timewise_counts() {
    let content = format!("{DECLARATION}<score-timewise version=\"4.0\">");
    assert_eq!(
        kind_of("time", "song.xml", &content),
        Some(FileKind::MusicXml)
    );
}

#[test]
fn a_feed_is_no_score() {
    let content = format!("{DECLARATION}<rss version=\"2.0\"><channel/></rss>");
    assert_eq!(kind_of("rss", "feed.xml", &content), None);
}

#[test]
fn a_mention_is_no_root() {
    let content = format!("{DECLARATION}<!-- <score-partwise> --><note><score-partwise/></note>");
    assert_eq!(kind_of("mention", "notes.xml", &content), None);
}

#[test]
fn a_too_short_file_is_no_score() {
    assert_eq!(kind_of("short", "a.xml", "<score-"), None);
    assert_eq!(kind_of("empty", "b.xml", ""), None);
    assert_eq!(kind_of("cut", "c.xml", "<?xml version="), None);
}

#[test]
fn other_extensions_need_no_look_inside() {
    assert_eq!(kind_of("gp", "song.gp5", "x"), Some(FileKind::GuitarPro));
    assert_eq!(
        kind_of("mxl", "song.musicxml", "x"),
        Some(FileKind::MusicXml)
    );
    assert_eq!(kind_of("txt", "notes.txt", "x"), None);
}

#[test]
fn a_missing_file_is_none() {
    assert_eq!(
        FileKind::of_file(&PathBuf::from("/nonexistent/a.xml")),
        None
    );
}
