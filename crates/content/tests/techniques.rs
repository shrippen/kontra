//! Techniques from MusicXML (LH-BEW-06): a slide is marked where it starts, ghost notes from
//! note heads in parentheses, hammer-ons and pull-offs on their targets.

// Test helpers outside #[test] functions may panic too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kontra_content::{import_bytes, FileKind};
use kontra_model::Technique;

const NOTES: &str = r#"
<note><pitch><step>E</step><octave>1</octave></pitch><duration>1</duration><notations><technical><hammer-on type="start"/></technical></notations></note>
<note><pitch><step>F</step><alter>1</alter><octave>1</octave></pitch><duration>1</duration><notations><technical><hammer-on type="stop"/></technical></notations></note>
<note><pitch><step>A</step><octave>1</octave></pitch><duration>1</duration><notations><slide type="start"/></notations></note>
<note><pitch><step>B</step><octave>1</octave></pitch><duration>1</duration><notations><slide type="stop"/></notations></note>
<note><pitch><step>E</step><octave>1</octave></pitch><duration>4</duration><notehead parentheses="yes">normal</notehead></note>
"#;

#[test]
fn musicxml_techniques() {
    let text = format!(
        r#"<?xml version="1.0"?><score-partwise><part-list><score-part id="P1"><part-name>Bass</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>{NOTES}</measure></part></score-partwise>"#
    );
    let result = import_bytes(text.as_bytes(), FileKind::MusicXml).unwrap();
    let techniques: Vec<Vec<Technique>> = result
        .arrangement(0)
        .unwrap()
        .notes
        .iter()
        .map(|n| n.techniques.clone())
        .collect();
    assert_eq!(
        techniques,
        vec![
            vec![],
            vec![Technique::HammerOn],
            vec![Technique::Slide],
            vec![],
            vec![Technique::Ghost],
        ]
    );
}
