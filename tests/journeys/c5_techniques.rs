//! J-C5 Techniken üben (P3 Mira, P4 Thomas): a MusicXML groove with a hammer-on, a pull-off, a
//! slide, dead notes (x) and a ghost note ( ), played with those techniques; every fourth technique note is plucked
//! instead. The techniques are told apart from the hit rate: each note gets a technique result,
//! the evaluation counts them per technique ("Hammer-on: 1 von 1"), and a plucked technique note
//! still counts as a hit. Over the cable a slap is a hint (Runde 17): a plucked slap is no
//! technique error.

use crate::common::{render, run_until, write_marked, Profile, COUNT_IN_S, RATE};
use kontra_appcore::UiEvent;
use std::time::Duration;

/// E1 h F♯1 p E1, A1 sl B1, then dead E1 (x), ghost A1 ( ), dead A1 (x); 120 BPM quarters on
/// the E and A strings (two bars).
const GROOVE: &str = r#"<?xml version="1.0"?>
<score-partwise><part-list><score-part id="P1"><part-name>Bass</part-name></score-part></part-list>
<part id="P1">
<measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>
<sound tempo="120"/>
<note><pitch><step>E</step><octave>1</octave></pitch><duration>1</duration><notations><technical><hammer-on type="start"/><string>4</string><fret>0</fret></technical></notations></note>
<note><pitch><step>F</step><alter>1</alter><octave>1</octave></pitch><duration>1</duration><notations><technical><hammer-on type="stop"/><pull-off type="start"/><string>4</string><fret>2</fret></technical></notations></note>
<note><pitch><step>E</step><octave>1</octave></pitch><duration>1</duration><notations><technical><pull-off type="stop"/><string>4</string><fret>0</fret></technical></notations></note>
<note><pitch><step>A</step><octave>1</octave></pitch><duration>1</duration><notations><slide type="start"/><technical><string>4</string><fret>5</fret></technical></notations></note>
</measure>
<measure number="2">
<note><pitch><step>B</step><octave>1</octave></pitch><duration>1</duration><notations><slide type="stop"/><technical><string>4</string><fret>7</fret></technical></notations></note>
<note><pitch><step>E</step><octave>1</octave></pitch><duration>1</duration><notehead>x</notehead><notations><technical><string>4</string><fret>0</fret></technical></notations></note>
<note><pitch><step>A</step><octave>1</octave></pitch><duration>1</duration><notehead parentheses="yes">normal</notehead><notations><technical><string>3</string><fret>0</fret></technical></notations></note>
<note><pitch><step>A</step><octave>1</octave></pitch><duration>1</duration><notehead>x</notehead><notations><technical><string>3</string><fret>0</fret></technical></notations></note>
</measure>
</part></score-partwise>"#;

/// Five judged technique notes: hammer-on, pull-off, slide, dead, dead. The fourth is plucked;
/// the ghost note ( ) is a soft pluck and only shows its sign (round 10).
const PLAIN_EVERY: usize = 4;

#[test]
fn techniques_are_judged_apart_from_the_hit_rate() {
    let profile = Profile::new("c5");
    let score = profile.dir.join("Groove.musicxml");
    std::fs::write(&score, GROOVE).unwrap();
    let take = profile.dir.join("techniken.wav");

    // Step 1: the groove; the views get a sign per technique note.
    let mut core = profile.core(None);
    core.open(&score).unwrap();
    let notes: serde_json::Value = serde_json::from_str(&core.notes_json()).unwrap();
    let signs: Vec<&str> = notes
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["technique"].as_str().unwrap())
        .collect();
    assert_eq!(
        signs,
        ["", "hammer", "pull", "slide", "", "dead", "ghost", "dead"],
        "a slide is marked where it starts (round 10)"
    );
    let staff: serde_json::Value = serde_json::from_str(&core.staff_json()).unwrap();
    let written: Vec<&str> = staff["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["technique"].as_str().unwrap())
        .collect();
    assert_eq!(written, signs, "the notation shows the same signs");
    let a = core.arrangement().unwrap().clone();
    let mistakes = kontra_session::Mistakes {
        playing: kontra_session::Playing::Articulated {
            plain_every: PLAIN_EVERY,
        },
        ..Default::default()
    };
    let samples = kontra_session::render_performance(&a, 1.0, COUNT_IN_S, mistakes, RATE);
    kontra_session::write_mono_wav(&take, &samples, RATE).unwrap();

    // Step 2: played in song mode with the techniques.
    let mut core = profile.core(Some(&take));
    core.open(&score).unwrap();
    core.set_mode("song");
    core.play();
    let events = run_until(&mut core, Duration::from_secs(10), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let mut told: Vec<(usize, &str, &str)> = events
        .iter()
        .filter_map(|e| match e {
            UiEvent::Technique {
                index,
                technique,
                result,
            } => Some((*index, *technique, *result)),
            _ => None,
        })
        .collect();
    told.sort();
    assert_eq!(
        told,
        [
            (1, "hammer", "played"),
            (2, "pull", "played"),
            (3, "slide", "played"),
            (5, "dead", "other"),
            (7, "dead", "played"),
        ],
        "one result per judged technique note, a slide's at its start; the plucked dead note is \
         another technique"
    );
    assert!(
        !events.iter().any(|e| matches!(e, UiEvent::Extra)),
        "hammer-on and pull-off targets are no extra notes"
    );

    // Step 3: the evaluation counts them apart; the hit rate is unchanged.
    let summary = events
        .iter()
        .find_map(|e| match e {
            UiEvent::Finished { summary_json } => Some(summary_json.clone()),
            _ => None,
        })
        .unwrap();
    let summary: serde_json::Value = serde_json::from_str(&summary).unwrap();
    let texts: Vec<&str> = summary["techniques"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["text"].as_str().unwrap())
        .collect();
    assert_eq!(
        texts,
        [
            "Slide: 1 von 1",
            "Hammer-on: 1 von 1",
            "Pull-off: 1 von 1",
            "Dead Note: 1 von 2"
        ]
    );
    assert_eq!(
        summary["hitRate"], 1.0,
        "a plucked dead note hits, and a dead note is not judged by its pitch"
    );
    assert_eq!(summary["untoldText"], "");
}

/// Step 4 (Runde 17): a slap groove over the cable with the slap plucked. Slap and pop are hints:
/// no technique error, nothing counted, and the evaluation says why.
#[test]
fn slap_over_the_cable_is_a_hint() {
    let profile = Profile::new("c5-slap");
    let song = profile.dir.join("Slap.mid");
    write_marked(&song, "Slap", &[28, 31, 33, 28], &["slap"]);
    let take = profile.dir.join("slap.wav");

    let mut core = profile.core(None);
    core.open(&song).unwrap();
    let notes: serde_json::Value = serde_json::from_str(&core.notes_json()).unwrap();
    assert_eq!(notes[0]["technique"], "slap");
    let mistakes = kontra_session::Mistakes {
        playing: kontra_session::Playing::Articulated { plain_every: 1 },
        ..Default::default()
    };
    render(&take, core.arrangement().unwrap(), mistakes);

    let mut core = profile.core(Some(&take));
    core.open(&song).unwrap();
    core.set_mode("song");
    core.play();
    let events = run_until(&mut core, Duration::from_secs(10), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let marks: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            UiEvent::Technique { result, .. } => Some(*result),
            _ => None,
        })
        .collect();
    assert!(
        marks.iter().all(|r| r.is_empty()),
        "a plucked slap is no technique error: {marks:?}"
    );

    let summary = events
        .iter()
        .find_map(|e| match e {
            UiEvent::Finished { summary_json } => Some(summary_json.clone()),
            _ => None,
        })
        .unwrap();
    let summary: serde_json::Value = serde_json::from_str(&summary).unwrap();
    assert_eq!(
        summary["techniques"],
        serde_json::json!([]),
        "nothing counted"
    );
    assert_eq!(summary["hitRate"], 1.0, "the note counts");
    assert_eq!(
        summary["untoldText"],
        "Nur als Hinweis gezeigt, nicht gewertet: Slap. Ihre Erkennung ist noch nicht bei jedem \
         Bass sicher."
    );
}
