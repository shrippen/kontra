//! Plain-text tablature for the terminal: one column per sixteenth, highest string on top.

use kontra_model::{Arrangement, TICKS_PER_QUARTER};

const STEP: u64 = TICKS_PER_QUARTER / 4;

/// Renders the first `bars` bars.
pub fn render(arrangement: &Arrangement, bars: usize) -> String {
    let strings = arrangement.instrument.tuning.strings.len();
    let Some(last_bar) = arrangement
        .bars
        .get(bars.min(arrangement.bars.len()).saturating_sub(1))
    else {
        return String::new();
    };
    let end = last_bar.end_ticks();
    let mut lines: Vec<String> = arrangement
        .instrument
        .tuning
        .strings
        .iter()
        .map(|p| format!("{:<2}|", p.name().to_string()))
        .collect();
    for bar in arrangement.bars.iter().take(bars) {
        let mut tick = bar.start_ticks;
        while tick < bar.end_ticks().min(end) {
            let notes = arrangement.notes_between(tick, tick + STEP);
            let width = notes
                .iter()
                .filter_map(|n| n.fret)
                .map(|f| f.to_string().len())
                .max()
                .unwrap_or(1);
            for (s, line) in lines.iter_mut().enumerate() {
                let fret = notes
                    .iter()
                    .find(|n| n.string == Some(s as u8))
                    .and_then(|n| n.fret);
                match fret {
                    Some(f) => line.push_str(&format!("{f:-<width$}")),
                    None => line.push_str(&"-".repeat(width)),
                }
            }
            tick += STEP;
        }
        lines.iter_mut().for_each(|l| l.push('|'));
    }
    let mut out = String::new();
    for line in lines.iter().rev().take(strings) {
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::{Instrument, Note, Pitch};

    #[test]
    fn renders_frets_on_strings() {
        let mut a = Arrangement::new("t", Instrument::default_bass());
        let mut n = Note::new(0, TICKS_PER_QUARTER, Pitch(33));
        n.string = Some(0);
        n.fret = Some(5);
        a.notes.push(n);
        a.normalize();
        let text = render(&a, 1);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[3].starts_with("E |5---"), "{text}");
        assert!(lines[0].starts_with("G |----"));
    }
}
