//! Hardware checks (round 10): takes that play a plan of sounds and keep what the input heard,
//! for the loopback latency and for recordings of a real bass that go into the corpus.
//!
//! ```text
//!            zero (1 s after the input's first sample)
//!              │ click  click  click  click │ note  note …      plan, in s after zero
//! output ──────┴──────────────────────────────────────────►
//! input  ─► raw samples (Take::samples, from Take::start_s) and detected notes (plan time)
//!
//! loopback: output cabled to input ─► detected onset − planned tone = round trip
//! bass:     annotation = planned beat + the player's latency (calibration) ─► korpus pair
//! ```

use crate::live::{input_kind, open_output, LiveOutput, LiveSetup};
use kontra_engine::{Accent, Annotation, Clock, EngineError, Event, Input, Output, Sound};
use kontra_model::{DetectedNote, Instrument, Pitch, Technique};
use std::sync::mpsc::{channel, Receiver};

/// Time from the input's first sample to the plan's zero: the output is running by then.
const LEAD_S: f64 = 1.0;
/// Tempo of the bass takes: slow enough for slaps and slides on every beat.
pub const CHECK_BPM: f64 = 80.0;
/// Clicks before the first note (one 4/4 bar).
const COUNT_IN_BEATS: f64 = 4.0;
/// Silence kept after the last note, for its decay.
const TAIL_BEATS: f64 = 2.0;
const MS_PER_S: f64 = 1000.0;
/// Concert pitch of the planned tones.
const A4_HZ: f64 = 440.0;

/// Loopback: tones at a pitch the tracker hears well, far enough apart to tell them apart.
const LOOPBACK_TONES: usize = 12;
const LOOPBACK_MIDI: u8 = 40; // E2
const LOOPBACK_SPACING_S: f64 = 0.6;
const LOOPBACK_TONE_S: f64 = 0.25;
/// A detection belongs to a tone if it comes at most this late (no cable: nothing comes).
const LOOPBACK_WINDOW_S: f64 = 0.3;

/// What the output plays during a take.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TakeSound {
    Click { downbeat: bool },
    Tone { midi: u8, duration_s: f64 },
}

/// What the input heard, on the plan's time (0 = the plan's zero).
#[derive(Debug, Clone, PartialEq)]
pub struct Take {
    pub sample_rate: u32,
    pub samples: Vec<f32>,
    /// Plan time of `samples[0]` (negative: recorded before zero).
    pub start_s: f64,
    pub notes: Vec<DetectedNote>,
    /// Input and output dropouts during the take.
    pub xruns: u64,
    /// The audio server's block (quantum) in frames, once known.
    pub block_frames: Option<u32>,
}

/// A running take: plays its plan and records the input.
#[derive(Debug)]
pub struct LiveTake {
    clock: Clock,
    output: LiveOutput,
    input: Input,
    events: Receiver<Event>,
    plan: Vec<(f64, TakeSound)>,
    length_s: f64,
    zero_s: Option<f64>,
    notes: Vec<DetectedNote>,
}

impl LiveTake {
    /// Starts input and output; the plan begins `LEAD_S` after the input's first sample and
    /// the take ends `length_s` after that.
    pub fn start(
        setup: &LiveSetup,
        instrument: &Instrument,
        plan: Vec<(f64, TakeSound)>,
        length_s: f64,
    ) -> Result<LiveTake, EngineError> {
        let clock = Clock::new();
        let (tx, rx) = channel();
        let output = open_output(setup.output_device.as_deref(), &clock, tx.clone());
        let input = Input::start(
            &input_kind(&setup.input),
            instrument,
            setup.source,
            &clock,
            tx,
            None,
        )?;
        input.record();
        Ok(LiveTake {
            clock,
            output,
            input,
            events: rx,
            plan,
            length_s,
            zero_s: None,
            notes: Vec::new(),
        })
    }

    /// True when no sound card was found (the plan is not heard).
    pub fn output_is_silent(&self) -> bool {
        matches!(self.output, LiveOutput::Silent(_))
    }

    /// Polls; true once the take is over.
    pub fn poll(&mut self) -> bool {
        if self.zero_s.is_none() {
            let Some(start) = self.input.started_s() else {
                return false;
            };
            let zero = start + LEAD_S;
            for (at, sound) in &self.plan {
                self.output.schedule(zero + at, engine_sound(*sound));
            }
            self.zero_s = Some(zero);
        }
        let Some(zero) = self.zero_s else {
            return false;
        };
        while let Ok(event) = self.events.try_recv() {
            if let Event::Note(mut note) = event {
                note.onset_s -= zero;
                self.notes.push(note);
            }
        }
        self.clock.now_s() > zero + self.length_s
    }

    /// Ends the take and hands over what was heard.
    pub fn finish(self) -> Take {
        let recording = self.input.stop_recording().unwrap_or_default();
        let rate = self.input.sample_rate.max(1);
        let started = self.input.started_s().unwrap_or(0.0);
        let zero = self.zero_s.unwrap_or(started + LEAD_S);
        Take {
            sample_rate: rate,
            samples: recording.samples,
            start_s: started + recording.first_sample as f64 / f64::from(rate) - zero,
            notes: self.notes,
            xruns: self.input.xruns() + self.output.xruns(),
            block_frames: self.input.block_frames(),
        }
    }
}

fn engine_sound(sound: TakeSound) -> Sound {
    match sound {
        TakeSound::Click { downbeat } => Sound::Click(if downbeat {
            Accent::Downbeat
        } else {
            Accent::Beat
        }),
        TakeSound::Tone { midi, duration_s } => Sound::Tone {
            hz: Pitch(midi).hz(A4_HZ) as f32,
            duration_s: duration_s as f32,
            velocity: 0.8,
        },
    }
}

/// The loopback plan: tones only, no clicks (a click has no pitch to find).
pub fn loopback_plan() -> Vec<(f64, TakeSound)> {
    (0..LOOPBACK_TONES)
        .map(|i| {
            let tone = TakeSound::Tone {
                midi: LOOPBACK_MIDI,
                duration_s: LOOPBACK_TONE_S,
            };
            (i as f64 * LOOPBACK_SPACING_S, tone)
        })
        .collect()
}

/// Length of the loopback take.
pub fn loopback_length_s() -> f64 {
    LOOPBACK_TONES as f64 * LOOPBACK_SPACING_S + LOOPBACK_WINDOW_S
}

/// Output to input through a cable: how late each tone came back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoundTrip {
    pub median_ms: f64,
    /// Largest distance of a tone from the median.
    pub spread_ms: f64,
    pub heard: usize,
    pub sent: usize,
}

/// The round trip of a loopback take; `None` if no tone came back.
pub fn round_trip(plan: &[(f64, TakeSound)], take: &Take) -> Option<RoundTrip> {
    let mut delays: Vec<f64> = plan
        .iter()
        .filter_map(|(at, sound)| {
            let TakeSound::Tone { midi, .. } = sound else {
                return None;
            };
            take.notes
                .iter()
                .filter(|n| same_pitch_class(n.midi, *midi))
                .map(|n| n.onset_s - at)
                .filter(|d| (0.0..=LOOPBACK_WINDOW_S).contains(d))
                .min_by(f64::total_cmp)
        })
        .collect();
    if delays.is_empty() {
        return None;
    }
    delays.sort_by(f64::total_cmp);
    let median = delays[delays.len() / 2];
    let spread = delays
        .iter()
        .map(|d| (d - median).abs())
        .fold(0.0, f64::max);
    Some(RoundTrip {
        median_ms: median * MS_PER_S,
        spread_ms: spread * MS_PER_S,
        heard: delays.len(),
        sent: plan.len(),
    })
}

/// The tone came back, maybe an octave off (the tracker's usual error on a sine).
fn same_pitch_class(heard: f64, planned: u8) -> bool {
    (heard.round() - f64::from(planned)).rem_euclid(12.0) == 0.0
}

/// One note to play in a bass take, on a 4-string bass in standard tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckNote {
    /// Beats after the count-in.
    pub beat: f64,
    pub midi: u8,
    /// 0 = E string.
    pub string: u8,
    pub fret: u8,
    pub technique: Option<Technique>,
}

/// A bass take: what to play, with a click.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckTake {
    /// Short name, also the recording's file name.
    pub name: &'static str,
    pub notes: Vec<CheckNote>,
}

impl CheckTake {
    fn beats(&self) -> f64 {
        self.notes
            .iter()
            .map(|n| n.beat)
            .fold(0.0, f64::max)
            .floor()
            + 1.0
    }

    /// Clicks on every beat from the count-in to the last note's beat.
    pub fn plan(&self) -> Vec<(f64, TakeSound)> {
        let beat_s = 60.0 / CHECK_BPM;
        let clicks = (COUNT_IN_BEATS + self.beats()) as usize;
        (0..clicks)
            .map(|i| {
                let click = TakeSound::Click {
                    downbeat: i % 4 == 0,
                };
                (i as f64 * beat_s, click)
            })
            .collect()
    }

    pub fn length_s(&self) -> f64 {
        (COUNT_IN_BEATS + self.beats() + TAIL_BEATS) * 60.0 / CHECK_BPM
    }

    /// The corpus annotation of a take: each note at its beat plus the player's latency (what
    /// "play on the click" measured), in seconds from the take's first sample.
    pub fn annotate(&self, take: &Take, latency_ms: f64) -> Vec<Annotation> {
        let beat_s = 60.0 / CHECK_BPM;
        self.notes
            .iter()
            .map(|n| Annotation {
                onset_s: (COUNT_IN_BEATS + n.beat) * beat_s + latency_ms / MS_PER_S - take.start_s,
                midi: f64::from(n.midi),
                technique: n.technique,
                // Each slide's arrival is a note of its own here, not IDMT's unannotated target.
                slide_cents: None,
            })
            .collect()
    }
}

/// The bass takes of a hardware check: plain notes, then each technique Kontra tells apart.
pub fn check_takes() -> Vec<CheckTake> {
    use Technique as T;
    // (string, fret) on a 4-string bass: E1 A1 D2 G2 open.
    const OPEN: [u8; 4] = [28, 33, 38, 43];
    let note = |beat: f64, string: u8, fret: u8, technique: Option<Technique>| CheckNote {
        beat,
        midi: OPEN[usize::from(string)] + fret,
        string,
        fret,
        technique,
    };
    let quarters = |pattern: &[(u8, u8)], technique: Option<Technique>| -> Vec<CheckNote> {
        pattern
            .iter()
            .enumerate()
            .map(|(i, (string, fret))| note(i as f64, *string, *fret, technique))
            .collect()
    };
    // A plucked note on the beat and a legato one on the "and", four times.
    let pairs = |string: u8, from: u8, to: u8, technique: Technique| -> Vec<CheckNote> {
        (0..4)
            .flat_map(|i| {
                let beat = f64::from(i);
                [
                    note(beat, string, from, None),
                    note(beat + 0.5, string, to, Some(technique)),
                ]
            })
            .collect()
    };
    vec![
        CheckTake {
            name: "open-strings",
            notes: quarters(
                &[
                    (0, 0),
                    (0, 0),
                    (1, 0),
                    (1, 0),
                    (2, 0),
                    (2, 0),
                    (3, 0),
                    (3, 0),
                ],
                None,
            ),
        },
        CheckTake {
            name: "fretted",
            notes: quarters(
                &[
                    (0, 3),
                    (0, 5),
                    (1, 3),
                    (1, 5),
                    (2, 3),
                    (2, 5),
                    (3, 3),
                    (3, 5),
                ],
                None,
            ),
        },
        CheckTake {
            name: "slap",
            notes: quarters(&[(0, 0); 8], Some(T::Slap)),
        },
        CheckTake {
            name: "pop",
            notes: quarters(&[(3, 7); 8], Some(T::Pop)),
        },
        CheckTake {
            name: "dead",
            notes: quarters(&[(1, 0); 8], Some(T::Muted)),
        },
        CheckTake {
            name: "hammer-on",
            notes: pairs(1, 0, 2, T::HammerOn),
        },
        CheckTake {
            name: "pull-off",
            notes: pairs(1, 2, 0, T::PullOff),
        },
        CheckTake {
            name: "slide",
            notes: pairs(1, 5, 7, T::Slide),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::InputSource;

    fn heard(onset_s: f64) -> DetectedNote {
        DetectedNote {
            onset_s,
            midi: 40.0,
            confidence: 0.9,
            level_db: -12.0,
            source: InputSource::Line,
            technique: None,
        }
    }

    fn take(notes: Vec<DetectedNote>, start_s: f64) -> Take {
        Take {
            sample_rate: 48_000,
            samples: Vec::new(),
            start_s,
            notes,
            xruns: 0,
            block_frames: None,
        }
    }

    #[test]
    fn round_trip_is_the_median_delay() {
        let plan = loopback_plan();
        // 21 ms back, one tone 25 ms, one lost; a stray note before the first tone and another pitch
        // where the lost one was.
        let mut notes: Vec<_> = plan
            .iter()
            .skip(1)
            .map(|(at, _)| heard(at + 0.021))
            .collect();
        notes[3].onset_s += 0.004;
        notes.push(heard(-0.5));
        notes.push(DetectedNote {
            midi: 45.0,
            ..heard(plan[0].0 + 0.01)
        });
        let r = round_trip(&plan, &take(notes, -1.0)).unwrap();
        assert!((r.median_ms - 21.0).abs() < 1e-6, "{r:?}");
        assert!((r.spread_ms - 4.0).abs() < 1e-6, "{r:?}");
        assert_eq!((r.heard, r.sent), (LOOPBACK_TONES - 1, LOOPBACK_TONES));
        assert_eq!(round_trip(&plan, &take(Vec::new(), -1.0)), None, "no cable");
    }

    #[test]
    fn takes_annotate_beats_after_the_count_in() {
        let takes = check_takes();
        let names: Vec<_> = takes.iter().map(|t| t.name).collect();
        assert_eq!(
            names,
            [
                "open-strings",
                "fretted",
                "slap",
                "pop",
                "dead",
                "hammer-on",
                "pull-off",
                "slide"
            ]
        );
        let hammer = &takes[5];
        assert_eq!(hammer.plan().len(), 8, "count-in and four beats");
        // 80 BPM: 0.75 s a beat; count-in 3 s; recorded from 1 s before zero; 20 ms latency.
        let a = hammer.annotate(&take(Vec::new(), -1.0), 20.0);
        assert_eq!(a.len(), 8);
        assert!((a[0].onset_s - 4.02).abs() < 1e-9, "{:?}", a[0]);
        assert!((a[1].onset_s - 4.395).abs() < 1e-9, "{:?}", a[1]);
        assert_eq!(
            (a[1].midi, a[1].technique),
            (35.0, Some(Technique::HammerOn))
        );
        assert_eq!(takes[3].notes[0].midi, 50, "pop: D3 on the G string");
    }
}
