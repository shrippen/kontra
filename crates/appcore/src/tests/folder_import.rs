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
