//! J-B6 Song aus dem Netz (P2 Jens, P3 Mira, P1 Lea): search page for the browser, the
//! downloaded file under "New in Downloads", a MuseScore file dropped onto the library.

use crate::common::{write_midi, Profile};
use kontra_appcore::{Opened, PackageAudio};
use serde_json::Value;

/// A five-string bass line as MuseScore 4 saves it (trimmed): two bars with tab.
const MSCX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<museScore version="4.70"><Score>
  <metaTag name="workTitle">Downloaded Groove</metaTag>
  <Part id="1"><Staff><StaffType group="tablature"/></Staff><trackName>Bass</trackName>
    <Instrument id="electric-bass-5-str"><instrumentId>pluck.bass.electric</instrumentId>
      <StringData><frets>24</frets><string>23</string><string>28</string><string>33</string><string>38</string><string>43</string></StringData>
    </Instrument></Part>
  <Staff id="1">
    <Measure><voice><TimeSig><sigN>4</sigN><sigD>4</sigD></TimeSig>
      <Chord><durationType>half</durationType><Note><pitch>23</pitch><fret>0</fret><string>4</string></Note></Chord>
      <Chord><durationType>half</durationType><Note><pitch>28</pitch><fret>0</fret><string>3</string></Note></Chord>
    </voice></Measure>
    <Measure><voice><Chord><durationType>whole</durationType><Note><pitch>33</pitch><fret>0</fret><string>2</string></Note></Chord></voice></Measure>
  </Staff>
</Score></museScore>
"#;

fn downloads(core: &kontra_appcore::Core) -> Vec<Value> {
    let json: Value = serde_json::from_str(&core.downloads_json()).unwrap();
    json["files"].as_array().unwrap().clone()
}

#[test]
fn song_from_the_web() {
    let profile = Profile::new("b6");
    let folder = profile.dir.join("Downloads");
    std::fs::create_dir_all(&folder).unwrap();
    let mut core = profile.core(None);
    core.set_download_dir(&folder);

    // Step 1: the search opens the site's own page; Kontra fetches nothing.
    let url = core.search_url("songsterr", "Seven Nation Army");
    assert_eq!(
        url,
        "https://www.songsterr.com/?pattern=Seven%20Nation%20Army"
    );
    assert!(core
        .search_url("musescore", "Autumn Leaves")
        .starts_with("https://musescore.com/sheetmusic?text="));
    assert_eq!(core.search_url("songsterr", "  "), "");
    assert!(downloads(&core).is_empty());

    // Step 2-3: the downloaded file shows with its kind; one click opens it with the bass.
    std::fs::write(folder.join("groove.mscx"), MSCX).unwrap();
    std::fs::write(folder.join("invoice.pdf"), b"%PDF").unwrap();
    let files = downloads(&core);
    assert_eq!(files.len(), 1, "{files:?}");
    assert_eq!(files[0]["name"], "groove.mscx");
    assert_eq!(files[0]["kind"], "MuseScore");
    assert_eq!(files[0]["age"], "gerade eben");
    let path = files[0]["path"].as_str().unwrap();
    assert_eq!(core.open(path.as_ref()).unwrap(), Opened::Ready);
    let a = core.arrangement().unwrap();
    assert_eq!(a.title, "Downloaded Groove");
    assert_eq!(a.instrument.tuning.strings.len(), 5);
    assert_eq!(a.notes.len(), 3);

    // Opened once, it is in the library and no longer new.
    assert!(downloads(&core).is_empty());

    // Step 4: several files dropped at once: added without playing; a broken one is named.
    let dropped = profile.dir.join("dropped");
    std::fs::create_dir_all(&dropped).unwrap();
    write_midi(&dropped.join("riff.mid"), "Riff", &[28, 33, 38, 43]);
    // A package made from a MIDI piece (its score is no zip of its own).
    write_midi(&profile.dir.join("lesson.mid"), "Lesson", &[28, 31, 33]);
    core.open(&profile.dir.join("lesson.mid")).unwrap();
    let package = dropped.join("lesson.kontra");
    core.export_package(
        &profile.dir.join("lesson.mid"),
        &package,
        PackageAudio::Link,
    )
    .unwrap();
    std::fs::write(dropped.join("cut.mscz"), b"PK\x03\x04 cut").unwrap();
    let (added, failed) = core.import_files(&[
        dropped.join("riff.mid"),
        package.clone(),
        dropped.join("cut.mscz"),
    ]);
    assert_eq!(added, 2);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0].0, "cut.mscz");
    assert!(failed[0].1.starts_with("Die Datei"), "{failed:?}");
}
