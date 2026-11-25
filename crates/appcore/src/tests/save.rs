//! A failing save or a broken file reaches the player as a notice.

use super::*;

/// A non-empty folder where the file should be: writing the file fails on every system, even
/// for a user who may write anywhere.
fn block(dir: &Path, name: &str) {
    let _ = std::fs::remove_file(dir.join(name));
    std::fs::create_dir_all(dir.join(name).join("inside")).unwrap();
}

#[test]
fn a_failed_library_save_is_told() {
    let dir = temp_dir("save-library-failed");
    let mut core = Core::new(&dir, QaOptions::default());
    block(&dir, "library.json");

    core.save_library();

    let notice = core.take_notice().expect("a notice");
    assert!(notice.contains("library.json"), "{notice}");
    assert!(!notice.contains("save.failed"), "untranslated: {notice}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_failed_settings_save_is_told() {
    let dir = temp_dir("save-settings-failed");
    let mut core = Core::new(&dir, QaOptions::default());
    block(&dir, "settings.json");

    core.update_settings(|s| s.view = "tab".to_string());

    let notice = core.take_notice().expect("a notice");
    assert!(notice.contains("settings.json"), "{notice}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_working_save_says_nothing() {
    let dir = temp_dir("save-works");
    let mut core = Core::new(&dir, QaOptions::default());

    core.save_library();
    core.update_settings(|s| s.view = "tab".to_string());

    assert_eq!(core.take_notice(), None);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_broken_library_at_start_is_told() {
    let dir = temp_dir("save-library-broken");
    std::fs::write(dir.join("library.json"), "{ not json").unwrap();

    let mut core = Core::new(&dir, QaOptions::default());

    let notice = core.take_notice().expect("a notice");
    assert!(notice.contains("library.json"), "{notice}");
    assert!(!notice.contains("save.broken"), "untranslated: {notice}");
    assert!(dir.join("library.json.broken").exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_notice_reaches_the_tick() {
    let dir = temp_dir("save-tick");
    let mut core = Core::new(&dir, QaOptions::default());
    block(&dir, "library.json");
    core.save_library();

    let events = core.tick();

    assert!(events.iter().any(|e| matches!(e, UiEvent::Message(_))));
    assert_eq!(core.take_notice(), None);
    let _ = std::fs::remove_dir_all(dir);
}

/// Two failures before the next tick: the player reads both in the one message, oldest first
/// (the note bar has one place; one notice per tick overwrote the first 16 ms later).
#[test]
fn notices_queue_up() {
    let dir = temp_dir("save-queue");
    let mut core = Core::new(&dir, QaOptions::default());
    block(&dir, "library.json");
    block(&dir, "settings.json");

    core.save_library();
    core.update_settings(|s| s.view = "tab".to_string());

    let events = core.tick();
    let messages: Vec<&String> = events
        .iter()
        .filter_map(|e| match e {
            UiEvent::Message(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(messages.len(), 1, "{messages:?}");
    let library = messages[0].find("library.json").expect("library notice");
    let settings = messages[0].find("settings.json").expect("settings notice");
    assert!(library < settings, "{}", messages[0]);
    assert_eq!(core.take_notice(), None);
    let _ = std::fs::remove_dir_all(dir);
}
