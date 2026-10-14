//! J-B6 Song aus dem Netz (P2 Jens, P3 Mira, P1 Lea): Songsterr search in Kontra (a fake web,
//! never the network), search page for the browser, the downloaded file under "New in
//! Downloads", a MuseScore file dropped onto the library.

use crate::common::{write_midi, Profile};
use kontra_appcore::{Core, Opened, PackageAudio};
use kontra_session::{Http, HttpError};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Songsterr's answer for "seven nation army" (14.10.2026, shortened).
const SONGSTERR: &str = r#"[
  {"songId":265,"artist":"The White Stripes","title":"Seven Nation Army","tracks":[
    {"instrumentId":29,"tuning":[64,59,55,50,45,40]},{"instrumentId":1024}],"isJunk":false},
  {"songId":397405,"artist":"The White Stripes","title":"Seven Nation Army (With Bass Guitar)",
   "tracks":[{"instrumentId":29},{"instrumentId":33,"tuning":[43,38,33,28]}],"popularTrackBass":1}
]"#;

/// A web that answers from memory and counts requests.
#[derive(Debug)]
struct FakeWeb {
    answer: Result<Vec<u8>, HttpError>,
    urls: Mutex<Vec<String>>,
}

impl Http for FakeWeb {
    fn get(&self, url: &str) -> Result<Vec<u8>, HttpError> {
        self.urls.lock().unwrap().push(url.to_string());
        self.answer.clone()
    }
}

/// Ticks like the UI until the search is done; its JSON, `None` after 5 s.
fn finished(core: &mut Core) -> Option<Value> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if let Some(json) = core.song_search_poll() {
            return serde_json::from_str(&json).ok();
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    None
}

#[test]
fn search_songsterr_in_kontra() {
    let profile = Profile::new("b6-songsterr");
    let mut core = profile.core(None);
    let web = Arc::new(FakeWeb {
        answer: Ok(SONGSTERR.as_bytes().to_vec()),
        urls: Mutex::new(Vec::new()),
    });
    core.set_song_http(web.clone());

    // Step 1: one request on Enter; a second one while it runs sends nothing.
    let running: Value = serde_json::from_str(&core.song_search("seven nation army")).unwrap();
    assert_eq!(running["state"], "running");
    core.song_search("seven nation army");
    let done = finished(&mut core).expect("search finishes");
    assert_eq!(web.urls.lock().unwrap().len(), 1);
    assert_eq!(done["state"], "done");

    // Title, artist, bass in words (P7), the page of the bass track for the browser.
    let hits = done["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0]["bass"], false);
    assert_eq!(hits[0]["detail"], "ohne Bassspur");
    assert_eq!(hits[1]["detail"], "Bass: E A D G");
    assert!(hits[1]["url"]
        .as_str()
        .unwrap()
        .starts_with("https://www.songsterr.com/a/wsa/"));
    assert!(hits[1]["url"].as_str().unwrap().ends_with("-tab-s397405t1"));

    // Step 5: offline the reason is named; nothing else breaks.
    core.set_song_http(Arc::new(FakeWeb {
        answer: Err(HttpError::Offline("no route".into())),
        urls: Mutex::new(Vec::new()),
    }));
    core.song_search("seven nation army");
    let failed = finished(&mut core).expect("search finishes");
    assert_eq!(failed["state"], "error");
    assert!(failed["message"]
        .as_str()
        .unwrap()
        .starts_with("Keine Verbindung"));
    assert!(failed["hits"].as_array().unwrap().is_empty());
}

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
