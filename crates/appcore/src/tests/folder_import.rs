//! "Ordner importieren": which files a folder import takes.

use super::*;

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
