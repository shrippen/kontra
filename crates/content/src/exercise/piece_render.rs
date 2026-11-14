//! Renders a `PiecePlan` as bass line, chord names, rehearsal marks and drums.
//!
//! ```text
//! plan ─check─▶ bars of chords ─▶ one slot per bar played (parts × times)
//!   slot: groove_tones ─▶ line (roots, fifth, chord tones, walking, approach)
//!         ─▶ slash bass ─▶ seeded twist ─▶ fill on the part's last beat
//!   parts ─▶ drums (groove, light, none; a snare fill before the next part)
//! ```

use super::chord::Chord;
use super::piece_plan::{Checked, CheckedPart};
use super::variety::{fill, lead_in, side, BarAt, Reach, Rng, Run};
use super::*;

/// Chance of a twist in a bar: one in this many in the first play of a part, in the repeats.
const TWIST_FIRST: usize = 10;
const TWIST_REPEAT: usize = 3;
/// A fill is drawn once in this many at the end of a part.
const FILL_ODDS: usize = 2;
/// A whole-step approach is drawn once in this many.
const WHOLE_ODDS: usize = 3;
/// General MIDI snare roll of a drum fill: four sixteenths on the last beat.
const FILL_STROKES: u32 = 4;
const HALF_BAR: u32 = BAR / 2;

/// What the rendering of a plan gives: the pass for the writer, drums and rehearsal marks.
pub(super) struct Rendered {
    pub(super) pass: Pass,
    pub(super) drums: Vec<(u32, u8)>,
    pub(super) parts: Vec<(u32, String)>,
    pub(super) key: u8,
}

/// The suffix of a title that tells a variant: nothing for the plain piece.
pub(super) fn variant_suffix(seed: u64) -> String {
    if seed == 0 {
        return String::new();
    }
    format!(" · Variante {seed}")
}

/// The suffix of a title that tells a length other than normal: a different length is a
/// different piece (LH-BIB-15), so it needs its own title and progress, like a variant.
pub(super) fn length_suffix(length: Length) -> &'static str {
    match length {
        Length::Short => " · kurz",
        Length::Normal => "",
        Length::Long => " · lang",
    }
}

impl Section {
    /// The rehearsal mark over the part's first bar, as German bands say it (LH-BIB-12); the
    /// twelve bars of a shuffle's blues are a chorus. The app shows Strophe and Refrain in its
    /// language (`GENERATED_PARTS` in appcore).
    fn mark(self, feel: PieceFeel) -> &'static str {
        match (self, feel) {
            (Section::Intro, _) => "Intro",
            (Section::Verse, PieceFeel::Shuffle) => "Chorus",
            (Section::Verse, PieceFeel::Straight) => "Strophe",
            (Section::PreChorus, _) => "Pre-Chorus",
            (Section::Chorus, _) => "Refrain",
            (Section::Bridge, _) => "Bridge",
            (Section::Solo, _) => "Solo",
            (Section::Outro, _) => "Outro",
        }
    }
}

/// Renders `plan` as an exercise: the plan's title, key and tempo in the name, `seed` 0 for
/// the plain piece.
pub fn render_piece(plan: &PiecePlan, level: Level, seed: u64) -> Result<Exercise, Vec<PlanError>> {
    render_at(
        plan,
        level,
        seed,
        &variant_suffix(seed),
        f64::from(plan.bpm),
    )
}

/// `render_piece_json`, but the title tells the piece apart by `number` instead of by the
/// seed: no suffix for 1, " · Variante 2" for 2 and so on. The seed only varies the notes.
pub fn render_piece_numbered(
    text: &str,
    level: Level,
    seed: u64,
    number: u32,
) -> Result<Exercise, Vec<PlanError>> {
    let plan = PiecePlan::from_json(text).map_err(|e| vec![e])?;
    // The first piece has no suffix, like seed 0.
    let suffix = if number > 1 {
        variant_suffix(u64::from(number))
    } else {
        String::new()
    };
    render_at(&plan, level, seed, &suffix, f64::from(plan.bpm))
}

/// `render_piece` for a plan in JSON; a plan that does not read is one `Json` error.
pub fn render_piece_json(text: &str, level: Level, seed: u64) -> Result<Exercise, Vec<PlanError>> {
    let plan = PiecePlan::from_json(text).map_err(|e| vec![e])?;
    render_piece(&plan, level, seed)
}

fn render_at(
    plan: &PiecePlan,
    level: Level,
    seed: u64,
    suffix: &str,
    bpm: f64,
) -> Result<Exercise, Vec<PlanError>> {
    let rendered = pass_of(plan, level, seed)?;
    let pass = &rendered.pass;
    let key = signature(rendered.key, pass.to_major);
    let key_name = format!(
        "{}{}",
        name(i32::from(rendered.key), key),
        if pass.minor { "m" } else { "" }
    );
    let title = format!("{} in {key_name} ({bpm:.0} BPM){suffix}", pass.label,);
    let line = Line {
        tones: &pass.tones,
        markers: &pass.markers,
        parts: &rendered.parts,
        drums: &rendered.drums,
        tempos: &[(0, bpm)],
        keys: &[(0, key, pass.minor)],
        meter: Meter::Common,
        judging: Judging::PitchAndTime,
        feel: match plan.feel {
            PieceFeel::Straight => Feel::Straight,
            PieceFeel::Shuffle => Feel::Shuffle,
        },
    };
    let midi = write(&title, &line);
    Ok(Exercise { title, midi })
}

/// One bar played: part, play of the part, bar of the part.
struct Slot<'a> {
    part: usize,
    time: u32,
    bar: usize,
    chords: &'a [Chord],
}

/// Everything the notes of one bar depend on.
struct Bar<'a> {
    start: u32,
    chords: &'a [Chord],
    roots: Vec<i32>,
    rhythm: Rhythm,
    line: PlanLine,
    /// The root the bar leads to.
    next: i32,
    /// Slot number: a walking line alternates its shape with it.
    index: usize,
}

/// The whole piece as one pass; the tonic closes it. `None` where `plan` does not check out.
pub(super) fn pass_of(
    plan: &PiecePlan,
    level: Level,
    seed: u64,
) -> Result<Rendered, Vec<PlanError>> {
    let checked = plan.check()?;
    Ok(render_checked(
        &checked,
        plan.feel,
        bare_title(&plan.title, &plan.key),
        level,
        seed,
    ))
}

/// Words a title may put after the key's root to name its mode: "A minor", "A-Moll".
const MODE_WORDS: [&str; 4] = ["minor", "major", "moll", "dur"];

/// The plan's title without a key at its end: the rendered title adds the key itself
/// (QA 14.11.: a model's "Motown in A" became "Motown in A in A (104 BPM)"). Takes the key as
/// written ("Am"), its root ("A") and the root with a mode word ("A minor", "A-Moll").
fn bare_title<'a>(title: &'a str, key: &str) -> &'a str {
    let title = title.trim();
    let key = key.trim();
    let root = key.strip_suffix('m').unwrap_or(key);
    let mut endings = vec![key.to_string(), root.to_string()];
    for word in MODE_WORDS {
        endings.push(format!("{root} {word}"));
        endings.push(format!("{root}-{word}"));
    }

    for ending in endings {
        let suffix = format!(" in {ending}");
        let cut = title.len().saturating_sub(suffix.len());
        let Some(end) = title.get(cut..) else {
            continue;
        };
        if cut > 0 && end.eq_ignore_ascii_case(&suffix) {
            return title[..cut].trim_end();
        }
    }
    title
}

/// `pass_of` once `plan` is known to check out (a built-in style's own plan, never bad).
pub(super) fn render_checked(
    checked: &Checked,
    feel: PieceFeel,
    title: &str,
    level: Level,
    seed: u64,
) -> Rendered {
    let base = low_root(checked.key);
    let fifths = key_fifths(checked.key, checked.minor);
    let root_of = |chord: &Chord| nearest(i32::from(chord.root), base);

    let mut slots = Vec::new();
    for (p, part) in checked.parts.iter().enumerate() {
        for time in 0..part.part.times {
            for (bar, chords) in part.bars.iter().enumerate() {
                slots.push(Slot {
                    part: p,
                    time,
                    bar,
                    chords,
                });
            }
        }
    }
    let closing = closing_chord(checked, fifths);
    let closing_root = root_of(&closing);
    let scale = key_scale(checked);

    let mut tones = Vec::new();
    let mut markers = Vec::new();
    for (i, slot) in slots.iter().enumerate() {
        let part = &checked.parts[slot.part];
        let start = i as u32 * BAR;
        let next = slots
            .get(i + 1)
            .map_or(closing_root, |s| root_of(&s.chords[0]));
        let bar = Bar {
            start,
            chords: slot.chords,
            roots: slot.chords.iter().map(root_of).collect(),
            rhythm: level.rhythm(part.rhythm),
            line: part.part.line,
            next,
            index: i,
        };
        for (k, chord) in slot.chords.iter().enumerate() {
            markers.push((start + k as u32 * HALF_BAR, chord.name.clone()));
        }

        let mut rng = Rng::new(
            seed,
            &[slot.part as u64, u64::from(slot.time), slot.bar as u64],
        );
        let mut bar_tones = bar_tones(&bar, seed, &mut rng);
        let odds = if slot.time == 0 {
            TWIST_FIRST
        } else {
            TWIST_REPEAT
        };
        if seed != 0 && slot.chords.len() == 1 && rng.one_in(odds) {
            let at = BarAt {
                start,
                chord: &slot.chords[0],
                root: bar.roots[0],
                next,
            };
            super::variety::vary(&mut bar_tones, &at, level, &mut rng);
        }

        // The last beat of a part: a run to the next part's first note.
        let ends_part = slot.time + 1 == part.part.times && slot.bar + 1 == part.bars.len();
        let seeded_fill = seed != 0 && rng.one_in(FILL_ODDS);
        if ends_part && (part.part.fill || seeded_fill) {
            let run = if seed != 0 && rng.one_in(2) {
                Run::Chromatic
            } else {
                Run::Scale
            };
            let last = bar.roots.last().copied().unwrap_or(next);
            fill(
                &mut bar_tones,
                start + 3 * QUARTER,
                next,
                run,
                side(last, next),
                &scale,
            );
        }
        tones.extend(bar_tones);
    }

    let FormLayout {
        parts,
        mut drums,
        end,
    } = parts_and_drums(checked, feel, &slots);
    markers.push((end, closing.name.clone()));
    tones.push(Tone {
        start: end,
        length: BAR,
        key: closing_root,
        stroke: Stroke::Plain,
        position: None,
    });
    if checked
        .parts
        .last()
        .is_some_and(|p| p.part.drums != Drums::None)
    {
        drums.extend([(end, KICK), (end, HIHAT)]);
    }

    let pass = Pass {
        tones,
        markers,
        length: end + BAR,
        minor: checked.minor,
        to_major: if checked.minor { RELATIVE_MAJOR } else { 0 },
        label: format!("{}{}", title.trim(), level.label()),
    };
    Rendered {
        pass,
        drums,
        parts,
        key: checked.key,
    }
}

/// The chord that ends the piece: the first chord on the tonic, else the plain tonic triad.
fn closing_chord(checked: &Checked, fifths: i8) -> Chord {
    let tonic = checked
        .parts
        .iter()
        .flat_map(|p| p.bars.iter().flatten())
        .find(|c| c.root == checked.key);
    let mut chord = tonic.cloned().unwrap_or_else(|| {
        let spelled = name(i32::from(checked.key), fifths);
        Chord::triad(checked.key, checked.minor, spelled)
    });
    // A slash bass belongs to the chord progression, not to the end.
    chord.bass = None;
    chord.name = chord.name.split('/').next().unwrap_or_default().to_string();
    chord
}

/// Pitch classes of the key's scale, for the runs of a fill.
fn key_scale(checked: &Checked) -> Vec<i32> {
    let steps: &[i32] = if checked.minor { &MINOR } else { &MAJOR };
    steps
        .iter()
        .map(|s| (i32::from(checked.key) + s).rem_euclid(OCTAVE))
        .collect()
}

/// One bar of notes: one chord fills it, two take a half each.
fn bar_tones(bar: &Bar, seed: u64, rng: &mut Rng) -> Vec<Tone> {
    let halves = bar.chords.len();
    let mut tones = Vec::new();
    for k in 0..halves {
        let (offset, length) = if halves == 1 {
            (0, BAR)
        } else {
            (k as u32 * HALF_BAR, HALF_BAR)
        };
        let next = if k + 1 < halves {
            bar.roots[k + 1]
        } else {
            bar.next
        };
        let reach = if seed != 0 && rng.one_in(WHOLE_ODDS) {
            Reach::Whole
        } else {
            Reach::Half
        };
        let seg = Segment {
            start: bar.start + offset,
            length,
            chord: &bar.chords[k],
            root: bar.roots[k],
            next,
            reach,
        };
        tones.extend(segment_tones(&seg, bar, k));
    }
    tones
}

/// The part of a bar one chord plays.
struct Segment<'a> {
    start: u32,
    length: u32,
    chord: &'a Chord,
    root: i32,
    next: i32,
    reach: Reach,
}

fn segment_tones(seg: &Segment, bar: &Bar, k: usize) -> Vec<Tone> {
    let mut tones = if bar.line == PlanLine::Walking {
        walk(seg, bar.index, k)
    } else {
        let raw = groove_tones(seg.start, seg.root, seg.chord.tones(), bar.rhythm);
        clip(raw, seg)
    };
    let roots = [seg.root, octave_root(seg.root)];
    let tone_at = |t: &Tone| t.stroke == Stroke::Plain && roots.contains(&t.key);
    let chord_tones = seg.chord.tones();
    match bar.line {
        PlanLine::RootFifth => {
            for t in tones.iter_mut().filter(|t| tone_at(t)) {
                if t.start - seg.start == seg.length / 2 {
                    t.key = fit(t.key + chord_tones[2]);
                }
            }
        }
        PlanLine::ChordTones => {
            for t in tones.iter_mut().filter(|t| tone_at(t)) {
                let rel = t.start - seg.start;
                if rel == 0 || !rel.is_multiple_of(QUARTER) {
                    continue;
                }
                let interval = match rel / QUARTER {
                    1 => chord_tones[1],
                    2 => chord_tones[2],
                    _ if seg.chord.has_color() => chord_tones[3],
                    _ => OCTAVE,
                };
                t.key = fit(t.key + interval);
            }
        }
        PlanLine::Approach => lead_into_next(&mut tones, seg),
        PlanLine::Roots | PlanLine::Walking => {}
    }

    // Slash chord: its bass note plays on the first beat.
    if let Some(bass) = seg.chord.bass {
        for t in tones.iter_mut().filter(|t| t.start == seg.start) {
            t.key = nearest(i32::from(bass), t.key);
        }
    }
    tones
}

/// What of a full bar of groove sounds in the segment; a note that rings over its end stops.
fn clip(raw: Vec<Tone>, seg: &Segment) -> Vec<Tone> {
    let end = seg.start + seg.length;
    raw.into_iter()
        .filter(|t| t.start < end)
        .map(|t| Tone {
            length: t.length.min(end - t.start),
            ..t
        })
        .collect()
}

/// The last pitched note before the chord changes leads to the next root.
fn lead_into_next(tones: &mut [Tone], seg: &Segment) {
    let Some(key) = lead_in(seg.chord, seg.root, seg.next, seg.reach) else {
        return;
    };
    let last = tones
        .iter_mut()
        .rfind(|t| t.stroke == Stroke::Plain && t.start > seg.start);
    if let Some(tone) = last {
        tone.key = key;
    }
}

/// A walking line in quarters: root, chord tones, an approach to the next root; in a half bar
/// the root and a chord tone (first half) or the approach (second half). Alternates its shape
/// so the line does not repeat itself bar after bar; without a seventh (`has_color`) the
/// alternate shape takes the octave instead, never the flat seventh `tones` falls back to.
fn walk(seg: &Segment, index: usize, half: usize) -> Vec<Tone> {
    let c = seg.chord.tones();
    let (second, third) = if index.is_multiple_of(2) {
        (c[1], c[2])
    } else {
        let seventh = if seg.chord.has_color() { c[3] } else { OCTAVE };
        (c[2], seventh)
    };
    let lead = approach_note(seg.root, seg.next);
    let notes = match (seg.length, half) {
        (BAR, _) => vec![seg.root, seg.root + second, seg.root + third, lead],
        (_, 0) => vec![seg.root, seg.root + second],
        _ => vec![seg.root, lead],
    };
    notes
        .into_iter()
        .enumerate()
        .map(|(i, key)| Tone {
            start: seg.start + i as u32 * QUARTER,
            length: QUARTER,
            key: fit(key).clamp(LOWEST, HIGHEST),
            stroke: Stroke::Plain,
            position: None,
        })
        .collect()
}

/// Rehearsal marks, drums and the tick the closing bar starts at.
struct FormLayout {
    parts: Vec<(u32, String)>,
    drums: Vec<(u32, u8)>,
    end: u32,
}

fn parts_and_drums(checked: &Checked, feel: PieceFeel, slots: &[Slot]) -> FormLayout {
    let mut marks = Vec::new();
    let mut drums = Vec::new();
    let mut start = 0;
    for (i, part) in checked.parts.iter().enumerate() {
        let bars = (part.part.times as usize * part.bars.len()) as u32;
        marks.push((start, part.part.section.mark(feel).to_string()));
        let last = i + 1 == checked.parts.len();
        drums.extend(part_drums(part, start, bars, last));
        start += bars * BAR;
    }
    debug_assert_eq!(start, slots.len() as u32 * BAR);
    FormLayout {
        parts: marks,
        drums,
        end: start,
    }
}

/// Drums of a part from `start`: the groove's beat (light: only kick and hi-hat), and a snare
/// fill on the last beat before the next part. The drums keep the plan's groove at any level.
fn part_drums(part: &CheckedPart, start: u32, bars: u32, last: bool) -> Vec<(u32, u8)> {
    if part.part.drums == Drums::None {
        return Vec::new();
    }
    let mut hits: Vec<(u32, u8)> = (0..bars)
        .flat_map(|b| drum_bar(start + b * BAR, part.rhythm))
        .filter(|(_, key)| part.part.drums == Drums::Full || *key != SNARE)
        .collect();
    if last {
        return hits;
    }
    let fill_from = start + bars * BAR - QUARTER;
    hits.retain(|(at, _)| *at < fill_from);
    hits.extend((0..FILL_STROKES).map(|s| (fill_from + s * SIXTEENTH, SNARE)));
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{import_bytes, FileKind};
    use kontra_model::TICKS_PER_QUARTER as MODEL_QUARTER;

    /// A minimal, valid plan with one part; override fields on the result as the test needs.
    fn plan(bars: &[&str], line: PlanLine, groove: &str) -> PiecePlan {
        PiecePlan {
            title: "Test".to_string(),
            key: "C".to_string(),
            bpm: 90,
            feel: PieceFeel::Straight,
            parts: vec![PlanPart {
                section: Section::Verse,
                bars: bars.iter().map(|b| (*b).to_string()).collect(),
                times: 1,
                groove: groove.to_string(),
                line,
                fill: false,
                drums: Drums::Full,
            }],
        }
    }

    fn render(plan: &PiecePlan, seed: u64) -> Exercise {
        render_piece(plan, Level::Medium, seed).expect("valid plan")
    }

    /// QA 14.11.: a title naming the key got the key twice.
    #[test]
    fn a_title_naming_the_key_names_it_once() {
        let mut p = plan(&["A"], PlanLine::Roots, "quarters");
        p.key = "A".to_string();
        p.title = "Motown in A".to_string();
        assert_eq!(render(&p, 0).title, "Motown in A (90 BPM)");
        p.title = "Motown in a".to_string();
        assert_eq!(render(&p, 0).title, "Motown in A (90 BPM)");
        p.title = "Rain in Spain".to_string();
        assert_eq!(render(&p, 0).title, "Rain in Spain in A (90 BPM)");
        p.title = " in A".to_string();
        assert_eq!(render(&p, 0).title, "in A in A (90 BPM)");
        // Review #327: other spellings of the key.
        p.key = "Am".to_string();
        p.parts[0].bars = vec!["Am".to_string()];
        for title in [
            "Groove in A minor",
            "Groove in A-Moll",
            "Groove in a moll",
            "Groove in A",
        ] {
            p.title = title.to_string();
            assert_eq!(render(&p, 0).title, "Groove in Am (90 BPM)", "{title}");
        }
        p.key = "F#".to_string();
        p.parts[0].bars = vec!["F#".to_string()];
        p.title = "Funk in F# major".to_string();
        assert_eq!(render(&p, 0).title, "Funk in F# (90 BPM)");
    }

    fn arrangement(exercise: &Exercise) -> kontra_model::Arrangement {
        let imported = import_bytes(&exercise.midi, FileKind::Midi).expect("imports");
        imported
            .arrangement(imported.suggested().expect("bass track"))
            .expect("arrangement")
    }

    /// Pitch classes of the notes in bar `index`, in order.
    fn bar_pitch_classes(a: &kontra_model::Arrangement, index: usize) -> Vec<i32> {
        let bar = &a.bars[index];
        let end = a.bars.get(index + 1).map_or(u64::MAX, |b| b.start_ticks);
        a.notes
            .iter()
            .filter(|n| (bar.start_ticks..end).contains(&n.start_ticks))
            .map(|n| i32::from(n.pitch.0).rem_euclid(OCTAVE))
            .collect()
    }

    /// Bug (reviewer): a walking line without a seventh played the flat seventh `tones()`
    /// falls back to (groove chords do mean to play it; `walk`'s *alternate shape* should not).
    /// Reproduced with key E, bar "A/C#", `line: walking`, seed 3 (`kontra-cli import` showed
    /// C#-E-G-A# over A major, i.e. a flat seventh (G) where no seventh was asked for).
    #[test]
    fn walking_without_a_seventh_never_adds_one() {
        // Two bars of a plain A major triad: the second (index 1) takes `walk`'s alternate
        // shape, which used to reach for `tones()[3]`, the flat-seventh fallback.
        let mut p = plan(&["A", "A"], PlanLine::Walking, "quarters");
        p.key = "E".to_string();
        let e = render(&p, 0);
        let a = arrangement(&e);
        let second_bar = bar_pitch_classes(&a, 1);
        let a_pc: i32 = 9; // A
        let flat_seventh = (a_pc + 10).rem_euclid(OCTAVE); // G
        assert!(
            !second_bar.contains(&flat_seventh),
            "{second_bar:?} contains the flat seventh of A"
        );
    }

    #[test]
    fn roots_line_never_leaves_the_root() {
        let p = plan(&["C", "G"], PlanLine::Roots, "eighths");
        let e = render(&p, 0);
        let a = arrangement(&e);
        for pc in bar_pitch_classes(&a, 0) {
            assert_eq!(pc, 0, "C major, roots line");
        }
    }

    #[test]
    fn root_fifth_line_adds_a_fifth_halfway_through_the_bar() {
        let p = plan(&["C"], PlanLine::RootFifth, "quarters");
        let e = render(&p, 0);
        let a = arrangement(&e);
        let third_beat = a
            .notes
            .iter()
            .find(|n| n.start_ticks == a.bars[0].start_ticks + 2 * MODEL_QUARTER)
            .expect("third beat");
        assert_eq!(i32::from(third_beat.pitch.0).rem_euclid(OCTAVE), 7, "fifth");
    }

    #[test]
    fn chord_tones_line_plays_third_fifth_and_octave() {
        let p = plan(&["C"], PlanLine::ChordTones, "quarters");
        let e = render(&p, 0);
        let a = arrangement(&e);
        let pcs: Vec<i32> = bar_pitch_classes(&a, 0);
        assert_eq!(pcs, vec![0, 4, 7, 0], "root, third, fifth, octave (no 7th)");
    }

    #[test]
    fn approach_line_leads_into_the_next_chord() {
        let p = plan(&["C", "F"], PlanLine::Approach, "eighths");
        let e = render(&p, 0);
        let a = arrangement(&e);
        let last = a
            .notes
            .iter()
            .filter(|n| n.start_ticks < a.bars[1].start_ticks)
            .max_by_key(|n| n.start_ticks)
            .expect("last note of bar 1");
        let pc = i32::from(last.pitch.0).rem_euclid(OCTAVE);
        // F is pitch class 5; a half or whole step away, not F itself.
        assert_ne!(pc, 5, "approach note, not the next root itself");
        let distance = (pc - 5).rem_euclid(OCTAVE).min((5 - pc).rem_euclid(OCTAVE));
        assert!(distance == 1 || distance == 2, "{pc} not close to F");
    }

    #[test]
    fn walking_line_plays_four_quarters_a_bar() {
        let p = plan(&["C", "G"], PlanLine::Walking, "eighths");
        let e = render(&p, 0);
        let a = arrangement(&e);
        assert_eq!(bar_pitch_classes(&a, 0).len(), 4);
        assert_eq!(bar_pitch_classes(&a, 0)[0], 0, "starts on the root");
    }

    #[test]
    fn slash_chord_plays_its_bass_note_on_beat_one() {
        let p = plan(&["C/E"], PlanLine::Roots, "quarters");
        let e = render(&p, 0);
        let a = arrangement(&e);
        let first = a.notes.first().expect("a note");
        assert_eq!(
            i32::from(first.pitch.0).rem_euclid(OCTAVE),
            4,
            "E on beat one"
        );
    }

    #[test]
    fn two_chords_split_the_bar() {
        let p = plan(&["C G"], PlanLine::Roots, "quarters");
        let e = render(&p, 0);
        let a = arrangement(&e);
        assert!(a
            .markers
            .iter()
            .any(|m| m.tick == a.bars[0].start_ticks && m.name == "C"));
        let half = a.bars[0].start_ticks + 2 * MODEL_QUARTER;
        assert!(a.markers.iter().any(|m| m.tick == half && m.name == "G"));
        let first_half: Vec<i32> = a
            .notes
            .iter()
            .filter(|n| n.start_ticks < half)
            .map(|n| i32::from(n.pitch.0).rem_euclid(OCTAVE))
            .collect();
        let second_half: Vec<i32> = a
            .notes
            .iter()
            .filter(|n| n.start_ticks >= half && n.start_ticks < a.bars[1].start_ticks)
            .map(|n| i32::from(n.pitch.0).rem_euclid(OCTAVE))
            .collect();
        assert!(first_half.iter().all(|pc| *pc == 0), "{first_half:?}");
        assert!(second_half.iter().all(|pc| *pc == 7), "{second_half:?}");
    }

    #[test]
    fn a_fill_plays_at_least_three_notes_on_the_last_beat() {
        let mut p = plan(&["C"], PlanLine::Roots, "quarters");
        p.parts[0].fill = true;
        p.parts.push(PlanPart {
            section: Section::Outro,
            bars: vec!["F".to_string()],
            times: 1,
            groove: "quarters".to_string(),
            line: PlanLine::Roots,
            fill: false,
            drums: Drums::Full,
        });
        let e = render(&p, 0);
        let a = arrangement(&e);
        let fill_from = a.bars[0].start_ticks + 3 * MODEL_QUARTER;
        let count = a
            .notes
            .iter()
            .filter(|n| (fill_from..a.bars[1].start_ticks).contains(&n.start_ticks))
            .count();
        assert!(count >= 3, "{count} notes in the fill");
    }

    #[test]
    fn drums_none_has_no_drum_notes() {
        let mut p = plan(&["C", "C"], PlanLine::Roots, "quarters");
        p.parts[0].drums = Drums::None;
        let e = render(&p, 0);
        let a = arrangement(&e);
        assert!(a.drums.is_empty(), "{:?}", a.drums);
    }

    #[test]
    fn same_seed_renders_the_same_midi() {
        let p = plan(&["C", "G", "Am", "F"], PlanLine::Roots, "eighths");
        let first = render(&p, 7);
        let second = render(&p, 7);
        assert_eq!(first.midi, second.midi);
    }

    #[test]
    fn different_seeds_can_render_different_notes() {
        let mut p = plan(&["C"], PlanLine::Roots, "quarters");
        p.parts[0].times = 8;
        // Found by trying seeds 1..20 against seed 0's notes: both differ from the plain
        // piece and from each other, so the twist in `vary` really depends on the seed.
        let plain = notes_of(&render(&p, 0));
        let mut differing = Vec::new();
        for seed in 1..20 {
            let notes = notes_of(&render(&p, seed));
            if notes != plain {
                differing.push((seed, notes));
            }
        }
        assert!(
            differing.len() >= 2,
            "only {} seeds varied",
            differing.len()
        );
        let (_, a) = &differing[0];
        let (_, b) = &differing[1];
        assert_ne!(a, b, "two varied seeds produced the same notes");
    }

    #[test]
    fn seed_zero_never_varies() {
        let mut p = plan(&["C"], PlanLine::Roots, "quarters");
        p.parts[0].times = 8;
        let e = render(&p, 0);
        let a = arrangement(&e);
        // Four quarters per bar, eight bars, one closing tonic bar: no twist, no fill.
        assert_eq!(a.notes.len(), 8 * 4 + 1, "{} notes", a.notes.len());
    }

    fn notes_of(e: &Exercise) -> Vec<i32> {
        arrangement(e)
            .notes
            .iter()
            .map(|n| i32::from(n.pitch.0))
            .collect()
    }
}
