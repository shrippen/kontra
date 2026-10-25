//! Subtitles of course videos (LH-LRN-14): the cues of a WebVTT file, shown under the video.
//!
//! ```text
//! WEBVTT
//!
//! 00:01.000 --> 00:04.500      a cue: start --> end (hours optional), settings after it ignored
//! Zeigefinger auf den 5. Bund.  its text, up to the next blank line
//! ```
//!
//! Styling, regions and notes are left out: the text is what P7 needs.

const ARROW: &str = "-->";
const MS_PER_S: u64 = 1000;
const S_PER_MIN: u64 = 60;

/// One subtitle: shown from `start_ms` to `end_ms`.
#[derive(Debug, Clone, PartialEq)]
pub struct Cue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// The cues of a WebVTT text, in file order; blocks that are no cue are skipped.
pub fn cues(text: &str) -> Vec<Cue> {
    let mut out = Vec::new();
    let normalized = text.replace("\r\n", "\n");
    for block in normalized.split("\n\n") {
        let mut lines = block.lines().skip_while(|l| !l.contains(ARROW));
        let Some(timing) = lines.next() else {
            continue;
        };
        let Some((start, rest)) = timing.split_once(ARROW) else {
            continue;
        };
        let end = rest.split_whitespace().next().unwrap_or_default();
        let (Some(start_ms), Some(end_ms)) = (time_ms(start.trim()), time_ms(end)) else {
            continue;
        };
        let text = lines.collect::<Vec<_>>().join("\n");
        out.push(Cue {
            start_ms,
            end_ms,
            text: strip_tags(&text),
        });
    }
    out
}

/// "01:02.500" or "1:01:02.500" → milliseconds.
fn time_ms(time: &str) -> Option<u64> {
    let (clock, ms) = time.split_once('.')?;
    let ms: u64 = ms.parse().ok()?;
    // Checked: the file comes from a foreign course; a time too long to count drops the cue.
    let mut s: u64 = 0;
    for part in clock.split(':') {
        s = s.checked_mul(S_PER_MIN)?.checked_add(part.parse().ok()?)?;
    }
    s.checked_mul(MS_PER_S)?.checked_add(ms)
}

/// Text without inline tags ("<i>so</i>" → "so"); cues are shown as plain text.
fn strip_tags(text: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in text.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_cues_with_and_without_hours() {
        let vtt = "WEBVTT\r\n\r\n1\r\n00:01.000 --> 00:04.500 align:start\r\nZeige<i>finger</i>\r\nauf 5\r\n\r\nNOTE skip me\r\n\r\n1:00:00.250 --> 1:00:01.000\r\nEnde\r\n";
        assert_eq!(
            cues(vtt),
            [
                Cue {
                    start_ms: 1000,
                    end_ms: 4500,
                    text: "Zeigefinger\nauf 5".into()
                },
                Cue {
                    start_ms: 3_600_250,
                    end_ms: 3_601_000,
                    text: "Ende".into()
                },
            ]
        );
    }

    #[test]
    fn skips_a_cue_too_long_to_count() {
        let vtt = "WEBVTT\n\n18446744073709551615:00.000 --> 18446744073709551615:01.000\nNie\n";
        assert!(cues(vtt).is_empty());
    }
}
