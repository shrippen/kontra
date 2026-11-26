//! Settings, view per piece, setlists and progress survive a restart (LH-ALL-03, LH-FOR-03).

use crate::common::{write_midi, Profile};
use kontra_appcore::{View, Window};

#[test]
fn settings_and_library_survive_restart() {
    let profile = Profile::new("restart");
    let score = profile.dir.join("stueck.mid");
    write_midi(&score, "Stueck", &[28, 33, 38, 43]);
    {
        let mut core = profile.core(None);
        core.open(&score).unwrap();
        core.set_view(View::Notation);
        core.reconfigure(|s| {
            s.strings = 5;
            s.latency_ms = 23.0;
            s.strictness = Window::Strict;
        });
        core.remix(|s| s.backing_volume = 0.4);
        core.setlist_create("Freitag");
        core.setlist_add_loaded("Freitag");
    }
    let mut core = profile.core(None);
    let s = core.settings();
    assert_eq!(
        (s.strings, s.latency_ms, s.strictness),
        (5, 23.0, Window::Strict)
    );
    assert_eq!(s.backing_volume, 0.4);
    assert_eq!(s.language.as_deref(), Some("de"));
    assert!(core.recent_json().contains("stueck"));
    assert!(core.setlists_json().contains("Freitag"));
    core.open(&score).unwrap();
    assert_eq!(core.view(), View::Notation);
}
