//! Repeats and directions (D.C., D.S., segno, coda, fine) in every format with bars.

// Test helpers outside #[test] functions may panic too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Method;
use kontra_content::{import_bytes, FileKind};
use kontra_model::Arrangement;

fn order(a: &Arrangement) -> Vec<u32> {
    a.bars.iter().map(|b| b.source_index).collect()
}

/// Pitches played, one whole note per bar: E1 + source bar index.
fn pitches(a: &Arrangement) -> Vec<u8> {
    a.notes.iter().map(|n| n.pitch.0).collect()
}

#[test]
fn gp5_dal_segno_al_coda_skips_repeat() {
    let data = common::gp5(&common::directions_song());
    let a = import_bytes(&data, FileKind::GuitarPro)
        .unwrap()
        .arrangement(0)
        .unwrap();
    // 0 1 :| twice, Segno at 2, to Coda at 3, D.S. al Coda at 4, Coda at 5.
    assert_eq!(order(&a), [0, 1, 0, 1, 2, 3, 4, 2, 3, 5]);
    assert_eq!(pitches(&a), [28, 29, 28, 29, 30, 31, 32, 30, 31, 33]);
    assert_eq!(a.bars[7].start_ticks, 7 * 3840);
}

#[test]
fn gp5_jump_to_missing_sign_is_ignored() {
    let mut song = common::directions_song();
    song.directions[common::SLOT_SEGNO] = -1;
    let data = common::gp5(&song);
    let a = import_bytes(&data, FileKind::GuitarPro)
        .unwrap()
        .arrangement(0)
        .unwrap();
    assert_eq!(order(&a), [0, 1, 0, 1, 2, 3, 4, 5]);
}

fn gpif_order(extras: &[&str]) -> Vec<u32> {
    let score = common::gpif_bars(extras);
    let data = common::zip(&[("Content/score.gpif", score.as_bytes())], Method::Stored);
    let a = import_bytes(&data, FileKind::GuitarPro7)
        .unwrap()
        .arrangement(0)
        .unwrap();
    order(&a)
}

#[test]
fn gpif_da_capo_al_fine() {
    let order = gpif_order(&[
        r#"<Repeat start="true" end="false" count="0"/>"#,
        r#"<Repeat start="false" end="true" count="2"/><Directions><Target>Fine</Target></Directions>"#,
        "",
        "<Directions><Jump>DaCapoAlFine</Jump></Directions>",
    ]);
    // Repeat played twice the first time, once after D.C.; stop at Fine.
    assert_eq!(order, [0, 1, 0, 1, 2, 3, 0, 1]);
}

#[test]
fn gpif_segno_segno_al_double_coda() {
    let order = gpif_order(&[
        "",
        "<Directions><Target>SegnoSegno</Target></Directions>",
        "<Directions><Jump>DaDoubleCoda</Jump></Directions>",
        "<Directions><Jump>DaSegnoSegnoAlDoubleCoda</Jump></Directions>",
        "",
        "<Directions><Target>DoubleCoda</Target></Directions>",
    ]);
    assert_eq!(order, [0, 1, 2, 3, 1, 2, 5]);
}

#[test]
fn gpx_da_capo_plays_last_ending() {
    let score = common::gpif_bars(&[
        r#"<Repeat start="true" end="false" count="0"/>"#,
        r#"<Repeat start="false" end="true" count="2"/><AlternateEndings>1</AlternateEndings>"#,
        "<AlternateEndings>2</AlternateEndings>",
        "<Directions><Jump>DaCapo</Jump></Directions>",
    ]);
    let data = common::bcfz(&common::bcfs(&[("score.gpif", score.as_bytes())]));
    let a = import_bytes(&data, FileKind::GuitarPro6)
        .unwrap()
        .arrangement(0)
        .unwrap();
    assert_eq!(order(&a), [0, 1, 0, 2, 3, 0, 2, 3]);
}

fn musicxml_order(extras: &[&str]) -> Vec<u32> {
    let text = common::musicxml_bars(extras);
    let a = import_bytes(text.as_bytes(), FileKind::MusicXml)
        .unwrap()
        .arrangement(0)
        .unwrap();
    order(&a)
}

#[test]
fn musicxml_dal_segno_to_coda() {
    let order = musicxml_order(&[
        "",
        r#"<direction><direction-type><segno/></direction-type><sound segno="s1"/></direction>"#,
        r#"<direction><direction-type><words>To Coda</words></direction-type><sound tocoda="c1"/></direction>"#,
        r#"<direction><direction-type><words>D.S. al Coda</words></direction-type><sound dalsegno="s1"/></direction>"#,
        r#"<direction><direction-type><coda/></direction-type><sound coda="c1"/></direction>"#,
    ]);
    assert_eq!(order, [0, 1, 2, 3, 1, 2, 4]);
}

#[test]
fn musicxml_da_capo_al_fine_with_repeat() {
    let order = musicxml_order(&[
        r#"<barline location="left"><repeat direction="forward"/></barline>"#,
        r#"<barline location="right"><repeat direction="backward"/></barline><sound fine="yes"/>"#,
        r#"<barline location="right"><sound dacapo="yes"/></barline>"#,
    ]);
    assert_eq!(order, [0, 1, 0, 1, 2, 0, 1]);
}

#[test]
fn musicxml_bars_keep_written_repeat_signs_and_endings() {
    // |: A | B(1.) :| C(2.) |  → played A B A C; every played bar shows its written signs.
    let text = common::musicxml_bars(&[
        r#"<barline location="left"><repeat direction="forward"/></barline>"#,
        r#"<barline location="left"><ending number="1" type="start"/></barline><barline location="right"><ending number="1" type="stop"/><repeat direction="backward"/></barline>"#,
        r#"<barline location="left"><ending number="2" type="start"/></barline><barline location="right"><ending number="2" type="stop"/></barline>"#,
    ]);
    let a = import_bytes(text.as_bytes(), FileKind::MusicXml)
        .unwrap()
        .arrangement(0)
        .unwrap();
    assert_eq!(order(&a), [0, 1, 0, 2]);
    let signs: Vec<(bool, bool, u32)> = a
        .bars
        .iter()
        .map(|b| (b.repeat_open, b.repeat_close, b.endings))
        .collect();
    assert_eq!(
        signs,
        [
            (true, false, 0),
            (false, true, 1),
            (true, false, 0),
            (false, false, 2)
        ]
    );
}
