//! Live practice: connects the engine (input events, output) with a `Session` and keeps the
//! metronome, count-in, reference tones and backing track in step with the transport.
//!
//! ```text
//! engine events ──► Practice::handle ──► Session::on_note
//! Practice::tick (every ~10 ms) ──► Session::update ──► schedule clicks/tones ahead ──► Output
//!                                                   └─► restart backing on play/seek/speed/loop
//! ```

use crate::session::{Mode, Session, SessionEvent};
use crate::settings::ScoreSettings;
use crate::transport::LoopRange;
use kontra_engine::{
    Accent, BackingAudio, BackingTrack, Clock, Event, Gains, Output, Resolution, Sound,
};
use kontra_model::{Arrangement, TICKS_PER_QUARTER};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// How far ahead sounds are scheduled.
const LOOKAHEAD_S: f64 = 0.25;
/// Delay between a user action and the first sound, so scheduling is never late.
const START_LEAD_S: f64 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PracticeOptions {
    pub metronome: bool,
    pub count_in_bars: u32,
    /// Play each expected note as a synthetic bass tone (LH-WIE-03).
    pub reference_tones: bool,
    /// Where arrangement time 0 lies in the backing file, in seconds.
    pub backing_offset_s: f64,
    /// Backing track pitch in semitones (LH-WIE-02); the notes are transposed by `content`.
    pub transpose_semitones: i32,
}

impl Default for PracticeOptions {
    fn default() -> Self {
        PracticeOptions {
            metronome: true,
            count_in_bars: 1,
            reference_tones: false,
            backing_offset_s: 0.0,
            transpose_semitones: 0,
        }
    }
}

/// What the UI hears from a running practice.
#[derive(Debug, Clone, PartialEq)]
pub enum PracticeEvent {
    Session(SessionEvent),
    Level {
        peak_db: f32,
        rms_db: f32,
        clipped: bool,
    },
    Tuner {
        note: String,
        cents: f64,
    },
    Error(String),
    InputEnded,
    /// Ear training: a note played as an answer, fractional MIDI.
    Heard {
        midi: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Beat {
    position_s: f64,
    accent: Accent,
}

pub struct Practice<O: Output> {
    clock: Clock,
    output: O,
    session: Session,
    options: PracticeOptions,
    beats: Vec<Beat>,
    scheduled: HashSet<(u32, u64)>,
    backing: Option<BackingTrack>,
    /// Stretched backing per speed (key: speed in permille); also keeps every Arc alive.
    stretched: HashMap<u32, BackingAudio>,
    last_pass: u32,
    was_held: bool,
}

impl<O: Output> std::fmt::Debug for Practice<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Practice")
            .field("mode", &self.session.mode())
            .field("options", &self.options)
            .finish()
    }
}

impl<O: Output> Practice<O> {
    pub fn new(
        arrangement: Arrangement,
        mode: Mode,
        settings: ScoreSettings,
        options: PracticeOptions,
        output: O,
        clock: Clock,
    ) -> Self {
        let beats = beats_of(&arrangement);
        Practice {
            clock,
            output,
            session: Session::new(arrangement, mode, settings),
            options,
            beats,
            scheduled: HashSet::new(),
            backing: None,
            stretched: HashMap::new(),
            last_pass: 0,
            was_held: false,
        }
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn output(&self) -> &O {
        &self.output
    }

    /// Sets the backing track (Wait mode never plays it: the clock stops at every note),
    /// sounding transposed as the options say.
    pub fn set_backing(&mut self, track: Option<BackingTrack>) {
        self.backing = track;
        // The mixer shifts the pitch while it plays (round 8: no rendering ahead).
        self.output
            .set_transpose(self.options.transpose_semitones as f32);
        self.stretched.clear();
    }

    /// Sets the mixer's volumes (LH-WIE-03, LH-MOD-05); takes effect at once, also while playing.
    pub fn set_gains(&mut self, gains: Gains) {
        self.output.set_gains(gains);
    }

    pub fn set_options(&mut self, options: PracticeOptions) {
        self.options = options;
        self.reschedule();
    }

    /// Transposes while playing (LH-WIE-02): `arrangement` is the piece moved by `semitones`
    /// (notes laid out again by `content`). The run goes on; the backing shifts in the mixer
    /// and the reference tones follow from now. False when the notes differ in more than
    /// pitch: then nothing changed.
    pub fn retarget(&mut self, arrangement: Arrangement, semitones: i32) -> bool {
        if !self.session.retarget(arrangement) {
            return false;
        }
        self.options.transpose_semitones = semitones;
        self.output.set_transpose(semitones as f32);
        self.reschedule();
        true
    }

    /// Starts after the count-in.
    pub fn play(&mut self) {
        let start = self.clock.now_s() + START_LEAD_S;
        self.play_from(start);
    }

    /// Like `play`, but the count-in starts exactly at engine time `count_in_s`; lines up with a
    /// recording that started at a known time (its first sample = first count-in click).
    pub fn play_from(&mut self, count_in_s: f64) {
        let now = count_in_s;
        let speed = self.session.transport().speed();
        let beat_s = self.beat_length_s() / speed;
        let count_in = self.beats_per_bar() * self.options.count_in_bars as usize;
        let start = now + count_in as f64 * beat_s;
        self.output.clear_from(self.clock.now_s());
        self.scheduled.clear();
        for i in 0..count_in {
            let accent = if i % self.beats_per_bar().max(1) == 0 {
                Accent::Downbeat
            } else {
                Accent::Beat
            };
            self.output
                .schedule(now + i as f64 * beat_s, Sound::Click(accent));
        }
        self.session.play(start);
        self.restart_backing(start);
    }

    /// Arrangement position now (seconds at original tempo).
    pub fn position_s(&self) -> f64 {
        self.session.transport().position_at(self.clock.now_s()).0
    }

    pub fn is_playing(&self) -> bool {
        self.session.transport().is_running()
    }

    /// Jumps to `position_s` and re-plans sounds and backing.
    pub fn seek(&mut self, position_s: f64) {
        let now = self.clock.now_s() + START_LEAD_S;
        self.session.seek(now, position_s);
        self.reschedule();
    }

    pub fn pause(&mut self) {
        let now = self.clock.now_s();
        self.session.pause(now);
        self.output.clear_from(now);
        self.output.stop_backing(now);
        self.scheduled.clear();
    }

    pub fn set_speed(&mut self, speed: f64) {
        let now = self.clock.now_s() + START_LEAD_S;
        self.session.set_speed(now, speed);
        self.reschedule();
    }

    pub fn set_loop(&mut self, range: Option<LoopRange>) {
        let now = self.clock.now_s() + START_LEAD_S;
        self.session.set_loop(now, range);
        self.reschedule();
    }

    /// Feeds one engine event.
    pub fn handle(&mut self, event: Event) -> Option<PracticeEvent> {
        match event {
            Event::Note(note) => {
                self.session.on_note(&note);
                None
            }
            Event::Technique(revision) => {
                self.session.on_revision(&revision);
                None
            }
            Event::Level {
                peak_db,
                rms_db,
                clipped,
            } => Some(PracticeEvent::Level {
                peak_db,
                rms_db,
                clipped,
            }),
            Event::Tuner(r) => Some(PracticeEvent::Tuner {
                note: r.pitch.to_string(),
                cents: r.cents,
            }),
            Event::Error(message) => Some(PracticeEvent::Error(message)),
            Event::Ended => Some(PracticeEvent::InputEnded),
        }
    }

    /// Advances the session and schedules what is due; call every 10–20 ms.
    pub fn tick(&mut self) -> Vec<PracticeEvent> {
        let now = self.clock.now_s();
        let events = self.session.update(now);
        let transport = self.session.transport();
        let pass = transport.pass();
        let held = transport.is_held();
        let speed_changed = events
            .iter()
            .any(|e| matches!(e, SessionEvent::SpeedChanged(_)));
        if speed_changed || (self.was_held && !held) {
            self.reschedule();
        }
        if pass != self.last_pass {
            self.last_pass = pass;
            self.prune(pass);
        }
        self.was_held = held;
        self.schedule_ahead(now);
        events.into_iter().map(PracticeEvent::Session).collect()
    }

    /// Clears everything scheduled from now and plans again (speed, loop, seek changed).
    fn reschedule(&mut self) {
        let now = self.clock.now_s();
        self.output.clear_from(now + START_LEAD_S);
        self.scheduled.clear();
        if self.session.transport().is_running() {
            self.restart_backing(now + START_LEAD_S);
        }
    }

    fn prune(&mut self, pass: u32) {
        self.scheduled.retain(|(p, _)| *p + 1 >= pass);
    }

    /// Schedules clicks and reference tones in the next `LOOKAHEAD_S`, across a loop wrap.
    fn schedule_ahead(&mut self, now: f64) {
        let transport = self.session.transport();
        let (anchor_real, anchor_pos, rate, pass) = transport.anchor();
        if !transport.is_running() || rate == 0.0 {
            return;
        }
        let from_real = now.max(anchor_real);
        let position = anchor_pos + (from_real - anchor_real) * rate;
        let horizon = anchor_pos + (now + LOOKAHEAD_S - anchor_real) * rate;
        if horizon <= position {
            return;
        }
        // Real time at which this pass would be at position 0.
        let origin = anchor_real - anchor_pos / rate;
        let looping = transport.loop_range();
        let (start, end) = looping.map_or((0.0, transport.end_s()), |l| (l.start_s, l.end_s));
        let mut windows = vec![(pass, position, horizon.min(end), origin)];
        if horizon > end && looping.is_some() {
            let wrap_real = origin + end / rate;
            windows.push((
                pass + 1,
                start,
                start + (horizon - end),
                wrap_real - start / rate,
            ));
        }
        let hold = self.hold_limit();
        for (window_pass, from, to, window_origin) in windows {
            let to = hold.map_or(to, |h| to.min(h + 1e-9));
            self.schedule_window(window_pass, from, to, window_origin, rate);
        }
    }

    /// In Wait mode nothing is scheduled beyond the note the clock will stop at.
    fn hold_limit(&self) -> Option<f64> {
        if !matches!(self.session.mode(), Mode::Wait) {
            return None;
        }
        self.session.next_expected().map(|e| e.position_s)
    }

    fn schedule_window(&mut self, pass: u32, from: f64, to: f64, origin: f64, speed: f64) {
        if self.options.metronome {
            for (i, beat) in self
                .beats
                .iter()
                .enumerate()
                .filter(|(_, b)| b.position_s >= from && b.position_s < to)
            {
                if self.scheduled.insert((pass, i as u64)) {
                    self.output
                        .schedule(origin + beat.position_s / speed, Sound::Click(beat.accent));
                }
            }
        }
        self.schedule_drums(pass, from, to, origin, speed);
        if !self.options.reference_tones {
            return;
        }
        let arrangement = self.session.arrangement();
        let tones: Vec<(u64, f64, Sound)> = arrangement
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.needs_attack())
            .filter_map(|(i, n)| {
                let at = arrangement.tempo.seconds_at(n.start_ticks);
                if at < from || at >= to {
                    return None;
                }
                let duration = (arrangement.tempo.seconds_at(n.end_ticks()) - at) / speed;
                let hz = n.pitch.hz(arrangement.instrument.a4_hz) as f32;
                Some((
                    1 << 40 | i as u64,
                    at,
                    Sound::Tone {
                        hz,
                        duration_s: duration as f32,
                        velocity: 0.8,
                    },
                ))
            })
            .collect();
        for (key, at, sound) in tones {
            if self.scheduled.insert((pass, key)) {
                self.output.schedule(origin + at / speed, sound);
            }
        }
    }

    /// Drum accompaniment of the arrangement (LH-LRN-01); its volume is the mixer's drum bus.
    fn schedule_drums(&mut self, pass: u32, from: f64, to: f64, origin: f64, speed: f64) {
        const DRUM_KEYS: u64 = 2 << 40;
        let arrangement = self.session.arrangement();
        let first = arrangement
            .drums
            .partition_point(|d| arrangement.tempo.seconds_at(d.tick) < from);
        let due: Vec<(u64, f64, Sound)> = arrangement.drums[first..]
            .iter()
            .enumerate()
            .map(|(i, d)| (first + i, arrangement.tempo.seconds_at(d.tick), d.kind))
            .take_while(|(_, at, _)| *at < to)
            .map(|(i, at, kind)| (DRUM_KEYS | i as u64, at, Sound::Drum(kind)))
            .collect();
        for (key, at, sound) in due {
            if self.scheduled.insert((pass, key)) {
                self.output.schedule(origin + at / speed, sound);
            }
        }
    }

    /// Starts the backing so that its audio lines up with the transport from `at_s` on, and
    /// queues the restart at the next loop start.
    fn restart_backing(&mut self, at_s: f64) {
        if matches!(self.session.mode(), Mode::Wait) {
            return;
        }
        let speed = self.session.transport().speed();
        let Some(audio) = self.stretched_backing(speed) else {
            return;
        };
        let sample_rate = f64::from(self.output.sample_rate());
        let (position, _) = self.session.transport().position_at(at_s);
        let frame_of = |p: f64| {
            (((p + self.options.backing_offset_s) / speed) * sample_rate).max(0.0) as usize
        };
        self.output.stop_backing(at_s);
        self.output
            .play_backing(Arc::clone(&audio), at_s, frame_of(position));
        if let Some(LoopRange { start_s, end_s }) = self.session.transport().loop_range() {
            let wrap = at_s + (end_s - position) / speed;
            self.output.play_backing(audio, wrap, frame_of(start_s));
        }
    }

    fn stretched_backing(&mut self, speed: f64) -> Option<BackingAudio> {
        let key = (speed * 1000.0).round() as u32;
        if let Some(audio) = self.stretched.get(&key) {
            return Some(Arc::clone(audio));
        }
        let track = self.backing.as_ref()?;
        let audio: BackingAudio = Arc::new(track.interleaved(speed, self.stretch_resolution()));
        self.stretched.insert(key, Arc::clone(&audio));
        Some(audio)
    }

    /// Fine stretch window only when the lowest string is below E1 (five and six strings):
    /// the longer window resolves partials of low B at the cost of more latency.
    fn stretch_resolution(&self) -> Resolution {
        const E1: u8 = 28;
        match self.session.arrangement().instrument.tuning.lowest() {
            Some(low) if low.0 < E1 => Resolution::Fine,
            _ => Resolution::Standard,
        }
    }

    fn beats_per_bar(&self) -> usize {
        self.session
            .arrangement()
            .bars
            .first()
            .map_or(4, |b| usize::from(b.signature.numerator.max(1)))
    }

    fn beat_length_s(&self) -> f64 {
        let a = self.session.arrangement();
        let bar = a.bars.first();
        let ticks = bar.map_or(TICKS_PER_QUARTER, |b| {
            b.signature.bar_ticks() / u64::from(b.signature.numerator.max(1))
        });
        a.tempo.seconds_at(ticks)
    }
}

/// Metronome beats from the bars: one click per numerator unit, accent on the downbeat.
fn beats_of(arrangement: &Arrangement) -> Vec<Beat> {
    let mut beats = Vec::new();
    for bar in &arrangement.bars {
        let count = u64::from(bar.signature.numerator.max(1));
        let step = bar.signature.bar_ticks() / count;
        for i in 0..count {
            let tick = bar.start_ticks + i * step;
            let accent = if i == 0 {
                Accent::Downbeat
            } else {
                Accent::Beat
            };
            beats.push(Beat {
                position_s: arrangement.tempo.seconds_at(tick),
                accent,
            });
        }
    }
    beats
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::{Instrument, Note, Pitch, TempoMap};

    #[derive(Debug, Default)]
    struct Recorder {
        sounds: Vec<(f64, Sound)>,
        backings: Vec<(f64, usize)>,
        cleared: Vec<f64>,
        gains: Option<Gains>,
    }

    impl Output for Recorder {
        fn sample_rate(&self) -> u32 {
            1000
        }
        fn schedule(&mut self, at_s: f64, sound: Sound) -> bool {
            self.sounds.push((at_s, sound));
            true
        }
        fn clear_from(&mut self, from_s: f64) {
            self.cleared.push(from_s);
            self.sounds.retain(|(t, _)| *t < from_s);
        }
        fn play_backing(&mut self, _audio: BackingAudio, at_s: f64, from_frame: usize) {
            self.backings.push((at_s, from_frame));
        }
        fn stop_backing(&mut self, _at_s: f64) {}
        fn set_gains(&mut self, gains: Gains) {
            self.gains = Some(gains);
        }
    }

    fn arrangement() -> Arrangement {
        let mut a = Arrangement::new("Clicks", Instrument::default_bass());
        a.tempo = TempoMap::constant(120.0);
        for i in 0..16 {
            a.notes.push(Note::new(
                i * TICKS_PER_QUARTER,
                TICKS_PER_QUARTER,
                Pitch(28),
            ));
        }
        a.normalize();
        a
    }

    fn run(p: &mut Practice<Recorder>, clock: &Clock, seconds: f64) {
        let end = clock.now_s() + seconds;
        while clock.now_s() < end {
            p.tick();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn count_in_then_clicks_on_the_beat() {
        let clock = Clock::new();
        let mut p = Practice::new(
            arrangement(),
            Mode::Play,
            ScoreSettings::default(),
            PracticeOptions::default(),
            Recorder::default(),
            clock.clone(),
        );
        p.play();
        // Count-in ends after 2 s; run into the song.
        run(&mut p, &clock, 2.6);
        let times: Vec<f64> = p.output().sounds.iter().map(|(t, _)| *t).collect();
        // 4 count-in clicks 0.5 s apart, then the song's clicks continue the grid.
        assert!(times.len() >= 6, "{times:?}");
        for pair in times.windows(2) {
            assert!(((pair[1] - pair[0]) - 0.5).abs() < 1e-6, "{times:?}");
        }
    }

    #[test]
    fn loop_wraps_clicks_without_duplicates() {
        let clock = Clock::new();
        let options = PracticeOptions {
            count_in_bars: 0,
            ..PracticeOptions::default()
        };
        let mut p = Practice::new(
            arrangement(),
            Mode::Play,
            ScoreSettings::default(),
            options,
            Recorder::default(),
            clock.clone(),
        );
        p.set_speed(2.0);
        p.set_loop(Some(LoopRange {
            start_s: 0.0,
            end_s: 1.0,
        }));
        p.play();
        run(&mut p, &clock, 1.2);
        let times: Vec<f64> = p.output().sounds.iter().map(|(t, _)| *t).collect();
        // At double speed the loop (2 beats) lasts 0.5 s: clicks every 0.25 s, no doubles.
        for pair in times.windows(2) {
            assert!(((pair[1] - pair[0]) - 0.25).abs() < 1e-6, "{times:?}");
        }
        assert!(times.len() >= 4);
    }

    #[test]
    fn drums_play_on_their_ticks() {
        use kontra_model::{DrumHit, DrumKind};
        let clock = Clock::new();
        let mut a = arrangement();
        a.drums = (0..8)
            .map(|i| DrumHit {
                tick: i * TICKS_PER_QUARTER / 2,
                kind: DrumKind::HiHat,
            })
            .collect();
        let options = PracticeOptions {
            metronome: false,
            count_in_bars: 0,
            ..PracticeOptions::default()
        };
        let mut p = Practice::new(
            a,
            Mode::Play,
            ScoreSettings::default(),
            options,
            Recorder::default(),
            clock.clone(),
        );
        p.play();
        run(&mut p, &clock, 0.6);
        let times: Vec<f64> = p
            .output()
            .sounds
            .iter()
            .filter(|(_, s)| matches!(s, Sound::Drum(DrumKind::HiHat)))
            .map(|(t, _)| *t)
            .collect();
        // Eighths at 120 BPM: 0.25 s apart, none twice.
        assert!(times.len() >= 3, "{times:?}");
        for pair in times.windows(2) {
            assert!(((pair[1] - pair[0]) - 0.25).abs() < 1e-6, "{times:?}");
        }
    }

    #[test]
    fn gains_reach_the_output() {
        let mut p = Practice::new(
            arrangement(),
            Mode::Play,
            ScoreSettings::default(),
            PracticeOptions::default(),
            Recorder::default(),
            Clock::new(),
        );
        let gains = Gains {
            monitor: 0.5,
            ..Gains::default()
        };
        p.set_gains(gains);
        assert_eq!(p.output().gains, Some(gains));
    }
}
