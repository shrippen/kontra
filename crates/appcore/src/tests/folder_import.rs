//! "Ordner importieren": which files a folder import takes.

use super::*;
use crate::folders::walk_folder;

/// Audit 14.11., Fund 3: the folder import asks `FileKind` which endings count, so a
/// Karaoke-MIDI file (`.kar`) is taken like the other formats.
#[test]
fn a_folder_import_takes_a_kar_file() {
    let dir = temp_dir("kar-folder");
    let folder = dir.join("tabs");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::copy(demo(), folder.join("lied.kar")).unwrap();
    let mut core = Core::new(&dir, QaOptions::default());

    let (added, failed) = core.import_folder(&folder, false);
    assert_eq!(added, 1, "{failed:?}");
    let _ = std::fs::remove_dir_all(dir);
}

/// The open dialog filter lists every extension the importer reads, plus packages.
#[test]
fn open_filter_lists_what_the_importer_reads() {
    let dir = temp_dir("filter");
    let core = Core::new(&dir, QaOptions::default());
    let filter = core.open_filter();
    let patterns: Vec<&str> = filter.split(' ').collect();
    for (extension, _) in kontra_session::FileKind::EXTENSIONS {
        assert!(
            patterns.contains(&format!("*.{extension}").as_str()),
            "{extension}"
        );
    }
    assert!(
        patterns.contains(&"*.kar") && patterns.contains(&"*.kontra"),
        "{filter}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// Owner decision, round 23: the folder import takes a plain `.xml` only if it is MusicXML.
#[test]
fn a_folder_import_skips_a_feed_xml() {
    let dir = temp_dir("xml-folder");
    let folder = dir.join("downloads");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::copy(demo(), folder.join("lied.mid")).unwrap();
    std::fs::write(folder.join("feed.xml"), "<?xml version=\"1.0\"?><rss/>").unwrap();
    std::fs::write(
        folder.join("song.xml"),
        "<?xml version=\"1.0\"?><score-partwise version=\"4.0\"><part-list/></score-partwise>",
    )
    .unwrap();
    let mut core = Core::new(&dir, QaOptions::default());

    let (added, failed) = core.import_folder(&folder, false);

    let names: Vec<&str> = failed.iter().map(|(name, _)| name.as_str()).collect();
    assert!(!names.contains(&"feed.xml"), "{failed:?}");
    assert_eq!(added + failed.len(), 2, "{added} {failed:?}");
    let _ = std::fs::remove_dir_all(dir);
}

/// Owner decision, round 23: a plain `.xml` counts if it is MusicXML, not if it is a feed.
#[test]
fn a_linked_folder_takes_a_musicxml_xml_and_not_a_feed() {
    let dir = temp_dir("walk-xml");
    std::fs::write(
        dir.join("song.xml"),
        "<?xml version=\"1.0\"?><score-partwise version=\"4.0\"><part-list/></score-partwise>",
    )
    .unwrap();
    std::fs::write(dir.join("feed.xml"), "<?xml version=\"1.0\"?><rss/>").unwrap();

    let walked = walk_folder(&dir);

    assert_eq!(walked.files, [dir.join("song.xml")]);
    let _ = std::fs::remove_dir_all(dir);
}

/// An unreadable `.xml` is not "no score": the import names it and the reason, like any file.
#[cfg(unix)]
#[test]
fn a_folder_import_names_an_unreadable_xml() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("xml-unreadable");
    let folder = dir.join("downloads");
    std::fs::create_dir_all(&folder).unwrap();
    let locked = folder.join("locked.xml");
    std::fs::write(&locked, "<score-partwise/>").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let mut core = Core::new(&dir, QaOptions::default());

    let (_, failed) = core.import_folder(&folder, false);

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
    let names: Vec<&str> = failed.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["locked.xml"], "{failed:?}");
    let _ = std::fs::remove_dir_all(dir);
}
